# dapz

> **dap** **z**ip — 对 AI Coding Agent 极其友好的 DAP 压缩代理

[![Rust](https://img.shields.io/badge/rust-2024%20edition-orange.svg)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

**dapz** 是一个对 AI Coding Agent 极其友好的 DAP (Debug Adapter Protocol) 压缩代理。
它将 Debug Adapter Protocol 的 Server→Client 消息进行 Token 敏感的智能压缩，
让 LLM 用更少的上下文理解更多调试信息。

---

## 快速开始

```bash
# 作为 DAP 代理运行（推荐）
dapz --backend lldb-vscode

# 使用环境变量配置
DAPZ_BACKEND_CMD="lldb-vscode" dapz

# 限制输出
DAPZ_MAX_OUTPUT_LENGTH=2000 dapz --backend debug-adapter
```

## 压缩策略

| 拦截器 | 覆盖事件/响应 | 压缩策略 | 预期节省 |
|--------|------------|----------|---------|
| `OutputCompressor` | `output` 事件 | 重复行折叠 + 类别缩写 | 30-60% |
| `VariablesCompressor` | `variables` 响应 | 长值截断 | 20-40% |
| `StackTraceCompressor` | `stackTrace` 响应 | 路径缩写 + 函数参数裁剪 | 40-70% |
| `CappingInterceptor` | output/stackTrace/variables | 截断到 N 条后压缩 | 80-95% |

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

## 环境变量

| 变量 | 说明 | 默认值 |
|------|------|--------|
| `DAPZ_BACKEND_CMD` | 后端 DAP 服务器命令 | 必填 |
| `DAPZ_LOG_LEVEL` | 日志级别 | `info` |
| `DAPZ_OUTPUT_FORMAT` | 输出格式 (`json`, `passthrough`) | `json` |
| `DAPZ_ENABLE_OUTPUT_COMPRESS` | 输出事件压缩 | `true` |
| `DAPZ_ENABLE_VARIABLES_COMPRESS` | variables 响应压缩 | `true` |
| `DAPZ_ENABLE_STACKTRACE_COMPRESS` | stackTrace 响应压缩 | `true` |
| `DAPZ_MAX_FRAMES` | 最大栈帧数 | `0` (不限) |
| `DAPZ_MAX_VARIABLES` | 最大变量数 | `0` (不限) |
| `DAPZ_MAX_OUTPUT_LENGTH` | 输出文本最大字符数 | `0` (不限) |

## 开发

```bash
just setup   # 安装 prek hooks
just fmt     # 格式化代码
just lint    # clippy 检查
just test    # 运行测试
just qa      # 全部检查
```

## 许可证

MIT
