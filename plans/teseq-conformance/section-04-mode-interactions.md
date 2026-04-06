---
section: "04"
title: "Mode Interaction Scenarios"
status: in-progress
reviewed: true
goal: "Create multi-sequence scenarios testing mode combinations that individual handler tests don't cover — DECOM+scroll, DECCOLM transitions, alt screen roundtrips, IRM, and cross-cutting mode interactions"
success_criteria:
  - "DECOM+DECSTBM scenarios validate cursor positioning within scroll regions"
  - "DECCOLM scenarios validate 80↔132 column transitions with content clearing"
  - "DECCOLM negative control validates side effects without Mode 40 (no resize)"
  - "Alt screen (1049) scenarios validate enter/exit roundtrip preserving primary screen"
  - "Alt screen re-entry: 1049 retains alt content, 1047 clears alt content"
  - "Alt screen mode leakage scenario validates DECOM/DECAWM/IRM flags survive screen swap"
  - "IRM scenarios validate insert mode at right margin, with wrap-pending, and with wide chars"
  - "Mode state assertions via ScenarioOutcome.mode verify flags, not just grid content"
  - "Mode combination scenarios run at 80x24, 97x33, and 120x40"
  - "34+ mode interaction scenarios pass across all subsections (9+6+7+7+5)"
  - "Satisfies mission criteria: DECOM+DECSTBM, DECCOLM+DECAWM, alt screen (1049/1047), IRM coverage"
inspired_by:
  - "ori_term vttest menu1 (DECCOLM) — 132-column mode transitions"
  - "ori_term vttest menu2 (origin mode, scroll regions) — DECOM+DECSTBM interactions"
  - "ori_term handler/tests.rs — origin mode, scroll region, insert mode individual tests"
depends_on: ["01", "02"]
third_party_review:
  status: resolved
  updated: 2026-04-06
sections:
  - id: "04.0"
    title: "Scaffolding & Harness Extension"
    status: complete
  - id: "04.1"
    title: "Origin Mode + Scroll Region Scenarios"
    status: complete
  - id: "04.2"
    title: "DECCOLM Column Mode Scenarios"
    status: complete
  - id: "04.3"
    title: "Alt Screen Scenarios"
    status: complete
  - id: "04.4"
    title: "Insert Mode & Wrap Scenarios"
    status: complete
  - id: "04.5"
    title: "Cross-Cutting Mode Interactions"
    status: complete
  - id: "04.R"
    title: "Third Party Review Findings"
    status: in-progress
  - id: "04.N"
    title: "Completion Checklist"
    status: in-progress
---

# Section 04: Mode Interaction Scenarios

**Status:** In Progress
**Goal:** Test multi-mode combinations where individual handler tests fall short. The existing handler tests validate each mode in isolation. These scenarios test the *interaction* between modes — the edge cases that only manifest when multiple modes are active simultaneously. Also exercises negative controls (DECCOLM without Mode 40) and state dimensions invisible to grid-only assertions (mode flags surviving alt-screen swap).

**Success Criteria:**

- [x] DECOM+DECSTBM: cursor stays within scroll region, scrolling respects origin
- [x] DECCOLM: 80→132 and 132→80 transitions clear screen, preserve modes
- [x] DECCOLM negative control: ?3h/?3l without Mode 40 clears/homes but does NOT resize
- [x] Alt screen: 1049 enter/exit preserves primary screen content
- [x] Alt screen mode leakage: DECOM/DECAWM/IRM flags survive screen swap (not saved/restored)
- [x] Alt screen re-entry: mode 1049 retains alt content, mode 1047 clears alt content
- [x] IRM: insert mode at right margin, with wrap-pending, and with wide chars
- [x] Mode flag assertions verify TermMode state, not just grid content
- [x] Cross-cutting: DECCOLM+DECOM+DECSTBM, IRM at right margin with DECAWM
- [x] Multi-size variants for mode interactions (97x33, 120x40)
- [x] Scrollback integrity checked where relevant (DECSTBM overflow, DECCOLM clear)
- [x] 34+ mode interaction scenarios pass (9+6+7+7+5)

**Context:** The vttest conformance plan fixed several mode interaction bugs: DECOM cursor positioning was garbled (fixed in Section 02), DECCOLM was a no-op (fixed in Section 03), border fills failed at non-80x24 sizes. These teseq scenarios serve as permanent regression guards for those fixes and explore additional interaction patterns.

**Reference implementations:**
- **ori_term** `handler/tests.rs` — origin mode tests, scroll region tests, insert mode tests
- **ori_term** `vttest/menu1.rs` — DECCOLM 132-column transitions
- **ori_term** `vttest/menu2.rs` — origin mode + scroll region interactions
- **ori_term** `handler/modes.rs` — `apply_deccolm()` has distinct Mode 40 on/off branches
- **ori_term** `alt_screen.rs` — `swap_alt()` swaps grids/cursor/keyboard stacks/image caches but NOT `TermMode` flags

**Depends on:** Section 01 (TeseqHarness), Section 02 (basic scenario pattern).

**Boundary with Section 06.1 (Mode Combination Workflows):** Section 04 owns **isolated mode interactions** — each scenario tests one mode family or one specific cross-cutting combination with focused assertions (mode flags, column counts, scrollback integrity). Section 06.1 owns **multi-step workflow narratives** — scenarios that chain 3+ mode operations into a realistic usage sequence (e.g., full DECCOLM lifecycle: write at 80 -> switch to 132 -> set DECOM + scroll region -> switch back to 80 -> verify). If a scenario tests a single mode combination at one terminal size, it belongs here (Section 04). If it chains multiple mode transitions into a multi-phase workflow validating cumulative state, it belongs in Section 06.1. The Section 06.1 scenarios `mode_scroll_origin_fill`, `mode_deccolm_full_cycle`, and `mode_alt_with_modes` are deliberately more complex sequences that build on Section 04's validated primitives — they should NOT be deduplicated into Section 04.

