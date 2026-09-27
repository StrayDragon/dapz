---
depends_on: []
---

## Why

refactor-specs-compact 落地了 75 个嵌套场景后，0.5.0 收口门禁要求 `bdd.run_command`，而项目规则
（llmanspec/AGENTS.md）是 BDD-off。当时的权宜方案是给全部 75 个场景打 `@skip`（runnable=false）——
这是按场景征收的永久税：每个新增场景都必须记得打标，否则 finalize 再次中止。需要中心化根治：
让可执行场景的验收指向项目真实的测试套件，后代写场景零特殊标记。

## What Changes

- `llmanspec/config.yaml`：启用 `bdd.run_command: cargo test --quiet`（无占位符 = batch-once，
  validate --specs 与 change 收口各执行一次完整 Rust 测试套件）；不加 framework 绑定
- 移除全部 75 个 `@skip` 标签，场景恢复 runnable
- `llmanspec/AGENTS.md`：更新过时的 BDD-off 规则——场景是 doc-GWT 映射到既有 Rust 测试，
  无 pytest-bdd/cucumber、无 `bdd` cargo feature；新场景无需任何特殊标签
- 收口语义变化：finalize 的 bdd 门禁从"结构性跳过"变为"真实执行 cargo test"

## Out of Scope

- 引入 pytest-bdd / cucumber / cargo bdd feature 等按场景步骤绑定
- 修改任何场景文本、规则描述、req id

## Impact

- Code: 无（`cargo test` 行为不变，仅被 llman 门禁调用）
- Spec: 无规范性变更（仅移除 `@skip` 标签元数据）
- Infra: `llmanspec/config.yaml`、`llmanspec/AGENTS.md`
