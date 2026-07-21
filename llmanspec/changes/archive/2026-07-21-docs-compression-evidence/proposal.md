---
depends_on: []
---

## Why

首批压缩白名单已落地，但证据形态落后于 lspz：`bench-report` 缺 `ScopesCompressor` / `ExceptionInfoCompressor` fixtures，且只有「压缩 JSON vs 原始」单列，没有 **TOON vs 原始** 双列。README 区间估计无法用同一套可复现命令对齐。

## What Changes

- `fixtures/bench/`：新增 scopes / exceptionInfo 场景
- `examples/bench-report.rs`（及必要时 `compress-demo`）：纳入上述拦截器；对压缩后 body 再 `value_to_toon`，输出 **紧凑 vs 原始 / TOON vs 原始** 双列
- README「压缩效果」表改为与报告一致的实测数字（对齐 lspz README 形态）
- 不改拦截器语义（纯证据 / 文档 / 示例）

## Out of Scope

- 新压缩策略；调 capping 默认；Agent SDK daemon；BDD-on

## Impact

- Code: `fixtures/bench/*`, `examples/bench-report.rs`, `examples/compress-demo.rs`（可选）, `README.md`
- Spec: codec / interceptors（证据可复现的 MUST）或仅文档型 delta
