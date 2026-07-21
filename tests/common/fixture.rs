//! Temporary fixture helpers for integration tests.

/// Create a temporary Python script for debugging.
pub fn create_test_script(contents: &str) -> (std::path::PathBuf, tempfile::TempDir) {
    let dir = tempfile::tempdir().expect("create temp dir");
    let script_path = dir.path().join("test_debug.py");
    std::fs::write(&script_path, contents).expect("write test script");
    (script_path, dir)
}
