---
name: "llman-sdd-propose"
description: "Create a new llman SDD change proposal with planning artifacts (proposal, delta specs, tasks) in one pass. Use when the user asks to define a formal change — especially for behavioral contract changes that modify MUST/SHALL requirements."
metadata:
  version: "0.0.64"
  llman_sdd:
    bdd_mode: "off"
    skill_set: "default"
---

# LLMAN SDD Propose


Create a new change and generate all planning artifacts in one pass (proposal + delta specs + tasks; design optional), then validate and suggest next actions.


## Pipeline Position

```mermaid
flowchart LR
    explore["llman-sdd-explore<br/>Explore"] --> propose
    propose["★ llman-sdd-propose ★<br/>Propose (you are here)"]
    propose --> apply["llman-sdd-apply<br/>Implement"]
    apply --> verify["llman-sdd-verify<br/>Verify"]
    verify --> archive["llman-sdd-archive<br/>Archive"]

    style propose fill:#fff3cd,stroke:#ffc107,stroke-width:3px
```

> 📍 You are in the propose phase → next: `llman-sdd-apply` (implement)
> 📎 For small changes (no behavioral contract changes), use `llman-sdd-quick` (quick path)

## Hard Constraints

- **Must confirm change id with user before writing files**: change boundaries must stay clear. **Exception**: when the user requests the lightweight draft path (see "Lightweight draft path" below), MUST NOT ask for an id — derive it via `change new --from` and announce it.
- **BDD-off delta specs must have at least one op + one scenario**: otherwise validation fails. (BDD-on uses live specs on the feature branch instead.)
- **Don't ask "should I continue?"**: execute the full propose phase in one pass, generate artifacts and validate.

- **If change already exists**: STOP and suggest `llman-sdd-apply`; to fill missing artifacts, edit `llmanspec/changes/<id>/` directly (or enable `extra_skills: [llman-sdd-continue]`).


## Lightweight draft path (draft proposal only)

When the user's intent is to **quickly capture a proposal** (e.g. "draft a proposal", "draft a change", "note down X") and no change id is provided, take this lightweight path — do **not** run the full propose flow:

1. **MUST NOT ask the user for a change id.**
2. Generate a legal, meaningful change id directly from the user's description:
   - Prefer naming conventions declared in the repo's `llmanspec/AGENTS.md` (if any).
   - With no explicit convention, name by the description's semantics (CLI `--from` does kebab-case sanitizing + legality checks).
3. Call the CLI scaffolding to create the draft shell:
   ```bash
   llman sdd change new --from "<user description>"
   ```
   This creates only `proposal.md` (draft skeleton) under `llmanspec/changes/<derived id>/` — no tasks/design/specs/attach required.
4. **MUST announce the derived id to the user** (e.g. "Created draft change `<id>`; flesh it out at `llmanspec/changes/<id>/proposal.md`"). The user may rename or promote it to a formal change on request.
5. Full propose (triage + tasks + specs + attach) starts only when the user **explicitly asks to formalize**.

Boundary: if the description involves MUST/SHALL behavioral contract changes, multi-file impact, or needs triage, suggest upgrading to full propose rather than stopping at a draft.

## Steps

### 0) Preflight
- Read `llmanspec/config.yaml` for project context, rules, locale.
- `llman sdd validate --all --strict --no-interactive`: ensure current artifacts are clean.
  - If pre-existing errors, stop and report (stacking new changes on dirty artifacts causes cascading errors).
- **Check spec valid_scope integrity**: use `llman sdd list --specs --json` to list all specs, then for each spec verify every path in its `valid_scope` exists on disk. If any scope file/directory is missing, stop and suggest updating the spec (remove the deleted path from `valid_scope`).

### 1) Assess change scale (triage)
   - **Behavioral contract change** (modify MUST/SHALL, change external behavior) → full SDD workflow
   - **Implementation change** (refactor, typo, perf) → quick path via `llman-sdd-quick`
   - **Meta-spec change** (SDD templates/process) → full SDD workflow
   - When uncertain, choose full SDD (conservative).
2. Use `llman sdd context --task "<goal>" --paths "<scope>"` to find relevant specs.
   - If context unavailable, rebuild with `llman sdd index rebuild` (default `pageindex`, no model needed) and continue.
3. Gather input:
   - A short description of the change
   - A change id (or derive one; kebab-case, verb prefix: `add-`, `update-`, `remove-`, `refactor-`)
   - The impacted capability/capabilities (to name `specs/<capability>/`)
   - Confirm the final id before writing files

### 2) Ensure project is initialized:
   - `llmanspec/` must exist; if missing, tell the user to run `llman sdd init`, then STOP.

