## Design: Proxy CLI knobs

| Flag | Default | Notes |
|------|---------|-------|
| `--output` | `json` | **冻结**：IDE/调试器友好；MCP/SDK 仍 TOON |
| `--metrics` | false | OR with `DAPZ_METRICS=1\|true\|yes\|on` |
| `--transport` | `stdio` | `tcp://host:port`；`ws://`/`wss://` 需 `transport-websocket` |

Transport creation mirrors lspz `create_transport`. TCP always available；WS feature-gated with clear error when disabled.
