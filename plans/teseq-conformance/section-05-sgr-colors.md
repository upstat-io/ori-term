---
section: "05"
title: "SGR & Color Scenarios"
status: in-progress
reviewed: true
goal: "Create comprehensive SGR scenarios covering text attributes, underline styles, underline colors, all color modes (16, 256, TrueColor), color resolution edge cases (bold-as-bright, DIM priority, inverse, DECSCNM), and selective resets — with rendered cell attribute validation"
success_criteria:
  - "Basic attribute scenarios: bold, dim, italic, underline, blink (slow + fast), inverse, hidden, strikethrough"
  - "Underline style scenarios: single, double, curly, dotted, dashed — mutually exclusive via ALL_UNDERLINES, plus SGR 4:0 cancel sub-param"
  - "Underline color scenarios: SGR 58 (set) and SGR 59 (reset) with 256 and TrueColor"
  - "16-color scenarios: foreground and background for all 8 base + 8 bright colors"
  - "256-color scenarios: indexed colors via SGR 38;5;N and 48;5;N"
  - "TrueColor scenarios: RGB colors via SGR 38;2;R;G;B and 48;2;R;G;B"
  - "Bold-as-bright promotion tested: bold + ANSI color 0-7 promotes to bright color 8-15"
  - "DIM + bold interaction: DIM takes priority over bold-as-bright — no bright promotion when DIM is set"
  - "Selective reset scenarios: SGR 0/21/22/23/24/25/27/28/29/39/49/59 each tested individually"
  - "Inverse + explicit colors: fg/bg swap verified via resolved Rgb"
  - "DECSCNM + SGR inverse cross-cutting: double-swap produces normal appearance"
  - "CellFlags assertions use contains() — never exact equality (non-SGR flags may be present)"
  - "Parameterless SGR (CSI m with no params) tested as equivalent to SGR 0 reset"
  - "Satisfies mission criteria: 16/256/TrueColor, bold-as-bright, dim, inverse, underline styles, attribute coverage"
inspired_by:
  - "ori_term term/handler/sgr.rs — SGR dispatch and color parsing"
  - "ori_term term/handler/tests.rs — 40+ SGR attribute tests including underline styles, cancel codes, underline colors"
  - "ori_term term/renderable/mod.rs — resolve_fg, resolve_bg, apply_inverse color resolution"
  - "ori_term term/renderable/tests.rs — color resolution tests including bold+dim, DECSCNM, inverse"
  - "WezTerm term/src/test/csi.rs — color palette and attribute tests"
depends_on: ["01", "02"]
third_party_review:
  status: findings
  updated: 2026-04-06
sections:
  - id: "05.0"
    title: "Scaffolding & Harness Extension"
    status: complete
  - id: "05.1"
    title: "Text Attribute Scenarios"
    status: complete
  - id: "05.2"
    title: "Underline Style & Color Scenarios"
    status: complete
  - id: "05.3"
    title: "16-Color & Bold-as-Bright Scenarios"
    status: complete
  - id: "05.4"
    title: "256-Color & TrueColor Scenarios"
    status: complete
  - id: "05.5a"
    title: "Selective Attribute Resets (SGR 21-29)"
    status: complete
  - id: "05.5b"
    title: "Default Color & Template Resets (SGR 0/39/49/59)"
    status: complete
  - id: "05.6"
    title: "Color Resolution Edge Cases"
    status: complete
  - id: "05.7"
    title: "Attribute Stacking & Combination Scenarios"
    status: complete
  - id: "05.R"
    title: "Third Party Review Findings"
    status: complete
  - id: "05.N"
    title: "Completion Checklist"
    status: not-started
---

# Section 05: SGR & Color Scenarios

**Status:** In Progress
**Goal:** Comprehensive SGR coverage through teseq scenarios. While handler tests validate SGR parsing at the byte level, these scenarios validate the *rendered result* — CellFlags and resolved colors as they appear in `RenderableContent`. This catches bugs in the color resolution pipeline (bold-as-bright promotion, dim application, inverse swapping, DECSCNM interaction, underline color resolution) that byte-level tests miss.

**Success Criteria:**

- [x] All 9 basic text attributes tested via cell flag inspection (8 base + blink_fast)
- [x] All 5 underline styles tested (single, double, curly, dotted, dashed) with mutual exclusion + 4:0 cancel sub-param
- [x] Underline colors tested (SGR 58 set, SGR 59 reset) via `cell_underline_color_at`
- [x] 16-color foreground/background tested with correct Rgb resolution
- [x] 256-color indexed colors tested
- [x] TrueColor RGB colors tested
- [x] Bold-as-bright color promotion validated
- [x] DIM + bold interaction validated (DIM takes priority, no bright promotion)
- [x] All selective resets tested (SGR 21/22/23/24/25/27/28/29/39/49/59)
- [x] Inverse + DECSCNM cross-cutting validated
- [x] CellFlags assertions use `contains()` pattern throughout
- [x] Attribute stacking, reset, and parameterless SGR (CSI m) validated
- [x] 40+ SGR scenarios pass (target: ~55)

**Context:** SGR scenarios differ from other scenario types because the *grid text* doesn't change based on attributes — "hello" looks the same whether it's bold or not in `grid_text()`. Instead, these scenarios need to inspect `RenderableCell` attributes (flags, fg, bg, underline_color). The harness needs cell attribute inspection helpers beyond plain text comparison.

**Important implementation note — CellFlags assertions:** `CellFlags` contains both SGR flags (BOLD, DIM, ITALIC, etc.) and internal grid flags (WIDE_CHAR, WIDE_CHAR_SPACER, WRAP, LEADING_WIDE_CHAR_SPACER). Assertions **must** use `flags.contains(CellFlags::BOLD)` — never `flags == CellFlags::BOLD`. Exact equality will fail when non-SGR flags are incidentally set.

**Important implementation note — bold_is_bright config:** The `TeseqHarness` constructor calls `Term::new()` which sets `bold_is_bright: true` by default (`term/mod.rs:233`). Testing `bold_is_bright: false` requires calling `term.set_bold_is_bright(false)`. Section 05.0 adds a `TeseqHarness::set_bold_is_bright()` method to support this.

**Reference implementations:**
- **ori_term** `term/handler/sgr.rs`: SGR dispatch — maps `Attr` variants to CellFlags and colors. 62 lines, handles all underline styles, all cancel codes, and underline colors.
- **ori_term** `term/handler/tests.rs`: 40+ SGR tests covering every attribute, every cancel code, underline styles, underline colors (truecolor and 256), and colon-separator variants.
- **ori_term** `term/renderable/mod.rs`: `resolve_fg()`, `resolve_bg()`, `apply_inverse()` — the color resolution pipeline. `resolve_fg` applies bold-as-bright and dim, with DIM taking priority when both are set.
- **ori_term** `term/renderable/tests.rs`: Color resolution tests including bold+dim interaction (5 tests), DECSCNM reverse video (3 tests), inverse swap.
- **ori_term** `term/snapshot.rs`: `renderable_content_into()` — DECSCNM palette swap, underline color resolution via `palette.resolve()`.
- **ori_term** `cell/mod.rs`: CellFlags bitflags — 16 flags including 5 underline variants with `ALL_UNDERLINES` mutual exclusion mask. `CellExtra` — `underline_color: Option<Color>` stored in heap-allocated optional data.

