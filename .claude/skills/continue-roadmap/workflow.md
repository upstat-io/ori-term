# /continue-roadmap — Sub-agent Workflow

**This file is read by the sub-agent dispatched from `SKILL.md`.** The protocol is deliberately thin: the scanner (`roadmap_scan.py --json`) pre-computes every gate decision, focus-context field, and next-unblocked pointer. The sub-agent's job is to run the scanner once, apply mechanical auto-fixes, and fill a handoff template from the JSON.

**The goal is SPEED.** A clean run completes in **3–8 tool calls** and well under a minute. This is a status reporter, not an investigator. All investigation — code reading, compiler invocation, test analysis, typeck diagnosis, git archaeology — is `/roadmap-work`'s job (Opus, full capability). Your job is to run the scanner, apply auto-fixes the scanner flagged, and hand off. Nothing else.

## Tool-call budget

| Phase | Expected tool calls |
|---|---|
| Read this file (`workflow.md`) | 1 |
| Run `roadmap_scan.py --json` | 1 |
| Auto-fix edits (Step 2a/2c) | 0–N (one `Edit` per mismatch; usually 0–3) |
| `plan-annotations.sh --cleanup-only` / `--count` (Step 2b) | 0–2 |
| `Skill: commit-push` if any auto-fix ran (Step 2d) | 0–1 |
| **Total for a clean run** | **3–8** |

**If you cross ~15 total tool calls, you are off-contract.** Stop whatever you are doing, fill the handoff with whatever scanner results you have, and escalate. Do NOT keep going to "understand" the failure.

## Hard bans — what you MUST NOT do

These are load-bearing invariants. A sub-agent that violates ANY of these has broken the skill's speed contract and has wandered into /roadmap-work's territory.

- **NEVER run any compiler/test/build binary.** Banned commands include (non-exhaustive): `cargo`, `cargo check`, `cargo run`, `cargo test`, `cargo clippy`, `cargo b`, `cargo t`, `cargo st`, `cargo stf`, `cargo test --all`, `cargo clippy --all -- -D warnings`, `cargo build --all`, `cargo test --all`, `cargo test --all && cargo clippy --all`, `ori`, `oric`, `./target/debug/ori`, `./target/release/ori`, `~/.local/bin/ori`, any script under `diagnostics/` **except `diagnostics/state.sh show` / `check` / `known-failing`** (read-only cache reads, no compilation), any script under `scripts/` that invokes the compiler. The scanner's JSON is the complete world-state you are allowed to observe.
- **NEVER read `.rs`, `.ori`, `.toml` source files** or any file under `compiler/`, `library/`, `tests/`, `scripts/`. Plan-doc edits in `plans/` are fine (Step 2); source reads are not.
- **NEVER investigate test failures, typecheck errors, dirty-tree contents, bug repros, or diagnostic output.** When a gate fires, ESCALATE IMMEDIATELY. Do NOT peek inside to "understand why" — the parent + user + `/roadmap-work` own that step.
- **NEVER run `git log`, `git blame`, `git show`, `git diff`, `git bisect`**, or any git archaeology. The scanner already captured the only git state gates need (via `dirty_tree`).
- **NEVER run intelligence-graph queries** (_(intel-query not available in this project; use Grep/Glob)_) — that belongs to `/review-plan` / `/roadmap-work`, not scan+gates.
- **NEVER read `CLAUDE.md` or `.claude/rules/*.md`** — not needed for JSON transcription; the scanner pre-computes every field.
- **NEVER read section files by hand** — their fields are in `focus_context` / `next_unblocked`.
- **NEVER validate blockers by opening sibling plan files** — the scanner already classifies them.
- **NEVER edit `.rs`, `.ori`, or any file under `compiler/`, `library/`, `tests/`.**
- **NEVER run `git add` / `git commit` directly** — commits go through `/commit-push`.

**You DO:**
- Run the scanner once, parse JSON
- Apply mechanical auto-fixes to stale frontmatter (plan-doc-only writes)
- Build the handoff block from JSON fields
- Stop and escalate when any gate fires — the parent handles escalations

