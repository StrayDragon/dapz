# Tip: 常用包管理器默认路径自动发现（dapz + lspz）

> **状态**：dapz 已补齐（`resolve_tool` + mock HOME 单测 + clean-PATH harness）；lspz 仍待扩展。
> **触发**：2026-07-20 — `uv tool install debugpy` 已装在 `~/.local/bin`，但 Cursor agent shell 的 `PATH` 不含该目录；`python3 -c "import debugpy"` 也失败（uv tool 隔离 venv）。

## 问题

Agent / CI / 精简 PATH 环境下：

- `which debugpy` / `which basedpyright-langserver` 等失败
- 系统 `python3` 无法 `import` 进 uv/pipx 工具环境里的包
- 只能靠用户改 `PATH` / 环境变量 → 脆弱

## 目标

两个仓库都只依赖**常用包管理器默认布局**自动发现，不要求用户改环境变量：

| 来源 | 典型路径 |
|------|----------|
| 用户 bin（uv tool / pipx） | `$HOME/.local/bin/<tool>` |
| uv tools 隔离环境 | `$XDG_DATA_HOME/uv/tools/<pkg>/bin/`（默认 `~/.local/share/uv/tools/...`） |
| cargo install | `$CARGO_HOME/bin` 或 `~/.cargo/bin` |
| go install | `$GOPATH/bin` / `~/go/bin` |
| npm / bun 全局 | `~/.local/share/npm/bin`、`~/.bun/bin`（按需） |
| 系统 PATH | `which` 已有项优先 |

## API（两边对齐）

```text
resolve_tool(name) -> Option<PathBuf>
  1. PATH 上的可执行文件
  2. ~/.local/bin/<name>
  3. ~/.cargo/bin / ~/go/bin / npm|bun 默认目录
  4. （debugpy）uv tools wrapper / python -m debugpy.adapter
```

- **dapz**：`resolve_tool` / `DiscoveryContext` / `resolve_python_debug_adapter`（`adapters.rs` + `check-env.sh` clean-PATH 验收）
- **lspz**：`basedpyright-langserver`、`rust-analyzer`、`gopls`、`typescript-language-server` 等（`languages.rs` 的 `which` 需扩展）

## 验收

- 干净 shell（`env -i HOME=$HOME PATH=/usr/bin:/bin`）+ 仅默认安装位置 → harness / MCP 仍能找到工具
- 文档写明：推荐 `uv tool install …`，无需 export PATH
- 两边共享同一套搜索顺序表（可复制，暂不强制抽 crate）
- 单元测：mock HOME / XDG / CARGO_HOME（不依赖进程全局 env）

## 实现排期

- [x] dapz：debugpy 最小发现（本轮 harness 用）
- [x] dapz：`resolve_tool` + cargo/go/npm/bun；单元测 mock HOME；clean-PATH check-env
- [ ] lspz：扩展 `languages.rs` / MCP backend resolve
- [x] dapz README「安装」节引用本 tip
- [ ] lspz README「安装」节引用本 tip