**Depends on:** Section 01 (TeseqHarness), Section 02 (basic pattern).

**File size note:** The `sgr.rs` test module will contain ~55 test functions at ~5-7 lines each, totaling ~350-400 lines. Test files are exempt from the 500-line limit per `code-hygiene.md`. No submodule split needed.

---

## 05.0 Scaffolding & Harness Extension

This subsection has two parts: (A) creating the module and directory scaffolding that 05.1-05.7 depend on, and (B) extending the harness with cell attribute inspection helpers and the `set_bold_is_bright` setter.

### 05.0a Module & Directory Scaffolding

**Why first:** Every test function in 05.1-05.7 lives in `sgr.rs` and every `.teseq` file lives in `scenarios/csi/sgr/`. These must exist and be wired up before any scenario work begins.

- [x] **Create scenario directory** `oriterm_core/tests/teseq/scenarios/csi/sgr/`. This is the home for all `.teseq` and `.toml` sidecar files in this section. Currently `scenarios/csi/` contains `cursor/`, `erase/`, `insert_delete/`, `reports/`, `modes/` — `sgr/` does not yet exist.

- [x] **Create `oriterm_core/tests/teseq/sgr.rs`** — the Rust test module for all SGR scenarios. Follow the `Option<ScenarioOutcome>` pattern established by `mode_interactions.rs` and `csi_reports.rs`, since every SGR test needs to perform cell-level assertions beyond the sidecar spec:
  ```rust
  //! SGR & color scenarios (attributes, underline styles, colors, selective resets).

  use std::path::Path;

  use vte::ansi::Color;

  use oriterm_core::cell::CellFlags;
  use oriterm_core::color::{Palette, Rgb};

  use super::harness::{
      self, ScenarioOutcome, TeseqHarness, assert_cell_flags_contain,
      assert_cell_flags_not_contain, cell_bg_at, cell_fg_at,
      cell_underline_color_at, reseq_available,
  };

  /// Run an SGR scenario and apply spec assertions.
  ///
  /// Returns `None` when `reseq` is unavailable (graceful skip with visible message).
  /// Returns the outcome for callers to perform cell attribute assertions.
  fn run_scenario(name: &str) -> Option<ScenarioOutcome> {
      if !reseq_available() {
          eprintln!("reseq not installed, skipping");
          return None;
      }
      let path = Path::new(env!("CARGO_MANIFEST_DIR"))
          .join("tests/teseq/scenarios/csi/sgr")
          .join(format!("{name}.teseq"));
      let mut h = TeseqHarness::from_scenario(&path);
      let outcome = h.run(&path);
      harness::assert_spec(&outcome, h.spec(), &format!("sgr_{name}"));
      Some(outcome)
  }
  ```
  Note: imports include `vte::ansi::Color` and `oriterm_core::color::Palette` — needed by 05.3+ color tests to construct expected Rgb values via `Palette::default().resolve(Color::Indexed(N))`. The import grouping follows code-hygiene.md: std, external (`vte`), internal (`oriterm_core`, `super`).
  Note the scenario path uses `scenarios/csi/sgr` (matching the directory created above) and the snapshot prefix uses `sgr_` (matching the module name). The `run_scenario` helper centralizes the `reseq_available()` guard and returns `Option<ScenarioOutcome>` — callers use `let Some(outcome) = run_scenario(...) else { return; };` for cell attribute assertions.

- [x] **Register the module in `oriterm_core/tests/teseq/main.rs`** — add `mod sgr;` in a new "Family modules (Section 05)" comment block:
  ```rust
  // Family modules (Section 05).
  mod sgr;
  ```
  Place this after the Section 04 block (`mod mode_interactions;`).

- [x] **Verify compilation** — `timeout 150 cargo test -p oriterm_core --test teseq -- sgr` should compile (no tests yet, so zero tests run).

### 05.0b Harness Extension: Cell Attribute Inspection Helpers

**File(s):** `oriterm_core/tests/teseq/harness/assertions.rs`, `oriterm_core/tests/teseq/harness/runner.rs`, `oriterm_core/tests/teseq/harness/mod.rs`

The current assertion helpers inspect grid text, cursor position, mode flags, and scrollback — but NOT cell attributes (flags, fg, bg, underline_color). `ScenarioOutcome` already stores `cells: Vec<RenderableCell>` (populated in `runner.rs:106`). This sub-step adds helpers to query individual cells.

- [x] **Add cell attribute inspection helpers to `assertions.rs`** — add these after the existing `assert_grid_cols` function. Add `use oriterm_core::cell::CellFlags;`, `use oriterm_core::color::Rgb;`, and `use oriterm_core::term::renderable::RenderableCell;` to the imports:
  ```rust
  /// Find the RenderableCell at (line, col) in the outcome's cell list.
  ///
  /// `ScenarioOutcome::cells` is `Vec<RenderableCell>` (row-major).
  /// `RenderableCell` has `line: usize` and `column: Column`. Linear scan
  /// is fine — test grids are small (80x24 = 1920 cells max).
  fn find_cell(outcome: &ScenarioOutcome, line: usize, col: usize) -> &RenderableCell {
      outcome.cells.iter()
          .find(|c| c.line == line && c.column.0 == col)
          .unwrap_or_else(|| panic!("no cell at line={line}, col={col}"))
  }

  /// Assert a cell's CellFlags contain the expected flags.
  ///
  /// Uses `contains()` — never exact equality — because CellFlags
  /// includes non-SGR flags (WIDE_CHAR, WRAP, etc.) that may be
  /// incidentally set.
  pub fn assert_cell_flags_contain(
      outcome: &ScenarioOutcome,
      line: usize,
      col: usize,
      expected: CellFlags,
  ) {
      let cell = find_cell(outcome, line, col);
      assert!(
          cell.flags.contains(expected),
          "cell ({line},{col}) flags {:?} missing expected {:?}",
          cell.flags, expected
      );
  }

  /// Assert a cell's CellFlags do NOT contain the specified flags.
  pub fn assert_cell_flags_not_contain(
      outcome: &ScenarioOutcome,
      line: usize,
      col: usize,
      unexpected: CellFlags,
  ) {
      let cell = find_cell(outcome, line, col);
      assert!(
          !cell.flags.intersects(unexpected),
          "cell ({line},{col}) flags {:?} unexpectedly contain {:?}",
          cell.flags, unexpected
      );
  }

  /// Get the foreground Rgb for a specific cell.
  pub fn cell_fg_at(outcome: &ScenarioOutcome, line: usize, col: usize) -> Rgb {
      find_cell(outcome, line, col).fg
  }

  /// Get the background Rgb for a specific cell.
  pub fn cell_bg_at(outcome: &ScenarioOutcome, line: usize, col: usize) -> Rgb {
      find_cell(outcome, line, col).bg
  }

  /// Get the underline color for a specific cell.
  ///
  /// Returns `None` when no custom underline color is set (SGR 59 or default).
  /// Returns `Some(Rgb)` when SGR 58 has set a custom underline color.
  pub fn cell_underline_color_at(
      outcome: &ScenarioOutcome,
      line: usize,
      col: usize,
  ) -> Option<Rgb> {
      find_cell(outcome, line, col).underline_color
  }
  ```
  Note: `find_cell` is private (not `pub`) — only the `pub` helpers are exported. Flag assertions use `contains()` and `intersects()` instead of exact equality. This is critical because `CellFlags` includes non-SGR flags (WIDE_CHAR=0x100, WIDE_CHAR_SPACER=0x200, WRAP=0x400, LEADING_WIDE_CHAR_SPACER=0x8000) that could be set on cells for reasons unrelated to SGR.

