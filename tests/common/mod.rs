//! Shared test utilities for DAP integration tests.
//!
//! Provides helpers for DAP transport (Content-Length framing), handshake
//! sequences, and response verification.

use std::io::{BufRead, Read, Write};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

/// Wraps a DAP server process's stdio streams.
pub struct DapSession {
    pub stdin: ChildStdin,
    reader: std::io::BufReader<ChildStdout>,
    process: Child,
}

impl DapSession {
    /// Spawn a DAP server from the given command.
    pub fn spawn(cmd: &str, args: &[&str]) -> std::io::Result<Self> {
        let mut child = Command::new(cmd)
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()?;

        let stdin = child.stdin.take().unwrap();
        let stdout = child.stdout.take().unwrap();

        Ok(Self {
            stdin,
            reader: std::io::BufReader::new(stdout),
            process: child,
        })
    }

    /// Send a JSON-RPC DAP message (Content-Length framed).
    pub fn send(&mut self, body: &str) -> std::io::Result<()> {
        let header = format!("Content-Length: {}\r\n\r\n", body.len());
        self.stdin.write_all(header.as_bytes())?;
        self.stdin.write_all(body.as_bytes())?;
        self.stdin.flush()?;
        Ok(())
    }

    /// Receive a DAP message using Content-Length framing.
    pub fn recv_message(&mut self) -> std::io::Result<String> {
        let mut content_length: Option<usize> = None;

        loop {
            let mut line = String::new();
            self.reader.read_line(&mut line)?;
            let trimmed = line.trim();
            if trimmed.is_empty() {
                break;
            }
            if let Some(len_str) = trimmed.strip_prefix("Content-Length: ") {
                content_length = len_str.trim().parse().ok();
            }
        }

        let len = content_length.unwrap_or(0);
        let mut buf = vec![0u8; len];
        if len > 0 {
            self.reader.read_exact(&mut buf)?;
        }

        Ok(String::from_utf8_lossy(&buf).into_owned())
    }

    /// Kill the DAP server process.
    pub fn kill(&mut self) -> std::io::Result<()> {
        self.process.kill()
    }
}

impl Drop for DapSession {
    fn drop(&mut self) {
        let _ = self.process.kill();
        let _ = self.process.wait();
    }
}

/// Perform the DAP handshake up to `stopped` event.
///
/// Returns the capabilities response body.
pub fn perform_handshake(session: &mut DapSession, script_path: &str) -> serde_json::Value {
    // 1. Initialize
    session
        .send(r#"{"seq":1,"type":"request","command":"initialize","arguments":{"clientID":"dapz-test","adapterID":"python","pathFormat":"path","linesStartAt1":true,"columnsStartAt1":true,"supportsVariableType":true,"supportsVariablePaging":true,"supportsRunInTerminalRequest":true,"locale":"en"}}"#)
        .expect("send initialize");

    let capabilities: serde_json::Value = loop {
        let raw = session.recv_message().expect("recv during init");
        let msg: serde_json::Value = serde_json::from_str(&raw).expect("parse DAP message");
        if msg.get("type").and_then(|v| v.as_str()) == Some("response")
            && msg.get("command").and_then(|v| v.as_str()) == Some("initialize")
        {
            break msg.get("body").cloned().unwrap_or_default();
        }
    };

    // 2. Launch (internalConsole avoids runInTerminal)
    session
        .send(&format!(
            r#"{{"seq":2,"type":"request","command":"launch","arguments":{{"program":"{script_path}","noDebug":false,"console":"internalConsole","stopOnEntry":true}}}}"#
        ))
        .expect("send launch");

    // After launch, server sends `initialized` event before responding to launch.
    // Wait for initialized, then send configurationDone.
    loop {
        let raw = session.recv_message().expect("recv during launch");
        let msg: serde_json::Value = serde_json::from_str(&raw).expect("parse DAP message");
        if msg.get("type").and_then(|v| v.as_str()) == Some("event")
            && msg.get("event").and_then(|v| v.as_str()) == Some("initialized")
        {
            break;
        }
    }

    // 3. configurationDone
    session
        .send(r#"{"seq":3,"type":"request","command":"configurationDone","arguments":{}}"#)
        .expect("send configurationDone");

    // After configurationDone, server sends launch response and program starts.
    // Wait for stopped event (entry breakpoint due to stopOnEntry).
    let mut got_launch_response = false;
    loop {
        let raw = session.recv_message().expect("recv after configDone");
        let msg: serde_json::Value = serde_json::from_str(&raw).expect("parse DAP message");
        if msg.get("type").and_then(|v| v.as_str()) == Some("event")
            && msg.get("event").and_then(|v| v.as_str()) == Some("stopped")
        {
            break;
        }
        if msg.get("type").and_then(|v| v.as_str()) == Some("response")
            && msg.get("command").and_then(|v| v.as_str()) == Some("launch")
        {
            got_launch_response = true;
        }
    }

    // Consume the launch response if not already consumed
    if !got_launch_response {
        let _raw = session.recv_message().expect("recv launch response");
    }

    capabilities
}

/// Create a temporary Python script for debugging.
pub fn create_test_script(contents: &str) -> (std::path::PathBuf, tempfile::TempDir) {
    let dir = tempfile::tempdir().expect("create temp dir");
    let script_path = dir.path().join("test_debug.py");
    std::fs::write(&script_path, contents).expect("write test script");
    (script_path, dir)
}
