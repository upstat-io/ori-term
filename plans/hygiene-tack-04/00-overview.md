---
plan: "hygiene-tack-04"
title: "Hygiene Cleanup — tack-conformance Section 04 slice"
status: in-progress
disposable: true
---

# Hygiene Tack 04 — Overview

## Mission

Finish the implementation hygiene cleanup of the Section 04 tack-conformance scenario
framework slice (`ce305091..efec3818`). The 4 Critical SSOT/algorithmic LEAKs were
fixed inline in commit `0b0806f2` (extract `scenario_name`, `prepare_and_navigate`,
`finish_and_assert`; migrate the smoke test to `quit_tack`). 24 follow-up findings
remain — 8 Major + 16 Minor — surfaced by the same impl-hygiene review and listed in
`section-01-cleanup.md`.

This is a **disposable cleanup plan**. It exists only to track the remaining
findings until they're fixed. Once `section-01-cleanup.md` is fully checked, the
entire `plans/hygiene-tack-04/` directory must be deleted via the cleanup step.

## Source Review

- **Reviewer**: `/impl-hygiene-review last commit`
- **Slice**: `ce305091..efec3818` (Section 04 implementation + 5 TPR follow-up commits)
- **Files in scope**: 21 .rs files in `crates/oriterm_test_support/src/{session,tack_framework}/`
  and `oriterm_core/tests/tack/test_menu/`
- **TPR loop**: 6 iterations, converged clean (TPR-04-001 through TPR-04-008,
  all resolved before this plan was created)
- **Critical fixes already landed**: `0b0806f2 refactor(tack-conformance): impl-hygiene Critical fixes`

## Finding Categories

### Major (8 findings)

**Cluster A — runner/mod.rs polish** (3 findings):
- LEAK-04-05: `parse_modes_screen` re-derives `default_parser` header extraction
- LEAK-04-06: `5_000` timeout literal hardcoded 4× (PARTIALLY fixed in 0b0806f2 — 2
  named constants added; the partial completion is acknowledged in the cleanup tracker)
- LEAK-04-07: `"tack [n] >"` hardcoded 3× (PARTIALLY fixed in 0b0806f2 — runner sites
  use `TACK_MAIN_MENU_PROMPT` const; the smoke test still inlines the literal)

**Cluster B — Test helper duplication** (4 DRY findings — all resolve via one new
`crates/oriterm_test_support/src/test_helpers.rs` module):
- DRY-04-01: `spawn_quit_on_keystroke` duplicated byte-for-byte across `teardown/tests.rs` and `runner/tests.rs`
- DRY-04-02: `spawn_silent_long_lived` / `spawn_silent_child` do the same thing in two files
- DRY-04-03: panic-payload downcast block duplicated 3× (`sync/tests.rs`, `teardown/tests.rs`, `runner/tests.rs`)
- DRY-04-04: 9× `#[cfg(unix)] /bin/sh` / `#[cfg(windows)] cmd.exe` `CommandBuilder` boilerplate

**Cluster C — Drift + Gap** (2 findings):
- BND-04-01: `quit_tack` drain timeout drift — plan says 200ms, test comment says 200ms,
  impl uses 150ms. Either align all three or extract a named constant.
- BND-04-02: `LiveSession` has no `Drop` guard / no `#[must_use]` — cleanup contract
  is purely prose; a Section 07 caller can silently forget `finish()` and lose the
  exit-status assertion.

### Minor (16 findings)

**Sub-cluster D — Named constants** (5 findings):
- LEAK-04-08: `snapshot_name` could consume `size_label` but reimplements the sub-format
- LEAK-04-09: `drain_blocking(50)` and `drain_blocking(150)` unnamed in `poll_until` and `quit_tack`
- HYG-04-08: `wait(300)` quiesce in `send` unnamed
- HYG-04-09: `wait_for_child_exit(2_000)` Phase 2 timeout unnamed
- HYG-04-10: `drain_blocking(50)` inside `poll_until` unnamed

**Sub-cluster E — Boundary error handling** (3 findings):
- BND-04-03: `LiveSession::session` is `pub` — leaks PtySession internals through the wrapper
- BND-04-04: `send_raw` silently swallows write+flush errors with no `Result`
- BND-04-05: PTY reader thread silently drops read errors (no log)

**Sub-cluster F — Surface polish** (5 findings):
- HYG-04-01: `feed_and_flush` private helper in middle of public methods in `sync/mod.rs`
- HYG-04-02: `spec.rs` missing `//!` module doc
- HYG-04-03: `parse_modes_screen` missing `#[must_use]`
- HYG-04-04: `LiveSession::finish` missing `#[must_use]`
- HYG-04-07: module doc in `session/sync/mod.rs` cross-references a plan file that
  will be archived (fragile)

**Sub-cluster G — Performance** (1 finding):
- HYG-04-05: `grid_text()` double-allocates `Vec<Vec<char>>` + per-row `String` per
  poll iteration. Test-suite wall-clock impact only, not production hot path.

**Sub-cluster H — Cross-suite duplication followup** (2 informational):
- DRY-04-05: bounded-poll wall-clock assertion shape duplicated 3× (intentional —
  per-consumer pins are load-bearing for the hygiene contract; OPTIONAL extraction)

## Mission Success Criteria

- [ ] All 24 hygiene findings in `section-01-cleanup.md` are `[x]` resolved or
      have an audit-trail rejection note explaining why they're factually incorrect
- [ ] `cargo test -p oriterm_test_support` green
- [ ] `cargo test -p oriterm_core --test tack` green
- [ ] `cargo clippy -p oriterm_test_support --all-targets -- -D warnings` green
- [ ] `./build-all.sh` green (debug + release cross-compile)
- [ ] `./clippy-all.sh` green
- [ ] `./test-all.sh` green
- [ ] The `plans/hygiene-tack-04/` directory has been deleted via the section-01
      cleanup step

## Quick Reference

| ID | Title | File | Status |
|----|-------|------|--------|
| 01 | Cleanup | `section-01-cleanup.md` | Not Started |