- [x] **Re-export new helpers from `harness/mod.rs`** — add to the existing `pub use assertions::{...}` line:
  ```rust
  pub use assertions::{
      analyze_response, assert_cell_flags_contain, assert_cell_flags_not_contain,
      assert_cursor, assert_event_snapshot, assert_grid_cols, assert_grid_snapshot,
      assert_mode_contains, assert_mode_not_contains, assert_pty_writes,
      assert_response_snapshot, assert_scrollback_empty, assert_spec,
      cell_bg_at, cell_fg_at, cell_underline_color_at, pipe_through_command,
  };
  ```

### 05.0c Harness Extension: `set_bold_is_bright` Setter

**File:** `oriterm_core/tests/teseq/harness/runner.rs`

The `TeseqHarness` constructor always creates `Term` with the default `bold_is_bright: true` (set in `term/mod.rs:233`). Section 05.3 needs to test the `bold_is_bright: false` code path in `resolve_fg()`. This adds a setter.

- [x] **Add `set_bold_is_bright` method to `TeseqHarness`** in `runner.rs`, after the existing `spec()` method:
  ```rust
  /// Toggle bold-as-bright color promotion.
  ///
  /// Default is `true` (set in `Term::new`). Call with `false` to test
  /// the code path where bold does not promote ANSI colors 0-7 to 8-15.
  pub fn set_bold_is_bright(&mut self, enabled: bool) {
      self.term.set_bold_is_bright(enabled);
  }
  ```

- [x] **Verify harness compiles** — `timeout 150 cargo test -p oriterm_core --test teseq` should pass with zero new failures (new helpers are unused until 05.1).

---

## 05.1 Text Attribute Scenarios

**File(s):** `oriterm_core/tests/teseq/scenarios/csi/sgr/attr_*.teseq`, `oriterm_core/tests/teseq/sgr.rs`

Create scenarios for all 8 basic SGR attributes plus the BlinkFast equivalence.

- [x] Create attribute scenarios:

  **`attr_bold.teseq`**:
  ```
  : Esc [ 1 m
  |Bold text|
  : Esc [ 0 m
  ```

  **`attr_dim.teseq`**:
  ```
  : Esc [ 2 m
  |Dim text|
  : Esc [ 0 m
  ```

  **`attr_italic.teseq`**, **`attr_underline.teseq`**, **`attr_blink.teseq`**, **`attr_inverse.teseq`**, **`attr_hidden.teseq`**, **`attr_strikethrough.teseq`** — same pattern with SGR codes 3, 4, 5, 7, 8, 9 respectively.

- [x] **`attr_blink_fast.teseq`** — SGR 6 (BlinkFast) maps to the same `BLINK` flag as SGR 5 (BlinkSlow):
  ```
  : Esc [ 6 m
  |Fast blink|
  : Esc [ 0 m
  ```
  Assert: `BLINK` flag set (both SGR 5 and 6 set the same flag, per `sgr.rs:46`). This validates the `BlinkSlow | BlinkFast` match arm.

- [x] Each scenario test in `sgr.rs` asserts:
  1. Text cells have the correct flag set (via `assert_cell_flags_contain`)
  2. After SGR 0 reset, cells written after reset have the flag cleared (via `assert_cell_flags_not_contain`)
  3. Grid text matches expected content (via insta snapshot)
  Example test shape (all 9 attribute tests follow this pattern):
  ```rust
  #[test]
  fn attr_bold() {
      let Some(outcome) = run_scenario("attr_bold") else { return; };
      // "Bold text" starts at line 0, col 0.
      assert_cell_flags_contain(&outcome, 0, 0, CellFlags::BOLD);
      assert_cell_flags_not_contain(&outcome, 0, 9, CellFlags::BOLD); // after reset
  }
  ```

---

## 05.2 Underline Style & Color Scenarios

**File(s):** `oriterm_core/tests/teseq/scenarios/csi/sgr/underline_*.teseq`

The codebase supports 5 underline styles that are mutually exclusive (setting one clears all others via `ALL_UNDERLINES` mask). SGR 58/59 set/clear a custom underline color stored in `CellExtra`.

- [x] **`underline_single.teseq`** — SGR `4` (or `4:1`):
  ```
  : Esc [ 4 m
  |Single underline|
  : Esc [ 0 m
  ```
  Assert: `UNDERLINE` set, none of `DOUBLE_UNDERLINE | CURLY_UNDERLINE | DOTTED_UNDERLINE | DASHED_UNDERLINE`.

- [x] **`underline_double.teseq`** — SGR `4:2`:
  ```
  : Esc [ 4 : 2 m
  |Double underline|
  : Esc [ 0 m
  ```
  Note: VTE uses colon sub-parameters for underline styles. `[4, 2]` in VTE's param array is the colon-separated form `4:2`, dispatched as `DoubleUnderline`. Semicolon-separated `4;2` produces two separate params `[4]` then `[2]`, dispatched as `Underline` then `Dim` — completely different behavior. The `.teseq` scenario MUST use `4 : 2` (colon, not semicolon). Reseq strips spaces on `: Esc` lines but preserves colons, so `4 : 2` becomes `4:2` in the output byte stream. Do NOT use `4 ; 2` — that would set underline+dim, not double underline. (Note: SGR 21 in VTE is `CancelBold`, not `DoubleUnderline` — see Section 05.5a.)
  Assert: `DOUBLE_UNDERLINE` set, `UNDERLINE` not set.

- [x] **`underline_curly.teseq`** — SGR `4:3`:
  ```
  : Esc [ 4 : 3 m
  |Curly underline|
  : Esc [ 0 m
  ```
  Assert: `CURLY_UNDERLINE` set, `UNDERLINE` not set.

