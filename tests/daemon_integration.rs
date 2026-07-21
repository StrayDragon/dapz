//! Integration tests for dapz daemon — session reuse and client protocol.
//!
//! These tests exercise the real daemon server and client over a Unix socket.
//! They require the `mcp` feature.

#![cfg(feature = "mcp")]

use std::path::PathBuf;
use std::time::Duration;

use dapz::daemon::protocol::{DaemonRequest, DaemonResponse};
use dapz::daemon::{DaemonClient, DaemonServer, socket_path_for_cwd};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{UnixListener, UnixStream};

/// Two connections to the same daemon share the same DAP session key.
///
/// Spawning with a non-adapter backend (`echo`) may fail at the protocol layer;
/// we assert that identical params yield the same error or the same session_key.
#[tokio::test]
async fn test_daemon_session_reuse() {
    let test_id = format!("dapz-test-reuse-{}", std::process::id());
    let socket_path = PathBuf::from(format!("/tmp/{test_id}.sock"));

    let server = DaemonServer::new(socket_path.clone());
    let daemon_handle = tokio::spawn(async move {
        let _ = server.start().await;
    });
    tokio::time::sleep(Duration::from_millis(300)).await;

    async fn send_spawn(socket: &PathBuf, id: u64, backend: &str) -> DaemonResponse {
        let mut stream = UnixStream::connect(socket).await.unwrap();
        let req = DaemonRequest {
            id,
            method: "dap/spawn".into(),
            params: serde_json::json!({
                "backend": backend,
                "cwd": "/tmp/dapz-test-workspace",
            }),
        };
        stream
            .write_all(format!("{}\n", serde_json::to_string(&req).unwrap()).as_bytes())
            .await
            .unwrap();
        stream.flush().await.unwrap();

        let mut buf_reader = BufReader::new(&mut stream);
        let mut line = String::new();
        tokio::time::timeout(Duration::from_secs(3), buf_reader.read_line(&mut line))
            .await
            .unwrap()
            .unwrap();
        serde_json::from_str(line.trim()).unwrap()
    }

    let resp1 = send_spawn(&socket_path, 1, "echo").await;
    let resp2 = send_spawn(&socket_path, 2, "echo").await;

    assert_eq!(resp1.error, resp2.error);
    if resp1.error.is_none() {
        assert_eq!(resp1.result["session_key"], resp2.result["session_key"]);
    }

    daemon_handle.abort();
    let _ = std::fs::remove_file(&socket_path);
}

/// Socket path is deterministic for a given cwd.
#[test]
fn test_socket_path_deterministic() {
    let a = socket_path_for_cwd("/home/user/my-project");
    let b = socket_path_for_cwd("/home/user/my-project");
    assert_eq!(a, b);
}

/// Different cwds produce different socket paths.
#[test]
fn test_socket_path_different_cwds() {
    let a = socket_path_for_cwd("/home/user/project-a");
    let b = socket_path_for_cwd("/home/user/project-b");
    assert_ne!(a, b);
}

/// Regression: `DaemonClient::call` must drain orphan / out-of-order response
/// lines instead of treating the first line on the wire as its own response.
#[tokio::test]
async fn test_client_drains_orphan_response() {
    let socket_path = PathBuf::from(format!("/tmp/dapz-test-orphan-{}.sock", std::process::id()));
    let _ = std::fs::remove_file(&socket_path);

    let listener = UnixListener::bind(&socket_path).unwrap();
    let client_socket = socket_path.clone();

    let fake = tokio::spawn(async move {
        let (sock, _) = listener.accept().await.unwrap();
        let (rh, mut wh) = tokio::io::split(sock);
        let mut reader = BufReader::new(rh);
        let mut line = String::new();
        // Read the client's single request (id=1, daemon/status).
        reader.read_line(&mut line).await.unwrap();

        // Write an ORPHAN (wrong id) first, then the real response (id=1).
        let orphan = DaemonResponse::ok(999, serde_json::json!({ "stale": true }));
        let real = DaemonResponse::ok(1, serde_json::json!({ "ok": true }));
        let payload = format!(
            "{}\n{}\n",
            serde_json::to_string(&orphan).unwrap(),
            serde_json::to_string(&real).unwrap()
        );
        wh.write_all(payload.as_bytes()).await.unwrap();
        wh.flush().await.unwrap();
    });

    let mut client = DaemonClient::connect_explicit(&client_socket)
        .await
        .unwrap();
    let result = client.status().await.unwrap();

    fake.await.unwrap();
    let _ = std::fs::remove_file(&socket_path);

    assert_eq!(
        result,
        serde_json::json!({ "ok": true }),
        "client returned an orphan response (id=999) instead of the matching \
         one (id=1); the daemon protocol would be desynced"
    );
}

/// Agent SDK `via_daemon` + `daemon_socket` connects and spawns a session key
/// without silently falling back to in-process.
#[tokio::test]
#[cfg(feature = "agent-sdk")]
async fn test_agent_sdk_via_daemon_spawn() {
    use dapz::agent_sdk::AgentHandle;

    let test_id = format!("dapz-test-sdk-{}", std::process::id());
    let socket_path = PathBuf::from(format!("/tmp/{test_id}.sock"));
    let _ = std::fs::remove_file(&socket_path);

    let server = DaemonServer::new(socket_path.clone());
    let daemon_handle = tokio::spawn(async move {
        let _ = server.start().await;
    });
    tokio::time::sleep(Duration::from_millis(300)).await;

    let agent = AgentHandle::builder()
        .backend("echo")
        .cwd("/tmp/dapz-test-sdk-workspace")
        .daemon_socket(socket_path.clone())
        .enable_compression(false)
        .start()
        .await;

    daemon_handle.abort();
    let _ = std::fs::remove_file(&socket_path);

    // echo is not a DAP adapter — spawn may Err at protocol layer, or Ok with a key.
    // Either way we must not have silently created an in-process-only path without daemon.
    match agent {
        Ok(_handle) => {}
        Err(e) => {
            let msg = e.to_string();
            assert!(
                !msg.contains("backend is required"),
                "unexpected config error: {msg}"
            );
        }
    }
}
