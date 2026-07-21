<!-- LLMANSPEC:START -->
# LLMAN Spec-Driven Development

This project uses llman SDD. Read `llmanspec/config.yaml` for SDD command behavior configuration, and `llmanspec/AGENTS.md` for additional project-specific rules.

## SDD Pipeline

Use `/llman-sdd-explore` to get started, then follow the pipeline: `/llman-sdd-propose` → `/llman-sdd-apply` → `/llman-sdd-verify` → `/llman-sdd-archive`.

Keep this managed block so `llman sdd init --update` can refresh it.
<!-- LLMANSPEC:END -->

# AGENTS.md

Project conventions for dapz (DAP compression proxy).

> **SSOT (Single Source of Truth)**: 本文档是 dapz 项目的通用规范文档。所有开发指南、文档都应引用本文档，避免重复和不一致。

---

## Rust

### Edition & Toolchain

- Edition 2024
- Stable toolchain (pinned in `rust-toolchain.toml`)
- Single crate `dapz` with feature flags (`cli`, `mcp`, `agent-sdk`, `transport-tcp`)

### Code Quality

- Clippy thresholds configured in `.clippy.toml`; lint flags in justfile
- Format config in `rustfmt.toml`
- Nightly-only options are not allowed

### 命名约定

| 类型 | 约定 | 示例 |
|------|------|------|
| Struct | `PascalCase` | `Proxy`, `Config` |
| Enum | `PascalCase` | `Direction`, `State` |
| Function | `snake_case` | `compress_output`, `truncate_value` |
| Const | `SCREAMING_SNAKE_CASE` | `MAX_BUFFER_SIZE` |
| Trait | `PascalCase` | `Transport`, `Interceptor` |

### 错误处理

- 使用 `Result<T, DapzError>` 统一错误类型
- 使用 `thiserror` 定义错误枚举
- 避免使用 `.unwrap()` 和 `.expect()`（除测试代码）

### 异步代码

- 使用 `tokio` 作为异步运行时
- 使用 `async-trait` 定义异步 trait
- 所有 I/O 操作基于异步

### 日志规范

- 使用 `tracing` 库（非 `log`）
- 日志级别：ERROR, WARN, INFO, DEBUG, TRACE
- 结构化日志，使用 `info!`, `debug!`, `error!`, `warn!`

---

## Scripts (`scripts/`)

- **Naming**: kebab-case
- **Avoid**: simple cargo command wrappers (call directly in justfile)
- **No underscores** in filenames

### 适合 scripts/ 的场景

- ✅ 复杂的验证逻辑（多步骤、条件判断）
- ✅ 需要与其他工具集成的脚本

### 不适合 scripts/ 的场景

- ❌ 简单的 `cargo fmt`, `cargo clippy`, `cargo test` 等

---

## Git Hooks (`prek.toml`)

- Only `repo = "builtin"` + `repo = "local"` — zero network dependency at runtime
- Local hooks: `pass_filenames = false` + `require_serial = true`
- Stages:
  - **pre-commit**: cargo-fmt, cargo-clippy (+ builtin checks)
  - **commit-msg**: conventional commits validation
  - **pre-push**: cargo-test

---

## Commits

- Conventional Commits format: `type(scope)!: description`
- Types: feat, fix, docs, style, refactor, perf, test, build, ci, chore, revert
- Subject line max 72 characters
- Validated by `scripts/check-conventional-commit.py`

---

## Task Runner (`justfile`)

- `just setup` = 安装 prek hooks
- `just fmt` = 格式化代码（写入）
- `just fmt-check` = 检查格式（只读）
- `just lint` = 运行 clippy
- `just test` = 运行测试
- `just qa` = fmt-check + lint + test
- `just bench-report` / `just gen-bench` = 从 `fixtures/bench` 再生压缩比率报告（写 `docs/src/benchmarks.md`，并同步 `README.md` 的 `BENCH-SUMMARY` 块）；`prek` pre-push 也会跑 `gen-bench`
- Shell: `bash -euo pipefail`

---

## 文档规范

### Markdown

- 使用 [Mermaid](https://mermaid.js.org/) 绘制图表
- 代码块指定语言：`rust`, `bash`
- 链接使用相对路径（内部文档）

### 代码注释

- 公共 API 必须有文档注释（`///` 或 `//!`）
- 模块级文档使用 `//!`

### 文档优先级

1. **AGENTS.md** (本文件) - 通用规范 SSOT
2. **docs/specs/** - 技术规格
3. **docs/plan/** - 阶段计划

---

## 测试规范

### 测试组织

- 单元测试：与源码同目录的 `tests` 模块
- 集成测试：`tests/` 目录
- 性能测试：`benches/` 目录

### 测试命名

- 测试函数：`test_<功能>_<场景>`
- 测试模块：`tests` 或 `<module>_tests`

---

## 依赖管理

| 依赖 | 用途 |
|------|------|
| tokio | 异步运行时 |
| serde | 序列化框架 |
| serde_json | JSON 支持 |
| async-trait | 异步 trait |
| thiserror | 错误定义 |
| tracing | 日志/追踪 |
| clap | CLI 参数解析 |

---

## 安全考虑

### 输入验证

- 验证外部输入（配置、命令参数）
- 避免未检查的索引访问（使用 `.get()`）
- 命令注入防护（验证 backend_cmd）

### 错误处理

- 压缩失败降级到透明转发
- 传输错误记录日志，不 panic
- 配置错误早期检测

---

## 版本管理

### 语义化版本

- v0.0.1: crate name locking
- v0.1.0: MVP（Proxy + TOON + MCP + Agent SDK + harness）
- v0.2.0: SDD + AgentPool + metrics + Tier-1 exception
- v0.3.0: daemon + MCP 默认连 daemon
- v0.3.1: lifecycle / lldb / Proxy 输出格式 / 协议硬化 / bench 同步
- v0.4.0: `dapz init`（Claude Code MCP，对齐 lspz）（当前）

### 检查点规则

1. **验证**: 运行 `just qa` 确保无错误
2. **提交**: 按照 Conventional Commits 格式提交
3. **版本号更新**: 在 Cargo.toml 中递增版本号
4. **打 tag**: 创建对应版本的 git tag: `git tag v{版本号}`

## 参考资源

- [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- [The Rust Style Guide](https://rust-lang.github.io/style-guide/)
- [Conventional Commits](https://www.conventionalcommits.org/)
- [DAP Specification](https://microsoft.github.io/debug-adapter-protocol/specification)
