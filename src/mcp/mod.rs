//! # dapz MCP server
//!
//! Exposes DAP debug capabilities as MCP tools (Tier-0 observation set).
//!
//! Pattern ported from lspz `mcp/` — session pool + tool server.
//! Default CLI path uses [`DaemonMcpServer`]; `--no-daemon` uses in-process [`McpServer`].

pub use adapter_kind::AdapterKind;
pub use daemon_server::DaemonMcpServer;
pub use pool::{DapPool, pool_key};
pub use server::McpServer;
pub use session::DapSession;

mod adapter_kind;
mod daemon_server;
mod pool;
mod server;
mod session;
