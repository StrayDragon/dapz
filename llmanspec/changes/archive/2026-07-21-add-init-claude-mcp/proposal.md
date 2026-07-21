---
depends_on: []
---

## Why

准发布已齐，但缺少 lspz 同款 `init --global`：无法一键注册 Claude Code MCP、写入 `DAPZ.md`、在 `CLAUDE.md` 注入 `@DAPZ.md`。用户体验与 lspz 不对齐。

## What Changes

- 移植 `src/init/`（awareness / settings / constants / run）
- CLI：`dapz init`（`--global` / `--no-patch` / `--show` / `--uninstall` / `--dry-run` / `--force` / `--auto-patch`）
- 需 `--features mcp` 构建；注册 `mcpServers.dapz` → `["mcp"]`
- 更新 README / ROADMAP；放宽 `llmanspec/AGENTS.md` 对 init 的禁止

## Out of Scope

- uri / roots / config_watcher；Cursor 以外 IDE 的专用注入

## Impact

- Code: `src/init/*`, `src/main.rs`, `src/adapters.rs`（adapter table）, `Cargo.toml`（tempfile）
- Spec: product-requirements 或新建 init 能力；proxy/mcp CLI
- Docs: README, ROADMAP
