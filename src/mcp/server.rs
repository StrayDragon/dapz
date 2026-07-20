//! MCP server — expose Tier-0 DAP operations as tools.
//!
//! Pattern ported from lspz `mcp/server.rs` (rmcp 2.x ServerHandler).

use std::sync::Arc;
use std::time::Duration;

use rmcp::{
    ErrorData, ServerHandler,
    model::{
        CallToolRequestParams, CallToolResult, ContentBlock, ListToolsResult,
        PaginatedRequestParams, ServerCapabilities, ServerInfo, Tool,
    },
    service::{RequestContext, RoleServer},
};
use serde_json::{Value, json};
use tokio::sync::Mutex;
use tracing::info;

use super::pool::{DapPool, pool_key};
use super::session::DapSession;
use crate::adapters::{lookup_by_extension, lookup_by_language};
use crate::codec::json_rpc::DapMessage;
use crate::codec::toon::value_to_toon;
use crate::interceptors::Interceptor;
use crate::interceptors::evaluate::EvaluateCompressor;
use crate::interceptors::output::OutputCompressor;
use crate::interceptors::scopes::ScopesCompressor;
use crate::interceptors::stacktrace::StackTraceCompressor;
use crate::interceptors::variables::VariablesCompressor;
use crate::proxy::Direction;

// ─── Tool Definitions ──────────────────────────────────────────────────────

pub(crate) fn tool_definitions() -> Vec<Tool> {
    vec![
        Tool::new(
            "debug_launch",
            "Start a DAP debug session: initialize, launch program, optional breakpoints, \
             configurationDone, wait for first stopped. Returns TOON.",
            rmcp::model::object(DebugLaunchInput::json_schema()),
        ),
        Tool::new(
            "debug_attach",
            "Attach to a running debuggee (DAP attach). Arguments are adapter-specific JSON. Returns TOON.",
            rmcp::model::object(DebugAttachInput::json_schema()),
        ),
        Tool::new(
            "set_breakpoints",
            "Set breakpoints on a source file in the active session.",
            rmcp::model::object(SetBreakpointsInput::json_schema()),
        ),
        Tool::new(
            "set_exception_breakpoints",
            "Set exception breakpoints via DAP filters (e.g. debugpy raised/uncaught).",
            rmcp::model::object(SetExceptionBreakpointsInput::json_schema()),
        ),
        Tool::new(
            "continue",
            "Continue execution; waits for next stopped/terminated. Returns TOON.",
            rmcp::model::object(ThreadInput::json_schema()),
        ),
        Tool::new(
            "step_over",
            "Step over (DAP next); waits for stopped/terminated.",
            rmcp::model::object(ThreadInput::json_schema()),
        ),
        Tool::new(
            "step_into",
            "Step into; waits for stopped/terminated.",
            rmcp::model::object(ThreadInput::json_schema()),
        ),
        Tool::new(
            "step_out",
            "Step out; waits for stopped/terminated.",
            rmcp::model::object(ThreadInput::json_schema()),
        ),
        Tool::new(
            "pause",
            "Pause the debuggee; waits for stopped.",
            rmcp::model::object(ThreadInput::json_schema()),
        ),
        Tool::new(
            "get_threads",
            "List threads (DAP threads).",
            rmcp::model::object(SessionRefInput::json_schema()),
        ),
        Tool::new(
            "get_stack",
            "Get stackTrace for a thread (compressed TOON).",
            rmcp::model::object(GetStackInput::json_schema()),
        ),
        Tool::new(
            "get_scopes",
            "Get scopes for a stack frame (compressed TOON).",
            rmcp::model::object(GetScopesInput::json_schema()),
        ),
        Tool::new(
            "get_variables",
            "Get variables for a variablesReference (compressed TOON).",
            rmcp::model::object(GetVariablesInput::json_schema()),
        ),
        Tool::new(
            "evaluate",
            "Evaluate an expression in a frame (compressed TOON).",
            rmcp::model::object(EvaluateInput::json_schema()),
        ),
        Tool::new(
            "get_exception",
            "Get exceptionInfo for a thread (compressed TOON). DAP-specific — not LSP diagnostics.",
            rmcp::model::object(ThreadInput::json_schema()),
        ),
        Tool::new(
            "get_source",
            "Fetch source content via DAP sourceReference.",
            rmcp::model::object(GetSourceInput::json_schema()),
        ),
        Tool::new(
            "get_output",
            "Drain buffered output events (compressed TOON).",
            rmcp::model::object(SessionRefInput::json_schema()),
        ),
        Tool::new(
            "wait_stopped",
            "Wait for the next stopped event.",
            rmcp::model::object(WaitStoppedInput::json_schema()),
        ),
        Tool::new(
            "disconnect",
            "Disconnect the debug session.",
            rmcp::model::object(DisconnectInput::json_schema()),
        ),
        Tool::new(
            "terminate",
            "Terminate the debuggee.",
            rmcp::model::object(SessionRefInput::json_schema()),
        ),
        Tool::new(
            "send_raw",
            "Escape hatch: send any DAP command (Tier-1 passthrough).",
            rmcp::model::object(SendRawInput::json_schema()),
        ),
    ]
}

