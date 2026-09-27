# Design: update-bdd-batch-runner

## 问题

`refactor-specs-compact` 落地 75 个嵌套场景后，llman-sdd 0.5.0 的收口门禁
（`decideCloseOutHarness`）要求 `bdd.run_command`：needsSpecsChange ∧ hasExecutable ∧
无 run_command → finalize 中止。当时项目规则是 BDD-off，遂以 75 个 `@skip` 标签
（runnable=false）绕过——这是按场景征收的永久税，且使收口门禁退化为结构性跳过。

## 方案（中心化根治）

1. `llmanspec/config.yaml` 启用 `bdd.run_command: cargo test --quiet`。无 `{feature_*}`
   占位符 → batch-once：`validate --specs` 与 change 收口各执行一次完整 Rust 测试套件。
   场景的"可执行"语义 = 其验收映射到既有测试（`tests/` 与单测），由套件统一兑现。
2. 移除全部 75 个 `@skip`，场景恢复 runnable（enforced/pending 统计不受标签影响，前后一致）。
3. `llmanspec/AGENTS.md` 的 BDD-off 规则改为"doc-GWT + batch acceptance runner"：
   无 pytest-bdd/cucumber、无 `bdd` cargo feature；新场景零特殊标记。

## 后果

- 后代作者：写普通 `场景:` 即可，finalize 门禁自动跑 `cargo test --quiet`，无需记得打标。
- 收口从"跳过"变为"真实执行测试套件"——门禁更强而非更弱。
- 场景文本、规则描述、req id 全部不变（纯元数据与基建变更）。
