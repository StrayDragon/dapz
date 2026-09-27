# language: zh-CN
# capability: daemon
# purpose: 长驻 DAP daemon：按 project cwd 的 Unix socket 会话池（非 LSP workspace/docs）。
# scope: src/, tests/

功能: daemon

  @req:r48
  规则: DAP daemon
    系统 MUST 在 feature mcp 下提供长驻 daemon，通过 Unix socket NDJSON JSON-RPC 管理 DAP adapter 会话。

    @skip
    场景: cli-daemon
      假如 features mcp
      当 运行 dapz daemon
      那么 进程监听 Unix socket

  @req:r49
  规则: socket 按 cwd
    daemon socket 路径 MUST 由规范化 project cwd 派生（~/.cache/dapz/），不得使用 LSP workspace roots 模型。

    @skip
    场景: stable-socket
      假如 同一 cwd 两种路径拼写
      当 计算 socket_path
      那么 得到相同路径

  @req:r50
  规则: dap RPC 扩展
    daemon MUST 支持 dap/spawn、dap/request、dap/wait_event、dap/invoke、dap/remove、daemon/status、daemon/shutdown。

    @skip
    场景: rpc-has-invoke
      假如 daemon 协议
      当 检查方法表
      那么 含 dap/invoke 与 dap/remove

  @req:r51
  规则: 会话 key
    daemon 会话 MUST 使用 backend[+cwd] pool_key（与 DapPool 一致），不得按 LSP language id 建会话。

    @skip
    场景: pool-key
      假如 spawn backend+cwd
      当 创建会话
      那么 key 等于 pool_key(backend,cwd)

  @req:r53
  规则: dap invoke remove
    daemon MUST 支持 dap/invoke（高阶 DapSession 操作）与 dap/remove；dap/spawn MUST 支持 replace 以重建会话。

    @skip
    场景: invoke-launch
      假如 已 spawn 的 session
      当 dap/invoke launch_program
      那么 返回 stopped 体

  @req:r54
  规则: idle 回收
    daemon MUST 周期性 reap 空闲 DAP 会话；当 pool 为空且无客户端连接持续空闲时 MUST 自行 shutdown 并清理 socket。

    @skip
    场景: reap-idle
      假如 会话超过 idle TTL 无 I/O
      当 reaper 周期到达
      那么 会话从 pool 移除

  @req:r55
  规则: owns_daemon
    DaemonClient 在自动启动 daemon 时 MUST 标记 owns_daemon；Drop 时 MUST best-effort 请求 daemon/shutdown；Agent SDK 经 daemon 路径 MUST 复用同一 owns_daemon 语义。

    @skip
    场景: sdk-drop-shutdown
      假如 AgentHandle 经 connect_or_start 持有 owns_daemon
      当 句柄 Drop
      那么 daemon 收到 shutdown 或进程退出
