//! Daemon status snapshot for `daemon/status`.

use std::time::Instant;

use serde::Serialize;

/// Runtime status reported by the daemon.
#[derive(Debug, Clone, Serialize)]
pub struct DaemonStatus {
    /// When the daemon started.
    #[serde(skip)]
    started_at: Instant,
    /// Total client connections accepted.
    pub total_connections: u64,
    /// Total RPC requests handled.
    pub total_requests: u64,
    /// Active session keys.
    pub sessions: Vec<String>,
}

impl DaemonStatus {
    /// Create a fresh status.
    pub fn new() -> Self {
        Self {
            started_at: Instant::now(),
            total_connections: 0,
            total_requests: 0,
            sessions: Vec::new(),
        }
    }

    /// Record a new client connection.
    pub fn record_connection(&mut self) {
        self.total_connections += 1;
    }

    /// Record an RPC request.
    pub fn record_request(&mut self) {
        self.total_requests += 1;
    }

    /// Uptime in seconds.
    pub fn uptime_secs(&self) -> u64 {
        self.started_at.elapsed().as_secs()
    }
}

impl Default for DaemonStatus {
    fn default() -> Self {
        Self::new()
    }
}
