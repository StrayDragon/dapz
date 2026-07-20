## Design: AgentPool (DAP vs LSP)

### lspz (LSP)

- Key = **language id** (`rust`, `python`)
- Shared `workspace_root`; document sync (`notify_change` / didOpen)
- Query surface: diagnostics / completions / symbols

### dapz (DAP) — this change

- Key = **session name** chosen by the embedder (e.g. `py-main`, `debugpy`)
- No workspace/document model — debug adapters are process-oriented
- Query/control surface: Tier-0 DAP (`launch`, `get_stack`, `get_variables`, …)
- Validation backend for e2e remains **debugpy** (not language servers)

### API sketch

```text
AgentPool::builder()
  .register("py", "python3 -m debugpy.adapter")
  .enable_compression(true)
  .start_all()  // registers only; lazy spawn

pool.launch("py", program, …)
pool.get_stack("py", …)
pool.shutdown_all()
```
