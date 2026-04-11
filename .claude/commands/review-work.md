---
name: review-work
description: Review actual implementation work, not just a plan. Use when the user asks to review work done across committed history, staged changes, unstaged changes, or a plan section; perform a deep investigation against `CLAUDE.md`, all repo rule files, and recently modified plans, then record validated findings in the owning plan section when one exists.
allowed-tools: Read, Grep, Glob, Bash, Task, Edit, Write
---

# Review Work Command

Review the implementation first. Treat git history, the index, the worktree, current files, `CLAUDE.md`, `.claude/rules/*.md`, and recent plan files as the evidence set. A plan is a coordination artifact, not the sole source of scope.

This command is for independent, adversarial review:
- Trust current files, fresh command output, and git objects.
- Distrust summaries, checklists, commit messages, and prior agent claims until verified.
- Review the real work, not the story about the work.

## Usage

```
/review-work [target]
```

`target` can be:
- A plan directory or section file (e.g., `plans/tack-conformance/`, `plans/tack-conformance/section-09-verification.md`)
- A section ID or keywords (e.g., `section-09`, `09`, `tack verification`)
- A git range or commit selector (e.g., `HEAD~5..HEAD`, `abc123..def456`, `last commit`)
- Uncommitted work selectors (`staged`, `unstaged`, `worktree`, `current branch`)
- Explicit files or directories

If no target is provided, it defaults to `HEAD~3..HEAD` plus any staged/unstaged changes.

---

## Workflow

### Step 1: Resolve Scope

Resolve the target in this order:
1. Existing path from the user.
2. Explicit git range or commit selector.
3. Explicit uncommitted-work selector (`staged`, `unstaged`, `worktree`, `current branch`).
4. Plan match from `plans/*/index.md`, `plans/*/00-overview.md`, and `plans/*/section-*.md`.
5. If nothing explicit was given, start with a recent committed slice plus any uncommitted work:
   - committed changes from `HEAD~3..HEAD`
   - staged changes from `git diff --cached`
   - unstaged changes from `git diff`

Broaden the scope if it's too narrow to be coherent (e.g., if it's just a fixup for previous commits).

### Step 2: Gather Evidence

#### 2.1 Git Evidence
Collect whichever apply:
- Committed diff stat and patch for the range.
- Commit log.
- Staged/unstaged diffs.
- `git status --short`.

#### 2.2 File Inventory
Identify all changed files, tests that should cover them, and adjacent code needed to understand behavior. **Read the full changed files, not just diff hunks.**

#### 2.3 Standards Packet
Read (every relevant file, not a summary):
- `CLAUDE.md` (project root)
- `.claude/rules/impl-hygiene.md` — SSOT, No Side Logic, finding categories (LEAK / DRIFT / GAP / WASTE / EXPOSURE / BLOAT / NOTE)
- `.claude/rules/tests.md` — matrix testing rule, interaction testing, cross-platform verification, graceful skip protocol, performance invariants
- `.claude/rules/code-hygiene.md` — file size, module organization, error handling, function size
- `.claude/rules/test-organization.md` — sibling `tests.rs` pattern
- `.claude/rules/crate-boundaries.md` — ownership and allowed dependency direction
- Per-crate rule files under `.claude/rules/oriterm*.md` (e.g. `oriterm_ui.md`, `oriterm_core.md`, `oriterm_mux.md`, `oriterm_ipc.md`, `oriterm.md`) — read every file whose `paths:` glob matches a file under review
- Any other `.claude/rules/*.md` whose `paths:` glob matches changed files

#### 2.4 Plan Context
Gather recently modified plans to detect plan drift:
1. Check `plans/` for recent changes in git.
2. If a plan or section was named, read its `index.md`, `00-overview.md`, and target section.
3. Check `plans/bug-tracker/` for any in-progress bug fix sections that might cover the same code.

### Step 3: Review Implementation

Perform an independent verification pass:
- Rerun key tests with the mandatory timeout: `timeout 150 cargo test -p <crate>`, `timeout 150 ./test-all.sh` (never run a test command without a timeout per CLAUDE.md §MANDATORY TEST TIMEOUTS).
- Use repo-native diagnostic / test scripts — `./build-all.sh` for cross-compile verification, `./clippy-all.sh` for lint regressions, `cargo test -p oriterm_core --test teseq` / `--test tack` / `--test vttest` for terminal-conformance regressions, `cargo test -p oriterm_ui` for widget harness regressions.
- Verify claims from current outputs and files — do not trust prior agent summaries.
- For GPU render path changes, verify under `oriterm/src/gpu/visual_regression/` with `render_frame_cached()`; bugs in the cached path are invisible to `render_frame()`.
- For allocation / performance claims, run `oriterm_core/tests/alloc_regression.rs` and `oriterm_core/tests/rss_regression.rs`.

