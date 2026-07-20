//! MCP server backed by a dapz daemon.
//!
//! Unlike [`McpServer`](super::McpServer) which manages adapters in-process,
//! this server delegates session ops to a long-lived daemon via Unix socket.

use std::sync::Arc;

use rmcp::{
    ErrorData, ServerHandler,
    model::{
        CallToolRequestParams, CallToolResult, ContentBlock, ListToolsResult,
        PaginatedRequestParams, ServerCapabilities, ServerInfo,
    },
    service::{RequestContext, RoleServer},
};
use serde_json::{Value, json};
use tokio::sync::Mutex;
use tracing::{info, warn};

use super::pool::pool_key;
use super::server::{
    DebugAttachInput, DebugLaunchInput, DisconnectInput, EvaluateInput, GetScopesInput,
    GetSourceInput, GetStackInput, GetVariablesInput, SendRawInput, SessionRefInput,
    SetBreakpointsInput, SetExceptionBreakpointsInput, ThreadInput, WaitStoppedInput,
    compress_output_events, compress_response, parse_args, resolve_backend, to_toon,
    tool_definitions,
};
use crate::daemon::DaemonClient;
use crate::daemon::protocol::SpawnParams;
use crate::daemon::resolve_project_cwd;

/// Backend command + optional cwd (same shape as in-process MCP).
type BackendCwd = (String, Option<String>);
type LastSessionKey = Option<BackendCwd>;

/// MCP server that auto-connects to (or starts) a dapz daemon.
#[derive(Clone)]
pub struct DaemonMcpServer {
    client: Arc<Mutex<Option<DaemonClient>>>,
    /// Project cwd used for socket identity (DAP, not LSP roots).
    project_cwd: Arc<String>,
    last_key: Arc<Mutex<LastSessionKey>>,
}

impl DaemonMcpServer {
    /// Create a daemon-backed MCP server for `project_cwd`.
    pub fn new(project_cwd: String) -> Self {
        let project_cwd = resolve_project_cwd(&project_cwd);
        Self {
            client: Arc::new(Mutex::new(None)),
            project_cwd: Arc::new(project_cwd),
            last_key: Arc::new(Mutex::new(None)),
        }
    }

    async fn ensure_connected(&self) -> Result<(), ErrorData> {
        let mut guard = self.client.lock().await;
        if guard.is_some() {
            return Ok(());
        }
        match DaemonClient::connect_or_start(&self.project_cwd).await {
            Ok(client) => {
                info!(cwd = %self.project_cwd, "Connected to dapz daemon for MCP");
                *guard = Some(client);
                Ok(())
            }
            Err(e) => {
                warn!("Daemon connect failed: {e}");
                Err(ErrorData::internal_error(
                    format!(
                        "Cannot connect to dapz daemon: {e}. Try `dapz daemon` or use `--no-daemon`."
                    ),
                    None,
                ))
            }
        }
    }

    async fn spawn_session(
        &self,
        backend: &str,
        cwd: Option<&str>,
        extra_args: &[String],
        replace: bool,
    ) -> Result<String, ErrorData> {
        self.ensure_connected().await?;
        let params = SpawnParams {
            backend: backend.to_string(),
            cwd: cwd.map(str::to_string),
            extra_args: extra_args.to_vec(),
            replace,
        };
        let mut guard = self.client.lock().await;
        let client = guard
            .as_mut()
            .ok_or_else(|| ErrorData::internal_error("Daemon not connected", None))?;
        client
            .spawn_session(params)
            .await
            .map_err(|e| ErrorData::internal_error(e.to_string(), None))
    }

    async fn invoke(&self, session_key: &str, op: &str, args: Value) -> Result<Value, ErrorData> {
        self.ensure_connected().await?;
        let mut guard = self.client.lock().await;
        let client = guard
            .as_mut()
            .ok_or_else(|| ErrorData::internal_error("Daemon not connected", None))?;
        client
            .invoke(session_key, op, args)
            .await
            .map_err(|e| ErrorData::internal_error(e.to_string(), None))
    }

    async fn remember(&self, backend: &str, cwd: Option<&str>) {
        *self.last_key.lock().await = Some((backend.to_string(), cwd.map(|s| s.to_string())));
    }

