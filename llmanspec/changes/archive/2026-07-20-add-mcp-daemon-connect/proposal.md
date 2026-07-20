---
depends_on: []
---

## Why

`add-daemon` 已提供长驻 socket，但 `dapz mcp` 仍进程内 spawn adapter。对齐 lspz：默认 MCP 经 daemon 复用会话。

## What Changes

- `DaemonMcpServer`：默认 `dapz mcp` 经 `DaemonClient` + `dap/spawn` / `dap/invoke`
- CLI：`--no-daemon` 回退进程内 `McpServer`；可选 `--cwd` / `DAPZ_DAEMON_CWD`
- Daemon RPC：`dap/invoke`、`dap/remove`；`dap/spawn.replace`
- Delta：mcp + daemon + product-requirements

## Out of Scope

- Agent SDK 走 daemon；Windows named pipes；BDD-on

## Impact

- Code: `src/mcp/daemon_server.rs`, `src/main.rs`, `src/daemon/*`
- Spec: mcp / daemon / product-requirements
