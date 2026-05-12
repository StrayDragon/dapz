//! WebSocket transport — connect to DAP servers via WebSocket.
//!
//! This module is only available with the `transport-websocket` feature.
//! See [rmcp](https://docs.rs/rmcp) for MCP-based WebSocket support.

use std::process::ExitStatus;

use crate::error::DapzError;
use crate::transport::Transport;

/// WebSocket transport for DAP communication.
///
/// Enabled with the `transport-websocket` feature flag.
pub struct WsTransport;

impl WsTransport {
    /// Connect to a DAP server via WebSocket URL.
    pub async fn connect(_url: &str) -> Result<Self, DapzError> {
        Err(DapzError::Config(
            "WebSocket transport not yet implemented".into(),
        ))
    }
}

#[async_trait::async_trait]
impl Transport for WsTransport {
    async fn receive(&mut self) -> Result<Vec<u8>, DapzError> {
        Err(DapzError::Protocol(
            "WebSocket transport not yet implemented".into(),
        ))
    }

    async fn send(&mut self, _data: &[u8]) -> Result<(), DapzError> {
        Err(DapzError::Protocol(
            "WebSocket transport not yet implemented".into(),
        ))
    }

    fn try_wait(&mut self) -> Result<Option<ExitStatus>, DapzError> {
        Ok(None)
    }
}
