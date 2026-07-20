//! Interceptor trait and chain.
//!
//! All Server→Client DAP message transformations go through the interceptor chain.
//!
//! DAP has two types of Server→Client messages:
//! - **Events** (e.g., `output`, `stopped`, `breakpoint`)
//! - **Responses** (e.g., `variables`, `stackTrace`, `scopes`)
//!
//! ## Architecture
//!
//! [MermaidChart:../docs/mmd/interceptor-chain.mmd]

pub mod capping;
pub mod evaluate;
pub mod output;
pub mod scopes;
pub mod stacktrace;
pub mod utils;
pub mod variables;

use std::sync::Arc;

use tokio::sync::RwLock;

use crate::codec::json_rpc::DapMessage;
use crate::config::Config;
use crate::error::DapzError;
use crate::proxy::Direction;

/// A single interceptor in the chain.
///
/// Each interceptor can inspect and selectively transform DAP messages.
#[async_trait::async_trait]
pub trait Interceptor: Send + Sync {
    /// Unique name for logging and configuration.
    fn name(&self) -> &str;

    /// Whether this interceptor should process the given message.
    fn applies_to(&self, msg: &DapMessage, direction: Direction) -> bool;

    /// Transform the message body/arguments.
    ///
    /// Returns:
    /// - `Ok(Some(msg))` — modified message to use
    /// - `Ok(None)` — drop the message
    /// - `Err(_)` — fail open; caller should forward original
    async fn intercept(
        &self,
        msg: DapMessage,
        direction: Direction,
    ) -> Result<Option<DapMessage>, DapzError>;
}

/// A chain of interceptors executed in order.
///
/// Holds a shared config reference for runtime enable/disable checks.
pub struct InterceptorChain {
    interceptors: Vec<Box<dyn Interceptor>>,
    config: Arc<RwLock<Config>>,
}

impl InterceptorChain {
    /// Create a new chain with the given interceptors and shared config.
    pub fn new(interceptors: Vec<Box<dyn Interceptor>>, config: Arc<RwLock<Config>>) -> Self {
        Self {
            interceptors,
            config,
        }
    }

    /// Process a DAP message through all matching interceptors.
    ///
    /// Skips interceptors that are disabled in the current config.
    /// On interceptor failure, logs a WARN and returns the original message (fail-open).
    pub async fn process(
        &self,
        msg: DapMessage,
        direction: Direction,
    ) -> Result<Option<DapMessage>, DapzError> {
        let config = self.config.read().await;
        let mut msg = Some(msg);
        for interceptor in &self.interceptors {
            if let Some(ref m) = msg
                && interceptor.applies_to(m, direction)
                && config.is_interceptor_enabled(interceptor.name())
            {
                let original = m.clone();
                match interceptor.intercept(m.clone(), direction).await {
                    Ok(Some(new_msg)) => msg = Some(new_msg),
                    Ok(None) => return Ok(None),
                    Err(e) => {
                        tracing::warn!(
                            interceptor = %interceptor.name(),
                            error = %e,
                            "Interceptor failed, forwarding original"
                        );
                        return Ok(Some(original));
                    }
                }
            }
        }
        Ok(msg)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codec::json_rpc::DapMessage;
    use std::sync::Arc;
    use tokio::sync::RwLock;

    #[tokio::test]
    async fn test_empty_chain_passthrough() {
        let config = Arc::new(RwLock::new(Config {
            backend_cmd: "test".into(),
            ..Default::default()
        }));
        let chain = InterceptorChain::new(vec![], config);

        let msg = DapMessage {
            seq: 1,
            msg_type: "event".into(),
            command: None,
            event: Some("output".into()),
            request_seq: None,
            success: None,
            body: None,
            arguments: None,
        };

        let result = chain
            .process(msg.clone(), Direction::ServerToClient)
            .await
            .unwrap();
        assert!(result.is_some());
    }
}
