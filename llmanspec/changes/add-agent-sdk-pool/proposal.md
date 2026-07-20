---
depends_on: []
---

## Why

lspz 提供独立 `AgentPool`（按 language 懒加载 `AgentHandle`），与 MCP pool 对称。dapz 仅有 `mcp::DapPool` + 单会话 `AgentHandle`，多后端/多会话嵌入场景缺少一等 SDK API。

## What Changes（draft / BDD-off）

- 新增 `src/agent_sdk/pool.rs`（文件级 copy-port lspz，适配 DAP：按 backend/key 而非 LSP language+workspace）
- `AgentPool::builder()` 注册 backend；懒 `start`；查询方法委托 `AgentHandle` Tier-0 API
- 单测：复用同一 key、未知 backend 报错、drop/shutdown
- Delta：`agent-sdk` 增加 AgentPool MUST 需求（`feature: false` 场景）

## Out of Scope

- daemon / workspace roots / uri
- 改变现有 `AgentHandle` Tier-0 方法集
- 启用 BDD

## Impact

- Code: `src/agent_sdk/*`, tests
- Spec: `llmanspec/specs/agent-sdk`
- Risk: 与 `DapPool` key 语义不一致 — design 时对齐（建议 pool key = backend[+cwd]）

## Status

Draft only. Formalize with `/llman-sdd-propose` when ready to apply.