#[cfg(test)]
fn tool_count() -> usize {
    tool_definitions().len()
}

// ─── Input Types ───────────────────────────────────────────────────────────

pub(crate) trait JsonSchema {
    fn json_schema() -> Value;
}

#[derive(Debug, serde::Deserialize)]
pub(crate) struct DebugLaunchInput {
    pub program: String,
    pub cwd: Option<String>,
    pub args: Option<Vec<String>>,
    pub backend: Option<String>,
    pub language: Option<String>,
    /// Breakpoints: list of { path, lines }
    pub breakpoints: Option<Vec<BreakpointSpec>>,
    pub backend_args: Option<Vec<String>>,
}

#[derive(Debug, serde::Deserialize)]
pub(crate) struct BreakpointSpec {
    pub path: String,
    pub lines: Vec<i64>,
}

#[derive(Debug, serde::Deserialize)]
pub(crate) struct DebugAttachInput {
    /// Adapter-specific attach arguments (passed as DAP attach body).
    pub arguments: Value,
    pub backend: Option<String>,
    pub language: Option<String>,
    pub cwd: Option<String>,
    pub backend_args: Option<Vec<String>>,
}

#[derive(Debug, serde::Deserialize)]
pub(crate) struct SetExceptionBreakpointsInput {
    pub backend: Option<String>,
    pub cwd: Option<String>,
    pub filters: Vec<String>,
}

