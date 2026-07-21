## Design: Proxy output formats (lspz-aligned)

### Matrix

| Format | Interceptors | Wire shape |
|--------|--------------|------------|
| `passthrough` | skip | original bytes |
| `json` | yes | DAP frame, compressed body |
| `toon` (default) | yes | DAP frame; `body = { "format": "toon", "text": <toon> }` |

Encode failure → fail-open original bytes (WARN).

### TOON library

`value_to_toon` → `toon_format::encode_default`. No hand-rolled encoder for the public path.

### DAP vs LSP envelope

lspz wraps JSON-RPC `params`/`result`. dapz wraps DAP `body` the same `{format,text}` pattern so clients can detect TOON without breaking seq/type/command/event/request_seq/success.
