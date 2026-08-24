# language: en
# capability: mcp
# purpose: MCP 服务器：Tier-0 调试工具与 DapPool 会话复用。
# scope: src/mcp/, tests/

Feature: mcp

  @req:r10 @human
  Scenario: MCP 双模式
    - 系统 MUST 在 feature mcp 下实现 MCP 服务器（daemon 默认或 --no-daemon 进程内）并暴露 Tier-0 调试工具。

  @req:r18 @human
  Scenario: 工具集合
    - 系统 MUST 注册 Tier-0 工具集以及 debug_attach、get_source、get_exception、set_exception_breakpoints、send_raw。

  @req:r26 @human
  Scenario: 会话池
    - 系统 MUST 通过 DapPool 按 key 复用 DapSession。

  @req:r33 @human
  Scenario: 默认 TOON
    - MCP 观测类工具输出 MUST 默认使用 TOON（或配置的紧凑格式）。

  @req:r37 @human
  Scenario: Tier-1 异常工具
    - 系统 MUST 注册 debug_attach、get_source、get_exception、set_exception_breakpoints，并保留 send_raw。

  @req:r52 @human
  Scenario: MCP 默认 daemon
    - 系统 MUST 默认使 dapz mcp 经 DaemonMcpServer 连接或自动启动 dapz daemon；MUST 提供 --no-daemon 回退进程内 McpServer。

  @req:r64 @human
  Scenario: DAP 规范优先
    - 系统 MUST 使 DapSession 的 initialize/launch 默认仅使用 DAP 规范通用字段；适配器扩展（如 debugpy console）MUST 仅在已识别的 AdapterKind 下启用，并在文档中标明兼容范围。

  @req:r65 @human
  Scenario: threadId 解析
    - 当调用方未提供 threadId 时，系统 MUST 通过 threads 请求解析线程 id，MUST NOT 盲默认 threadId=1。

  @req:r10 @human
  Scenario: mcp-start
    - MUST hold: Given 启用 features mcp; When 运行 dapz mcp; Then 进程以 MCP stdio 服务运行.
    Given 启用 features mcp
    When 运行 dapz mcp
    Then 进程以 MCP stdio 服务运行

  @req:r18 @human
  Scenario: tool-count-21
    - MUST hold: Given list_tools; When 计数; Then 恰好 21 个工具.
    Given list_tools
    When 计数
    Then 恰好 21 个工具

  @req:r26 @human
  Scenario: pool-reuse
    - MUST hold: Given 相同 pool key 两次 debug_launch; When 获取会话; Then 复用同一后端会话策略成立.
    Given 相同 pool key 两次 debug_launch
    When 获取会话
    Then 复用同一后端会话策略成立

  @req:r33 @human
  Scenario: stack-toon
    - MUST hold: Given get_stack 成功; When 返回内容; Then 为 TOON 文本而非冗长 JSON.
    Given get_stack 成功
    When 返回内容
    Then 为 TOON 文本而非冗长 JSON

  @req:r37 @human
  Scenario: has-exception-tools
    - MUST hold: Given MCP list_tools; When 扫描名称; Then 含 get_exception 与 debug_attach.
    Given MCP list_tools
    When 扫描名称
    Then 含 get_exception 与 debug_attach

  @req:r52 @human
  Scenario: mcp-daemon-default
    - MUST hold: Given features mcp; When 运行 dapz mcp 无 --no-daemon; Then 工具调用经 daemon socket.
    Given features mcp
    When 运行 dapz mcp 无 --no-daemon
    Then 工具调用经 daemon socket

  @req:r52 @human
  Scenario: mcp-no-daemon
    - MUST hold: Given 传 --no-daemon; When 运行 dapz mcp; Then 使用进程内 DapPool.
    Given 传 --no-daemon
    When 运行 dapz mcp
    Then 使用进程内 DapPool

  @req:r64 @human
  Scenario: generic-no-console
    - MUST hold: Given backend 不含 debugpy; When launch_program; Then launch 参数无 console 字段.
    Given backend 不含 debugpy
    When launch_program
    Then launch 参数无 console 字段

  @req:r64 @human
  Scenario: debugpy-console
    - MUST hold: Given backend 含 debugpy; When launch_program; Then launch 含 console=internalConsole.
    Given backend 含 debugpy
    When launch_program
    Then launch 含 console=internalConsole

  @req:r65 @human
  Scenario: resolve-tid
    - MUST hold: Given threads 返回 id=42 且未传 threadId; When get_stack; Then 请求使用 threadId=42.
    Given threads 返回 id=42 且未传 threadId
    When get_stack
    Then 请求使用 threadId=42