---

## 04.0 Scaffolding & Harness Extension

This subsection has two parts: (A) creating the module and directory scaffolding that 04.1-04.5 depend on, and (B) extending the harness with mode flag assertion helpers.

### 04.0a Module & Directory Scaffolding

**Why first:** Every test function in 04.1-04.5 lives in `mode_interactions.rs` and every `.teseq` file lives in `scenarios/csi/modes/`. These must exist and be wired up before any scenario work begins.

- [x] **Create scenario directory** `oriterm_core/tests/teseq/scenarios/csi/modes/`. This is the home for all `.teseq` and `.toml` sidecar files in this section. Currently `scenarios/csi/` contains `cursor/`, `erase/`, `insert_delete/`, `reports/` — `modes/` does not yet exist.

- [x] **Create `oriterm_core/tests/teseq/mode_interactions.rs`** — the Rust test module for all mode interaction scenarios. Start with minimal imports; the mode-specific imports (`assert_grid_cols`, `assert_mode_contains`, `assert_mode_not_contains`, `TermMode`) are added after 04.0b implements them. Follow the pattern established by `csi_reports.rs`:
  ```rust
  //! Mode interaction scenarios (DECOM, DECCOLM, alt screen, IRM, cross-cutting).

  use std::path::Path;

  use oriterm_core::TermMode;

  use super::harness::{
      self, ScenarioOutcome, TeseqHarness, assert_grid_cols, assert_mode_contains,
      assert_mode_not_contains, assert_scrollback_empty, reseq_available,
  };

  /// Run a mode interaction scenario and apply spec assertions.
  ///
  /// Returns `None` when `reseq` is unavailable (graceful skip with visible message).
  /// Returns the outcome for callers to perform additional mode/grid assertions.
  fn run_scenario(name: &str) -> Option<ScenarioOutcome> {
      if !reseq_available() {
          eprintln!("reseq not installed, skipping");
          return None;
      }
      let path = Path::new(env!("CARGO_MANIFEST_DIR"))
          .join("tests/teseq/scenarios/csi/modes")
          .join(format!("{name}.teseq"));
      let mut h = TeseqHarness::from_scenario(&path);
      let outcome = h.run(&path);
      harness::assert_spec(&outcome, h.spec(), &format!("mode_interactions_{name}"));
      Some(outcome)
  }
  ```
  Note the scenario path uses `scenarios/csi/modes` (matching the directory created above) and the snapshot prefix uses `mode_interactions_` (matching the module name). The `run_scenario` helper centralizes the `reseq_available()` guard (with `eprintln!` skip message) and returns `Option<ScenarioOutcome>` — callers use `let Some(outcome) = run_scenario(...) else { return; };` for mode/grid assertions beyond the sidecar spec.

- [x] **Register the module in `oriterm_core/tests/teseq/main.rs`** — add `mod mode_interactions;` in a new "Family modules (Section 04)" comment block, following the existing pattern:
  ```rust
  // Family modules (Section 04).
  mod mode_interactions;
  ```
  Place this after the Section 03 block (`mod csi_reports;`).

- [x] **Verify compilation** — `timeout 150 cargo test -p oriterm_core --test teseq -- mode_interactions` should compile (no tests yet, so zero tests run).

After 04.0b adds the mode assertion helpers, the imports in `mode_interactions.rs` should include the full set (already present in the scaffold above).

### 04.0b Harness Extension: Mode Flag Assertions

**File(s):** `oriterm_core/tests/teseq/harness/runner.rs`, `oriterm_core/tests/teseq/harness/assertions.rs`, `oriterm_core/tests/teseq/harness/mod.rs`

The current `ScenarioOutcome` (in `runner.rs`) exposes grid text, cursor, events, cols, rows, and scrollback length — but NOT terminal mode flags. `RenderableContent` already includes `mode: TermMode` (see `oriterm_core/src/term/renderable/mod.rs:141`). This sub-step extends the harness to make mode state assertable.

- [x] **Add `mode` field to `ScenarioOutcome`** in `runner.rs`:
  ```rust
  // Add import at top of runner.rs (oriterm_core re-exports TermMode at crate root):
  use oriterm_core::TermMode;

  // In ScenarioOutcome struct, add after scrollback_len:
  /// Terminal mode flags at snapshot time.
  pub mode: TermMode,
  ```
  In the `outcome()` method body, populate from `content.mode`:
  ```rust
  ScenarioOutcome {
      // ...existing fields...
      scrollback_len: content.scrollback_len,
      mode: content.mode,
  }
  ```

- [x] **Add `assert_mode_contains` and `assert_mode_not_contains` assertion helpers** in `assertions.rs`. Add `use oriterm_core::TermMode;` to the imports, then:
  ```rust
  /// Assert that specific TermMode flags are set.
  pub fn assert_mode_contains(outcome: &ScenarioOutcome, expected: TermMode) {
      assert!(
          outcome.mode.contains(expected),
          "expected mode flags {:?} to be set, but mode is {:?}",
          expected, outcome.mode
      );
  }

  /// Assert that specific TermMode flags are NOT set.
  pub fn assert_mode_not_contains(outcome: &ScenarioOutcome, unexpected: TermMode) {
      assert!(
          !outcome.mode.contains(unexpected),
          "expected mode flags {:?} to NOT be set, but mode is {:?}",
          unexpected, outcome.mode
      );
  }
  ```

- [x] **Add `assert_grid_cols` assertion helper** in `assertions.rs` for DECCOLM column count verification:
  ```rust
  /// Assert grid column count (for DECCOLM resize validation).
  pub fn assert_grid_cols(outcome: &ScenarioOutcome, expected_cols: usize) {
      assert_eq!(
          outcome.cols, expected_cols,
          "expected {} columns, got {}",
          expected_cols, outcome.cols
      );
  }
  ```

