---
depends_on: []
---

## Why

v0.1 Tier-0 砍掉 `attach` / `source` / `exceptionInfo` / `setExceptionBreakpoints`（仅 `send_raw`）。真实 bug 闭环里异常路径常见；升一等公民可减少 Agent 拼 raw DAP 的负担，并可选加 `ExceptionInfoCompressor`。

## What Changes（draft / BDD-off）

- MCP tools：`debug_attach`、`get_source`、`get_exception`、`set_exception_breakpoints`（名称可再定）
- Agent SDK 对称方法
- 可选：`ExceptionInfoCompressor` + 拦截白名单
- e2e：debugpy 抛异常 fixture（与现有 non-exception loop 并存）
- Delta：修改 `product-requirements` r34 / `mcp` r37 / `agent-sdk` r31（从 MUST NOT 专用 API → MUST 提供）；scenarios `feature: false`

## Out of Scope

- daemon
- 全 DAP 方法一等公民化
- 启用 BDD

## Impact

- Code: `src/mcp/*`, `src/agent_sdk/*`, interceptors, tests/harness
- Spec: product-requirements, mcp, agent-sdk, interceptors
- Risk: 扩大 Agent 表面 — 需更新 README/_PLAN §3 工具表

## Status

Draft only. **Optional** per `_PLAN.md` §9 P4；正式化前先确认产品要不要升 Tier-1。
