//! # dapz
//!
//! AI-friendly DAP compression proxy — token-efficient Debug Adapter Protocol proxy.
//!
//! ## Feature Flags
//!
//! | Feature | Description | Default |
//! |---------|-------------|---------|
//! | `cli` | CLI binary (clap, tracing-subscriber) | yes |
//! | `mcp` | MCP server (rmcp) | no |
//! | `agent-sdk` | Agent SDK API | no |
//! | `transport-tcp` | TCP transport | no |
//! | `transport-websocket` | WebSocket transport | no |
//!
//! ## Architecture
//!
//! [MermaidChart:./docs/mmd/architecture.mmd]

pub mod codec;
pub mod config;
pub mod error;
pub mod interceptors;
pub mod proxy;
pub mod transport;

// Re-exports for convenience.
pub use config::{CappingConfig, Config, OutputFormat};
pub use error::DapzError;
pub use interceptors::capping::CappingInterceptor;
pub use interceptors::output::OutputCompressor;
pub use interceptors::stacktrace::StackTraceCompressor;
pub use interceptors::variables::VariablesCompressor;
pub use proxy::{Direction, Proxy, State};
pub use transport::Transport;
pub use transport::stdio::StdioTransport;
pub use transport::tcp::TcpTransport;

/// WebSocket transport (requires `transport-websocket` feature).
#[cfg(feature = "transport-websocket")]
pub use transport::websocket::WsTransport;
