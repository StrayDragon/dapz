---
depends_on: []
---

## Why

lspz `just qa` 含 `doc-check`，另有 `just verify`（scripts/verify-all.sh：fmt/clippy/test/docs/SDD/prek）。dapz 目前 `just qa` 仅 fmt+clippy+test，文档与 SDD 门禁未进入日常路径，跟进成熟度时容易漂移。

## What Changes（draft / BDD-off）

- `justfile`：`qa` 增加 `doc-check`；新增 `doc` / `doc-check` / `doc-test`
- 新增 `scripts/verify-all.sh` + `just verify`（env 可选 + qa + `llman sdd validate --all --strict --no-interactive` + prek）
- 文档：`AGENTS.md` / `ROADMAP` 写明 qa vs verify
- Delta：`ssot-rules` 增加门禁 MUST（doc-only scenarios, `feature: false`）

## Out of Scope

- 改应用运行时行为
- 启用 BDD / 强制 CI 跑 debugpy e2e（仍由 harness 负责）

## Impact

- Code: `justfile`, `scripts/verify-all.sh`
- Spec: `ssot-rules`
- Risk: `cargo doc` 若有警告变错误需先清 API docs

## Status

Draft only. Formalize when ready; good small first apply after pool or before it.
