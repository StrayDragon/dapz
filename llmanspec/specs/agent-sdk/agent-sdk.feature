# language: zh-CN
# capability: agent-sdk
# purpose: Agent SDK：AgentHandle + session-key AgentPool（DAP；非 LSP language pool）。
# scope: src/agent_sdk/, tests/

功能: agent-sdk

  @req:r7
  规则: AgentHandle
    系统 MUST 在 feature agent-sdk 下提供 AgentHandle：默认进程内管理 DAP 会话，或在 via_daemon 模式下经 daemon 管理会话。

    @skip
    场景: agent-start
      假如 AgentHandle::builder 配置 backend
      当 调用 start
      那么 返回可用句柄

  @req:r15
  规则: Tier-0 方法
    AgentHandle MUST 提供 launch、set_breakpoints、continue_、step_*、pause、get_threads、get_stack、get_scopes、get_variables、evaluate、drain_output、wait_stopped、disconnect、terminate、send_raw。

    @skip
    场景: debug-loop
      假如 fixture 脚本与断点
      当 launch→get_stack→evaluate→disconnect
      那么 各步骤成功

  @req:r23
  规则: TOON 返回
    观测类方法 MUST 返回 TOON 字符串（或等价紧凑表示）。

    @skip
    场景: stack-toon
      假如 get_stack
      当 成功返回
      那么 内容为 TOON

  @req:r31
  规则: Tier-1 异常方法
    AgentHandle MUST 提供 attach、get_source、get_exception、set_exception_breakpoints（可用 send_raw 作为补充）。

    @skip
    场景: has-attach-api
      假如 查阅 AgentHandle API
      当 检查公开方法
      那么 存在 attach 与 get_exception

  @req:r39
  规则: AgentPool
    系统 MUST 在 feature agent-sdk 下提供 AgentPool，按 session key 管理多个 AgentHandle（DAP 会话；非 LSP language key）。

    @skip
    场景: pool-register
      假如 AgentPoolBuilder
      当 register 两个 session key 后 start_all
      那么 返回含两个已注册 backend 的 AgentPool

  @req:r40
  规则: 懒加载会话
    AgentPool MUST 在首次使用某 session key 时懒启动对应 backend，不得在 start_all 时强制全部 spawn。

    @skip
    场景: lazy-spawn
      假如 已 register 但未调用的 key
      当 首次 get_stack 或 launch
      那么 才创建 AgentHandle

  @req:r41
  规则: Tier-0 委托
    AgentPool MUST 将 launch/get_stack/get_variables 等 Tier-0 调用委托给对应 AgentHandle，且 MUST NOT 暴露 LSP 文档同步 API。

    @skip
    场景: no-lsp-docs
      假如 查阅 AgentPool 公开 API
      当 检查方法名
      那么 无 notify_change/get_diagnostics/workspace_root 专用 LSP API

  @req:r57
  规则: SDK 经 daemon
    系统 MUST 使 feature agent-sdk 下的 AgentHandle/AgentPool 能通过 via_daemon(true) 经 DaemonClient 连接或自动启动 dapz daemon 以复用 backend+cwd 会话；默认 MUST 保持进程内会话；连不上 daemon 时 MUST 返回错误且 MUST NOT 静默回退进程内。

    @skip
    场景: sdk-via-daemon
      假如 AgentBuilder via_daemon true
      当 调用 start
      那么 经 DaemonClient 连接或启动 daemon

    @skip
    场景: sdk-in-process-default
      假如 未设 via_daemon
      当 调用 start
      那么 本地 spawn DapSession 且不要求 daemon socket