- [x] **Re-export new helpers from `harness/mod.rs`** — add to the existing `pub use assertions::{...}` line:
  ```rust
  pub use assertions::{
      analyze_response, assert_cursor, assert_event_snapshot, assert_grid_cols,
      assert_grid_snapshot, assert_mode_contains, assert_mode_not_contains,
      assert_pty_writes, assert_response_snapshot, assert_scrollback_empty,
      assert_spec, pipe_through_command,
  };
  ```
  This adds `assert_grid_cols`, `assert_mode_contains`, and `assert_mode_not_contains` to the three already-exported assertion helpers.

- [x] **Verify harness compiles** — `timeout 150 cargo test -p oriterm_core --test teseq` should pass with zero new failures (new helpers are unused until 04.1).

These helpers are used throughout 04.1-04.5 to verify mode state, not just grid content.

---

## 04.1 Origin Mode + Scroll Region Scenarios

**File(s):** `oriterm_core/tests/teseq/scenarios/csi/modes/origin_*.teseq` (scenario files), `oriterm_core/tests/teseq/scenarios/csi/modes/il_in_scroll_region.teseq`, `oriterm_core/tests/teseq/scenarios/csi/modes/dl_in_scroll_region.teseq` (IL/DL scenarios), `oriterm_core/tests/teseq/mode_interactions.rs` (test functions created in 04.0a)

- [x] **`origin_scroll_basic.teseq`** — Set scroll region, enable DECOM, verify cursor is relative:
  ```
  : Esc [ 5 ; 20 r
  : Esc [ ? 6 h
  : Esc [ 1 ; 1 H
  |Origin top|.
  : Esc [ 16 ; 1 H
  |Origin bottom|
  ```
  `origin_scroll_basic.toml`: `[expect] cursor = { col = 13, line = 19 }`
  Grid snapshot shows "Origin top" at absolute row 4 (scroll region top, 0-based). "Origin bottom" at absolute row 19 (region bottom - 1, 0-based). CUP 16;1 with DECOM → `goto_origin_aware(15, 0)` → offset=4, clamped to max=19, so line=19. Cursor ends at col 13 (after writing "Origin bottom", 13 chars).

- [x] **`origin_scroll_overflow.teseq`** — Fill scroll region past capacity, verify scrolling:
  ```
  : Esc [ 10 ; 15 r
  : Esc [ ? 6 h
  : Esc [ 1 ; 1 H
  |Line 01|.
  |Line 02|.
  |Line 03|.
  |Line 04|.
  |Line 05|.
  |Line 06|.
  |Line 07|.
  |Line 08|.
  ```
  `origin_scroll_overflow.toml`:
  ```toml
  [terminal]
  scrollback = 10
  ```
  With 6-line scroll region (rows 10-15, 0-based 9..15), writing 8 lines causes scrolling within the region. The first 2 lines scroll off the region top. **Scrollback check:** With `scrollback = 10` in the sidecar, lines scrolled out of a sub-region are lost (sub-region scroll does not push to scrollback — only full-screen scroll does). Assert `scrollback_len == 0` to verify no spurious scrollback pollution.

- [x] **`origin_cursor_save_restore.teseq`** — DECSC/DECRC with origin mode:
  ```
  : Esc [ 5 ; 20 r
  : Esc [ ? 6 h
  : Esc [ 3 ; 10 H
  : Esc 7
  : Esc [ 1 ; 1 H
  |moved|
  : Esc 8
  |restored|
  ```
  CUP 3;10 with DECOM → absolute line=2+4=6, col=9. DECSC saves. CUP 1;1 homes within region. DECRC restores to saved position. "restored" should appear at absolute row 6, col 9.

- [x] **`il_in_scroll_region.teseq`** — IL within DECSTBM, verify lines outside region do not shift:
  ```
  |Line 1|
  . CR/^M LF/^J
  |Line 2|
  . CR/^M LF/^J
  |Line 3|
  . CR/^M LF/^J
  |Line 4|
  . CR/^M LF/^J
  |Line 5|
  . CR/^M LF/^J
  |Line 6|
  : Esc [ 2 ; 5 r
  : Esc [ 3 ; 1 H
  : Esc [ 1 L
  ```
  Grid snapshot: Row 0 "Line 1" unchanged (above region). Row 1 "Line 2" unchanged (region top). Row 2 blank (inserted). Row 3 "Line 3" (shifted down). Row 4 "Line 4" (shifted down). Row 5 "Line 6" unchanged (below region — Line 5 pushed out of region bottom). Validates IL only shifts within DECSTBM scroll region boundaries. <!-- unblocks:02.R -->

- [x] **`dl_in_scroll_region.teseq`** — DL within DECSTBM, verify lines outside region do not shift:
  ```
  |Line 1|
  . CR/^M LF/^J
  |Line 2|
  . CR/^M LF/^J
  |Line 3|
  . CR/^M LF/^J
  |Line 4|
  . CR/^M LF/^J
  |Line 5|
  . CR/^M LF/^J
  |Line 6|
  : Esc [ 2 ; 5 r
  : Esc [ 3 ; 1 H
  : Esc [ 1 M
  ```
  Grid snapshot: Row 0 "Line 1" unchanged (above region). Row 1 "Line 2" unchanged (region top). Row 2 "Line 4" (shifted up from row 3). Row 3 "Line 5" (shifted up). Row 4 blank (new blank at region bottom). Row 5 "Line 6" unchanged (below region). Validates DL only shifts within DECSTBM scroll region boundaries. <!-- unblocks:02.R -->