Review for:
- Correctness bugs and regressions (grid mutation, VTE handler, reflow, selection, search, damage tracking).
- Memory / resource ownership issues (Arc cloning in hot paths, GPU resource leaks, buffer shrink discipline, unbounded growth vectors).
- Unsafe / FFI hazards (`unsafe_code = "deny"` — any `unsafe` block needs justification, and platform FFI must have counterparts on every supported target).
- Missing or weak tests (matrix coverage, semantic pins, negative pins, graceful skip protocol).
- Cross-platform drift (`#[cfg(target_os = ...)]` blocks missing a branch for Linux / macOS / Windows).
- Rule violations (`CLAUDE.md`, `.claude/rules/*.md`, per-crate rules files).
- Hygiene problems (file size > 500 lines, inline test modules, `println!` debugging, `Arc` in hot paths).
- Performance invariant violations (zero idle CPU, zero allocations in hot render path, stable RSS, buffer shrink discipline).
- Plan / implementation drift.

### Step 4: Record Findings

1. Report findings to the user, ordered by severity.
2. **If an owning plan section exists**, record validated findings in that section's `Third Party Review Findings` block using TPR format.
3. **If NO owning plan section exists** (completed plan, cross-cutting issue, or orphan finding), file validated findings as bugs in `plans/bug-tracker/` using `/add-bug` format.

#### Finding Format (plan-owned):
```md
- [ ] `[TPR-{section}-{ordinal}][{severity}]` `file:line` — Short finding summary.
  Evidence: Explain the specific mismatch, regression, or missing case.
  Impact: Explain why the work is incomplete, unsafe, or non-compliant.
  Required plan update: State what must be validated and integrated.
```

#### Finding Format (bug-tracker fallback):
```md
- [ ] `[BUG-{section}-{ordinal}][{severity}]` **{Short title}** — found by review-work.
  Repro: {test file or minimal repro steps}
  Subsystem: {crate/file path}
  Found: {YYYY-MM-DD} | Source: review-work
```

Map findings to `plans/bug-tracker/` subsystem sections by crate ownership (see `.claude/rules/crate-boundaries.md` for authoritative ownership):
- `oriterm_core` (grid, VTE, cell, palette, selection, search, terminfo conformance) → section covering terminal emulation bugs
- `oriterm_ui` (widgets, WindowRoot, interaction, pipeline, animation, test harness) → section covering UI framework bugs
- `oriterm_mux` (pane lifecycle, IO thread, snapshot buffer, PTY, mux backend) → section covering pane server bugs
- `oriterm_ipc` (Unix sockets, Windows named pipes, mio integration) → section covering IPC bugs
- `oriterm` (app shell, winit event loop, GPU rendering, session model, font pipeline, config) → section covering app-shell bugs
- `crates/oriterm_test_support` (test helpers) → section covering test support bugs
- `crates/portable-pty`, `crates/vte`, `crates/wgpu-hal` (vendored patches) → treat as external; route findings to the upstream subsystem that depends on the patched crate
- `docs/`, `.claude/`, `plans/` → documentation / tooling / plan section

Check `plans/bug-tracker/00-overview.md` for the current section mapping if multiple subsystems could apply. If no specific section exists, file under the closest existing one and note the subsystem explicitly in the Subsystem: field.

Severities: `high`, `medium`, `low`.

### Step 5: Update Plan Metadata

If findings are added to a plan section (TPR format):
- Set section frontmatter `status: in-progress`.
- Set `third_party_review.status: findings`.
- Set `third_party_review.updated` to today's date.
- If the plan overview/index was marked complete/resolved, set it back to in-progress/active.

If findings are added to the bug-tracker (fallback):
- No metadata changes needed — the bug-tracker is always open.

---

## Mandatory Standards Checks

Every review must explicitly test the work against:
- **TDD for Bugs**: Bug fixes must have matrix tests, semantic pins, and negative pins per `.claude/rules/tests.md` §Matrix Testing Rule.
- **Fix Completeness**: No workarounds, hacks, or "temporary" fixes per CLAUDE.md §NO WORKAROUNDS.
- **Code Hygiene**: Sibling `tests.rs`, file size ≤ 500 lines, `log` macros instead of `println!`, no `unwrap()` in library code, `#[deny(clippy::all)]` clean.
- **Crate Boundaries**: Changes respect the allowed dependency direction and per-crate ownership per `.claude/rules/crate-boundaries.md`.
- **Performance Invariants**: Zero idle CPU beyond cursor blink, zero allocations in the hot render path, stable RSS under sustained output, buffer shrink discipline. Regression tests in `oriterm_core/tests/alloc_regression.rs` and `oriterm/src/app/event_loop_helpers/tests.rs` still green.
- **Cross-Platform**: Every `#[cfg(target_os = ...)]` has counterparts for all three supported targets. Windows cross-compile (`cargo build --target x86_64-pc-windows-gnu`) green.
- **GPU Render Path**: Tests use `render_frame_cached()` not `render_frame()` when the change touches the content-cached path.
- **Reference Repo Consultation**: For VT / terminfo / widget / GPU work, the fix was compared against the corresponding reference implementation in `~/projects/reference_repos/console_repos/` (tmux, alacritty, wezterm, ghostty, ratatui, ptyxis, termenv).

## Output Pattern

1. List findings first, ordered by severity, with file references.
2. State the reviewed scope (commit range, diffs, major files).
3. State which standards were checked.
4. State whether a plan section was updated.
5. Mention any verification gaps.
