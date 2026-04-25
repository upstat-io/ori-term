---
bug: "BUG-07-007"
title: "vttest screen-walker scaffold duplicated across 13+ functions in two crates"
severity: "medium"
status: in-progress
goal: "The vttest screen-walking control-flow skeleton (loop on grid_text → break on \"Enter choice number\" → per-screen action → send \\r → cap on max_screens) lives in exactly one canonical home in `oriterm_test_support`, and every text-side and GPU-side menu test calls into it instead of carrying its own copy."
success_criteria:
  - "`oriterm_test_support` exports `walk_vttest_screens(session, max_screens, extra_sentinels, on_screen) -> usize` (or shape agreed in §1.5 consensus)"
  - "All 8 `oriterm_core/tests/vttest/menu*.rs` outer `run_menu*_*` functions delegate the screen-loop to the helper"
  - "All 7 `oriterm/src/gpu/visual_regression/vttest/{mod.rs,menus_3_8.rs}` `run_menu*_golden` functions delegate the screen-loop to the helper"
  - "Per-file sub-walkers `walk_menu3_subscreens` and `walk_menu6_subscreens` either delegate to the canonical helper or are deleted"
  - "Net deletion ≥ ~250 lines across both crates (helper itself ~30 lines)"
  - "`cargo test -p oriterm_core --test vttest` green with zero insta snapshot drift"
  - "`cargo test -p oriterm --features gpu-tests -- vttest_golden` green (locally — skips on CI without GPU)"
  - "`./test-all.sh`, `./build-all.sh`, `./clippy-all.sh` all green"
subsystem: "crates/oriterm_test_support + oriterm_core/tests/vttest/ + oriterm/src/gpu/visual_regression/vttest/"
found: "2026-04-07"
source: "impl-hygiene-review (tack-conformance section 01.N)"
third_party_review:
  status: none
  updated: null
---

# Fix: BUG-07-007 — vttest screen-walker scaffold duplicated across 13+ functions in two crates

**Status:** In Progress
**Severity:** medium
**Goal:** The vttest screen-walking control-flow skeleton lives in exactly one canonical home in `oriterm_test_support`. Every text-side and GPU-side menu test calls into it via a small per-screen closure that handles the variation (insta snapshot vs GPU golden vs structural assertion). Net result: ~250 lines of duplication eliminated across two crates with zero behavioral change.

**Success Criteria:**
- [ ] `oriterm_test_support` exports a screen-walker helper (final API decided in §1.5 consensus)
- [ ] All 8 text-side `run_menu*_*` outer functions delegate the loop to the helper
- [ ] All 7 GPU-side `run_menu*_golden` outer functions delegate the loop to the helper
- [ ] Per-file sub-walkers (`walk_menu3_subscreens`, `walk_menu6_subscreens`) either delegate or are deleted
- [ ] `cargo test -p oriterm_core --test vttest` green with zero insta snapshot drift
- [ ] `./test-all.sh`, `./build-all.sh`, `./clippy-all.sh` all green
- [ ] Net deletion ≥ ~250 lines

**Context:** Discovered during impl-hygiene-review of the tack-conformance section 01.N closing pass. The vttest screen-walking pattern was copied verbatim across 17 functions in two crates when the GPU-side mirror was created. Per `impl-hygiene.md` §Algorithmic DRY: "Cross-crate duplication: even 2 instances = extract to a shared crate" — this 17-instance pattern is overdue for extraction. The duplication is structural (same control-flow skeleton; only per-screen actions and parameter values vary), so a higher-order helper with a closure parameter is the correct remediation.

---

## 1. Root Cause Analysis

