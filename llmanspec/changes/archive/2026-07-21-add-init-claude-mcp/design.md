## Design

Copy-port from lspz `src/init/` with DAP naming:

| lspz | dapz |
|------|------|
| `LSPZ.md` / `@LSPZ.md` | `DAPZ.md` / `@DAPZ.md` |
| `LSPZ_CLAUDE_DIR` | `DAPZ_CLAUDE_DIR` |
| `mcpServers.lspz` | `mcpServers.dapz` |
| language table | `generate_adapter_table` (debugpy / lldb) |

Global MCP file: `~/.claude.json`. Project: `.claude/settings.json`. Atomic write + `.bak` backup. Fail if binary lacks `mcp` feature.
