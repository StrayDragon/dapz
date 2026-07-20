<div align="center">
  <img src="docs/assets/logo.svg" alt="dapz" width="160" height="160"/>

  # dapz

  [![](https://img.shields.io/crates/v/dapz?style=flat-square&logo=rust&label=crates.io)](https://crates.io/crates/dapz)
  [![](https://img.shields.io/docsrs/dapz?style=flat-square&logo=docsdotrs&label=docs.rs)](https://docs.rs/dapz)
  [![](https://img.shields.io/crates/l/dapz?style=flat-square&color=blue)](https://github.com/straydragon/dapz/blob/main/LICENSE)
  [![](https://img.shields.io/badge/edition-2024-orange?style=flat-square)](https://blog.rust-lang.org/2025/02/20/Rust-2024-Edition.html)
  [![](https://img.shields.io/github/actions/workflow/status/straydragon/dapz/ci.yml?style=flat-square&logo=github&label=CI)](https://github.com/straydragon/dapz/actions)

  **dap** **z**ip — 压缩 DAP 消息，给 AI 智能体省 token

  [快速开始](#快速开始) · [压缩效果](#压缩效果) · [架构](#架构) · [API 文档](https://docs.rs/dapz)
</div>

---

**dapz** 跑在 AI 编码智能体和 DAP 调试适配器之间。它拦截调试器响应（`stackTrace`、`variables`、`evaluate`、`output`），重写成更紧凑的格式，少用不少 token。按 token 计费或者上下文窗口有限的时候尤其有用。

## 架构

```
Agent (DAP 客户端) ←→ dapz ←→ DAP 服务器 (debugpy, lldb-vscode, ...)
```

客户端发给服务器的请求原样透传。服务器返回的响应经过一组拦截器：去掉冗余字段、缩写路径、截断长值、删除无用元数据。任何拦截器出错就转发原始消息——不会搞坏你的调试会话。

### 拦截器链

```
DAP Server → [CappingInterceptor] → [OutputCompressor] → [EvaluateCompressor]
           → [VariablesCompressor] → [StackTraceCompressor] → [ScopesCompressor] → Agent
```

| 拦截器 | 作用 | 失败行为 |
|--------|------|---------|
| `CappingInterceptor` | 截断超长 output/stackTrace/variables | 透传原始消息 |
| `OutputCompressor` | 折叠重复行、缩写类别、去 ANSI、缩写 source | 透传原始消息 |
| `EvaluateCompressor` | 截断结果、移除内存地址 | 透传原始消息 |
| `VariablesCompressor` | 类型前缀、数组摘要、长值截断、移除噪声字段 | 透传原始消息 |
| `StackTraceCompressor` | 过滤合成帧、路径去重+缩写、函数参数裁剪、移除噪声字段 | 透传原始消息 |
| `ScopesCompressor` | 去掉 source/line 等位置噪声，保留 variablesReference | 透传原始消息 |

## 压缩效果

用 tiktoken 在真实 debugpy DAP 会话输出上测量。

| 拦截器 | DAP 消息 | 主要策略 | 预期节省 |
|--------|---------|---------|---------|
| OutputCompressor | `output` 事件 | 重复行折叠 + 类别缩写 + ANSI 剥离 | 30–60% |
| VariablesCompressor | `variables` 响应 | 类型前缀合并 + 数组摘要 + 长值截断 | 40–60% |
| StackTraceCompressor | `stackTrace` 响应 | 路径缩写 + 函数裁剪 + 合成帧过滤 + 源去重 | 40–70% |
| EvaluateCompressor | `evaluate` 响应 | 结果截断 + 内存地址移除 | 10–30% |
| CappingInterceptor | 任意大响应 | 截断到 N 条 | 80–95% |

> **注意**: 实际节省取决于调试会话的具体情况。深层调用栈 + 大数组变量的场景节省最大。

## 快速开始

### 安装 debug 后端（推荐）

```bash
uv tool install debugpy
# 无需 export PATH：dapz 会自动搜 ~/.local/bin 与 uv tools 默认布局
# 详见 docs/tips/00-tool-path-discovery.md
```

### 三种用法

**CLI 代理** — 透明压缩 DAP 流量：

```bash
dapz proxy --backend "python3 -m debugpy.adapter"
dapz proxy --output toon --backend "python3 -m debugpy.adapter"
```

**MCP 服务器** — 把调试能力暴露给 Cursor 等 Agent（默认经 daemon 复用会话）：

```bash
cargo install dapz --features mcp
dapz mcp                 # 自动连接/启动 dapz daemon（按项目 cwd）
dapz mcp --no-daemon     # 进程内 adapter（不经 daemon）
dapz daemon              # 单独长驻；dapz daemon list 查看状态
```

工具：`debug_launch` / `debug_attach` → `get_stack` / `get_scopes` / `get_variables` / `evaluate` / `get_exception` → `step_*` / `continue` → `disconnect`（21 tools；详见 `_PLAN.md` §3）。

**Agent SDK** — 嵌入 Rust Agent：

```toml
dapz = { version = "0.3", default-features = false, features = ["agent-sdk"] }
```

```rust
use dapz::agent_sdk::AgentHandle;

let mut agent = AgentHandle::builder()
    .backend("python3 -m debugpy.adapter")
    .start()
    .await?;
let stopped = agent.launch("script.py", None, None, Some(&[("script.py".into(), vec![10])])).await?;
let stack = agent.get_stack(Some(1), Some(20)).await?;
```

### 作为库使用（拦截器）

```toml
[dependencies]
dapz = { version = "0.3", default-features = false }
```

```rust
use dapz::interceptors::InterceptorChain;
use dapz::interceptors::output::OutputCompressor;
use dapz::interceptors::stacktrace::StackTraceCompressor;

let chain = InterceptorChain::new(
    vec![Box::new(OutputCompressor), Box::new(StackTraceCompressor)],
    config,
);
let compressed = chain.process(msg, Direction::ServerToClient).await?;
```

## 验证

```bash
just qa            # fmt + clippy + test
just harness-env   # rustc / python / debugpy / mcp build
just harness       # 全部门禁（含 debugpy e2e）
```

## CLI 选项

| 参数 | 环境变量 | 说明 | 默认值 |
|------|---------|------|--------|
| `--backend` | `DAPZ_BACKEND_CMD` | 后端 DAP 服务器命令 | **必填** |
| `--log-level` | `DAPZ_LOG_LEVEL` | 日志级别 | `info` |
| `--output` | `DAPZ_OUTPUT_FORMAT` | 输出格式 | `json` |
| `--compress-output` | `DAPZ_ENABLE_OUTPUT_COMPRESS` | 启用 output 压缩 | `true` |
| `--compress-variables` | `DAPZ_ENABLE_VARIABLES_COMPRESS` | 启用 variables 压缩 | `true` |
| `--compress-stacktrace` | `DAPZ_ENABLE_STACKTRACE_COMPRESS` | 启用 stackTrace 压缩 | `true` |
| `--compress-evaluate` | `DAPZ_ENABLE_EVALUATE_COMPRESS` | 启用 evaluate 压缩 | `true` |
| `--max-frames` | `DAPZ_MAX_FRAMES` | 最大栈帧数 | `0` (不限) |
| `--max-variables` | `DAPZ_MAX_VARIABLES` | 最大变量数 | `0` (不限) |
| `--max-output-length` | `DAPZ_MAX_OUTPUT_LENGTH` | output 最大字符数 | `0` (不限) |
| `--max-evaluate-length` | `DAPZ_MAX_EVALUATE_LENGTH` | evaluate 结果最大字符数 | `500` |
| `--max-value-length` | `DAPZ_MAX_VALUE_LENGTH` | 变量值最大字符数 | `120` |

## Feature flags

| Flag | 启用内容 | 默认 |
|------|---------|------|
| `cli` | `dapz` 二进制 (clap, tracing-subscriber) | 开 |
| `mcp` | MCP 服务器 (rmcp) | 关 |
| `agent-sdk` | AgentHandle + AgentPool (隐含 `mcp`) | 关 |
| `transport-tcp` | TcpTransport | 关 (代码始终包含) |
| `transport-websocket` | WsTransport | 关 |

## 压缩策略详解

### OutputCompressor

1. **ANSI 剥离** — 移除 `\x1b[...m` CSI 颜色/样式序列（LLM 无法渲染）
2. **重复行折叠** — `line\nline\nline` → `line (x3)`
3. **类别缩写** — `stdout`→`O`, `stderr`→`E`, `console`→`C`
4. **Source 缩写** — 缩短路径、移除 checksums/adapterData
5. **噪声字段移除** — 删除 `data`、`line`、`column`

### VariablesCompressor

1. **类型前缀** — `"name": "x", "type": "int"` → `"name": "int: x"`（省一个字段）
2. **数组摘要** — `indexedVariables > 0` → `Array[0..N)`
3. **值截断** — 超长字符串截断到 `max_value_length` 字符
4. **噪声字段移除** — 删除 `memoryReference`、`declarationLocationReference`、`valueLocationReference`

### StackTraceCompressor

1. **合成帧过滤** — 移除 `presentationHint: "label"` 或 `"subtle"` 的帧
2. **源路径缩写** — `/home/user/project/src/main.rs` → `src/main.rs`
3. **源路径去重** — 连续相同路径 → 空字符串（隐含重复）
4. **函数名裁剪** — `foo(a: i32, b: String)` → `foo(...)`
5. **噪声字段移除** — 删除 `instructionPointerReference`、`moduleId`

### EvaluateCompressor

1. **结果截断** — `result` 字符串截断到 `max_evaluate_length`
2. **内存地址移除** — 删除 `memoryReference`
3. **保留** — `type`、`variablesReference`、`namedVariables`、`indexedVariables`、`presentationHint`

## 开发

```bash
just setup            # 安装 prek hooks
just fmt              # 格式化代码
just lint             # clippy 检查
just test             # 运行单元测试
just test-integration # 运行 debugpy 集成测试（需要 debugpy-adapter）
just bench            # 运行性能基准测试
just qa               # 全部检查（fmt + lint + test）
```

### 集成测试

推荐 `uv tool install debugpy`。adapter 不必在 PATH 上——`resolve_tool` / harness 会查 `~/.local/bin` 与 uv tools 默认目录。

```bash
just harness-env
just test-integration
```

测试会启动真实的 debugpy 会话，执行 DAP 握手，发送 `stackTrace`/`variables`/`evaluate` 请求，并通过拦截器链验证压缩效果。

## 许可证

MIT
