//! Message codec layer.
//!
//! Provides JSON-RPC 2.0 framing for DAP protocol messages.
//!
//! The Debug Adapter Protocol uses a JSON-RPC-based wire format with
//! `Content-Length` header framing, identical to LSP's transport layer.

pub mod json_rpc;
