---
name: add-bug
description: Add a bug to the bug-tracker plan. Minimal research at add-time — capture repro, location, severity, and source. TRIGGER proactively when ANY bug is encountered during ANY work — unrelated bugs, edge cases, test failures, suspicious behavior, code smells that look like bugs. If in doubt, file it. Better safe than sorry — verification happens at review time.
allowed-tools: Read, Grep, Glob, Edit, Write, Bash
argument-hint: "[description or file:line]"
---

# Add Bug

File a bug in `plans/bug-tracker/` under the correct subsystem section.

## Proactive Triggering — MANDATORY

This skill MUST be invoked proactively whenever you encounter a bug that is **not part of your current task**. Do NOT:
- Gloss over it as "not related"
- Note it mentally and move on
- Say "this is a separate issue" without filing
- Assume someone else will catch it
- Skip it because you're "in the middle of something"

**If in doubt, file it.** Verification happens when bugs are reviewed (`/review-bugs`). A false positive costs nothing; a missed bug costs everything.

### When to trigger (non-exhaustive)
- You see a test failure unrelated to your current work
- You notice suspicious behavior while reading code
- A code journey or exploration reveals unexpected output
- You encounter an edge case that probably doesn't work
- You find a TODO/FIXME/HACK comment that describes an unfixed bug
- A rendered frame, widget paint, or escape-sequence handler produces the wrong output
- A platform-specific `#[cfg(target_os = ...)]` branch is missing on one of the supported targets
- Any test is `#[ignore]`-d and the reason looks fixable
- A performance invariant (zero idle CPU, zero allocations in hot render path, stable RSS) appears to be violated

## Usage

```
/add-bug [description]
```

The description can be:
- A free-text bug description: `/add-bug cursor blink keeps frame budget gate open when no other animation is active`
- A file reference: `/add-bug oriterm/src/gpu/window_renderer/render.rs:218 — copy extent mismatch during resize`
- Context from the current conversation (no args needed if a bug was just discussed)

## Workflow

### Step 1: Determine Subsystem

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

### Step 2: Check for Duplicates

Before adding, scan the target section file for existing bugs that match:

```
Read plans/bug-tracker/section-{NN}-*.md
```

If a duplicate exists, note it to the user instead of adding a new entry.

### Step 3: Assign ID and Severity

**ID format:** `BUG-{section}-{ordinal}` — ordinal is the next sequential number in that section (count existing bugs + 1).

**Severity:**
- `critical` — blocks correctness in the subsystem, data corruption, crash
- `high` — wrong output, silent failure, should fix when touching adjacent code
- `medium` — edge case failure, workaround exists, fix opportunistically
- `low` — cosmetic, minor inconvenience, tracked for dedicated passes

### Step 4: Minimal Research

Do just enough to write a useful bug entry. DO NOT deep-dive — the code may change before the fix:

1. Confirm the bug exists (quick grep or test run if trivial)
2. Identify the approximate location (crate + file, not exact line)
3. Note any obvious repro (existing test file, or 2-3 line Ori snippet)
4. Intelligence graph blast-radius check. Follow the canonical intel-summary injection protocol:

   @.claude/skills/dual-tpr/compose-intel-summary.md

   Per SSOT Step F — /add-bug uses `callers "<buggy function>" --repo ori` to assess blast radius and `file-symbols "<subsystem path>" --repo ori` to identify related code.

### Step 5: Write the Bug Entry

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

When filing a bug from a dual-source review, add reviewer provenance to the body (not the Source field): `Reviewer: codex`, `Reviewer: gemini`, or `Reviewers: codex + gemini (agreement)`.

### Step 6: Cross-Reference Check

Quick check: is there an active roadmap section or reroute plan touching this area?

```
Grep for the affected file/function in plans/roadmap/section-*.md and plans/*/section-*.md
```

If an active plan section covers this area, note it in the bug entry:
```markdown
  Note: Active work in roadmap section {NN} touches this area.
```

This is informational only — the bug still belongs in the bug-tracker (the plan may not cover this specific issue).

### Step 7: Confirm to User

Report what was filed:
```
Filed: [BUG-{section}-{ordinal}][{severity}] {title}
  Section: {section name} (plans/bug-tracker/section-{NN}-*.md)
  Cross-ref: {any active plan sections, or "none"}
```

### Step 8: Resume Prior Workflow — MANDATORY

**`/add-bug` is almost always invoked mid-task** (proactive filing during `/continue-roadmap`, `/tpr-review`, `/fix-bug`, etc.). After confirming the filing, **immediately resume the interrupted workflow.** Do NOT stop, wait for user input, or present the filing as a standalone deliverable. The bug filing is a side-effect — the main task is still in progress.

If you were in the middle of:
- `/tpr-review` → continue fixing findings or re-running the transport
- `/continue-roadmap` → continue implementing the current subsection
- `/fix-bug` → continue with the current phase
- Any other workflow → pick up exactly where you left off

The user should not need to prompt you to continue.

## Fix Workflow — What Happens Next

Filing a bug is capture only. When a bug is picked up for fixing (via `/review-bugs`, `/continue-roadmap`, or direct request), the **`/fix-bug`** command enforces plan-section rigor:

1. **Investigation** — root cause analysis, reference-repo consultation (tmux / alacritty / wezterm / ghostty / ratatui / ptyxis / termenv under `~/projects/reference_repos/console_repos/`), protocol spec cross-check (vt100.net, XTerm ctlseqs, ECMA-48, terminfo)
2. **Fix section file** — `plans/bug-tracker/fix-BUG-{section}-{ordinal}.md` created with full plan-section structure
3. **TDD matrix** — all tests written and verified failing BEFORE implementation
4. **Implementation** — fix applied, tests pass unchanged
5. **Completion checklist** — test-all, clippy-all, TPR review, impl-hygiene review

**Every bug fix gets this rigor.** No ad-hoc fixes. The fix section file is the permanent record of investigation, approach, and verification — it stays in the bug tracker alongside the section files.

See `/fix-bug` for the full workflow and fix section template.