- [x] **`underline_dotted.teseq`** — SGR `4:4`:
  ```
  : Esc [ 4 : 4 m
  |Dotted underline|
  : Esc [ 0 m
  ```
  Assert: `DOTTED_UNDERLINE` set, `UNDERLINE` not set.

- [x] **`underline_dashed.teseq`** — SGR `4:5`:
  ```
  : Esc [ 4 : 5 m
  |Dashed underline|
  : Esc [ 0 m
  ```
  Assert: `DASHED_UNDERLINE` set, `UNDERLINE` not set.

- [x] **`underline_mutual_exclusion.teseq`** — switching styles clears the previous:
  ```
  : Esc [ 4 m
  |S|
  : Esc [ 4 : 3 m
  |C|
  : Esc [ 4 : 2 m
  |D|
  : Esc [ 0 m
  ```
  Assert: 'S' has UNDERLINE only, 'C' has CURLY_UNDERLINE only, 'D' has DOUBLE_UNDERLINE only. Each position has none of the other underline flags.

- [x] **`underline_color_truecolor.teseq`** — SGR 58;2;R;G;B sets underline color:
  ```
  : Esc [ 4 m
  : Esc [ 58 ; 2 ; 255 ; 0 ; 128 m
  |Colored underline|
  : Esc [ 0 m
  ```
  Assert via `cell_underline_color_at()`: `Some(Rgb { r: 255, g: 0, b: 128 })`.
  Note: The underline color in `RenderableCell` is resolved through `palette.resolve()` in `snapshot.rs:132`. For TrueColor `Color::Spec`, this is a passthrough.

- [x] **`underline_color_256.teseq`** — SGR 58;5;N sets indexed underline color:
  ```
  : Esc [ 4 m
  : Esc [ 58 ; 5 ; 196 m
  |Indexed underline|
  : Esc [ 0 m
  ```
  Assert: `cell_underline_color_at()` returns the palette-resolved Rgb for index 196.

- [x] **`underline_color_reset.teseq`** — SGR 59 clears underline color:
  ```
  : Esc [ 4 m
  : Esc [ 58 ; 2 ; 255 ; 0 ; 0 m
  |Red UL|
  : Esc [ 59 m
  |Default UL|
  : Esc [ 0 m
  ```
  Assert: 'R' (col 0) of "Red UL" has `Some(Rgb { r: 255, g: 0, b: 0 })`, 'D' (col 0) of "Default UL" has `None`.

- [x] **`underline_cancel_subparam.teseq`** — SGR `4:0` cancels underline via sub-parameter:
  ```
  : Esc [ 4 : 3 m
  |Curly|
  : Esc [ 4 : 0 m
  |None|
  : Esc [ 0 m
  ```
  Assert: 'C' has `CURLY_UNDERLINE`, 'N' has none of `ALL_UNDERLINES`. This tests the `[4, 0] => CancelUnderline` VTE dispatch path (csi.rs:288), which is distinct from `SGR 24` (`[24] => CancelUnderline`). Both paths should produce the same result — clearing all underline styles via `ALL_UNDERLINES` mask.

- [x] **`underline_color_survives_style_change.teseq`** — underline color persists when style changes:
  ```
  : Esc [ 4 m
  : Esc [ 58 ; 2 ; 0 ; 255 ; 0 m
  |A|
  : Esc [ 4 : 3 m
  |B|
  : Esc [ 0 m
  ```
  Assert: Both 'A' (UNDERLINE) and 'B' (CURLY_UNDERLINE) have underline color `Some(Rgb { r: 0, g: 255, b: 0 })`. This matches the existing handler test `sgr_underline_color_survives_underline_type_change`.

- [ ] **TPR checkpoint** — `/tpr-review` covering 05.1-05.2 implementation work

---

## 05.3 16-Color & Bold-as-Bright Scenarios

**File(s):** `oriterm_core/tests/teseq/scenarios/csi/sgr/color_*.teseq`

- [x] **`color_16_fg.teseq`** — All 8 foreground colors + 8 bright:
  ```
  : Esc [ 30 m
  |blk|
  : Esc [ 31 m
  |red|
  : Esc [ 32 m
  |grn|
  : Esc [ 33 m
  |yel|
  : Esc [ 34 m
  |blu|
  : Esc [ 35 m
  |mag|
  : Esc [ 36 m
  |cyn|
  : Esc [ 37 m
  |wht|
  : Esc [ 90 m
  |Bblk|
  : Esc [ 91 m
  |Bred|
  : Esc [ 0 m
  ```
  Assert each cell group has the correct resolved Rgb from the palette. Use `Palette::default()` in the test to get expected colors and compare via `cell_fg_at()`.

- [x] **`color_16_bg.teseq`** — All 8 background colors + 8 bright:
  ```
  : Esc [ 40 m
  |blk|
  : Esc [ 41 m
  |red|
  : Esc [ 42 m
  |grn|
  : Esc [ 43 m
  |yel|
  : Esc [ 44 m
  |blu|
  : Esc [ 45 m
  |mag|
  : Esc [ 46 m
  |cyn|
  : Esc [ 47 m
  |wht|
  : Esc [ 100 m
  |Bblk|
  : Esc [ 101 m
  |Bred|
  : Esc [ 0 m
  ```
  Assert each cell group has the correct resolved Rgb background from the palette. Use `Palette::default()` in the test to get expected colors and compare via `cell_bg_at()`.

- [x] **`color_bold_bright.teseq`** — Bold + ANSI color 0-7 triggers bright promotion:
  ```
  : Esc [ 1 ; 31 m
  |Bold red|
  : Esc [ 0 m
  ```
  Assert fg resolves to bright red (palette index 9) not normal red (palette index 1).
  `bold_is_bright` defaults to `true` (`term/mod.rs:233`) and the TeseqHarness uses this default.

- [x] **`color_bold_no_promote_above_7.teseq`** — Bold + indexed 100 does NOT promote:
  ```
  : Esc [ 1 m
  : Esc [ 38 ; 5 ; 100 m
  |No promote|
  : Esc [ 0 m
  ```
  Assert fg resolves to palette index 100, not 108.

- [x] **`color_bold_bright_disabled`** — Rust test (not `.teseq` scenario) that creates a harness, calls `set_bold_is_bright(false)` (added in 05.0c), feeds SGR 1;31 + text via `proc.advance()`, and asserts fg resolves to normal red (palette index 1) not bright red:
  ```rust
  #[test]
  fn color_bold_bright_disabled() {
      if !reseq_available() {
          eprintln!("reseq not installed, skipping");
          return;
      }
      let path = Path::new(env!("CARGO_MANIFEST_DIR"))
          .join("tests/teseq/scenarios/csi/sgr/color_bold_bright.teseq");
      let mut h = TeseqHarness::from_scenario(&path);
      h.set_bold_is_bright(false);
      let outcome = h.run(&path);
      // Bold + red with bold_is_bright=false should resolve to normal red,
      // not bright red. palette index 1 = Red, palette index 9 = BrightRed.
      let palette = oriterm_core::color::Palette::default();
      let normal_red = palette.resolve(vte::ansi::Color::Indexed(1));
      assert_eq!(cell_fg_at(&outcome, 0, 0), normal_red);
  }
  ```
  This tests the `bold_is_bright: false` code path in `resolve_fg()` — when `bold_is_bright` is false, the `is_bold` checks in all three color branches are skipped, so bold has no effect on color resolution. Reuses the same `.teseq` file as `color_bold_bright` but with the config toggled. Matches existing renderable tests `bold_is_bright_false_skips_indexed_promotion` and `bold_is_bright_false_skips_named_promotion`.

