# Changelog

## v0.0.1 (2026-05-12)

Crate name registration — initial dapz project scaffolding.

- Project structure mirroring lspz architecture
- Cargo workspace with feature flags (cli, mcp, agent-sdk, transport-tcp, transport-websocket)
- JSON-RPC 2.0 codec with Content-Length frame parsing
- Transport abstraction (Transport trait) with StdioTransport, TcpTransport, MockTransport
- DAP proxy state machine (Created → Initializing → Ready → ShuttingDown → Exited)
- Interceptor trait and chain for Server→Client message transformation
- 3 compressors: OutputCompressor, VariablesCompressor, StackTraceCompressor
- Capping interceptor for limiting output/stack/variables sizes
- CLI entry point via clap with env-var overrides
- Unified error type (DapzError) with thiserror
- Config struct with builder pattern and env-var fallbacks
- SSOT documentation generation framework
- Justfile, prek hooks, CI configuration
