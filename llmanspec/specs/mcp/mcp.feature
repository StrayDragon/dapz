# language: zh-CN
# capability: mcp
# purpose: MCP 服务器：Tier-0 调试工具与 DapPool 会话复用。
# scope: src/mcp/, tests/

功能: mcp

  @req:r10
  规则: MCP 双模式
    系统 MUST 在 feature mcp 下实现 MCP 服务器（daemon 默认或 --no-daemon 进程内）并暴露 Tier-0 调试工具。

    场景: mcp-start
      假如 启用 features mcp
      当 运行 dapz mcp
      那么 进程以 MCP stdio 服务运行

  @req:r18
  规则: 工具集合
    系统 MUST 注册 Tier-0 工具集以及 debug_attach、get_source、get_exception、set_exception_breakpoints、send_raw。

    场景: tool-count-21
      假如 list_tools
      当 计数
      那么 恰好 21 个工具

    场景: has-exception-tools
      假如 MCP list_tools
      当 扫描名称
      那么 含 get_exception 与 debug_attach

  @req:r26
  规则: 会话池
    系统 MUST 通过 DapPool 按 key 复用 DapSession。

    场景: pool-reuse
      假如 相同 pool key 两次 debug_launch
      当 获取会话
      那么 复用同一后端会话策略成立

  @req:r33
  规则: 默认 TOON
    MCP 观测类工具输出 MUST 默认使用 TOON（或配置的紧凑格式）。

    场景: stack-toon
      假如 get_stack 成功
      当 返回内容
      那么 为 TOON 文本而非冗长 JSON

  @req:r52
  规则: MCP 默认 daemon
    系统 MUST 默认使 dapz mcp 经 DaemonMcpServer 连接或自动启动 dapz daemon；MUST 提供 --no-daemon 回退进程内 McpServer。

    场景: mcp-daemon-default
      假如 features mcp
      当 运行 dapz mcp 无 --no-daemon
      那么 工具调用经 daemon socket

    场景: mcp-no-daemon
      假如 传 --no-daemon
      当 运行 dapz mcp
      那么 使用进程内 DapPool

  @req:r64
  规则: DAP 规范优先
    系统 MUST 使 DapSession 的 initialize/launch 默认仅使用 DAP 规范通用字段；适配器扩展（如 debugpy console）MUST 仅在已识别的 AdapterKind 下启用，并在文档中标明兼容范围。

    场景: generic-no-console
      假如 backend 不含 debugpy
      当 launch_program
      那么 launch 参数无 console 字段

    场景: debugpy-console
      假如 backend 含 debugpy
      当 launch_program
      那么 launch 含 console=internalConsole

  @req:r65
  规则: threadId 解析
    调用方未提供 threadId 时，系统 MUST 通过 threads 请求解析线程 id，MUST NOT 盲默认 threadId=1。

    场景: resolve-tid
      假如 threads 返回 id=42 且未传 threadId
      当 get_stack
      那么 请求使用 threadId=42
