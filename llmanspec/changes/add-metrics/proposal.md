---
depends_on: []
---

## Why

lspz 有 `metrics.rs` + `MetredInterceptor`（字节进出、失败数、压缩比、延迟）。dapz 压缩效果目前靠 bench/fixture 报告，运行时无可观测快照，Agent/运维难以判断某次会话是否在省 token。

## What Changes（draft / BDD-off）

- 新增 `src/metrics.rs`（copy-port lspz 语义，类型名可保留或改为 `MeteredInterceptor`）
- Proxy/InterceptorChain 可选挂载 metrics（默认 off 或 debug log summary）
- 单测：snapshot 累加与 ratio 计算
- Delta：新 capability `metrics` 或挂到 `proxy`/`interceptors`（`feature: false`）

## Out of Scope

- Prometheus/OTLP 导出（首版进程内 snapshot + tracing 即可）
- daemon 侧聚合
- 启用 BDD

## Impact

- Code: `src/metrics.rs`, `src/proxy.rs` / interceptors wiring, tests
- Spec: new or extend existing
- Risk: 热路径分配 — 保持与 lspz 同级的轻量原子/计数

## Status

Draft only. Prefer after `add-just-verify-doc-check` so docs build stays green.
