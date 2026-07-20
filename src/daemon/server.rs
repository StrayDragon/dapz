//! Daemon server — long-lived DAP adapter session manager.

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use anyhow::Context;
use serde_json::json;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{UnixListener, UnixStream};
use tokio::sync::{Mutex, watch};
use tracing::{debug, error, info, warn};

use super::protocol::{
    DaemonRequest, DaemonResponse, DapRequestParams, InvokeParams, RemoveParams, SpawnParams,
    WaitEventParams,
};
use super::status::DaemonStatus;
use crate::mcp::{DapPool, DapSession, pool_key};
use serde_json::Value;

/// The daemon server.
pub struct DaemonServer {
    socket_path: PathBuf,
    pool: Arc<Mutex<DapPool>>,
    status: Arc<Mutex<DaemonStatus>>,
    active_connections: Arc<AtomicU64>,
}

impl DaemonServer {
    /// Create a new daemon server bound to `socket_path`.
    pub fn new(socket_path: PathBuf) -> Self {
        Self {
            socket_path,
            pool: Arc::new(Mutex::new(DapPool::new())),
            status: Arc::new(Mutex::new(DaemonStatus::new())),
            active_connections: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Listen until SIGINT or `daemon/shutdown`.
    pub async fn start(self) -> Result<(), anyhow::Error> {
        if self.socket_path.exists() {
            std::fs::remove_file(&self.socket_path).with_context(|| {
                format!("Failed to remove stale socket: {:?}", self.socket_path)
            })?;
        }
        if let Some(parent) = self.socket_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let listener = UnixListener::bind(&self.socket_path)
            .with_context(|| format!("Failed to bind to {:?}", self.socket_path))?;

        let (shutdown_tx, mut shutdown_rx) = watch::channel(false);
        {
            let tx = shutdown_tx.clone();
            tokio::spawn(async move {
                tokio::signal::ctrl_c().await.ok();
                info!("Daemon received shutdown signal");
                let _ = tx.send(true);
            });
        }

        info!(path = %self.socket_path.display(), "DAP daemon listening");

        let pool = self.pool.clone();
        let status = self.status.clone();
        let active_connections = self.active_connections.clone();

        loop {
            tokio::select! {
                _ = shutdown_rx.changed() => {
                    if *shutdown_rx.borrow() {
                        info!("Daemon shutdown requested, tearing down");
                        break;
                    }
                }
                accepted = listener.accept() => {
                    match accepted {
                        Ok((stream, _)) => {
                            active_connections.fetch_add(1, Ordering::Relaxed);
                            status.lock().await.record_connection();
                            let pool = pool.clone();
                            let status = status.clone();
                            let conns = active_connections.clone();
                            let shutdown_tx = shutdown_tx.clone();
                            tokio::spawn(async move {
                                let _guard = ConnectionGuard(conns);
                                if let Err(e) =
                                    handle_client(stream, pool, status, shutdown_tx).await
                                {
                                    warn!(error = %e, "Client handler exited with error");
                                }
                            });
                        }
                        Err(e) => error!(error = %e, "Daemon accept error"),
                    }
                }
            }
        }

        pool.lock().await.clear();
        status.lock().await.sessions.clear();
        if self.socket_path.exists() {
            let _ = std::fs::remove_file(&self.socket_path);
        }
        info!(path = %self.socket_path.display(), "Daemon stopped cleanly");
        Ok(())
    }
}

struct ConnectionGuard(Arc<AtomicU64>);

impl Drop for ConnectionGuard {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::Relaxed);
    }
}

async fn handle_client(
    stream: UnixStream,
    pool: Arc<Mutex<DapPool>>,
    status: Arc<Mutex<DaemonStatus>>,
    shutdown_tx: watch::Sender<bool>,
) -> Result<(), anyhow::Error> {
    let (rh, mut wh) = tokio::io::split(stream);
    let mut reader = BufReader::new(rh);
    let mut line = String::new();

    loop {
        line.clear();
        let n = reader.read_line(&mut line).await?;
        if n == 0 {
            break;
        }
        let req: DaemonRequest = match serde_json::from_str(line.trim()) {
            Ok(r) => r,
            Err(e) => {
                let resp = DaemonResponse::err(0, format!("invalid request JSON: {e}"));
                write_response(&mut wh, &resp).await?;
                continue;
            }
        };

        status.lock().await.record_request();
        debug!(method = %req.method, id = req.id, "daemon RPC");

        let resp = dispatch(req, &pool, &status, &shutdown_tx).await;
        write_response(&mut wh, &resp).await?;
        if resp.error.is_none() && matches_shutdown(&resp) {
            break;
        }
    }
    Ok(())
}

fn matches_shutdown(resp: &DaemonResponse) -> bool {
    resp.result
        .get("shutdown")
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
}

async fn write_response(
    writer: &mut tokio::io::WriteHalf<UnixStream>,
    resp: &DaemonResponse,
) -> Result<(), anyhow::Error> {
    let mut bytes = serde_json::to_vec(resp)?;
    bytes.push(b'\n');
    writer.write_all(&bytes).await?;
    writer.flush().await?;
    Ok(())
}

