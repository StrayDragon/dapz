---
depends_on: []
---

## Why

MCP 已默认经 daemon 复用 `backend+cwd` 会话（`add-mcp-daemon-connect`），但 Agent SDK 仍始终进程内 `DapSession::spawn`。嵌入式 Rust Agent 与 MCP 路径会话不共享，长驻复用 / idle reap / owns_daemon 生命周期也不一致。对齐 `_PLAN.md` §9 成熟度债与 lspz「长驻复用」体验。

## What Changes

- `AgentHandle` / `AgentPool` 可选（或默认）经 `DaemonClient` 连接/自动启动 dapz daemon
- Builder：`--cwd` / project cwd 派生 socket；`in_process` / `no_daemon` 显式回退进程内（对称 MCP `--no-daemon`）
- 观测/控制调用经 `dap/spawn` + `dap/invoke`（复用既有 daemon RPC，不新造协议）
- 失败语义：连不上 daemon 时返回错误（不静默回退进程内），与 `DaemonMcpServer` 一致
- Delta：`agent-sdk` + `daemon` + `product-requirements`

## Out of Scope

- Windows named pipes；BDD-on；改压缩白名单；Proxy CLI 默认 toon；lldb e2e

## Impact

- Code: `src/agent_sdk/*`, `src/daemon/client.rs`（复用）, tests
- Spec: agent-sdk / daemon / product-requirements
- Docs: README Agent SDK 段说明 daemon 复用
