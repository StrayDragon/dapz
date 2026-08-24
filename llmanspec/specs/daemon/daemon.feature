# language: en
# capability: daemon
# purpose: 长驻 DAP daemon：按 project cwd 的 Unix socket 会话池（非 LSP workspace/docs）。
# scope: src/, tests/

Feature: daemon

  @req:r48 @human
  Scenario: DAP daemon
    - 系统 MUST 在 feature mcp 下提供长驻 daemon，通过 Unix socket NDJSON JSON-RPC 管理 DAP adapter 会话。

  @req:r49 @human
  Scenario: socket 按 cwd
    - daemon socket 路径 MUST 由规范化 project cwd 派生（~/.cache/dapz/），不得使用 LSP workspace roots 模型。

  @req:r50 @human
  Scenario: dap RPC 扩展
    - daemon MUST 支持 dap/spawn、dap/request、dap/wait_event、dap/invoke、dap/remove、daemon/status、daemon/shutdown。

  @req:r51 @human
  Scenario: 会话 key
    - daemon 会话 MUST 使用 backend[+cwd] pool_key（与 DapPool 一致），不得按 LSP language id 建会话。

  @req:r53 @human
  Scenario: dap invoke remove
    - daemon MUST 支持 dap/invoke（高阶 DapSession 操作）与 dap/remove；dap/spawn MUST 支持 replace 以重建会话。

  @req:r54 @human
  Scenario: idle 回收
    - daemon MUST 周期性 reap 空闲 DAP 会话；当 pool 为空且无客户端连接持续空闲时 MUST 自行 shutdown 并清理 socket。

  @req:r55 @human
  Scenario: owns_daemon
    - DaemonClient 在自动启动 daemon 时 MUST 标记 owns_daemon；Drop 时 MUST best-effort 请求 daemon/shutdown；Agent SDK 经 daemon 路径 MUST 复用同一 owns_daemon 语义。

  @req:r48 @human
  Scenario: cli-daemon
    - MUST hold: Given features mcp; When 运行 dapz daemon; Then 进程监听 Unix socket.
    Given features mcp
    When 运行 dapz daemon
    Then 进程监听 Unix socket

  @req:r49 @human
  Scenario: stable-socket
    - MUST hold: Given 同一 cwd 两种路径拼写; When 计算 socket_path; Then 得到相同路径.
    Given 同一 cwd 两种路径拼写
    When 计算 socket_path
    Then 得到相同路径

  @req:r50 @human
  Scenario: rpc-has-invoke
    - MUST hold: Given daemon 协议; When 检查方法表; Then 含 dap/invoke 与 dap/remove.
    Given daemon 协议
    When 检查方法表
    Then 含 dap/invoke 与 dap/remove

  @req:r51 @human
  Scenario: pool-key
    - MUST hold: Given spawn backend+cwd; When 创建会话; Then key 等于 pool_key(backend,cwd).
    Given spawn backend+cwd
    When 创建会话
    Then key 等于 pool_key(backend,cwd)

  @req:r53 @human
  Scenario: invoke-launch
    - MUST hold: Given 已 spawn 的 session; When dap/invoke launch_program; Then 返回 stopped 体.
    Given 已 spawn 的 session
    When dap/invoke launch_program
    Then 返回 stopped 体

  @req:r54 @human
  Scenario: reap-idle
    - MUST hold: Given 会话超过 idle TTL 无 I/O; When reaper 周期到达; Then 会话从 pool 移除.
    Given 会话超过 idle TTL 无 I/O
    When reaper 周期到达
    Then 会话从 pool 移除

  @req:r55 @human
  Scenario: sdk-drop-shutdown
    - MUST hold: Given AgentHandle 经 connect_or_start 持有 owns_daemon; When 句柄 Drop; Then daemon 收到 shutdown 或进程退出.
    Given AgentHandle 经 connect_or_start 持有 owns_daemon
    When 句柄 Drop
    Then daemon 收到 shutdown 或进程退出
