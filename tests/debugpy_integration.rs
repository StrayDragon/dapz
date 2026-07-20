//! Integration test with debugpy DAP server.
//!
//! Performs a real DAP session with debugpy and verifies that interceptors
//! correctly compress responses. Requires `debugpy-adapter` on PATH.
//!
//! Run with:
//! ```bash
//! cargo test --test debugpy_integration -- --ignored
//! ```

use std::time::Duration;

use dapz::codec::json_rpc::DapMessage;
use dapz::interceptors::capping::CappingInterceptor;
use dapz::interceptors::evaluate::EvaluateCompressor;
use dapz::interceptors::output::OutputCompressor;
use dapz::interceptors::stacktrace::StackTraceCompressor;
use dapz::interceptors::variables::VariablesCompressor;
use dapz::proxy::Direction;

mod common;

fn debugpy_adapter_bin() -> String {
    std::env::var("DAPZ_DEBUGPY_BACKEND")
        .ok()
        .filter(|s| !s.is_empty() && !s.contains(' '))
        .or_else(dapz::resolve_python_debug_adapter)
        .filter(|s| !s.contains(' '))
        .unwrap_or_else(|| "debugpy-adapter".into())
}

/// Create a test Python script with functions, variables, and arrays.
fn create_test_script() -> (std::path::PathBuf, tempfile::TempDir) {
    common::create_test_script(
        r#"
def compute(x, y):
    result = x + y
    name = "hello world this is a test"
    items = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10]
    return result

def main():
    a = 42
    b = "a string value"
    val = compute(a, 100)
    print(val)

main()
"#,
    )
}

/// Build the standard interceptor chain for testing.
fn build_test_chain() -> dapz::interceptors::InterceptorChain {
    let config = std::sync::Arc::new(tokio::sync::RwLock::new(dapz::Config {
        backend_cmd: "debugpy-adapter".into(),
        capping: dapz::CappingConfig {
            max_frames: 0,
            max_variables: 0,
            max_output_length: 0,
            max_evaluate_length: 500,
            max_value_length: 120,
        },
        enable_output_compress: true,
        enable_variables_compress: true,
        enable_stacktrace_compress: true,
        enable_evaluate_compress: true,
        enable_scopes_compress: true,
        output_format: dapz::OutputFormat::Json,
        log_level: "info".into(),
    }));

    let interceptors: Vec<Box<dyn dapz::interceptors::Interceptor>> = vec![
        Box::new(CappingInterceptor::new(0, 0, 0)),
        Box::new(OutputCompressor),
        Box::new(EvaluateCompressor::new(500)),
        Box::new(VariablesCompressor::new(120)),
        Box::new(StackTraceCompressor),
    ];

    dapz::interceptors::InterceptorChain::new(interceptors, config)
}

/// Run a single message through the interceptor chain.
async fn run_through_chain(msg: DapMessage) -> DapMessage {
    let chain = build_test_chain();
    chain
        .process(msg, Direction::ServerToClient)
        .await
        .expect("chain processing should not fail")
        .expect("chain should not drop message")
}

/// Helper to run synchronous DAP session in a blocking thread pool.
async fn run_dap_session<F, R>(f: F) -> R
where
    F: FnOnce() -> R + Send + 'static,
    R: Send + 'static,
{
    tokio::time::timeout(Duration::from_secs(15), tokio::task::spawn_blocking(f))
        .await
        .expect("DAP session timed out (15s)")
        .expect("DAP session panicked")
}

#[tokio::test]
#[ignore = "requires debugpy-adapter on PATH"]
async fn test_debugpy_handshake_succeeds() {
    let capabilities = run_dap_session(move || {
        let (script_path, _dir) = create_test_script();
        let adapter = debugpy_adapter_bin();
        let mut session = common::DapSession::spawn(&adapter, &[])
            .unwrap_or_else(|e| panic!("spawn {adapter}: {e}"));
        let caps = common::perform_handshake(&mut session, script_path.to_str().unwrap());
        session.kill().ok();
        caps
    })
    .await;

    assert!(
        capabilities.as_object().map(|o| o.len()).unwrap_or(0) > 0,
        "should receive capabilities from initialize"
    );
}

#[tokio::test]
#[ignore = "requires debugpy-adapter on PATH"]
async fn test_debugpy_stacktrace_compression() {
    let (stacktrace_resp, frame_id) = run_dap_session(move || {
        let (script_path, _dir) = create_test_script();
        let adapter = debugpy_adapter_bin();
        let mut session =
            common::DapSession::spawn(&adapter, &[]).unwrap_or_else(|e| panic!("spawn {adapter}: {e}"));
        let _caps = common::perform_handshake(&mut session, script_path.to_str().unwrap());

        // Send stackTrace request
        session
            .send(
                r#"{"seq":10,"type":"request","command":"stackTrace","arguments":{"threadId":1,"startFrame":0,"levels":20}}"#,
            )
            .expect("send stackTrace");

        let raw = session.recv_message().expect("recv stackTrace");
        let parsed: DapMessage = serde_json::from_str(&raw).expect("parse stackTrace response");
        let frame_id = parsed
            .body
            .as_ref()
            .and_then(|b| b.get("stackFrames"))
            .and_then(|f| f.as_array())
            .and_then(|f| f.first())
            .and_then(|f| f.get("id"))
            .and_then(|v| v.as_u64())
            .unwrap_or(0);

        session.kill().ok();
        (raw, frame_id)
    })
    .await;

    // Process through interceptor chain
    let parsed: DapMessage = serde_json::from_str(&stacktrace_resp).expect("parse");
    let processed = run_through_chain(parsed).await;

    // Verify compression
    let frames = processed
        .body
        .as_ref()
        .and_then(|b| b.get("stackFrames"))
        .and_then(|v| v.as_array());
    assert!(frames.is_some(), "should have stackFrames");
    if let Some(frames) = frames {
        for frame in frames {
            let obj = frame.as_object().unwrap();
            assert!(
                !obj.contains_key("instructionPointerReference"),
                "instructionPointerReference should be pruned"
            );
            assert!(!obj.contains_key("moduleId"), "moduleId should be pruned");
        }
    }

    assert!(frame_id > 0, "should have a valid frame ID: {frame_id}");
}

