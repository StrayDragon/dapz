//! Debug adapter lookup and path discovery.
//!
//! Tier-0 backends: **debugpy** (required for harness) and optional **lldb-dap** /
//! **lldb-vscode** for C/C++/Rust. Discovery checks common package-manager install
//! locations so agents / harnesses work without hand-tuned `PATH`.

use std::path::{Path, PathBuf};

/// Result of resolving a backend adapter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdapterInfo {
    /// Language id (e.g. `"python"`).
    pub language: &'static str,
    /// Backend command to spawn (may include args, shell-words split later).
    pub backend: String,
}

/// Search roots for package-manager default layouts (testable without mutating env).
#[derive(Debug, Clone, Default)]
pub struct DiscoveryContext {
    /// `PATH`-like search list (directories only).
    pub path_dirs: Vec<PathBuf>,
    /// `$HOME`.
    pub home: Option<PathBuf>,
    /// `$XDG_DATA_HOME` (else derived from `home` as `~/.local/share`).
    pub xdg_data_home: Option<PathBuf>,
    /// `$CARGO_HOME` (else `~/.cargo`).
    pub cargo_home: Option<PathBuf>,
    /// `$GOPATH` (else `~/go`).
    pub go_path: Option<PathBuf>,
}

impl DiscoveryContext {
    /// Build from the current process environment.
    pub fn from_env() -> Self {
        let home = std::env::var_os("HOME").map(PathBuf::from);
        let path_dirs = std::env::var_os("PATH")
            .map(|p| std::env::split_paths(&p).collect())
            .unwrap_or_default();
        let xdg_data_home = std::env::var_os("XDG_DATA_HOME")
            .map(PathBuf::from)
            .or_else(|| home.as_ref().map(|h| h.join(".local/share")));
        let cargo_home = std::env::var_os("CARGO_HOME")
            .map(PathBuf::from)
            .or_else(|| home.as_ref().map(|h| h.join(".cargo")));
        let go_path = std::env::var_os("GOPATH")
            .map(PathBuf::from)
            .or_else(|| home.as_ref().map(|h| h.join("go")));
        Self {
            path_dirs,
            home,
            xdg_data_home,
            cargo_home,
            go_path,
        }
    }

