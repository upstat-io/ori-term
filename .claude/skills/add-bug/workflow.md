# /add-bug — Inline Workflow

**This file is the protocol SKILL.md executes inline in main context.** Plan-doc-only workflow — no code changes, no compiler changes, no commits.

**You do NOT:**
- Edit `.rs`, `.ori`, or anything under `compiler/`, `library/`, `tests/`
- Run `git add` / `git commit` — leave the markdown change unstaged for the caller to bundle
- Invoke `/commit-push` — the caller owns committing
- Dispatch a sub-agent — execution happens inline
- Deep-dive into code — minimal research only (the bug may change before fix time)

**You DO:**
- Read `plans/bug-tracker/section-{NN}-*.md` files (tracker path)
- Read `plans/<plan-name>/section-{NN}-*.md` files (inline path)
- Append to the target plan file (markdown writes)
- Run _(intel-query not available in this project; use Grep/Glob)_ for blast-radius (shell commands)
- Run quick grep/test to confirm the bug exists (shell commands)

---

## Step 0: Big-Picture Analysis — MANDATORY GATE

**Pull back before filing.** Every `/add-bug` invocation decides one routing question:

> **Will the rest of the plan complete without this bug being fixed?**

- **YES** (or no active plan) → **Tracker path** (Steps T1–T7) — bug lives in `plans/bug-tracker/`, independent lifecycle
- **NO** (plan blocks on this fix) → **Inline path** (Steps I1–I5) — bug inlined as new subsection in the active plan; full `/fix-bug` rigor applied to the subsection

### Argument form

| Caller-passed form | Routing decision |
|---|---|
| `/add-bug --inline <plan-section-path> ...` | Inline path — user has already decided; skip the question |
| `/add-bug ...` (no `--inline`) | Ask via `AskUserQuestion` (interactive) — see §Interactive question below |

### Interactive question (when `--inline` NOT passed)

Invoke `AskUserQuestion`:

- **Question:** "Will the rest of the plan complete without this bug being fixed? Pull back and check the whole plan, not just your current section."
- **Header:** "Bug scope analysis"
- **Options** (per `.claude/rules/ask-user-question.md` — recommended option at index 0 with rationale):
  1. `Tracker bug — plan completes without this fix (Recommended)` — Recommended because most `/add-bug` invocations are tangential discoveries with independent lifecycles. Pick this ONLY after confirming the rest of the plan can finish with this bug open. Routes to tracker path.
  2. `Plan blocker — inline into active plan section` — Pick when the plan cannot reach its stated goal with this bug open (per CLAUDE.md §"Plan-Blocker Bugs Belong IN the Plan"). Routes to inline path and will prompt for the plan section path.
  3. `No active plan — standard tracker bug` — Pick when `/add-bug` was invoked outside any plan context (manual session, standalone bug filing). Routes to tracker path.

### Routing rules

- Option 1 or 3 selected → proceed to **Step T1** (Tracker path).
- Option 2 selected → ask a follow-up `AskUserQuestion` for the plan section path (unless already inferable from recent edits):
  - **Question:** "Which plan section is blocked? Paste the relative path to the `plans/<plan>/section-{NN}-*.md` file."
  - **Header:** "Inline target section"
  - **Options:** auto-fill up to 3 plausible options from recently-edited plan section files + an "Other" fallback.
  - After the user confirms a path → proceed to **Step I1** (Inline path).
- `--inline <path>` passed → proceed directly to **Step I1** with the given path.

### Autopilot interaction

- `/add-bug` is not itself invoked autopilot-style, but `/fix-bug --autopilot` may invoke it for discovered-during-fix bugs. In that case, `--inline` is NEVER auto-routed — autopilot always takes the tracker path (the active `/fix-bug` owns its own fix section; a blocker discovered mid-fix goes into the tracker and triggers fix-interference shelving per `/fix-bug` SKILL.md §Autopilot Mode).

---

## Tracker Path — Steps T1–T7

Use this path when Step 0 routed to tracker (option 1 or 3, or autopilot).

## Step T1: Determine Subsystem

Map the bug to one of the subsystem sections in `plans/bug-tracker/`. The authoritative list (and current open/closed counts) is in `plans/bug-tracker/00-overview.md` — re-read that file any time you are unsure which section to target.

