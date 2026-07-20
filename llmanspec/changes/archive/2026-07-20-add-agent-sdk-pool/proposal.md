---
depends_on: []
---

## Why

lspz 提供独立 `AgentPool`（按 language 懒加载 `AgentHandle`），与 MCP pool 对称。dapz 仅有 `mcp::DapPool` + 单会话 `AgentHandle`，多后端/多会话嵌入场景缺少一等 SDK API。

## What Changes

- 新增 `src/agent_sdk/pool.rs`（文件级 copy-port lspz 结构，**DAP 适配**：按 **session key** 注册 backend，不做 LSP language/workspace/document sync）
- `AgentPool::builder().register(key, backend).start_all()`；懒 `handle_for`；Tier-0 方法委托 `AgentHandle`
- 单测：mock 复用同一 key、未知 key 报错
- Delta：`agent-sdk` 增加 AgentPool MUST（scenarios `feature: false`，BDD-off）

## Capabilities

| Capability | Impact |
|------------|--------|
| agent-sdk | ADD AgentPool requirements |

## Out of Scope

- daemon / workspace roots / uri / didOpen 文档同步（LSP 概念，DAP 不适用）
- 改变现有 `AgentHandle` Tier-0 方法集
- 启用 BDD

## Impact

- Code: `src/agent_sdk/*`, tests
- Spec: `llmanspec/specs/agent-sdk`
- Risk: 与 `DapPool` key 语义 — SDK 用显式 session key；MCP 仍用 backend[+cwd]