- [x] **Multi-size variants** — create separate `.teseq` + `.toml` pairs for each size:
  - `origin_scroll_basic_97x33.teseq` + `.toml` — DECSTBM adjusted to `8;28` (proportional to 97x33), `.toml` sets `[terminal] cols = 97 rows = 33`. Separate test function `origin_scroll_basic_97x33()` in `mode_interactions.rs`.
  - `origin_scroll_basic_120x40.teseq` + `.toml` — DECSTBM adjusted to `10;35`, `.toml` sets `[terminal] cols = 120 rows = 40`. Separate test function `origin_scroll_basic_120x40()`.
  - `origin_scroll_overflow_97x33.teseq` + `.toml` — scroll region sized for 97x33, `.toml` sets `[terminal] cols = 97 rows = 33 scrollback = 10`. Separate test function `origin_scroll_overflow_97x33()`.
  - `origin_scroll_overflow_120x40.teseq` + `.toml` — scroll region sized for 120x40. Separate test function `origin_scroll_overflow_120x40()`.
  Each variant gets its own insta golden snapshot (size-specific). The test functions use the centralized guard pattern: `let Some(outcome) = run_scenario("origin_scroll_basic_97x33") else { return; };` (or just `run_scenario(...)` when no additional assertions are needed).

- [x] **Verify 04.1 compiles and passes** — `timeout 150 cargo test -p oriterm_core --test teseq -- mode_interactions::origin` should run all origin mode tests.

---

## 04.2 DECCOLM Column Mode Scenarios

**File(s):** `oriterm_core/tests/teseq/scenarios/csi/modes/deccolm_*.teseq` (scenario files), `oriterm_core/tests/teseq/mode_interactions.rs` (test functions — same module as 04.1, grouped under a `// DECCOLM` section comment)

Mode 40 (ENABLE_MODE_3) must be enabled for DECCOLM to actually resize the grid. The code in `apply_deccolm()` (`handler/modes.rs:187-206`) has two distinct branches: (1) with Mode 40 → resize + side effects, (2) without Mode 40 → side effects only (clear, home, reset margins, but NO resize). Both branches must be tested.

- [x] **`deccolm_80_to_132.teseq`** — Switch to 132 columns with Mode 40:
  ```
  |Content at 80 cols|.
  : Esc [ ? 3 h
  |Now at 132 columns|
  ```
  `deccolm_80_to_132.toml`:
  ```toml
  [setup]
  pre_feed = ["\\x1b[?40h"]
  [terminal]
  cols = 80
  rows = 24
  ```
  Grid snapshot shows screen cleared on mode switch, "Now at 132 columns" visible, grid is 132 columns wide. **Assert `outcome.cols == 132`** via `assert_grid_cols`. Cursor at line 0, col 18 (after writing "Now at 132 columns").

- [x] **`deccolm_132_to_80.teseq`** — Switch back to 80:
  ```
  : Esc [ ? 3 h
  |Wide content at 132|.
  : Esc [ ? 3 l
  |Back to 80|
  ```
  `deccolm_132_to_80.toml`:
  ```toml
  [setup]
  pre_feed = ["\\x1b[?40h"]
  ```
  Mode 40 required for DECCOLM resize. Grid snapshot shows screen cleared again, grid back to 80 columns. **Assert `outcome.cols == 80`** via `assert_grid_cols`. Note: `apply_deccolm(false)` uses `self.deccolm_default_cols` (initialized to `cols` from `Term::new`), so resetting returns to the sidecar's configured column count.

- [x] **`deccolm_wrap_interaction.teseq`** — DECCOLM + DECAWM interaction at 132 columns:
  ```
  : Esc [ ? 3 h
  : Esc [ ? 7 h
  |AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAXX|
  ```
  `deccolm_wrap_interaction.toml`:
  ```toml
  [setup]
  pre_feed = ["\\x1b[?40h"]
  ```
  The text line must contain exactly 132 A's + "xx" (134 chars total). At 132 columns with DECAWM on, the 132 A's fill row 0, then "xx" wraps to row 1. **When creating this file, verify the A count is exactly 132** (e.g., `python3 -c "print('A'*132 + 'xx')" | wc -c` should output 135 including newline).

- [x] **`deccolm_resets_scroll_region.teseq`** — DECCOLM resets scroll region:
  ```
  : Esc [ 5 ; 20 r
  : Esc [ ? 3 h
  : Esc [ 999 ; 1 H
  |At bottom|
  ```
  `deccolm_resets_scroll_region.toml`:
  ```toml
  [setup]
  pre_feed = ["\\x1b[?40h"]
  ```
  After DECCOLM switch, scroll region is reset to full screen (see `apply_deccolm()` line 198: `self.grid_mut().set_scroll_region(1, None)`). CUP 999;1 goes to the last visible row (not limited by old scroll region). Grid snapshot confirms "At bottom" is at the last row.

- [x] **`deccolm_no_mode40.teseq`** — DECCOLM without Mode 40 — negative control:
  ```
  |Content before|.
  |Line two|
  : Esc [ 5 ; 10 r
  : Esc [ ? 3 h
  |After DECCOLM|
  ```
  No `.toml` sidecar (defaults to 80x24, no Mode 40 pre_feed). Without Mode 40 enabled, `apply_deccolm()` skips the resize (the `if self.mode.contains(TermMode::ENABLE_MODE_3)` guard fails) but still runs: (1) reset scroll region to full screen, (2) clear screen, (3) home cursor. **Assert `outcome.cols == 80`** — grid stays at 80 columns. Grid snapshot shows "After DECCOLM" at row 0 col 0 (original content cleared). Scroll region should be reset (verified indirectly: CUP 999;1 would reach last row).

