# DAP Compatibility Surface

How dapz shapes DAP `initialize` / `launch` and resolves `threadId`.

Compression field policy is versioned separately as **`dapz-compress/1`**
(`crate::interceptors::DAPZ_COMPRESS_CONTRACT`). Bump that id when changing
which optional DAP fields Agents observe after compression.

## Adapter profiles

| Kind | Backend detect | `initialize.adapterID` | Launch extras |
|------|----------------|------------------------|---------------|
| **Generic** | (default) | `dapz` | none |
| **Debugpy** | cmd contains `debugpy` | `python` | `console: "internalConsole"` (debugpy extension, not DAP core) |
| **Lldb** | cmd contains `lldb` | `lldb-dap` | none |

Verified with:

- debugpy via `uv tool` / `python3 -m debugpy.adapter` (2026-07)
- system `lldb-dap` (2026-07)

## Launch body (all kinds)

Always may include: `program`, `cwd?`, `args?`, `stopOnEntry`, `noDebug`.

No other keys unless the active profile adds a documented extra.

## `initialized` event

Adapters may emit `initialized`:

- **Early** — after `initialize` response
- **Late** — after receiving `launch` / `attach` (e.g. debugpy)

`DapSession::wait_initialized` prefers the pending-event buffer, then waits.
`initialize()` itself never blocks on `initialized`.

## `threadId`

When a tool omits `thread_id`, dapz calls `threads` and uses the first
`threads[].id`. It does **not** blind-default to `1`. If there are no threads,
the operation fails with a protocol error.
