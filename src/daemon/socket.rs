//! Daemon socket path helpers (cwd-keyed, not LSP workspace-roots).

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;

/// Resolve a project cwd into a canonicalized absolute path when possible.
pub fn resolve_project_cwd(cwd: &str) -> String {
    std::fs::canonicalize(cwd)
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_else(|_| cwd.to_string())
}

/// Unix socket path for a project cwd: `~/.cache/dapz/<slug>-<hash>.sock`.
pub fn socket_path_for_cwd(cwd: &str) -> PathBuf {
    let canonical = resolve_project_cwd(cwd);
    let hash = hash_string(&canonical);
    let slug = cwd_slug(&canonical);
    dapz_cache_dir().join(format!("{slug}-{hash:016x}.sock"))
}

fn hash_string(s: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    s.hash(&mut hasher);
    hasher.finish()
}

fn cwd_slug(path: &str) -> String {
    std::path::Path::new(path)
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "unknown".into())
}

/// Cache directory for dapz daemon sockets.
pub fn dapz_cache_dir() -> PathBuf {
    dirs::cache_dir()
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join("dapz")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_socket_path_stable() {
        let a = socket_path_for_cwd("/home/user/project");
        let b = socket_path_for_cwd("/home/user/project");
        assert_eq!(a, b);
    }

    #[test]
    fn test_socket_path_different_cwds() {
        let a = socket_path_for_cwd("/home/user/project-a");
        let b = socket_path_for_cwd("/home/user/project-b");
        assert_ne!(a, b);
    }

    #[test]
    fn test_slug() {
        assert_eq!(cwd_slug("/home/user/my-project"), "my-project");
    }
}
