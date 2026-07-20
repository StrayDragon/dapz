# llmanspec AGENTS.md

This file is referenced by the root `AGENTS.md` managed block. Use it to add
project-specific rules, context, or conventions that you want AI agents to follow.

<!-- Add your rules below this line -->

## dapz SDD conventions

- Change ids: kebab-case with verb prefix (`add-`, `update-`, `remove-`, `refactor-`, `docs-`).
- Capability dirs under `llmanspec/specs/`: prefer `proxy`, `mcp`, `agent-sdk`, `transport`, `codec`, `adapters`, `interceptors`, `ssot-rules` (mirror lspz where sensible; DAP-specific names ok).
- **`req_id` MUST be globally unique** across all capabilities (use `llman sdd spec next-req-id` / `project dedupe-req-ids`).
- **BDD-off**: do not enable `bdd:` in `config.yaml`. Scenario rows MUST use `feature: false` (doc-only GWT).
- Reference implementation: `../lspz` is **read-only**; copy-port files, do **not** extract a shared crate.
- Do not add `daemon` / `init` / `uri` / `config_watcher` unless `_PLAN.md` §9 explicitly promotes them.
- Validation gate before apply: `llman sdd validate <change-id> --strict --no-interactive`.
- Project qa after apply: `just qa` (and `just harness` when touching discovery / debugpy).
