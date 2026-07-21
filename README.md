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
dapz = { version = "0.4", default-features = false, features = ["agent-sdk"] }
```

**CLI 代理** — 透明压缩 DAP 流量：

```bash
dapz proxy --backend "python3 -m debugpy.adapter"
dapz proxy --output json --backend "python3 -m debugpy.adapter"   # IDE / 结构化管道
dapz proxy --metrics --backend "python3 -m debugpy.adapter"
dapz proxy --transport tcp://127.0.0.1:4711 --backend unused
```

**MCP 服务器** — 把调试能力暴露给 Cursor / Claude Desktop 等（默认经 daemon 复用会话）：

```bash
cargo install dapz --features mcp
dapz mcp                 # 自动连接/启动 dapz daemon（按项目 cwd）
dapz mcp --no-daemon     # 进程内 adapter
dapz daemon              # 单独长驻；dapz daemon --cwd . list
```

工具：`debug_launch` / `debug_attach` → `get_stack` / `get_scopes` / `get_variables` / `evaluate` / `get_exception` → `step_*` / `continue` → `disconnect`（21 tools）。

Claude Code 一键注入（对齐 lspz）：

```bash
cargo install dapz --features mcp
dapz init --global          # ~/.claude.json + ~/.claude/DAPZ.md + @DAPZ.md
dapz init --global --show
dapz init --global --uninstall
```

## 压缩效果

用 `cl100k_base`（tiktoken）在 `fixtures/bench` 上测量。紧凑 = 拦截器后的 DAP JSON；TOON = 压缩后 `body` 经 `value_to_toon`（对齐 MCP/SDK 默认出口）。

<!-- BENCH-SUMMARY:START -->
| 拦截器 | DAP 消息 | 紧凑 vs 原始 | TOON vs 原始 |
|--------|---------|-------------|-------------|
| OutputCompressor | `output` | 44.5% | 59.8% |
| VariablesCompressor | `variables` | 43.6% | 55.4% |
| StackTraceCompressor | `stackTrace` | 52.1% | 44.8% |
| ScopesCompressor | `scopes` | 43.9% | 58.8% |
| EvaluateCompressor | `evaluate` | 8.1% | 33.9% |
| ExceptionInfoCompressor | `exceptionInfo` | 17.9% | 29.0% |
| CappingInterceptor | 大响应截断 | 40.6% | 63.7% |
| **总体（20 场景）** | — | **40.0%** | **49.3%** |

> 紧凑格式对小输入收益有限；TOON 在 MCP/SDK 路径下额外省 token。完整表：[docs/src/benchmarks.md](docs/src/benchmarks.md)（`just gen-bench` 同步更新本表与完整报告）；快速演示：`just compress-demo`。
<!-- BENCH-SUMMARY:END -->

## 输出格式

| 格式 | 说明 | 适用场景 |
|------|------|---------|
| `toon`（默认） | 自描述行协议 + 表格；Proxy 将 `body` 包成 `{format:toon,text}`，MCP/SDK 直接返回 TOON 文本 | LLM / Agent 消费 |
| `json` | 拦截链压缩后的紧凑 DAP JSON 帧 | IDE：`dapz proxy --output json` |
| `passthrough` | 原始 DAP JSON 不动 | 调试 |

Proxy 额外旋钮：`--metrics`（或 `DAPZ_METRICS`）、`--transport stdio|tcp://host:port`（默认 stdio）。

## Feature flags

| Flag | 启用内容 | 默认 |
|------|---------|------|
| `cli` | `dapz` 二进制（clap、tracing-subscriber） | 开 |
| `mcp` | MCP + daemon（rmcp） | 关 |
| `agent-sdk` | AgentHandle + AgentPool（隐含 `mcp`） | 关 |
| `transport-tcp` | TcpTransport（代码始终包含；flag 兼容） | 关 |

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
# 从 crates.io（默认 cli）
cargo install dapz

# 启用 MCP / daemon
cargo install dapz --features mcp

# Agent SDK（隐含 mcp）
cargo install dapz --features agent-sdk

# 从源码
git clone https://github.com/straydragon/dapz && cd dapz
cargo install --path . --features mcp
```

### 调试适配器（无需改 PATH）

推荐用包管理器默认布局；dapz 按 PATH → `~/.local/bin` → uv tools / cargo 等顺序发现：

```bash
# 必测：Python debugpy
uv tool install debugpy

# 可选：C/C++/Rust（lldb-dap / lldb-vscode）
# pacman -S lldb   # 或系统包提供 lldb-dap
```

详见 [docs/tips/00-tool-path-discovery.md](docs/tips/00-tool-path-discovery.md)。

## 快速验证

```bash
just compress-demo   # 拦截器 token 演示
just qa              # fmt + clippy + test + doc-check + prek
just verify          # qa + doc-test + SDD + prek
just harness-env     # rustc / python / debugpy / optional lldb / mcp
just harness         # 全部门禁（debugpy e2e 必测；lldb e2e 可选）
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
- [CHANGELOG.md](CHANGELOG.md) / [ROADMAP.md](ROADMAP.md) — 版本与路线
- [DAP 兼容面](docs/specs/002-dap-compatibility.md) — adapter / threadId / `dapz-compress/1`
- [压缩基准](docs/src/benchmarks.md) — `just gen-bench`
- [DAP 规范](docs/DAP-Specification.html) — Debug Adapter Protocol 参考
- [AGENTS.md](AGENTS.md) — 项目约定与架构不变量
- [llmanspec/](llmanspec/) — SDD 规格（`llman sdd list --specs`）

## 许可证

MIT
