# language: zh-CN
# capability: product-requirements
# purpose: 产品需求：AI 友好 DAP 压缩代理、三模态、Tier-0 收紧范围与透传兼容。
# scope: src/, tests/

功能: product-requirements

  @req:r11
  规则: AI 友好 DAP 代理
    系统 MUST 构建对 AI Coding Agent 友好的 DAP 压缩代理层。

    场景: agent-debug-loop
      假如 Agent 使用 dapz MCP 或 Agent SDK
      当 执行 Tier-0 调试循环
      那么 可观测 stack/vars/output 并完成 disconnect

  @req:r19
  规则: Token 压缩
    系统 MUST 对 Server→Client 方向的 DAP 消息进行 Token 敏感压缩；压缩失败时 MUST 透传原始消息。

    场景: compress-or-passthrough
      假如 拦截器内部错误
      当 处理 ServerToClient 消息
      那么 转发原始消息且会话不崩溃

  @req:r27
  规则: 三模态加 daemon
    系统 MUST 支持 Library、Proxy CLI、MCP（默认经 daemon），并 MAY 直接运行 dapz daemon；Agent SDK MUST 支持 opt-in via_daemon 复用同一 daemon；无 LSP 文档同步。

    场景: sdk-via-daemon
      假如 Agent SDK via_daemon true
      当 调用 launch
      那么 会话落在 daemon pool

  @req:r34
  规则: Tier-0 加 Tier-1 异常
    系统 MUST 以 launch 闭环为默认路径，并 MUST 提供 attach/source/exceptionInfo/setExceptionBreakpoints 专用 MCP/SDK API（仍可用 send_raw）。

    场景: exception-tools-present
      假如 list_tools
      当 检查工具名
      那么 包含 get_exception 与 set_exception_breakpoints

  @req:r38
  规则: DAP 兼容透传
    系统 MUST 对非拦截白名单的 DAP 消息保持透传兼容（body 语义不变）；拦截与 Session 封装 MUST 以 DAP 规范字段为默认，适配器特例 MUST 显式分档。

    场景: compat-doc
      假如 查阅 dap-compatibility 文档
      当 检查矩阵
      那么 含 debugpy 与 lldb-dap 分档说明

  @req:r67
  规则: Claude Code init
    系统 MUST 提供 dapz init（含 --global）以注册 MCP server、写入 DAPZ.md，并在 CLAUDE.md 注入 @DAPZ.md；MUST 支持 --show / --uninstall / --dry-run / --force / --no-patch；当二进制未启用 mcp feature 时 MUST 失败并提示重建。

    场景: init-global-mcp
      假如 DAPZ_CLAUDE_DIR 指向临时目录且 mcp feature 开启
      当 dapz init --global
      那么 创建 mcpServers.dapz 与 DAPZ.md 且 CLAUDE.md 含 @DAPZ.md

    场景: init-show
      假如 已注册 MCP
      当 dapz init --show
      那么 打印 Binary/MCP/DAPZ.md/CLAUDE.md 状态

    场景: init-no-mcp-feature
      假如 二进制无 mcp feature
      当 dapz init
      那么 以非零退出并提示 --features mcp