### 3) Create change directory and artifacts
   - Prefer `llman sdd change new <change-id>` for the draft `proposal.md` shell (or create `llmanspec/changes/<change-id>/` manually).

   - If the change already exists, STOP and suggest filling missing artifacts or `llman-sdd-apply` (optionally enable continue via `extra_skills`).

   - Flesh out `proposal.md` (Why / What Changes / Capabilities / Impact)
   - `design.md` only when tradeoffs/migrations matter
   - `tasks.md` as an ordered checklist (include validation commands)
   - **BDD-off**: also create `specs/<capability>/spec.toon` deltas (standalone TOON, one per file):
     - Prefer authoring helpers: `llman sdd change delta skeleton` / `add-req` / `add-scenario`
     - Include at least one `add_requirement`/`modify_requirement` op (statement MUST contain MUST/SHALL) and at least one matching op scenario row
   - **BDD-on**: do **not** use `change delta` (CLI rejects it) — edit live `llmanspec/specs/**` on the feature branch (see 4b); then `llman sdd change attach <change-id>`

### 4) Validate:
   ```bash
   llman sdd validate <change-id> --strict --no-interactive
   ```
   This MUST pass before proceeding. If TOON parse errors appear, fix quoting:
   values containing commas/colons/brackets must be double-quoted in tabular rows.

### 4a) BDD mode check — before deciding scenario authoring style
- Read `llmanspec/config.yaml`. Is there a `bdd:` block?
  - **Yes (BDD-on)**: follow section 4b below for BDD-on authoring rules.
  - **No (BDD-off)**: if this change involves executable behavior scenarios (Given/When/Then the user will want to run), ask **once, up front**: "This change looks like it has executable behavior. Enable BDD-on mode so scenarios can be validated as `.feature` files? (adds a `bdd:` block to `config.yaml`.)"
    - If **yes**: show the exact `bdd:` block to add (pick a `run_command` matching the project's test framework — `cargo test --features bdd` for rstest-bdd, `pytest {feature_dir} -k {feature_name} -v` for pytest-bdd). Let the user confirm or edit it, write it to `config.yaml`, then proceed with 4b rules.
    - If **no**: proceed with BDD-off authoring (scenarios stay in TOON as documentation; the `feature` field is ignored).
- **Do NOT silently add the `bdd:` block** — always ask first. Adding it changes how `validate`/`index` behave project-wide.

### 4b) BDD-on mode — only when `config.yaml` has a `bdd:` block (Git-native)
- Work on a **non-default Git feature branch** (never propose/implement BDD-on changes on main/master).
- **Partitioned SSOT**: edit live `spec.toon` (constraints) and `*.feature` (executable GWT + `@req`); never dual-write the same scenario id. Dual-write shape reference:

  | Scenario type | `spec.toon` `scenarios[]` | `*.feature` |
  |---|---|---|
  | Executable (`@req` / harness-driven) | **MUST NOT** appear (requirements in toon, examples in .feature) | **only** place for executable GWT |
  | Non-executable (doc-only) | `feature: false` + GWT ok | n/a (do not place) |

  Key point: under Partitioned SSOT, do **not** write `feature: true` rows in toon at all; requirement statements live in toon, executable examples live in `.feature` linked back via `@req:<req_id>`.
- Change shell: `llman sdd change new <change-id>` → fill proposal/tasks → `llman sdd change attach <change-id>`.
- Do **not** run solidify / use `change delta` / create feature_delta; if an active `*.feature.delta.toon` already exists, migrate first.
- **BDD-off** (no `bdd:`): use `change delta …`; no feature branch / attach / checkpoint.

### 4c) BDD-off delta authoring (no `bdd:` block)
- Create the change shell: `llman sdd change new <change-id>`.
- Constraints and scenarios stay in change-scoped TOON via `llman sdd change delta skeleton|add-req|…`.
- Archive later: `llman sdd change archive <id>` merges those deltas into main `spec.toon`.

### 5) Summarize and suggest next step:
   - Enter implementation phase: `llman-sdd-apply`.
   - If you need to think more: `llman-sdd-explore`.

> 💡 Proposal done → next: `llman-sdd-apply` (implement)

Before acting, read `llmanspec/config.yaml` and follow its `context` and `rules` if present.

Common commands:
- `llman sdd context --task "<description>" --paths "<files>"` (find relevant specs). Uses the pageindex agentic tree backend (needs `LLMAN_SDD_INDEX_CHAT_MODEL`). Preset via `LLMAN_SDD_INDEX_BACKEND`.
- `llman sdd list` (list changes)
- `llman sdd list --specs` (list specs with purpose/scope metadata)
- `llman sdd show <id>` (show change/spec)
- `llman sdd validate <id>` (validate a change or spec)
- `llman sdd validate --all` (bulk validate)
- `llman sdd index rebuild` (rebuild the pageindex tree index — no model needed)
- `llman sdd index check` (check index freshness)
- `llman sdd change new <id>` (create draft `changes/<id>/proposal.md`)


- `llman sdd change delta …` (BDD-off only: TOON delta authoring; rejected when BDD-on)

- `llman sdd change archive <id>` (seal a change; BDD-on: docs only after checkpoint / finalize fallback; BDD-off: merge TOON deltas)
- `llman sdd archive freeze [--before YYYY-MM-DD] [--keep-recent N] [--dry-run]` (freeze archived dirs)
- `llman sdd archive thaw [--change <id> ...] [--dest <path>]` (restore from cold-backup)
- `llman sdd graph [CHANGE] [--format mermaid] [--scope active|archived|all] [--depth N]` (generate change dependency graph)
- `llman sdd project migrate [--kind format|partitioned|legacy-bdd|auto]` (one-shot migrations)
Validation fixes (TOON standalone specs):

1) Missing validation scope (`Spec valid_scope must not be empty`):
Main specs MUST carry a non-empty `valid_scope` inside the `.toon` document.
`llmanspec/specs/<feature-id>/spec.toon`:
```toon
kind: llman.sdd.spec
name: sample
purpose: "One-line overview."
valid_scope[1]: src
requirements[1]{req_id,title,statement}:
  r1,Title,System MUST do something.
scenarios[1]{req_id,id,given,when,then}:
  r1,happy,"",a trigger happens,the outcome is observed
```

