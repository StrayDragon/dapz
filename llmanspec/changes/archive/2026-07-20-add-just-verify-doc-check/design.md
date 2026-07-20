## Design: verify / doc-check

Mirror lspz gate layout without pulling LSP-specific steps:

| Recipe | dapz | Notes |
|--------|------|-------|
| `just qa` | fmt + clippy + test + **doc-check** | Daily gate |
| `just verify` | qa contents + SDD validate + prek | Pre-release / apply-cycle |
| `just harness` | env + qa + **debugpy e2e** | DAP-specific; stays separate |

BDD remains off; `llman sdd validate` is documentation-contract only.
