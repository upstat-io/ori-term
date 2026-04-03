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

---

## 07.R Third Party Review Findings

- [x] `[BUG-07-001][high]` Auto-release one per UTC day limit.
  **Fixed 2026-03-30.** `bump-build.sh` now appends a sequence number (`.2`, `.3`, ...) when the current BUILD_NUMBER already has today's date. Format: `0.2.0-alpha.YYYYMMDD[.N]`.

- [x] `[BUG-07-002][high]` `release.yml` no longer validates tag matches workspace version.
  **Fixed 2026-03-30.** Added "Verify tag matches workspace version" step in `release.yml` that extracts the version from Cargo.toml and compares it to `github.ref_name`. Fails with a clear error if they don't match.

- [x] `[BUG-07-003][medium]` Auto-release `Cargo.lock` stale after version bump.
  **Fixed 2026-03-30.** Added `cargo generate-lockfile` step in `auto-release.yml` after `sync-version.sh` and before `git add`. Also added Rust toolchain installation step since `cargo` is needed.

---