#[tokio::test]
#[ignore = "requires debugpy-adapter on PATH"]
async fn test_debugpy_variables_and_evaluate_compression() {
    let (stacktrace_resp, variables_resp, evaluate_resp) = run_dap_session(move || {
        let (script_path, _dir) = create_test_script();
        let adapter = debugpy_adapter_bin();
        let mut session =
            common::DapSession::spawn(&adapter, &[]).unwrap_or_else(|e| panic!("spawn {adapter}: {e}"));
        let _caps = common::perform_handshake(&mut session, script_path.to_str().unwrap());

        // stackTrace
        session
            .send(
                r#"{"seq":10,"type":"request","command":"stackTrace","arguments":{"threadId":1,"startFrame":0,"levels":20}}"#,
            )
            .expect("send stackTrace");
        let stack_raw = session.recv_message().expect("recv stackTrace");

        // Get frameId
        let stack_parsed: DapMessage =
            serde_json::from_str(&stack_raw).expect("parse stackTrace");
        let frame_id = stack_parsed
            .body
            .as_ref()
            .and_then(|b| b.get("stackFrames"))
            .and_then(|f| f.as_array())
            .and_then(|f| f.first())
            .and_then(|f| f.get("id"))
            .and_then(|v| v.as_u64())
            .unwrap_or(0);

        // scopes
        if frame_id > 0 {
            session
                .send(&format!(
                    r#"{{"seq":11,"type":"request","command":"scopes","arguments":{{"frameId":{frame_id}}}}}"#
                ))
                .expect("send scopes");
        }
        let scopes_resp = session.recv_message().expect("recv scopes");

        let scopes_parsed: DapMessage =
            serde_json::from_str(&scopes_resp).expect("parse scopes");
        let var_ref = scopes_parsed
            .body
            .as_ref()
            .and_then(|b| b.get("scopes"))
            .and_then(|s| s.as_array())
            .and_then(|s| s.first())
            .and_then(|s| s.get("variablesReference"))
            .and_then(|v| v.as_u64())
            .unwrap_or(0);

        // variables
        let variables_raw = if var_ref > 0 {
            session
                .send(&format!(
                    r#"{{"seq":12,"type":"request","command":"variables","arguments":{{"variablesReference":{var_ref}}}}}"#
                ))
                .expect("send variables");
            session.recv_message().expect("recv variables")
        } else {
            String::new()
        };

        // evaluate
        let evaluate_raw = if frame_id > 0 {
            session
                .send(&format!(
                    r#"{{"seq":13,"type":"request","command":"evaluate","arguments":{{"expression":"a + 100","frameId":{frame_id},"context":"hover"}}}}"#
                ))
                .expect("send evaluate");
            session.recv_message().expect("recv evaluate")
        } else {
            String::new()
        };

        session.kill().ok();
        (stack_raw, variables_raw, evaluate_raw)
    })
    .await;

    // Verify stackTrace compression
    let parsed: DapMessage = serde_json::from_str(&stacktrace_resp).expect("parse");
    let processed = run_through_chain(parsed).await;
    let frames = processed
        .body
        .as_ref()
        .and_then(|b| b.get("stackFrames"))
        .and_then(|v| v.as_array());
    if let Some(frames) = frames {
        for frame in frames {
            assert!(
                !frame
                    .as_object()
                    .unwrap()
                    .contains_key("instructionPointerReference")
            );
        }
    }

    // Verify variables compression
    if !variables_resp.is_empty() {
        let parsed: DapMessage = serde_json::from_str(&variables_resp).expect("parse");
        let processed = run_through_chain(parsed).await;
        if let Some(ref body) = processed.body
            && let Some(vars) = body.get("variables").and_then(|v| v.as_array())
        {
            for var in vars {
                let obj = var.as_object().unwrap();
                assert!(!obj.contains_key("memoryReference"));
                assert!(!obj.contains_key("type"));
                let name = var.get("name").and_then(|v| v.as_str()).unwrap_or("");
                assert!(name.contains(": "), "name should have type prefix: {name}");
            }
        }
    }

    // Verify evaluate compression
    if !evaluate_resp.is_empty() {
        let parsed: DapMessage = serde_json::from_str(&evaluate_resp).expect("parse");
        let processed = run_through_chain(parsed).await;
        assert!(
            !processed
                .body
                .as_ref()
                .map(|b| b.as_object().unwrap().contains_key("memoryReference"))
                .unwrap_or(false),
            "memoryReference should be pruned from evaluate"
        );
    }
}