    async fn resolve_session_key(
        &self,
        backend: Option<&str>,
        cwd: Option<&str>,
    ) -> Result<(String, Option<String>), ErrorData> {
        if let Some(b) = backend {
            return Ok((b.to_string(), cwd.map(|s| s.to_string())));
        }
        self.last_key.lock().await.clone().ok_or_else(|| {
            ErrorData::invalid_request(
                "No active session. Call debug_launch first or pass backend/cwd.",
                None,
            )
        })
    }

    async fn session_key_for(
        &self,
        backend: Option<&str>,
        cwd: Option<&str>,
    ) -> Result<String, ErrorData> {
        let (b, c) = self.resolve_session_key(backend, cwd).await?;
        Ok(pool_key(&b, c.as_deref()))
    }
}

impl ServerHandler for DaemonMcpServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build()).with_instructions(
            "dapz MCP server (daemon mode) — Tier-0 DAP debug tools. \
             Auto-connects to dapz daemon for the project cwd (starts one if needed). \
             Flow: debug_launch → get_stack → get_scopes → get_variables → evaluate → step/continue → disconnect. \
             Use --no-daemon for in-process adapters. Outputs are TOON.",
        )
    }

    async fn list_tools(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, ErrorData> {
        Ok(ListToolsResult::with_all_items(tool_definitions()))
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, ErrorData> {
        let CallToolRequestParams {
            name, arguments, ..
        } = request;
        let args = arguments.map(Value::Object);

        let text = match name.as_ref() {
            "debug_launch" => self.handle_debug_launch(parse_args(args)?).await?,
            "debug_attach" => self.handle_debug_attach(parse_args(args)?).await?,
            "set_breakpoints" => self.handle_set_breakpoints(parse_args(args)?).await?,
            "set_exception_breakpoints" => {
                self.handle_set_exception_breakpoints(parse_args(args)?)
                    .await?
            }
            "continue" => self.handle_continue(parse_args(args)?).await?,
            "step_over" => self.handle_step_over(parse_args(args)?).await?,
            "step_into" => self.handle_step_into(parse_args(args)?).await?,
            "step_out" => self.handle_step_out(parse_args(args)?).await?,
            "pause" => self.handle_pause(parse_args(args)?).await?,
            "get_threads" => self.handle_get_threads(parse_args(args)?).await?,
            "get_stack" => self.handle_get_stack(parse_args(args)?).await?,
            "get_scopes" => self.handle_get_scopes(parse_args(args)?).await?,
            "get_variables" => self.handle_get_variables(parse_args(args)?).await?,
            "evaluate" => self.handle_evaluate(parse_args(args)?).await?,
            "get_exception" => self.handle_get_exception(parse_args(args)?).await?,
            "get_source" => self.handle_get_source(parse_args(args)?).await?,
            "get_output" => self.handle_get_output(parse_args(args)?).await?,
            "wait_stopped" => self.handle_wait_stopped(parse_args(args)?).await?,
            "disconnect" => self.handle_disconnect(parse_args(args)?).await?,
            "terminate" => self.handle_terminate(parse_args(args)?).await?,
            "send_raw" => self.handle_send_raw(parse_args(args)?).await?,
            other => {
                return Err(ErrorData::invalid_request(
                    format!("Unknown tool: {other}"),
                    None,
                ));
            }
        };

        info!(tool = %name, "MCP tool completed (daemon)");
        Ok(CallToolResult::success(vec![ContentBlock::text(text)]))
    }
}

impl DaemonMcpServer {
    async fn handle_debug_launch(&self, input: DebugLaunchInput) -> Result<String, ErrorData> {
        let backend = resolve_backend(
            Some(&input.program),
            input.language.as_deref(),
            input.backend.as_deref(),
        )?;
        let cwd = input.cwd.as_deref();
        let extra = input.backend_args.unwrap_or_default();
        let key = self.spawn_session(&backend, cwd, &extra, true).await?;

        let mut args = json!({
            "program": input.program,
            "cwd": cwd,
            "args": input.args,
        });
        if let Some(bps) = input.breakpoints {
            args["breakpoints"] = json!(
                bps.into_iter()
                    .map(|b| json!({ "path": b.path, "lines": b.lines }))
                    .collect::<Vec<_>>()
            );
        }

        let stopped = self.invoke(&key, "launch_program", args).await?;
        self.remember(&backend, cwd).await;
        to_toon(&json!({
            "ok": true,
            "backend": backend,
            "cwd": cwd,
            "stopped": stopped,
            "daemon": true,
        }))
    }

    async fn handle_debug_attach(&self, input: DebugAttachInput) -> Result<String, ErrorData> {
        let backend = resolve_backend(None, input.language.as_deref(), input.backend.as_deref())?;
        let cwd = input.cwd.as_deref();
        let extra = input.backend_args.unwrap_or_default();
        let key = self.spawn_session(&backend, cwd, &extra, true).await?;
        let body = self
            .invoke(&key, "attach", json!({ "arguments": input.arguments }))
            .await?;
        self.remember(&backend, cwd).await;
        to_toon(&json!({
            "ok": true,
            "backend": backend,
            "cwd": cwd,
            "attach": body,
            "daemon": true,
        }))
    }

    async fn handle_set_breakpoints(
        &self,
        input: SetBreakpointsInput,
    ) -> Result<String, ErrorData> {
        let key = self
            .session_key_for(input.backend.as_deref(), input.cwd.as_deref())
            .await?;
        let body = self
            .invoke(
                &key,
                "set_breakpoints",
                json!({ "source": input.source, "lines": input.lines }),
            )
            .await?;
        to_toon(&body)
    }

    async fn handle_set_exception_breakpoints(
        &self,
        input: SetExceptionBreakpointsInput,
    ) -> Result<String, ErrorData> {
        let key = self
            .session_key_for(input.backend.as_deref(), input.cwd.as_deref())
            .await?;
        let body = self
            .invoke(
                &key,
                "set_exception_breakpoints",
                json!({ "filters": input.filters }),
            )
            .await?;
        to_toon(&body)
    }

    async fn handle_continue(&self, input: ThreadInput) -> Result<String, ErrorData> {
        let key = self
            .session_key_for(input.backend.as_deref(), input.cwd.as_deref())
            .await?;
        let body = self
            .invoke(&key, "continue", json!({ "thread_id": input.thread_id }))
            .await?;
        to_toon(&body)
    }

    async fn handle_step_over(&self, input: ThreadInput) -> Result<String, ErrorData> {
        let key = self
            .session_key_for(input.backend.as_deref(), input.cwd.as_deref())
            .await?;
        let body = self
            .invoke(&key, "step_over", json!({ "thread_id": input.thread_id }))
            .await?;
        to_toon(&body)
    }

    async fn handle_step_into(&self, input: ThreadInput) -> Result<String, ErrorData> {
        let key = self
            .session_key_for(input.backend.as_deref(), input.cwd.as_deref())
            .await?;
        let body = self
            .invoke(&key, "step_into", json!({ "thread_id": input.thread_id }))
            .await?;
        to_toon(&body)
    }

    async fn handle_step_out(&self, input: ThreadInput) -> Result<String, ErrorData> {
        let key = self
            .session_key_for(input.backend.as_deref(), input.cwd.as_deref())
            .await?;
        let body = self
            .invoke(&key, "step_out", json!({ "thread_id": input.thread_id }))
            .await?;
        to_toon(&body)
    }

    async fn handle_pause(&self, input: ThreadInput) -> Result<String, ErrorData> {
        let key = self
            .session_key_for(input.backend.as_deref(), input.cwd.as_deref())
            .await?;
        let body = self
            .invoke(&key, "pause", json!({ "thread_id": input.thread_id }))
            .await?;
        to_toon(&body)
    }

    async fn handle_get_threads(&self, input: SessionRefInput) -> Result<String, ErrorData> {
        let key = self
            .session_key_for(input.backend.as_deref(), input.cwd.as_deref())
            .await?;
        let body = self.invoke(&key, "get_threads", json!({})).await?;
        to_toon(&body)
    }

    async fn handle_get_stack(&self, input: GetStackInput) -> Result<String, ErrorData> {
        let key = self
            .session_key_for(input.backend.as_deref(), input.cwd.as_deref())
            .await?;
        let body = self
            .invoke(
                &key,
                "get_stack",
                json!({ "thread_id": input.thread_id, "levels": input.levels }),
            )
            .await?;
        let compressed = compress_response("stackTrace", body).await;
        to_toon(&compressed)
    }

    async fn handle_get_scopes(&self, input: GetScopesInput) -> Result<String, ErrorData> {
        let key = self
            .session_key_for(input.backend.as_deref(), input.cwd.as_deref())
            .await?;
        let body = self
            .invoke(&key, "get_scopes", json!({ "frame_id": input.frame_id }))
            .await?;
        let compressed = compress_response("scopes", body).await;
        to_toon(&compressed)
    }

    async fn handle_get_variables(&self, input: GetVariablesInput) -> Result<String, ErrorData> {
        let key = self
            .session_key_for(input.backend.as_deref(), input.cwd.as_deref())
            .await?;
        let body = self
            .invoke(
                &key,
                "get_variables",
                json!({ "variables_reference": input.variables_reference }),
            )
            .await?;
        let compressed = compress_response("variables", body).await;
        to_toon(&compressed)
    }

    async fn handle_evaluate(&self, input: EvaluateInput) -> Result<String, ErrorData> {
        let key = self
            .session_key_for(input.backend.as_deref(), input.cwd.as_deref())
            .await?;
        let body = self
            .invoke(
                &key,
                "evaluate",
                json!({
                    "expression": input.expression,
                    "frame_id": input.frame_id,
                    "context": input.context,
                }),
            )
            .await?;
        let compressed = compress_response("evaluate", body).await;
        to_toon(&compressed)
    }

    async fn handle_get_exception(&self, input: ThreadInput) -> Result<String, ErrorData> {
        let key = self
            .session_key_for(input.backend.as_deref(), input.cwd.as_deref())
            .await?;
        let body = self
            .invoke(
                &key,
                "get_exception",
                json!({ "thread_id": input.thread_id }),
            )
            .await?;
        let compressed = compress_response("exceptionInfo", body).await;
        to_toon(&compressed)
    }

    async fn handle_get_source(&self, input: GetSourceInput) -> Result<String, ErrorData> {
        let key = self
            .session_key_for(input.backend.as_deref(), input.cwd.as_deref())
            .await?;
        let body = self
            .invoke(
                &key,
                "get_source",
                json!({
                    "source_reference": input.source_reference,
                    "path": input.path,
                }),
            )
            .await?;
        to_toon(&body)
    }

    async fn handle_get_output(&self, input: SessionRefInput) -> Result<String, ErrorData> {
        let key = self
            .session_key_for(input.backend.as_deref(), input.cwd.as_deref())
            .await?;
        let body = self.invoke(&key, "drain_output", json!({})).await?;
        let events = body
            .get("outputs")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();
        let compressed = compress_output_events(events).await;
        to_toon(&compressed)
    }

    async fn handle_wait_stopped(&self, input: WaitStoppedInput) -> Result<String, ErrorData> {
        let key = self
            .session_key_for(input.backend.as_deref(), input.cwd.as_deref())
            .await?;
        let body = self
            .invoke(
                &key,
                "wait_stopped",
                json!({ "timeout_ms": input.timeout_ms }),
            )
            .await?;
        to_toon(&body)
    }

    async fn handle_disconnect(&self, input: DisconnectInput) -> Result<String, ErrorData> {
        let key = self
            .session_key_for(input.backend.as_deref(), input.cwd.as_deref())
            .await?;
        let body = self
            .invoke(
                &key,
                "disconnect",
                json!({ "terminate_debuggee": input.terminate_debuggee }),
            )
            .await?;
        *self.last_key.lock().await = None;
        to_toon(&body)
    }

    async fn handle_terminate(&self, input: SessionRefInput) -> Result<String, ErrorData> {
        let key = self
            .session_key_for(input.backend.as_deref(), input.cwd.as_deref())
            .await?;
        let body = self.invoke(&key, "terminate", json!({})).await?;
        to_toon(&body)
    }

    async fn handle_send_raw(&self, input: SendRawInput) -> Result<String, ErrorData> {
        let key = self
            .session_key_for(input.backend.as_deref(), input.cwd.as_deref())
            .await?;
        let body = self
            .invoke(
                &key,
                "send_raw",
                json!({
                    "command": input.command,
                    "arguments": input.arguments.unwrap_or(json!({})),
                }),
            )
            .await?;
        to_toon(&body)
    }
}