---

## Step 1 — Run the Scanner

```bash
python3 .claude/skills/continue-roadmap/roadmap_scan.py --json
```

If args were passed to `/continue-roadmap` (e.g., `section-4`, `4`, keyword), append the plan dir and/or section number as positional args. Without args, the scanner auto-selects via reroute priority then first-incomplete.

Parse stdout as JSON. All subsequent steps read this object; do not open plan files by hand.

`render_json` (in `roadmap_scan.py`) emits exactly three top-level fields — nothing else exists in the JSON envelope:

- `focus_context` — plan full name (`plan_full_name`), description (`plan_description`), section goal (`section_goal`), subsection list (`subsections`), progress text (`plan_progress_pct` / `plan_progress_text` / `section_progress_text`). Feeds the handoff's focus-context block verbatim.
- `next_unblocked` — `{subsection_id, item_content, item_lineno, unblocked_count, blocked_count}` for the first actionable `- [ ]` item. May be `null` if the focus section has no unblocked items.
- `gates` — 10 pre-computed gate entries, each `{fires: bool, severity, payload}`. Unfired gates have an empty `payload: {}`; firing gates carry `options` + `question` where user interaction is needed.

If you need cross-plan diagnostics (mismatches, orphan blockers, health signals), invoke `roadmap_scan.py` WITHOUT `--json` — the rich-text mode includes them. The `--json` envelope is intentionally minimal; do not expect `focus.plan` / `focus.section` / `health.*` top-level keys to exist.

If `scanner exit code != 0` or JSON parse fails: return `<escalate-to-parent>` with the raw stderr. Do not try to recover.

## Step 1.5 — Read cached repo state (fast)

```bash
diagnostics/state.sh show --json
```

Capture the JSON. This is a read-only cache query (sub-100 ms) that tells you whether the tree is in a plan-documented "known-failing" state so you don't have to infer it from `dirty_tree` or the user's prior messages. The cache is maintained by `/commit-push` (post-push SHA bump) and `state.sh refresh --full` (explicit boundaries).

Fields to extract:
- `.test_suite.status` — `known-failing` | `clean` | `unknown`
- `.test_suite.totals.failed` — current expected-failing count
- `.test_suite.known_failing_count` — total files in the documented known-failing set
- `.test_suite.remediation[0].plan` + `.subsection` — plan pointer for the remediation
- `.clippy.status` — `clean` | `warnings` | `unknown`
- `.hygiene.status` — `clean` | `noise` | `unknown`

If the state file is missing or `state.sh` exits non-zero: log the failure and set state values to `"unknown"` in the handoff block below. Do NOT try to `refresh` it — that's slow work owned by the parent or by `/commit-push`.

If `state.sh check` would return `obsolete` (cache SHA != HEAD SHA), state is stale but still informative — include it in the handoff, flagged as `stale (pre-commit ${cache_sha})`. The parent can decide whether to invalidate.

## Step 2 — Auto-fix Cleanup (mechanical, silent)

**Cleanup is never user-facing.** Any `auto-fix` gate that fires runs inline without asking, without surfacing options, without emitting an escalation. The user has pre-approved all cleanup — scanner-detected cruft is mechanical by definition.

### 2a. Stale frontmatter

If `gates.stale_frontmatter.fires` is true, iterate `payload.focus_plan_mismatches` and apply these rules via the Edit tool on the relevant plan/section/overview files:

| Scanner issue | Fix |
|---|---|
| `frontmatter=complete but N unchecked` | set status `in-progress` (or `not-started` if 0 checked) |
| `frontmatter=not-started but N checked` | set status `in-progress` (or `complete` if 0 unchecked) |
| `frontmatter=in-progress but all items checked` | set status `complete` |
| `frontmatter=in-progress but 0 items checked` | set status `not-started` |
| `third_party_review.status=findings but all N findings resolved (should be resolved)` | set `third_party_review.status: resolved` (keep `updated:` date as-is) |
| `third_party_review.status=findings but no TPR findings parsed (should be resolved or none)` | set `third_party_review.status: resolved` if `updated:` is present, else `none` with `updated: null` |
| `third_party_review.status=resolved but N finding(s) still open (should be findings)` | set `third_party_review.status: findings` |