#[derive(Debug, serde::Deserialize)]
pub(crate) struct GetSourceInput {
    pub backend: Option<String>,
    pub cwd: Option<String>,
    pub source_reference: i64,
    pub path: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
pub(crate) struct SessionRefInput {
    pub backend: Option<String>,
    pub cwd: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
pub(crate) struct ThreadInput {
    pub backend: Option<String>,
    pub cwd: Option<String>,
    pub thread_id: Option<i64>,
}

#[derive(Debug, serde::Deserialize)]
pub(crate) struct SetBreakpointsInput {
    pub backend: Option<String>,
    pub cwd: Option<String>,
    pub source: String,
    pub lines: Vec<i64>,
}

#[derive(Debug, serde::Deserialize)]
pub(crate) struct GetStackInput {
    pub backend: Option<String>,
    pub cwd: Option<String>,
    pub thread_id: Option<i64>,
    pub levels: Option<i64>,
}

#[derive(Debug, serde::Deserialize)]
pub(crate) struct GetScopesInput {
    pub backend: Option<String>,
    pub cwd: Option<String>,
    pub frame_id: i64,
}

#[derive(Debug, serde::Deserialize)]
pub(crate) struct GetVariablesInput {
    pub backend: Option<String>,
    pub cwd: Option<String>,
    pub variables_reference: i64,
}

#[derive(Debug, serde::Deserialize)]
pub(crate) struct EvaluateInput {
    pub backend: Option<String>,
    pub cwd: Option<String>,
    pub expression: String,
    pub frame_id: Option<i64>,
    pub context: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
pub(crate) struct WaitStoppedInput {
    pub backend: Option<String>,
    pub cwd: Option<String>,
    pub timeout_ms: Option<u64>,
}

#[derive(Debug, serde::Deserialize)]
pub(crate) struct DisconnectInput {
    pub backend: Option<String>,
    pub cwd: Option<String>,
    pub terminate_debuggee: Option<bool>,
}

#[derive(Debug, serde::Deserialize)]
pub(crate) struct SendRawInput {
    pub backend: Option<String>,
    pub cwd: Option<String>,
    pub command: String,
    pub arguments: Option<Value>,
}

impl JsonSchema for DebugLaunchInput {
    fn json_schema() -> Value {
        json!({
            "type": "object",
            "properties": {
                "program": { "type": "string", "description": "Path to program to debug" },
                "cwd": { "type": "string" },
                "args": { "type": "array", "items": { "type": "string" } },
                "backend": { "type": "string", "description": "Adapter command; default debugpy for .py" },
                "language": { "type": "string" },
                "breakpoints": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "properties": {
                            "path": { "type": "string" },
                            "lines": { "type": "array", "items": { "type": "integer" } }
                        },
                        "required": ["path", "lines"]
                    }
                },
                "backend_args": { "type": "array", "items": { "type": "string" } }
            },
            "required": ["program"]
        })
    }
}

impl JsonSchema for DebugAttachInput {
    fn json_schema() -> Value {
        json!({
            "type": "object",
            "properties": {
                "arguments": { "type": "object", "description": "DAP attach arguments (adapter-specific)" },
                "backend": { "type": "string" },
                "language": { "type": "string" },
                "cwd": { "type": "string" },
                "backend_args": { "type": "array", "items": { "type": "string" } }
            },
            "required": ["arguments"]
        })
    }
}

impl JsonSchema for SetExceptionBreakpointsInput {
    fn json_schema() -> Value {
        json!({
            "type": "object",
            "properties": {
                "backend": { "type": "string" },
                "cwd": { "type": "string" },
                "filters": { "type": "array", "items": { "type": "string" } }
            },
            "required": ["filters"]
        })
    }
}

impl JsonSchema for GetSourceInput {
    fn json_schema() -> Value {
        json!({
            "type": "object",
            "properties": {
                "backend": { "type": "string" },
                "cwd": { "type": "string" },
                "source_reference": { "type": "integer" },
                "path": { "type": "string" }
            },
            "required": ["source_reference"]
        })
    }
}

impl JsonSchema for SessionRefInput {
    fn json_schema() -> Value {
        json!({
            "type": "object",
            "properties": {
                "backend": { "type": "string" },
                "cwd": { "type": "string" }
            }
        })
    }
}

impl JsonSchema for ThreadInput {
    fn json_schema() -> Value {
        json!({
            "type": "object",
            "properties": {
                "backend": { "type": "string" },
                "cwd": { "type": "string" },
                "thread_id": { "type": "integer" }
            }
        })
    }
}

impl JsonSchema for SetBreakpointsInput {
    fn json_schema() -> Value {
        json!({
            "type": "object",
            "properties": {
                "backend": { "type": "string" },
                "cwd": { "type": "string" },
                "source": { "type": "string" },
                "lines": { "type": "array", "items": { "type": "integer" } }
            },
            "required": ["source", "lines"]
        })
    }
}

impl JsonSchema for GetStackInput {
    fn json_schema() -> Value {
        json!({
            "type": "object",
            "properties": {
                "backend": { "type": "string" },
                "cwd": { "type": "string" },
                "thread_id": { "type": "integer" },
                "levels": { "type": "integer" }
            }
        })
    }
}

impl JsonSchema for GetScopesInput {
    fn json_schema() -> Value {
        json!({
            "type": "object",
            "properties": {
                "backend": { "type": "string" },
                "cwd": { "type": "string" },
                "frame_id": { "type": "integer" }
            },
            "required": ["frame_id"]
        })
    }
}

impl JsonSchema for GetVariablesInput {
    fn json_schema() -> Value {
        json!({
            "type": "object",
            "properties": {
                "backend": { "type": "string" },
                "cwd": { "type": "string" },
                "variables_reference": { "type": "integer" }
            },
            "required": ["variables_reference"]
        })
    }
}

impl JsonSchema for EvaluateInput {
    fn json_schema() -> Value {
        json!({
            "type": "object",
            "properties": {
                "backend": { "type": "string" },
                "cwd": { "type": "string" },
                "expression": { "type": "string" },
                "frame_id": { "type": "integer" },
                "context": { "type": "string" }
            },
            "required": ["expression"]
        })
    }
}

impl JsonSchema for WaitStoppedInput {
    fn json_schema() -> Value {
        json!({
            "type": "object",
            "properties": {
                "backend": { "type": "string" },
                "cwd": { "type": "string" },
                "timeout_ms": { "type": "integer" }
            }
        })
    }
}

impl JsonSchema for DisconnectInput {
    fn json_schema() -> Value {
        json!({
            "type": "object",
            "properties": {
                "backend": { "type": "string" },
                "cwd": { "type": "string" },
                "terminate_debuggee": { "type": "boolean" }
            }
        })
    }
}

impl JsonSchema for SendRawInput {
    fn json_schema() -> Value {
        json!({
            "type": "object",
            "properties": {
                "backend": { "type": "string" },
                "cwd": { "type": "string" },
                "command": { "type": "string" },
                "arguments": { "type": "object" }
            },
            "required": ["command"]
        })
    }
}

// ─── Compression helpers ───────────────────────────────────────────────────

pub(crate) async fn compress_response(command: &str, body: Value) -> Value {
    let msg = DapMessage {
        seq: 1,
        msg_type: "response".into(),
        command: Some(command.into()),
        event: None,
        request_seq: Some(0),
        success: Some(true),
        body: Some(body.clone()),
        arguments: None,
    };
    let interceptor: Option<Box<dyn Interceptor>> = match command {
        "stackTrace" => Some(Box::new(StackTraceCompressor)),
        "scopes" => Some(Box::new(ScopesCompressor)),
        "variables" => Some(Box::new(VariablesCompressor::new(120))),
        "evaluate" => Some(Box::new(EvaluateCompressor::new(500))),
        "exceptionInfo" => Some(Box::new(
            crate::interceptors::exception::ExceptionInfoCompressor::default(),
        )),
        _ => None,
    };
    if let Some(interceptor) = interceptor {
        match interceptor.intercept(msg, Direction::ServerToClient).await {
            Ok(Some(m)) => m.body.unwrap_or(body),
            _ => body,
        }
    } else {
        body
    }
}

pub(crate) async fn compress_output_events(events: Vec<Value>) -> Value {
    let mut compressed = Vec::new();
    for body in events {
        let msg = DapMessage {
            seq: 1,
            msg_type: "event".into(),
            command: None,
            event: Some("output".into()),
            request_seq: None,
            success: None,
            body: Some(body.clone()),
            arguments: None,
        };
        match OutputCompressor
            .intercept(msg, Direction::ServerToClient)
            .await
        {
            Ok(Some(m)) => compressed.push(m.body.unwrap_or(body)),
            _ => compressed.push(body),
        }
    }
    json!({ "outputs": compressed })
}

pub(crate) fn to_toon(value: &Value) -> Result<String, ErrorData> {
    value_to_toon(value).map_err(|e| ErrorData::internal_error(e.to_string(), None))
}

pub(crate) fn parse_args<T: serde::de::DeserializeOwned>(
    arguments: Option<Value>,
) -> Result<T, ErrorData> {
    let obj = match arguments {
        Some(Value::Object(map)) => Value::Object(map),
        Some(other) => other,
        None => json!({}),
    };
    serde_json::from_value(obj).map_err(|e| ErrorData::invalid_request(e.to_string(), None))
}

pub(crate) fn resolve_backend(
    program: Option<&str>,
    language: Option<&str>,
    backend: Option<&str>,
) -> Result<String, ErrorData> {
    if let Some(b) = backend {
        return Ok(b.to_string());
    }
    if let Some(lang) = language
        && let Some(info) = lookup_by_language(lang)
    {
        return Ok(info.backend.to_string());
    }
    if let Some(program) = program
        && let Some(ext) = std::path::Path::new(program)
            .extension()
            .and_then(|e| e.to_str())
        && let Some(info) = lookup_by_extension(ext)
    {
        return Ok(info.backend.to_string());
    }
    Err(ErrorData::invalid_request(
        "Cannot resolve backend. Pass `backend` or use a .py program / language=python.",
        None,
    ))
}

// ─── Server ────────────────────────────────────────────────────────────────

/// Backend command + optional cwd.
type BackendCwd = (String, Option<String>);
/// Last `(backend, cwd)` used by `debug_launch`.
type LastSessionKey = Option<BackendCwd>;

/// MCP server exposing Tier-0 DAP tools.
#[derive(Clone)]
pub struct McpServer {
    pool: Arc<Mutex<DapPool>>,
    /// Last backend/cwd used by debug_launch (so follow-up tools can omit them).
    last_key: Arc<Mutex<LastSessionKey>>,
}

impl McpServer {
    /// Create a new MCP server with an empty session pool.
    pub fn new() -> Self {
        Self {
            pool: Arc::new(Mutex::new(DapPool::new())),
            last_key: Arc::new(Mutex::new(None)),
        }
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

    async fn get_session(
        &self,
        backend: &str,
        cwd: Option<&str>,
    ) -> Result<Arc<Mutex<DapSession>>, ErrorData> {
        let key = pool_key(backend, cwd);
        let pool = self.pool.lock().await;
        pool.get_by_key(&key).ok_or_else(|| {
            ErrorData::invalid_request(
                format!("No session for key '{key}'. Call debug_launch first."),
                None,
            )
        })
    }
}

impl Default for McpServer {
    fn default() -> Self {
        Self::new()
    }
}

impl ServerHandler for McpServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build()).with_instructions(
            "dapz MCP server — Tier-0 DAP debug tools for agents. \
             Flow: debug_launch → get_stack → get_scopes → get_variables → evaluate → step/continue → disconnect. \
             Outputs are TOON. Use send_raw for Tier-1 DAP commands.",
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

        info!(tool = %name, "MCP tool completed");
        Ok(CallToolResult::success(vec![ContentBlock::text(text)]))
    }
}

