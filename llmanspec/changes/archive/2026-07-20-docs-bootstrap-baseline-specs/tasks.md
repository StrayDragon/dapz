# Tasks: docs-bootstrap-baseline-specs

## 1. Author deltas

- [x] 1.1 product-requirements delta（三模态 + Tier-0 范围）
- [x] 1.2 proxy / transport / codec deltas
- [x] 1.3 adapters delta（`resolve_tool` 搜索顺序）
- [x] 1.4 interceptors delta（capping + 5 compressors）
- [x] 1.5 mcp + agent-sdk deltas（17 tools；无已砍 API）
- [x] 1.6 ssot-rules delta

## 2. Validate

- [x] 2.1 `llman sdd validate docs-bootstrap-baseline-specs --strict --no-interactive --stage spec`
- [x] 2.2 `just qa`（确认无意外代码回归）

## 3. Seal

- [x] 3.1 `llman sdd change archive docs-bootstrap-baseline-specs`（合并进 live specs）
- [x] 3.2 `llman sdd validate --all --strict --no-interactive`
- [x] 3.3 勾选 `_PLAN.md` §9 P0；准备下一 change（建议 `add-agent-sdk-pool`）