| Section | Subsystem                    | File                              | Typical crates / paths                                                                 |
|---------|------------------------------|-----------------------------------|----------------------------------------------------------------------------------------|
| 01      | UI Widgets                   | `section-01-ui-widgets.md`        | `oriterm_ui/src/widgets/`                                                              |
| 02      | Settings Dialog              | `section-02-settings-dialog.md`   | `oriterm_ui/src/widgets/dialog/`, `oriterm/src/session/` settings                      |
| 03      | UI Framework                 | `section-03-ui-framework.md`      | `oriterm_ui/src/window_root/`, `interaction/`, `pipeline/`, `animation/`, `testing/`   |
| 04      | Fonts                        | `section-04-fonts.md`             | `oriterm/src/font/`, swash/skrifa glyph pipeline, UI font registry                     |
| 05      | Config                       | `section-05-config.md`            | `oriterm/src/config/`, TOML parse, hot reload                                          |
| 06      | Rendering & Perf             | `section-06-rendering-perf.md`    | `oriterm/src/gpu/`, compositor, atlas, cached render path, perf invariants             |
| 07      | CI & Build                   | `section-07-ci-build.md`          | `.github/`, `build-all.sh`, `test-all.sh`, `clippy-all.sh`, cross-compile              |
| 08      | Core Terminal                | `section-08-core-terminal.md`     | `oriterm_core/` — grid, VTE handler, reflow, selection, search, teseq / tack / vttest |
| 09      | Session & Tab/Window         | `section-09-session.md`           | `oriterm/src/session/`, split tree, floating layer, nav, layout compute                |
| 10      | Platform Windows             | `section-10-platform-windows.md`  | Any `#[cfg(windows)]` branch, ConPTY, `x86_64-pc-windows-gnu` cross-compile issues     |
| 11      | Mux & Pane I/O               | `section-11-mux.md`               | `oriterm_mux/`, IO thread, snapshot double-buffer, PTY, mux backend, IPC               |

If unclear, check the file path or ask. If it spans subsystems, file in the one where the **fix** belongs (not where the symptom appears). If a bug doesn't fit any existing section, create a new section file following the `section-NN-<topic>.md` naming convention and update `00-overview.md`'s Quick Reference table.

## Step T2: Check for Duplicates

Before adding, scan the target section file for existing bugs that match:

```
Read plans/bug-tracker/section-{NN}-*.md
```

If a duplicate exists, note it to the caller instead of adding a new entry.

## Step T3: Assign ID and Severity

**ID format:** `BUG-{section}-{ordinal}` — ordinal is the next sequential number in that section (count existing bugs + 1).

**Severity:**
- `critical` — blocks correctness in the subsystem, data corruption, crash
- `high` — wrong output, silent failure, should fix when touching adjacent code
- `medium` — edge case failure, workaround exists, fix opportunistically
- `low` — cosmetic, minor inconvenience, tracked for dedicated passes

## Step T4: Minimal Research

Do just enough to write a useful bug entry. DO NOT deep-dive — the code may change before the fix:

1. Confirm the bug exists (quick grep or test run if trivial)
2. Identify the approximate location (crate + file, not exact line)
3. Note any obvious repro (existing test file, or minimal repro steps)

## Step T5: Write the Bug Entry

Append to the `## Open Bugs` section of the target file:

```markdown
- [ ] `[BUG-{section}-{ordinal}][{severity}]` **{Short title}**
  Repro: {test file path or minimal repro steps}
  Subsystem: {crate/file path}
  Found: {YYYY-MM-DD} | Source: {source value from the canonical list below}
```

If a fix section already exists (from a prior `/fix-bug` that was interrupted), add a cross-ref:
```markdown
  Fix: `plans/bug-tracker/fix-BUG-{section}-{ordinal}.md`
```