- [x] Multi-size variant: `deccolm_132_to_80` at 120x40 — verifies `deccolm_default_cols` restores to 120 (not hardcoded 80). `deccolm_132_to_80_120x40.toml`:
  ```toml
  [setup]
  pre_feed = ["\\x1b[?40h"]
  [terminal]
  cols = 120
  rows = 40
  ```
  **Assert `outcome.cols == 120`** after `?3l`. This catches regressions where reset hardcodes 80 instead of using `deccolm_default_cols`.

- [x] **TPR checkpoint** — `/tpr-review` covering 04.0–04.2 implementation work

---

## 04.3 Alt Screen Scenarios

**File(s):** `oriterm_core/tests/teseq/scenarios/csi/modes/altscreen_*.teseq` (scenario files), `oriterm_core/tests/teseq/mode_interactions.rs` (test functions — grouped under `// Alt screen` section comment)

`swap_alt()` (mode 1049) swaps grids, saves/restores cursor positions, swaps keyboard mode stacks and image caches, and toggles `TermMode::ALT_SCREEN`. It does NOT clear the alt grid on entry — content persists across exits. In contrast, `swap_alt_clear()` (mode 1047) resets the alt grid before swapping. Critically, neither variant saves/restores other `TermMode` flags — DECOM, DECAWM, IRM, etc. remain global. This is verified in `alt_screen.rs` — no mention of ORIGIN, LINE_WRAP, or INSERT in that file.

- [x] **`altscreen_roundtrip.teseq`** — Enter and exit alt screen preserving primary:
  ```
  |Primary screen content|.
  |Line two|.
  : Esc [ ? 1049 h
  |Alt screen content|.
  : Esc [ ? 1049 l
  ```
  `altscreen_roundtrip.toml`: `[expect] cursor = { col = 0, line = 2 }`
  Grid snapshot after exit should show primary screen content restored. Cursor restored to position when alt was entered: after writing two lines plus their LFs, cursor is at line 2, col 0. **Assert `assert_mode_not_contains(outcome, TermMode::ALT_SCREEN)`** — confirms we're back on primary.

- [x] **`altscreen_cursor.teseq`** — Cursor position saved/restored:
  ```
  : Esc [ 10 ; 20 H
  : Esc [ ? 1049 h
  : Esc [ 1 ; 1 H
  |alt|
  : Esc [ ? 1049 l
  ```
  `altscreen_cursor.toml`: `[expect] cursor = { col = 19, line = 9 }`
  (Cursor restored to pre-alt-screen position: CUP 10;20 is 1-based → 0-based line=9, col=19)

- [x] **`altscreen_content_isolation.teseq`** — Alt screen doesn't bleed to primary:
  ```
  |Primary|.
  : Esc [ ? 1049 h
  |AAAAAAAAAA|.
  |BBBBBBBBBB|.
  : Esc [ ? 1049 l
  ```
  Grid snapshot confirms primary content preserved, alt content gone.

- [x] **`altscreen_mode_leakage.teseq`** — Mode flags survive alt screen swap:
  ```
  : Esc [ ? 6 h
  : Esc [ ? 7 l
  : Esc [ 4 h
  : Esc [ ? 1049 h
  |alt content|
  : Esc [ ? 1049 l
  ```
  Before entering alt: enable DECOM (`?6h`), disable DECAWM (`?7l`), enable IRM (`4h`). After roundtrip, verify all three flags survived:
  - **Assert `assert_mode_contains(outcome, TermMode::ORIGIN)`**
  - **Assert `assert_mode_not_contains(outcome, TermMode::LINE_WRAP)`**
  - **Assert `assert_mode_contains(outcome, TermMode::INSERT)`**
  Grid content: primary screen should be blank (nothing written before alt switch). The grid snapshot is secondary here — the mode flag assertions are the primary value.

- [x] **`altscreen_reentry_1049.teseq`** — Alt screen enter → write → exit → re-enter via mode 1049:
  ```
  : Esc [ ? 1049 h
  |First visit|.
  : Esc [ ? 1049 l
  |Primary restored|.
  : Esc [ ? 1049 h
  |Second visit|
  ```
  Mode 1049 calls `swap_alt()` which does NOT clear the alt grid on entry. On re-entry, the alt grid retains content from the first visit. Grid snapshot shows "First visit" on row 0 AND "Second visit" on the row where the restored alt cursor lands. **Assert `assert_mode_contains(outcome, TermMode::ALT_SCREEN)`**. This is the expected DEC behavior — only mode 1047 clears on entry.

- [x] **`altscreen_reentry_1047.teseq`** — Alt screen enter → write → exit → re-enter via mode 1047:
  ```
  : Esc [ ? 1047 h
  |First visit|.
  : Esc [ ? 1047 l
  : Esc [ ? 1047 h
  |Second visit|
  ```
  Mode 1047 calls `swap_alt_clear()` which resets the alt grid on entry. On re-entry, "First visit" content is gone. Grid snapshot shows only "Second visit" on a clean alt screen. **Assert `assert_mode_contains(outcome, TermMode::ALT_SCREEN)`**. Contrast with `altscreen_reentry_1049` above — this validates the behavioral difference between modes 1049 and 1047.

- [x] Multi-size variant: `altscreen_roundtrip` at 97x33. Verifies alt screen swap works correctly at non-80x24 sizes — the alt grid is lazily allocated at the current grid dimensions, so this confirms `ensure_alt_grid()` uses the correct size.

---

## 04.4 Insert Mode & Wrap Scenarios

**File(s):** `oriterm_core/tests/teseq/scenarios/csi/modes/insert_wrap_*.teseq`, `oriterm_core/tests/teseq/scenarios/csi/modes/irm_*.teseq`, `oriterm_core/tests/teseq/scenarios/csi/modes/wrap_*.teseq` (scenario files), `oriterm_core/tests/teseq/mode_interactions.rs` (test functions — grouped under `// Insert mode & wrap` section comment)