**Do NOT fix** any mismatch outside the focus plan. Report those in the handoff's Gate results as informational only.

**Why the TPR drift rows matter**: the `tpr_findings` gate (Step 3) fires whenever `third_party_review.status == "findings"`, independent of whether any finding checkbox is still open. Historically the drift was unhealable — `/verify-tpr` has nothing to triage when 0 findings are open, so `/continue-roadmap` would loop on this gate forever. These rows close the loop by letting Step 2a silently flip the stale status BEFORE Step 3 evaluates `tpr_findings`.

### 2b. Stale plan annotations

If `gates.stale_plan_annotations.fires` is true (count > 0), run the mechanical cleanup tool scoped to the focus plan:

```bash
bash .claude/skills/impl-hygiene-review/plan-annotations.sh --cleanup-only --plan <focus_context.plan_name>
```

This is a shell invocation of an idempotent tool — the "never edit .rs" rule applies to manual edits, not to calling a dedicated cleanup script. The tool strips stale plan-specific annotations (TPR, CROSS, BUG, §, Phase, section-NN refs) from `.rs` files in the focus plan's scope. Spec references (`Spec: Clause N.M`) are permanent and are NOT removed.

After the tool returns, verify the new count via `--count`. Include the "before → after" numbers in the handoff.

### 2c. Bug-entry marker drift (auto-fix Superseded markers)

If `gates.bug_marker_drift.fires` is true and `payload.missing_marker_count > 0`, iterate `payload.auto_fix_edits` and apply each edit to the named bug-tracker section file via the Edit tool.

Each edit specifies:
- `file`: target path (always `plans/bug-tracker/section-NN-*.md`)
- `bug_id`: which bug entry receives the marker
- `header_lineno`: 1-based line number of the entry header
- `insert_line`: the full `Superseded by:` line to insert (already formatted with attribution)
- `rationale`: human-readable explanation of why the marker is being added

Application pattern: Read the file, locate the bug entry header at `header_lineno`, walk forward through the indented body lines until you hit the next blank line or the next `- [` header, insert `insert_line` immediately after the last body line. Preserve indentation. The auto-fix is idempotent — if a `Superseded by:` line already exists in the entry body, skip silently (the validator is conservative about this).

