---
depends_on: []
---

## Why

lspz `just qa` 含 `doc-check`，另有 `just verify`（scripts/verify-all.sh）。dapz `just qa` 仅 fmt+clippy+test，文档与 SDD 门禁未进入日常路径。

## What Changes

- `justfile`：`qa` 增加 `doc-check`；新增 `doc` / `doc-check` / `doc-test` / `verify`
- 新增 `scripts/verify-all.sh`（qa + `llman sdd validate --all --strict --no-interactive` + prek）
- Delta：`ssot-rules` 门禁 MUST（`feature: false`）
- 更新 agent-sdk purpose（含 AgentPool）与 CHANGELOG

## Capabilities

| Capability | Impact |
|------------|--------|
| ssot-rules | ADD verify/doc-check requirements |

## Out of Scope

- 改应用运行时；启用 BDD；强制 CI debugpy e2e

## Impact

- `justfile`, `scripts/verify-all.sh`, docs
