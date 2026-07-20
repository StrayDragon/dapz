//! Daemon client — connect to a running dapz daemon (optional auto-start).

use std::path::PathBuf;
use std::time::Duration;

use anyhow::Context;
use serde_json::Value;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;
use tracing::{debug, info, warn};

use super::protocol::{
    DaemonRequest, DaemonResponse, DapRequestParams, InvokeParams, RemoveParams, SpawnParams,
    WaitEventParams,
};
use super::socket::socket_path_for_cwd;

const DAEMON_STARTUP_TIMEOUT: Duration = Duration::from_secs(10);

/// Client connection to a dapz daemon.
pub struct DaemonClient {
    reader: BufReader<tokio::io::ReadHalf<UnixStream>>,
    writer: tokio::io::WriteHalf<UnixStream>,
    next_id: u64,
}

impl DaemonClient {
    /// Connect to an existing daemon for `cwd`, or spawn `dapz daemon` in the background.
    pub async fn connect_or_start(cwd: &str) -> Result<Self, anyhow::Error> {
        let socket_path = socket_path_for_cwd(cwd);
        if let Ok(stream) = UnixStream::connect(&socket_path).await {
            info!(path = %socket_path.display(), "Connected to existing dapz daemon");
            return Ok(Self::from_stream(stream));
        }

        info!(path = %socket_path.display(), "No daemon found, auto-starting");
        spawn_background_daemon(&socket_path, cwd)?;

        let start = std::time::Instant::now();
        while start.elapsed() < DAEMON_STARTUP_TIMEOUT {
            tokio::time::sleep(Duration::from_millis(200)).await;
            if let Ok(stream) = UnixStream::connect(&socket_path).await {
                info!("Connected to freshly spawned dapz daemon");
                return Ok(Self::from_stream(stream));
            }
        }
        Err(anyhow::anyhow!(
            "timed out waiting for daemon at {}",
            socket_path.display()
        ))
    }

    fn from_stream(stream: UnixStream) -> Self {
        let (rh, wh) = tokio::io::split(stream);
        Self {
            reader: BufReader::new(rh),
            writer: wh,
            next_id: 1,
        }
    }

    async fn call(&mut self, method: &str, params: Value) -> Result<Value, anyhow::Error> {
        let id = self.next_id;
        self.next_id += 1;
        let req = DaemonRequest {
            id,
            method: method.into(),
            params,
        };
        let mut bytes = serde_json::to_vec(&req)?;
        bytes.push(b'\n');
        self.writer.write_all(&bytes).await?;
        self.writer.flush().await?;

        // Drain orphan responses (id mismatch) until ours arrives — mirrors lspz.
        loop {
            let mut line = String::new();
            self.reader.read_line(&mut line).await?;
            if line.trim().is_empty() {
                continue;
            }
            let resp: DaemonResponse = serde_json::from_str(line.trim())
                .with_context(|| format!("invalid daemon response: {line}"))?;
            if resp.id != id {
                warn!(
                    expected = id,
                    got = resp.id,
                    "discarding orphan daemon response"
                );
                continue;
            }
            if let Some(err) = resp.error {
                anyhow::bail!(err);
            }
            return Ok(resp.result);
        }
    }

    /// `dap/spawn` — returns `session_key`.
    pub async fn spawn_session(&mut self, params: SpawnParams) -> Result<String, anyhow::Error> {
        let result = self
            .call("dap/spawn", serde_json::to_value(params)?)
            .await?;
        result
            .get("session_key")
            .and_then(|v| v.as_str())
            .map(str::to_owned)
            .ok_or_else(|| anyhow::anyhow!("missing session_key in spawn response"))
    }

    /// `dap/remove`.
    pub async fn remove_session(&mut self, session_key: &str) -> Result<(), anyhow::Error> {
        let params = RemoveParams {
            session_key: session_key.into(),
        };
        let _ = self
            .call("dap/remove", serde_json::to_value(params)?)
            .await?;
        Ok(())
    }

    /// `dap/invoke` — high-level session op on the daemon.
    pub async fn invoke(
        &mut self,
        session_key: &str,
        op: &str,
        args: Value,
    ) -> Result<Value, anyhow::Error> {
        let params = InvokeParams {
            session_key: session_key.into(),
            op: op.into(),
            args,
        };
        self.call("dap/invoke", serde_json::to_value(params)?).await
    }

    /// `dap/request`.
    pub async fn dap_request(
        &mut self,
        session_key: &str,
        command: &str,
        arguments: Value,
    ) -> Result<Value, anyhow::Error> {
        let params = DapRequestParams {
            session_key: session_key.into(),
            command: command.into(),
            arguments,
        };
        self.call("dap/request", serde_json::to_value(params)?)
            .await
    }

    /// `dap/wait_event`.
    pub async fn wait_event(
        &mut self,
        session_key: &str,
        event: &str,
        timeout_ms: Option<u64>,
    ) -> Result<Value, anyhow::Error> {
        let params = WaitEventParams {
            session_key: session_key.into(),
            event: event.into(),
            timeout_ms,
        };
        self.call("dap/wait_event", serde_json::to_value(params)?)
            .await
    }

    /// `daemon/status`.
    pub async fn status(&mut self) -> Result<Value, anyhow::Error> {
        self.call("daemon/status", Value::Object(Default::default()))
            .await
    }

    /// `daemon/shutdown`.
    pub async fn shutdown(&mut self) -> Result<(), anyhow::Error> {
        let _ = self
            .call("daemon/shutdown", Value::Object(Default::default()))
            .await?;
        Ok(())
    }
}

fn spawn_background_daemon(socket_path: &PathBuf, cwd: &str) -> Result<(), anyhow::Error> {
    let exe = std::env::current_exe().context("current_exe")?;
    debug!(
        path = %socket_path.display(),
        cwd,
        "Spawning background dapz daemon"
    );
    std::process::Command::new(exe)
        .arg("daemon")
        .arg("--socket")
        .arg(socket_path)
        .arg("--cwd")
        .arg(cwd)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .context("spawn dapz daemon")?;
    Ok(())
}
