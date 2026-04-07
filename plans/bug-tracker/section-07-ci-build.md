---
section: "07"
title: "CI & Build Bugs"
status: in-progress
reviewed: true
goal: "Track and fix bugs in CI workflows, release automation, and build scripts"
depends_on: []
third_party_review:
  status: resolved
  updated: 2026-03-30
sections:
  - id: "07.1"
    title: "Active Bugs"
    status: not-started
  - id: "07.R"
    title: "Third Party Review Findings"
    status: complete
---

# Section 07: CI & Build Bugs

**Status:** Not Started
**Goal:** Track and fix bugs in CI workflows, release automation, and build scripts.

**Note:** This section is never marked complete. New bugs are appended as discovered.

---

## 07.1 Active Bugs

- [ ] `[BUG-07-004][medium]` **Windows PTY size propagation test removed** — found by tpr-review.
  Repro: `#[cfg(unix)]` gate on `pty_size_is_propagated` test means Windows CI has zero PTY size coverage. ConPTY-size regressions can now slip through unchecked.
  Subsystem: `oriterm_core/tests/vttest.rs:226`
  Found: 2026-04-02 | Source: tpr-review
  Fix: Add a Windows-specific PTY size test using ConPTY (not `stty`), or use a cross-platform approach that works on both Unix and Windows.

- [ ] `[BUG-07-005][medium]` **`./clippy-all.sh` does not lint test targets — 11 pre-existing clippy violations in `oriterm_core/tests/vttest/`** — found by continue-roadmap.
  Repro: `cargo clippy -p oriterm_core --test vttest -- -D warnings` produces 11 errors. `./clippy-all.sh` runs `cargo clippy --workspace -- -D warnings` which only checks lib + bin targets, so test-target violations have been silently passing CI.
  Subsystem: `clippy-all.sh` + `oriterm_core/tests/vttest/menu*.rs`
  Found: 2026-04-07 | Source: continue-roadmap
  Locations:
  - `oriterm_core/tests/vttest/menu1.rs:107:14`, `124:14`, `133:14` — `needless_range_loop` (3×)
  - `oriterm_core/tests/vttest/menu2.rs:49:26` — `string_slice`
  - `oriterm_core/tests/vttest/menu4.rs:4:38` — `doc_markdown`
  - `oriterm_core/tests/vttest/menu5.rs:14:5` — doc list item without indentation
  - `oriterm_core/tests/vttest/menu6.rs:11:14`, `11:28` — `doc_markdown` (2×)
  - `oriterm_core/tests/vttest/menu7.rs:3:55` — `doc_markdown`
  - `oriterm_core/tests/vttest/menu8.rs:11:1` — `too_many_lines` (124/100)
  - `oriterm_core/tests/vttest/menu8.rs:12:39` — `redundant_closure_for_method_calls`
  Fix: (1) update each violation site, and (2) add `--all-targets` to `./clippy-all.sh` so test-target lints are gated by CI going forward. None caused by tack-conformance section 01.3 PtySession migration — verified by reading the diffs against violation lines.
  Note: Active work in tack-conformance section 01 touches `oriterm_core/tests/vttest/session.rs` and the menu*.rs imports, but does not modify the lines flagged above.

---

## 07.R Third Party Review Findings

- [x] `[BUG-07-001][high]` Auto-release one per UTC day limit.
  **Fixed 2026-03-30.** `bump-build.sh` now appends a sequence number (`.2`, `.3`, ...) when the current BUILD_NUMBER already has today's date. Format: `0.2.0-alpha.YYYYMMDD[.N]`.

- [x] `[BUG-07-002][high]` `release.yml` no longer validates tag matches workspace version.
  **Fixed 2026-03-30.** Added "Verify tag matches workspace version" step in `release.yml` that extracts the version from Cargo.toml and compares it to `github.ref_name`. Fails with a clear error if they don't match.

- [x] `[BUG-07-003][medium]` Auto-release `Cargo.lock` stale after version bump.
  **Fixed 2026-03-30.** Added `cargo generate-lockfile` step in `auto-release.yml` after `sync-version.sh` and before `git add`. Also added Rust toolchain installation step since `cargo` is needed.

---
