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

- [ ] `[BUG-07-007][medium]` **vttest screen-walker scaffold duplicated across 13+ functions in two crates** — found by impl-hygiene-review (tack-conformance section 01.N).
  Repro: read `oriterm/src/gpu/visual_regression/vttest/mod.rs:23-121` (run_menu1_golden + run_menu2_golden), `oriterm/src/gpu/visual_regression/vttest/menus_3_8.rs` (run_menu3/4/6/7/8_golden, 5 functions), and `oriterm_core/tests/vttest/menu1.rs` through `menu8.rs` (run_menuN_X functions, 8+ functions). Each function shares the SAME control-flow skeleton: `headless_env()` (GPU side only) → `PtySession::spawn_vttest(cols, rows)` → `wait_for("Enter choice number", 5000)` → optional snapshot of menu screen → `send(b"<digit>\r")` → loop walking screens with per-screen snapshot/golden assertion until `text.contains("Enter choice number")` → break on `screen > 20`. ~25 lines of identical scaffolding × 13+ instances. The only thing that varies is the per-screen action (insta::assert_snapshot! on the text side, assert_golden on the GPU side) and the menu digit/label.
  Subsystem: `oriterm_test_support` (canonical home for the helper) + `oriterm_core/tests/vttest/` + `oriterm/src/gpu/visual_regression/vttest/`
  Found: 2026-04-07 | Source: impl-hygiene-review (tack-conformance section 01.N)
  Severity: medium — pre-existing duplication that the section 01 deduplication faithfully preserved (zero behavioral change was the section mandate). Per impl-hygiene.md "cross-crate duplication: even 2 instances = extract to a shared crate" rule, this 13+ instance pattern is overdue for extraction.
  Fix: add a higher-order helper to `oriterm_test_support`:
  ```rust
  pub fn walk_vttest_screens(
      session: &mut PtySession,
      max_screens: usize,
      mut on_screen: impl FnMut(&mut PtySession, usize),
  ) {
      let mut screen = 1;
      loop {
          let text = session.grid_text();
          if text.contains("Enter choice number") { break; }
          on_screen(session, screen);
          session.send(b"\r");
          screen += 1;
          if screen > max_screens { break; }
      }
  }
  ```
  Each `run_menuN_*` function then collapses to ~5 lines that pass a closure for the per-screen snapshot/golden call. Eliminates ~250 lines of duplication across 13 functions in two crates.
  Note: discovered during the section 01 final hygiene pass. NOT introduced by section 01 — it's pre-deduplication code that the migration correctly preserved verbatim. Section 01 is closing out clean; this is a follow-up for `/fix-bug` (or rolled into a future section's "test infrastructure cleanup" subsection).

- [ ] `[BUG-07-006][medium]` **`./clippy-all.sh` does not enable feature flags — 9 pre-existing clippy violations in `oriterm_ui/src/testing/`** — found by continue-roadmap.
  Repro: `cargo clippy -p oriterm --features gpu-tests --tests -- -D warnings` produces 9 errors. `./clippy-all.sh` runs `cargo clippy --workspace -- -D warnings` which uses the default feature set. The `oriterm_ui::testing` module is gated behind `#[cfg(feature = "testing")]`, so it's never linted by CI. Same root cause family as `[BUG-07-005]` (clippy-all scope is too narrow), different surface area (feature-gated lib code vs unconditional test target code).
  Subsystem: `clippy-all.sh` + `oriterm_ui/src/testing/`
  Found: 2026-04-07 | Source: continue-roadmap
  Locations:
  - `oriterm_ui/src/testing/scene_snapshot/mod.rs:101:12,28,44` — `float_cmp` (3×)
  - `oriterm_ui/src/testing/scene_snapshot/mod.rs:123:5` — `if_not_else`
  - `oriterm_ui/src/testing/scene_snapshot/mod.rs:176:14` — clippy lint (TBD)
  - `oriterm_ui/src/testing/harness.rs:46:37` — clippy lint (TBD)
  - `oriterm_ui/src/testing/harness_dispatch.rs:56:13` — clippy lint (TBD)
  - `oriterm_ui/src/testing/mock_measurer/mod.rs:28:5` — clippy lint (TBD)
  - `oriterm_ui/src/testing/query.rs:19:25` — clippy lint (TBD)
  Fix: (1) update each violation site, and (2) add `--features testing` to `./clippy-all.sh` (or add `cargo clippy --workspace --all-features` as a sibling step) so feature-gated code is gated by CI going forward. None caused by tack-conformance section 01.4 GPU migration — verified by reading my diffs against violation lines (none of the modified files are oriterm_ui).
  Note: Active work in tack-conformance section 01.4 (GPU vttest migration) does not modify the lines flagged above. Discovered when running `cargo clippy --features gpu-tests --tests` to verify my changes were clean; my changes WERE clean — these errors come from the feature-gated `oriterm_ui::testing` module which my new dev-dep on `oriterm_test_support` had nothing to do with.

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

