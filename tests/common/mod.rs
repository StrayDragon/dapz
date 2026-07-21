//! Shared test utilities for DAP integration tests.
//!
//! Split into submodules so each integration binary can pull only what it needs:
//! - [`fixture`] — temporary scripts
//! - [`session`] — DAP stdio framing + handshake

pub mod fixture;
pub mod session;

pub use fixture::create_test_script;
pub use session::{DapSession, perform_handshake};
