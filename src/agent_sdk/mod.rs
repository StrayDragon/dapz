//! Agent SDK for embedding dapz in Rust coding agents.
//!
//! DAP-oriented (debug adapters / debugpy), not LSP language servers.

mod agent;
mod pool;

pub use agent::{AgentBuilder, AgentHandle};
pub use pool::{AgentPool, AgentPoolBuilder};