- **Symptom**: `oriterm_test_support` (the canonical home for cross-suite test plumbing) exposes `PtySession` + sync helpers, but the next layer up — vttest menu navigation — is open-coded at every call site. 17 instances of a 25-line skeleton differ only in per-screen action, max_screens cap, optional extra break sentinel, and (for one case) an interleaved intermediate send.
- **Proximate cause**: When the GPU-side `oriterm/src/gpu/visual_regression/vttest/` mirror was created, each `run_menu*_golden` function was hand-written by copying the corresponding `run_menu*_*` text-side function and swapping `insta::assert_snapshot!(...)` for `assert_golden(...)`. Section 01 of the tack-conformance plan explicitly preserved verbatim semantics during the GPU migration, so the duplication was preserved by design — the cleanup was deferred to a follow-up.
- **Root cause**: `oriterm_test_support` provides the *primitives* (`PtySession`, `wait_for`, `grid_text`, `send`) but not the *menu-walking algorithm* that those primitives compose into. Without a canonical algorithm, every consumer composes the primitives itself, and the composition gets copy-pasted. This is `LEAK:algorithmic-duplication` per `impl-hygiene.md` §Finding Categories — the algorithm has no canonical home.
- **Blast radius**: Test infrastructure only. No production code path. Behavioral change must be zero (insta snapshots and golden PNGs are the regression surface). 17 functions in 11 files across 2 crates need migration.
- **Affected files**:
  - `crates/oriterm_test_support/src/lib.rs` — add new module + re-export
  - `crates/oriterm_test_support/src/vttest_walker.rs` (or chosen location per §1.5) — new file with `walk_vttest_screens` helper
  - `oriterm_core/tests/vttest/menu1.rs` — migrate `run_menu1_cursor_movement`
  - `oriterm_core/tests/vttest/menu2.rs` — migrate `run_menu2_screen_features`
  - `oriterm_core/tests/vttest/menu3.rs` — migrate `run_menu3_character_sets` + decide fate of `walk_menu3_subscreens`
  - `oriterm_core/tests/vttest/menu4.rs` — migrate `run_menu4_double_size`
  - `oriterm_core/tests/vttest/menu5.rs` — migrate `run_menu5_keyboard` (LED + repeat sub-walks)
  - `oriterm_core/tests/vttest/menu6.rs` — migrate `run_menu6_reports` + decide fate of `walk_menu6_subscreens`
  - `oriterm_core/tests/vttest/menu7.rs` — migrate `run_menu7_vt52`
  - `oriterm_core/tests/vttest/menu8.rs` — migrate `run_menu8_vt102`
  - `oriterm/src/gpu/visual_regression/vttest/mod.rs` — migrate `run_menu1_golden`, `run_menu2_golden`
  - `oriterm/src/gpu/visual_regression/vttest/menus_3_8.rs` — migrate `run_menu3/4/6/7/8_golden`

