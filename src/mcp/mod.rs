//! # dapz MCP server
//!
//! Exposes DAP debug capabilities as MCP tools (Tier-0 observation set).
//!
//! Pattern ported from lspz `mcp/` — session pool + tool server.

pub use pool::{DapPool, pool_key};
pub use server::McpServer;
pub use session::DapSession;

mod pool;
mod server;
mod session;
