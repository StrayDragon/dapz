//! AgentPool — multi-session DAP handle management for embedded agents.
//!
//! File-level port of lspz `agent_sdk/pool.rs`, adapted for DAP:
//! - Key = **session name** (embedder-chosen), not LSP language id
//! - No workspace root / document sync (those are LSP concepts)
//! - Surface = Tier-0 debug control/observe APIs on [`AgentHandle`]

use std::collections::HashMap;

use serde_json::Value;

use super::AgentHandle;
use crate::error::DapzError;

/// A pool of DAP sessions managed by an embedder-chosen session key.
///
/// Sessions are lazily spawned on the first operation for a given key.
/// All operations delegate to [`AgentHandle`] instances internally.
pub struct AgentPool {
    handles: HashMap<String, AgentHandle>,
    backends: HashMap<String, BackendConfig>,
    compression: bool,
}

struct BackendConfig {
    backend: String,
    backend_args: Vec<String>,
}

impl std::fmt::Debug for AgentPool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AgentPool")
            .field("sessions", &self.backends.keys().collect::<Vec<_>>())
            .field("active_handles", &self.handles.len())
            .field("compression", &self.compression)
            .finish()
    }
}

impl AgentPool {
    /// Create a new [`AgentPoolBuilder`].
    pub fn builder() -> AgentPoolBuilder {
        AgentPoolBuilder::default()
    }

    async fn handle_for(&mut self, session: &str) -> Result<&mut AgentHandle, DapzError> {
        if !self.handles.contains_key(session) {
            let cfg = self.backends.get(session).ok_or_else(|| {
                DapzError::Config(format!("no backend registered for session '{session}'"))
            })?;
            let handle = AgentHandle::builder()
                .backend(&cfg.backend)
                .backend_args(cfg.backend_args.clone())
                .enable_compression(self.compression)
                .start()
                .await?;
            tracing::info!(session, "DAP handle created via pool");
            self.handles.insert(session.to_owned(), handle);
        }
        Ok(self.handles.get_mut(session).expect("just inserted"))
    }

    /// Insert a pre-constructed [`AgentHandle`] for testing.
    pub fn insert_handle(&mut self, session: &str, handle: AgentHandle) {
        self.backends
            .entry(session.to_owned())
            .or_insert_with(|| BackendConfig {
                backend: String::new(),
                backend_args: Vec::new(),
            });
        self.handles.insert(session.to_owned(), handle);
    }

    /// Number of registered session backends (not necessarily spawned).
    pub fn registered_len(&self) -> usize {
        self.backends.len()
    }

    /// Number of live spawned handles.
    pub fn active_len(&self) -> usize {
        self.handles.len()
    }

    /// Launch a program on the given session and wait for first stop.
    pub async fn launch(
        &mut self,
        session: &str,
        program: &str,
        cwd: Option<&str>,
        args: Option<&[String]>,
        breakpoints: Option<&[(String, Vec<i64>)]>,
    ) -> Result<String, DapzError> {
        self.handle_for(session)
            .await?
            .launch(program, cwd, args, breakpoints)
            .await
    }

    /// Set breakpoints on the given session.
    pub async fn set_breakpoints(
        &mut self,
        session: &str,
        path: &str,
        lines: &[i64],
    ) -> Result<String, DapzError> {
        self.handle_for(session)
            .await?
            .set_breakpoints(path, lines)
            .await
    }

    /// Continue execution.
    pub async fn continue_(
        &mut self,
        session: &str,
        thread_id: Option<i64>,
    ) -> Result<String, DapzError> {
        self.handle_for(session).await?.continue_(thread_id).await
    }

    /// Step over.
    pub async fn step_over(
        &mut self,
        session: &str,
        thread_id: Option<i64>,
    ) -> Result<String, DapzError> {
        self.handle_for(session).await?.step_over(thread_id).await
    }

    /// Step into.
    pub async fn step_into(
        &mut self,
        session: &str,
        thread_id: Option<i64>,
    ) -> Result<String, DapzError> {
        self.handle_for(session).await?.step_into(thread_id).await
    }

    /// Step out.
    pub async fn step_out(
        &mut self,
        session: &str,
        thread_id: Option<i64>,
    ) -> Result<String, DapzError> {
        self.handle_for(session).await?.step_out(thread_id).await
    }

    /// Pause.
    pub async fn pause(
        &mut self,
        session: &str,
        thread_id: Option<i64>,
    ) -> Result<String, DapzError> {
        self.handle_for(session).await?.pause(thread_id).await
    }

    /// List threads.
    pub async fn get_threads(&mut self, session: &str) -> Result<String, DapzError> {
        self.handle_for(session).await?.get_threads().await
    }

    /// Stack frames (compressed + TOON when enabled).
    pub async fn get_stack(
        &mut self,
        session: &str,
        thread_id: Option<i64>,
        levels: Option<i64>,
    ) -> Result<String, DapzError> {
        self.handle_for(session)
            .await?
            .get_stack(thread_id, levels)
            .await
    }

    /// Scopes for a frame.
    pub async fn get_scopes(&mut self, session: &str, frame_id: i64) -> Result<String, DapzError> {
        self.handle_for(session).await?.get_scopes(frame_id).await
    }

    /// Variables for a reference.
    pub async fn get_variables(
        &mut self,
        session: &str,
        variables_reference: i64,
    ) -> Result<String, DapzError> {
        self.handle_for(session)
            .await?
            .get_variables(variables_reference)
            .await
    }

