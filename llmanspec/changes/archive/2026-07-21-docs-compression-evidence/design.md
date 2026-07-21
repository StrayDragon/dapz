## Design: compression evidence

| 列 | 含义 | 输入 |
|----|------|------|
| 原始 (T) | fixture 消息 JSON 的 cl100k token | `serde_json::to_string(msg)` |
| 紧凑 (T) | 经对应拦截器后 JSON | interceptor output |
| TOON (T) | 紧凑 body（或整消息）经 `value_to_toon` | 对齐 MCP/SDK 默认出口 |

Scopes / Exception fixtures 只测字段裁剪与 details 截断，不引入新策略。README 表数字必须来自同一次 `just bench-report` 输出，禁止手写区间与报告脱节。
