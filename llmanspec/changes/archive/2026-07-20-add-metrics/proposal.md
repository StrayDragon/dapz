---
depends_on: []
---

## Why

lspz 有进程内压缩 metrics。dapz 运行时缺少 DAP 拦截链字节/延迟快照。

## What Changes

- 新增 `src/metrics.rs`：`MeteredInterceptor`（DAP `DapMessage` 适配；对照 lspz `MetredInterceptor`）
- `DAPZ_METRICS=1` 时 proxy 链包装计量
- 单测：sizes / disabled / failure / ratio
- Delta：新 capability `metrics`

## Out of Scope

- Prometheus/OTLP；daemon 聚合；启用 BDD

## Impact

- `src/metrics.rs`, `src/main.rs`, `src/lib.rs`
