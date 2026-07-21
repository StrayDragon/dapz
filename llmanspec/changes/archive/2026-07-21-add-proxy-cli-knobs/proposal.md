---
depends_on: []
---

## Why

`_PLAN.md` 成熟度债：Proxy 仅 stdio、metrics 仅 `DAPZ_METRICS` env，与 lspz CLI 旋钮不对齐。需要补 `--metrics` / `--transport`，并冻结 Proxy 默认 `json`（IDE）相对 MCP/SDK `toon` 的产品决策。

## What Changes

- `dapz proxy --metrics`（可与 `DAPZ_METRICS` 并用；任一为真即启用）
- `dapz proxy --transport stdio|tcp://host:port|ws://…`（默认 `stdio`）
- 文档：Proxy 默认输出保持 `json`；Agent/MCP 仍默认 TOON
- Delta：proxy / metrics / transport；更新 `_PLAN` 债表

## Out of Scope

- 改 Proxy 默认输出为 toon；config_watcher；lldb e2e；push/PR

## Impact

- Code: `src/main.rs`, README, `_PLAN.md`
- Spec: proxy, metrics, transport
