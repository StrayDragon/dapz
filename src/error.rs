//! Unified error type for dapz.
//!
//! Uses [`thiserror`] for ergonomic error derivation.

use std::io;

/// The unified error type for the dapz codebase.
#[derive(Debug, thiserror::Error)]
pub enum DapzError {
    #[error("IO error: {0}")]
    Io(#[from] io::Error),

    #[error("JSON parse error: {0}")]
    JsonParse(#[from] serde_json::Error),

    #[error("Protocol error: {0}")]
    Protocol(String),

    #[error("Server exited unexpectedly")]
    ServerExited,

    #[error("Configuration error: {0}")]
    Config(String),
}