- [ ] **TPR checkpoint** — `/tpr-review` covering 05.3 implementation work

---

## 05.4 256-Color & TrueColor Scenarios

**File(s):** `oriterm_core/tests/teseq/scenarios/csi/sgr/color_256_*.teseq`, `color_rgb_*.teseq`

- [x] **`color_256_fg.teseq`** — 256-color foreground:
  ```
  : Esc [ 38 ; 5 ; 196 m
  |Red 256|
  : Esc [ 38 ; 5 ; 46 m
  |Green 256|
  : Esc [ 38 ; 5 ; 21 m
  |Blue 256|
  : Esc [ 0 m
  ```
  Assert each cell has correct indexed color resolved to Rgb via `Palette::default().resolve(Color::Indexed(N))`.

- [x] **`color_256_bg.teseq`** — 256-color background (SGR 48;5;N):
  ```
  : Esc [ 48 ; 5 ; 196 m
  |Red bg|
  : Esc [ 48 ; 5 ; 46 m
  |Green bg|
  : Esc [ 48 ; 5 ; 21 m
  |Blue bg|
  : Esc [ 0 m
  ```
  Assert each cell has correct indexed color background resolved to Rgb via `Palette::default().resolve(Color::Indexed(N))`.

- [x] **`color_rgb_fg.teseq`** — TrueColor foreground:
  ```
  : Esc [ 38 ; 2 ; 255 ; 128 ; 0 m
  |Orange|
  : Esc [ 38 ; 2 ; 0 ; 255 ; 128 m
  |Spring|
  : Esc [ 0 m
  ```
  Assert each cell's fg Rgb matches the specified values exactly (TrueColor is passthrough — `Color::Spec(rgb)` resolves to `rgb` without palette lookup).

- [x] **`color_rgb_bg.teseq`** — TrueColor background (SGR 48;2;R;G;B):
  ```
  : Esc [ 48 ; 2 ; 255 ; 128 ; 0 m
  |Orange bg|
  : Esc [ 48 ; 2 ; 0 ; 255 ; 128 m
  |Spring bg|
  : Esc [ 0 m
  ```
  Assert each cell's bg Rgb matches the specified values exactly (TrueColor is passthrough — `Color::Spec(rgb)` resolves to `rgb` without palette lookup).

- [ ] **TPR checkpoint** — `/tpr-review` covering 05.4 implementation work

---

## 05.5a Selective Attribute Resets (SGR 21-29)

**File(s):** `oriterm_core/tests/teseq/scenarios/csi/sgr/reset_*.teseq`

The codebase implements all standard selective SGR resets. Each cancel code has a dedicated `Attr::Cancel*` variant in VTE (`crates/vte/src/ansi/dispatch/csi.rs`, SGR dispatch table) and a corresponding `remove()` call in `term/handler/sgr.rs`. This subsection covers attribute cancel codes (SGR 21-29).

- [x] **`reset_21_cancel_bold.teseq`** — SGR 21 cancels bold only:
  ```
  : Esc [ 1 ; 3 m
  |BI|
  : Esc [ 21 m
  |I|
  : Esc [ 0 m
  ```
  Assert "BI" has BOLD+ITALIC. Assert "I" has ITALIC but not BOLD.
  Note: SGR 21 maps to `Attr::CancelBold` in VTE (line 299), which removes only BOLD. SGR 22 maps to `Attr::CancelBoldDim` which removes BOLD and DIM simultaneously.

- [x] **`reset_22_cancel_bold_dim.teseq`** — SGR 22 cancels both bold and dim:
  ```
  : Esc [ 1 ; 2 m
  |BD|
  : Esc [ 22 m
  |Neither|
  : Esc [ 0 m
  ```
  Assert "BD" has BOLD+DIM. Assert "Neither" has neither BOLD nor DIM.

- [x] **`reset_23_cancel_italic.teseq`** — SGR 23 cancels italic:
  ```
  : Esc [ 3 ; 1 m
  |IB|
  : Esc [ 23 m
  |B|
  : Esc [ 0 m
  ```
  Assert "IB" has ITALIC+BOLD. Assert "B" has BOLD but not ITALIC.

- [x] **`reset_24_cancel_underline.teseq`** — SGR 24 cancels ALL underline styles:
  ```
  : Esc [ 4 : 3 m
  |Curly|
  : Esc [ 24 m
  |None|
  : Esc [ 0 m
  ```
  Assert "Curly" has CURLY_UNDERLINE. Assert "None" has none of ALL_UNDERLINES.
  This tests that SGR 24 clears the `ALL_UNDERLINES` mask (all 5 styles), not just `UNDERLINE`.

- [x] **`reset_25_cancel_blink.teseq`** — SGR 25 cancels blink:
  ```
  : Esc [ 5 ; 1 m
  |BlinkBold|
  : Esc [ 25 m
  |Bold|
  : Esc [ 0 m
  ```
  Assert "BlinkBold" has BLINK+BOLD. Assert "Bold" has BOLD but not BLINK.

- [x] **`reset_27_cancel_inverse.teseq`** — SGR 27 cancels inverse:
  ```
  : Esc [ 7 ; 3 m
  |InvItalic|
  : Esc [ 27 m
  |Italic|
  : Esc [ 0 m
  ```
  Assert "InvItalic" has INVERSE+ITALIC. Assert "Italic" has ITALIC but not INVERSE.

- [x] **`reset_28_cancel_hidden.teseq`** — SGR 28 cancels hidden:
  ```
  : Esc [ 8 ; 1 m
  |HidBold|
  : Esc [ 28 m
  |Bold|
  : Esc [ 0 m
  ```
  Assert "HidBold" has HIDDEN+BOLD. Assert "Bold" has BOLD but not HIDDEN.

- [x] **`reset_29_cancel_strike.teseq`** — SGR 29 cancels strikethrough:
  ```
  : Esc [ 9 ; 3 m
  |StrikeItalic|
  : Esc [ 29 m
  |Italic|
  : Esc [ 0 m
  ```
  Assert "StrikeItalic" has STRIKETHROUGH+ITALIC. Assert "Italic" has ITALIC but not STRIKETHROUGH.

