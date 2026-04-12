# ori_term

GPU-accelerated terminal emulator in Rust (same category as Alacritty, WezTerm, Ghostty). Opens a native frameless window, renders a terminal grid via wgpu, runs shell processes through ConPTY/PTY.

**Cross-platform: macOS, Windows, and Linux.** All code must compile and run correctly on all three platforms. Never write platform-specific code without corresponding implementations for the other two. Every `#[cfg(target_os = "...")]` block must have counterparts for all supported targets — no platform left behind. If a feature cannot be implemented on a platform, it must degrade gracefully with a compile-time `cfg` gate, not a runtime panic. CI builds and tests on all three. Local dev cross-compiles from WSL targeting `x86_64-pc-windows-gnu`.

**Broken Window Policy**: Fix EVERY issue you encounter — no exceptions. Never say "this is pre-existing", "this is unrelated", or "outside the scope". If you see it, you own it. Leaving broken code because "it was already broken" is explicitly forbidden.

**Do it properly, not just simply. Correct architecture over quick hacks; no shortcuts or "good enough" solutions.**

**NO WORKAROUNDS. NO HACKS. NO SHORTCUTS.**
- **Proper fixes only** — If a fix feels hacky, it IS hacky. Find the right solution.
- **When unsure, STOP and ASK** — Do not guess. Do not assume. Pause and ask the user for guidance.
- **Fact-check everything** — Verify behavior against reference implementations. Test your assumptions. Read the code you're modifying.
- **Consult reference repos** — Check `~/projects/reference_repos/console_repos/` for established patterns and idioms.
- **No "temporary" fixes** — There is no such thing. Today's temporary fix is tomorrow's permanent tech debt.
- **If you can't do it right, say so** — Communicate blockers rather than shipping bad code.

---

## Bug Discipline — `/add-bug`, `/fix-bug`, `/fix-next-bug`

Every bug discovered, anywhere, at any time, MUST get a concrete tracked artifact immediately. No mental notes, no "I'll remember", no comments-only. The two valid responses are:

- **Blocking or critical/high:** fix it NOW using `/fix-bug` (creates a fix section file with plan-section rigor: root cause analysis, TDD matrix, completion checklist including TPR + hygiene review). The discovery IS the assignment.
- **Non-blocking medium/low or unrelated to current task:** file it NOW using `/add-bug` (creates a tracked `- [ ]` entry in `plans/bug-tracker/` with repro, subsystem, severity). Filing via `/add-bug` is NOT deferral — it creates a concrete artifact that `/review-bugs` and `/fix-next-bug` will pick up. Deferral is when a bug has no artifact at all.

No "tracked for later" (without an artifact), no "known issue" (without filing), no "pre-existing" (as justification for skipping). **Pre-existing bugs MUST be tracked immediately** — "pre-existing" is diagnosis only, never justification for ignoring.

