---
depends_on: []
---

## Why

Config/CLI 默认已是 `toon`，但 `Proxy::process_server_message` 只做拦截链压缩后仍输出完整 DAP JSON，与 lspz（passthrough 短路 / json 压缩帧 / toon `{format,text}` 包装）不对齐，也未真正兑现 r35。

## What Changes

- Proxy 按 `OutputFormat` 分流：`passthrough` 原字节；`json` 压缩后 DAP JSON 帧；`toon` 压缩后把 `body` 换成 `{format:"toon",text}`（保留 seq/type/command/event/id 字段）
- `value_to_toon` 改用官方 `toon-format` crate（`encode_default`），MCP/SDK/bench 共用
- 单测覆盖三种格式；`just gen-bench` 刷新数字

## Out of Scope

- 改 MCP 工具名；adapter 发现；压缩字段策略（仍 `dapz-compress/1`）

## Impact

- Code: `src/proxy.rs`, `src/codec/toon.rs`, `Cargo.toml`, tests, README/bench 若数字变
- Spec: proxy, codec
