//! DAP proxy state machine and message loop.
//!
//! [MermaidChart:../docs/mmd/proxy-state-machine.mmd]

use std::sync::Arc;

use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::select;
use tokio::sync::RwLock;

use crate::codec::json_rpc;
use crate::codec::json_rpc::DapMessage;
use crate::config::Config;
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
    async fn process_server_message(&mut self, raw: &[u8]) -> Vec<u8> {
        let msg = match DapMessage::from_frame(raw) {
            Ok(m) => m,
            Err(_) => return raw.to_vec(),
        };

        let direction = Direction::ServerToClient;

        let processed = match self.interceptor_chain.process(msg, direction).await {
            Ok(Some(m)) => m,
            Ok(None) => return Vec::new(),
            Err(_) => return raw.to_vec(),
        };

        // Re-serialize the possibly transformed message
        match processed.to_bytes() {
            Ok(bytes) => bytes,
            Err(_) => raw.to_vec(),
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
    let mut header = String::new();
    loop {
        let mut line = String::new();
        let n = reader.read_line(&mut line).await.map_err(|e| {
            if e.kind() == std::io::ErrorKind::UnexpectedEof {
                DapzError::ServerExited
            } else {
                DapzError::Io(e)
            }
        })?;

        if n == 0 {
            return Err(DapzError::ServerExited);
        }

        header.push_str(&line);

        if line == "\r\n" || line == "\n" {
            break;
        }
    }

    let content_length = json_rpc::parse_content_length(&header)?;
    let mut body = vec![0u8; content_length as usize];
    reader.read_exact(&mut body).await.map_err(|e| {
        if e.kind() == std::io::ErrorKind::UnexpectedEof {
            DapzError::ServerExited
        } else {
            DapzError::Io(e)
        }
    })?;

    Ok([header.as_bytes(), &body].concat())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::transport::mock::MockTransport;

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
}
