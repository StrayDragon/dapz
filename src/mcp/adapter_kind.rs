//! Adapter kind — DAP-spec defaults vs documented extras.
//!
//! Discovery may be language-specific; **protocol fields** stay generic unless
//! [`AdapterKind`] explicitly enables a documented extension.

use std::fmt;

/// How dapz shapes `initialize` / `launch` for a backend.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AdapterKind {
    /// No adapter-specific launch keys. `adapterID` = `"dapz"`.
    #[default]
    Generic,
    /// debugpy / python adapters.
    ///
    /// Extras: `console: "internalConsole"` on launch (debugpy extension, not DAP core).
    /// Verified with uv-tool / `python3 -m debugpy.adapter` as of 2026-07.
    Debugpy,
    /// lldb-dap / lldb-vscode.
    ///
    /// Verified with system `lldb-dap` as of 2026-07; no launch extras.
    Lldb,
}

impl AdapterKind {
    /// Infer kind from a backend command string (path or shell form).
    pub fn from_backend(cmd: &str) -> Self {
        let c = cmd.to_ascii_lowercase();
        if c.contains("debugpy") {
            Self::Debugpy
        } else if c.contains("lldb") {
            Self::Lldb
        } else {
            Self::Generic
        }
    }

    /// Value for DAP `initialize.adapterID`.
    pub fn adapter_id(self) -> &'static str {
        match self {
            Self::Generic => "dapz",
            Self::Debugpy => "python",
            Self::Lldb => "lldb-dap",
        }
    }

    /// Whether launch should include debugpy `console` extension.
    pub fn wants_debugpy_console(self) -> bool {
        matches!(self, Self::Debugpy)
    }
}

impl fmt::Display for AdapterKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Generic => write!(f, "generic"),
            Self::Debugpy => write!(f, "debugpy"),
            Self::Lldb => write!(f, "lldb"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_backend_debugpy() {
        assert_eq!(
            AdapterKind::from_backend("python3 -m debugpy.adapter"),
            AdapterKind::Debugpy
        );
        assert_eq!(
            AdapterKind::from_backend("/home/u/.local/bin/debugpy-adapter"),
            AdapterKind::Debugpy
        );
    }

    #[test]
    fn test_from_backend_lldb() {
        assert_eq!(
            AdapterKind::from_backend("/usr/bin/lldb-dap"),
            AdapterKind::Lldb
        );
    }

    #[test]
    fn test_from_backend_generic() {
        assert_eq!(
            AdapterKind::from_backend("my-custom-dap"),
            AdapterKind::Generic
        );
    }
}
