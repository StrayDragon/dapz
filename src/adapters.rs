//! Debug adapter lookup and path discovery.
//!
//! Minimal mapping for Tier-0 (debugpy). Discovery checks common package-manager
//! install locations so agents / harnesses work without hand-tuned `PATH`.

use std::path::{Path, PathBuf};

/// Result of resolving a backend adapter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdapterInfo {
    /// Language id (e.g. `"python"`).
    pub language: &'static str,
    /// Backend command to spawn (may include args, shell-words split later).
    pub backend: String,
}

/// Look up adapter by file extension (without dot), e.g. `"py"`.
///
/// Resolves an executable path when possible (uv tool / `~/.local/bin` / PATH).
pub fn lookup_by_extension(ext: &str) -> Option<AdapterInfo> {
    match ext.to_ascii_lowercase().as_str() {
        "py" | "pyw" => Some(AdapterInfo {
            language: "python",
            backend: resolve_python_debug_adapter()
                .unwrap_or_else(|| "python3 -m debugpy.adapter".into()),
        }),
        _ => None,
    }
}

/// Look up adapter by language id, e.g. `"python"`.
pub fn lookup_by_language(language: &str) -> Option<AdapterInfo> {
    match language.to_ascii_lowercase().as_str() {
        "python" | "py" => Some(AdapterInfo {
            language: "python",
            backend: resolve_python_debug_adapter()
                .unwrap_or_else(|| "python3 -m debugpy.adapter".into()),
        }),
        _ => None,
    }
}

/// Discover a usable debugpy DAP adapter command.
///
/// Search order (first hit wins):
/// 1. `debugpy-adapter` on `PATH`
/// 2. `$HOME/.local/bin/debugpy-adapter` (uv tool / pipx user install)
/// 3. `$XDG_DATA_HOME/uv/tools/debugpy/bin/debugpy-adapter` (default `~/.local/share/...`)
/// 4. Same uv tools dir: `python3 -m debugpy.adapter`
/// 5. `python3 -m debugpy.adapter` if system/`PATH` python can `import debugpy`
pub fn resolve_python_debug_adapter() -> Option<String> {
    if let Some(p) = find_on_path("debugpy-adapter") {
        return Some(p);
    }

    if let Some(home) = std::env::var_os("HOME") {
        let user_bin = PathBuf::from(&home).join(".local/bin/debugpy-adapter");
        if is_executable(&user_bin) {
            return Some(user_bin.to_string_lossy().into_owned());
        }
    }

    if let Some(adapter) = uv_tools_debugpy_adapter() {
        return Some(adapter);
    }

    if python_can_import_debugpy("python3") {
        return Some("python3 -m debugpy.adapter".into());
    }

    None
}

fn uv_tools_debugpy_adapter() -> Option<String> {
    let data = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/share")))?;

    let tool_bin = data.join("uv/tools/debugpy/bin");
    let adapter = tool_bin.join("debugpy-adapter");
    if is_executable(&adapter) {
        return Some(adapter.to_string_lossy().into_owned());
    }

    let python = tool_bin.join("python3");
    if is_executable(&python) && python_can_import_debugpy(python.to_str()?) {
        return Some(format!("{} -m debugpy.adapter", python.display()));
    }

    None
}

fn find_on_path(cmd: &str) -> Option<String> {
    let path = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path) {
        let candidate = dir.join(cmd);
        if is_executable(&candidate) {
            return Some(candidate.to_string_lossy().into_owned());
        }
    }
    // Also probe ~/.local/bin even if missing from PATH (common agent/CI gap).
    if let Some(home) = std::env::var_os("HOME") {
        let candidate = PathBuf::from(home).join(".local/bin").join(cmd);
        if is_executable(&candidate) {
            return Some(candidate.to_string_lossy().into_owned());
        }
    }
    None
}

fn is_executable(path: &Path) -> bool {
    path.is_file()
        && std::fs::metadata(path)
            .map(|m| {
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    m.permissions().mode() & 0o111 != 0
                }
                #[cfg(not(unix))]
                {
                    true
                }
            })
            .unwrap_or(false)
}

fn python_can_import_debugpy(python: &str) -> bool {
    std::process::Command::new(python)
        .args(["-c", "import debugpy"])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lookup_py_extension() {
        let info = lookup_by_extension("py").unwrap();
        assert_eq!(info.language, "python");
        assert!(info.backend.contains("debugpy"));
    }

    #[test]
    fn test_lookup_python_language() {
        let info = lookup_by_language("python").unwrap();
        assert!(info.backend.contains("debugpy"));
    }

    #[test]
    fn test_lookup_unknown() {
        assert!(lookup_by_extension("rs").is_none());
        assert!(lookup_by_language("rust").is_none());
    }

    #[test]
    fn test_resolve_finds_local_or_fallback() {
        // Should not panic; may or may not find depending on machine.
        let _ = resolve_python_debug_adapter();
    }
}
