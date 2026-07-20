//! Runtime metrics for the DAP interceptor chain.
//!
//! File-level port of lspz `metrics.rs`, adapted to dapz's `Interceptor`
//! trait (`DapMessage` in/out — not LSP method+params).
//!
//! Note: lspz names the wrapper `MetredInterceptor` (historical typo);
//! dapz uses `MeteredInterceptor`.

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

use crate::codec::json_rpc::DapMessage;
use crate::error::DapzError;
use crate::interceptors::Interceptor;
use crate::proxy::Direction;

/// Configuration for runtime metrics collection.
#[derive(Debug, Default, Clone, serde::Deserialize)]
pub struct MetricsConfig {
    /// Whether metrics collection is enabled.
    pub enabled: bool,
    /// Periodic log interval in seconds (0 = log on milestones only).
    pub report_interval_secs: u64,
}

/// Atomic metrics snapshot for one interceptor.
#[derive(Debug)]
pub struct MetricsSnapshot {
    total_input_bytes: AtomicU64,
    total_output_bytes: AtomicU64,
    total_latency_us: AtomicU64,
    messages_processed: AtomicU64,
    failures: AtomicU64,
}

impl MetricsSnapshot {
    /// Create a zeroed snapshot.
    pub fn new() -> Self {
        Self {
            total_input_bytes: AtomicU64::new(0),
            total_output_bytes: AtomicU64::new(0),
            total_latency_us: AtomicU64::new(0),
            messages_processed: AtomicU64::new(0),
            failures: AtomicU64::new(0),
        }
    }

    /// Total input bytes measured.
    pub fn total_input_bytes(&self) -> u64 {
        self.total_input_bytes.load(Ordering::Relaxed)
    }

    /// Total output bytes measured.
    pub fn total_output_bytes(&self) -> u64 {
        self.total_output_bytes.load(Ordering::Relaxed)
    }

    /// Messages successfully processed.
    pub fn messages_processed(&self) -> u64 {
        self.messages_processed.load(Ordering::Relaxed)
    }

    /// Interceptor failures recorded.
    pub fn failures(&self) -> u64 {
        self.failures.load(Ordering::Relaxed)
    }

    /// Compression ratio `1.0 - output/input` (0 when no input).
    pub fn compression_ratio(&self) -> f64 {
        let input = self.total_input_bytes.load(Ordering::Relaxed);
        let output = self.total_output_bytes.load(Ordering::Relaxed);
        if input == 0 {
            return 0.0;
        }
        1.0 - (output as f64 / input as f64)
    }

    /// Average latency per processed message in microseconds.
    pub fn avg_latency_us(&self) -> f64 {
        let count = self.messages_processed.load(Ordering::Relaxed);
        if count == 0 {
            return 0.0;
        }
        self.total_latency_us.load(Ordering::Relaxed) as f64 / count as f64
    }

    /// Human-readable summary for tracing.
    pub fn summary(&self, name: &str) -> String {
        let processed = self.messages_processed.load(Ordering::Relaxed);
        let failures = self.failures.load(Ordering::Relaxed);
        let ratio = self.compression_ratio();
        let avg_lat = self.avg_latency_us();

        format!(
            "Metrics[{name}]: processed={processed}, compression_ratio={:.1}%, avg_latency_us={avg_lat:.0}, failures={failures}",
            ratio * 100.0
        )
    }
}

impl Default for MetricsSnapshot {
    fn default() -> Self {
        Self::new()
    }
}

fn message_size(msg: &DapMessage) -> u64 {
    msg.to_bytes().map(|b| b.len() as u64).unwrap_or(0)
}

/// Interceptor wrapper that records byte sizes and latency for DAP messages.
pub struct MeteredInterceptor {
    inner: Box<dyn Interceptor>,
    snapshot: Box<MetricsSnapshot>,
    enabled: bool,
}

impl MeteredInterceptor {
    /// Wrap an interceptor (metrics disabled until [`Self::enable`]).
    pub fn new(inner: Box<dyn Interceptor>) -> Self {
        Self {
            inner,
            snapshot: Box::new(MetricsSnapshot::new()),
            enabled: false,
        }
    }

    /// Enable metrics collection.
    pub fn enable(mut self) -> Self {
        self.enabled = true;
        self
    }

    /// Borrow the snapshot.
    pub fn snapshot(&self) -> &MetricsSnapshot {
        &self.snapshot
    }
}

#[async_trait::async_trait]
impl Interceptor for MeteredInterceptor {
    fn name(&self) -> &str {
        self.inner.name()
    }

    fn applies_to(&self, msg: &DapMessage, direction: Direction) -> bool {
        self.inner.applies_to(msg, direction)
    }

