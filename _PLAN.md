# dapz 准发布对齐计划（可实施 / 可验证）

> **临时 SSOT**：本文件驱动「一步到位模仿 `../lspz`」的实现。完成后归档到 `docs/plan/` 或删除。
> **规格依据**：[`docs/DAP-Specification.html`](docs/DAP-Specification.html)
> **模式依据**：`../lspz` v0.11.x（**只读参考，不抽共享 crate**）
> **策略**：Agent「发现 bug / 观测行为」**收紧后的最小闭环**；其余 **透传**。
> **SDD**：自 2026-07-20 起增量变更走 `llmanspec/` + `.agents/skills/llman-sdd-*`（见根 `AGENTS.md` managed block）。

---

## 0. 决策冻结

| 决策 | 选择 |
|------|------|
| 代码共享 | **分开写**；从 lspz **文件级 copy-port** |
| 准发布范围 | Proxy + TOON + MCP + Agent SDK + harness（**不做** daemon / SDD / 多 adapter） |
| Tier-0 收紧（2026-07-20） | **砍掉** `attach` / `source` / `exceptionInfo` / `setExceptionBreakpoints`（→ Tier-1 透传 + `send_raw`） |
| 参考后端 | **debugpy** 唯一必测 |
| 默认输出 | MCP/SDK：**TOON**；Proxy：`json` / `toon` / `passthrough` |
| 版本目标 | **v0.1.0** ✅ 已发布 |
| 后续工程 | **v0.2+** 经 llman SDD 逐项跟进 lspz 成熟度（见 §9） |

---

## 1. Agent 最小观测集（收紧版）

```text
launch → setBreakpoints → configurationDone
  → stopped
  → threads → stackTrace → scopes → variables → evaluate
  → output（缓冲）
  → continue | next | stepIn | stepOut | pause
  → terminated/exited → disconnect
```

### 1.1 Tier-0 — 一等公民（仅这些）

#### Requests（控制）

| DAP | 压缩 | MCP | Agent SDK |
|-----|------|-----|-----------|
| `initialize` | 否 | session 内部 | `start()` |
| `launch` | 否 | `debug_launch` | `launch` |
| `configurationDone` | 否 | launch 流程内 | 内部 |
| `setBreakpoints` | 否 | `set_breakpoints` | `set_breakpoints` |
| `continue` | 否 | `continue` | `continue_` |
| `next` | 否 | `step_over` | `step_over` |
| `stepIn` | 否 | `step_into` | `step_into` |
| `stepOut` | 否 | `step_out` | `step_out` |
| `pause` | 否 | `pause` | `pause` |
| `disconnect` | 否 | `disconnect` | `disconnect` |
| `terminate` | 否 | `terminate` | `terminate` |

#### Requests（观测）

| DAP | 压缩 | MCP | Agent SDK |
|-----|------|-----|-----------|
| `threads` | 否 | `get_threads` | `get_threads` |
| `stackTrace` | **是** | `get_stack` | `get_stack` |
| `scopes` | **是**（新建） | `get_scopes` | `get_scopes` |
| `variables` | **是** | `get_variables` | `get_variables` |
| `evaluate` | **是** | `evaluate` | `evaluate` |

#### Events

| DAP | 压缩 | Agent 获取 |
|-----|------|------------|
| `initialized` | 否 | 内部 |
| `stopped` | 否 | `wait_stopped` / 步进返回 |
| `continued` | 否 | 可选 |
| `output` | **是** | `get_output` |
| `terminated` / `exited` | 否 | 终止信息 |
| `thread` / `breakpoint` | 否 | 透传缓冲可选 |

### 1.2 明确降级到 Tier-1（透传 / `send_raw` only）

| DAP | 原因 |
|-----|------|
| `attach` | 首版只保证 launch 闭环 |
| `source` | 源码可读磁盘；非观测最小集 |
| `exceptionInfo` | 可用 stack+variables+output 覆盖多数 bug；后续加 |
| `setExceptionBreakpoints` | 随 exceptionInfo 延后 |
| Spec 其余全部 | 见原 DAP 全表 — Proxy 透传 |

### 1.3 压缩白名单

| 拦截器 | 匹配 | 状态 |
|--------|------|------|
| `CappingInterceptor` | output / stackTrace / variables | 已有 |
| `OutputCompressor` | event `output` | 已有 |
| `StackTraceCompressor` | `stackTrace` | 已有 |
| `VariablesCompressor` | `variables` | 已有 |
| `EvaluateCompressor` | `evaluate` | 已有 |
| `ScopesCompressor` | `scopes` | **新建** |
| ~~ExceptionInfoCompressor~~ | — | **不做（本轮）** |

