# dapz Project Roadmap

> **dap** **z**ip — 对 AI Coding Agent 友好的 DAP 压缩代理
> **当前**：v0.4.0（准发布能力已齐并含 `dapz init`；相对 lspz 为早期 0.x）

## 项目愿景

构建**三模态 DAP 压缩代理**：Proxy / MCP(+daemon) / Agent SDK，用 token 敏感压缩让调试观测更省上下文。

---

## 三种产品形态

```
                    ┌──────────────────────┐
                    │   dapz (单一 crate)   │
                    │  Feature flags 编译   │
                    └───────────┬──────────┘
                                │
        ┌───────────────────────┼───────────────────────┐
        │                       │                       │
        ▼                       ▼                       ▼
┌───────────────────┐  ┌───────────────────┐  ┌───────────────────┐
│  Library Mode     │  │   Proxy Mode      │  │   MCP Mode        │
│  (agent-sdk 等)   │  │   (default=cli)   │  │   (feature mcp)   │
└───────────────────┘  └───────────────────┘  └───────────────────┘
```

| 模式 | 目标用户 | 典型场景 | 集成方式 |
|------|----------|----------|----------|
| **Library** | 自研 Agent | 嵌入 `AgentHandle` | `features = ["agent-sdk"]` |
| **Proxy** | IDE / 管道 | 透明压缩 DAP 帧 | `dapz proxy --backend "…"` |
| **MCP** | Cursor 等 | 工具化调试观测 | `dapz mcp`（默认经 daemon） |

---

## 功能特性总览

### 拦截器（契约 `dapz-compress/1`）

| 拦截器 | 覆盖 | 策略概要 |
|--------|------|----------|
| `OutputCompressor` | `output` | ANSI / 重复行 / 噪音字段 |
| `VariablesCompressor` | `variables` | 截断、类型前缀、数组摘要 |
| `StackTraceCompressor` | `stackTrace` | 路径缩短、滤 synthetic |
| `ScopesCompressor` | `scopes` | 去掉 location 噪音 |
| `EvaluateCompressor` | `evaluate` | 截断 result |
| `ExceptionInfoCompressor` | `exceptionInfo` | 截断 details |
| `CappingInterceptor` | 大响应 | 条数 / 长度上限 |

### 输出格式（与 lspz 对齐）

| 格式 | Proxy | MCP / Agent SDK |
|------|-------|-----------------|
| `toon`（默认） | `body = {format:toon,text}` | 直接 TOON 文本（`toon-format`） |
| `json` | 压缩后的 DAP JSON 帧 | — |
| `passthrough` | 原字节，跳过拦截链 | — |

### 传输层

| 传输 | 说明 |
|------|------|
| `StdioTransport` | 子进程 stdio（默认） |
| `TcpTransport` | `tcp://host:port` |

> WebSocket stub 已移除（DAP 不需要）。

### 后端发现

- **必测**：debugpy（`uv tool install debugpy` 等，无需手改 PATH）
- **可选**：`lldb-dap` / `lldb-vscode`（C/C++/Rust）

### 相对 lspz：刻意不做

| 能力 | 状态 |
|------|------|
| `dapz init` / `--global`（Claude Code MCP 注入） | ✅ 已对齐 lspz |
| MCP workspace roots / `uri` | **跳过**（DAP 用 `backend` + `cwd`） |
| `config_watcher` | **跳过** |

---

## 版本历史

| 版本 | 里程碑 |
|------|--------|
| **v0.0.1** | crate name locking |
| **v0.1.0** | MVP：Proxy + TOON + MCP + Agent SDK + harness |
| **v0.2.0** | SDD + AgentPool + verify + metrics + Tier-1 exception |
| **v0.3.0** | daemon + MCP 默认连 daemon |
| **v0.3.1** | lifecycle / lldb / Proxy 输出格式 / 协议硬化 / bench 同步 |
| **v0.4.0** *(当前)* | `dapz init` 对齐 lspz（Claude Code MCP 注入） |
| **后续 0.x** | 可选更多 adapter profile；经 llman SDD 推进 |

详见 [CHANGELOG.md](CHANGELOG.md)。准发布实施记录见 [`_PLAN.md`](_PLAN.md)。

## 快速验证

```bash
just qa            # fmt + clippy + test + doc + prek
just harness       # 本地准发布门禁（debugpy e2e；lldb 可选）
just gen-bench     # 刷新 benchmarks.md + README BENCH-SUMMARY
```

```bash
dapz proxy --backend "…"
dapz mcp                   # 需 --features mcp
dapz daemon --cwd .
```

增量变更走 **llman SDD**（`llmanspec/` + `/llman-sdd-*` skills）。

## 技术栈

- **Rust** 2024 edition（`rust-toolchain.toml`）
- **Tokio** · **Serde** · **Clap** · **tracing** · **toon-format** · **rmcp**（mcp）

## 文档导航

- [README.md](README.md) — 用户入口
- [AGENTS.md](AGENTS.md) — 项目规范 SSOT
- [docs/specs/002-dap-compatibility.md](docs/specs/002-dap-compatibility.md) — adapter / threadId / 压缩契约
- [docs/src/benchmarks.md](docs/src/benchmarks.md) — 压缩基准（`just gen-bench`）

## 许可证

MIT
