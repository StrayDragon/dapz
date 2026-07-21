## Design: Agent SDK → daemon（已定稿）

| | in-process（**默认**） | daemon（**opt-in** `via_daemon(true)`） |
|--|----------------------|----------------------------------------|
| Session | 本地 `DapSession` | daemon `DapPool` via Unix socket |
| Launch / Tier-0 | 直接 session 方法 | `dap/spawn` + `dap/invoke` |
| Pool key | 本地 map | `backend[+cwd]`（与 MCP/daemon 一致） |
| Fail connect | N/A | `Err`（**不**静默回退进程内） |
| Drop | 杀本地 adapter | `owns_daemon` 时 best-effort `daemon/shutdown` |

### 为何默认 in-process（相对 MCP）

DAP 调试是**按需短会话**；LSP 是工作区常驻。Agent SDK 嵌入式循环默认本地 spawn 更轻；需要与 `dapz mcp` / 已有 daemon **共享会话**时再 `via_daemon(true)`。MCP 仍保持默认 daemon（多工具复用）。

### 复用边界

- 复用 `DaemonClient` RPC；不复制 MCP 工具层
- 压缩 + TOON 仍在 SDK 侧；压缩失败 `warn!` + 回退原 body（对齐 Proxy fail-open）
- Socket identity = `resolve_project_cwd`（builder `cwd` 或进程 cwd）
- 高级/测试：`daemon_socket(path)` → `connect_explicit`（无 auto-start）