---

## 2. lspz 文件级移植地图

| # | dapz 目标 | lspz 参考 | DoD | 状态 |
|---|-----------|-----------|-----|------|
| T1 | `transport/framing.rs` | framing.rs | 单测半包/cancel-safe；stdio 接入 | [x] |
| T2 | `codec/toon.rs` | toon.rs（**改成通用 Value→TOON**） | `value_to_toon` 单测 | [x] |
| T3 | `OutputFormat::Toon` | config | FromStr | [x] |
| T4 | `adapters.rs` | languages.rs 精简 | debugpy only | [x] |
| T5 | `mcp/session.rs` | mcp/session.rs | mock handshake+stopped | [x] |
| T6 | `mcp/pool.rs` | mcp/pool.rs | key 复用测 | [x] |
| T7 | `mcp/server.rs` | mcp/server.rs | tools=§3 | [x] |
| T8 | `agent_sdk/*` | agent_sdk/* | API=§3 | [x] |
| T9 | `interceptors/scopes.rs` | 自研 | 单测 | [x] |
| T10 | `main.rs` 子命令 | main.rs 骨架 | `proxy`/`mcp` | [x] |
| T11 | deps 对齐 | Cargo.toml | `--all-features` 编过 | [x] |
| T12 | harness | 自研 | `just harness` | [x] |

**不移植（0.1）**：daemon、workspace roots、init、uri、metrics、config_watcher、exception 相关。

---

## 3. MCP / Agent SDK（收紧冻结）

| MCP tool | Agent SDK | DAP |
|----------|-----------|-----|
| `debug_launch` | `launch` | launch + 内部握手/configurationDone；可选内嵌 breakpoints |
| `set_breakpoints` | `set_breakpoints` | setBreakpoints |
| `continue` | `continue_` | continue → wait stopped/terminated |
| `step_over` | `step_over` | next |
| `step_into` | `step_into` | stepIn |
| `step_out` | `step_out` | stepOut |
| `pause` | `pause` | pause |
| `get_threads` | `get_threads` | threads |
| `get_stack` | `get_stack` | stackTrace（压缩+TOON） |
| `get_scopes` | `get_scopes` | scopes（压缩+TOON） |
| `get_variables` | `get_variables` | variables（压缩+TOON） |
| `evaluate` | `evaluate` | evaluate（压缩+TOON） |
| `get_output` | `drain_output` | 缓冲 output |
| `wait_stopped` | `wait_stopped` | wait event stopped |
| `disconnect` | `disconnect` | disconnect |
| `terminate` | `terminate` | terminate |
| `send_raw` | `send_raw` | 任意（Tier-1 逃生舱） |

**已砍（勿实现专用 API）**：`debug_attach`、`get_source`、`get_exception`、`set_exception_breakpoints`。

---

## 4. 任务板

### Phase A — 地基

| ID | 任务 | 验收 | 状态 |
|----|------|------|------|
| A0 | deps：rmcp 2、thiserror 2 等靠拢 | `cargo build --all-features`（可 stub mcp） | [x] |
| A1 | framing + stdio/tcp 接入 | `cargo test framing` | [x] |
| A2 | 通用 TOON + OutputFormat::Toon | `cargo test toon` | [x] |
| A3 | ScopesCompressor only | `cargo test scopes` | [x] |
| A4 | CLI `proxy` 子命令 | `--help` 含 proxy | [x] |
| A5 | adapters.rs debugpy | lookup 单测 | [x] |

### Phase B — Session

| ID | 任务 | 验收 | 状态 |
|----|------|------|------|
| B1 | DapSession + event 缓冲 | mock 单测 | [x] |
| B2 | Tier-0 方法封装 | 每方法 1 测 | [x] |
| B3 | DapPool | 复用测 | [x] |

### Phase C — MCP + SDK

| ID | 任务 | 验收 | 状态 |
|----|------|------|------|
| C1–C2 | MCP server + tools §3 | list_tools 名字集合 | [x] |
| C3 | `dapz mcp` | feature gate | [x] |
| C4–C5 | AgentHandle (+ 简 pool) | `--features agent-sdk` test | [x] |

### Phase D — 门禁

| ID | 任务 | 验收 | 状态 |
|----|------|------|------|
| D1–D2 | check-env + run-harness + just | 退出码契约 | [x] |
| D3 | `tests/agent_debug_loop.rs` | launch→bp→stack→vars→eval→step→disconnect | [x] |
| D4 | Tier-1 透传回归 | body 相等抽测 | [x] |
| D5 | `just qa` | 全绿 | [x] |
| D6 | README + CHANGELOG 0.1.0 | 与 §3 一致 | [x] |

### D3 E2E fixture

```python
def bug():
    xs = [1, 2, 0]
    return xs[0] + xs[1]  # breakpoint here

def main():
    print("start")
    bug()
    print("end")

main()
```

断言：launch+bp → stopped → stack 含 bug → scopes/vars 见 xs → evaluate → output 含 start → step/continue → disconnect。
（不做 ZeroDivision / exceptionInfo 路径。）

---

## 5. Harness 契约

- `just harness-env`：rustc、python3、`import debugpy`、`cargo build --features mcp,agent-sdk`
- `just harness`：env → fmt-check → clippy → test → ignored e2e
- CI 无 debugpy：e2e SKIP；本地准发布：e2e 必跑
- clean-PATH：`env -i HOME=… PATH=/usr/bin:/bin` 仍能发现 `~/.local/bin` / uv tools / cargo bin 下的 adapter

---

## 6. Go/No-Go

- [x] §3 工具表全部实现（无已砍项）— MCP 17 tools + AgentHandle
- [x] Tier-1 透传回归测（D4）
- [x] `just harness` 本地绿（debugpy via `~/.local/bin` / uv tools 发现）
- [x] `just qa` 绿（`--all-features`）
- [x] README + CHANGELOG **0.1.0**

### 延后

- [x] dapz 包管理器默认路径自动发现完备化 — [`docs/tips/00-tool-path-discovery.md`](docs/tips/00-tool-path-discovery.md)
- [ ] lspz 同步扩展 `languages.rs`（同 tip）

---

## 7. Agent 开工指令

```text
1. 严格 Phase A→D；只读 ../lspz；不抽 crate；不做 daemon。
2. 不实现 attach/source/exceptionInfo 专用 API。
3. 完成 ID 后勾选本文件并写 §8。
4. v0.2+：走 llman SDD（propose → apply → verify → archive）；先 bootstrap 基线 specs。
```

---

## 8. 变更日志

| 日期 | 摘要 |
|------|------|
| 2026-07-20 | 初版 share 决策 |
| 2026-07-20 | DAP Tier-0/1 + lspz 地图 |
| 2026-07-20 | **收紧 Tier-0**：砍 attach/source/exception；开始 Phase A |
| 2026-07-20 | **Phase A 完成**：deps/framing/TOON/scopes/CLI proxy/adapters；`cargo test` 绿 |
| 2026-07-20 | **Phase B+C 完成**：DapSession/Pool/MCP 17 tools/AgentHandle；harness 脚本落地；e2e 待 debugpy |
| 2026-07-20 | `just qa`/clippy all-features 绿；`check-env` 缺 debugpy（预期） |
| 2026-07-20 | **准发布 0.1.0**：发现 `~/.local/bin`+uv tools；修 debugpy late-initialized launch；`just harness` PASS；D4 透传测；tip 两边备案 |
| 2026-07-20 | **SDD 引入**：`llmanspec/` + `.agents/skills`；`resolve_tool` 完备化；§9 v0.2；**P0 bootstrap specs archived** |

---

## 9. v0.2+ 草稿（SDD 驱动，跟进 lspz）

> 原则：不抽共享 crate；文件级 copy-port；**每个能力一条 SDD change**。
> 前置：bootstrap 基线 `llmanspec/specs/*`（反映 0.1 现状）后再改行为。

| 优先级 | 候选 change id | 对照 lspz | 说明 |
|--------|----------------|-----------|------|
| P0 | `docs-bootstrap-baseline-specs` | `llmanspec/specs/*` | ✅ 已 archive（2026-07-20）；9 条基线 capability |
| P1 | `add-agent-sdk-pool` | `agent_sdk/pool.rs` | 独立 AgentPool（与 MCP pool 对称） |
| P2 | `add-just-verify-doc-check` | `just verify` / `doc-check` | 工程门禁对齐 |
| P3 | `add-metrics` | `metrics.rs` | 压缩比/延迟观测 |
| P4 | `add-tier1-exception-apis` | MCP exception* | Tier-1 升一等公民（可选） |
| P5 | `add-daemon` | `daemon/*` | 长驻 session（明确延后，需单独决策） |

**不做（除非新决策）**：workspace roots / uri / init / config_watcher（DAP 场景收益低于 LSP）。
