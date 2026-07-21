//! DAP proxy state machine and message loop.
//!
//! [MermaidChart:../docs/mmd/proxy-state-machine.mmd]

use std::sync::Arc;

use tokio::io::{AsyncWriteExt, BufReader};
use tokio::select;
use tokio::sync::RwLock;

use crate::codec::json_rpc::DapMessage;
use crate::codec::toon::value_to_toon;
use crate::config::{Config, OutputFormat};
use crate::error::DapzError;
use crate::interceptors::InterceptorChain;
use crate::transport::Transport;

/// Direction of a DAP message.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    /// Client → Server
    ClientToServer,
    /// Server → Client
    ServerToClient,
}

/// Proxy state machine states.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    /// Initial state before [`Proxy::start`] is called.
    Created,
    /// Performing DAP initialize handshake.
    Initializing,
    /// Handshake complete, message loop running.
    Ready,
    /// Shutdown requested, draining remaining messages.
    ShuttingDown,
    /// Fully exited.
    Exited,
}

/// The dapz proxy.
///
/// Combines a client-side I/O (stdin/stdout) with a server-side [`Transport`]
/// and an [`InterceptorChain`] for Server→Client message transformation.
#[allow(dead_code)]
pub struct Proxy {
    config: Arc<RwLock<Config>>,
    state: State,
    transport: Box<dyn Transport>,
    interceptor_chain: InterceptorChain,
    /// Next sequence number (reserved for future Agent SDK use).
    next_seq: i64,
}

impl Proxy {
    /// Create a new [`Proxy`].
    pub fn new(
        config: Arc<RwLock<Config>>,
        transport: Box<dyn Transport>,
        interceptor_chain: InterceptorChain,
    ) -> Self {
        Self {
            config,
            state: State::Created,
            transport,
            interceptor_chain,
            next_seq: 1,
        }
    }

    /// Returns the current [`State`].
    pub fn state(&self) -> State {
        self.state
    }

    /// Start the proxy: handshake → message loop.
    pub async fn start(&mut self) -> Result<(), DapzError> {
        self.state = State::Initializing;
        tracing::info!("Proxy starting (handshake)");

        self.perform_handshake().await?;

        self.state = State::Ready;
        tracing::info!("Proxy ready, entering message loop");
        self.message_loop().await
    }

    /// DAP initialize handshake.
    async fn perform_handshake(&mut self) -> Result<(), DapzError> {
        let mut stdin = BufReader::new(tokio::io::stdin());
        let mut stdout = tokio::io::stdout();

        // Read client's "initialize" request
        let init_req = read_stdin_frame(&mut stdin).await?;
        self.transport.send(&init_req).await?;
        tracing::debug!("Forwarded 'initialize' request to server");

        // Forward server's "initialize" response to client
        let init_resp = self.transport.receive().await?;
        stdout.write_all(&init_resp).await?;
        stdout.flush().await?;
        tracing::debug!("Forwarded 'initialize' response to client");

        // Read client's "launch"/"attach" request and forward
        let launch_req = read_stdin_frame(&mut stdin).await?;
        self.transport.send(&launch_req).await?;
        tracing::debug!("Forwarded launch/attach request to server");

        // Forward server's response
        let launch_resp = self.transport.receive().await?;
        stdout.write_all(&launch_resp).await?;
        stdout.flush().await?;
        tracing::debug!("Forwarded launch/attach response to client");

        // Read "configurationDone" request and forward
        let config_done = read_stdin_frame(&mut stdin).await?;
        self.transport.send(&config_done).await?;
        tracing::debug!("Forwarded 'configurationDone' request");

        // Forward configurationDone response
        let config_resp = self.transport.receive().await?;
        stdout.write_all(&config_resp).await?;
        stdout.flush().await?;
        tracing::debug!("Configuration done, handshake complete");

        Ok(())
    }

