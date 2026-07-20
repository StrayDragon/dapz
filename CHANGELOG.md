# Changelog

## v0.3.0 (2026-07-20)

Long-lived DAP daemon + MCP auto-connect (aligned with lspz; DAP cwd/session model).

- **Daemon** (`--features mcp`): `dapz daemon` / `dapz daemon list`；Unix socket NDJSON（`dap/spawn`、`dap/request`、`dap/wait_event`、`dap/invoke`、`dap/remove`、`daemon/status`、`daemon/shutdown`）；socket 按 project cwd（`~/.cache/dapz/`）
- **MCP**: 默认 `DaemonMcpServer`（自动连接/启动 daemon）；`--no-daemon` 回退进程内 `McpServer`；可选 `--cwd` / `DAPZ_DAEMON_CWD`
- **SDD**: `add-daemon` + `add-mcp-daemon-connect` archived；BDD 仍关闭
- **Docs**: README 三模态含 daemon；`_PLAN.md` P5 完成

## v0.2.0 (2026-07-20)

Align dapz closer to lspz maturity for DAP/debugpy (BDD-off SDD).

- **SDD**: `llmanspec/` + `.agents/skills`; 10 capability specs; BDD intentionally off
- **Discovery**: `resolve_tool` / `DiscoveryContext` (PATH → user/cargo/go/npm/bun bins); clean-PATH harness
- **Agent SDK**: `AgentPool` keyed by **session name** (not LSP language)
- **Tooling**: `just qa` includes `doc-check`; `just verify` / `scripts/verify-all.sh`
- **Metrics**: `MeteredInterceptor` + `DAPZ_METRICS` (DAP frame byte/latency snapshots)
- **Tier-1 DAP**: `debug_attach` / `get_source` / `get_exception` / `set_exception_breakpoints` + `ExceptionInfoCompressor` (**21** MCP tools)
- **Harness**: `just harness` green with debugpy e2e

## v0.1.0 (2026-07-20)

Agent-usable Tier-0 DAP surface (aligned with lspz patterns; no shared crate).

- **Proxy**: CLI subcommands `dapz proxy` / `dapz mcp`; cancel-safe Content-Length framing
- **TOON**: `OutputFormat::{Json, Toon, Passthrough}` + generic `value_to_toon`
- **Compressors**: existing output/variables/stackTrace/evaluate + new `ScopesCompressor`
- **MCP** (`--features mcp`): 17 Tier-0 tools (`debug_launch` → stack/scopes/vars/eval/step → `disconnect`); no attach/source/exception APIs
- **Agent SDK** (`--features agent-sdk`): `AgentHandle` builder API returning TOON
- **Discovery**: resolve `debugpy-adapter` via PATH + `~/.local/bin` + uv tools default layout
- **Harness**: `just harness-env` / `just harness`; e2e `agent_debug_loop`; Tier-1 passthrough tests
- **Docs**: README three modes; tip `docs/tips/00-tool-path-discovery.md` (joint dapz+lspz follow-up)

## v0.0.1 (2026-05-12)

Crate name registration — initial dapz project scaffolding.

- Project structure mirroring lspz architecture
- Cargo workspace with feature flags (cli, mcp, agent-sdk, transport-tcp, transport-websocket)
- JSON-RPC 2.0 codec with Content-Length frame parsing
- Transport abstraction (Transport trait) with StdioTransport, TcpTransport, MockTransport
- DAP proxy state machine (Created → Initializing → Ready → ShuttingDown → Exited)
- Interceptor trait and chain for Server→Client message transformation
- 3 compressors: OutputCompressor, VariablesCompressor, StackTraceCompressor
- Capping interceptor for limiting output/stack/variables sizes
- CLI entry point via clap with env-var overrides
- Unified error type (DapzError) with thiserror
- Config struct with builder pattern and env-var fallbacks
- SSOT documentation generation framework
- Justfile, prek hooks, CI configuration
