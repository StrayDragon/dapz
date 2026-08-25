# language: en
# capability: codec
# purpose: 编解码：JSON-RPC 与 TOON/passthrough 输出。
# scope: src/codec/, tests/

Feature: codec

  @req:r8 @human
  Scenario: JSON-RPC 编解码
    - 系统 MUST 编解码 JSON-RPC 2.0 DAP 消息。

  @req:r16 @human
  Scenario: TOON 输出
    - 系统 MUST 提供 value_to_toon：将任意 JSON Value 编码为 TOON 文本，且 MUST 使用 toon-format（encode_default）实现，以保证与官方 TOON 规范一致。

  @req:r24 @human
  Scenario: Passthrough
    - output_format=passthrough 时 Proxy MUST NOT 改写消息体。

  @req:r58 @human
  Scenario: TOON 基准列
    - 系统 MUST 在 bench-report 示例中同时报告压缩 JSON 相对原始与 TOON 相对原始的 token 节省（cl100k_base），以便与 README 压缩效果表对齐。

  @req:r8 @human
  Scenario: roundtrip
    - MUST hold: Given 合法 DAP JSON 对象; When encode/decode; Then 字段语义保持.
    Given 合法 DAP JSON 对象
    When encode/decode
    Then 字段语义保持

  @req:r16 @human
  Scenario: toon-format-crate
    - MUST hold: Given 含嵌套对象与表格数组的 Value; When 调用 value_to_toon; Then 得到非空 TOON 字符串.
    Given 含嵌套对象与表格数组的 Value
    When 调用 value_to_toon
    Then 得到非空 TOON 字符串

  @req:r24 @human
  Scenario: passthrough-raw
    - MUST hold: Given passthrough 模式; When 转发任意 ServerToClient 帧; Then 字节语义与输入一致.
    Given passthrough 模式
    When 转发任意 ServerToClient 帧
    Then 字节语义与输入一致

  @req:r58 @human
  Scenario: bench-toon-column
    - MUST hold: Given fixtures/bench 用例; When 运行 examples/bench-report; Then 输出含紧凑与 TOON 两列节省.
    Given fixtures/bench 用例
    When 运行 examples/bench-report
    Then 输出含紧凑与 TOON 两列节省
