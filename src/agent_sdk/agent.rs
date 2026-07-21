//! Agent SDK — high-level DAP handle for embedded Rust agents.
//!
//! Pattern ported from lspz `agent_sdk/agent.rs` (DAP-adapted).
//! Default: in-process adapter. Opt-in: `via_daemon(true)` to share sessions with MCP.

use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;
use std::time::Duration;

use serde_json::{Value, json};

use crate::codec::json_rpc::DapMessage;
use crate::codec::toon::value_to_toon;
use crate::daemon::DaemonClient;
use crate::daemon::protocol::SpawnParams;
use crate::daemon::resolve_project_cwd;
use crate::error::DapzError;
use crate::interceptors::Interceptor;
use crate::interceptors::evaluate::EvaluateCompressor;
use crate::interceptors::output::OutputCompressor;
use crate::interceptors::scopes::ScopesCompressor;
use crate::interceptors::stacktrace::StackTraceCompressor;
use crate::interceptors::variables::VariablesCompressor;
use crate::mcp::DapSession;
use crate::mcp::pool_key;
use crate::proxy::Direction;

enum Backend {
    Local(DapSession),
    Daemon {
        client: DaemonClient,
        session_key: String,
    },
}

/// High-level handle to a DAP adapter session.
pub struct AgentHandle {
    backend: Backend,
    compression: bool,
}

impl AgentHandle {
    /// Create a new [`AgentBuilder`].
    pub fn builder() -> AgentBuilder {
        AgentBuilder::default()
    }

    /// Wrap an existing [`DapSession`] (unit tests / pool insertion helpers).
    #[cfg(test)]
    pub(crate) fn from_session(session: DapSession, compression: bool) -> Self {
        Self {
            backend: Backend::Local(session),
            compression,
        }
    }

    async fn call_op(
        &mut self,
        op: &str,
        args: Value,
        local: impl FnOnce(
            &mut DapSession,
        )
            -> Pin<Box<dyn Future<Output = Result<Value, DapzError>> + Send + '_>>,
    ) -> Result<Value, DapzError> {
        match &mut self.backend {
            Backend::Local(session) => local(session).await,
            Backend::Daemon {
                client,
                session_key,
            } => client
                .invoke(session_key, op, args)
                .await
                .map_err(|e| DapzError::Protocol(e.to_string())),
        }
    }

