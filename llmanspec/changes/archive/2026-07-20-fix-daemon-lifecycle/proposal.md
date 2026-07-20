---
depends_on: []
---

## Why

对照 lspz：daemon 无 idle reaper、pool 无死进程回收、client 无 owns_daemon Drop，长跑会泄漏 adapter 子进程。

## What Changes

- `DapPool::reap_if_dead` / `reap_idle`；`DapSession::try_wait`
- `DaemonServer::spawn_idle_reaper`（SESSION/DAEMON idle TTL）
- `DaemonClient::owns_daemon` + Drop shutdown；`connect_explicit` / `try_connect`
- `tests/daemon_integration.rs`

## Out of Scope

- Agent SDK 经 daemon；缩短 TTL 配置化

## Impact

- Code: `src/mcp/pool.rs`, `src/daemon/{server,client}.rs`, `tests/daemon_integration.rs`
- Spec: daemon