async fn dispatch(
    req: DaemonRequest,
    pool: &Arc<Mutex<DapPool>>,
    status: &Arc<Mutex<DaemonStatus>>,
    shutdown_tx: &watch::Sender<bool>,
) -> DaemonResponse {
    match req.method.as_str() {
        "dap/spawn" => match serde_json::from_value::<SpawnParams>(req.params) {
            Ok(params) => {
                let key = pool_key(&params.backend, params.cwd.as_deref());
                let mut p = pool.lock().await;
                if params.replace {
                    p.remove(&key);
                    let mut st = status.lock().await;
                    st.sessions.retain(|s| s != &key);
                }
                match p
                    .get_or_spawn(&params.backend, params.cwd.as_deref(), &params.extra_args)
                    .await
                {
                    Ok(_) => {
                        let mut st = status.lock().await;
                        if !st.sessions.contains(&key) {
                            st.sessions.push(key.clone());
                        }
                        DaemonResponse::ok(req.id, json!({ "session_key": key }))
                    }
                    Err(e) => DaemonResponse::err(req.id, e.to_string()),
                }
            }
            Err(e) => DaemonResponse::err(req.id, e.to_string()),
        },
        "dap/remove" => match serde_json::from_value::<RemoveParams>(req.params) {
            Ok(params) => {
                pool.lock().await.remove(&params.session_key);
                let mut st = status.lock().await;
                st.sessions.retain(|s| s != &params.session_key);
                DaemonResponse::ok(req.id, json!({ "removed": true }))
            }
            Err(e) => DaemonResponse::err(req.id, e.to_string()),
        },
        "dap/invoke" => match serde_json::from_value::<InvokeParams>(req.params) {
            Ok(params) => {
                let session = {
                    let p = pool.lock().await;
                    p.get_by_key(&params.session_key)
                };
                match session {
                    Some(s) => {
                        let mut guard = s.lock().await;
                        match invoke_session_op(&mut guard, &params.op, params.args).await {
                            Ok(body) => {
                                if params.op == "disconnect" {
                                    pool.lock().await.remove(&params.session_key);
                                    let mut st = status.lock().await;
                                    st.sessions.retain(|k| k != &params.session_key);
                                }
                                DaemonResponse::ok(req.id, body)
                            }
                            Err(e) => DaemonResponse::err(req.id, e),
                        }
                    }
                    None => DaemonResponse::err(
                        req.id,
                        format!("unknown session_key '{}'", params.session_key),
                    ),
                }
            }
            Err(e) => DaemonResponse::err(req.id, e.to_string()),
        },
        "dap/request" => match serde_json::from_value::<DapRequestParams>(req.params) {
            Ok(params) => {
                let session = {
                    let p = pool.lock().await;
                    p.get_by_key(&params.session_key)
                };
                match session {
                    Some(s) => {
                        let mut guard = s.lock().await;
                        match guard.send_request(&params.command, params.arguments).await {
                            Ok(body) => DaemonResponse::ok(req.id, body),
                            Err(e) => DaemonResponse::err(req.id, e.to_string()),
                        }
                    }
                    None => DaemonResponse::err(
                        req.id,
                        format!("unknown session_key '{}'", params.session_key),
                    ),
                }
            }
            Err(e) => DaemonResponse::err(req.id, e.to_string()),
        },
        "dap/wait_event" => match serde_json::from_value::<WaitEventParams>(req.params) {
            Ok(params) => {
                let timeout = Duration::from_millis(params.timeout_ms.unwrap_or(30_000));
                let session = {
                    let p = pool.lock().await;
                    p.get_by_key(&params.session_key)
                };
                match session {
                    Some(s) => {
                        let mut guard = s.lock().await;
                        match guard
                            .wait_for_event_where(&params.event, |_| true, timeout)
                            .await
                        {
                            Ok(body) => DaemonResponse::ok(req.id, body),
                            Err(e) => DaemonResponse::err(req.id, e.to_string()),
                        }
                    }
                    None => DaemonResponse::err(
                        req.id,
                        format!("unknown session_key '{}'", params.session_key),
                    ),
                }
            }
            Err(e) => DaemonResponse::err(req.id, e.to_string()),
        },
        "daemon/status" => {
            let st = status.lock().await;
            let keys = pool.lock().await.keys();
            DaemonResponse::ok(
                req.id,
                json!({
                    "uptime_secs": st.uptime_secs(),
                    "total_connections": st.total_connections,
                    "total_requests": st.total_requests,
                    "sessions": keys,
                }),
            )
        }
        "daemon/shutdown" => {
            let _ = shutdown_tx.send(true);
            DaemonResponse::ok(req.id, json!({ "shutdown": true }))
        }
        other => DaemonResponse::err(req.id, format!("unknown method: {other}")),
    }
}

