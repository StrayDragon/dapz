# language: en
# capability: product-requirements
# purpose: 产品需求：AI 友好 DAP 压缩代理、三模态、Tier-0 收紧范围与透传兼容。
# scope: src/, tests/

Feature: product-requirements

  @req:r11 @human
  Scenario: AI 友好 DAP 代理
    - 系统 MUST 构建对 AI Coding Agent 友好的 DAP 压缩代理层。

  @req:r19 @human
  Scenario: Token 压缩
    - 系统 MUST 对 Server→Client 方向的 DAP 消息进行 Token 敏感压缩；压缩失败时 MUST 透传原始消息。

  @req:r27 @human
  Scenario: 三模态加 daemon
    - 系统 MUST 支持 Library、Proxy CLI、MCP（默认经 daemon），并 MAY 直接运行 dapz daemon；Agent SDK MUST 支持 opt-in via_daemon 复用同一 daemon；无 LSP 文档同步。

  @req:r34 @human
  Scenario: Tier-0 加 Tier-1 异常
    - 系统 MUST 以 launch 闭环为默认路径，并 MUST 提供 attach/source/exceptionInfo/setExceptionBreakpoints 专用 MCP/SDK API（仍可用 send_raw）。

  @req:r38 @human
  Scenario: DAP 兼容透传
    - 系统 MUST 对非拦截白名单的 DAP 消息保持透传兼容（body 语义不变）；拦截与 Session 封装 MUST 以 DAP 规范字段为默认，适配器特例 MUST 显式分档。

  @req:r67 @human
  Scenario: Claude Code init
    - 系统 MUST 提供 dapz init（含 --global）以注册 MCP server、写入 DAPZ.md，并在 CLAUDE.md 注入 @DAPZ.md；MUST 支持 --show / --uninstall / --dry-run / --force / --no-patch；当二进制未启用 mcp feature 时 MUST 失败并提示重建。

  @req:r11 @human
  Scenario: agent-debug-loop
    - MUST hold: Given Agent 使用 dapz MCP 或 Agent SDK; When 执行 Tier-0 调试循环; Then 可观测 stack/vars/output 并完成 disconnect.
    Given Agent 使用 dapz MCP 或 Agent SDK
    When 执行 Tier-0 调试循环
    Then 可观测 stack/vars/output 并完成 disconnect

  @req:r19 @human
  Scenario: compress-or-passthrough
    - MUST hold: Given 拦截器内部错误; When 处理 ServerToClient 消息; Then 转发原始消息且会话不崩溃.
    Given 拦截器内部错误
    When 处理 ServerToClient 消息
    Then 转发原始消息且会话不崩溃

  @req:r27 @human
  Scenario: sdk-via-daemon
    - MUST hold: Given Agent SDK via_daemon true; When 调用 launch; Then 会话落在 daemon pool.
    Given Agent SDK via_daemon true
    When 调用 launch
    Then 会话落在 daemon pool

  @req:r34 @human
  Scenario: exception-tools-present
    - MUST hold: Given list_tools; When 检查工具名; Then 包含 get_exception 与 set_exception_breakpoints.
    Given list_tools
    When 检查工具名
    Then 包含 get_exception 与 set_exception_breakpoints

  @req:r38 @human
  Scenario: compat-doc
    - MUST hold: Given 查阅 dap-compatibility 文档; When 检查矩阵; Then 含 debugpy 与 lldb-dap 分档说明.
    Given 查阅 dap-compatibility 文档
    When 检查矩阵
    Then 含 debugpy 与 lldb-dap 分档说明

  @req:r67 @human
  Scenario: init-global-mcp
    - MUST hold: Given DAPZ_CLAUDE_DIR 指向临时目录且 mcp feature 开启; When dapz init --global; Then 创建 mcpServers.dapz 与 DAPZ.md 且 CLAUDE.md 含 @DAPZ.md.
    Given DAPZ_CLAUDE_DIR 指向临时目录且 mcp feature 开启
    When dapz init --global
    Then 创建 mcpServers.dapz 与 DAPZ.md 且 CLAUDE.md 含 @DAPZ.md

  @req:r67 @human
  Scenario: init-show
    - MUST hold: Given 已注册 MCP; When dapz init --show; Then 打印 Binary/MCP/DAPZ.md/CLAUDE.md 状态.
    Given 已注册 MCP
    When dapz init --show
    Then 打印 Binary/MCP/DAPZ.md/CLAUDE.md 状态

  @req:r67 @human
  Scenario: init-no-mcp-feature
    - MUST hold: Given 二进制无 mcp feature; When dapz init; Then 以非零退出并提示 --features mcp.
    Given 二进制无 mcp feature
    When dapz init
    Then 以非零退出并提示 --features mcp