impl McpServer {
    async fn handle_debug_launch(&self, input: DebugLaunchInput) -> Result<String, ErrorData> {
        let backend = resolve_backend(
            Some(&input.program),
            input.language.as_deref(),
            input.backend.as_deref(),
        )?;
        let cwd = input.cwd.as_deref();
        let extra = input.backend_args.unwrap_or_default();

        let session = {
            let mut pool = self.pool.lock().await;
            let key = pool_key(&backend, cwd);
            pool.remove(&key);
            pool.get_or_spawn(&backend, cwd, &extra)
                .await
                .map_err(|e| ErrorData::internal_error(e.to_string(), None))?
        };

        let bp_owned: Option<Vec<(String, Vec<i64>)>> = input
            .breakpoints
            .map(|list| list.into_iter().map(|b| (b.path, b.lines)).collect());

        let mut guard = session.lock().await;
        let stopped = guard
            .launch_program(
                &input.program,
                cwd,
                input.args.as_deref(),
                bp_owned.as_deref(),
            )
            .await
            .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
        drop(guard);

        self.remember(&backend, cwd).await;
        to_toon(&json!({
            "ok": true,
            "backend": backend,
            "cwd": cwd,
            "stopped": stopped,
        }))
    }

    async fn handle_debug_attach(&self, input: DebugAttachInput) -> Result<String, ErrorData> {
        let backend = resolve_backend(None, input.language.as_deref(), input.backend.as_deref())?;
        let cwd = input.cwd.as_deref();
        let extra = input.backend_args.unwrap_or_default();
        let session = {
            let mut pool = self.pool.lock().await;
            let key = pool_key(&backend, cwd);
            pool.remove(&key);
            pool.get_or_spawn(&backend, cwd, &extra)
                .await
                .map_err(|e| ErrorData::internal_error(e.to_string(), None))?
        };
        let mut guard = session.lock().await;
        let body = guard
            .attach(input.arguments)
            .await
            .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
        drop(guard);
        self.remember(&backend, cwd).await;
        to_toon(&json!({
            "ok": true,
            "backend": backend,
            "cwd": cwd,
            "attach": body,
        }))
    }

