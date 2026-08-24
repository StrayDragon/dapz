---
name: "llman-sdd-graph"
description: "Visualize llman SDD change dependency relationships as a mermaid graph. Use to understand blocking and depends_on relationships for planning or inspection. Auxiliary tool — not part of the main implementation pipeline."
metadata:
  version: "0.0.68"
  llman_sdd:
    bdd_mode: "off"
    skill_set: "default"
---

# LLMAN SDD Dependency Graph

Use this skill to visualize dependencies between changes.

## Pipeline Position

```mermaid
flowchart LR
    pipeline["Main pipeline:<br/>propose → apply → verify → archive"]
    graph["📎 llman-sdd-graph<br/>Dependency visualization (utility)"]
    graph -.->|available at any stage| pipeline

    style graph fill:#e8f4e8,stroke:#28a745,stroke-width:2px
```

> 📎 Utility tool, available at any pipeline stage. To propose → `llman-sdd-propose`. To implement → `llman-sdd-apply` only when `readyToImplement=true`.

## Usage

**Focus view (seed mode):** Show a specific change and its relationship neighborhood.

```bash
llman sdd graph <change-id>              # the change + direct relationships (depth 1)
llman sdd graph <change-id> --depth 3    # recurse 3 levels
llman sdd graph <change-id> --depth 0    # just the change itself
```

Seed mode traverses three directions: upstream (depends_on), downstream (depended by), and blocks, automatically discovering active and archived changes.

**Global view (scope mode):** Show all changes by scope.

```bash
llman sdd graph                          # all active changes (default)
llman sdd graph --scope archived         # all archived (completed) changes
llman sdd graph --scope all              # everything
```

## Output

- Output is a mermaid flowchart to stdout, pipeable to a file or renderer:
  ```
  llman sdd graph c50 > deps.mmd
  llman sdd graph c50 --depth 2 | mmdc -i - -o deps.png
  ```
- Archived (completed) changes are shown with "✓ done" suffix and green highlight.
- When the graph contains disconnected groups, each group renders as an independent subgraph labeled "Active", "Done", or "Mixed".

## Proposal frontmatter format

```yaml
---
depends_on:
  - other-change-id
blocks:
  - blocked-change-id
---

## Why
...
```

> 💡 This is just a utility — main flow: `llman-sdd-propose` (Branch binding + Specs landing) → `llman-sdd-apply` (requires `readyToImplement`) → `llman-sdd-verify` → `llman-sdd-archive`.

Before acting, read `llmanspec/config.yaml` and follow its `context` and `rules` if present.

Common commands:
- `llman sdd context --task "<description>" --paths "<files>"` (find relevant specs). Uses the pageindex agentic tree backend (needs `LLMAN_SDD_INDEX_CHAT_MODEL`). Preset via `LLMAN_SDD_INDEX_BACKEND`.
- `llman sdd list` (list changes)
- `llman sdd list --specs` (list specs with purpose/scope metadata)
- `llman sdd show <id>` (show change/spec; `--type change --output json` includes `stage` / `specsLanded` / `skipSpecsLanding` / `readyToImplement` — apply gate is `readyToImplement`, not vague "complete artifacts")
- `llman sdd validate <id>` (validate a change or spec)
- `llman sdd validate --all` (bulk validate)
- `llman sdd index rebuild` (rebuild the pageindex tree index — no model needed)
- `llman sdd index check` (check index freshness)
- `llman sdd change new <id>` (create planning-shell draft `changes/<id>/proposal.md` only; does not write live specs)
- `llman sdd change start <id> [--worktree]` (Designed→Full: clean tree on default branch → create `sdd/<id>` + attach; Branch binding only — not Specs landing, not apply-ready)
- `llman sdd change attach <id> [--force]` (bind an existing non-default feature branch + base SHA; rejects the default branch)
- `llman sdd change finalize <id> [--no-check]` (**recommended single-commit close-out** — after verify; dirty tree OK; gates + auto ff-merge + docs rename)
- `llman sdd change checkpoint <id> [--no-check]` (clean tree + gates before archive; strict sha = HEAD; finalize fallback)
- `llman sdd change diff <id> [--export-patch <path>]` (read-only `base...HEAD` review/export)
- `llman sdd change archive <id>` (seal: auto ff-merge into default branch, then rename docs to `changes/archive/`; prefer `finalize` for single-commit close-out)
- `llman sdd archive freeze [--before YYYY-MM-DD] [--keep-recent N] [--dry-run]` (freeze archived dirs)
- `llman sdd archive thaw [--change <id> ...] [--dest <path>]` (restore from cold-backup)
- `llman sdd graph [CHANGE] [--format mermaid] [--scope active|archived|all] [--depth N]` (generate change dependency graph)
- `llman sdd project migrate --kind spec-md2toon` (`.md`+fence → standalone `.toon`; `partitioned` removed)

Validation fixes (single-track feature-as-spec):

1) Missing header comments (`missing `# capability:`` header comment`):
Every `llmanspec/specs/<capability>/<capability>.feature` MUST start with:
```
# language: zh-CN
# capability: <capability>
# purpose: One-line overview.
# scope: src/
```

2) Tag grammar (`@human constraint scenario must carry an @req:<req_id> tag` / `orphan acceptance scenario`):
- Rules: `@req:<id> @human` — statement in the scenario description (MUST/SHALL required).
- Acceptance: `@executable` + at least one `@req:<id>` linking a rule.
- `@manual` requires `@human`. Never combine `@human` with `@executable`.

3) Legacy `spec.toon` present (`legacy spec.toon found ... run ... toon2features`):
Run `llman sdd project migrate --kind toon2features --yes`, review the diff, commit.

Git-native guardrail:
- **Branch binding** → **Specs landing**: first `change start` / `attach`, then edit live `.feature` files on the bound non-default branch and commit.
- Locked rules: modifying/removing existing `@human` scenarios fails the gate unless the proposal frontmatter has `rules_edit_acked: true`.
- Apply requires `readyToImplement=true` (or `skip_specs_landing`). Close-out prefers `change finalize`.
- Do not use `change delta` / solidify / `*.feature.delta.toon`.

## Ethics Governance
- `ethics.risk_level`: label risk as `low|medium|high|critical`.
- `ethics.prohibited_actions`: list actions that must never be performed.
- `ethics.required_evidence`: list evidence required before high-impact outputs.
- `ethics.refusal_contract`: define when to refuse and the safe alternative response.
- `ethics.escalation_policy`: define when to escalate for user confirmation / human review.
