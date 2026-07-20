//! JSON-RPC protocol types for daemon client-server communication (NDJSON).

use serde::{Deserialize, Serialize};

/// A request from a client to the daemon.
#[derive(Debug, Serialize, Deserialize)]
pub struct DaemonRequest {
    /// Unique request ID for correlation.
    pub id: u64,
    /// Method name.
    pub method: String,
    /// JSON params (varies by method).
    #[serde(default = "serde_json::Value::default")]
    pub params: serde_json::Value,
}

/// A response from the daemon to a client.
#[derive(Debug, Serialize, Deserialize)]
pub struct DaemonResponse {
    /// Matches the request ID.
    pub id: u64,
    /// `null` on success, error string on failure.
    #[serde(default)]
    pub error: Option<String>,
    /// Result value (null if error).
    #[serde(default = "serde_json::Value::default")]
    pub result: serde_json::Value,
}

impl DaemonResponse {
    /// Create a success response.
    pub fn ok(id: u64, result: serde_json::Value) -> Self {
        Self {
            id,
            error: None,
            result,
        }
    }

    /// Create an error response.
    pub fn err(id: u64, message: impl Into<String>) -> Self {
        Self {
            id,
            error: Some(message.into()),
            result: serde_json::Value::Null,
        }
    }
}

/// Parameters for `dap/spawn` — get or create a DAP adapter session.
#[derive(Debug, Serialize, Deserialize)]
pub struct SpawnParams {
    /// Adapter backend command (e.g. debugpy-adapter path).
    pub backend: String,
    /// Optional project cwd (also used in pool key).
    #[serde(default)]
    pub cwd: Option<String>,
    /// Extra arguments passed to the adapter.
    #[serde(default)]
    pub extra_args: Vec<String>,
    /// When true, drop any existing session for this key before spawn (fresh launch/attach).
    #[serde(default)]
    pub replace: bool,
}

/// Parameters for `dap/remove` — drop a session from the daemon pool.
#[derive(Debug, Serialize, Deserialize)]
pub struct RemoveParams {
    /// Session key (`pool_key(backend, cwd)`).
    pub session_key: String,
}

/// Parameters for `dap/invoke` — run a high-level [`crate::mcp::DapSession`] operation.
#[derive(Debug, Serialize, Deserialize)]
pub struct InvokeParams {
    /// Session key.
    pub session_key: String,
    /// Operation name (e.g. `launch_program`, `continue`, `get_stack`).
    pub op: String,
    /// Op-specific JSON arguments.
    #[serde(default = "serde_json::Value::default")]
    pub args: serde_json::Value,
}

/// Parameters for `dap/request` — send a DAP request and await the response body.
#[derive(Debug, Serialize, Deserialize)]
pub struct DapRequestParams {
    /// Session key (`pool_key(backend, cwd)`).
    pub session_key: String,
    /// DAP command name.
    pub command: String,
    /// DAP request arguments.
    #[serde(default = "serde_json::Value::default")]
    pub arguments: serde_json::Value,
}

/// Parameters for `dap/wait_event` — wait for a DAP event.
#[derive(Debug, Serialize, Deserialize)]
pub struct WaitEventParams {
    /// Session key.
    pub session_key: String,
    /// Event name (e.g. `stopped`).
    pub event: String,
    /// Timeout in milliseconds (default 30s).
    #[serde(default)]
    pub timeout_ms: Option<u64>,
}
