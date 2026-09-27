# language: zh-CN
# capability: transport
# purpose: 传输层：Content-Length 分帧与 stdio/tcp。
# scope: src/transport/, tests/

功能: transport

  @req:r14
  规则: Transport trait
    系统 MUST 定义 Transport trait 包含异步 receive 与 send。

    场景: trait-surface
      假如 Transport 实现
      当 调用 receive/send
      那么 完成一帧读写

  @req:r22
  规则: Content-Length 分帧
    所有传输层 MUST 处理 Content-Length 分帧，且读取路径 cancel-safe。

    场景: cancel-safe-frame
      假如 半包到达后任务取消再恢复
      当 继续 read_frame
      那么 不丢失已读字节

  @req:r30
  规则: 多协议支持
    系统 MUST 支持 StdioTransport 与 TcpTransport；dapz proxy --transport MUST 能选择 stdio（默认）或 tcp://host:port，且 MUST NOT 声称支持 WebSocket。

    场景: cli-transport-tcp
      假如 --transport tcp://127.0.0.1:1234
      当 启动 proxy
      那么 使用 TcpTransport

  @req:r36
  规则: Content-Length 上限
    分帧读取 MUST 拒绝超过约定上限的 body，避免无界分配。

    场景: reject-oversized
      假如 Content-Length 过大
      当 read_frame
      那么 返回错误且不分配完整 body

  @req:r62
  规则: 无 WebSocket 传输
    系统 MUST NOT 提供 WsTransport 或 transport-websocket feature；dapz proxy --transport MUST 仅接受 stdio 与 tcp://。

    场景: reject-ws
      假如 --transport ws://example
      当 启动 proxy
      那么 报错且提示仅 stdio/tcp