**Canonical source values** (use exactly one — this is the SSOT for bug provenance):
- `tpr-review` — found by `/tpr-review` dual-source review
- `code-journey` — found by `/code-journey`
- `manual` — found by the user or during manual work
- `continue-roadmap` — found while working on the roadmap
- `review-work` — found by `/review-work`
- `fix-bug` — found during an active `/fix-bug` workflow (Phase 1 investigation, Phase 3 TDD, Phase 4 test-all, Phase 5 TPR/hygiene)
- `fix-next-bug` — found during `/fix-next-bug` autopilot iteration
- `impl-hygiene-review` — found by `/impl-hygiene-review`
- `review-bugs` — found during `/review-bugs` triage
- `independent-review` — found by `/independent-review`
- `design-pattern-review` — found by `/design-pattern-review`
- `improve-tooling` — found during a `/improve-tooling` retrospective

When filing a bug from a dual-source review, add reviewer provenance to the body (not the Source field): `Reviewer: codex`, `Reviewer: gemini`, or `Reviewers: codex + gemini (agreement)`.

## Step T6: Cross-Reference Check

Quick check: is there an active roadmap section or reroute plan touching this area?

```
Grep for the affected file/function in plans/roadmap/section-*.md and plans/*/section-*.md
```

If an active plan section covers this area, note it in the bug entry:
```markdown
  Note: Active work in roadmap section {NN} touches this area.
```

This is informational only — the bug still belongs in the bug-tracker (the plan may not cover this specific issue).

## Step T7: Confirm to Caller and Return

Report what was filed in a single concise block:

```
Filed (tracker): [BUG-{section}-{ordinal}][{severity}] {title}
  Section: {section name} (plans/bug-tracker/section-{NN}-*.md)
  Cross-ref: {any active plan sections, or "none"}
  Commit: the markdown change is UNSTAGED. Caller bundles into their next commit.
```

Then:

- Return to the caller's workflow immediately.
- Do NOT stop, wait for user input, or prompt for next action.
- Inline execution means the caller IS the one reading this output — they resume where they left off.

---

## Inline Path — Steps I1–I5

Use this path when Step 0 routed to inline (option 2, or `--inline <path>` was passed).

**Scope assumption:** the bug blocks the plan's stated goal. The fix is plan-owned, not tracker-owned. No `BUG-XX-NNN` ID is assigned — the plan subsection IS the tracking artifact.

## Step I1: Verify Plan Section and Identify Subsection ID

1. Verify the plan section file exists and is readable:
   ```
   Read <plan-section-path>
   ```
2. Parse the YAML frontmatter `section:` field — this is the section number (e.g. `"04"`).
3. Scan existing subsections (both the frontmatter `sections:` list AND body `## {id} — ...` headings) for existing `{section}.BLOCKER-N` entries.
4. Assign the next ordinal: `{section}.BLOCKER-{N}` where `N` = count of existing `BLOCKER-*` subsections in this section + 1.
5. If the plan section already has `status: complete` in frontmatter, STOP and report: "Plan section {path} is already complete — inlining a blocker would reopen a closed section. Caller should either re-route to tracker path or explicitly re-open the section first." Return without appending.

## Step I2: Minimal Research

Same as Step T4 — do just enough to write a useful subsection:

1. Confirm the bug exists
2. Identify approximate location (crate + file)
3. Note repro (existing test file, or minimal repro steps)
4. Optional blast-radius check: Grep for the affected function/type and note the caller count in the bug entry. No intelligence graph in this project — Grep/Glob is the SSOT.

## Step I3: Append Subsection to Plan Section File

**Two edits in the same file — both required:**

### I3a: Frontmatter `sections:` list entry

Append to the `sections:` list in the YAML frontmatter (preserving existing entries and order):

```yaml
  - id: "{section}.BLOCKER-{N}"
    title: "{Short blocker title}"
    status: not-started
    kind: plan-blocker-inline
    found: "{YYYY-MM-DD}"
    source: "{canonical source value from tracker-path Step T5 list}"
```

The `kind: plan-blocker-inline` field distinguishes inlined blockers from planned subsections — reviewers and `/fix-bug` branch on this.

### I3b: Body subsection

Append to the END of the file (AFTER all existing subsections):

