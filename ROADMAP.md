# dapz Project Roadmap

> **dap** **z**ip — 对 AI Coding Agent 极其友好的 DAP 压缩代理

## 项目愿景

构建一个**三模态 DAP 压缩代理系统**，通过 Token 敏感的智能压缩，让 AI Coding Agent
用更少上下文理解更多调试信息。

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
│  (no-default)     │  │   (default=cli)   │  │   (feature mcp)   │
└───────────────────┘  └───────────────────┘  └───────────────────┘
```

| 模式 | 目标用户 | 典型场景 | 集成方式 |
|------|----------|----------|----------|
| **Library** | 自研 Agent CLI | 完全控制，零开销 | `dapz::Proxy` / crate 的直接引用 |
| **Proxy** | Claude Code/Continue/Cody | 即插即用，透明代理 | `dapz proxy --backend "python3 -m debugpy.adapter"` |
| **MCP** | 快速实验/多工具协同 | 融入生态，按需查询 | MCP server 配置 |

---

## 功能特性总览

### 拦截器

| 拦截器 | 覆盖方法 | 压缩策略 | Token 节省 |
|--------|----------|----------|-----------|
| `OutputCompressor` | `output` 事件 | 重复行折叠 + 类别缩写 | 30-60% |
| `VariablesCompressor` | `variables` 响应 | 长值截断 + 类型前缀 | 20-40% |
| `StackTraceCompressor` | `stackTrace` 响应 | 路径缩写 + 函数参数裁剪 | 40-70% |
| `CappingInterceptor` | output / stackTrace / variables | 截断到 N 条后压缩 | 80-95% |

### 传输层

| 传输 | Feature Flag | 说明 |
|------|-------------|------|
| `StdioTransport` | always | 子进程 stdio（默认） |
| `TcpTransport` | always | TCP socket 连接 |


## 版本历史

| 版本 | 里程碑 |
|------|--------|
| **v0.0.1** | crate name locking |
| **v0.1.0** | MVP: Proxy + TOON + MCP + Agent SDK + harness |
| **v0.2.0** *(当前)* | SDD + AgentPool + verify + metrics + Tier-1 exception APIs |
| **v0.3.0** *(规划)* | daemon（长驻 DAP session；见 `_PLAN.md` §9 P5） |

## 快速验证

```bash
just fmt           # 格式化
just lint          # clippy 检查
just test          # 运行测试
just qa            # 全部检查
just harness-env   # debugpy / 路径发现
just harness       # 本地准发布门禁（含 e2e）
```

### CLI 形态（与 lspz 对齐）

```bash
dapz proxy --backend "…"   # 透明 DAP 压缩代理
dapz mcp                   # MCP server（需 --features mcp）
```

增量变更走 **llman SDD**（`llmanspec/` + `/llman-sdd-*` skills）。

## 技术栈

- **Rust** 2024 edition
- **Tokio** 异步运行时
- **Serde** + **serde_json** 序列化
- **Clap** CLI 参数解析

## 文档导航

### 设计规格
- [docs/specs/001-tri-modal-architecture.md](docs/specs/001-tri-modal-architecture.md) — 三模态架构

### 开发指南
- [AGENTS.md](AGENTS.md) — 项目规范 SSOT

## 许可证

MIT