    /// Evaluate an expression.
    pub async fn evaluate(
        &mut self,
        session: &str,
        expression: &str,
        frame_id: Option<i64>,
        context: Option<&str>,
    ) -> Result<String, DapzError> {
        self.handle_for(session)
            .await?
            .evaluate(expression, frame_id, context)
            .await
    }

    /// Drain buffered output events.
    pub async fn drain_output(&mut self, session: &str) -> Result<String, DapzError> {
        self.handle_for(session).await?.drain_output().await
    }

    /// Wait for a stopped event.
    pub async fn wait_stopped(
        &mut self,
        session: &str,
        timeout: std::time::Duration,
    ) -> Result<String, DapzError> {
        self.handle_for(session).await?.wait_stopped(timeout).await
    }

    /// Disconnect the session.
    pub async fn disconnect(
        &mut self,
        session: &str,
        terminate_debuggee: Option<bool>,
    ) -> Result<String, DapzError> {
        self.handle_for(session)
            .await?
            .disconnect(terminate_debuggee)
            .await
    }

    /// Terminate the debuggee.
    pub async fn terminate(&mut self, session: &str) -> Result<String, DapzError> {
        self.handle_for(session).await?.terminate().await
    }

    /// Send a raw DAP request (Tier-1 escape hatch).
    pub async fn send_raw(
        &mut self,
        session: &str,
        command: &str,
        arguments: Value,
    ) -> Result<String, DapzError> {
        self.handle_for(session)
            .await?
            .send_raw(command, arguments)
            .await
    }

    /// Disconnect all live sessions (best-effort).
    pub async fn shutdown_all(mut self) -> Result<(), DapzError> {
        let keys: Vec<String> = self.handles.keys().cloned().collect();
        for session in keys {
            if let Some(mut handle) = self.handles.remove(&session)
                && let Err(e) = handle.disconnect(Some(true)).await
            {
                tracing::warn!(session, error = %e, "Failed to disconnect pool handle");
            }
        }
        Ok(())
    }
}

/// Builder for [`AgentPool`].
#[derive(Default)]
pub struct AgentPoolBuilder {
    backends: Vec<(String, String, Vec<String>)>,
    compression: bool,
}

impl AgentPoolBuilder {
    /// Register a DAP backend under a session key (not an LSP language id).
    pub fn register(mut self, session: impl Into<String>, backend: impl Into<String>) -> Self {
        self.backends
            .push((session.into(), backend.into(), Vec::new()));
        self
    }

    /// Register a backend with extra adapter CLI args.
    pub fn register_with_args(
        mut self,
        session: impl Into<String>,
        backend: impl Into<String>,
        args: Vec<String>,
    ) -> Self {
        self.backends.push((session.into(), backend.into(), args));
        self
    }

    /// Enable compression before TOON encoding (default: false on pool; handles inherit).
    pub fn enable_compression(mut self, enabled: bool) -> Self {
        self.compression = enabled;
        self
    }

    /// Build an [`AgentPool`] with registered backends.
    ///
    /// Sessions are **not** spawned eagerly — they are created lazily on first use.
    pub async fn start_all(self) -> Result<AgentPool, DapzError> {
        let mut backends = HashMap::new();
        for (session, backend, backend_args) in self.backends {
            backends.insert(
                session,
                BackendConfig {
                    backend,
                    backend_args,
                },
            );
        }

        Ok(AgentPool {
            handles: HashMap::new(),
            backends,
            compression: self.compression,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codec::json_rpc::DapMessage;
    use crate::mcp::DapSession;
    use crate::transport::mock::MockTransport;
    use serde_json::json;

    fn mock_handle(responses: Vec<DapMessage>) -> AgentHandle {
        let mock = MockTransport::new();
        for msg in responses {
            mock.push_message(msg.to_bytes().unwrap());
        }
        AgentHandle::from_session(DapSession::with_transport(Box::new(mock)), false)
    }

    #[tokio::test]
    async fn test_pool_register_without_spawn() {
        let pool = AgentPool::builder()
            .register("py", "python3 -m debugpy.adapter")
            .register("py2", "python3 -m debugpy.adapter")
            .start_all()
            .await
            .unwrap();
        assert_eq!(pool.registered_len(), 2);
        assert_eq!(pool.active_len(), 0);
    }

    #[tokio::test]
    async fn test_pool_unknown_session() {
        let mut pool = AgentPool::builder().start_all().await.unwrap();
        let err = pool.get_threads("missing").await.unwrap_err();
        assert!(err.to_string().contains("no backend registered"));
    }

    #[tokio::test]
    async fn test_pool_get_stack_via_inserted_handle() {
        let handle = mock_handle(vec![DapMessage {
            seq: 2,
            msg_type: "response".into(),
            command: Some("stackTrace".into()),
            event: None,
            request_seq: Some(1),
            success: Some(true),
            body: Some(json!({
                "stackFrames": [{
                    "id": 1,
                    "name": "bug",
                    "line": 4,
                    "column": 0,
                    "source": {"path": "/tmp/a.py"}
                }]
            })),
            arguments: None,
        }]);

        let mut pool = AgentPool::builder().start_all().await.unwrap();
        pool.insert_handle("py", handle);
        let out = pool.get_stack("py", Some(1), Some(10)).await.unwrap();
        assert!(out.contains("bug") || out.contains("stackFrames") || out.contains("items"));
        assert_eq!(pool.active_len(), 1);
    }
}