    async fn handle_set_breakpoints(
        &self,
        input: SetBreakpointsInput,
    ) -> Result<String, ErrorData> {
        let (backend, cwd) = self
            .resolve_session_key(input.backend.as_deref(), input.cwd.as_deref())
            .await?;
        let session = self.get_session(&backend, cwd.as_deref()).await?;
        let mut s = session.lock().await;
        let body = s
            .set_breakpoints(&input.source, &input.lines)
            .await
            .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
        to_toon(&body)
    }

    async fn handle_continue(&self, input: ThreadInput) -> Result<String, ErrorData> {
        let (backend, cwd) = self
            .resolve_session_key(input.backend.as_deref(), input.cwd.as_deref())
            .await?;
        let session = self.get_session(&backend, cwd.as_deref()).await?;
        let mut s = session.lock().await;
        let body = s
            .continue_(input.thread_id)
            .await
            .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
        to_toon(&body)
    }

    async fn handle_step_over(&self, input: ThreadInput) -> Result<String, ErrorData> {
        let (backend, cwd) = self
            .resolve_session_key(input.backend.as_deref(), input.cwd.as_deref())
            .await?;
        let session = self.get_session(&backend, cwd.as_deref()).await?;
        let mut s = session.lock().await;
        let body = s
            .step_over(input.thread_id)
            .await
            .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
        to_toon(&body)
    }

