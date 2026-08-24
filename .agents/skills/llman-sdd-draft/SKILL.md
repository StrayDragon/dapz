---
name: "llman-sdd-draft"
description: "Quickly capture a change idea as a draft proposal (proposal.md only, via `change new --from`). No tasks/design/specs/attach. Use to jot down ideas or future requirements; promote to full propose when ready."
metadata:
  version: "0.0.68"
  llman_sdd:
    bdd_mode: "off"
    skill_set: "default"
---

# LLMAN SDD Draft

Capture a change idea as a **draft proposal** (a `proposal.md` skeleton only). This is the lightweight entry point for "just record this idea / future need" — no triage, no tasks, no live specs, no attach. Promote to a formal change with `llman-sdd-propose` when the idea is ready to act on.

## Pipeline Position

```mermaid
flowchart LR
    draft["★ llman-sdd-draft ★<br/>Draft (you are here)"] -.->|"promote"| propose["llman-sdd-propose<br/>Propose"]
    propose --> apply["llman-sdd-apply<br/>Implement"]
    apply --> verify["llman-sdd-verify<br/>Verify"]
    verify --> archive["llman-sdd-archive<br/>Archive"]

    style draft fill:#fff3cd,stroke:#ffc107,stroke-width:3px
```

> 📍 You are at the draft stage → next: flesh out `proposal.md`, then run `llman-sdd-propose` to formalize
> 📎 This skill creates a **draft** change (proposal.md only). Full propose follows Git-native: tasks → Branch binding → Specs landing (see propose lifecycle diagram)
> 🗺️ Skill navigation ≠ Git-native lifecycle; Branch binding / Specs landing are not separate skills

## Hard Constraints

- **MUST NOT ask the user for a change id**: derive it from the description via `change new --from` and announce it.
- **MUST NOT create tasks/design/specs/attach**: this skill creates only the `proposal.md` draft shell. Full planning artifacts belong to `llman-sdd-propose`.
- **MUST NOT run triage or assess change scale**: that is propose's job. If the user wants to start implementing, suggest `llman-sdd-propose`.
- **Scope boundary**: if the description clearly involves MUST/SHALL behavioral contract changes or multi-file impact, suggest `llman-sdd-propose` instead of stopping at a draft — but still create the draft shell first so the idea isn't lost.
- **Frontmatter has a fixed schema**: when fleshing out `proposal.md`, only the allowed fields in `llmanspec/AGENTS.md` "Change Proposal Frontmatter SSOT" are accepted (`depends_on`, `blocks`, `branch`, `base_sha`/`baseSha`, `checkpointed`, `checkpoint_sha`/`checkpointSha`, `skip_specs_landing`). `status`/`title`/`priority`/`author` etc. are rejected by `llman sdd validate` as ERROR. Lifecycle stage is inferred — query it via `llman sdd status`/`show`, never store it in frontmatter. Do not re-declare frontmatter fields in the prose body (no `## Status` block); the body H1 is a human-readable title, not a repeat of the change id.

## Steps

### 0) Preflight
- Read `llmanspec/config.yaml` for project context, rules, locale.
- `llmanspec/` must exist; if missing, tell the user to run `llman sdd init`, then STOP.

### 1) Capture the description
- Take the user's description as-is (e.g. "draft: add a export-to-json command", "note down: we should support worktrees for sdd changes").
- **MUST NOT ask for a change id.** Derive it from the description.

### 2) Create the draft shell
```bash
llman sdd change new --from "<user description>"
```
- The CLI generates a legal kebab-case id (sanitized + validated), creates `llmanspec/changes/<derived id>/proposal.md` (a skeleton with `## Why` / `## What Changes` TODO sections), and prints the final id + path.
- If the derived id collides with an existing change, the CLI fails non-zero; suggest rephrasing the description or using `--force` to overwrite (rare for drafts).

### 3) Announce and hand off
- **MUST tell the user the derived id** (e.g. "Created draft change `<id>` at `llmanspec/changes/<id>/proposal.md`").
- Suggest next steps:
  - Flesh out `proposal.md` (Why / What Changes / Capabilities / Impact) now or later.
  - When ready to act on it, run `llman-sdd-propose` to formalize (triage + tasks → `change start`/`attach` → Specs landing).

> 💡 Draft captured → next: edit `proposal.md`, then `llman-sdd-propose` to formalize.

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

## Ethics Governance
- `ethics.risk_level`: label risk as `low|medium|high|critical`.
- `ethics.prohibited_actions`: list actions that must never be performed.
- `ethics.required_evidence`: list evidence required before high-impact outputs.
- `ethics.refusal_contract`: define when to refuse and the safe alternative response.
- `ethics.escalation_policy`: define when to escalate for user confirmation / human review.
