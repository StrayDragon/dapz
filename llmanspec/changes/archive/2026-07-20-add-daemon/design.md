## Design: DAP daemon ≠ LSP daemon

| | lspz | dapz |
|--|------|------|
| Socket key | workspace root (docs) | **project cwd** |
| Session key | language:backend:root | **backend::cwd** (`pool_key`) |
| RPC | lsp/request, sync_document | **dap/request**, dap/wait_event |
| Child | language server | **debug adapter** (debugpy…) |

Idle reaper: clear empty pool after idle; no LSP document buffers.