- **Proactive bug filing with `/add-bug`** — when you encounter ANY bug not related to your current task, invoke `/add-bug` immediately. Do NOT gloss over it as "not related", note it mentally and move on, or say "separate issue" without filing. If in doubt, file it — verification happens at `/review-bugs` time. A false positive costs nothing; a missed bug costs everything. Triggers: unrelated test failures, suspicious behavior, rendering glitches, broken layouts, wrong widget output, fixable TODO/FIXME comments describing unfixed bugs, platform-specific code paths that look broken or incomplete.
- **Bug fix rigor with `/fix-bug`** — when fixing ANY bug (whether from the bug tracker, discovered during plan work, or surfaced by TPR), use `/fix-bug BUG-XX-NNN`. This creates a fix section file (`plans/bug-tracker/fix-BUG-XX-NNN.md`) with plan-section rigor: investigation, root cause analysis, TDD matrix (semantic + negative pins), implementation, completion checklist (test-all, build-all, TPR, hygiene review). No ad-hoc bug fixes — every bug gets a fix section, even "obvious" ones. The fix section is the permanent record of investigation and verification. `/fix-bug` also has a `--autopilot` flag used by `/fix-next-bug` for fully autonomous loop runs.
- **Drain the queue with `/fix-next-bug`** — when you want to work down the bug tracker, use `/fix-next-bug`. It auto-picks the highest priority open bug and invokes `/fix-bug` with full rigor. It supports two modes: **interactive** (one bug at a time, prompt between bugs) and **autopilot** (fix every open bug end-to-end with zero interaction until the queue is empty). Autopilot still runs the full `/fix-bug` workflow per bug — investigation, TDD, implementation, TPR, hygiene — no shortcuts.
- **NEVER reason out of TPR findings** — when `/tpr-review` or `/review-work` surfaces a finding, the ONLY valid responses are: (1) fix it NOW, or (2) create a concrete implementation plan and execute it. You are NEVER permitted to dismiss findings as "pre-existing", "architectural limitation", "out of scope", "conservative/safe", "not a regression", or "future improvement". Marking a finding as resolved with a scope note or rationalization is DEFERRAL. The size of the fix is irrelevant — if the correct fix requires cross-crate refactoring, that IS the work. If genuinely blocked (need user decision, missing domain knowledge), use `AskUserQuestion` immediately.
- **Flaky tests ARE bugs** — if a test passes sometimes and fails sometimes, that is a bug — not noise. Do NOT retry and move on. Research the root cause (race condition, timing dependency, temp file collision, state leakage, non-deterministic ordering, GPU device-loss timing, surface reconfiguration races) and fix it so the test is deterministic. File via `/add-bug` if discovered during a different fix.
- **NEVER investigate "pre-existing?"** — do NOT use `git checkout`, `git stash`, `git bisect`, `git log --diff-filter`, or any git archaeology to determine whether a bug or test failure existed before your changes. **It does not matter.** The question "was this pre-existing?" is banned. The only valid question is: "is it fixed?" Spending time checking out old commits to see if something "was already broken" produces zero value. It's broken now → fix it now. The timeline is irrelevant. The fix is everything.
- **Fix interference = reorder, don't skip** — when fixing Bug A causes Bug B to surface (new failures that weren't in the original test run), this is INTERFERENCE, not a "pre-existing issue to ignore." The correct response: (1) revert or shelve Bug A's fix, (2) fix Bug B first using `/fix-bug` (it's now a dependency — full plan-section rigor applies), (3) re-apply Bug A's fix on top of Bug B's fix. Do NOT declare Bug A "fixed" when Bug B is interfering — that's shipping a regression.

---

## Coding Standards & Testing

Canonical homes for project conventions (read these before writing code):

- **`.claude/rules/code-hygiene.md`** — file organization (500-line limit, submodule discipline, single-responsibility, banner policy), error handling, `unsafe_code = "deny"`, clippy configuration, formatting, public API discipline, function size.
- **`.claude/rules/test-organization.md`** — sibling `tests.rs` pattern, inline-test-module ban, import style.
- **`.claude/rules/tests.md`** — TDD for bugs, matrix testing, interaction testing, cross-platform verification, performance invariants, flaky tests are bugs, mandatory 150s test timeout.
- **`.claude/rules/impl-hygiene.md`** — SSOT / No Side Logic / canonical homes / finding categories (LEAK / DRIFT / GAP / WASTE / EXPOSURE / BLOAT / NOTE), phase boundaries, algorithmic DRY, test-function naming.
- **`.claude/rules/crate-boundaries.md`** — per-crate ownership and allowed dependency direction.
- **Per-crate rules** — `.claude/rules/oriterm_core.md`, `oriterm_ui.md`, `oriterm_mux.md`, `oriterm_ipc.md`, `oriterm.md`. Each file's `paths:` glob scopes it to the owning crate's source tree.

**The coding-standards rules drawn from Alacritty, WezTerm, Ghostty, Ptyxis, Ratatui, Crossterm, Bubbletea, Lipgloss, Termenv live in `code-hygiene.md` — this file no longer restates them.**

---

## Commands

**Primary**: `./fmt-all.sh`, `./clippy-all.sh`, `./build-all.sh`, `./test-all.sh`
**Build**: `cargo build --target x86_64-pc-windows-gnu` (debug), `cargo build --target x86_64-pc-windows-gnu --release` (release)
**Teseq scenarios**: `cargo test -p oriterm_core --test teseq` (requires `reseq` — `sudo apt install teseq` on Linux; tests skip gracefully on macOS/Windows where teseq is unavailable)
**Update teseq snapshots**: `INSTA_UPDATE=1 cargo test -p oriterm_core --test teseq`
**After EVERY change, run `./build-all.sh`, `./clippy-all.sh`, and `./test-all.sh`. No exceptions. Do not skip any of these.**

