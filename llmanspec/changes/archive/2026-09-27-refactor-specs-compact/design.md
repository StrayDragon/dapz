# Design: refactor-specs-compact

## 决策策略

0.5.0 native 布局：`@req:<id>` 挂 `规则:` 头，GWT 以嵌套 `场景:`（假如/当/那么，缩进 规则 2 格 /
场景 4 格 / 步骤 6 格）表达。现状每个 req id 写成两条平级裸规则，转换 = 同 req id 成对合并 +
嵌套化：

- **转场景（66）**：规范规则（系统 MUST…）标题与描述逐字保留，其既有 GWT 嵌套为 `场景:`。
  全部 req id 均已带可挂真实代码的 GWT，无需保留裸规则（保留 0 条）。
- **合并（75）**：同 req id 的 `MUST hold:` 单行折叠为嵌套场景；标题变场景标题不变；步骤文本
  逐字保留（去 `MUST hold:` 前缀、Given/When/Then→假如/当/那么、去行尾句号）。
- **移除（1）**：mcp `@req:r37`（Tier-1 异常工具）规范文本为 r18（工具集合）真子集（r18 逐字含
  debug_attach、get_source、get_exception、set_exception_breakpoints、send_raw）；其场景
  `has-exception-tools` 保留为 r18 第二个场景。映射 r37 → r18。

结果：规则 142 → 66（全 enforced）、pending 142 → 0、场景 75、req id 仅消失 r37。

## 分能力映射表

| 能力 | 规则数 | 保留的规范规则（← 嵌入的场景） |
|---|---|---|
| adapters | 13→6 | r1←lookup-rs；r2←user-bin-without-path；r3←uv-tools-layout；r4←mock-home；r56←prefer-lldb-dap；r63←lldb-e2e-run + lldb-e2e-skip |
| agent-sdk | 17→8 | r7←agent-start；r15←debug-loop；r23←stack-toon；r31←has-attach-api；r39←pool-register；r40←lazy-spawn；r41←no-lsp-docs；r57←sdk-via-daemon + sdk-in-process-default |
| codec | 8→4 | r8←roundtrip；r16←toon-format-crate；r24←passthrough-raw；r58←bench-toon-column |
| daemon | 14→7 | r48←cli-daemon；r49←stable-socket；r50←rpc-has-invoke；r51←pool-key；r53←invoke-launch；r54←reap-idle；r55←sdk-drop-shutdown |
| interceptors | 18→9 | r9←cap-frames；r17←fold-output；r25←abbrev-path；r32←truncate-value；r5←truncate-eval；r6←scopes-keep-ref；r47←truncate-details；r59←bench-scopes-exception；r66←contract-const |
| mcp | 18→7 | r10←mcp-start；r18←tool-count-21 + has-exception-tools（吸收 r37）；r26←pool-reuse；r33←stack-toon；r52←mcp-daemon-default + mcp-no-daemon；r64←generic-no-console + debugpy-console；r65←resolve-tid |
| metrics | 6→3 | r44←record-sizes；r45←cli-or-env；r46←ratio |
| product-requirements | 14→6 | r11←agent-debug-loop；r19←compress-or-passthrough；r27←sdk-via-daemon；r34←exception-tools-present；r38←compat-doc；r67←init-global-mcp + init-show + init-no-mcp-feature |
| proxy | 14→6 | r12←proxy-start；r20←server-to-client-only；r28←help-subcommands；r35←passthrough-raw + json-compressed + toon-wrapped；r60←proxy-no-ws-help；r61←proxy-default-toon |
| ssot-rules | 10→5 | r13←managed-block；r21←plan-refs；r29←tip-aligned；r42←qa-prek；r43←verify-script |
| transport | 10→5 | r14←trait-surface；r22←cancel-safe-frame；r30←cli-transport-tcp；r36←reject-oversized；r62←reject-ws |

## 跨能力语义重叠评审（保留不改）

1. codec r24（编码层不改写 body）vs proxy r35（管道层不进拦截链）——validScope 不同。
2. agent-sdk r57 / mcp r52 / product-requirements r27（via_daemon 三处）——SDK / MCP / 产品三模态各一层。
3. mcp r18 vs product-requirements r34——MCP 工具注册 vs MCP+SDK 产品面（仅同能力 r37 被 r18 覆盖而移除）。
4. transport r62（禁 WsTransport）vs proxy r60 场景 proxy-no-ws-help（CLI help 不承诺 ws）。
5. transport r30（stdio/tcp 支持）vs r62（禁 ws）——主旨不同，部分重叠保留。

`project dedupe-req-ids`：无跨能力 id 冲突，不换号。

## 行为不变性

66 条规范描述与标题逐字保留；75 条 GWT 文本逐字保留仅重排版；唯一内容变化 r37 移除且其 MUST
语义已被 r18 逐字覆盖。不改 `src/`、`tests/`、justfile。

## 盘点基线（2026-09-27）

- `llman-sdd --version` = 0.5.0；工作树干净（main @ ac0a395）
- `review`：pending 142（11 能力全 pending）、stale 0、validate ok、warningCount 142
- `archive freeze --dry-run`：18 个 change 共 384K，未达冻结阈值，跳过冻结
- `project dedupe-req-ids --dry-run`：无冲突
