# language: en
# capability: agent-sdk
# purpose: Agent SDK：AgentHandle + session-key AgentPool（DAP；非 LSP language pool）。
# scope: src/agent_sdk/, tests/

Feature: agent-sdk

  @req:r7 @human
  Scenario: AgentHandle
    - 系统 MUST 在 feature agent-sdk 下提供 AgentHandle：默认进程内管理 DAP 会话，或在 via_daemon 模式下经 daemon 管理会话。

  @req:r15 @human
  Scenario: Tier-0 方法
    - AgentHandle MUST 提供 launch、set_breakpoints、continue_、step_*、pause、get_threads、get_stack、get_scopes、get_variables、evaluate、drain_output、wait_stopped、disconnect、terminate、send_raw。

  @req:r23 @human
  Scenario: TOON 返回
    - 观测类方法 MUST 返回 TOON 字符串（或等价紧凑表示）。

  @req:r31 @human
  Scenario: Tier-1 异常方法
    - AgentHandle MUST 提供 attach、get_source、get_exception、set_exception_breakpoints（可用 send_raw 作为补充）。

  @req:r39 @human
  Scenario: AgentPool
    - 系统 MUST 在 feature agent-sdk 下提供 AgentPool，按 session key 管理多个 AgentHandle（DAP 会话；非 LSP language key）。

  @req:r40 @human
  Scenario: 懒加载会话
    - AgentPool MUST 在首次使用某 session key 时懒启动对应 backend，不得在 start_all 时强制全部 spawn。

  @req:r41 @human
  Scenario: Tier-0 委托
    - AgentPool MUST 将 launch/get_stack/get_variables 等 Tier-0 调用委托给对应 AgentHandle，且 MUST NOT 暴露 LSP 文档同步 API。

  @req:r57 @human
  Scenario: SDK 经 daemon
    - 系统 MUST 使 feature agent-sdk 下的 AgentHandle/AgentPool 能通过 via_daemon(true) 经 DaemonClient 连接或自动启动 dapz daemon 以复用 backend+cwd 会话；默认 MUST 保持进程内会话；连不上 daemon 时 MUST 返回错误且 MUST NOT 静默回退进程内。

  @req:r7 @human
  Scenario: agent-start
    - MUST hold: Given AgentHandle::builder 配置 backend; When 调用 start; Then 返回可用句柄.
    Given AgentHandle::builder 配置 backend
    When 调用 start
    Then 返回可用句柄

  @req:r15 @human
  Scenario: debug-loop
    - MUST hold: Given fixture 脚本与断点; When launch→get_stack→evaluate→disconnect; Then 各步骤成功.
    Given fixture 脚本与断点
    When launch→get_stack→evaluate→disconnect
    Then 各步骤成功

  @req:r23 @human
  Scenario: stack-toon
    - MUST hold: Given get_stack; When 成功返回; Then 内容为 TOON.
    Given get_stack
    When 成功返回
    Then 内容为 TOON

  @req:r31 @human
  Scenario: has-attach-api
    - MUST hold: Given 查阅 AgentHandle API; When 检查公开方法; Then 存在 attach 与 get_exception.
    Given 查阅 AgentHandle API
    When 检查公开方法
    Then 存在 attach 与 get_exception

  @req:r39 @human
  Scenario: pool-register
    - MUST hold: Given AgentPoolBuilder; When register 两个 session key 后 start_all; Then 返回含两个已注册 backend 的 AgentPool.
    Given AgentPoolBuilder
    When register 两个 session key 后 start_all
    Then 返回含两个已注册 backend 的 AgentPool

  @req:r40 @human
  Scenario: lazy-spawn
    - MUST hold: Given 已 register 但未调用的 key; When 首次 get_stack 或 launch; Then 才创建 AgentHandle.
    Given 已 register 但未调用的 key
    When 首次 get_stack 或 launch
    Then 才创建 AgentHandle

  @req:r41 @human
  Scenario: no-lsp-docs
    - MUST hold: Given 查阅 AgentPool 公开 API; When 检查方法名; Then 无 notify_change/get_diagnostics/workspace_root 专用 LSP API.
    Given 查阅 AgentPool 公开 API
    When 检查方法名
    Then 无 notify_change/get_diagnostics/workspace_root 专用 LSP API

  @req:r57 @human
  Scenario: sdk-via-daemon
    - MUST hold: Given AgentBuilder via_daemon true; When 调用 start; Then 经 DaemonClient 连接或启动 daemon.
    Given AgentBuilder via_daemon true
    When 调用 start
    Then 经 DaemonClient 连接或启动 daemon

  @req:r57 @human
  Scenario: sdk-in-process-default
    - MUST hold: Given 未设 via_daemon; When 调用 start; Then 本地 spawn DapSession 且不要求 daemon socket.
    Given 未设 via_daemon
    When 调用 start
    Then 本地 spawn DapSession 且不要求 daemon socket
