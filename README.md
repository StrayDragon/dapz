<div align="center">
  <img src="docs/assets/logo.svg" alt="dapz" width="160" height="160"/>

  # dapz

  [![](https://img.shields.io/crates/v/dapz?style=flat-square&logo=rust&label=crates.io)](https://crates.io/crates/dapz)
  [![](https://img.shields.io/crates/dr/dapz?style=flat-square&logo=rust)](https://crates.io/crates/dapz)
  [![](https://img.shields.io/docsrs/dapz?style=flat-square&logo=docsdotrs&label=docs.rs)](https://docs.rs/dapz)
  [![](https://img.shields.io/github/stars/straydragon/dapz?style=flat-square&logo=github)](https://github.com/straydragon/dapz/stargazers)
  [![](https://img.shields.io/github/actions/workflow/status/straydragon/dapz/ci.yml?style=flat-square&logo=github&label=CI)](https://github.com/straydragon/dapz/actions)
  [![](https://img.shields.io/crates/l/dapz?style=flat-square&color=blue)](https://github.com/straydragon/dapz/blob/main/LICENSE)

  **dap** **z**ip — 压缩 DAP 消息，给 AI 智能体省 token

</div>

---

dapz 跑在 AI 编码智能体和 DAP 调试适配器之间。它拦截调试器响应（`stackTrace`、`variables`、`evaluate`、`output`、`exceptionInfo`），重写成更紧凑的格式，少用不少 token。按 token 计费或者上下文窗口有限的时候尤其有用。

## 工作原理

```
Agent (DAP 客户端) ←→ dapz ←→ DAP 服务器 (debugpy, lldb-dap, ...)
```

客户端发给服务器的请求原样透传。服务器返回的响应经过一组拦截器：去掉冗余字段、缩写路径、截断长值、删除无用元数据。任何拦截器出错就转发原始消息——不会搞坏你的调试会话。

拦截器链（失败均透传）：

```
DAP Server → [Capping] → [Output] → [Evaluate] → [Variables]
           → [StackTrace] → [Scopes] → [ExceptionInfo] → Agent
```

## 三种用法

**库** — 嵌入你自己的 Rust 智能体：

```toml
[dependencies]
dapz = { version = "0.3", default-features = false, features = ["agent-sdk"] }
```

**CLI 代理** — 透明压缩 DAP 流量：

```bash
dapz proxy --backend "python3 -m debugpy.adapter"
dapz proxy --output toon --backend "python3 -m debugpy.adapter"
dapz proxy --metrics --backend "python3 -m debugpy.adapter"
dapz proxy --transport tcp://127.0.0.1:4711 --backend unused
```

**MCP 服务器** — 把调试能力暴露给 Cursor 等 Agent（默认经 daemon 复用会话）：

```bash
cargo install dapz --features mcp
dapz mcp                 # 自动连接/启动 dapz daemon（按项目 cwd）
dapz mcp --no-daemon     # 进程内 adapter
dapz daemon              # 单独长驻；dapz daemon --cwd . list
```

工具：`debug_launch` / `debug_attach` → `get_stack` / `get_scopes` / `get_variables` / `evaluate` / `get_exception` → `step_*` / `continue` → `disconnect`（21 tools）。

## 压缩效果

用 `cl100k_base`（tiktoken）在 `fixtures/bench` 上测量。紧凑 = 拦截器后的 DAP JSON；TOON = 压缩后 `body` 经 `value_to_toon`（对齐 MCP/SDK 默认出口）。

| 拦截器 | DAP 消息 | 紧凑 vs 原始 | TOON vs 原始 |
|--------|---------|-------------|-------------|
| OutputCompressor | `output` | 44.4% | 59.7% |
| VariablesCompressor | `variables` | 43.6% | 65.2% |
| StackTraceCompressor | `stackTrace` | 51.1% | 57.5% |
| ScopesCompressor | `scopes` | 44.3% | 68.4% |
| EvaluateCompressor | `evaluate` | 8.1% | 31.0% |
| ExceptionInfoCompressor | `exceptionInfo` | 17.7% | 28.7% |
| CappingInterceptor | 大响应截断 | 40.5% | 63.4% |
| **总体（20 场景）** | — | **39.8%** | **55.4%** |

> 紧凑格式对小输入收益有限；TOON 在 MCP/SDK 路径下额外省 token。完整表：`just bench-report`；快速演示：`just compress-demo`。

## 输出格式

| 格式 | 说明 | 适用场景 |
|------|------|---------|
| `json`（**proxy 默认**） | 标准 JSON | IDE / 调试器管道（产品冻结，不改成 toon） |
| `toon` | 自描述行协议 + 表格 | MCP / Agent SDK 默认；`dapz proxy --output toon` |
| `passthrough` | 原始 DAP JSON 不动 | 调试 |

Proxy 额外旋钮：`--metrics`（或 `DAPZ_METRICS`）、`--transport stdio|tcp://host:port|ws://…`（默认 stdio；ws 需 `transport-websocket`，实现仍为 stub）。

## Feature flags

| Flag | 启用内容 | 默认 |
|------|---------|------|
| `cli` | `dapz` 二进制（clap、tracing-subscriber） | 开 |
| `mcp` | MCP + daemon（rmcp） | 关 |
| `agent-sdk` | AgentHandle + AgentPool（隐含 `mcp`） | 关 |
| `transport-tcp` | TcpTransport | 关（代码始终包含） |
| `transport-websocket` | WsTransport | 关 |

## Agent SDK

默认**进程内** spawn adapter（DAP 按需短会话）。若要与 `dapz mcp` / `dapz daemon` **共享会话**，opt-in：

```rust
use dapz::agent_sdk::AgentHandle;

// 默认：in-process
let mut agent = AgentHandle::builder()
    .backend("python3 -m debugpy.adapter")
    .start()
    .await?;

// 与 MCP 复用同一 daemon（连不上则报错，不静默回退）
let mut shared = AgentHandle::builder()
    .backend("python3 -m debugpy.adapter")
    .cwd("/path/to/project")
    .via_daemon(true)
    .start()
    .await?;

let stopped = agent
    .launch("script.py", None, None, Some(&[("script.py".into(), vec![10])]))
    .await?;
let stack = agent.get_stack(Some(1), Some(20)).await?;
agent.disconnect(Some(true)).await?;
```

完整 API 见 [docs.rs/dapz](https://docs.rs/dapz/latest/dapz/agent_sdk/)。

## Daemon

长驻 Unix socket（`~/.cache/dapz/<slug>-<hash>.sock`），按 **project cwd** 派生（非 LSP workspace roots）。会话 key = `backend::cwd`。空闲会话 / 空闲 daemon 会自动回收。

```bash
dapz daemon --cwd /path/to/project
dapz daemon --cwd /path/to/project list
```

## 安装

```bash
# 推荐：安装 debugpy 后端（harness 必测）
uv tool install debugpy
# 可选：lldb-dap / lldb-vscode（C/C++/Rust 映射）

cargo install dapz
cargo install dapz --features mcp
cargo install dapz --features agent-sdk

# 从源码
git clone https://github.com/straydragon/dapz && cd dapz
cargo install --path . --features mcp
```

路径发现会搜 `PATH`、`~/.local/bin`、uv tools 等，详见 [docs/tips/00-tool-path-discovery.md](docs/tips/00-tool-path-discovery.md)。

## 快速验证

```bash
just compress-demo   # 拦截器 token 演示
just qa              # fmt + clippy + test + doc-check + prek
just verify          # qa + doc-test + SDD + prek
just harness-env     # rustc / python / debugpy / mcp
just harness         # 全部门禁（含 debugpy e2e）
```

## 开发

```bash
just setup            # 安装 prek hooks
just fmt              # 格式化
just lint             # clippy
just test             # 单元 + 集成（非 ignored）
just test-integration # debugpy 集成（需 adapter）
just bench            # Criterion
```

Proxy CLI 常用参数见 `dapz proxy --help`（`--backend`、`--output`、capping 上限等）。

## 文档

- [API 参考](https://docs.rs/dapz) — 从 `///` 注释自动生成
- [DAP 规范](docs/DAP-Specification.html) — Debug Adapter Protocol 参考
- [AGENTS.md](AGENTS.md) — 项目约定与架构不变量
- [llmanspec/](llmanspec/) — SDD 规格（`llman sdd list --specs`）
- [拦截器细节](docs/) — 压缩策略与架构图（`docs/src/`）

## 许可证

MIT