2) No delta ops in a change: add at least one op + scenario in
`llmanspec/changes/<change-id>/specs/<feature-id>/spec.toon`:
```toon
kind: llman.sdd.delta
ops[1]{op,req_id,title,statement,from,to,name}:
  add_requirement,r1,Title,System MUST do something.,null,null,null
op_scenarios[1]{req_id,id,given,when,then}:
  r1,happy,"",a trigger happens,the outcome is observed
```

3) Tabular value quoting error ("Expected N tabular row values, but got M"):
Values containing **spaces**, commas, colons, or brackets MUST be double-quoted in tabular rows.
```toon
# BAD: spaces in an unquoted value split it into multiple values
r1,happy,"",a trigger happens,the outcome is observed

# GOOD: multi-word values quoted
r1,happy,"","a trigger happens","the outcome is observed"
```

4) BDD-on guardrail (Git-native Partitioned SSOT):
When `config.yaml` has `bdd:`: `spec.toon` = constraints / non-executable scenarios; `*.feature` = executable GWT (`@req`). Edit live files on a non-default branch → `change attach` → prefer `change finalize` (single commit) or fallback `checkpoint` → docs-only `change archive` → Git merge. Do not hunt for solidify, and do not create `*.feature.delta.toon` (if one already exists it is a migration blocker — run `project migrate --kind partitioned`). Empty requirements with no `.feature` = ERROR.

Notes:
- Each spec is a single standalone `.toon` file; there is no Markdown shell or ```toon fence.
- `null` represents missing optional fields.
- Migrate legacy `.md`+fence specs with `llman sdd migrate`.

## Context
- Gather the current change/spec state before acting.
- Prefer `llman sdd context --task --paths` to discover relevant specs instead of guessing or full scans.

## Goal
- State the concrete outcome for this command/skill execution.

## Constraints
- Keep changes minimal and scoped.
- Avoid guessing when identifiers or intent are ambiguous.
- Use `llman sdd context --task --paths` before reading full spec files.
- Choose workflow path based on change scale: behavioral contract changes use full SDD, implementation changes use quick path.

## Workflow
- Use `llman sdd` commands as the source of truth.
- Validate outcomes when files or specs are updated.
- Prefer `llman sdd context` over full reads or guessing.
- When context is unavailable follow error guidance (rebuild index or fall back to `list --specs --json`).

## Decision Policy
- Ask for clarification when a high-impact ambiguity remains.
- Stop instead of forcing through known validation errors.

## Output Contract
- Summarize actions taken.
- Provide resulting paths and validation status.

## Ethics Governance
- `ethics.risk_level`: classify risk as `low|medium|high|critical`.
- `ethics.prohibited_actions`: list actions that MUST NOT be performed.
- `ethics.required_evidence`: list required evidence before high-impact output.
- `ethics.refusal_contract`: define when to refuse and safe alternative response.
- `ethics.escalation_policy`: define when to escalate to user confirmation/review.