    async fn intercept(
        &self,
        msg: DapMessage,
        direction: Direction,
    ) -> Result<Option<DapMessage>, DapzError> {
        if !self.enabled {
            return self.inner.intercept(msg, direction).await;
        }

        let pre_size = message_size(&msg);
        let start = Instant::now();

        match self.inner.intercept(msg, direction).await {
            Ok(Some(new_msg)) => {
                let elapsed = start.elapsed().as_micros() as u64;
                let post_size = message_size(&new_msg);

                self.snapshot
                    .total_input_bytes
                    .fetch_add(pre_size, Ordering::Relaxed);
                self.snapshot
                    .total_output_bytes
                    .fetch_add(post_size, Ordering::Relaxed);
                self.snapshot
                    .total_latency_us
                    .fetch_add(elapsed, Ordering::Relaxed);
                let count = self
                    .snapshot
                    .messages_processed
                    .fetch_add(1, Ordering::Relaxed)
                    + 1;

                if count == 1 || count.is_multiple_of(100) {
                    tracing::info!("{}", self.snapshot.summary(self.inner.name()));
                }

                Ok(Some(new_msg))
            }
            Ok(None) => {
                let elapsed = start.elapsed().as_micros() as u64;
                self.snapshot
                    .total_input_bytes
                    .fetch_add(pre_size, Ordering::Relaxed);
                self.snapshot
                    .total_latency_us
                    .fetch_add(elapsed, Ordering::Relaxed);
                self.snapshot
                    .messages_processed
                    .fetch_add(1, Ordering::Relaxed);
                Ok(None)
            }
            Err(e) => {
                self.snapshot.failures.fetch_add(1, Ordering::Relaxed);
                Err(e)
            }
        }
    }
}

/// Log non-empty snapshots via tracing.
pub fn log_metrics_summary(wrappers: &[&MeteredInterceptor]) {
    for m in wrappers {
        if m.snapshot.messages_processed.load(Ordering::Relaxed) > 0 {
            tracing::info!("{}", m.snapshot.summary(m.name()));
        }
    }
}

/// Whether process env requests metrics (`DAPZ_METRICS=1|true|yes`).
pub fn metrics_enabled_from_env() -> bool {
    match std::env::var("DAPZ_METRICS") {
        Ok(v) => {
            let v = v.trim().to_ascii_lowercase();
            v == "1" || v == "true" || v == "yes" || v == "on"
        }
        Err(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    struct TestInterceptor;

    #[async_trait::async_trait]
    impl Interceptor for TestInterceptor {
        fn name(&self) -> &str {
            "test"
        }

        fn applies_to(&self, _msg: &DapMessage, _direction: Direction) -> bool {
            true
        }

        async fn intercept(
            &self,
            mut msg: DapMessage,
            _direction: Direction,
        ) -> Result<Option<DapMessage>, DapzError> {
            msg.body = Some(json!({"compressed": true}));
            Ok(Some(msg))
        }
    }

    fn sample_msg() -> DapMessage {
        DapMessage {
            seq: 1,
            msg_type: "response".into(),
            command: Some("variables".into()),
            event: None,
            request_seq: Some(1),
            success: Some(true),
            body: Some(json!({"variables": [{"name": "x", "value": "1".repeat(80)}]})),
            arguments: None,
        }
    }

    #[tokio::test]
    async fn test_metered_records_sizes() {
        let interceptor = MeteredInterceptor::new(Box::new(TestInterceptor)).enable();
        let _ = interceptor
            .intercept(sample_msg(), Direction::ServerToClient)
            .await
            .unwrap();

        assert_eq!(interceptor.snapshot.messages_processed(), 1);
        assert!(interceptor.snapshot.total_input_bytes() > 0);
        assert!(interceptor.snapshot.total_output_bytes() > 0);
    }

    #[tokio::test]
    async fn test_disabled_no_recording() {
        let interceptor = MeteredInterceptor::new(Box::new(TestInterceptor));
        let _ = interceptor
            .intercept(sample_msg(), Direction::ServerToClient)
            .await
            .unwrap();
        assert_eq!(interceptor.snapshot.messages_processed(), 0);
    }

    #[tokio::test]
    async fn test_failure_recorded() {
        struct FailInterceptor;
        #[async_trait::async_trait]
        impl Interceptor for FailInterceptor {
            fn name(&self) -> &str {
                "fail"
            }
            fn applies_to(&self, _: &DapMessage, _: Direction) -> bool {
                true
            }
            async fn intercept(
                &self,
                _: DapMessage,
                _: Direction,
            ) -> Result<Option<DapMessage>, DapzError> {
                Err(DapzError::Protocol("fail".into()))
            }
        }

        let interceptor = MeteredInterceptor::new(Box::new(FailInterceptor)).enable();
        let _ = interceptor
            .intercept(sample_msg(), Direction::ServerToClient)
            .await;
        assert_eq!(interceptor.snapshot.failures(), 1);
        assert_eq!(interceptor.snapshot.messages_processed(), 0);
    }

    #[test]
    fn test_compression_ratio() {
        let snap = MetricsSnapshot::new();
        snap.total_input_bytes.store(200, Ordering::Relaxed);
        snap.total_output_bytes.store(50, Ordering::Relaxed);
        assert!((snap.compression_ratio() - 0.75).abs() < 0.001);
    }
}