    fn data_home(&self) -> Option<&Path> {
        self.xdg_data_home.as_deref()
    }
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
        "c" | "h" => Some(AdapterInfo {
            language: "c",
            backend: resolve_lldb_debug_adapter().unwrap_or_else(|| "lldb-dap".into()),
        }),
        "cpp" | "cc" | "cxx" | "hpp" | "hxx" => Some(AdapterInfo {
            language: "cpp",
            backend: resolve_lldb_debug_adapter().unwrap_or_else(|| "lldb-dap".into()),
        }),
        "rs" => Some(AdapterInfo {
            language: "rust",
            backend: resolve_lldb_debug_adapter().unwrap_or_else(|| "lldb-dap".into()),
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
        "c" => Some(AdapterInfo {
            language: "c",
            backend: resolve_lldb_debug_adapter().unwrap_or_else(|| "lldb-dap".into()),
        }),
        "cpp" | "c++" => Some(AdapterInfo {
            language: "cpp",
            backend: resolve_lldb_debug_adapter().unwrap_or_else(|| "lldb-dap".into()),
        }),
        "rust" | "rs" => Some(AdapterInfo {
            language: "rust",
            backend: resolve_lldb_debug_adapter().unwrap_or_else(|| "lldb-dap".into()),
        }),
        _ => None,
    }
}

/// Resolve a tool binary via PATH + common package-manager default layouts.
///
/// Search order (first hit wins):
/// 1. Directories on `PATH`
/// 2. `$HOME/.local/bin/<name>` (uv tool / pipx user install)
/// 3. `$CARGO_HOME/bin/<name>` (default `~/.cargo/bin`)
/// 4. `$GOPATH/bin/<name>` (default `~/go/bin`)
/// 5. `$XDG_DATA_HOME/npm/bin/<name>` and `$HOME/.bun/bin/<name>` (optional JS globals)
pub fn resolve_tool(name: &str) -> Option<PathBuf> {
    resolve_tool_in(name, &DiscoveryContext::from_env())
}

/// Same as [`resolve_tool`], but with an explicit discovery context (for tests).
pub fn resolve_tool_in(name: &str, ctx: &DiscoveryContext) -> Option<PathBuf> {
    for dir in &ctx.path_dirs {
        let candidate = dir.join(name);
        if is_executable(&candidate) {
            return Some(candidate);
        }
    }

    if let Some(home) = &ctx.home {
        let user_bin = home.join(".local/bin").join(name);
        if is_executable(&user_bin) {
            return Some(user_bin);
        }
    }

    if let Some(cargo) = &ctx.cargo_home {
        let candidate = cargo.join("bin").join(name);
        if is_executable(&candidate) {
            return Some(candidate);
        }
    }

    if let Some(go) = &ctx.go_path {
        let candidate = go.join("bin").join(name);
        if is_executable(&candidate) {
            return Some(candidate);
        }
    }

    if let Some(data) = ctx.data_home() {
        let npm = data.join("npm/bin").join(name);
        if is_executable(&npm) {
            return Some(npm);
        }
    }

    if let Some(home) = &ctx.home {
        let bun = home.join(".bun/bin").join(name);
        if is_executable(&bun) {
            return Some(bun);
        }
    }

    None
}

/// Discover a usable debugpy DAP adapter command.
///
/// Search order (first hit wins):
/// 1. `debugpy-adapter` via [`resolve_tool`]
/// 2. `$XDG_DATA_HOME/uv/tools/debugpy/bin/debugpy-adapter`
/// 3. Same uv tools dir: `python3 -m debugpy.adapter`
/// 4. `python3 -m debugpy.adapter` if system/`PATH` python can `import debugpy`
pub fn resolve_python_debug_adapter() -> Option<String> {
    resolve_python_debug_adapter_in(&DiscoveryContext::from_env())
}

/// Same as [`resolve_python_debug_adapter`], with explicit context.
pub fn resolve_python_debug_adapter_in(ctx: &DiscoveryContext) -> Option<String> {
    if let Some(p) = resolve_tool_in("debugpy-adapter", ctx) {
        return Some(p.to_string_lossy().into_owned());
    }

    if let Some(adapter) = uv_tools_debugpy_adapter(ctx) {
        return Some(adapter);
    }

    if python_can_import_debugpy("python3") {
        return Some("python3 -m debugpy.adapter".into());
    }

    None
}

/// Discover an LLDB-based DAP adapter (`lldb-dap` or legacy `lldb-vscode`).
///
/// Search order (first hit wins):
/// 1. `lldb-dap` via [`resolve_tool`]
/// 2. `lldb-vscode` via [`resolve_tool`]
///
/// Optional second backend after debugpy — not required for harness.
pub fn resolve_lldb_debug_adapter() -> Option<String> {
    resolve_lldb_debug_adapter_in(&DiscoveryContext::from_env())
}

/// Same as [`resolve_lldb_debug_adapter`], with explicit context.
pub fn resolve_lldb_debug_adapter_in(ctx: &DiscoveryContext) -> Option<String> {
    for name in ["lldb-dap", "lldb-vscode"] {
        if let Some(p) = resolve_tool_in(name, ctx) {
            return Some(p.to_string_lossy().into_owned());
        }
    }
    None
}

fn uv_tools_debugpy_adapter(ctx: &DiscoveryContext) -> Option<String> {
    let data = ctx.data_home()?;
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
    use std::fs;
    use std::os::unix::fs::PermissionsExt;

    fn touch_exec(path: &Path) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, b"#!/bin/sh\nexit 0\n").unwrap();
        let mut perms = fs::metadata(path).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(path, perms).unwrap();
    }

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
        assert!(lookup_by_extension("java").is_none());
        assert!(lookup_by_language("go").is_none());
    }

    #[test]
    fn test_lookup_lldb_languages() {
        let rust = lookup_by_language("rust").unwrap();
        assert_eq!(rust.language, "rust");
        assert!(
            rust.backend.contains("lldb"),
            "expected lldb backend, got {}",
            rust.backend
        );
        let cpp = lookup_by_extension("cpp").unwrap();
        assert_eq!(cpp.language, "cpp");
    }

    #[test]
    fn test_resolve_lldb_prefers_lldb_dap() {
        let tmp = tempfile::tempdir().unwrap();
        let path_dir = tmp.path().join("path");
        touch_exec(&path_dir.join("lldb-dap"));
        touch_exec(&path_dir.join("lldb-vscode"));
        let ctx = DiscoveryContext {
            path_dirs: vec![path_dir.clone()],
            ..DiscoveryContext::default()
        };
        let found = resolve_lldb_debug_adapter_in(&ctx).unwrap();
        assert_eq!(found, path_dir.join("lldb-dap").to_string_lossy());
    }

    #[test]
    fn test_resolve_finds_local_or_fallback() {
        let _ = resolve_python_debug_adapter();
    }

    #[test]
    fn test_resolve_tool_prefers_path_then_user_bin() {
        let tmp = tempfile::tempdir().unwrap();
        let home = tmp.path().join("home");
        let path_dir = tmp.path().join("path");
        touch_exec(&path_dir.join("mytool"));
        touch_exec(&home.join(".local/bin/mytool"));

        let ctx = DiscoveryContext {
            path_dirs: vec![path_dir.clone()],
            home: Some(home),
            ..DiscoveryContext::default()
        };
        let found = resolve_tool_in("mytool", &ctx).unwrap();
        assert_eq!(found, path_dir.join("mytool"));
    }

    #[test]
    fn test_resolve_tool_user_bin_without_path() {
        let tmp = tempfile::tempdir().unwrap();
        let home = tmp.path().join("home");
        touch_exec(&home.join(".local/bin/debugpy-adapter"));

        let ctx = DiscoveryContext {
            path_dirs: vec![],
            home: Some(home.clone()),
            ..DiscoveryContext::default()
        };
        let found = resolve_tool_in("debugpy-adapter", &ctx).unwrap();
        assert_eq!(found, home.join(".local/bin/debugpy-adapter"));
    }

    #[test]
    fn test_resolve_tool_cargo_home() {
        let tmp = tempfile::tempdir().unwrap();
        let cargo = tmp.path().join("cargo-home");
        touch_exec(&cargo.join("bin/lldb-dap"));

        let ctx = DiscoveryContext {
            path_dirs: vec![],
            cargo_home: Some(cargo.clone()),
            ..DiscoveryContext::default()
        };
        let found = resolve_tool_in("lldb-dap", &ctx).unwrap();
        assert_eq!(found, cargo.join("bin/lldb-dap"));
    }

    #[test]
    fn test_resolve_tool_go_and_bun() {
        let tmp = tempfile::tempdir().unwrap();
        let home = tmp.path().join("home");
        let go = tmp.path().join("gopath");
        touch_exec(&go.join("bin/dlv"));
        touch_exec(&home.join(".bun/bin/some-js-dap"));

        let ctx_go = DiscoveryContext {
            path_dirs: vec![],
            go_path: Some(go.clone()),
            ..DiscoveryContext::default()
        };
        assert_eq!(resolve_tool_in("dlv", &ctx_go).unwrap(), go.join("bin/dlv"));

        let ctx_bun = DiscoveryContext {
            path_dirs: vec![],
            home: Some(home.clone()),
            ..DiscoveryContext::default()
        };
        assert_eq!(
            resolve_tool_in("some-js-dap", &ctx_bun).unwrap(),
            home.join(".bun/bin/some-js-dap")
        );
    }

    #[test]
    fn test_resolve_python_adapter_uv_tools_layout() {
        let tmp = tempfile::tempdir().unwrap();
        let data = tmp.path().join("xdg-data");
        touch_exec(&data.join("uv/tools/debugpy/bin/debugpy-adapter"));

        let ctx = DiscoveryContext {
            path_dirs: vec![],
            home: Some(tmp.path().join("home")),
            xdg_data_home: Some(data.clone()),
            ..DiscoveryContext::default()
        };
        let backend = resolve_python_debug_adapter_in(&ctx).unwrap();
        assert_eq!(
            backend,
            data.join("uv/tools/debugpy/bin/debugpy-adapter")
                .to_string_lossy()
        );
    }
}
