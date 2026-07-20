//! Daemon client — connect to a running dapz daemon (optional auto-start).

use std::io::ErrorKind;
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
    socket_path: PathBuf,
    reader: BufReader<tokio::io::ReadHalf<UnixStream>>,
    writer: tokio::io::WriteHalf<UnixStream>,
    next_id: u64,
    /// Whether this client is responsible for daemon lifecycle.
    owns_daemon: bool,
}

impl DaemonClient {
    /// Connect to an existing daemon for `cwd`, or spawn `dapz daemon` in the background.
    pub async fn connect_or_start(cwd: &str) -> Result<Self, anyhow::Error> {
        let socket_path = socket_path_for_cwd(cwd);

        // Try connecting to an existing daemon first.
        match UnixStream::connect(&socket_path).await {
            Ok(stream) => {
                info!(path = %socket_path.display(), "Connected to existing dapz daemon");
                return Ok(Self::from_stream(stream, socket_path, false));
            }
            Err(_) => {
                // Daemon not running — auto-start.
            }
        }

        info!(path = %socket_path.display(), "No daemon found, auto-starting");
        spawn_background_daemon(&socket_path, cwd)?;

        let start = std::time::Instant::now();
        while start.elapsed() < DAEMON_STARTUP_TIMEOUT {
            tokio::time::sleep(Duration::from_millis(200)).await;
            match UnixStream::connect(&socket_path).await {
                Ok(stream) => {
                    info!("Connected to freshly spawned dapz daemon");
                    return Ok(Self::from_stream(stream, socket_path, true));
                }
                Err(e)
                    if e.kind() == ErrorKind::ConnectionRefused
                        || e.kind() == ErrorKind::NotFound =>
                {
                    continue;
                }
                Err(e) => {
                    warn!(error = %e, "Unexpected error connecting to daemon");
                    continue;
                }
            }
        }
        Err(anyhow::anyhow!(
            "timed out waiting for daemon at {}",
            socket_path.display()
        ))
    }

    /// Connect to a daemon at a specific socket path (for testing).
    pub async fn connect_explicit(socket: &PathBuf) -> Result<Self, anyhow::Error> {
        let stream = UnixStream::connect(socket)
            .await
            .with_context(|| format!("Cannot connect to daemon at {:?}", socket))?;
        Ok(Self::from_stream(stream, socket.clone(), false))
    }

    /// Check if daemon is already running (without auto-start).
    pub async fn try_connect(cwd: &str) -> Result<Option<Self>, anyhow::Error> {
        let socket_path = socket_path_for_cwd(cwd);
        match UnixStream::connect(&socket_path).await {
            Ok(stream) => Ok(Some(Self::from_stream(stream, socket_path, false))),
            Err(_) => Ok(None),
        }
    }

    fn from_stream(stream: UnixStream, socket_path: PathBuf, owns_daemon: bool) -> Self {
        let (rh, wh) = tokio::io::split(stream);
        Self {
            socket_path,
            reader: BufReader::new(rh),
            writer: wh,
            next_id: 1,
            owns_daemon,
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
            tokio::time::timeout(Duration::from_secs(30), self.reader.read_line(&mut line))
                .await
                .context("Timeout waiting for daemon response")?
                .context("Daemon connection closed")?;
            if line.trim().is_empty() {
                continue;
            }
            let resp: DaemonResponse = serde_json::from_str(line.trim())
                .with_context(|| format!("invalid daemon response: {line}"))?;
            if resp.id != id {
                warn!(
                    expected = id,
                    got = resp.id,
                    error = ?resp.error,
                    "Drained orphan/out-of-order daemon response from a cancelled or \
                     earlier request; protocol stays in sync",
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

impl Drop for DaemonClient {
    fn drop(&mut self) {
        if !self.owns_daemon {
            debug!("DaemonClient dropped (owns_daemon=false)");
            return;
        }
        let socket = self.socket_path.clone();
        debug!(
            ?socket,
            "DaemonClient dropped (owns_daemon=true); requesting shutdown"
        );
        // Best-effort: Drop cannot await. Open a fresh connection on a helper
        // thread so we do not block the runtime that may still own this client.
        std::thread::Builder::new()
            .name("dapz-daemon-shutdown".into())
            .spawn(move || {
                let rt = match tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                {
                    Ok(rt) => rt,
                    Err(e) => {
                        warn!(error = %e, "Failed to build runtime for daemon shutdown");
                        return;
                    }
                };
                rt.block_on(async {
                    match DaemonClient::connect_explicit(&socket).await {
                        Ok(mut client) => {
                            if let Err(e) = client.shutdown().await {
                                warn!(error = %e, "daemon/shutdown after Drop failed");
                            }
                        }
                        Err(e) => {
                            debug!(error = %e, "Could not connect to shut down owned daemon");
                        }
                    }
                });
            })
            .ok();
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
