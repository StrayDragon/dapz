//! Stdio transport — child process stdio.
//!
//! Spawns a DAP server as a child process and communicates via stdin/stdout.

use std::process::ExitStatus;

use tokio::io::{AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, ChildStdout};

use crate::error::DapzError;
use crate::transport::Transport;
use crate::transport::framing::{self, FrameState};

/// Stdio-based transport for child process DAP servers.
pub struct StdioTransport {
    child: Option<Child>,
    reader: BufReader<ChildStdout>,
    writer: ChildStdin,
    frame_state: FrameState,
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
            frame_state: FrameState::new(),
        })
    }
}

#[async_trait::async_trait]
impl Transport for StdioTransport {
    async fn receive(&mut self) -> Result<Vec<u8>, DapzError> {
        framing::read_frame_with_state(&mut self.reader, &mut self.frame_state).await
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
