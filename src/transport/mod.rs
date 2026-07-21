//! Transport abstraction.
//!
//! Defines the I/O trait that all transports must implement for DAP communication.
//!
//! ## Available Transports
//!
//! | Transport | Protocol | Feature Flag | Status |
//! |-----------|----------|-------------|--------|
//! | `StdioTransport` | Child process stdio | always | ✅ |
//! | `TcpTransport` | TCP socket | always | ✅ |
//! | `MockTransport` | In-memory FIFO | always (testing) | ✅ |

pub(crate) mod framing;
pub mod mock;
pub mod stdio;
pub mod tcp;

use std::process::ExitStatus;

use crate::error::DapzError;

/// Abstract I/O channel for DAP communication.
///
/// All DAP message I/O (regardless of transport protocol) is defined by this trait.
/// Implementations handle Content-Length framing internally and expose raw framed bytes.
#[async_trait::async_trait]
pub trait Transport: Send + Sync {
    /// Receive one raw DAP message (Content-Length framed).
    async fn receive(&mut self) -> Result<Vec<u8>, DapzError>;

    /// Send raw bytes to the DAP server/client.
    async fn send(&mut self, data: &[u8]) -> Result<(), DapzError>;

    /// Check whether the underlying process has exited.
    fn try_wait(&mut self) -> Result<Option<ExitStatus>, DapzError> {
        Ok(None)
    }
}
