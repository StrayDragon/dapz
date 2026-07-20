//! DAP session pool — reuse adapter processes by backend+cwd key.
//!
//! Pattern ported from lspz `mcp/pool.rs` (simplified: no document tracking).

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

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

    /// Look up an existing session by key, reaping a dead child first.
    pub fn get_by_key(&mut self, key: &str) -> Option<Arc<Mutex<DapSession>>> {
        self.reap_if_dead(key);
        self.sessions.get(key).cloned()
    }

    /// Remove `key` if its adapter child has exited (`try_wait` → `Some`).
    pub fn reap_if_dead(&mut self, key: &str) -> bool {
        let Some(session) = self.sessions.get(key).cloned() else {
            return false;
        };
        let Ok(mut guard) = session.try_lock() else {
            return false; // busy — leave for later
        };
        match guard.try_wait() {
            Ok(Some(status)) => {
                drop(guard);
                tracing::warn!(%key, ?status, "Removing dead DAP session");
                self.sessions.remove(key);
                true
            }
            _ => false,
        }
    }

    /// Remove sessions whose last I/O was longer ago than `idle_threshold`.
    ///
    /// Only reaps sessions that are not currently locked (try_lock). Busy
    /// sessions are skipped until a later pass.
    pub fn reap_idle(&mut self, idle_threshold: Duration) -> usize {
        let now = std::time::Instant::now();
        let before = self.sessions.len();
        self.sessions.retain(|_, s| match s.try_lock() {
            Ok(guard) => now.duration_since(guard.last_used_at()) < idle_threshold,
            Err(_) => true, // busy — keep
        });
        before - self.sessions.len()
    }

    /// Get or spawn a session for `backend` (+ optional cwd / extra args).
    ///
    /// Dead child processes (`try_wait` reports exit) are removed before reuse.
    pub async fn get_or_spawn(
        &mut self,
        backend: &str,
        cwd: Option<&str>,
        extra_args: &[String],
    ) -> Result<Arc<Mutex<DapSession>>, DapzError> {
        let key = pool_key(backend, cwd);
        self.reap_if_dead(&key);
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

    /// Drop all sessions (kills adapter children via Drop).
    pub fn clear(&mut self) {
        let n = self.sessions.len();
        self.sessions.clear();
        if n > 0 {
            tracing::info!(cleared = n, "Cleared all DAP sessions");
        }
    }

    /// List session keys currently in the pool.
    pub fn keys(&self) -> Vec<String> {
        self.sessions.keys().cloned().collect()
    }

    /// Check whether a session exists for the given key.
    pub fn contains_key(&self, key: &str) -> bool {
        self.sessions.contains_key(key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transport::mock::MockTransport;
    use std::time::Duration;

    fn make_pool(keys: &[&str]) -> DapPool {
        let mut pool = DapPool::new();
        for k in keys {
            pool.insert(
                *k,
                DapSession::with_transport(Box::new(MockTransport::new())),
            );
        }
        pool
    }

    #[test]
    fn test_pool_key_with_and_without_cwd() {
        assert_eq!(pool_key("debugpy", None), "debugpy");
        assert_eq!(pool_key("debugpy", Some("/tmp/p")), "debugpy::/tmp/p");
    }

    #[test]
    fn test_is_empty_for_fresh_pool() {
        assert!(DapPool::new().is_empty());
    }

    #[test]
    fn test_clear_removes_all_sessions() {
        let mut pool = make_pool(&["debugpy::/p", "lldb::/p"]);
        assert_eq!(pool.keys().len(), 2);
        pool.clear();
        assert!(pool.is_empty());
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
    async fn test_get_by_key_returns_independent_arc() {
        let mut pool = make_pool(&["debugpy::/p"]);
        let a = pool.get_by_key("debugpy::/p").unwrap();
        let b = pool.get_by_key("debugpy::/p").unwrap();
        assert!(Arc::ptr_eq(&a, &b));
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

    #[test]
    fn test_reap_idle_zero_reaps_all() {
        let mut pool = make_pool(&["debugpy::/p", "lldb::/p"]);
        assert_eq!(pool.keys().len(), 2);

        let reaped = pool.reap_idle(Duration::ZERO);

        assert_eq!(reaped, 2);
        assert!(pool.is_empty());
    }

    #[test]
    fn test_reap_idle_large_threshold_keeps_all() {
        let mut pool = make_pool(&["debugpy::/p"]);

        let reaped = pool.reap_idle(Duration::from_secs(3600));

        assert_eq!(reaped, 0);
        assert!(!pool.is_empty());
    }

    #[test]
    fn test_reap_idle_is_selective() {
        let mut pool = make_pool(&["a::/p", "b::/p"]);
        assert_eq!(pool.reap_idle(Duration::from_secs(3600)), 0);
        assert_eq!(pool.keys().len(), 2);

        assert_eq!(pool.reap_idle(Duration::ZERO), 2);
        assert!(pool.is_empty());
    }

    #[test]
    fn test_reap_if_dead_removes_exited_session() {
        let mock = MockTransport::new();
        mock.mark_exited();
        let mut pool = DapPool::new();
        pool.insert("debugpy::/p", DapSession::with_transport(Box::new(mock)));
        assert!(pool.contains_key("debugpy::/p"));
        assert!(pool.reap_if_dead("debugpy::/p"));
        assert!(!pool.contains_key("debugpy::/p"));
    }

    #[test]
    fn test_reap_if_dead_keeps_live_session() {
        let mut pool = make_pool(&["debugpy::/p"]);
        assert!(!pool.reap_if_dead("debugpy::/p"));
        assert!(pool.contains_key("debugpy::/p"));
    }
}