    async fn compress_response(&self, command: &str, body: Value) -> Value {
        if !self.compression {
            return body;
        }
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
                Ok(None) => {
                    tracing::warn!(
                        command,
                        "Agent SDK compressor dropped message; forwarding original body"
                    );
                    body
                }
                Err(e) => {
                    tracing::warn!(
                        command,
                        error = %e,
                        "Agent SDK compressor failed; forwarding original body"
                    );
                    body
                }
            }
        } else {
            body
        }
    }

    fn to_out(&self, value: &Value) -> Result<String, DapzError> {
        value_to_toon(value)
    }

    /// Launch a program and wait for first stop.
    pub async fn launch(
        &mut self,
        program: &str,
        cwd: Option<&str>,
        args: Option<&[String]>,
        breakpoints: Option<&[(String, Vec<i64>)]>,
    ) -> Result<String, DapzError> {
        let mut invoke_args = json!({ "program": program });
        if let Some(c) = cwd {
            invoke_args["cwd"] = json!(c);
        }
        if let Some(a) = args {
            invoke_args["args"] = json!(a);
        }
        if let Some(bps) = breakpoints {
            invoke_args["breakpoints"] = json!(
                bps.iter()
                    .map(|(path, lines)| json!({ "path": path, "lines": lines }))
                    .collect::<Vec<_>>()
            );
        }
        let prog = program.to_owned();
        let cwd_owned = cwd.map(str::to_owned);
        let args_owned = args.map(|a| a.to_vec());
        let bps_owned = breakpoints.map(|b| b.to_vec());
        let body = self
            .call_op("launch_program", invoke_args, move |s| {
                Box::pin(async move {
                    s.launch_program(
                        &prog,
                        cwd_owned.as_deref(),
                        args_owned.as_deref(),
                        bps_owned.as_deref(),
                    )
                    .await
                })
            })
            .await?;
        self.to_out(&body)
    }

    /// Set breakpoints.
    pub async fn set_breakpoints(
        &mut self,
        path: &str,
        lines: &[i64],
    ) -> Result<String, DapzError> {
        let path_owned = path.to_owned();
        let lines_owned = lines.to_vec();
        let body = self
            .call_op(
                "set_breakpoints",
                json!({ "source": path, "lines": lines }),
                move |s| {
                    Box::pin(async move { s.set_breakpoints(&path_owned, &lines_owned).await })
                },
            )
            .await?;
        self.to_out(&body)
    }

    /// Continue execution.
    pub async fn continue_(&mut self, thread_id: Option<i64>) -> Result<String, DapzError> {
        let body = self
            .call_op("continue", json!({ "thread_id": thread_id }), move |s| {
                Box::pin(async move { s.continue_(thread_id).await })
            })
            .await?;
        self.to_out(&body)
    }

    /// Step over.
    pub async fn step_over(&mut self, thread_id: Option<i64>) -> Result<String, DapzError> {
        let body = self
            .call_op("step_over", json!({ "thread_id": thread_id }), move |s| {
                Box::pin(async move { s.step_over(thread_id).await })
            })
            .await?;
        self.to_out(&body)
    }

    /// Step into.
    pub async fn step_into(&mut self, thread_id: Option<i64>) -> Result<String, DapzError> {
        let body = self
            .call_op("step_into", json!({ "thread_id": thread_id }), move |s| {
                Box::pin(async move { s.step_into(thread_id).await })
            })
            .await?;
        self.to_out(&body)
    }

    /// Step out.
    pub async fn step_out(&mut self, thread_id: Option<i64>) -> Result<String, DapzError> {
        let body = self
            .call_op("step_out", json!({ "thread_id": thread_id }), move |s| {
                Box::pin(async move { s.step_out(thread_id).await })
            })
            .await?;
        self.to_out(&body)
    }

    /// Pause.
    pub async fn pause(&mut self, thread_id: Option<i64>) -> Result<String, DapzError> {
        let body = self
            .call_op("pause", json!({ "thread_id": thread_id }), move |s| {
                Box::pin(async move { s.pause(thread_id).await })
            })
            .await?;
        self.to_out(&body)
    }

    /// List threads.
    pub async fn get_threads(&mut self) -> Result<String, DapzError> {
        let body = self
            .call_op("get_threads", json!({}), move |s| {
                Box::pin(async move { s.get_threads().await })
            })
            .await?;
        self.to_out(&body)
    }

    /// Stack frames (compressed + TOON when enabled).
    pub async fn get_stack(
        &mut self,
        thread_id: Option<i64>,
        levels: Option<i64>,
    ) -> Result<String, DapzError> {
        let body = self
            .call_op(
                "get_stack",
                json!({ "thread_id": thread_id, "levels": levels }),
                move |s| Box::pin(async move { s.get_stack(thread_id, levels).await }),
            )
            .await?;
        let compressed = self.compress_response("stackTrace", body).await;
        self.to_out(&compressed)
    }

    /// Scopes for a frame.
    pub async fn get_scopes(&mut self, frame_id: i64) -> Result<String, DapzError> {
        let body = self
            .call_op("get_scopes", json!({ "frame_id": frame_id }), move |s| {
                Box::pin(async move { s.get_scopes(frame_id).await })
            })
            .await?;
        let compressed = self.compress_response("scopes", body).await;
        self.to_out(&compressed)
    }

    /// Variables for a reference.
    pub async fn get_variables(&mut self, variables_reference: i64) -> Result<String, DapzError> {
        let body = self
            .call_op(
                "get_variables",
                json!({ "variables_reference": variables_reference }),
                move |s| Box::pin(async move { s.get_variables(variables_reference).await }),
            )
            .await?;
        let compressed = self.compress_response("variables", body).await;
        self.to_out(&compressed)
    }

    /// Evaluate an expression.
    pub async fn evaluate(
        &mut self,
        expression: &str,
        frame_id: Option<i64>,
        context: Option<&str>,
    ) -> Result<String, DapzError> {
        let expr = expression.to_owned();
        let ctx = context.map(str::to_owned);
        let body = self
            .call_op(
                "evaluate",
                json!({
                    "expression": expression,
                    "frame_id": frame_id,
                    "context": context,
                }),
                move |s| Box::pin(async move { s.evaluate(&expr, frame_id, ctx.as_deref()).await }),
            )
            .await?;
        let compressed = self.compress_response("evaluate", body).await;
        self.to_out(&compressed)
    }

    /// Drain buffered output events.
    pub async fn drain_output(&mut self) -> Result<String, DapzError> {
        let body = self
            .call_op("drain_output", json!({}), move |s| {
                Box::pin(async move { Ok(json!({ "outputs": s.drain_output() })) })
            })
            .await?;
        let events = body
            .get("outputs")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();
        let mut compressed = Vec::new();
        for ev in events {
            if self.compression {
                let msg = DapMessage {
                    seq: 1,
                    msg_type: "event".into(),
                    command: None,
                    event: Some("output".into()),
                    request_seq: None,
                    success: None,
                    body: Some(ev.clone()),
                    arguments: None,
                };
                match OutputCompressor
                    .intercept(msg, Direction::ServerToClient)
                    .await
                {
                    Ok(Some(m)) => compressed.push(m.body.unwrap_or(ev)),
                    Ok(None) => {
                        tracing::warn!(
                            "Agent SDK output compressor dropped event; forwarding original"
                        );
                        compressed.push(ev);
                    }
                    Err(e) => {
                        tracing::warn!(
                            error = %e,
                            "Agent SDK output compressor failed; forwarding original"
                        );
                        compressed.push(ev);
                    }
                }
            } else {
                compressed.push(ev);
            }
        }
        self.to_out(&json!({ "outputs": compressed }))
    }

    /// Wait for a stopped event.
    pub async fn wait_stopped(&mut self, timeout: Duration) -> Result<String, DapzError> {
        let timeout_ms = timeout.as_millis() as u64;
        let body = self
            .call_op(
                "wait_stopped",
                json!({ "timeout_ms": timeout_ms }),
                move |s| Box::pin(async move { s.wait_stopped(timeout).await }),
            )
            .await?;
        self.to_out(&body)
    }

    /// Disconnect.
    pub async fn disconnect(
        &mut self,
        terminate_debuggee: Option<bool>,
    ) -> Result<String, DapzError> {
        let body = self
            .call_op(
                "disconnect",
                json!({ "terminate_debuggee": terminate_debuggee }),
                move |s| Box::pin(async move { s.disconnect(terminate_debuggee).await }),
            )
            .await?;
        self.to_out(&body)
    }

    /// Terminate.
    pub async fn terminate(&mut self) -> Result<String, DapzError> {
        let body = self
            .call_op("terminate", json!({}), move |s| {
                Box::pin(async move { s.terminate().await })
            })
            .await?;
        self.to_out(&body)
    }

    /// Send a raw DAP request.
    pub async fn send_raw(&mut self, command: &str, arguments: Value) -> Result<String, DapzError> {
        let cmd = command.to_owned();
        let args = arguments.clone();
        let body = self
            .call_op(
                "send_raw",
                json!({ "command": command, "arguments": arguments }),
                move |s| Box::pin(async move { s.send_raw(&cmd, args).await }),
            )
            .await?;
        self.to_out(&body)
    }

    /// Attach to a running debuggee (adapter-specific attach args).
    pub async fn attach(&mut self, arguments: Value) -> Result<String, DapzError> {
        let args = arguments.clone();
        let body = self
            .call_op("attach", json!({ "arguments": arguments }), move |s| {
                Box::pin(async move { s.attach(args).await })
            })
            .await?;
        self.to_out(&body)
    }

    /// Fetch source by DAP `sourceReference`.
    pub async fn get_source(
        &mut self,
        source_reference: i64,
        path: Option<&str>,
    ) -> Result<String, DapzError> {
        let path_owned = path.map(str::to_owned);
        let body = self
            .call_op(
                "get_source",
                json!({ "source_reference": source_reference, "path": path }),
                move |s| {
                    Box::pin(
                        async move { s.get_source(source_reference, path_owned.as_deref()).await },
                    )
                },
            )
            .await?;
        self.to_out(&body)
    }

    /// Get exception details for a thread.
    pub async fn get_exception(&mut self, thread_id: Option<i64>) -> Result<String, DapzError> {
        let body = self
            .call_op(
                "get_exception",
                json!({ "thread_id": thread_id }),
                move |s| Box::pin(async move { s.get_exception_info(thread_id).await }),
            )
            .await?;
        let body = self.compress_response("exceptionInfo", body).await;
        self.to_out(&body)
    }

    /// Configure exception breakpoints (adapter filter ids).
    pub async fn set_exception_breakpoints(
        &mut self,
        filters: &[String],
    ) -> Result<String, DapzError> {
        let filters_owned = filters.to_vec();
        let body = self
            .call_op(
                "set_exception_breakpoints",
                json!({ "filters": filters }),
                move |s| Box::pin(async move { s.set_exception_breakpoints(&filters_owned).await }),
            )
            .await?;
        self.to_out(&body)
    }
}

