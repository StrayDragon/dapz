---
depends_on: []
---

## Why

Session 层硬编码 `adapterID=python`、`console=internalConsole`、盲默认 `threadId=1`，把 debugpy 习惯当成通用 DAP。适配器升级或换 backend 时易静默失败。需要：**规范字段优先**；特例显式标注兼容面。

## What Changes

- `AdapterKind`：从 backend 推断（Debugpy / Lldb / Generic）；`initialize.adapterID` 随之设置
- `launch_program`：仅规范字段；`console=internalConsole` **仅** Debugpy profile
- `initialized`：缓冲已有则用，否则等待（early/late 兼容）
- `threadId`：缺省时 `threads` 取首个，禁止盲 `1`
- 文档：`docs/specs/002-dap-compatibility.md`（矩阵 + `dapz-compress/1`）
- Delta：mcp + product-requirements（+ interceptors 契约一句）

## Out of Scope

- 每 adapter 一套压缩器；config_watcher / roots；改 MCP 工具名

## Impact

- Code: `src/mcp/session.rs`, agent_sdk start path, docs
- Spec: mcp, product-requirements, interceptors
- Tests: session unit + existing e2e still green
