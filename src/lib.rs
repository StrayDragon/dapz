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

pub mod adapters;
pub mod codec;
pub mod config;
pub mod error;
pub mod interceptors;
pub mod metrics;
pub mod proxy;
pub mod transport;

#[cfg(feature = "mcp")]
pub mod mcp;

#[cfg(feature = "agent-sdk")]
pub mod agent_sdk;

// Re-exports for convenience.
pub use adapters::{
    AdapterInfo, DiscoveryContext, lookup_by_extension, lookup_by_language,
    resolve_python_debug_adapter, resolve_python_debug_adapter_in, resolve_tool, resolve_tool_in,
};
pub use codec::toon::value_to_toon;
pub use config::{CappingConfig, Config, OutputFormat};
pub use error::DapzError;
pub use interceptors::capping::CappingInterceptor;
pub use interceptors::evaluate::EvaluateCompressor;
pub use interceptors::exception::ExceptionInfoCompressor;
pub use interceptors::output::OutputCompressor;
pub use interceptors::scopes::ScopesCompressor;
pub use interceptors::stacktrace::StackTraceCompressor;
pub use interceptors::variables::VariablesCompressor;
pub use metrics::{MeteredInterceptor, MetricsConfig, MetricsSnapshot, metrics_enabled_from_env};
pub use proxy::{Direction, Proxy, State};
pub use transport::Transport;
pub use transport::stdio::StdioTransport;
pub use transport::tcp::TcpTransport;

/// WebSocket transport (requires `transport-websocket` feature).
#[cfg(feature = "transport-websocket")]
pub use transport::websocket::WsTransport;
