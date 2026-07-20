---
depends_on: []
---

## Why

v0.1.0 已发布，但 `llmanspec/specs/` 为空。后续跟进 lspz 成熟度（AgentPool / verify / metrics / Tier-1）必须先有**基线契约**，否则 delta 无法锚定、verify 无法对照。

本 change **只建立文档契约**（反映当前已实现行为），不改应用代码。

## What Changes

- 新增基线 capability specs（经 archive 合并进 `llmanspec/specs/`）：
  - `product-requirements` — 三模态 + Tier-0 收紧范围
  - `proxy` — 透明代理、拦截链、失败透传
  - `transport` — Content-Length 分帧、stdio/tcp/ws
  - `codec` — JSON / TOON / passthrough
  - `adapters` — `resolve_tool` + debugpy 发现顺序
  - `interceptors` — capping + 5 个 DAP compressor
  - `mcp` — 17 Tier-0 tools + pool（无 attach/exception 专用 API）
  - `agent-sdk` — AgentHandle Tier-0 API（pool 列为后续 gap）
  - `ssot-rules` — AGENTS / `_PLAN` / tip 文档优先级
- 写入 `tasks.md` 验收：`llman sdd validate` + `just qa`（无代码 diff 预期）

## Capabilities

| Capability | Impact |
|------------|--------|
| product-requirements | NEW baseline |
| proxy | NEW baseline |
| transport | NEW baseline |
| codec | NEW baseline |
| adapters | NEW baseline（含 path discovery） |
| interceptors | NEW baseline |
| mcp | NEW baseline |
| agent-sdk | NEW baseline |
| ssot-rules | NEW baseline |

## Out of Scope

- daemon / init / uri / metrics / config_watcher
- Tier-1 专用 MCP API（attach/source/exception）
- 抽共享 crate；改应用代码

## Impact

- Affected code: none（契约-only）
- Affected docs: `llmanspec/specs/*` after archive
- Risks: 基线写错会导致后续 change 误改行为 — tasks 要求对照 `_PLAN.md` §1–§3 与源码再 validate
