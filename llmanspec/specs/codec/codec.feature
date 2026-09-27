# language: zh-CN
# capability: codec
# purpose: 编解码：JSON-RPC 与 TOON/passthrough 输出。
# scope: src/codec/, tests/

功能: codec

  @req:r8
  规则: JSON-RPC 编解码
    系统 MUST 编解码 JSON-RPC 2.0 DAP 消息。

    场景: roundtrip
      假如 合法 DAP JSON 对象
      当 encode/decode
      那么 字段语义保持

  @req:r16
  规则: TOON 输出
    系统 MUST 提供 value_to_toon：将任意 JSON Value 编码为 TOON 文本，且 MUST 使用 toon-format（encode_default）实现，以保证与官方 TOON 规范一致。

    场景: toon-format-crate
      假如 含嵌套对象与表格数组的 Value
      当 调用 value_to_toon
      那么 得到非空 TOON 字符串

  @req:r24
  规则: Passthrough
    output_format=passthrough 时 Proxy MUST NOT 改写消息体。

    场景: passthrough-raw
      假如 passthrough 模式
      当 转发任意 ServerToClient 帧
      那么 字节语义与输入一致

  @req:r58
  规则: TOON 基准列
    系统 MUST 在 bench-report 示例中同时报告压缩 JSON 相对原始与 TOON 相对原始的 token 节省（cl100k_base），以便与 README 压缩效果表对齐。

    场景: bench-toon-column
      假如 fixtures/bench 用例
      当 运行 examples/bench-report
      那么 输出含紧凑与 TOON 两列节省
