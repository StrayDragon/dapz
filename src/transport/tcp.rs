//! TCP transport — connect to DAP servers via TCP socket.
//!
//! Useful for remote debugging scenarios.

use std::process::ExitStatus;

use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;

use crate::codec::json_rpc;
use crate::error::DapzError;
use crate::transport::Transport;

/// TCP socket transport for DAP communication.
pub struct TcpTransport {
    reader: BufReader<tokio::io::ReadHalf<TcpStream>>,
    writer: tokio::io::WriteHalf<TcpStream>,
}

impl TcpTransport {
    /// Connect to a DAP server at the given TCP address.
    pub async fn connect(addr: &str) -> Result<Self, DapzError> {
        let stream = TcpStream::connect(addr).await?;
        let (reader, writer) = tokio::io::split(stream);
        Ok(Self {
            reader: BufReader::new(reader),
            writer,
        })
    }
}

#[async_trait::async_trait]
impl Transport for TcpTransport {
    async fn receive(&mut self) -> Result<Vec<u8>, DapzError> {
        let mut header = String::new();
        loop {
            let mut line = String::new();
            let n = self.reader.read_line(&mut line).await.map_err(|e| {
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
        self.reader.read_exact(&mut body).await.map_err(|e| {
            if e.kind() == std::io::ErrorKind::UnexpectedEof {
                DapzError::ServerExited
            } else {
                DapzError::Io(e)
            }
        })?;

        let mut result = header.into_bytes();
        result.extend_from_slice(&body);
        Ok(result)
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