    async fn handle_step_into(&self, input: ThreadInput) -> Result<String, ErrorData> {
        let (backend, cwd) = self
            .resolve_session_key(input.backend.as_deref(), input.cwd.as_deref())
            .await?;
        let session = self.get_session(&backend, cwd.as_deref()).await?;
        let mut s = session.lock().await;
        let body = s
            .step_into(input.thread_id)
            .await
            .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
        to_toon(&body)
    }

    async fn handle_step_out(&self, input: ThreadInput) -> Result<String, ErrorData> {
        let (backend, cwd) = self
            .resolve_session_key(input.backend.as_deref(), input.cwd.as_deref())
            .await?;
        let session = self.get_session(&backend, cwd.as_deref()).await?;
        let mut s = session.lock().await;
        let body = s
            .step_out(input.thread_id)
            .await
            .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
        to_toon(&body)
    }

    async fn handle_pause(&self, input: ThreadInput) -> Result<String, ErrorData> {
        let (backend, cwd) = self
            .resolve_session_key(input.backend.as_deref(), input.cwd.as_deref())
            .await?;
        let session = self.get_session(&backend, cwd.as_deref()).await?;
        let mut s = session.lock().await;
        let body = s
            .pause(input.thread_id)
            .await
            .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
        to_toon(&body)
    }

    async fn handle_get_threads(&self, input: SessionRefInput) -> Result<String, ErrorData> {
        let (backend, cwd) = self
            .resolve_session_key(input.backend.as_deref(), input.cwd.as_deref())
            .await?;
        let session = self.get_session(&backend, cwd.as_deref()).await?;
        let mut s = session.lock().await;
        let body = s
            .get_threads()
            .await
            .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
        to_toon(&body)
    }

    async fn handle_get_stack(&self, input: GetStackInput) -> Result<String, ErrorData> {
        let (backend, cwd) = self
            .resolve_session_key(input.backend.as_deref(), input.cwd.as_deref())
            .await?;
        let session = self.get_session(&backend, cwd.as_deref()).await?;
        let mut s = session.lock().await;
        let body = s
            .get_stack(input.thread_id, input.levels)
            .await
            .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
        let compressed = compress_response("stackTrace", body).await;
        to_toon(&compressed)
    }

    async fn handle_get_scopes(&self, input: GetScopesInput) -> Result<String, ErrorData> {
        let (backend, cwd) = self
            .resolve_session_key(input.backend.as_deref(), input.cwd.as_deref())
            .await?;
        let session = self.get_session(&backend, cwd.as_deref()).await?;
        let mut s = session.lock().await;
        let body = s
            .get_scopes(input.frame_id)
            .await
            .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
        let compressed = compress_response("scopes", body).await;
        to_toon(&compressed)
    }

    async fn handle_get_variables(&self, input: GetVariablesInput) -> Result<String, ErrorData> {
        let (backend, cwd) = self
            .resolve_session_key(input.backend.as_deref(), input.cwd.as_deref())
            .await?;
        let session = self.get_session(&backend, cwd.as_deref()).await?;
        let mut s = session.lock().await;
        let body = s
            .get_variables(input.variables_reference)
            .await
            .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
        let compressed = compress_response("variables", body).await;
        to_toon(&compressed)
    }

    async fn handle_evaluate(&self, input: EvaluateInput) -> Result<String, ErrorData> {
        let (backend, cwd) = self
            .resolve_session_key(input.backend.as_deref(), input.cwd.as_deref())
            .await?;
        let session = self.get_session(&backend, cwd.as_deref()).await?;
        let mut s = session.lock().await;
        let body = s
            .evaluate(&input.expression, input.frame_id, input.context.as_deref())
            .await
            .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
        let compressed = compress_response("evaluate", body).await;
        to_toon(&compressed)
    }