- [x] **`reset_selective_preserves_others.teseq`** — comprehensive test: apply bold+italic+underline+blink+inverse+strikethrough, then cancel them one at a time, verifying each cancel removes only its target:
  ```
  : Esc [ 1 ; 3 ; 4 ; 5 ; 7 ; 9 m
  |All|
  : Esc [ 22 m
  |NoBold|
  : Esc [ 23 m
  |NoItalic|
  : Esc [ 24 m
  |NoUL|
  : Esc [ 25 m
  |NoBlink|
  : Esc [ 27 m
  |NoInv|
  : Esc [ 29 m
  |NoStrike|
  : Esc [ 0 m
  ```
  Assert progressive attribute removal — each step only removes its target flag while others remain. Concrete assertions:
  - "All" (line 0, col 0): BOLD+ITALIC+UNDERLINE+BLINK+INVERSE+STRIKETHROUGH all set
  - "NoBold" (line 1, col 0): ITALIC+UNDERLINE+BLINK+INVERSE+STRIKETHROUGH set, BOLD cleared
  - "NoItalic" (line 2, col 0): UNDERLINE+BLINK+INVERSE+STRIKETHROUGH set, BOLD+ITALIC cleared
  - "NoUL" (line 3, col 0): BLINK+INVERSE+STRIKETHROUGH set, ALL_UNDERLINES cleared
  - "NoBlink" (line 4, col 0): INVERSE+STRIKETHROUGH set, BLINK cleared
  - "NoInv" (line 5, col 0): STRIKETHROUGH set, INVERSE cleared
  - "NoStrike" (line 6, col 0): no SGR flags

- [ ] **TPR checkpoint** — `/tpr-review` covering 05.5a implementation work

---

## 05.5b Default Color & Template Resets (SGR 0/39/49/59)

**File(s):** `oriterm_core/tests/teseq/scenarios/csi/sgr/reset_*.teseq`

This subsection covers the full-reset (SGR 0) and color-specific resets (SGR 39/49/59) that operate on colors rather than attribute flags.

- [x] **`reset_sgr0.teseq`** — SGR 0 clears all attributes and colors:
  ```
  : Esc [ 1 ; 3 ; 4 ; 31 ; 42 m
  |All on|
  : Esc [ 0 m
  |Clean|
  ```
  Assert "All on" (line 0, col 0) has BOLD+ITALIC+UNDERLINE flags, fg=red (`Palette::default().resolve(Color::Named(Red))`), bg=green (`Palette::default().resolve(Color::Named(Green))`). Assert "Clean" (line 1, col 0) has no SGR flags (BOLD, DIM, ITALIC, UNDERLINE, BLINK, INVERSE, HIDDEN, STRIKETHROUGH all cleared), default fg (palette foreground), default bg (palette background).

- [x] **`reset_39_default_fg.teseq`** — SGR 39 resets fg to default, preserving other attrs:
  ```
  : Esc [ 1 ; 31 m
  |Red bold|
  : Esc [ 39 m
  |Default fg bold|
  : Esc [ 0 m
  ```
  Assert: "Red bold" (line 0, col 0) has BOLD flag + red fg. "Default fg bold" (line 1, col 0) has BOLD flag + default fg (palette foreground color via `Palette::default().resolve(Color::Named(Foreground))`), bg unchanged (still default).

- [x] **`reset_49_default_bg.teseq`** — SGR 49 resets bg to default, preserving other attrs:
  ```
  : Esc [ 42 ; 3 m
  |Green bg italic|
  : Esc [ 49 m
  |Default bg italic|
  : Esc [ 0 m
  ```
  Assert: "Green bg italic" (line 0, col 0) has ITALIC flag + green bg. "Default bg italic" (line 1, col 0) has ITALIC flag preserved, bg resets to default palette background.

- [x] **`reset_59_underline_color.teseq`** — SGR 59 resets underline color, preserving underline style:
  ```
  : Esc [ 4 m
  : Esc [ 58 ; 2 ; 200 ; 100 ; 50 m
  |Colored|
  : Esc [ 59 m
  |Default|
  : Esc [ 0 m
  ```
  Assert: "Colored" (line 0, col 0) has UNDERLINE flag + `cell_underline_color_at` returns `Some(Rgb { r: 200, g: 100, b: 50 })`. "Default" (line 1, col 0) has UNDERLINE flag preserved + `cell_underline_color_at` returns `None`.

- [ ] **TPR checkpoint** — `/tpr-review` covering 05.5b implementation work

---

## 05.6 Color Resolution Edge Cases

**File(s):** `oriterm_core/tests/teseq/scenarios/csi/sgr/edge_*.teseq`

These scenarios test the color resolution pipeline in `term/renderable/mod.rs` — the layer between raw `Color` enum values and final `Rgb` output.

- [x] **`edge_dim_bold_named.teseq`** — DIM + BOLD on a named color: DIM takes priority:
  ```
  : Esc [ 1 ; 2 ; 31 m
  |DimBoldRed|
  : Esc [ 0 m
  ```
  Assert: fg resolves to dimmed red, NOT bright red. Compute expected via `Palette::default().resolve(Color::Named(vte::ansi::NamedColor::DimRed))` (add `use vte::ansi::NamedColor;` to `sgr.rs` imports). This validates the DIM-wins-over-bold-as-bright rule in `resolve_fg()` — the `is_dim` branch takes priority over the `bold_is_bright && is_bold` branch for Named colors. Matches existing renderable test `bold_plus_dim_named_color`.

- [x] **`edge_dim_bold_indexed.teseq`** — DIM + BOLD on indexed 0-7: no bright promotion:
  ```
  : Esc [ 1 ; 2 m
  : Esc [ 38 ; 5 ; 2 m
  |DimBoldGreen|
  : Esc [ 0 m
  ```
  Assert: fg resolves to dimmed green (2/3 of each channel of palette index 2), NOT bright green (palette index 10). `dim_rgb` is `pub(crate)` so the test computes the expected value inline: `let base = palette.resolve(Color::Indexed(2)); let expected = Rgb { r: (base.r as u16 * 2 / 3) as u8, g: (base.g as u16 * 2 / 3) as u8, b: (base.b as u16 * 2 / 3) as u8 };`.