If `payload.orphan_marker_count > 0`, iterate `payload.orphan_findings` and emit each as an `info`-level entry in the handoff's Gate results — these are NOT auto-fixed (a bug entry's marker points at a plan that doesn't claim it; user must reconcile manually). Surface format: `Orphan supersede marker: {bug_id} declares {declared_target}, but plan {claiming_plans or '(none)'} doesn't claim it.`

This gate exists because the `Superseded by:` lifecycle marker is the routing signal `/fix-bug` Phase 0 uses to skip Phase -1 (the ~170k-token rules-file re-read). Missing markers re-trigger the waste on every invocation. The plan frontmatter `supersedes:` field is the canonical SSOT — bug entries are derived; this gate enforces the derivation.

### 2d. Commit all cleanup together

If ANY fix was applied in 2a, 2b, or 2c, commit everything via `Skill: commit-push` with a message like `chore(plans): auto-fix stale frontmatter + plan annotations + bug marker drift`. Do not create separate commits per fix type — one cleanup commit covers the pass.

## Step 3 — Evaluate Remaining Gates

After Step 2, only `block`-severity and `info`-severity gates remain. For each entry with `fires: true`, act per the table below. If ANY gate with `severity: "block"` fires, stop and return `<escalate-to-parent>` — do NOT proceed to the normal handoff. The parent will invoke `AskUserQuestion` with the `payload.options` array.

| Gate key | Severity when fires | Action |
|---|---|---|
| `parse_error_sections` | `block` | Escalate. One or more sections have YAML parse errors — focus selection is unreliable. Fix the YAML before proceeding. |
| `stale_frontmatter` | `auto-fix` | Handled silently in Step 2a — never escalates. |
| `stale_plan_annotations` | `auto-fix` | Handled silently in Step 2b — never escalates. |
| `bug_marker_drift` | `auto-fix` | Handled silently in Step 2c — auto-inserts missing `Superseded by:` markers; surfaces orphan markers as info. Never escalates. |
| `unmet_dependencies` | `block` (dep cycle or unresolvable-plus-unmet edge case) / `info` (unresolved refs only) | Normally does NOT fire — `crawl_workspace` transitively follows `depends_on` to the first unblocked section before gates run, so the focus you see is already past the dep chain. If it DOES fire `block`, it means the walker hit a cycle or an unresolvable ref alongside an unmet ref — escalate with the payload's options (switch/proceed/pick-different). `info` severity surfaces stale unresolvable refs for the user's awareness; does not block. |
| `unreviewed_plan` | `block` | Escalate. `payload.options` offers `/review-plan`, proceed-anyway, pick-different. |
| `tpr_findings` | `block` | Escalate. Parent invokes `/verify-tpr` with `payload.next_skill_arg`. |
| `critical_bugs` | `block` | Escalate. Parent invokes `/fix-bug` with the bug IDs from `payload.bugs`. Includes bugs elevated from `high` when they block focus-section items via `<!-- blocked-by:BUG-XXX -->` annotations (marked `elevated: true` in the payload). |
| `high_bugs` | `info` | Include bug IDs in handoff summary. Not blocking. Bugs elevated to `critical_bugs` via blocked-by are removed from this list. |
| `dirty_tree` | `block` | Escalate. `payload.options` offers `/commit-push` or proceed-dirty. **NEVER** run destructive git commands to clean up. |

When multiple block-severity gates fire together (e.g., unreviewed + dirty tree), escalate with ALL of them listed — the parent will ask the user each one via sequential `AskUserQuestion` calls.

## Step 4 — Pacing Question (only when no block-gates fire)

If Step 3 resolved cleanly, ask the user via `AskUserQuestion`:

- **Question**: "How should I pace Section {focus_context.section_number}?"
- **Options**:
  - `subsection-by-subsection` (recommended) — pause after each subsection completes
  - `full-section` — run all subsections continuously without pausing

Record the choice for the handoff. The subsection close-out retrospective (`/improve-tooling`) is mandatory regardless of pacing — pacing only controls pause-for-review, not gate skipping.

## Step 5 — Emit Handoff

Return the handoff block in this EXACT format (filling from JSON fields):

```
## Handoff to parent

**Focus plan**: {focus_context.plan_dir}
**Focus section file**: {focus_context.section_file}
**Focus subsection id**: {next_unblocked.subsection_id}
**Pacing choice**: {from Step 4}

### Focus context
## Focus: {focus_context.plan_full_name} — Section {focus_context.section_number}: {focus_context.section_title}

**Plan**: {focus_context.plan_description}
**Section goal**: {focus_context.section_goal}
**Plan progress**: {focus_context.plan_progress_pct}% ({focus_context.plan_progress_text})
**Section progress**: {focus_context.section_progress_text}

Subsections:
  {each subsection: "  {id}  {title}  [{status}]"}

### Gate results
- Stale frontmatter: {fixed count or "none"}
- Stale plan annotations: {count or "none"}
- Unreviewed plan: {pass | block}
- TPR findings: {none | N open}
- Critical bugs: {none | N}
- High bugs: {none | N (list IDs)}
- Dirty tree: {clean | N files}

### Cached repo state (from Step 1.5)
- Test suite: {test_suite.status} ({test_suite.totals.passed} passed / {test_suite.totals.failed} failed / {test_suite.totals.skipped} skipped at cache SHA {head_sha})
- Known-failing files: {test_suite.known_failing_count}
- Remediation: {test_suite.remediation[0].plan} §{test_suite.remediation[0].subsection} ({test_suite.remediation[0].class}) — or "none" if array empty
- Clippy: {clippy.status}
- Repo hygiene: {hygiene.status}{ " — " + hygiene.notes if notes present }
- Cache freshness: {fresh | stale (dirty tree) | obsolete (SHA mismatch, was {cache_sha}) | missing}

