# language: en
# capability: metrics
# purpose: DAP 拦截链运行时指标（字节/延迟/压缩比；非 LSP diagnostics metrics）。
# scope: src/metrics.rs, src/main.rs, tests/

Feature: metrics

  @req:r44 @human
  Scenario: MeteredInterceptor
    - 系统 MUST 提供 MeteredInterceptor，在启用时记录 DAP 消息输入/输出字节、延迟与失败数。

  @req:r45 @human
  Scenario: 默认关闭
    - Metrics 默认 MUST 关闭；仅当 --metrics 或 DAPZ_METRICS 显式启用时 proxy 拦截链才包装计量（零开销默认路径）。

  @req:r46 @human
  Scenario: 压缩比快照
    - MetricsSnapshot MUST 提供 compression_ratio 与 summary，便于 tracing 观测 DAP 压缩效果。

  @req:r44 @human
  Scenario: record-sizes
    - MUST hold: Given 启用的 MeteredInterceptor; When 处理一条 ServerToClient DAP 消息; Then messages_processed 增 1 且字节计数增加.
    Given 启用的 MeteredInterceptor
    When 处理一条 ServerToClient DAP 消息
    Then messages_processed 增 1 且字节计数增加

  @req:r45 @human
  Scenario: cli-or-env
    - MUST hold: Given 传 --metrics 或设 DAPZ_METRICS=1; When 启动 proxy 链; Then 计量包装启用.
    Given 传 --metrics 或设 DAPZ_METRICS=1
    When 启动 proxy 链
    Then 计量包装启用

  @req:r46 @human
  Scenario: ratio
    - MUST hold: Given input=200 output=50; When 调用 compression_ratio; Then 返回约 0.75.
    Given input=200 output=50
    When 调用 compression_ratio
    Then 返回约 0.75
