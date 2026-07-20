# Tip: 常用包管理器默认路径自动发现（dapz + lspz）

> **状态**：待办 — 准发布后 **dapz / lspz 一起实现**（本 tip 为 SSOT 备忘）。
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

## 建议 API（两边对齐）

```text
resolve_tool(name) -> Option<PathBuf>
  1. PATH 上的可执行文件
  2. ~/.local/bin/<name>
  3. 语言生态默认目录（uv tools / cargo / go …）
  4. （可选）已知 wrapper 的 shebang → 同目录 python -m …
```

- **dapz**：`debugpy-adapter` / `python -m debugpy.adapter`（已在 `adapters.rs` + `check-env.sh` 落地第一版）
- **lspz**：`basedpyright-langserver`、`rust-analyzer`、`gopls`、`typescript-language-server` 等（`languages.rs` 的 `which` 需扩展）

## 验收

- 干净 shell（`env -i HOME=$HOME PATH=/usr/bin:/bin`）+ 仅默认安装位置 → harness / MCP 仍能找到工具
- 文档写明：推荐 `uv tool install …`，无需 export PATH
- 两边共享同一套搜索顺序表（可复制，暂不强制抽 crate）

## 实现排期

- [x] dapz：debugpy 最小发现（本轮 harness 用）
- [ ] dapz：补齐 cargo/其他 adapter；单元测 mock HOME
- [ ] lspz：扩展 `languages.rs` / MCP backend resolve
- [ ] 两边 README「安装」节引用本 tip
