## Design: Tier-1 exception APIs (DAP ≠ LSP)

| LSP (lspz) | DAP (dapz) |
|------------|------------|
| publishDiagnostics | exceptionInfo / stopped reason |
| textDocument/source N/A | `source` request by sourceReference |
| N/A attach language server | `attach` to running debuggee |

Filters for `setExceptionBreakpoints` are adapter-defined (debugpy: `raised`/`uncaught` etc.).
Compression targets long `details` / stack strings — not diagnostic merging.
