---
depends_on: []
---

## Why

产品决策翻转：Proxy 默认输出改为 TOON，与 MCP/SDK 一致，便于 Agent 直连 proxy 省 token。IDE 需要 JSON 时显式 `--output json`。

## What Changes

- CLI / Config / env 缺省：`toon`
- README、`_PLAN` 债表
- Delta：proxy（及必要时 codec/product-requirements）

## Out of Scope

- 移除 websocket；lldb e2e；config_watcher / roots

## Impact

- Code: `src/main.rs`, `src/config.rs`, README, `_PLAN.md`
- Spec: proxy