- [x] **`irm_insert.teseq`** — Insert mode shifts characters right:
  ```
  |ABCDEFGHIJ|
  : Esc [ 1 ; 4 H
  : Esc [ 4 h
  |XY|
  : Esc [ 4 l
  ```
  CUP 1;4 (1-based) → 0-based line=0, col=3. Enable IRM (`CSI 4h`), type "XY" at col 3, existing chars shift right. Characters past the right edge are lost. Grid shows "ABCXYDEFGH" on row 0 (J pushed off the right edge at col 79, or truncated at actual content end).

- [x] **`irm_at_right_margin.teseq`** — IRM insert when cursor is near the right margin:
  ```
  : Esc [ 1 ; 78 H
  |AB|
  : Esc [ 1 ; 78 H
  : Esc [ 4 h
  |XY|
  : Esc [ 4 l
  ```
  `irm_at_right_margin.toml`: `[expect] cursor = { col = 79, line = 0 }`
  CUP 1;78 → col 77. Write "AB" at cols 77-78. Then reposition to col 77, enable IRM, insert "XY". With IRM on, existing "AB" shifts right — "B" pushes off the right edge. Grid shows "XY" at cols 77-78, "A" at col 79. Cursor at col 79, line 0 — no wrap occurred.

- [x] **`irm_wide_char.teseq`** — IRM insert with wide (CJK) character:
  ```
  |ABCDEFGHIJ|
  : Esc [ 1 ; 4 H
  : Esc [ 4 h
  ```
  After the CSI IRM enable, feed a wide CJK character (e.g., U+597D "好"). The teseq format for this: `|好|` (literal UTF-8 in a text line). `insert_blank(2)` should shift "DEFGHIJ" right by 2 positions, then the wide char occupies cols 3-4.
  Full `.teseq`:
  ```
  |ABCDEFGHIJ|
  : Esc [ 1 ; 4 H
  : Esc [ 4 h
  |好|
  : Esc [ 4 l
  ```
  Grid shows "ABC好DEFGH" on row 0 (IJ pushed off right edge by the 2-cell-wide insert).

- [x] **`wrap_at_margin.teseq`** — Auto-wrap at right margin:
  ```
  : Esc [ 1 ; 1 H
  |AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAXX|
  ```
  The text line must contain exactly 80 A's + "xx" (82 chars total). At 80 columns with DECAWM on (default), the 80 A's fill row 0, then "xx" wraps to row 1. **Verify the A count is exactly 80.**
  Grid snapshot shows 80 chars on row 0, "xx" on row 1.

- [x] **`wrap_disabled.teseq`** — DECAWM off prevents wrap:
  ```
  : Esc [ ? 7 l
  : Esc [ 1 ; 1 H
  |AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAXX|
  ```
  `wrap_disabled.toml`: `[expect] cursor = { col = 79, line = 0 }`
  The text line must contain 80+ characters. With DECAWM off, characters beyond column 79 overwrite at column 79 (see `Handler::input` lines 46-51: cursor snapped back to last column). Grid snapshot shows only 80 chars on row 0, with the last char being 'x' (overwritten at col 79). Cursor stays at col 79, line 0 — no wrap occurred.

- [x] **`wrap_disabled_wide_char.teseq`** — Wide char at right margin with DECAWM off:
  ```
  : Esc [ ? 7 l
  : Esc [ 1 ; 1 H
  |AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA|
  : Esc [ 1 ; 80 H
  |好X|
  ```
  First line: exactly 78 A's (fills cols 0-77). CUP 1;80 → col 79 (last column). Feed wide char "好" (width 2) — it doesn't fit (col 79 + 2 > 80), so cursor is set to wrap-pending (`col = cols`). Then "X" is fed: since DECAWM is off, cursor snaps back to col 79, "X" overwrites. Grid shows row 0 with 78 A's followed by a space at col 78 and "X" at col 79. No wrap to row 1.

- [x] Multi-size variant: `wrap_at_margin` at 120x40 to verify wrap behavior at non-80-column widths. Adjust A count to exactly 120.

---

## 04.5 Cross-Cutting Mode Interactions

**File(s):** `oriterm_core/tests/teseq/scenarios/csi/modes/cross_*.teseq` (scenario files), `oriterm_core/tests/teseq/mode_interactions.rs` (test functions — grouped under `// Cross-cutting` section comment)

These scenarios test mode combinations that span multiple subsections. Each exercises a real regression surface where bugs have historically appeared in terminal emulators.

- [x] **`cross_deccolm_with_decom_decstbm.teseq`** — DECCOLM while DECOM and DECSTBM are active:
  ```
  : Esc [ 5 ; 20 r
  : Esc [ ? 6 h
  : Esc [ 3 ; 10 H
  |Before DECCOLM|
  : Esc [ ? 3 h
  : Esc [ 1 ; 1 H
  |After DECCOLM|
  ```
  `cross_deccolm_with_decom_decstbm.toml`:
  ```toml
  [setup]
  pre_feed = ["\\x1b[?40h"]
  ```
  `apply_deccolm()` resets the scroll region to full screen (`set_scroll_region(1, None)`) and homes cursor via `goto_origin_aware(0, 0)`. With DECOM still active after DECCOLM, cursor homes to the new (full-screen) region top. "Before DECCOLM" content is cleared. "After DECCOLM" should appear at row 0, col 0. **Assert `outcome.cols == 132`** and **`assert_mode_contains(outcome, TermMode::ORIGIN)`** — DECOM flag survives DECCOLM.

