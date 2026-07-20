---
depends_on: []
---

## Why

v0.1 将 attach/source/exception 降为 `send_raw`。真实 debugpy bug 闭环常需 exceptionInfo；升一等公民减少 Agent 拼 raw DAP。

## What Changes

- Session/MCP/SDK：`debug_attach`、`get_source`、`get_exception`、`set_exception_breakpoints`
- `ExceptionInfoCompressor` + 链接入
- 修改基线：取消「禁止专用 Tier-1 API」；工具数 17→21
- 单测：mock exceptionInfo 压缩；tool list 含新工具
- **验证后端仍为 debugpy**（非 LSP diagnostics）

## Capabilities

| Capability | Impact |
|------------|--------|
| product-requirements | MODIFY Tier-0/1 边界 |
| mcp | ADD tools；MODIFY 禁令 |
| agent-sdk | ADD methods；MODIFY 禁令 |
| interceptors | ADD ExceptionInfoCompressor |

## Out of Scope

- daemon；全 DAP 表面；BDD-on
