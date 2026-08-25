# language: en
# capability: transport
# purpose: 传输层：Content-Length 分帧与 stdio/tcp。
# scope: src/transport/, tests/

Feature: transport

  @req:r14 @human
  Scenario: Transport trait
    - 系统 MUST 定义 Transport trait 包含异步 receive 与 send。

  @req:r22 @human
  Scenario: Content-Length 分帧
    - 所有传输层 MUST 处理 Content-Length 分帧，且读取路径 cancel-safe。

  @req:r30 @human
  Scenario: 多协议支持
    - 系统 MUST 支持 StdioTransport 与 TcpTransport；dapz proxy --transport MUST 能选择 stdio（默认）或 tcp://host:port，且 MUST NOT 声称支持 WebSocket。

  @req:r36 @human
  Scenario: Content-Length 上限
    - 分帧读取 MUST 拒绝超过约定上限的 body，避免无界分配。

  @req:r62 @human
  Scenario: 无 WebSocket 传输
    - 系统 MUST NOT 提供 WsTransport 或 transport-websocket feature；dapz proxy --transport MUST 仅接受 stdio 与 tcp://。

  @req:r14 @human
  Scenario: trait-surface
    - MUST hold: Given Transport 实现; When 调用 receive/send; Then 完成一帧读写.
    Given Transport 实现
    When 调用 receive/send
    Then 完成一帧读写

  @req:r22 @human
  Scenario: cancel-safe-frame
    - MUST hold: Given 半包到达后任务取消再恢复; When 继续 read_frame; Then 不丢失已读字节.
    Given 半包到达后任务取消再恢复
    When 继续 read_frame
    Then 不丢失已读字节

  @req:r30 @human
  Scenario: cli-transport-tcp
    - MUST hold: Given --transport tcp://127.0.0.1:1234; When 启动 proxy; Then 使用 TcpTransport.
    Given --transport tcp://127.0.0.1:1234
    When 启动 proxy
    Then 使用 TcpTransport

  @req:r36 @human
  Scenario: reject-oversized
    - MUST hold: Given Content-Length 过大; When read_frame; Then 返回错误且不分配完整 body.
    Given Content-Length 过大
    When read_frame
    Then 返回错误且不分配完整 body

  @req:r62 @human
  Scenario: reject-ws
    - MUST hold: Given --transport ws://example; When 启动 proxy; Then 报错且提示仅 stdio/tcp.
    Given --transport ws://example
    When 启动 proxy
    Then 报错且提示仅 stdio/tcp
