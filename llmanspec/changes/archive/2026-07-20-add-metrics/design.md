## Design: metrics (DAP)

lspz wraps LSP interceptors that take `(method, params)`.
dapz wraps DAP interceptors that take `DapMessage`.

Sizing uses `DapMessage::to_bytes()` length (framed payload), not LSP JSON-RPC params alone.

Enable via `DAPZ_METRICS` env (default off → zero overhead).
