---
depends_on: []
---

## Why

debugpy 为必测后端；C/C++/Rust 调试常用 lldb-dap。需发现层映射，但不强制 e2e。

## What Changes

- `lookup_by_extension` / `lookup_by_language`：c/cpp/rust → lldb
- `resolve_lldb_debug_adapter`：优先 `lldb-dap` 再 `lldb-vscode`
- `just qa` 末尾跑 prek（对齐 lspz）；CI 加深 doc/doctest
- README 产品化结构

## Out of Scope

- lldb harness e2e；js-debug 等更多后端

## Impact

- Code: `src/adapters.rs`, justfile, `.github/workflows/ci.yml`, README, `_PLAN.md`
- Spec: adapters, ssot-rules