**Reference implementations**: This is a test-infrastructure consolidation, not a protocol/design question. No external terminal-emulator reference is directly comparable (alacritty/wezterm/ghostty don't run vttest as automated CI), so reference-repo consultation is intentionally skipped per `/fix-bug` Phase 1 step 5.

---

## 1.5 Fix Consensus (via /tp-help)

Independent dual-source design review of the proposed fix approach. Run BEFORE tests or implementation to catch wrong-approach errors before they lock in.

- **Proposed approach (pre-consensus)**:
  - Add `crates/oriterm_test_support/src/vttest_walker.rs` (new file) exporting:
    ```rust
    pub fn walk_vttest_screens<F>(
        session: &mut PtySession,
        max_screens: usize,
        extra_sentinels: &[&str],
        mut on_screen: F,
    ) -> usize
    where
        F: FnMut(&mut PtySession, usize),
    {
        let mut screen = 1;
        loop {
            let text = session.grid_text();
            if text.contains("Enter choice number")
                || extra_sentinels.iter().any(|s| text.contains(s))
            {
                break;
            }
            on_screen(session, screen);
            session.send(b"\r");
            if screen >= max_screens {
                break;
            }
            screen += 1;
        }
        screen.saturating_sub(if screen == 1 { 1 } else { 0 })
    }
    ```
  - Re-export from `lib.rs` as `pub use vttest_walker::walk_vttest_screens`.
  - Each call site collapses from ~25 lines to ~5 lines:
    ```rust
    let count = walk_vttest_screens(&mut s, 20, &[], |session, screen| {
        insta::assert_snapshot!(format!("{label}_01_cursor_{screen:02}"), session.grid_text());
    });
    assert!(count > 0, "{label}: should have captured at least one screen");
    ```
  - For menu5 auto-repeat sub-walk (interleaved intermediate send), put extra work inside the closure:
    ```rust
    walk_vttest_screens(&mut s, 5, &["Menu 5"], |session, screen| {
        insta::assert_snapshot!(format!("{label}_05_repeat_{screen:02}"), session.grid_text());
        session.send(b"a");
        std::thread::sleep(std::time::Duration::from_millis(500));
        session.drain();
        // Walker sends b"\r" after closure returns.
    });
    ```
  - Per-file sub-walkers (`walk_menu3_subscreens`, `walk_menu6_subscreens`) collapse to direct `walk_vttest_screens` calls with their auxiliary state (`saw_line_drawing`, `all_text`) captured in mutable variables by the closure.

- **tp-help run scratch dir**: `/tmp/tpr-round-ori_term-0qCYabAO/`

### Round 1
- **Codex summary** (status: advice): "Use a concrete vttest helper in test_support, but pass the captured screen text into the closure." Confirms the higher-order helper approach, the `&mut PtySession` boundary (no trait), the `extra_sentinels: &[&str]` shape, the `walk_vttest_screens` name, the `vttest_walker` top-level module home, and the lib.rs re-export pattern. Explicitly rejects builder + config-struct + ScreenSource trait alternatives. **One refinement**: change closure signature from `FnMut(&mut PtySession, usize)` to `FnMut(&mut PtySession, &str, usize)` because the helper already calls `grid_text()` for the sentinel check; passing it into the closure avoids the per-screen redundant allocation that all 17 call sites currently incur.
- **Gemini summary** (status: advice): "Implement higher-order `walk_vttest_screens` in a new module, passing grid text to the closure for efficiency." Independently arrives at the same `&str text` parameter conclusion as Codex (independent corroboration, not an echo). Confirms module location at `crates/oriterm_test_support/src/vttest_walker/mod.rs` (parallel to `tack_framework/` and `terminfo/`). Confirms hardcoded `\r` advance is YAGNI-correct (all 17 sites use it). Confirms `PtySession::send`'s internal `wait(300)` covers timing. Migration patterns for menu3/5/6/7 sub-walkers and no-ops match Codex's recommendations.
- **Agreement points**: (1) Higher-order helper is the correct fix per impl-hygiene.md §Algorithmic DRY remediation hierarchy. (2) Public API is concrete `&mut PtySession` — no trait abstraction (hits §No Premature Abstraction at impl-hygiene.md:401). (3) Module home is `crates/oriterm_test_support/src/vttest_walker/mod.rs` (root sibling to `tack_framework/`, `terminfo/`, `session/`). (4) `extra_sentinels: &[&str]` is the right shape (no `IntoIterator`/`AsRef<str>` generality). (5) Hardcoded `\r` advance + `PtySession::send`'s 300ms wait is the correct timing contract. (6) Closure signature should include `&str` text to avoid redundant `grid_text()` allocation. (7) Migration of 17 call sites + their existing 200+ insta snapshots is the correct regression matrix (no `ScreenSource` trait needed for unit-testability).
- **Disagreement points**: None. Both reviewers agree on every load-bearing decision.
- **Independent code verification**:
  - `grid_text()` allocates: confirmed `crates/oriterm_test_support/src/session/mod.rs:373-381` (builds `Vec<Vec<char>>` + `String` per call).
  - `grid_chars()` allocates: confirmed `crates/oriterm_test_support/src/session/mod.rs:391-404` (`vec![vec![' '; cols]; lines]` per call).
  - `send()` waits 300ms: confirmed `crates/oriterm_test_support/src/session/mod.rs:341-345` (`self.wait(300)` after flush).
  - Text-side menu1 calls `grid_text()` and asserts on the result: confirmed `oriterm_core/tests/vttest/menu1.rs:21-29`.
  - GPU-side menu1 calls `grid_text()` for sentinel only (asserts via `assert_golden(&s, ...)`): confirmed `oriterm/src/gpu/visual_regression/vttest/mod.rs:46-60`.
  - Menu5 auto-repeat interleaved send: confirmed `oriterm_core/tests/vttest/menu5.rs:55-77`.
  - Module pattern (`tack_framework/`, `terminfo/`, `session/` are root siblings, re-exported from lib.rs): confirmed `crates/oriterm_test_support/src/lib.rs:16-28`.
  - Crate-boundaries explicitly assigns shared vttest fixture helpers to `oriterm_test_support`: confirmed `.claude/rules/crate-boundaries.md` §`crates/oriterm_test_support` (cited lines 108-118 by Codex).
  - File-size 500-line limit: confirmed `.claude/rules/code-hygiene.md` §File Organization. `session/mod.rs` is 442 lines — adding the walker there would push it over the limit.
  - Algorithmic DRY remediation hierarchy puts higher-order functions ahead of trait abstractions: confirmed `.claude/rules/impl-hygiene.md:198-206`.
  - No Premature Abstraction (single-implementor traits → delete): confirmed `.claude/rules/impl-hygiene.md:401-405`.
- **Outcome**: **Persuaded divergence** — the original `FnMut(&mut PtySession, usize)` signature is replaced with `FnMut(&mut PtySession, &str, usize)`. Every other element of the proposed approach is preserved unchanged. No round 2 needed; both reviewers converged in round 1 with mutual agreement on the divergence.

### Final agreed approach

Add `crates/oriterm_test_support/src/vttest_walker/mod.rs` with the helper:

```rust
//! Higher-order screen walker for vttest menu tests.
//!
//! Replaces 17 hand-rolled copies of the same control-flow skeleton
//! across `oriterm_core/tests/vttest/` and
//! `oriterm/src/gpu/visual_regression/vttest/` with a single canonical
//! algorithm that takes a per-screen closure for the variation.

use crate::PtySession;

/// Walk vttest screens, calling `on_screen` for each non-sentinel screen
/// until `"Enter choice number"` (or any string in `extra_sentinels`)
/// appears in `grid_text()`, or `max_screens` iterations elapse.
///
/// The closure receives the session (so it can `send` interleaved keys
/// or call `grid_chars` for structural assertions), the current grid
/// text already captured by the helper for the sentinel check (so the
/// closure does not re-pay the `grid_text()` allocation cost), and the
/// 1-based screen index. After the closure returns, `\r` is sent to
/// advance to the next screen.
///
/// Returns the number of screens for which `on_screen` was called.
pub fn walk_vttest_screens<F>(
    session: &mut PtySession,
    max_screens: usize,
    extra_sentinels: &[&str],
    mut on_screen: F,
) -> usize
where
    F: FnMut(&mut PtySession, &str, usize),
{
    let mut count = 0usize;
    let mut screen = 1usize;
    loop {
        let text = session.grid_text();
        if text.contains("Enter choice number")
            || extra_sentinels.iter().any(|s| text.contains(s))
        {
            break;
        }
        if count >= max_screens {
            break;
        }
        on_screen(session, &text, screen);
        count += 1;
        session.send(b"\r");
        screen += 1;
    }
    count
}
```

Re-export from `crates/oriterm_test_support/src/lib.rs`:
```rust
pub mod vttest_walker;
pub use vttest_walker::walk_vttest_screens;
```

Migration patterns:
- **Simple text-side** (`menu1/2/4/8`): `walk_vttest_screens(&mut s, 20, &[], |_, text, screen| { insta::assert_snapshot!(format!("{label}_..._{screen:02}"), text); });`
- **Simple GPU-side** (`menu1/2/4/8`): `walk_vttest_screens(&mut s, 20, &[], |session, _text, screen| { assert_golden(session, &format!("vttest_..._{screen:02}"), &gpu, &pipelines, &mut renderer); });`
- **Structural-assertion text-side** (`menu8`): closure calls `let grid = session.grid_chars(); assert_vt102_screen_structure(&grid, text, screen, &label); insta::assert_snapshot!(...);`
- **Sub-menu return** (`menu5` LED + repeat): `walk_vttest_screens(&mut s, 10, &["Menu 5"], |...| { ... });`
- **Interleaved intermediate send** (`menu5` repeat): closure body calls `session.send(b"a"); std::thread::sleep(...); session.drain();` BEFORE returning; helper sends `\r` after.
- **Aux state via closure capture** (`menu3` line-drawing detection, `menu6` accumulated text): outer mutable variables (`let mut saw_drawing = false; let mut all_text = String::new();`) captured by `&mut` reference inside the closure.
- **No-op** (`menu7`): `|_, _, _| {}`. Underscore-prefixed parameters avoid clippy unused-arg noise.

Module home: `crates/oriterm_test_support/src/vttest_walker/mod.rs` (root sibling to `tack_framework/`, `terminfo/`, `session/`). NOT inside `session/` (that file is at 442 lines and any addition would push past the 500-line `code-hygiene.md` limit; also `session/sync/` is bounded-poll plumbing, not vttest-specific).

The text-side adapter `oriterm_core/tests/vttest/session.rs` (which currently re-exports `PtySession` and `vttest_available`) MUST be extended to re-export `walk_vttest_screens` so the existing `super::session::*` import convention in the menu*.rs files keeps working unchanged.

---

## 2. TDD — Test Matrix

Write ALL tests BEFORE the fix. Verify they fail against current code.

### Helper unit tests (in `crates/oriterm_test_support/src/vttest_walker/tests.rs`)

The helper is small and takes `&mut PtySession` directly. To keep tests fast and not require a real PTY, the helper itself is too tightly coupled to PtySession to mock cleanly. Two complementary test strategies:

1. **Live PTY smoke test (gated on `vttest_available()`)** — verifies end-to-end correctness via the real system that the helper is designed to drive.
2. **Migration-as-regression** — the existing 200+ insta snapshots + 17 structural assertions across the migrated call sites form the de facto regression matrix. Zero snapshot drift after migration ⇒ helper preserves semantics.

### Exact failing case
- [ ] **Live smoke test**: `walk_vttest_screens` on a real `PtySession::spawn_vttest(80, 24)` with a no-op closure walks menu 1 to completion (returns count > 0, terminates on "Enter choice number")

### Edge cases
- [ ] **Empty walk**: vttest at the main menu (already showing "Enter choice number") — walker calls `on_screen` zero times, returns 0
- [ ] **Max screens cap**: walker terminates after `max_screens` iterations even if sentinel never appears (use a small cap like 3 in a smoke test that calls the closure with enough screens left in the menu to verify cap behavior)
- [ ] **Extra sentinel**: walker terminates on a custom sentinel (validate the `&["Menu 5"]` case via menu5 migration green)

### Cross-pattern coverage
- [ ] **Snapshot per screen using `text` arg**: closure body `insta::assert_snapshot!(format!(...), text)` — text-side menu1/2/4/8 use the helper-supplied `&str` directly (no second `grid_text()` call; allocation savings)
- [ ] **Snapshot per screen ignoring `text` arg**: closure signature `|session, _text, screen|` — GPU-side menu1/2/4/8 ignore text and use `session` for `assert_golden(session, ...)`
- [ ] **Structural assertion using `text` arg**: closure body `assert_vt102_screen_structure(&session.grid_chars(), text, screen, &label)` — menu8 text-side
- [ ] **No-op closure with all-underscore args**: `|_, _, _| {}` — menu7 text + GPU; verifies no clippy unused-arg noise from helper signature
- [ ] **Interleaved intermediate send inside closure**: closure body `session.send(b"a"); std::thread::sleep(...); session.drain();` — menu5 auto-repeat sub-walk; helper's `send(b"\r")` runs after closure returns
- [ ] **Auxiliary `bool` state via closure capture**: outer `let mut saw_drawing = false;` mutated by `&mut` capture inside closure — menu3 `walk_menu3_subscreens` collapse
- [ ] **Auxiliary `String` state via closure capture**: outer `let mut all_text = String::new(); ... all_text.push_str(text);` — menu6 `walk_menu6_subscreens` collapse (uses the helper-supplied `text` arg)

### Cross-feature interactions
- [ ] **Sub-menu re-entry**: menu3 / menu6 walk multiple sub-items in sequence; each sub-item walk uses the helper independently (validated by migrated menu3/menu6 tests passing)
- [ ] **Sub-menu sentinel**: menu5 walks sub-items that return to "Menu 5" (validated by migrated menu5 test passing)

### Semantic pin
- [ ] **Insta snapshot stability**: zero `*.snap.new` files generated by the migrated text-side tests — the migration preserves the exact text-per-screen captured today (verified via `git status --porcelain -- '*.snap.new'` empty after `cargo test -p oriterm_core --test vttest`)
- [ ] **Helper returns the captured screen count**: returned `usize` matches the number of times `on_screen` was called (verified by helper unit test asserting count == max_screens cap when sentinel never appears, count == screens-before-sentinel otherwise)

### Negative pin
- [ ] **Helper does NOT swallow the sentinel screen**: `on_screen` is NOT called for the screen that contains "Enter choice number" (verified by helper unit test where the first `grid_text()` already contains the sentinel — closure must not be invoked)
- [ ] **Helper does NOT skip the cap check on first iteration**: with `max_screens = 0`, walker calls `on_screen` zero times and returns 0 (verified by helper unit test)
- [ ] **Migration deletes ALL pre-existing copies of the loop body**: net `git diff --stat` shows substantial line deletion in `oriterm_core/tests/vttest/` and `oriterm/src/gpu/visual_regression/vttest/` (verified by counting `loop {` blocks containing `Enter choice number` in those directories — should be 0 after migration)

### Verify tests fail before fix
- [ ] Pre-fix: `walk_vttest_screens` does not exist → smoke test compiles but cannot reference the symbol → fails to build (which IS the failure mode for "the helper does not yet exist")
- [ ] Pre-fix: `grep -rn 'Enter choice number' oriterm_core/tests/vttest oriterm/src/gpu/visual_regression/vttest | wc -l` ≥ 17 → confirms duplication is present
- [ ] Post-fix: `grep -rn 'Enter choice number' oriterm_core/tests/vttest oriterm/src/gpu/visual_regression/vttest | wc -l` should drop to 0 (only inside the canonical helper, which lives in `oriterm_test_support`, not these directories)

---

## 2.5 Fix Plan TPR Findings

**Gate:** Skipped — medium severity, non-elevated subsystem (test infrastructure / `oriterm_test_support`, not in the GPU/VTE/mux/IPC/cfg-platform elevated list), round-1 `/tp-help` consensus converged with mutual agreement on every load-bearing decision (sole refinement — pass `&str text` into closure — was independently proposed by BOTH reviewers, not a 1-vs-1 disagreement).

Plan TPR: Skipped per Phase 2.5 gate criteria.

---

## 3. Implementation

- [ ] **Step 1**: Create `crates/oriterm_test_support/src/vttest_walker/mod.rs` with the helper function (+ `tests.rs` sibling for smoke tests). Final API per §1.5 consensus:
  ```rust
  //! Higher-order screen walker for vttest menu tests.

  use crate::PtySession;

  /// Walk vttest screens, calling `on_screen` for each non-sentinel screen
  /// until `"Enter choice number"` (or any string in `extra_sentinels`)
  /// appears in `grid_text()`, or `max_screens` iterations elapse.
  ///
  /// The closure receives the session (for interleaved sends or
  /// `grid_chars` access), the captured grid text the helper already
  /// allocated for the sentinel check (so the closure does not re-pay
  /// the `grid_text()` cost), and the 1-based screen index. After the
  /// closure returns, `\r` is sent to advance.
  ///
  /// Returns the number of screens for which `on_screen` was called.
  pub fn walk_vttest_screens<F>(
      session: &mut PtySession,
      max_screens: usize,
      extra_sentinels: &[&str],
      mut on_screen: F,
  ) -> usize
  where
      F: FnMut(&mut PtySession, &str, usize),
  {
      let mut count = 0usize;
      let mut screen = 1usize;
      loop {
          let text = session.grid_text();
          if text.contains("Enter choice number")
              || extra_sentinels.iter().any(|s| text.contains(s))
          {
              break;
          }
          if count >= max_screens {
              break;
          }
          on_screen(session, &text, screen);
          count += 1;
          session.send(b"\r");
          screen += 1;
      }
      count
  }

  #[cfg(test)]
  mod tests;
  ```
- [ ] **Step 2**: Re-export from `crates/oriterm_test_support/src/lib.rs`: add `pub mod vttest_walker;` and `pub use vttest_walker::walk_vttest_screens;`
- [ ] **Step 3**: Extend `oriterm_core/tests/vttest/session.rs` adapter to re-export `walk_vttest_screens` so the existing `super::session::*` import in `menu*.rs` sees the new symbol unchanged.
- [ ] **Step 4**: Write smoke tests in `crates/oriterm_test_support/src/vttest_walker/tests.rs` (gated on `vttest_available()`):
  - `walk_vttest_screens_walks_menu1_to_completion` — spawn vttest, walk menu 1 with no-op closure, assert returned count > 0 and helper terminates on `"Enter choice number"`.
  - `walk_vttest_screens_zero_calls_at_main_menu` — spawn vttest, immediately call walker with no-op closure (no menu selected), assert returned count == 0.
  - `walk_vttest_screens_max_screens_cap_terminates_loop` — spawn vttest, select menu 2, call walker with `max_screens = 2` and counting closure, assert closure invoked exactly 2 times.
  - `walk_vttest_screens_extra_sentinel_terminates_loop` — spawn vttest, select menu 5 (which has sub-menu return on `"Menu 5"`), assert walker terminates on the extra sentinel before max_screens.
- [ ] **Step 5**: Migrate `oriterm_core/tests/vttest/menu1.rs::run_menu1_cursor_movement` — closure signature `|_, text, screen|`; preserves `assert!(count > 0, ...)` post-condition using helper return value. Pre-walk `s.send(b"1\r")` happens BEFORE the helper, after the initial `wait_for + insta::assert_snapshot!(menu)`.
- [ ] **Step 6**: Migrate `oriterm_core/tests/vttest/menu2.rs::run_menu2_screen_features` — same shape as menu1; the structural assertions (`screen == 11/12/15` match arm) live inside the closure body.
- [ ] **Step 7**: Migrate `oriterm_core/tests/vttest/menu3.rs::run_menu3_character_sets` — collapse `walk_menu3_subscreens` helper into TWO direct `walk_vttest_screens` calls (one per sub-item: 8 then 9). The `saw_line_drawing: bool` aux state captured by closure for sub-item 8.
- [ ] **Step 8**: Migrate `oriterm_core/tests/vttest/menu4.rs::run_menu4_double_size` — simple text-side pattern.
- [ ] **Step 9**: Migrate `oriterm_core/tests/vttest/menu5.rs::run_menu5_keyboard` — TWO walker calls (LED + repeat). Both pass `&["Menu 5"]` as extra_sentinel. The repeat sub-walk's closure body calls `session.send(b"a"); std::thread::sleep(...); session.drain();` BEFORE returning (helper sends `\r` after).
- [ ] **Step 10**: Migrate `oriterm_core/tests/vttest/menu6.rs::run_menu6_reports` — collapse `walk_menu6_subscreens` helper into a per-sub-item `walk_vttest_screens` call. The `total_screens: usize`, `saw_da_response: bool`, `saw_dsr_response: bool` aux state captured by closure references.
- [ ] **Step 11**: Migrate `oriterm_core/tests/vttest/menu7.rs::run_menu7_vt52` — closure is `|_, _, _| {}`; the post-walk `assert!(count > 0, ...)` uses the return value to verify navigation.
- [ ] **Step 12**: Migrate `oriterm_core/tests/vttest/menu8.rs::run_menu8_vt102` — closure body calls `let grid = session.grid_chars(); assert_vt102_screen_structure(&grid, text, screen, &label); insta::assert_snapshot!(...);`.
- [ ] **Step 13**: Migrate `oriterm/src/gpu/visual_regression/vttest/mod.rs::{run_menu1_golden, run_menu2_golden}` — closure receives `&str text` but uses `session` for `assert_golden(session, name, &gpu, &pipelines, &mut renderer)`.
- [ ] **Step 14**: Migrate `oriterm/src/gpu/visual_regression/vttest/menus_3_8.rs::{run_menu3/4/6/7/8_golden}` — same GPU pattern. menu7_golden uses no-op `|_, _, _| {}` (no assertions per the existing comment "VT52 output is non-deterministic").
- [ ] **Step 15**: Run `./fmt-all.sh && ./build-all.sh && ./clippy-all.sh && timeout 150 ./test-all.sh` — all green
- [ ] **Step 16**: Verify zero insta snapshot drift: `git status --porcelain -- '*.snap.new'` must be empty
- [ ] **Step 17**: Verify duplication eliminated: `grep -rn 'Enter choice number' oriterm_core/tests/vttest oriterm/src/gpu/visual_regression/vttest` should return ZERO matches (only the canonical helper at `crates/oriterm_test_support/src/vttest_walker/mod.rs` should contain the literal)
- [ ] **Step 18**: `/commit-push` to land the fix before Phase 5 reviews

---

## R. Third Party Review Findings

{Initially empty — populated during Phase 5 completion checklist after `/tpr-review` runs.}

---

## 4. Completion Checklist

- [ ] All new tests pass unchanged after fix (no test modifications needed)
- [ ] Matrix completeness verified — every cell in the §2 matrix has a test
- [ ] Debug AND release builds pass (`cargo b && cargo b --release`)
- [ ] Windows cross-compile green (`cargo build --target x86_64-pc-windows-gnu`)
- [ ] No GPU render path touched (test infrastructure only) — visual regression suite green incidentally
- [ ] No hot render path touched — alloc/RSS regression tests unaffected
- [ ] `timeout 150 ./test-all.sh` green — no regressions
- [ ] `./clippy-all.sh` green
- [ ] `./build-all.sh` green (workspace + cross-compile)
- [ ] `cargo test -p oriterm_core --test vttest` green with zero insta snapshot drift
- [ ] `cargo test -p oriterm --features gpu-tests -- vttest_golden` builds (skips on no-GPU CI; smoke-runs locally)
- [ ] `/commit-push` — commit all changes before review
- [ ] Plan TPR (Phase 2.5) — completed or skipped per §2.5 gate
- [ ] `/tpr-review` (Phase 5 — code review) passed — independent dual-source review of the IMPLEMENTATION found no actionable findings
- [ ] `/impl-hygiene-review` passed — MUST run AFTER code `/tpr-review` is clean
- [ ] **Capability regression gate** — N/A: the fix is a pure refactor of test scaffolding, no capability is disabled or weakened
- [ ] `/improve-tooling` retrospective completed — capture any tooling gaps surfaced during this fix
- [ ] Bug entry in `plans/bug-tracker/section-07-ci-build.md` updated to `- [x]` with resolution details
- [ ] Fix section frontmatter `status` updated to `complete`
- [ ] Bug-tracker `00-overview.md` Quick Reference open bug count for section 07 decremented by 1
- [ ] Final `/commit-push` — commit closure artifacts

**Exit Criteria:** `crates/oriterm_test_support` exports `walk_vttest_screens`. All 17 hand-rolled `loop { let text = session.grid_text(); if text.contains("Enter choice number") { break; } ... s.send(b"\r"); ... }` blocks in `oriterm_core/tests/vttest/` and `oriterm/src/gpu/visual_regression/vttest/` are deleted. `cargo test -p oriterm_core --test vttest` passes with zero insta snapshot drift. `./test-all.sh`, `./build-all.sh`, `./clippy-all.sh` all green. Net deletion ≥ ~250 lines.