---

## Workspace Layout

The workspace has 5 primary member crates plus `crates/oriterm_test_support` (test helpers) and three vendored dependency patches (`crates/vte`, `crates/portable-pty`, `crates/wgpu-hal`). Per-crate ownership and the allowed dependency direction live in `.claude/rules/crate-boundaries.md` and in the per-crate rules files under `.claude/rules/oriterm*.md`.

- `oriterm_core` — terminal emulation library (grid, VTE, cell, palette, selection, search). Standalone.
- `oriterm_ui` — UI framework (widgets, WindowRoot, interaction, pipeline, animation, test harness). Depends on `oriterm_core` only.
- `oriterm_mux` — pane server (PTY I/O, pane lifecycle, mux backend, snapshot double-buffer). Depends on `oriterm_core` + `oriterm_ipc`.
- `oriterm_ipc` — platform IPC transport (Unix sockets, Windows named pipes). Standalone.
- `oriterm` — application shell (winit event loop, GPU, font pipeline, session model, session/split-tree/floating/nav). Consumes all of the above.

**Litmus test:** Can this code be tested in a `#[test]` without a GPU, display server, or terminal? If yes → `oriterm_ui`. If no → `oriterm`. See `.claude/rules/crate-boundaries.md` for the full litmus test and per-crate ownership table.

To find the authoritative path for a type or module, use `cargo metadata` or `cargo test -p <crate>` rather than a hardcoded path table here — paths drift, `cargo metadata` does not.

## Reference Repos

The reference implementations this project compares itself against live at `~/projects/reference_repos/` on the developer machine. They are **not a dependency** — they are a precondition for design decisions, and missing them should produce a clear error (not a silent fallback).

- **Terminal emulators / multiplexers** (`~/projects/reference_repos/console_repos/`): tmux, alacritty, wezterm, ghostty, ratatui, crossterm, bubbletea, lipgloss, ptyxis, termenv, notcurses. Consult these for VT parsing, grid storage, reflow, selection, color detection, resize handling, RAII cleanup, and protocol conformance.
- **GUI / widget frameworks** (`~/projects/reference_repos/gui_repos/`): egui, iced, zed/GPUI, druid, masonry, makepad. Consult these for widget composition, hit testing, pipeline orchestration, layout, focus management.
- **Chromium UI subset** (`~/projects/reference_repos/chromium_ui/` — `ui/aura/`, `ui/gfx/geometry/`): architectural reference for the `oriterm_ui` layer (Rect = Point + Size, half-open intervals, epsilon-clamped SizeF, `GetNonClientComponent` hit testing, `WindowTargeter` strategy).
- **Font pipeline** (`~/projects/reference_repos/swash/`, `~/projects/reference_repos/fontations/`): swash v0.2.6 + skrifa for pure-Rust rasterization with hinting.

Each reference repo has its own subsystem-specific strengths — the per-crate rules files under `.claude/rules/oriterm*.md` cite the relevant one when discussing a subsystem convention.

---

## Plans

Implementation plans live in `plans/`. Each plan is a directory with an `index.md`, `00-overview.md`, and numbered section files (`section-01-*.md`, `section-02-*.md`, etc.).

When the user says **"continue plan X"** or **"resume plan X"** or **"pick up plan X"**:
1. Look in `plans/` for a directory matching the name (fuzzy match — "threading" matches `threaded-pty`, "font" matches `font-rendering`, etc.).
2. Read `00-overview.md` for the full context and mandate.
3. Read each `section-*.md` to find the first section with `status: not-started` or `status: in-progress`.
4. Resume work from that section.
5. **After completing each section**, update the plan files: set YAML status to `complete`, check checkboxes, update `index.md`, and record any deviations.

Plans are the source of truth for multi-session work. Keep them in sync with reality.

**Review Gate:** Every roadmap section has `reviewed: true/false` in its frontmatter. Sections with `reviewed: false` have NOT been vetted by `/review-plan` and must not be implemented without review. `/continue-roadmap` enforces this gate automatically — it will stop and warn before working on an unreviewed section.

---

## Current State

See [plans/roadmap/](plans/roadmap/) — the roadmap is the current state. 28 sections, 8 tiers. Use `/continue-roadmap` to resume work. Old prototype in `_old/` for reference.
