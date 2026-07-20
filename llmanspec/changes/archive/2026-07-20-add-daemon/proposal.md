---
depends_on: []
---

## Why

lspz 用长驻 daemon 复用 language server；dapz MCP/SDK 每次进程内 spawn adapter 成本高。需要对齐长驻 session，但按 **DAP/cwd** 建模（非 LSP workspace + document sync）。

## What Changes

- `src/daemon/`：Unix socket NDJSON JSON-RPC；`dap/spawn`、`dap/request`、`dap/wait_event`、`daemon/status`、`daemon/shutdown`
- Socket：`~/.cache/dapz/<slug>-<hash>.sock`（按 **cwd/project root** 规范化，非 LSP roots API）
- CLI：`dapz daemon` / `dapz daemon list`（feature `mcp`）
- `DapPool::clear` / keys；单测 socket 路径稳定性
- Delta：`daemon` capability + product-requirements 更新

## Out of Scope

- MCP auto-connect daemon（可后续）；document sync；BDD-on；Windows named pipes（首版 Unix）

## Impact

- Code: `src/daemon/*`, `src/main.rs`, `src/mcp/pool.rs`, Cargo.toml `dirs`
- Spec: new `daemon`
