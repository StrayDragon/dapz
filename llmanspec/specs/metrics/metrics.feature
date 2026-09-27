# language: zh-CN
# capability: metrics
# purpose: DAP 拦截链运行时指标（字节/延迟/压缩比；非 LSP diagnostics metrics）。
# scope: src/metrics.rs, src/main.rs, tests/

功能: metrics

  @req:r44
  规则: MeteredInterceptor
    系统 MUST 提供 MeteredInterceptor，在启用时记录 DAP 消息输入/输出字节、延迟与失败数。

    场景: record-sizes
      假如 启用的 MeteredInterceptor
      当 处理一条 ServerToClient DAP 消息
      那么 messages_processed 增 1 且字节计数增加

  @req:r45
  规则: 默认关闭
    Metrics 默认 MUST 关闭；仅当 --metrics 或 DAPZ_METRICS 显式启用时 proxy 拦截链才包装计量（零开销默认路径）。

    场景: cli-or-env
      假如 传 --metrics 或设 DAPZ_METRICS=1
      当 启动 proxy 链
      那么 计量包装启用

  @req:r46
  规则: 压缩比快照
    MetricsSnapshot MUST 提供 compression_ratio 与 summary，便于 tracing 观测 DAP 压缩效果。

    场景: ratio
      假如 input=200 output=50
      当 调用 compression_ratio
      那么 返回约 0.75
