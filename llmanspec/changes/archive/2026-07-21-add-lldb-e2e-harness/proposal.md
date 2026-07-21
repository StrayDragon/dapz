---
depends_on: []
---

## Why

`_PLAN` P7 仅有 lldb 发现；缺 e2e 闭环。本地已有 `/usr/bin/lldb-dap`，应对齐 debugpy：ignored 测试 + harness 可选跑。

## What Changes

- `fixtures/e2e/lldb_bug.c` + `tests/lldb_integration.rs`（compile `-g` → launch → stack → disconnect）
- `scripts/check-env.sh` 导出 `DAPZ_LLDB_BACKEND`；`run-harness.sh` 可选跑 lldb e2e（缺则 SKIP，不拖垮 debugpy 门禁）
- Delta：adapters（或 product-requirements）

## Out of Scope

- 改 launch_program 通用化 adapterID；Windows

## Impact

- tests/, scripts/, fixtures/, Cargo.toml, README/`_PLAN`