async fn invoke_session_op(
    session: &mut DapSession,
    op: &str,
    args: Value,
) -> Result<Value, String> {
    let map_err = |e: crate::error::DapzError| e.to_string();
    match op {
        "launch_program" => {
            let program = args
                .get("program")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "launch_program requires args.program".to_string())?;
            let cwd = args.get("cwd").and_then(|v| v.as_str());
            let prog_args: Option<Vec<String>> = args
                .get("args")
                .cloned()
                .and_then(|v| serde_json::from_value(v).ok());
            let breakpoints: Option<Vec<(String, Vec<i64>)>> = args
                .get("breakpoints")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|bp| {
                            let path = bp.get("path")?.as_str()?.to_string();
                            let lines: Vec<i64> = bp
                                .get("lines")?
                                .as_array()?
                                .iter()
                                .filter_map(|l| l.as_i64())
                                .collect();
                            Some((path, lines))
                        })
                        .collect()
                });
            session
                .launch_program(program, cwd, prog_args.as_deref(), breakpoints.as_deref())
                .await
                .map_err(map_err)
        }
        "attach" => {
            let attach_args = args.get("arguments").cloned().unwrap_or(Value::Null);
            session.attach(attach_args).await.map_err(map_err)
        }
        "set_breakpoints" => {
            let path = args
                .get("source")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "set_breakpoints requires args.source".to_string())?;
            let lines: Vec<i64> = args
                .get("lines")
                .and_then(|v| v.as_array())
                .map(|a| a.iter().filter_map(|l| l.as_i64()).collect())
                .unwrap_or_default();
            session.set_breakpoints(path, &lines).await.map_err(map_err)
        }
        "set_exception_breakpoints" => {
            let filters: Vec<String> = args
                .get("filters")
                .cloned()
                .and_then(|v| serde_json::from_value(v).ok())
                .unwrap_or_default();
            session
                .set_exception_breakpoints(&filters)
                .await
                .map_err(map_err)
        }
        "continue" => session
            .continue_(args.get("thread_id").and_then(|v| v.as_i64()))
            .await
            .map_err(map_err),
        "step_over" => session
            .step_over(args.get("thread_id").and_then(|v| v.as_i64()))
            .await
            .map_err(map_err),
        "step_into" => session
            .step_into(args.get("thread_id").and_then(|v| v.as_i64()))
            .await
            .map_err(map_err),
        "step_out" => session
            .step_out(args.get("thread_id").and_then(|v| v.as_i64()))
            .await
            .map_err(map_err),
        "pause" => session
            .pause(args.get("thread_id").and_then(|v| v.as_i64()))
            .await
            .map_err(map_err),
        "get_threads" => session.get_threads().await.map_err(map_err),
        "get_stack" => session
            .get_stack(
                args.get("thread_id").and_then(|v| v.as_i64()),
                args.get("levels").and_then(|v| v.as_i64()),
            )
            .await
            .map_err(map_err),
        "get_scopes" => {
            let frame_id = args
                .get("frame_id")
                .and_then(|v| v.as_i64())
                .ok_or_else(|| "get_scopes requires args.frame_id".to_string())?;
            session.get_scopes(frame_id).await.map_err(map_err)
        }
        "get_variables" => {
            let vr = args
                .get("variables_reference")
                .and_then(|v| v.as_i64())
                .ok_or_else(|| "get_variables requires args.variables_reference".to_string())?;
            session.get_variables(vr).await.map_err(map_err)
        }
        "evaluate" => {
            let expression = args
                .get("expression")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "evaluate requires args.expression".to_string())?;
            session
                .evaluate(
                    expression,
                    args.get("frame_id").and_then(|v| v.as_i64()),
                    args.get("context").and_then(|v| v.as_str()),
                )
                .await
                .map_err(map_err)
        }
        "get_exception" => session
            .get_exception_info(args.get("thread_id").and_then(|v| v.as_i64()))
            .await
            .map_err(map_err),
        "get_source" => {
            let source_reference = args
                .get("source_reference")
                .and_then(|v| v.as_i64())
                .ok_or_else(|| "get_source requires args.source_reference".to_string())?;
            session
                .get_source(source_reference, args.get("path").and_then(|v| v.as_str()))
                .await
                .map_err(map_err)
        }
        "drain_output" => Ok(json!({ "outputs": session.drain_output() })),
        "wait_stopped" => {
            let timeout_ms = args
                .get("timeout_ms")
                .and_then(|v| v.as_u64())
                .unwrap_or(30_000);
            session
                .wait_stopped(Duration::from_millis(timeout_ms))
                .await
                .map_err(map_err)
        }
        "disconnect" => session
            .disconnect(args.get("terminate_debuggee").and_then(|v| v.as_bool()))
            .await
            .map_err(map_err),
        "terminate" => session.terminate().await.map_err(map_err),
        "send_raw" => {
            let command = args
                .get("command")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "send_raw requires args.command".to_string())?;
            let arguments = args.get("arguments").cloned().unwrap_or(json!({}));
            session.send_raw(command, arguments).await.map_err(map_err)
        }
        other => Err(format!("unknown dap/invoke op: {other}")),
    }
}
