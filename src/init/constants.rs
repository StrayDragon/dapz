//! Path constants and embedded content for Claude Code integration.

/// Claude Code configuration directory name.
pub const CLAUDE_DIR: &str = ".claude";

/// Claude Code settings file name (project-level: hooks, permissions, env).
pub const SETTINGS_JSON: &str = "settings.json";

/// Claude Code user config file name (user-level: MCP servers, preferences).
/// MCP servers are registered here, not in settings.json.
pub const CLAUDE_JSON: &str = ".claude.json";

/// DAPZ awareness file name (written to `~/.claude/DAPZ.md`).
pub const DAPZ_MD: &str = "DAPZ.md";

/// Claude Code main instruction file name.
pub const CLAUDE_MD: &str = "CLAUDE.md";

/// Reference directive to add to CLAUDE.md.
pub const DAPZ_MD_REF: &str = "@DAPZ.md";

/// MCP server key in settings / `.claude.json`.
pub const MCP_SERVER_KEY: &str = "dapz";

/// Environment variable to override the Claude config directory.
pub const CLAUDE_DIR_ENV: &str = "DAPZ_CLAUDE_DIR";

/// Slim awareness content for Claude Code integration.
pub const DAPZ_SLIM: &str = r#"# dapz — DAP via MCP

Prefer dapz MCP tools for debug sessions (launch, stack, scopes, variables, evaluate, step) instead of ad-hoc debugger CLIs. Compressed TOON responses use fewer tokens.

## Tools (Tier-0 / Tier-1)

| Tool | Purpose |
|------|---------|
| `debug_launch` / `debug_attach` | Start session |
| `set_breakpoints` | Source breakpoints |
| `get_threads` / `get_stack` / `get_scopes` / `get_variables` | Observe state |
| `evaluate` / `get_exception` | Inspect values / exceptions |
| `get_output` | Drain buffered `output` events |
| `step_*` / `continue` / `pause` | Control execution |
| `disconnect` / `terminate` | End session |
| `send_raw` | Escape hatch (any DAP command) |

- Prefer **debugpy** for Python (`uv tool install debugpy`); optional **lldb-dap** for C/C++/Rust.
- Sessions are keyed by `backend` + project `cwd` (not LSP workspace roots).
- Default MCP path uses `dapz daemon` for session reuse; `--no-daemon` is in-process.
"#;