- [x] `[BUG-07-008][medium]` **`oriterm_test_support` PtySession test uses `#[cfg(unix)]` instead of a runtime gate** — found by /tp-help pre-check (Codex) during /review-plan on tack-conformance section 02.
  **Fixed 2026-04-07.** Resolved by tack-conformance section 02.3. `crates/oriterm_test_support/src/session/tests.rs::pty_session_drains_simple_output` no longer carries `#[cfg(unix)]` — replaced with a portable two-arm test (`/bin/sh -c "printf hello"` on Unix, `cmd.exe /C "echo hello"` on Windows) wrapped in `#[cfg(unix)] / #[cfg(windows)]` blocks INSIDE the `#[test] fn`, restoring Windows ConPTY drain coverage. Verified by `cargo build --target x86_64-pc-windows-gnu -p oriterm_test_support --tests` and the host `cargo test -p oriterm_test_support` (12 tests pass).
  Repro: open `crates/oriterm_test_support/src/session/tests.rs:16` — `pty_session_drains_simple_output` is wrapped in `#[cfg(unix)]` so the test source does not even compile on Windows. The test spawns `/bin/sh -c "printf hello"` and asserts the PTY drain contains `hello`.
  Subsystem: `crates/oriterm_test_support/src/session/tests.rs`
  Found: 2026-04-07 | Source: /tp-help pre-check (Codex)
  Severity: medium — same antipattern family as `[BUG-07-004]` (Windows PTY size test removed by `#[cfg(unix)]`). CLAUDE.md cross-platform rule: "All code must compile and run correctly on all three platforms… Every `#[cfg(target_os = "...")]` block must have counterparts for all supported targets — no platform left behind." Section 02 of `tack-conformance` explicitly bans this exact pattern in its skip-discipline subsection — Section 01 contradicts the very rule Section 02 articulates.
  Fix: **Owned by tack-conformance Section 02.3.** The fix is a portable two-arm test (`/bin/sh -c "printf hello"` on Unix, `cmd.exe /C "echo hello"` on Windows) so Windows gets real ConPTY drain coverage. Implementation steps and full code listing live in `plans/tack-conformance/section-02-terminfo-provisioning.md` under 02.3. When that section lands, check this box, add a "Fixed YYYY-MM-DD" line, and the bug closes automatically. Do NOT run a separate `/fix-bug BUG-07-008` — Section 02's skip-discipline subsection IS the fix.

---

## 07.R Third Party Review Findings

- [x] `[BUG-07-001][high]` Auto-release one per UTC day limit.
  **Fixed 2026-03-30.** `bump-build.sh` now appends a sequence number (`.2`, `.3`, ...) when the current BUILD_NUMBER already has today's date. Format: `0.2.0-alpha.YYYYMMDD[.N]`.

- [x] `[BUG-07-002][high]` `release.yml` no longer validates tag matches workspace version.
  **Fixed 2026-03-30.** Added "Verify tag matches workspace version" step in `release.yml` that extracts the version from Cargo.toml and compares it to `github.ref_name`. Fails with a clear error if they don't match.

- [x] `[BUG-07-003][medium]` Auto-release `Cargo.lock` stale after version bump.
  **Fixed 2026-03-30.** Added `cargo generate-lockfile` step in `auto-release.yml` after `sync-version.sh` and before `git add`. Also added Rust toolchain installation step since `cargo` is needed.

---
