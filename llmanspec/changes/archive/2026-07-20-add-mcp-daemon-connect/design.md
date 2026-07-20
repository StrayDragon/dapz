## Design: MCP → daemon (DAP)

| | in-process (`--no-daemon`) | default (daemon) |
|--|---------------------------|------------------|
| Session | local `DapPool` | daemon `DapPool` via socket |
| Launch | `DapSession::launch_program` | `dap/spawn(replace)` + `dap/invoke(launch_program)` |
| Fail | N/A | error tool call（不静默回退） |

Socket identity = `resolve_project_cwd`（进程 cwd 或 `--cwd`），与 `dapz daemon` 一致。