/// Builder for [`AgentHandle`].
pub struct AgentBuilder {
    backend: Option<String>,
    backend_args: Vec<String>,
    compression: bool,
    /// Opt-in: route session through dapz daemon (share with MCP). Default false.
    via_daemon: bool,
    /// Project cwd for daemon socket identity (defaults to process cwd).
    cwd: Option<String>,
    /// When set with `via_daemon`, connect to this socket only (no auto-start).
    daemon_socket: Option<PathBuf>,
}

impl AgentBuilder {
    /// Backend adapter command.
    pub fn backend(mut self, cmd: impl Into<String>) -> Self {
        self.backend = Some(cmd.into());
        self
    }

    /// Extra CLI args for the adapter.
    pub fn backend_args(mut self, args: Vec<String>) -> Self {
        self.backend_args = args;
        self
    }

    /// Enable response compression before TOON encoding (default: true).
    pub fn enable_compression(mut self, enabled: bool) -> Self {
        self.compression = enabled;
        self
    }

    /// Route through dapz daemon (opt-in). Default is in-process — DAP sessions are on-demand.
    pub fn via_daemon(mut self, enabled: bool) -> Self {
        self.via_daemon = enabled;
        self
    }

    /// Project cwd used for daemon socket identity (and pool key).
    pub fn cwd(mut self, cwd: impl Into<String>) -> Self {
        self.cwd = Some(cwd.into());
        self
    }