### Next unblocked item
Subsection {next_unblocked.subsection_id}: {next_unblocked.item_content}
({next_unblocked.unblocked_count} unblocked, {next_unblocked.blocked_count} blocked remaining)

### Next command for the parent
Skill: roadmap-work {focus_context.section_file} {next_unblocked.subsection_id}
```

**If escalating**, keep ALL prior blocks (`## Handoff to parent` header, `### Focus context`, `### Gate results`, `### Next unblocked item`) EXACTLY as above — the Focus Context block is MANDATORY on every return, escalation included. Only the final `### Next command for the parent` block is replaced with:

```
### <escalate-to-parent>
**Gates fired**:
{list each firing block-severity gate with its payload}

**User questions for AskUserQuestion**:
{for each block-gate, emit a question + the payload.options array verbatim so the parent can call AskUserQuestion without re-deriving options}

**Relevant paths**:
{collected from gate payloads: section_path, files list, bug source sections, etc.}
```

Rationale: the parent's Step A prints the Focus Context block to the user before any `AskUserQuestion` prompt so the user knows which plan/section the gate decisions apply to. Omitting Focus Context on escalation forces the parent to re-query the sub-agent (wasted tokens) or fire gates blind (wrong answer likely).

---

## Return Format Example (escalation path)

When `unreviewed_plan` and `dirty_tree` both fire, the sub-agent returns the FULL handoff — Focus Context and all — with the final block swapped for `<escalate-to-parent>`:

```
## Handoff to parent

**Focus plan**: plans/empty-container-typeck-phase-contract
**Focus section file**: plans/empty-container-typeck-phase-contract/section-03-bodies-pass-integration.md
**Focus subsection id**: 03.1
**Pacing choice**: (not asked — blocked on gates)

### Focus context
## Focus: Empty Container Typeck Phase Contract — Section 03: Bodies Pass Integration

**Plan**: {plan_description from scanner}
**Section goal**: {section_goal from scanner}
**Plan progress**: 40% (2/5 sections complete)
**Section progress**: 0/4 subsections complete

Subsections:
  03.1  Wire bodies pass into typeck driver   [not-started]
  03.2  Propagate empty-container constraints [not-started]
  03.3  Integration tests                     [not-started]
  03.4  Plan-annotation cleanup               [not-started]

### Gate results
- Stale frontmatter: none
- Stale plan annotations: none
- Unreviewed plan: block
- TPR findings: none
- Critical bugs: none
- High bugs: none
- Dirty tree: 3 files

### Next unblocked item
Subsection 03.1: Wire bodies pass into typeck driver
(4 unblocked, 0 blocked remaining)

### <escalate-to-parent>
**Gates fired**: unreviewed_plan (block), dirty_tree (block)

**User questions for AskUserQuestion**:

Q1 (unreviewed_plan):
  question: "Section 03 has `reviewed: false`. Its assumptions have not been validated against the current codebase. How do you want to proceed?"
  options:
    - "Run /review-plan now (recommended)"  → next_skill: review-plan, arg: {section_path}
    - "Proceed anyway"  → next_skill: null
    - "Pick a different section"  → next_skill: null

Q2 (dirty_tree):
  question: "Working tree has 3 pending files from other sessions. How do you want to proceed?"
  options:
    - "Run /commit-push (recommended)"  → next_skill: commit-push
    - "Proceed with dirty tree"  → next_skill: null

**Relevant paths**: {section_path}, {each file in dirty_tree.files}
```

The parent surfaces the Focus Context block verbatim (per `SKILL.md` Step A), emits a one-line gate summary, then invokes `AskUserQuestion` with Q1, waits for the answer, dispatches the chosen skill if any, then does the same for Q2. After both resolve, the parent re-dispatches `/continue-roadmap` for a fresh scan — do NOT reuse this scan's output across the escalation boundary.
