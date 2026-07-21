## Design: DAP protocol surface

### AdapterKind

| Kind | Detect (backend cmd substring) | adapterID | Launch extras |
|------|--------------------------------|-----------|---------------|
| Debugpy | `debugpy` | `python` | `console: internalConsole`（debugpy 扩展；验证：uv tool debugpy as of 2026-07） |
| Lldb | `lldb` | `lldb-dap` | none |
| Generic | else | `dapz` | none |

### Launch body (all kinds)

`program`, `cwd?`, `args?`, `stopOnEntry`, `noDebug` — DAP-common. No other keys unless profile adds.

### initialized

`wait_for_event_where` already prefers buffer → works for early (post-initialize) and late (post-launch) emitters. Document; do not wait in `initialize()` itself (keeps debugpy late path).

### threadId

```text
resolve_thread_id(opt):
  if opt Some → use
  else threads → first threads[].id
  else Err("no threads")
```

### Compress contract

Constant `DAPZ_COMPRESS_CONTRACT = "dapz-compress/1"` in interceptors; field-delete set only changes with contract bump.
