//! # dapz Daemon
//!
//! Long-lived background process that manages DAP adapter sessions.
//!
//! ## DAP vs LSP
//!
//! Unlike lspz (workspace roots + document sync), dapz daemon identity is a
//! **project cwd**. Sessions use [`crate::mcp::pool_key`] (`backend` + cwd),
//! not LSP language ids.

mod client;
pub mod protocol;
mod server;
mod socket;
mod status;

pub use client::DaemonClient;
pub use protocol::{DaemonRequest, DaemonResponse, SpawnParams};
pub use server::DaemonServer;
pub use socket::{dapz_cache_dir, resolve_project_cwd, socket_path_for_cwd};
pub use status::DaemonStatus;
