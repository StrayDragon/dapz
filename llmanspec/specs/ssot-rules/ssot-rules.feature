# language: zh-CN
# capability: ssot-rules
# purpose: 文档 SSOT：AGENTS、计划优先级与路径发现 tip。
# scope: AGENTS.md, _PLAN.md, docs/, llmanspec/

功能: ssot-rules

  @req:r13
  规则: AGENTS SSOT
    项目 MUST 以根 AGENTS.md 为通用规范 SSOT，并保留 LLMANSPEC managed block。

    场景: managed-block
      假如 打开 AGENTS.md
      当 查找 LLMANSPEC 标记
      那么 存在 START/END managed block

  @req:r21
  规则: 计划优先级
    文档优先级 MUST 为 AGENTS.md → docs/specs → docs/plan；准发布临时计划为 _PLAN.md。

    场景: plan-refs
      假如 阅读 _PLAN.md 与 ROADMAP
      当 交叉引用
      那么 版本目标与 CLI 形态一致

  @req:r29
  规则: 路径发现 tip
    工具路径发现约定 MUST 记录于 docs/tips/00-tool-path-discovery.md 并与 adapters 实现对齐。

    场景: tip-aligned
      假如 对比 tip 搜索顺序与 resolve_tool
      当 检查文档
      那么 顺序表一致

  @req:r42
  规则: qa 含 prek
    just qa MUST 包含 cargo doc --no-deps --all-features（doc-check），并在 prek 可用时运行 prek run --all-files。

    场景: qa-prek
      假如 打开 justfile
      当 查看 qa 配方
      那么 含 prek 或明确 WARN skip

  @req:r43
  规则: verify 全量门禁
    项目 MUST 提供 just verify（或等价脚本）运行 qa、llman sdd validate --all --strict，并在本地准发布路径可用。

    场景: verify-script
      假如 运行 just verify（或 scripts/verify-all.sh）
      当 完成校验
      那么 退出码 0 或明确失败于子步骤