    async fn handle_get_exception(&self, input: ThreadInput) -> Result<String, ErrorData> {
        let (backend, cwd) = self
            .resolve_session_key(input.backend.as_deref(), input.cwd.as_deref())
            .await?;
        let session = self.get_session(&backend, cwd.as_deref()).await?;
        let mut s = session.lock().await;
        let body = s
            .get_exception_info(input.thread_id)
            .await
            .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
        let compressed = compress_response("exceptionInfo", body).await;
        to_toon(&compressed)
    }

    async fn handle_get_source(&self, input: GetSourceInput) -> Result<String, ErrorData> {
        let (backend, cwd) = self
            .resolve_session_key(input.backend.as_deref(), input.cwd.as_deref())
            .await?;
        let session = self.get_session(&backend, cwd.as_deref()).await?;
        let mut s = session.lock().await;
        let body = s
            .get_source(input.source_reference, input.path.as_deref())
            .await
            .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
        to_toon(&body)
    }

    async fn handle_set_exception_breakpoints(
        &self,
        input: SetExceptionBreakpointsInput,
    ) -> Result<String, ErrorData> {
        let (backend, cwd) = self
            .resolve_session_key(input.backend.as_deref(), input.cwd.as_deref())
            .await?;
        let session = self.get_session(&backend, cwd.as_deref()).await?;
        let mut s = session.lock().await;
        let body = s
            .set_exception_breakpoints(&input.filters)
            .await
            .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
        to_toon(&body)
    }

    async fn handle_get_output(&self, input: SessionRefInput) -> Result<String, ErrorData> {
        let (backend, cwd) = self
            .resolve_session_key(input.backend.as_deref(), input.cwd.as_deref())
            .await?;
        let session = self.get_session(&backend, cwd.as_deref()).await?;
        let mut s = session.lock().await;
        let events = s.drain_output();
        let compressed = compress_output_events(events).await;
        to_toon(&compressed)
    }

    async fn handle_wait_stopped(&self, input: WaitStoppedInput) -> Result<String, ErrorData> {
        let (backend, cwd) = self
            .resolve_session_key(input.backend.as_deref(), input.cwd.as_deref())
            .await?;
        let timeout = Duration::from_millis(input.timeout_ms.unwrap_or(30_000));
        let session = self.get_session(&backend, cwd.as_deref()).await?;
        let mut s = session.lock().await;
        let body = s
            .wait_stopped(timeout)
            .await
            .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
        to_toon(&body)
    }

    async fn handle_disconnect(&self, input: DisconnectInput) -> Result<String, ErrorData> {
        let (backend, cwd) = self
            .resolve_session_key(input.backend.as_deref(), input.cwd.as_deref())
            .await?;
        let key = pool_key(&backend, cwd.as_deref());
        let session = self.get_session(&backend, cwd.as_deref()).await?;
        let mut s = session.lock().await;
        let body = s
            .disconnect(input.terminate_debuggee)
            .await
            .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
        let text = to_toon(&body)?;
        drop(s);
        self.pool.lock().await.remove(&key);
        *self.last_key.lock().await = None;
        Ok(text)
    }

    async fn handle_terminate(&self, input: SessionRefInput) -> Result<String, ErrorData> {
        let (backend, cwd) = self
            .resolve_session_key(input.backend.as_deref(), input.cwd.as_deref())
            .await?;
        let session = self.get_session(&backend, cwd.as_deref()).await?;
        let mut s = session.lock().await;
        let body = s
            .terminate()
            .await
            .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
        to_toon(&body)
    }

    async fn handle_send_raw(&self, input: SendRawInput) -> Result<String, ErrorData> {
        let (backend, cwd) = self
            .resolve_session_key(input.backend.as_deref(), input.cwd.as_deref())
            .await?;
        let session = self.get_session(&backend, cwd.as_deref()).await?;
        let mut s = session.lock().await;
        let body = s
            .send_raw(&input.command, input.arguments.unwrap_or(json!({})))
            .await
            .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
        to_toon(&body)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_definitions_count() {
        assert_eq!(tool_count(), 21);
        let tools = tool_definitions();
        assert_eq!(tools.len(), 21);
        let names: Vec<_> = tools.iter().map(|t| t.name.as_ref()).collect();
        assert!(names.contains(&"debug_launch"));
        assert!(names.contains(&"debug_attach"));
        assert!(names.contains(&"get_stack"));
        assert!(names.contains(&"get_exception"));
        assert!(names.contains(&"get_source"));
        assert!(names.contains(&"set_exception_breakpoints"));
        assert!(names.contains(&"send_raw"));
    }
}
