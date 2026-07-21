# Tasks: agent-sdk-daemon-connect

- [x] 1. 定稿默认策略：daemon 默认 vs opt-in（写进 design / README）
- [x] 2. `AgentBuilder` / `AgentPoolBuilder`：cwd + in_process/via_daemon；经 `DaemonClient::connect_or_start`
- [x] 3. Daemon-backed handle：Tier-0/Tier-1 走 `dap/spawn` + `dap/invoke`；压缩+TOON 仍本地
- [x] 4. 单测：mock/daemon_integration 覆盖 connect 失败不静默回退、invoke launch→stack
- [x] 5. Delta specs；`llman sdd validate agent-sdk-daemon-connect --strict --no-interactive`
- [x] 6. `just qa`（触及 daemon 时加 `just harness` 可选）
- [x] 7. README Agent SDK 段：与 MCP daemon 复用说明
