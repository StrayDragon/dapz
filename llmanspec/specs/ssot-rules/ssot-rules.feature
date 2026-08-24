# language: en
# capability: ssot-rules
# purpose: 文档 SSOT：AGENTS、计划优先级与路径发现 tip。
# scope: AGENTS.md, _PLAN.md, docs/, llmanspec/

Feature: ssot-rules

  @req:r13 @human
  Scenario: AGENTS SSOT
    - 项目 MUST 以根 AGENTS.md 为通用规范 SSOT，并保留 LLMANSPEC managed block。

  @req:r21 @human
  Scenario: 计划优先级
    - 文档优先级 MUST 为 AGENTS.md → docs/specs → docs/plan；准发布临时计划为 _PLAN.md。

  @req:r29 @human
  Scenario: 路径发现 tip
    - 工具路径发现约定 MUST 记录于 docs/tips/00-tool-path-discovery.md 并与 adapters 实现对齐。

  @req:r42 @human
  Scenario: qa 含 prek
    - just qa MUST 包含 cargo doc --no-deps --all-features（doc-check），并在 prek 可用时运行 prek run --all-files。

  @req:r43 @human
  Scenario: verify 全量门禁
    - 项目 MUST 提供 just verify（或等价脚本）运行 qa、llman sdd validate --all --strict，并在本地准发布路径可用。

  @req:r13 @human
  Scenario: managed-block
    - MUST hold: Given 打开 AGENTS.md; When 查找 LLMANSPEC 标记; Then 存在 START/END managed block.
    Given 打开 AGENTS.md
    When 查找 LLMANSPEC 标记
    Then 存在 START/END managed block

  @req:r21 @human
  Scenario: plan-refs
    - MUST hold: Given 阅读 _PLAN.md 与 ROADMAP; When 交叉引用; Then 版本目标与 CLI 形态一致.
    Given 阅读 _PLAN.md 与 ROADMAP
    When 交叉引用
    Then 版本目标与 CLI 形态一致

  @req:r29 @human
  Scenario: tip-aligned
    - MUST hold: Given 对比 tip 搜索顺序与 resolve_tool; When 检查文档; Then 顺序表一致.
    Given 对比 tip 搜索顺序与 resolve_tool
    When 检查文档
    Then 顺序表一致

  @req:r42 @human
  Scenario: qa-prek
    - MUST hold: Given 打开 justfile; When 查看 qa 配方; Then 含 prek 或明确 WARN skip.
    Given 打开 justfile
    When 查看 qa 配方
    Then 含 prek 或明确 WARN skip

  @req:r43 @human
  Scenario: verify-script
    - MUST hold: Given 运行 just verify（或 scripts/verify-all.sh）; When 完成校验; Then 退出码 0 或明确失败于子步骤.
    Given 运行 just verify（或 scripts/verify-all.sh）
    When 完成校验
    Then 退出码 0 或明确失败于子步骤
