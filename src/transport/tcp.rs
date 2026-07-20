//! TCP transport — connect to DAP servers via TCP socket.

use std::process::ExitStatus;

use tokio::io::{AsyncWriteExt, BufReader};
use tokio::net::TcpStream;

use crate::error::DapzError;
use crate::transport::Transport;
use crate::transport::framing::{self, FrameState};

/// TCP socket transport for DAP communication.
pub struct TcpTransport {
    reader: BufReader<tokio::io::ReadHalf<TcpStream>>,
    writer: tokio::io::WriteHalf<TcpStream>,
    frame_state: FrameState,
}

impl TcpTransport {
    /// Connect to a DAP server at the given TCP address.
    pub async fn connect(addr: &str) -> Result<Self, DapzError> {
        let stream = TcpStream::connect(addr).await?;
        let (reader, writer) = tokio::io::split(stream);
        Ok(Self {
            reader: BufReader::new(reader),
            writer,
            frame_state: FrameState::new(),
        })
    }
}

#[async_trait::async_trait]
impl Transport for TcpTransport {
    async fn receive(&mut self) -> Result<Vec<u8>, DapzError> {
        framing::read_frame_with_state(&mut self.reader, &mut self.frame_state).await
    }

    async fn send(&mut self, data: &[u8]) -> Result<(), DapzError> {
        self.writer.write_all(data).await?;
        self.writer.flush().await?;
        Ok(())
    }

    fn try_wait(&mut self) -> Result<Option<ExitStatus>, DapzError> {
        Ok(None)
    }
}