- [x] **`cross_1049_with_decom_decstbm.teseq`** — Alt screen with active DECOM and scroll region:
  ```
  : Esc [ 5 ; 20 r
  : Esc [ ? 6 h
  : Esc [ 3 ; 1 H
  |Origin content|
  : Esc [ ? 1049 h
  : Esc [ 1 ; 1 H
  |Alt in origin mode|
  : Esc [ ? 1049 l
  ```
  DECOM is set before entering alt screen. On alt screen, DECOM persists (TermMode flags are global, not per-grid). However, the scroll region is per-Grid state — the alt grid starts with a full-screen scroll region (0..rows), independent of the primary grid's DECSTBM 5;20 region. So CUP 1;1 on the alt screen with DECOM active goes to absolute row 0 (alt grid's region top), not row 4 (primary's region top). After returning to primary, cursor restores and primary content ("Origin content" at absolute row 6) is intact. **Assert primary grid content preserved** and **`assert_mode_contains(outcome, TermMode::ORIGIN)`**.

- [x] **`cross_irm_at_margin_with_decawm.teseq`** — IRM insert at right margin with DECAWM on:
  ```
  : Esc [ ? 7 h
  : Esc [ 1 ; 79 H
  |AB|
  : Esc [ 1 ; 79 H
  : Esc [ 4 h
  |XY|
  : Esc [ 4 l
  ```
  CUP 1;79 → col 78. Write "AB": 'A' at col 78, 'B' at col 79 (wrap-pending after). Reposition to col 78, enable IRM, insert "XY". With IRM + DECAWM on: insert at col 78 shifts 'A' and 'B' right. 'B' is pushed past col 79 and wraps to next line (or is lost — depends on whether IRM insert triggers wrap). Grid snapshot captures the actual behavior for golden comparison.

- [x] **`cross_scrollback_after_decstbm_overflow.teseq`** — Verify scrollback integrity after scroll region overflow:
  ```
  |Line A|
  . CR/^M LF/^J
  |Line B|
  . CR/^M LF/^J
  |Line C|
  : Esc [ 2 ; 3 r
  : Esc [ 2 ; 1 H
  |X1|
  . CR/^M LF/^J
  |X2|
  . CR/^M LF/^J
  |X3|
  ```
  `cross_scrollback_after_decstbm_overflow.toml`:
  ```toml
  [terminal]
  rows = 5
  scrollback = 10
  ```
  Write 3 lines (rows 0-2 of 5-row terminal). Set sub-region DECSTBM 2;3 (0-based rows 1-2). CUP to region start, write 3 lines within sub-region, causing 1 scroll within the region. Since it's a sub-region scroll, **assert `scrollback_len == 0`** — no content should leak to scrollback. Grid snapshot shows: row 0 "Line A" (above region, untouched), row 1 "X2" (scrolled up within region), row 2 "X3" (last written, region bottom), rows 3-4 empty (below region, untouched).

- [x] **`cross_scrollback_after_deccolm_clear.teseq`** — Verify DECCOLM clear doesn't pollute scrollback:
  ```
  |Line 1|
  . CR/^M LF/^J
  |Line 2|
  . CR/^M LF/^J
  |Line 3|
  : Esc [ ? 3 h
  |After DECCOLM|
  ```
  `cross_scrollback_after_deccolm_clear.toml`:
  ```toml
  [setup]
  pre_feed = ["\\x1b[?40h"]
  [terminal]
  scrollback = 10
  ```
  DECCOLM clear uses `erase_display(All)` which clears the viewport but should NOT push erased content to scrollback. **Assert `scrollback_len == 0`**.

---

## 04.R Third Party Review Findings

- [x] `[TPR-04-001][medium]` `plans/teseq-conformance/section-04-mode-interactions.md:97`, `plans/teseq-conformance/section-04-mode-interactions.md:148`, `oriterm_core/tests/teseq/harness/mod.rs:11`, `oriterm_core/tests/teseq/main.rs:19`, `oriterm_core/tests/teseq/csi_reports.rs:10` — Section 04 is missing the concrete test-entry scaffolding needed to execute the new custom assertions.
  Validation: The existing harness exports only the current assertion helpers via `harness/mod.rs`, so `assert_mode_contains`, `assert_mode_not_contains`, and `assert_grid_cols` will not be callable until that file is updated. The existing test suite also requires every scenario family to have a registered module in `main.rs` and, when per-scenario custom assertions are needed, a concrete `run_scenario() -> ScenarioOutcome` helper like `csi_reports.rs`. Section 04 currently names `mode_interactions.rs` only as a file target in 04.1 and scopes 04.0 to `runner.rs`/`assertions.rs`, but it never explicitly creates `mode_interactions.rs`, registers `mod mode_interactions;`, creates `scenarios/csi/modes/`, or re-exports the new helpers from `harness/mod.rs`.
  **Resolution:** Added 04.0a (scaffolding subsection) with explicit steps: create `scenarios/csi/modes/` directory, create `mode_interactions.rs` with `run_scenario` helper, register mod in `main.rs`, re-export helpers in `harness/mod.rs`.

- [x] `[TPR-04-002][medium]` `plans/teseq-conformance/section-04-mode-interactions.md:146`, `plans/teseq-conformance/section-04-mode-interactions.md:240`, `plans/teseq-conformance/section-04-mode-interactions.md:332`, `plans/teseq-conformance/section-04-mode-interactions.md:504`, `plans/teseq-conformance/section-06-workflows.md:79` — Section 04 now overlaps heavily with Section 06.1, so ownership between “interaction scenarios” and “workflow scenarios” is no longer crisp.
  Validation: Section 04 now owns origin overflow, DECCOLM transitions, alt-screen mode interactions, and cross-cutting combinations. Section 06.1 still owns near-duplicate mode-combination workflows for the same families: `mode_scroll_origin_fill`, `mode_deccolm_full_cycle`, and `mode_alt_with_modes`. That leaves both sections claiming the same behavioral surface with different granularity, which is exactly the kind of plan drift that causes duplicated scenario authoring and ambiguous resume points.
  **Resolution:** Added boundary delineation note in section intro. Section 04 = isolated mode interactions (single combination, focused assertions). Section 06.1 = multi-step workflow narratives (3+ mode operations chained into realistic usage sequences). The 06.1 scenarios are deliberately more complex and build on 04’s validated primitives.

- [x] `[TPR-04-003][low]` `plans/teseq-conformance/section-04-mode-interactions.md:72`, `plans/teseq-conformance/section-04-mode-interactions.md:627`, `plans/teseq-conformance/section-07-verification.md:75` — Downstream verification counts are stale after the Section 04 expansion.
  Validation: Section 04 now targets 34+ mode scenarios, but Section 07 still budgets the entire `csi/modes/` family as `12+`. If Section 04 scope is accepted as written, Section 07’s coverage matrix and total-scenario accounting will under-report the planned test surface.
  **Resolution:** (1) Section-04 counts updated to 34+ (9+6+7+7+5 with expanded multi-size variants), (2) section-07 updated to CSI Modes: 34+ and total: 106+, (3) completion checklist includes plan sync step to verify section-07 counts.

- [x] `[TPR-04-004][low]` `plans/teseq-conformance/section-04-mode-interactions.md:105`, `plans/teseq-conformance/section-04-mode-interactions.md:110`, `plans/teseq-conformance/section-04-mode-interactions.md:121`, `plans/teseq-conformance/section-04-mode-interactions.md:309`, `oriterm_core/tests/teseq/mode_interactions.rs:14`, `oriterm_core/tests/teseq/mode_interactions.rs:16`, `oriterm_core/tests/teseq/mode_interactions.rs:34` — Section 04 still documents the pre-refactor `run_scenario() -> ScenarioOutcome` helper and per-test `reseq_available()` guards, but the committed implementation now centralizes graceful skip handling inside `run_scenario()` and returns `Option<ScenarioOutcome>`.
  Resolved: Fixed on 2026-04-06. Updated 04.0a scaffold snippet to show `Option<ScenarioOutcome>` return with centralized `reseq_available()` guard and `eprintln!` skip message. Updated 04.1 multi-size guidance to document `let Some(outcome) = ... else { return; }` pattern. Removed stale import snippet after 04.0b.

---

## 04.N Completion Checklist

- [x] **Scaffolding** (04.0a): `scenarios/csi/modes/` directory created, `mode_interactions.rs` created with `run_scenario` helper, `mod mode_interactions;` registered in `main.rs`, compilation verified
- [x] **Harness extended** (04.0b): `ScenarioOutcome.mode` field added in `runner.rs`, `assert_mode_contains`, `assert_mode_not_contains`, `assert_grid_cols` added in `assertions.rs`, all three re-exported in `harness/mod.rs` (3 new helpers)
- [x] Origin mode + scroll region scenarios: basic, overflow, cursor save/restore, IL/DL in region, multi-size x4 (9 scenarios)
- [x] DECCOLM scenarios: 80→132, 132→80, wrap interaction, scroll region reset, no-Mode-40 negative control, multi-size 120x40 (6 scenarios)
- [x] Alt screen scenarios: roundtrip, cursor, content isolation, mode leakage, re-entry 1049, re-entry 1047, multi-size 97x33 (7 scenarios)
- [x] Insert mode and wrap scenarios: IRM insert, IRM at margin, IRM wide char, wrap at margin, wrap disabled, wrap disabled wide char, multi-size wrap (7 scenarios)
- [x] Cross-cutting scenarios: DECCOLM+DECOM+DECSTBM, 1049+DECOM+DECSTBM, IRM+margin+DECAWM, scrollback after DECSTBM overflow, scrollback after DECCOLM clear (5 scenarios)
- [x] Multi-size variants for mode interactions across all subsections (97x33, 120x40)
- [x] Mode flag assertions used in alt screen, DECCOLM, and cross-cutting scenarios
- [x] Scrollback integrity assertions used in overflow and clear scenarios
- [x] **File size check**: `mode_interactions.rs` must not exceed 500 lines (CLAUDE.md rule). If approaching the limit after 04.4, split into submodules (e.g., `mode_interactions/mod.rs` + `mode_interactions/origin.rs` + `mode_interactions/deccolm.rs` etc.) per test-organization.md. With 34 test functions at ~10 lines each plus the `run_scenario` helper, estimated ~380 lines — within budget but monitor during 04.5.
- [x] 34+ total mode interaction scenarios pass (9+6+7+7+5)
- [x] All insta snapshots reviewed for correctness
- [x] `./build-all.sh` green, `./clippy-all.sh` green
- [x] `timeout 150 ./test-all.sh` green — no regressions
- [x] Plan annotation cleanup
- [x] All TPR checkpoint findings resolved
- [ ] **Plan sync** — update plan metadata:
  - [ ] This section's frontmatter `status` → `complete`
  - [x] `00-overview.md` Quick Reference table updated
  - [x] `index.md` section status updated
  - [x] `section-07-verification.md` scenario count updated (CSI Modes: 34+, total: 106+)
- [ ] `/tpr-review` passed (final, full-section)
- [ ] `/impl-hygiene-review last commit` passed

**Exit Criteria:** `timeout 150 cargo test -p oriterm_core --test teseq -- mode_interactions` passes with 34+ scenarios testing mode combinations. DECOM+DECSTBM, DECCOLM transitions (including negative control without Mode 40), alt screen roundtrips (including mode leakage, 1049 vs 1047 re-entry semantics), IRM edge cases (margin, wide chars), and cross-cutting mode combinations all validated at multiple sizes. Mode flags verified via TermMode assertions, not just grid content. Scrollback integrity checked after sub-region overflow and DECCOLM clear. These scenarios serve as permanent regression guards for the bugs fixed during vttest conformance. Zero regressions.