    /// Main message loop using [`tokio::select!`].
    ///
    /// - Client → Server: transparent forward
    /// - Server → Client: interceptor chain → forward
    async fn message_loop(&mut self) -> Result<(), DapzError> {
        let mut stdin = BufReader::new(tokio::io::stdin());
        let mut stdout = tokio::io::stdout();

        loop {
            select! {
                // ── Client → Server (transparent passthrough) ──────
                client_msg = read_stdin_frame(&mut stdin) => {
                    let msg_bytes = match client_msg {
                        Ok(bytes) => bytes,
                        Err(DapzError::ServerExited) => {
                            tracing::info!("Client stdin closed, shutting down");
                            self.state = State::Exited;
                            return Ok(());
                        }
                        Err(e) => {
                            tracing::error!(error = %e, "Error reading from client");
                            return Err(e);
                        }
                    };

                    // Check for disconnect
                    if is_disconnect(&msg_bytes) {
                        tracing::info!("Received 'disconnect' from client");
                        self.transport.send(&msg_bytes).await?;
                        self.state = State::ShuttingDown;
                        let resp = self.transport.receive().await?;
                        stdout.write_all(&resp).await?;
                        stdout.flush().await?;
                        self.state = State::Exited;
                        return Ok(());
                    }

                    self.transport.send(&msg_bytes).await?;
                }

                // ── Server → Client (through interceptor chain) ────
                server_msg = self.transport.receive() => {
                    let msg_bytes = match server_msg {
                        Ok(bytes) => bytes,
                        Err(DapzError::ServerExited) => {
                            tracing::warn!("Server exited unexpectedly");
                            self.state = State::Exited;
                            return Err(DapzError::ServerExited);
                        }
                        Err(e) => {
                            tracing::error!(error = %e, "Error reading from server");
                            return Err(e);
                        }
                    };

                    let processed = self.process_server_message(&msg_bytes).await;
                    if !processed.is_empty() {
                        stdout.write_all(&processed).await?;
                        stdout.flush().await?;
                    }
                }
            }
        }
    }

    /// Process a raw server→client message through the interceptor chain.
    ///
    /// Returns the (possibly transformed) frame bytes, or empty if dropped.
    /// Always succeeds: on error, returns the original raw bytes (fail-open).
    ///
    /// Format handling (aligned with lspz):
    /// - `passthrough` — original bytes, no interceptors
    /// - `json` — compress, re-serialize DAP JSON frame
    /// - `toon` — compress, wrap `body` as `{ format: "toon", text }`
    async fn process_server_message(&mut self, raw: &[u8]) -> Vec<u8> {
        let format = self.config.read().await.output_format;
        if format == OutputFormat::Passthrough {
            return raw.to_vec();
        }

        let msg = match DapMessage::from_frame(raw) {
            Ok(m) => m,
            Err(_) => return raw.to_vec(),
        };

        let direction = Direction::ServerToClient;
        let processed = match self.interceptor_chain.process(msg, direction).await {
            Ok(Some(m)) => m,
            Ok(None) => return Vec::new(),
            Err(e) => {
                tracing::warn!(error = %e, "Interceptor failed, fail-open");
                return raw.to_vec();
            }
        };

        if format == OutputFormat::Toon {
            return self.toon_dap_frame(processed, raw);
        }

        match processed.to_bytes() {
            Ok(bytes) => bytes,
            Err(e) => {
                tracing::warn!(error = %e, "Failed to serialize DAP message, fail-open");
                raw.to_vec()
            }
        }
    }

    /// Wrap compressed DAP `body` as `{ format: "toon", text }` (lspz-style envelope).
    fn toon_dap_frame(&self, mut msg: DapMessage, raw: &[u8]) -> Vec<u8> {
        let body = msg.body.take().unwrap_or(serde_json::Value::Null);
        let text = match value_to_toon(&body) {
            Ok(t) => t,
            Err(e) => {
                tracing::warn!(error = %e, "TOON encode failed, fail-open");
                return raw.to_vec();
            }
        };
        msg.body = Some(serde_json::json!({
            "format": "toon",
            "text": text,
        }));
        match msg.to_bytes() {
            Ok(bytes) => bytes,
            Err(e) => {
                tracing::warn!(error = %e, "Failed to serialize TOON DAP frame, fail-open");
                raw.to_vec()
            }
        }
    }
}

