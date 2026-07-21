//! Agent Tier-0 debug loop against debugpy (ignored unless harness runs it).
//!
//! ```bash
//! cargo test --all-features --test agent_debug_loop -- --ignored --test-threads=1
//! ```

use std::time::Duration;

use dapz::adapters::resolve_python_debug_adapter;
use dapz::agent_sdk::AgentHandle;

#[path = "common/fixture.rs"]
mod fixture;

fn create_fixture() -> (std::path::PathBuf, tempfile::TempDir) {
    fixture::create_test_script(
        r#"
def bug():
    xs = [1, 2, 0]
    return xs[0] + xs[1]

def main():
    print("start")
    bug()
    print("end")

main()
"#,
    )
}

fn backend_cmd() -> String {
    std::env::var("DAPZ_DEBUGPY_BACKEND")
        .ok()
        .filter(|s| !s.is_empty())
        .or_else(resolve_python_debug_adapter)
        .expect("debugpy adapter not found; run: uv tool install debugpy")
}

#[tokio::test]
#[ignore = "requires debugpy (uv tool install debugpy / pip install debugpy)"]
async fn test_agent_debug_loop_breakpoint_observe() {
    let (script, _dir) = create_fixture();
    let script_str = script.to_str().unwrap().to_string();
    // Break on the return line inside bug() — approximate line 4 in the fixture.
    let breakpoints = vec![(script_str.clone(), vec![4_i64])];
    let backend = backend_cmd();

    let mut agent = AgentHandle::builder()
        .backend(&backend)
        .enable_compression(true)
        .start()
        .await
        .unwrap_or_else(|e| panic!("spawn adapter `{backend}`: {e}"));

    let stopped = tokio::time::timeout(
        Duration::from_secs(45),
        agent.launch(&script_str, None, None, Some(breakpoints.as_slice())),
    )
    .await
    .expect("launch timeout")
    .expect("launch");

    assert!(
        stopped.contains("stopped")
            || stopped.contains("breakpoint")
            || stopped.contains("threadId")
            || stopped.contains("reason"),
        "unexpected stopped payload: {stopped}"
    );

    let stack = agent.get_stack(Some(1), Some(20)).await.expect("stack");
    assert!(
        stack.contains("bug") || stack.contains("main") || stack.contains("items"),
        "stack missing frames: {stack}"
    );

    let _ = agent.get_scopes(1).await;
    let _ = agent.evaluate("1+1", Some(1), Some("repl")).await;
    let _ = agent.drain_output().await;

    let _ = agent.disconnect(Some(true)).await;
}
