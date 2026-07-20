//! Agent SDK — high-level DAP handle for embedded Rust agents.
//!
//! Pattern ported from lspz `agent_sdk/agent.rs` (simplified, Tier-0 only).

use std::time::Duration;

use serde_json::Value;

use crate::codec::json_rpc::DapMessage;
use crate::codec::toon::value_to_toon;
use crate::error::DapzError;
use crate::interceptors::Interceptor;
use crate::interceptors::evaluate::EvaluateCompressor;
use crate::interceptors::output::OutputCompressor;
use crate::interceptors::scopes::ScopesCompressor;
use crate::interceptors::stacktrace::StackTraceCompressor;
use crate::interceptors::variables::VariablesCompressor;
use crate::mcp::DapSession;
use crate::proxy::Direction;

/// High-level handle to a DAP adapter session.
pub struct AgentHandle {
    session: DapSession,
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
            session,
            compression,
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
                _ => body,
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
        let body = self
            .session
            .launch_program(program, cwd, args, breakpoints)
            .await?;
        self.to_out(&body)
    }

    pub async fn set_breakpoints(
        &mut self,
        path: &str,
        lines: &[i64],
    ) -> Result<String, DapzError> {
        let body = self.session.set_breakpoints(path, lines).await?;
        self.to_out(&body)
    }

    pub async fn continue_(&mut self, thread_id: Option<i64>) -> Result<String, DapzError> {
        let body = self.session.continue_(thread_id).await?;
        self.to_out(&body)
    }

    pub async fn step_over(&mut self, thread_id: Option<i64>) -> Result<String, DapzError> {
        let body = self.session.step_over(thread_id).await?;
        self.to_out(&body)
    }

    pub async fn step_into(&mut self, thread_id: Option<i64>) -> Result<String, DapzError> {
        let body = self.session.step_into(thread_id).await?;
        self.to_out(&body)
    }

    pub async fn step_out(&mut self, thread_id: Option<i64>) -> Result<String, DapzError> {
        let body = self.session.step_out(thread_id).await?;
        self.to_out(&body)
    }

    pub async fn pause(&mut self, thread_id: Option<i64>) -> Result<String, DapzError> {
        let body = self.session.pause(thread_id).await?;
        self.to_out(&body)
    }

    pub async fn get_threads(&mut self) -> Result<String, DapzError> {
        let body = self.session.get_threads().await?;
        self.to_out(&body)
    }

    pub async fn get_stack(
        &mut self,
        thread_id: Option<i64>,
        levels: Option<i64>,
    ) -> Result<String, DapzError> {
        let body = self.session.get_stack(thread_id, levels).await?;
        let compressed = self.compress_response("stackTrace", body).await;
        self.to_out(&compressed)
    }

    pub async fn get_scopes(&mut self, frame_id: i64) -> Result<String, DapzError> {
        let body = self.session.get_scopes(frame_id).await?;
        let compressed = self.compress_response("scopes", body).await;
        self.to_out(&compressed)
    }

    pub async fn get_variables(&mut self, variables_reference: i64) -> Result<String, DapzError> {
        let body = self.session.get_variables(variables_reference).await?;
        let compressed = self.compress_response("variables", body).await;
        self.to_out(&compressed)
    }

    pub async fn evaluate(
        &mut self,
        expression: &str,
        frame_id: Option<i64>,
        context: Option<&str>,
    ) -> Result<String, DapzError> {
        let body = self.session.evaluate(expression, frame_id, context).await?;
        let compressed = self.compress_response("evaluate", body).await;
        self.to_out(&compressed)
    }

    pub async fn drain_output(&mut self) -> Result<String, DapzError> {
        let events = self.session.drain_output();
        let mut compressed = Vec::new();
        for body in events {
            if self.compression {
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
            } else {
                compressed.push(body);
            }
        }
        self.to_out(&serde_json::json!({ "outputs": compressed }))
    }

    pub async fn wait_stopped(&mut self, timeout: Duration) -> Result<String, DapzError> {
        let body = self.session.wait_stopped(timeout).await?;
        self.to_out(&body)
    }

    pub async fn disconnect(
        &mut self,
        terminate_debuggee: Option<bool>,
    ) -> Result<String, DapzError> {
        let body = self.session.disconnect(terminate_debuggee).await?;
        self.to_out(&body)
    }

    pub async fn terminate(&mut self) -> Result<String, DapzError> {
        let body = self.session.terminate().await?;
        self.to_out(&body)
    }

    pub async fn send_raw(&mut self, command: &str, arguments: Value) -> Result<String, DapzError> {
        let body = self.session.send_raw(command, arguments).await?;
        self.to_out(&body)
    }

    /// Attach to a running debuggee (adapter-specific attach args).
    pub async fn attach(&mut self, arguments: Value) -> Result<String, DapzError> {
        let body = self.session.attach(arguments).await?;
        self.to_out(&body)
    }

    /// Fetch source by DAP `sourceReference`.
    pub async fn get_source(
        &mut self,
        source_reference: i64,
        path: Option<&str>,
    ) -> Result<String, DapzError> {
        let body = self.session.get_source(source_reference, path).await?;
        self.to_out(&body)
    }

    /// Get exception details for a thread.
    pub async fn get_exception(&mut self, thread_id: Option<i64>) -> Result<String, DapzError> {
        let body = self.session.get_exception_info(thread_id).await?;
        let body = self.compress_response("exceptionInfo", body).await;
        self.to_out(&body)
    }

    /// Configure exception breakpoints (adapter filter ids).
    pub async fn set_exception_breakpoints(
        &mut self,
        filters: &[String],
    ) -> Result<String, DapzError> {
        let body = self.session.set_exception_breakpoints(filters).await?;
        self.to_out(&body)
    }
}

/// Builder for [`AgentHandle`].
pub struct AgentBuilder {
    backend: Option<String>,
    backend_args: Vec<String>,
    compression: bool,
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

    /// Spawn the adapter and return a handle (does not initialize/launch yet).
    pub async fn start(self) -> Result<AgentHandle, DapzError> {
        let backend = self
            .backend
            .ok_or_else(|| DapzError::Config("backend is required".into()))?;
        let session = DapSession::spawn_with_args(&backend, &self.backend_args)?;
        Ok(AgentHandle {
            session,
            compression: self.compression,
        })
    }
}

impl Default for AgentBuilder {
    fn default() -> Self {
        Self {
            backend: None,
            backend_args: Vec::new(),
            compression: true,
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
}
