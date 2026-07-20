## Design notes (bootstrap)

### Approach

Capture **as-is** v0.1 behavior as BDD-off live specs via change deltas → archive merge.
No application code in this change.

### Mapping from lspz

| lspz capability | dapz capability | Notes |
|-----------------|-----------------|-------|
| product-requirements | product-requirements | DAP Tier-0; no daemon in MVP |
| proxy / transport / codec | same names | Protocol is DAP not LSP |
| languages | adapters | debugpy-first + `resolve_tool` |
| interceptors | interceptors | DAP message types |
| mcp / agent-sdk | mcp / agent-sdk | 17 tools; AgentPool = gap |
| ssot-rules | ssot-rules | Include `_PLAN.md` + tips |
| daemon / uri / init | — | Explicitly omitted |

### Req_id scheme

Per-capability `r1…` starting at 1 (fresh baselines). Later changes use `modify_requirement` / new ids.

### Follow-ups (not this change)

See `_PLAN.md` §9: `add-agent-sdk-pool`, `add-just-verify-doc-check`, `add-metrics`, …