- [x] **`edge_dim_bold_truecolor.teseq`** — DIM + BOLD on TrueColor: only dim applies (bold doesn't affect TrueColor):
  ```
  : Esc [ 1 ; 2 m
  : Esc [ 38 ; 2 ; 150 ; 120 ; 90 m
  |DimBoldTC|
  : Esc [ 0 m
  ```
  Assert: fg resolves to `Rgb { r: 100, g: 80, b: 60 }` (2/3 of each channel).

- [x] **`edge_inverse_colors.teseq`** — Inverse with explicit fg/bg swaps them:
  ```
  : Esc [ 31 ; 42 m
  |Red on green|
  : Esc [ 7 m
  |Inverted|
  : Esc [ 0 m
  ```
  Assert: "Red on green" has fg=red, bg=green. "Inverted" has fg=green, bg=red. This validates `apply_inverse()` in `renderable/mod.rs` — when `INVERSE` flag is set, fg and bg are swapped. Matches existing renderable test `apply_inverse_swaps_defaults`.

- [x] **`edge_decscnm_basic.teseq`** — DECSCNM (mode 5) swaps default fg/bg:
  ```toml
  # edge_decscnm_basic.toml
  [setup]
  pre_feed = ["\\x1b[?5h"]
  ```
  ```
  |Normal text|
  ```
  Assert: default cells have fg=original_bg and bg=original_fg. DECSCNM is implemented by cloning and swapping the palette before color resolution (`snapshot.rs:92-106`).

- [x] **`edge_decscnm_inverse.teseq`** — DECSCNM + SGR 7 = double swap = normal appearance:
  ```toml
  # edge_decscnm_inverse.toml
  [setup]
  pre_feed = ["\\x1b[?5h"]
  ```
  ```
  : Esc [ 7 m
  |Double swapped|
  : Esc [ 0 m
  ```
  Assert: "Double swapped" has fg=original_fg, bg=original_bg (DECSCNM swaps palette, SGR 7 swaps again → back to normal). This matches the existing renderable test `decscnm_plus_inverse_is_double_swap`.

- [x] **`edge_decscnm_explicit_color.teseq`** — DECSCNM does NOT affect explicitly set colors:
  ```toml
  # edge_decscnm_explicit_color.toml
  [setup]
  pre_feed = ["\\x1b[?5h"]
  ```
  ```
  : Esc [ 38 ; 2 ; 100 ; 200 ; 50 m
  |Explicit|
  : Esc [ 0 m
  ```
  Assert: fg is `Rgb { r: 100, g: 200, b: 50 }` — DECSCNM only swaps the default fg/bg palette entries, not explicit TrueColor values. This is because `resolve_fg` on `Color::Spec(rgb)` returns `rgb` directly (only dim reduces it) — the palette swap affects `Color::Named(Foreground/Background)` resolution but not `Color::Spec`.

- [ ] **TPR checkpoint** — `/tpr-review` covering 05.6 implementation work

---

## 05.7 Attribute Stacking & Combination Scenarios

**File(s):** `oriterm_core/tests/teseq/scenarios/csi/sgr/combo_*.teseq`

- [x] **`combo_stack.teseq`** — Multiple attributes stacked in one CSI:
  ```
  : Esc [ 1 ; 3 ; 4 ; 31 m
  |Bold italic underline red|
  : Esc [ 0 m
  |Normal|
  ```
  Assert all four attributes active on first text (BOLD+ITALIC+UNDERLINE flags, red fg), none on second.

- [x] **`combo_separate_sequences.teseq`** — Attributes accumulate across separate SGR sequences:
  ```
  : Esc [ 1 m
  : Esc [ 3 m
  : Esc [ 4 m
  |Stacked|
  : Esc [ 0 m
  ```
  Assert BOLD+ITALIC+UNDERLINE all set. This validates that SGR attributes are additive (each SGR modifies the cursor template, not replaces it).

- [x] **`combo_color_last_wins.teseq`** — Multiple fg colors in one sequence: last wins:
  ```
  : Esc [ 31 ; 32 ; 33 m
  |Yellow wins|
  : Esc [ 0 m
  ```
  Assert fg is yellow (SGR 33), not red (SGR 31) or green (SGR 32).

- [x] **`combo_dim_then_bold.teseq`** — Setting dim then bold: both flags set, DIM takes priority for color:
  ```
  : Esc [ 2 ; 1 ; 31 m
  |DimBold|
  : Esc [ 0 m
  ```
  Assert: BOLD and DIM flags both set. fg resolves to dim red (not bright red).

- [x] **`combo_empty_sgr_resets.teseq`** — `CSI m` (no parameters) is equivalent to SGR 0:
  ```
  : Esc [ 1 ; 3 ; 31 m
  |Styled|
  : Esc [ m
  |Plain|
  ```
  Assert: "Styled" has BOLD+ITALIC+red fg. "Plain" has no SGR flags and default fg/bg. This validates that parameterless SGR dispatches as `Attr::Reset` (VTE csi.rs:284 — `[0]` matches, and the missing param defaults to 0 per ECMA-48). Matches existing handler test `sgr_empty_params_resets`.

- [x] **`combo_sgr_persists_cursor_move.teseq`** — SGR attributes survive cursor movement:
  ```
  : Esc [ 1 ; 31 m
  |A|
  : Esc [ 5 G
  |B|
  : Esc [ 0 m
  ```
  Assert: Both 'A' (col 0) and 'B' (col 4) have BOLD flag and red fg.

---

## 05.R Third Party Review Findings

- [x] `[TPR-05-001][medium]` [plans/teseq-conformance/section-05-sgr-colors.md:11](/home/eric/projects/ori_term/plans/teseq-conformance/section-05-sgr-colors.md#L11), [plans/teseq-conformance/section-05-sgr-colors.md:78](/home/eric/projects/ori_term/plans/teseq-conformance/section-05-sgr-colors.md#L78), [plans/teseq-conformance/section-05-sgr-colors.md:442](/home/eric/projects/ori_term/plans/teseq-conformance/section-05-sgr-colors.md#L442), [oriterm_core/tests/teseq/scenarios/csi/sgr/color_16_fg.teseq:1](/home/eric/projects/ori_term/oriterm_core/tests/teseq/scenarios/csi/sgr/color_16_fg.teseq#L1), [oriterm_core/tests/teseq/scenarios/csi/sgr/color_16_bg.teseq:1](/home/eric/projects/ori_term/oriterm_core/tests/teseq/scenarios/csi/sgr/color_16_bg.teseq#L1), [oriterm_core/tests/teseq/sgr.rs:257](/home/eric/projects/ori_term/oriterm_core/tests/teseq/sgr.rs#L257) — Section 05 marks the full 16-color matrix complete, but the committed scenarios only exercise codes `30-37`, `90-91`, `40-47`, and `100-101`, and the Rust assertions sample only black, red, green, bright black, and bright red.
  Validation: the plan explicitly claims "all 8 base + 8 bright colors" for both foreground and background, yet the fixtures stop after `91`/`101`, so bright green through bright white (`92-97` and `102-107`) never appear in the teseq corpus. Because the grid snapshots do not encode color, those missing bright colors have no other coverage in this section. This leaves six bright foreground mappings and six bright background mappings unpinned while the checklist and success criteria report the 16-color work as done.

- [x] `[TPR-05-002][low]` [plans/teseq-conformance/section-05-sgr-colors.md:4](/home/eric/projects/ori_term/plans/teseq-conformance/section-05-sgr-colors.md#L4), [plans/teseq-conformance/section-05-sgr-colors.md:70](/home/eric/projects/ori_term/plans/teseq-conformance/section-05-sgr-colors.md#L70), [plans/teseq-conformance/index.md:91](/home/eric/projects/ori_term/plans/teseq-conformance/index.md#L91), [plans/teseq-conformance/00-overview.md:37](/home/eric/projects/ori_term/plans/teseq-conformance/00-overview.md#L37), [plans/teseq-conformance/00-overview.md:189](/home/eric/projects/ori_term/plans/teseq-conformance/00-overview.md#L189) — The coordinating plan files are out of sync with the committed Section 05 work.
  Validation: `section-05-sgr-colors.md` frontmatter is `in-progress` and its subsections `05.0` through `05.7` are all marked complete, but the prose header in the same file still says `Status: Not Started`. The higher-level plan files also still report Section 05 as `Not Started` and leave the SGR mission criterion unchecked. That stale status is enough to mislead `/continue-plan` style workflows and to hide the actual review/fix backlog behind a false "not started" state.

- [x] `[TPR-05-003][medium]` [plans/teseq-conformance/section-05-sgr-colors.md:11](/home/eric/projects/ori_term/plans/teseq-conformance/section-05-sgr-colors.md#L11), [plans/teseq-conformance/section-05-sgr-colors.md:78](/home/eric/projects/ori_term/plans/teseq-conformance/section-05-sgr-colors.md#L78), [plans/teseq-conformance/section-05-sgr-colors.md:442](/home/eric/projects/ori_term/plans/teseq-conformance/section-05-sgr-colors.md#L442), [plans/teseq-conformance/section-05-sgr-colors.md:468](/home/eric/projects/ori_term/plans/teseq-conformance/section-05-sgr-colors.md#L468), [oriterm_core/tests/teseq/scenarios/csi/sgr/color_16_fg.teseq:1](/home/eric/projects/ori_term/oriterm_core/tests/teseq/scenarios/csi/sgr/color_16_fg.teseq#L1), [oriterm_core/tests/teseq/scenarios/csi/sgr/color_16_bg.teseq:1](/home/eric/projects/ori_term/oriterm_core/tests/teseq/scenarios/csi/sgr/color_16_bg.teseq#L1), [oriterm_core/tests/teseq/sgr.rs:257](/home/eric/projects/ori_term/oriterm_core/tests/teseq/sgr.rs#L257), [oriterm_core/tests/teseq/sgr.rs:315](/home/eric/projects/ori_term/oriterm_core/tests/teseq/sgr.rs#L315) — Section 05 still claims full 16-color foreground/background coverage, but the teseq corpus only emits bright colors `90-91` and `100-101`, and the background assertions only sample five of the ten emitted color groups.
  Validation: the current fixtures stop at bright red, so bright green through bright white (`92-97`, `102-107`) never appear in the Section 05 teseq scenarios at all. On top of that, `color_16_bg()` asserts black, red, cyan, white, and bright black, but leaves green, yellow, blue, magenta, and bright red unpinned even within the existing fixture. The targeted test run still passes because the missing mappings are never exercised, so the plan’s “all 8 base + 8 bright colors” and “foreground/background tested with correct Rgb resolution” claims remain overstated.

- [x] `[TPR-05-004][low]` [CLAUDE.md:33](/home/eric/projects/ori_term/CLAUDE.md#L33), [plans/teseq-conformance/section-05-sgr-colors.md:105](/home/eric/projects/ori_term/plans/teseq-conformance/section-05-sgr-colors.md#L105), [oriterm_core/tests/teseq/sgr.rs:1](/home/eric/projects/ori_term/oriterm_core/tests/teseq/sgr.rs#L1) — The Section 05 implementation violates the repo’s file-size rule and the plan text incorrectly treats `sgr.rs` as exempt.
  Validation: `oriterm_core/tests/teseq/sgr.rs` is 917 lines in the current tree, while `CLAUDE.md` sets a hard 500-line limit for source files except files literally named `tests.rs`. The plan note says “Test files are exempt from the 500-line limit per `code-hygiene.md`,” but that exemption is narrower than the file actually used here. This is a standards violation rather than a runtime bug, but the review gate is supposed to catch exactly this kind of rule drift before the section is marked complete.

---

## 05.N Completion Checklist

- [ ] Scaffolding complete: `scenarios/csi/sgr/` directory, `sgr.rs` module, `main.rs` registration (05.0a)
- [ ] Cell attribute inspection helpers added to harness: `assert_cell_flags_contain`, `assert_cell_flags_not_contain`, `cell_fg_at`, `cell_bg_at`, `cell_underline_color_at` (05.0b)
- [ ] `set_bold_is_bright()` method added to `TeseqHarness` (05.0c)
- [ ] Re-exports in `harness/mod.rs` updated for all new helpers (05.0b)
- [ ] Text attribute scenarios: bold, dim, italic, underline, blink, blink_fast, inverse, hidden, strikethrough (9 scenarios, 05.1)
- [ ] Underline style scenarios: single, double, curly, dotted, dashed, mutual exclusion, cancel-subparam (7 scenarios, 05.2)
- [ ] Underline color scenarios: truecolor, 256-color, reset, style-change survival (4 scenarios, 05.2)
- [ ] 16-color scenarios: fg colors, bg colors, bold-as-bright, bold-no-promote, bold-bright-disabled (5 scenarios, 05.3)
- [ ] 256-color scenarios: fg and bg indexed colors (2 scenarios, 05.4)
- [ ] TrueColor scenarios: fg and bg RGB colors (2 scenarios, 05.4)
- [ ] Selective attribute resets: SGR 21/22/23/24/25/27/28/29, progressive removal (9 scenarios, 05.5a)
- [ ] Default color/template resets: SGR 0/39/49/59 (4 scenarios, 05.5b)
- [ ] Color resolution edge cases: dim+bold (3 variants), inverse, DECSCNM (3 variants) (7 scenarios, 05.6)
- [ ] Combination scenarios: stacking, separate sequences, last-wins, empty-sgr-resets, dim+bold, cursor-move persistence (6 scenarios, 05.7)
- [ ] 40+ total SGR scenarios pass (target: ~55)
- [ ] `./build-all.sh` green, `./clippy-all.sh` green
- [ ] `timeout 150 ./test-all.sh` green — no regressions
- [ ] Plan annotation cleanup
- [ ] All TPR checkpoint findings resolved
- [ ] **Plan sync** — update plan metadata:
  - [ ] This section's frontmatter `status` -> `complete`
  - [ ] `00-overview.md` Quick Reference table updated
  - [ ] `index.md` section status updated
- [ ] `/tpr-review` passed (final, full-section)
- [ ] `/impl-hygiene-review last commit` passed

**Exit Criteria:** `timeout 150 cargo test -p oriterm_core --test teseq -- sgr` passes with 40+ SGR scenarios (target: ~55). Cell attribute inspection validates CellFlags (via `contains()` pattern), resolved Rgb colors, and underline colors. Bold-as-bright promotion, bold-as-bright disabled, DIM+bold priority, 256-color indexed, TrueColor RGB, all 5 underline styles, underline colors, all selective resets (SGR 21-29/39/49/59), inverse color swap, DECSCNM cross-cutting — all validated against the color resolution pipeline. Zero regressions.