    /// Connect to an existing daemon socket (no auto-start). Implies `via_daemon(true)`.
    pub fn daemon_socket(mut self, path: impl Into<PathBuf>) -> Self {
        self.daemon_socket = Some(path.into());
        self.via_daemon = true;
        self
    }

    /// Start a handle. Daemon connect failures return `Err` (no silent in-process fallback).
    pub async fn start(self) -> Result<AgentHandle, DapzError> {
        let backend = self
            .backend
            .ok_or_else(|| DapzError::Config("backend is required".into()))?;

        if self.via_daemon {
            let mut client = if let Some(ref sock) = self.daemon_socket {
                DaemonClient::connect_explicit(sock).await.map_err(|e| {
                    DapzError::Protocol(format!("Cannot connect to dapz daemon: {e}"))
                })?
            } else {
                let connect_cwd = match &self.cwd {
                    Some(c) => resolve_project_cwd(c),
                    None => {
                        let cur = std::env::current_dir()
                            .map(|p| p.to_string_lossy().into_owned())
                            .unwrap_or_else(|_| ".".into());
                        resolve_project_cwd(&cur)
                    }
                };
                DaemonClient::connect_or_start(&connect_cwd)
                    .await
                    .map_err(|e| {
                        DapzError::Protocol(format!(
                            "Cannot connect to dapz daemon: {e}. Start `dapz daemon` or omit via_daemon."
                        ))
                    })?
            };

            let cwd_for_key = self.cwd.as_deref();
            let session_key = client
                .spawn_session(SpawnParams {
                    backend: backend.clone(),
                    cwd: cwd_for_key.map(str::to_owned),
                    extra_args: self.backend_args.clone(),
                    replace: false,
                })
                .await
                .map_err(|e| DapzError::Protocol(e.to_string()))?;

            tracing::info!(
                %session_key,
                expected = %pool_key(&backend, cwd_for_key),
                "AgentHandle connected via daemon"
            );

            Ok(AgentHandle {
                backend: Backend::Daemon {
                    client,
                    session_key,
                },
                compression: self.compression,
            })
        } else {
            let session = DapSession::spawn_with_args(&backend, &self.backend_args)?;
            Ok(AgentHandle {
                backend: Backend::Local(session),
                compression: self.compression,
            })
        }
    }
}

impl Default for AgentBuilder {
    fn default() -> Self {
        Self {
            backend: None,
            backend_args: Vec::new(),
            compression: true,
            via_daemon: false,
            cwd: None,
            daemon_socket: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transport::mock::MockTransport;
    use serde_json::json;

    #[tokio::test]
    async fn test_agent_get_stack_with_mock() {
        let mock = MockTransport::new();
        mock.push_message(
            DapMessage {
                seq: 2,
                msg_type: "response".into(),
                command: Some("stackTrace".into()),
                event: None,
                request_seq: Some(1),
                success: Some(true),
                body: Some(json!({
                    "stackFrames": [{
                        "id": 1,
                        "name": "main",
                        "line": 10,
                        "column": 0,
                        "source": {"path": "/tmp/a.py"}
                    }]
                })),
                arguments: None,
            }
            .to_bytes()
            .unwrap(),
        );

        let mut agent = AgentHandle::from_session(DapSession::with_transport(Box::new(mock)), true);
        let out = agent.get_stack(Some(1), Some(10)).await.unwrap();
        assert!(out.contains("main") || out.contains("stackFrames") || out.contains("items"));
    }

    #[tokio::test]
    async fn test_via_daemon_connect_failure_no_fallback() {
        let result = AgentHandle::builder()
            .backend("echo")
            .daemon_socket(PathBuf::from("/tmp/dapz-no-such-daemon-socket.sock"))
            .start()
            .await;
        let Err(err) = result else {
            panic!("expected connect failure, got Ok handle");
        };
        let msg = err.to_string();
        assert!(
            msg.contains("Cannot connect") || msg.contains("daemon"),
            "unexpected error: {msg}"
        );
    }

    #[tokio::test]
    async fn test_default_start_is_in_process_shape() {
        // Missing backend still fails with Config — proves via_daemon is not required.
        let result = AgentHandle::builder().start().await;
        let Err(err) = result else {
            panic!("expected Config error");
        };
        assert!(err.to_string().contains("backend"));
    }
}