```markdown
---

## {section}.BLOCKER-{N} — {Short blocker title}

**Kind:** plan-blocker-inline
**Status:** not-started
**Found:** {YYYY-MM-DD}
**Source:** {canonical source value}
**Severity:** {critical|high|medium|low}
**Blocks:** plan completion — rest of plan cannot reach stated goal without this fix

### Why This Blocks the Plan

{1-3 sentences — which plan goal / success criterion cannot be met without this fix.
Cite the specific frontmatter `goal:` clause or `success_criteria:` bullet this blocker invalidates.}

### 1. Root Cause Analysis (to be filled by /fix-bug Phase 1)

- **Symptom:** {what was observed — error, wrong output, crash}
- **Proximate cause:** {what code produced the wrong behavior — pending Phase 1}
- **Root cause:** {the architectural/logical flaw — pending Phase 1}
- **Blast radius:** {what else is affected — pending Phase 1}
- **Affected files:** {pending Phase 1 — approximate: {crate/path from Step I2}}

### 1.5 Fix Consensus (to be filled by /fix-bug Phase 1.75)

Pending — `/tp-help` consensus in Phase 1.75.

### 2. TDD Matrix (to be filled by /fix-bug Phase 2)

Pending — design after `/tp-help` consensus.

### 2.5 Fix Plan TPR (to be filled by /fix-bug Phase 2.5)

Pending — may be skipped per `/fix-bug` gate criteria (severity + subsystem complexity).

### 3. Implementation (to be filled by /fix-bug Phase 4)

Pending — approach finalized in Phase 2.

### R. TPR Findings (to be filled by /fix-bug Phase 5)

Pending — code review after implementation.

### N. Completion Checklist

- [ ] `/tp-help` consensus recorded in §1.5
- [ ] TDD matrix designed and recorded in §2 (with positive + negative pins)
- [ ] Plan TPR recorded in §2.5 (or marked Skipped with gate-criteria justification)
- [ ] Implementation complete; tests pass; `timeout 150 cargo test --all` green
- [ ] Code `/tpr-review` clean (§R)
- [ ] `/impl-hygiene-review` clean
- [ ] `/improve-tooling` retrospective recorded
- [ ] Capability regression gate passed (per `/fix-bug` Phase 4 step 6)
- [ ] Subsection marked `status: complete` in frontmatter `sections:` list
- [ ] `/sync-claude` doc sync clean
- [ ] Closure committed via `/commit-push`

### Repro

```
{test file path or minimal repro — from Step I2}
```

### Blast Radius (Intelligence Graph)

```
{pasted output from `callers` + `file-symbols` queries — from Step I2}
```

### Source

`{canonical source value}` — per `.claude/skills/add-bug/workflow.md` Step T5 enumeration.

{If from dual-source review, also:}
**Reviewer:** codex | gemini | codex + gemini (agreement)
```

## Step I4: Cross-Reference Check

Quick check for existing tracker bugs describing this same issue:

```
Grep for matching title/subsystem in plans/bug-tracker/section-*.md
```

If a tracker bug exists, add a cross-ref to the inline subsection body (above `### Source`):

```markdown
**Cross-ref:** `[BUG-{section}-{ordinal}]` — tracker entry describing same bug; close this tracker entry when the inline subsection completes (per CLAUDE.md §"Plan-Blocker Bugs Belong IN the Plan": PLAN SECTION is the fix section; tracker entry closes pointing to the subsection).
```

Do NOT modify the tracker entry in this step — the caller's `/fix-bug` phase will close it at Phase 5 (subsection closure). Noting the cross-ref in the subsection is sufficient.

## Step I5: Confirm to Caller and Return

Report what was inlined in a single concise block:

```
Inlined (plan blocker): {section}.BLOCKER-{N} — {title}
  Plan section: {plan-section-path}
  Severity: {severity}
  Cross-ref: {tracker BUG-XX-NNN if one exists, or "none"}
  Next step: invoke /fix-bug with an inline target to execute full rigor:
            `/fix-bug inline:{plan-section-path}#{section}.BLOCKER-{N}`
            (or shorthand `/fix-bug {plan-section-path}#{section}.BLOCKER-{N}`).
            /fix-bug runs Phase -1 through Phase 6 in-place against the
            subsection body — see /fix-bug SKILL.md §Inline Mode Phase Overrides.
  Commit: both edits (frontmatter + body) are UNSTAGED. Caller bundles into
          their next commit.
```

Then:

- Return to the caller's workflow immediately.
- Do NOT pause, do NOT prompt for next action.
- Inline execution means the caller IS the one reading this output — they resume where they left off.
