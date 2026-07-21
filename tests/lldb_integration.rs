//! Optional lldb-dap e2e (ignored unless harness enables it).
//!
//! ```bash
//! cargo test --all-features --test lldb_integration -- --ignored --test-threads=1
//! ```

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use dapz::adapters::resolve_lldb_debug_adapter;
use dapz::agent_sdk::AgentHandle;

fn backend_cmd() -> String {
    std::env::var("DAPZ_LLDB_BACKEND")
        .ok()
        .filter(|s| !s.is_empty())
        .or_else(resolve_lldb_debug_adapter)
        .expect("lldb-dap/lldb-vscode not found")
}

fn compile_fixture(src: &Path, out: &Path) {
    let status = Command::new("cc")
        .args(["-g", "-O0", "-o"])
        .arg(out)
        .arg(src)
        .status()
        .expect("spawn cc");
    assert!(
        status.success(),
        "failed to compile {}: {status}",
        src.display()
    );
}

/// Pull a `threadId` integer out of TOON/JSON-ish text.
fn extract_thread_id(text: &str) -> Option<i64> {
    for marker in ["threadId:", "threadId,", "\"threadId\":"] {
        if let Some(rest) = text.split(marker).nth(1) {
            let digits: String = rest
                .trim_start()
                .chars()
                .take_while(|c| c.is_ascii_digit())
                .collect();
            if let Ok(id) = digits.parse::<i64>() {
                return Some(id);
            }
        }
    }
    None
}

#[tokio::test]
#[ignore = "requires lldb-dap (or lldb-vscode) and a C compiler (cc)"]
async fn test_lldb_launch_stack_disconnect() {
    let src = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixtures/e2e/lldb_bug.c");
    assert!(src.is_file(), "missing fixture {}", src.display());

    let tmp = tempfile::tempdir().expect("tempdir");
    let bin = tmp.path().join("lldb_bug");
    compile_fixture(&src, &bin);

    let backend = backend_cmd();
    let bin_str = bin.to_str().expect("utf8 path").to_string();
    let cwd = tmp.path().to_str().expect("utf8 cwd");

    let mut agent = AgentHandle::builder()
        .backend(&backend)
        .enable_compression(true)
        .start()
        .await
        .unwrap_or_else(|e| panic!("spawn adapter `{backend}`: {e}"));

    // No breakpoints → launch_program sets stopOnEntry=true (portable across adapters).
    let stopped = tokio::time::timeout(
        Duration::from_secs(60),
        agent.launch(&bin_str, Some(cwd), None, None),
    )
    .await
    .expect("launch timeout")
    .expect("launch");

    assert!(
        stopped.contains("stopped")
            || stopped.contains("breakpoint")
            || stopped.contains("threadId")
            || stopped.contains("reason")
            || stopped.contains("entry"),
        "unexpected stopped payload: {stopped}"
    );

    let threads = agent.get_threads().await.expect("threads");
    let tid = extract_thread_id(&stopped)
        .or_else(|| extract_thread_id(&threads))
        .unwrap_or(1);

    let stack = agent.get_stack(Some(tid), Some(20)).await.expect("stack");
    assert!(
        stack.contains("bug")
            || stack.contains("main")
            || stack.contains("items")
            || stack.contains("stackFrames"),
        "stack missing frames (tid={tid}): {stack}"
    );

    let _ = agent.disconnect(Some(true)).await;
}