/// Check if a client message is a "disconnect" request.
fn is_disconnect(raw: &[u8]) -> bool {
    if let Ok(msg) = DapMessage::from_frame(raw) {
        return msg.msg_type == "request" && msg.command.as_deref() == Some("disconnect");
    }
    false
}

/// Read one complete Content-Length framed message from stdin.
async fn read_stdin_frame(reader: &mut BufReader<tokio::io::Stdin>) -> Result<Vec<u8>, DapzError> {
    // Stdin is only used in the client→server select arm; a local FrameState is
    // enough because we do not cancel this future mid-header across calls in a
    // way that requires persistence beyond a single successful frame. For
    // symmetry with transports we still use the shared cancel-safe reader.
    let mut state = crate::transport::framing::FrameState::new();
    crate::transport::framing::read_frame_with_state(reader, &mut state).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::interceptors::output::OutputCompressor;
    use crate::transport::mock::MockTransport;
    use serde_json::json;

    fn output_event_raw() -> Vec<u8> {
        let msg = DapMessage {
            seq: 3,
            msg_type: "event".into(),
            command: None,
            event: Some("output".into()),
            request_seq: None,
            success: None,
            body: Some(json!({
                "category": "stdout",
                "output": "\u{001b}[31mred\u{001b}[0m\n",
                "source": { "name": "a.py", "path": "/tmp/a.py" }
            })),
            arguments: None,
        };
        msg.to_bytes().unwrap()
    }

    #[tokio::test]
    async fn test_new_proxy_state() {
        let config = Arc::new(RwLock::new(Config {
            backend_cmd: "test".into(),
            ..Default::default()
        }));
        let transport = Box::new(MockTransport::new());
        let chain = InterceptorChain::new(vec![], config.clone());

        let proxy = Proxy::new(config, transport, chain);
        assert_eq!(proxy.state(), State::Created);
    }

    #[tokio::test]
    async fn test_passthrough_skips_compression() {
        let config = Arc::new(RwLock::new(
            Config::builder()
                .backend_cmd("mock")
                .output_format(OutputFormat::Passthrough)
                .build()
                .unwrap(),
        ));
        let chain = InterceptorChain::new(vec![Box::new(OutputCompressor)], config.clone());
        let mut proxy = Proxy::new(config, Box::new(MockTransport::new()), chain);
        let raw = output_event_raw();
        let out = proxy.process_server_message(&raw).await;
        assert_eq!(out, raw);
    }

    #[tokio::test]
    async fn test_json_compresses_but_keeps_dap_body() {
        let config = Arc::new(RwLock::new(
            Config::builder()
                .backend_cmd("mock")
                .output_format(OutputFormat::Json)
                .enable_output_compress(true)
                .build()
                .unwrap(),
        ));
        let chain = InterceptorChain::new(vec![Box::new(OutputCompressor)], config.clone());
        let mut proxy = Proxy::new(config, Box::new(MockTransport::new()), chain);
        let raw = output_event_raw();
        let out = proxy.process_server_message(&raw).await;
        assert_ne!(out, raw);
        let msg = DapMessage::from_frame(&out).unwrap();
        let body = msg.body.unwrap();
        assert!(body.get("format").is_none());
        let output = body["output"].as_str().unwrap_or("");
        assert!(!output.contains('\u{001b}'));
    }

    #[tokio::test]
    async fn test_toon_wraps_body() {
        let config = Arc::new(RwLock::new(
            Config::builder()
                .backend_cmd("mock")
                .output_format(OutputFormat::Toon)
                .enable_output_compress(true)
                .build()
                .unwrap(),
        ));
        let chain = InterceptorChain::new(vec![Box::new(OutputCompressor)], config.clone());
        let mut proxy = Proxy::new(config, Box::new(MockTransport::new()), chain);
        let raw = output_event_raw();
        let out = proxy.process_server_message(&raw).await;
        let msg = DapMessage::from_frame(&out).unwrap();
        assert_eq!(msg.event.as_deref(), Some("output"));
        let body = msg.body.unwrap();
        assert_eq!(body["format"], "toon");
        let text = body["text"].as_str().unwrap();
        assert!(!text.is_empty());
    }
}
