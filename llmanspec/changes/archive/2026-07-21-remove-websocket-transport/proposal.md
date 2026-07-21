---
depends_on: []
---

## Why

`WsTransport` 仅为 stub，DAP 适配器几乎不用 WebSocket。保留 feature 会误导用户。直接移除，CLI 仅保留 `stdio` 与 `tcp://`。

## What Changes

- 删除 `src/transport/websocket.rs`、`transport-websocket` feature、CLI `ws://` 分支
- 更新 README / Cargo.toml / lib docs / transport spec
- `_PLAN` 债表

## Out of Scope

- 实现真 WS；lldb e2e

## Impact

- Code: transport, main, Cargo.toml, README
- Spec: transport, proxy (transport help text)
