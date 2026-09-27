---
depends_on: []
branch: sdd/refactor-specs-compact
base_branch: main
base_sha: ac0a39529cad7f3c2ceeb2f6aa1c95c7c9b005d3
---

## Why

llman-sdd 0.5.0 review 基线：11 个 capability 共 142 条规则全部为裸规则（pending 142、enforced 0）。
现状每个 req id 写成两条平级裸规则（规范描述 + 同 id 的 `MUST hold: GWT` 单行），而 0.5.0 native
布局期望 GWT 以嵌套 `场景:`（假如/当/那么）挂在规则之下。specs-compact 目标：压降裸规则至 0，
规范行为不变。

## What Changes

- 逐 capability 将同 req id 的 `MUST hold:` 单行规则合并为其规范规则的嵌套 `场景:`（66 条规范规则
  全部转场景，75 条 GWT 单行合并嵌入，文本逐字保留）
- 移除 mcp `@req:r37`（Tier-1 异常工具）：规范文本为 r18（工具集合）真子集，场景
  `has-exception-tools` 保留为 r18 第二个场景；映射 r37 → r18
- 跨能力语义重叠 5 组经评审均不等价（层级不同），保留不改、不换号
- 仅编辑 `llmanspec/specs/**/*.feature`；不改 `src/`、`tests/`、justfile，规范行为不变

## Out of Scope

- 归档冻结（18 个 change 共 384K，未达冻结阈值）
- 跨能力 req id 换号（`project dedupe-req-ids` 无冲突）
- 任何 Rust 代码 / 测试行为变更

## Impact

- Code: 无
- Spec: 全部 11 个 capability（adapters, agent-sdk, codec, daemon, interceptors, mcp, metrics,
  product-requirements, proxy, ssot-rules, transport）
- 预期门禁：`validate --specs --strict` 全绿；`review` pending 142 → 0；规则 142 → 66（全 enforced）
