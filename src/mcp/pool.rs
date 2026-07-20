//! DAP session pool — reuse adapter processes by backend+cwd key.
//!
//! Pattern ported from lspz `mcp/pool.rs` (simplified: no document tracking).

use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::Mutex;

use super::session::DapSession;
use crate::error::DapzError;

/// Build a stable pool key from backend command and working directory.
pub fn pool_key(backend: &str, cwd: Option<&str>) -> String {
    match cwd {
        Some(c) if !c.is_empty() => format!("{backend}::{c}"),
        _ => backend.to_string(),
    }
}

/// Pool of [`DapSession`]s keyed by [`pool_key`].
#[derive(Default)]
pub struct DapPool {
    sessions: HashMap<String, Arc<Mutex<DapSession>>>,
}

impl DapPool {
    /// Create an empty pool.
    pub fn new() -> Self {
        Self::default()
    }

    /// Get an existing session by key, if present.
    pub fn get_by_key(&self, key: &str) -> Option<Arc<Mutex<DapSession>>> {
        self.sessions.get(key).cloned()
    }

    /// Get or spawn a session for `backend` (+ optional cwd / extra args).
    pub async fn get_or_spawn(
        &mut self,
        backend: &str,
        cwd: Option<&str>,
        extra_args: &[String],
    ) -> Result<Arc<Mutex<DapSession>>, DapzError> {
        let key = pool_key(backend, cwd);
        if let Some(existing) = self.sessions.get(&key) {
            return Ok(existing.clone());
        }

        let session = DapSession::spawn_with_args(backend, extra_args)?;
        let wrapped = Arc::new(Mutex::new(session));
        self.sessions.insert(key, wrapped.clone());
        Ok(wrapped)
    }

    /// Insert a pre-built session (tests).
    pub fn insert(&mut self, key: impl Into<String>, session: DapSession) {
        self.sessions
            .insert(key.into(), Arc::new(Mutex::new(session)));
    }

    /// Number of live sessions.
    pub fn len(&self) -> usize {
        self.sessions.len()
    }

    /// Whether the pool is empty.
    pub fn is_empty(&self) -> bool {
        self.sessions.is_empty()
    }

    /// Remove and drop a session by key.
    pub fn remove(&mut self, key: &str) -> Option<Arc<Mutex<DapSession>>> {
        self.sessions.remove(key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transport::mock::MockTransport;

    #[test]
    fn test_pool_key_with_and_without_cwd() {
        assert_eq!(pool_key("debugpy", None), "debugpy");
        assert_eq!(pool_key("debugpy", Some("/tmp/p")), "debugpy::/tmp/p");
    }

    #[tokio::test]
    async fn test_get_by_key_returns_inserted() {
        let mut pool = DapPool::new();
        let session = DapSession::with_transport(Box::new(MockTransport::new()));
        pool.insert("k1", session);
        assert!(pool.get_by_key("k1").is_some());
        assert!(pool.get_by_key("missing").is_none());
        assert_eq!(pool.len(), 1);
    }

    #[tokio::test]
    async fn test_insert_independent_arcs() {
        let mut pool = DapPool::new();
        pool.insert(
            "a",
            DapSession::with_transport(Box::new(MockTransport::new())),
        );
        pool.insert(
            "b",
            DapSession::with_transport(Box::new(MockTransport::new())),
        );
        assert_eq!(pool.len(), 2);
        assert!(pool.remove("a").is_some());
        assert_eq!(pool.len(), 1);
    }
}
