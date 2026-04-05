---
section: "05"
title: "SGR & Color Scenarios"
status: not-started
reviewed: false
goal: "Create comprehensive SGR scenarios covering text attributes and all color modes (16, 256, TrueColor) with rendered cell attribute validation"
success_criteria:
  - "Basic attribute scenarios: bold, dim, italic, underline, blink, inverse, hidden, strikethrough"
  - "16-color scenarios: foreground and background for all 8 base + 8 bright colors"
  - "256-color scenarios: indexed colors via SGR 38;5;N and 48;5;N"
  - "TrueColor scenarios: RGB colors via SGR 38;2;R;G;B and 48;2;R;G;B"
  - "Bold-as-bright promotion tested: bold + ANSI color 0-7 → bright color 8-15"
  - "Attribute reset scenarios: SGR 0 clears all attributes"
  - "Satisfies mission criteria: 16/256/TrueColor, bold-as-bright, attribute coverage"
inspired_by:
  - "ori_term handler/sgr.rs — SGR dispatch and color parsing"
  - "ori_term handler/tests.rs — SGR attribute tests"
  - "WezTerm term/src/test/csi.rs — color palette and attribute tests"
depends_on: ["01", "02"]
third_party_review:
  status: none
  updated: null
sections:
  - id: "05.1"
    title: "Text Attribute Scenarios"
    status: not-started
  - id: "05.2"
    title: "16-Color & Bold-as-Bright Scenarios"
    status: not-started
  - id: "05.3"
    title: "256-Color & TrueColor Scenarios"
    status: not-started
  - id: "05.4"
    title: "Attribute Reset & Combination Scenarios"
    status: not-started
  - id: "05.R"
    title: "Third Party Review Findings"
    status: not-started
  - id: "05.N"
    title: "Completion Checklist"
    status: not-started
---

# Section 05: SGR & Color Scenarios

**Status:** Not Started
**Goal:** Comprehensive SGR coverage through teseq scenarios. While handler tests validate SGR parsing at the byte level, these scenarios validate the *rendered result* — CellFlags and resolved colors as they appear in `RenderableContent`. This catches bugs in the color resolution pipeline (bold-as-bright promotion, dim application, inverse swapping) that byte-level tests miss.

**Success Criteria:**

- [ ] All 8 basic text attributes tested via cell flag inspection
- [ ] 16-color foreground/background tested with correct Rgb resolution
- [ ] 256-color indexed colors tested
- [ ] TrueColor RGB colors tested
- [ ] Bold-as-bright color promotion validated
- [ ] Attribute stacking and reset validated
- [ ] 15+ SGR scenarios pass

**Context:** SGR scenarios differ from other scenario types because the *grid text* doesn't change based on attributes — "hello" looks the same whether it's bold or not in `grid_text()`. Instead, these scenarios need to inspect `RenderableCell` attributes (flags, fg, bg colors). The harness needs to support attribute inspection beyond plain text comparison.

**Reference implementations:**
- **ori_term** `handler/sgr.rs:1-100`: SGR dispatch table mapping parameter values to CellFlags and colors
- **ori_term** `term/renderable/mod.rs:25-49`: `RenderableCell` with resolved `fg: Rgb`, `bg: Rgb`, `flags: CellFlags`
- **ori_term** `term/renderable/tests.rs`: Color resolution tests including bold-as-bright, dim, inverse

**Depends on:** Section 01 (TeseqHarness), Section 02 (basic pattern).

---

## 05.1 Text Attribute Scenarios

**File(s):** `oriterm_core/tests/teseq/scenarios/csi/sgr/attr_*.teseq`, `oriterm_core/tests/teseq/sgr.rs`

Each attribute scenario sets an SGR attribute, writes text, then verifies the cell flags.

