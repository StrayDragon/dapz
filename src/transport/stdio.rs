//! Stdio transport — child process stdio.
//!
//! Spawns a DAP server as a child process and communicates via stdin/stdout.
//!
//! This is the primary transport for DAP proxies, matching the typical
//! DAP server deployment pattern.

use std::process::ExitStatus;

use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, ChildStdout};

use crate::codec::json_rpc;
use crate::error::DapzError;
use crate::transport::Transport;

/// Stdio-based transport for child process DAP servers.
pub struct StdioTransport {
    child: Option<Child>,
    reader: BufReader<ChildStdout>,
    writer: ChildStdin,
}

impl StdioTransport {
    /// Spawn a new DAP server process and create a stdio transport.
    ///
    /// The `command` is split using `shell-words` for proper argument parsing.
    pub fn spawn(command: &str, args: &[String]) -> Result<Self, DapzError> {
        let parts: Vec<String> = shell_words::split(command)
            .map_err(|e| DapzError::Config(format!("invalid command '{command}': {e}")))?;
        let mut iter = parts.into_iter();
        let program = iter
            .next()
            .ok_or_else(|| DapzError::Config("empty backend command".into()))?;
        let mut cmd_args: Vec<String> = iter.collect();
        cmd_args.extend_from_slice(args);

        let mut child = tokio::process::Command::new(&program)
            .args(&cmd_args)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::inherit())
            .kill_on_drop(true)
            .spawn()
            .map_err(DapzError::Io)?;

        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| DapzError::Protocol("failed to capture stdout".into()))?;
        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| DapzError::Protocol("failed to capture stdin".into()))?;

        Ok(Self {
            child: Some(child),
            reader: BufReader::new(stdout),
            writer: stdin,
        })
    }
}

#[async_trait::async_trait]
impl Transport for StdioTransport {
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
        Ok(self
            .child
            .as_mut()
            .and_then(|c| c.try_wait().ok())
            .flatten())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_spawn_nonexistent_command() {
        let result = StdioTransport::spawn("nonexistent-debugger-12345", &[]);
        assert!(result.is_err());
    }
}