- [ ] Add cell attribute inspection helpers to `assertions.rs` (or as methods on `ScenarioOutcome`):
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

  /// Get the CellFlags for a specific cell position.
  pub fn cell_flags_at(outcome: &ScenarioOutcome, line: usize, col: usize) -> CellFlags {
      find_cell(outcome, line, col).flags
  }

  /// Get the foreground Rgb for a specific cell.
  pub fn cell_fg_at(outcome: &ScenarioOutcome, line: usize, col: usize) -> Rgb {
      find_cell(outcome, line, col).fg
  }

  /// Get the background Rgb for a specific cell.
  pub fn cell_bg_at(outcome: &ScenarioOutcome, line: usize, col: usize) -> Rgb {
      find_cell(outcome, line, col).bg
  }
  ```
  Note: These are free functions in `assertions.rs`, not methods on `ScenarioOutcome`, to keep `runner.rs` clean.

- [ ] Create attribute scenarios:

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

- [ ] Each scenario asserts the correct `CellFlags` are set on the text cells and cleared on subsequent text after SGR 0 reset.

---

## 05.2 16-Color & Bold-as-Bright Scenarios

**File(s):** `oriterm_core/tests/teseq/scenarios/csi/sgr/color_*.teseq`

- [ ] **`color_16_fg.teseq`** — All 8 foreground colors + 8 bright:
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
  Assert each cell group has the correct resolved Rgb from the palette.

- [ ] **`color_16_bg.teseq`** — Background colors (SGR 40-47, 100-107).

- [ ] **`color_bold_bright.teseq`** — Bold + ANSI color 0-7 triggers bright promotion:
  ```
  : Esc [ 1 ; 31 m
  |Bold red|
  : Esc [ 0 m
  ```
  Assert fg resolves to bright red (palette index 9) not normal red (palette index 1).
  ori_term has `bold_is_bright: true` by default (`term/mod.rs:233`).

- [ ] **TPR checkpoint** — `/tpr-review` covering 05.1–05.2 implementation work

---

## 05.3 256-Color & TrueColor Scenarios

**File(s):** `oriterm_core/tests/teseq/scenarios/csi/sgr/color_256_*.teseq`, `color_rgb_*.teseq`

- [ ] **`color_256_fg.teseq`** — 256-color foreground:
  ```
  : Esc [ 38 ; 5 ; 196 m
  |Red 256|
  : Esc [ 38 ; 5 ; 46 m
  |Green 256|
  : Esc [ 38 ; 5 ; 21 m
  |Blue 256|
  : Esc [ 0 m
  ```
  Assert each cell has correct indexed color resolved to Rgb.

- [ ] **`color_256_bg.teseq`** — 256-color background (SGR 48;5;N).

- [ ] **`color_rgb_fg.teseq`** — TrueColor foreground:
  ```
  : Esc [ 38 ; 2 ; 255 ; 128 ; 0 m
  |Orange|
  : Esc [ 38 ; 2 ; 0 ; 255 ; 128 m
  |Spring|
  : Esc [ 0 m
  ```
  Assert each cell's fg Rgb matches the specified values exactly.

- [ ] **`color_rgb_bg.teseq`** — TrueColor background (SGR 48;2;R;G;B).

---

## 05.4 Attribute Reset & Combination Scenarios

**File(s):** `oriterm_core/tests/teseq/scenarios/csi/sgr/combo_*.teseq`

- [ ] **`combo_stack.teseq`** — Multiple attributes stacked:
  ```
  : Esc [ 1 ; 3 ; 4 ; 31 m
  |Bold italic underline red|
  : Esc [ 0 m
  |Normal|
  ```
  Assert all four attributes active on first text, none on second.

- [ ] **`combo_selective_reset.teseq`** — Reset individual attributes:
  ```
  : Esc [ 1 ; 3 ; 4 m
  |All on|
  : Esc [ 22 m
  |No bold|
  : Esc [ 23 m
  |No italic|
  : Esc [ 24 m
  |No underline|
  ```
  Assert progressive attribute removal.

- [ ] **`combo_inverse_color.teseq`** — Inverse with explicit colors:
  ```
  : Esc [ 31 ; 42 m
  |Red on green|
  : Esc [ 7 m
  |Inverted|
  : Esc [ 0 m
  ```
  Assert inverted text has fg=green, bg=red (swapped by renderable color resolution).

---

## 05.R Third Party Review Findings

- None.

---

## 05.N Completion Checklist

- [ ] Cell attribute inspection helpers added to harness (cell_flags_at, cell_fg_at, cell_bg_at)
- [ ] Text attribute scenarios: bold, dim, italic, underline, blink, inverse, hidden, strikethrough (8 scenarios)
- [ ] 16-color scenarios: fg colors, bg colors, bold-as-bright promotion (3 scenarios)
- [ ] 256-color scenarios: fg and bg indexed colors (2 scenarios)
- [ ] TrueColor scenarios: fg and bg RGB colors (2 scenarios)
- [ ] Combination scenarios: stacking, selective reset, inverse+color (3 scenarios)
- [ ] 15+ total SGR scenarios pass
- [ ] `./build-all.sh` green, `./clippy-all.sh` green
- [ ] `timeout 150 ./test-all.sh` green — no regressions
- [ ] Plan annotation cleanup
- [ ] All TPR checkpoint findings resolved
- [ ] **Plan sync** — update plan metadata:
  - [ ] This section's frontmatter `status` → `complete`
  - [ ] `00-overview.md` Quick Reference table updated
  - [ ] `index.md` section status updated
- [ ] `/tpr-review` passed (final, full-section)
- [ ] `/impl-hygiene-review last commit` passed

**Exit Criteria:** `timeout 150 cargo test -p oriterm_core --test teseq -- sgr` passes with 15+ SGR scenarios. Cell attribute inspection validates CellFlags and resolved Rgb colors. Bold-as-bright promotion, 256-color indexed, and TrueColor RGB all validated against palette resolution. Zero regressions.
