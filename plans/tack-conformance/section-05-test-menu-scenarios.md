---
section: "05"
title: "Tack Scenarios: Test Menu (begin testing)"
status: not-started
reviewed: false
goal: "Populate the scenario catalog with every navigable screen under tack's `n) begin testing` submenu: modes/glitches, ACS/graphic rendition, color, and cursor movement. Const `ScenarioSpec` values and per-scenario parsers live in `oriterm_test_support::tack_framework::scenarios::*` so both text tests in `oriterm_core/tests/tack/` and GPU goldens in `oriterm/src/gpu/visual_regression/tack/` (Section 07) reference the same const. Test wrapper functions live in `oriterm_core/tests/tack/test_menu/*.rs`. Each scenario has an insta snapshot and a programmatic semantic assertion. Color/cursor scenarios run at three sizes (80x24, 97x33, 120x40)."
success_criteria:
  - "`crates/oriterm_test_support/src/tack_framework/scenarios/` contains const ScenarioSpec values for every test menu screen: modes, acs, graphic_rendition, color, cursor_movement (one submodule per screen family)"
  - "Per-scenario parser fns (function pointers, since `ScenarioSpec` is `const`) live next to their consts in `tack_framework::scenarios::{family}::parse_*`"
  - "`oriterm_core/tests/tack/test_menu/` contains test wrapper modules that import const scenarios from `oriterm_test_support::tack_framework::scenarios::*` and define `#[test] fn` wrappers calling `ScenarioRunner::run(&...)` / `run_at(&..., cols, rows)`"
  - "At least 12 scenarios total exist as `pub const` ScenarioSpec values across the test_menu modules"
  - "Each scenario has its own custom parser when typed assertions are needed (e.g., `parse_color_screen` extracts the named color rows; `parse_cursor_screen` extracts the cursor position from tack's status line)"
  - "Each scenario has at least one programmatic semantic assertion BEYOND the insta snapshot — naming what fact the test guards (e.g., `assert!(facts.named_colors.contains(\"red\"))`)"
  - "`tack_modes_*`, `tack_color_*`, `tack_cursor_*` scenarios are duplicated across the (80x24, 97x33, 120x40) size matrix using `ScenarioRunner::run_at(spec, cols, rows)`"
  - "All scenarios run deterministically (10 consecutive passes per scenario) — no flake threshold tolerance"
  - "All scenarios skip cleanly when `tack`/`tic` are unavailable via `ScenarioRunner::available()`"
  - "`timeout 150 cargo test -p oriterm_core --test tack -- test_menu` passes (entire test_menu submodule)"
  - "Satisfies mission criterion: 'Tack test scenarios cover: modes/glitches, ACS/graphic rendition, color, cursor movement'"
inspired_by:
  - "ori_term Section 04 framework (plans/tack-conformance/section-04-scenario-framework.md — ScenarioSpec/TackNavigator/ScenarioRunner)"
  - "ori_term vttest menu1 size matrix (oriterm_core/tests/vttest/menu1.rs — same 80x24/97x33/120x40 pattern)"
  - "ncurses tack source (begin testing menu items: modes, glitches, ACS, color, cursor movement)"
depends_on: ["04"]
third_party_review:
  status: none
  updated: null
sections:
  - id: "05.1"
    title: "Modes/glitches scenarios (expand from Section 04)"
    status: not-started
  - id: "05.2"
    title: "ACS / graphic rendition scenarios"
    status: not-started
  - id: "05.3"
    title: "Color scenarios (size matrix)"
    status: not-started
  - id: "05.4"
    title: "Cursor movement scenarios (size matrix)"
    status: not-started
  - id: "05.5"
    title: "Determinism + size matrix verification"
    status: not-started
  - id: "05.R"
    title: "Third Party Review Findings"
    status: not-started
  - id: "05.N"
    title: "Completion Checklist"
    status: not-started
---

# Section 05: Tack Scenarios — Test Menu (begin testing)

**Status:** Not Started
**Goal:** Build out the catalog of scenarios accessible from tack's `n) begin testing` submenu. Const `ScenarioSpec` values + per-scenario parsers live in `crates/oriterm_test_support/src/tack_framework/scenarios/` (one submodule per screen family). Test wrapper `#[test] fn` files live in `oriterm_core/tests/tack/test_menu/` and import the consts. Section 07's GPU goldens reference the SAME consts — single source of truth for "how do you reach the modes screen" and "what does the parser extract".

The catalog covers modes/glitches (am, bce, bw, km, mir, msgr, xenl), ACS/graphic rendition (line-drawing chars, SGR styles), color (named colors, 256-color block), and cursor movement (cup, csr, hpa, vpa, scroll regions). Color and cursor scenarios run at three sizes — 80x24, 97x33, 120x40 — using `ScenarioRunner::run_at` to catch size-dependent regressions.

**Success Criteria:**

- [ ] `oriterm_core/tests/tack/test_menu/` contains: `modes.rs` (expanded from Section 04), `acs.rs`, `graphic_rendition.rs`, `color.rs`, `cursor_movement.rs`
- [ ] At least 12 scenarios across these modules — exact list in subsection checklists below
- [ ] Each scenario passes `cargo test` deterministically (10 reruns clean)
- [ ] All scenarios skip cleanly when `tack`/`tic` unavailable
- [ ] Color/cursor scenarios run at three sizes via `run_at`
- [ ] `timeout 150 cargo test -p oriterm_core --test tack -- test_menu` green
- [ ] Satisfies mission criterion: test menu coverage

**Context:** Section 04 builds the framework and proves it with ONE scenario (`tack_modes_am`). Section 05 fills in the rest of the test menu catalog: every navigable screen under `n) begin testing` becomes a scenario. Each scenario captures a snapshot AND extracts typed facts via a per-screen parser. The typed facts are what gives this catalog teeth — without them, "the snapshot didn't change" is a weak assertion (the snapshot can be updated thoughtlessly during a refactor and silently lose meaning). With typed facts, every test names what it guards: "this test fails if `am` is missing from the modes screen", "this test fails if `red` isn't in the color screen named-color list", etc.

The size matrix (80x24, 97x33, 120x40) for color/cursor scenarios mirrors the existing vttest convention from `oriterm_core/tests/vttest/menu1.rs`. It catches regressions where a feature works at the default 80x24 but breaks at a non-standard size — historical bugs in the cell loop, scroll regions, and DECCOLM resizing all surfaced this way.

**Reference implementations:**
- **Section 04** `plans/tack-conformance/section-04-scenario-framework.md`: framework consumed here.
- **ori_term vttest menu1** `oriterm_core/tests/vttest/menu1.rs:vttest_menu1_80x24/97x33/120x40`: existing size-matrix pattern this section adopts for color/cursor scenarios.
- **ori_term vttest menu3** `oriterm_core/tests/vttest/menu3.rs:assert_has_line_drawing_chars`: existing parser pattern (extract typed facts from grid_chars). Section 05's per-scenario parsers follow the same idea.
- **ncurses tack source** (man page documents the begin-testing submenu items): the canonical list of screens.

**Depends on:** Section 04 (framework).

---

## 05.1 Modes/glitches scenarios (expand from Section 04)

**File(s):** `crates/oriterm_test_support/src/tack_framework/scenarios/modes.rs` (NEW — const scenarios + parser), `oriterm_core/tests/tack/test_menu/modes.rs` (expand from Section 04's stub — test wrapper functions only)

**Layout reminder:** the const ScenarioSpec values and the `parse_modes_screen` function pointer go in `oriterm_test_support::tack_framework::scenarios::modes` (workspace-internal crate). The `#[test] fn` wrappers go in `oriterm_core/tests/tack/test_menu/modes.rs` (integration test target). The two files share nothing except the import line `use oriterm_test_support::tack_framework::scenarios::modes::*;` in the test wrapper.

Section 04 added `TACK_MODES_AM`. This subsection adds the rest of the modes scenarios — one per ori_term-supported boolean cap that tack tests on the modes screen.

- [ ] Add `TACK_MODES_BCE`:
  ```rust
  pub const TACK_MODES_BCE: ScenarioSpec = ScenarioSpec {
      id: "tack_modes_bce",
      menu_path: &[
          MenuStep { send: b"n", wait_for: "begin testing" },
          MenuStep { send: b"m", wait_for: "modes" },
      ],
      ready_anchor: "modes",
      parser: parse_modes_screen,
  };
  ```
  Note: same menu_path as `TACK_MODES_AM` — both navigate to the modes screen, then assert different facts. The parser extracts ALL known cap labels at once (`parse_modes_screen` returns the full list); each scenario asserts on a different cap.

- [ ] Add `TACK_MODES_BW`, `TACK_MODES_KM`, `TACK_MODES_MIR`, `TACK_MODES_MSGR`, `TACK_MODES_XENL` — same pattern. 6 scenarios total in `modes.rs` after this subsection (`am` from Section 04 + 5 added here, plus `bce` listed above = 7).

  Wait — recount: `am` (from Section 04), `bce`, `bw`, `km`, `mir`, `msgr`, `xenl` = 7 total. Adjust the count below.

- [ ] Add `#[test] fn` wrappers for each:
  ```rust
  #[test]
  fn tack_modes_bce() {
      if !ScenarioRunner::available() { return; }
      let outcome = ScenarioRunner::run(&TACK_MODES_BCE);
      assert!(
          outcome.parsed.capability_labels.iter().any(|c| c == "bce"),
          "expected `bce` in capability_labels, got {:?}\nGrid:\n{}",
          outcome.parsed.capability_labels, outcome.grid_text
      );
      // No insta snapshot for the duplicate-navigation cases — the
      // grid is the same as `tack_modes_am`'s snapshot. We assert
      // a different fact, that's all. The snapshot is deduplicated.
  }
  ```

  **Snapshot deduplication:** since `tack_modes_bce`/`bw`/`km`/`mir`/`msgr`/`xenl` all visit the SAME modes screen, they all produce the same grid. We snapshot once (in `tack_modes_am`) and the rest of the modes-screen tests assert facts WITHOUT calling `insta::assert_snapshot!`. This avoids 6 duplicate `.snap` files.

- [ ] Verify in the parser that all 7 known caps are detected when present:
  ```rust
  // Add to oriterm_core/tests/tack/test_menu/modes/tests.rs (sibling tests file).
  // Tests the parser in isolation against a hand-crafted grid string.
  #[test]
  fn parse_modes_screen_finds_all_known_caps() {
      let grid = "modes test\nam bce bw km mir msgr xenl ...\n";
      let facts = super::parse_modes_screen(grid);
      assert_eq!(
          facts.capability_labels,
          vec!["am", "bce", "bw", "km", "mir", "msgr", "xenl"]
              .into_iter().map(String::from).collect::<Vec<_>>()
      );
  }

  #[test]
  fn parse_modes_screen_handles_missing_caps() {
      let grid = "modes test\nam xenl\n";
      let facts = super::parse_modes_screen(grid);
      assert_eq!(facts.capability_labels, vec!["am".to_string(), "xenl".to_string()]);
  }
  ```
  Restructure `modes.rs` → `modes/mod.rs` + `modes/tests.rs` to fit the sibling-tests convention.

- [ ] Run the modes scenarios:
  ```
  timeout 150 cargo test -p oriterm_core --test tack -- test_menu::modes
  ```
  All 7 must pass.

---

## 05.2 ACS / graphic rendition scenarios

**File(s):** `oriterm_core/tests/tack/test_menu/acs.rs`, `oriterm_core/tests/tack/test_menu/graphic_rendition.rs`

ACS = Alternate Character Set, the DEC line-drawing graphics. Tack tests these via the `n) begin testing` -> `g` (graphic rendition) and `n) begin testing` -> `a` (ACS) submenus. Verify the tack submenu key for each empirically — the smoke test in Section 03 captures the begin-testing menu, look at it to confirm the keys.

- [ ] Create `oriterm_core/tests/tack/test_menu/acs.rs`:
  ```rust
  use oriterm_test_support::tack_framework::{MenuStep, ScenarioRunner, ScenarioSpec, ScreenFacts};

  /// Custom parser for the ACS screen: scans for line-drawing chars
  /// (the DEC special graphics codepoints `\u{2500}`–`\u{257F}`) and
  /// records the count of distinct line-drawing chars found.
  fn parse_acs_screen(grid: &str) -> ScreenFacts {
      let mut chars: std::collections::HashSet<char> = std::collections::HashSet::new();
      for ch in grid.chars() {
          if ('\u{2500}'..='\u{257F}').contains(&ch) {
              chars.insert(ch);
          }
      }
      ScreenFacts {
          header_text: grid.lines().next().unwrap_or("").to_string(),
          capability_labels: Vec::new(),
          notes: vec![format!("distinct_line_drawing_chars={}", chars.len())],
      }
  }

  pub const TACK_ACS_GRAPHIC_CHARS: ScenarioSpec = ScenarioSpec {
      id: "tack_acs_graphic_chars",
      menu_path: &[
          MenuStep { send: b"n", wait_for: "begin testing" },
          // Update this key after Section 03 smoke test reveals the
          // exact tack key for the ACS submenu (likely `a` or `g`).
          MenuStep { send: b"a", wait_for: "ACS" },
      ],
      ready_anchor: "ACS",
      parser: parse_acs_screen,
  };

  #[test]
  fn tack_acs_graphic_chars() {
      if !ScenarioRunner::available() { return; }
      let outcome = ScenarioRunner::run(&TACK_ACS_GRAPHIC_CHARS);
      // Tack's ACS screen draws box-drawing borders — at least 4
      // distinct line chars must be present (vertical, horizontal,
      // top-left, top-right at minimum).
      let count_note = outcome.parsed.notes.iter()
          .find(|n| n.starts_with("distinct_line_drawing_chars="))
          .expect("parser must record distinct_line_drawing_chars");
      let count: usize = count_note
          .trim_start_matches("distinct_line_drawing_chars=")
          .parse()
          .expect("count is integer");
      assert!(
          count >= 4,
          "expected ≥4 distinct line-drawing chars, got {count}\nGrid:\n{}",
          outcome.grid_text
      );
      insta::assert_snapshot!(outcome.id, outcome.grid_text);
  }
  ```

- [ ] Create `oriterm_core/tests/tack/test_menu/graphic_rendition.rs`:
  ```rust
  use oriterm_test_support::tack_framework::{MenuStep, ScenarioRunner, ScenarioSpec, ScreenFacts};

  /// Parser for the graphic rendition screen: looks for SGR style
  /// labels (bold, dim, italic, underline, blink, reverse, invisible).
  /// Tack draws each label in the corresponding style — we can't
  /// inspect styles from grid_text alone (it's plain chars), so this
  /// parser just verifies the LABELS are present. Styles are the
  /// domain of the GPU golden tests in Section 07.
  fn parse_graphic_rendition_screen(grid: &str) -> ScreenFacts {
      const SGR_LABELS: &[&str] = &[
          "bold", "dim", "underline", "blink", "reverse",
      ];
      let mut found = Vec::new();
      for label in SGR_LABELS {
          if grid.contains(label) {
              found.push((*label).to_string());
          }
      }
      ScreenFacts {
          header_text: grid.lines().next().unwrap_or("").to_string(),
          capability_labels: found,
          notes: Vec::new(),
      }
  }

  pub const TACK_GRAPHIC_RENDITION_SGR: ScenarioSpec = ScenarioSpec {
      id: "tack_graphic_rendition_sgr",
      menu_path: &[
          MenuStep { send: b"n", wait_for: "begin testing" },
          // Update key after Section 03 captures the begin-testing menu.
          MenuStep { send: b"g", wait_for: "graphic" },
      ],
      ready_anchor: "graphic",
      parser: parse_graphic_rendition_screen,
  };

  #[test]
  fn tack_graphic_rendition_sgr() {
      if !ScenarioRunner::available() { return; }
      let outcome = ScenarioRunner::run(&TACK_GRAPHIC_RENDITION_SGR);
      // At least bold and reverse must be present — they're the
      // SGRs every terminal supports, including ori_term.
      assert!(outcome.parsed.capability_labels.contains(&"bold".to_string()));
      assert!(outcome.parsed.capability_labels.contains(&"reverse".to_string()));
      insta::assert_snapshot!(outcome.id, outcome.grid_text);
  }
  ```

- [ ] Wire both files into `oriterm_core/tests/tack/test_menu/mod.rs`:
  ```rust
  pub mod acs;
  pub mod graphic_rendition;
  pub mod modes;
  ```

- [ ] Run: `timeout 150 cargo test -p oriterm_core --test tack -- test_menu::acs test_menu::graphic_rendition`. Both scenarios must pass.

---

## 05.3 Color scenarios (size matrix)

**File(s):** `oriterm_core/tests/tack/test_menu/color.rs`

Color is the highest-value tack screen for ori_term: it tests `setaf`/`setab` for both ANSI 16 and 256-color, plus the named-color list. We run it at three sizes (80x24, 97x33, 120x40) to catch cell-loop or palette regressions that only manifest at non-default sizes.

- [ ] Create `oriterm_core/tests/tack/test_menu/color.rs`:
  ```rust
  use oriterm_test_support::tack_framework::{MenuStep, ScenarioRunner, ScenarioSpec, ScreenFacts};

  /// Parser for the color screen: extracts named color rows.
  ///
  /// Tack's color screen labels each color sample with its name
  /// (`black`, `red`, `green`, `yellow`, `blue`, `magenta`, `cyan`,
  /// `white`). We grep for the literal names. Style/RGB validation
  /// is deferred to GPU golden images in Section 07.
  fn parse_color_screen(grid: &str) -> ScreenFacts {
      const NAMED_COLORS: &[&str] = &[
          "black", "red", "green", "yellow",
          "blue", "magenta", "cyan", "white",
      ];
      let mut found = Vec::new();
      for c in NAMED_COLORS {
          if grid.contains(c) {
              found.push((*c).to_string());
          }
      }
      ScreenFacts {
          header_text: grid.lines().next().unwrap_or("").to_string(),
          capability_labels: found,
          notes: Vec::new(),
      }
  }

  pub const TACK_COLOR: ScenarioSpec = ScenarioSpec {
      id: "tack_color",
      menu_path: &[
          MenuStep { send: b"n", wait_for: "begin testing" },
          MenuStep { send: b"c", wait_for: "color" },
      ],
      ready_anchor: "color",
      parser: parse_color_screen,
  };

  fn run_color_at(cols: u16, rows: u16, snapshot_name: &str) {
      if !ScenarioRunner::available() { return; }
      let outcome = ScenarioRunner::run_at(&TACK_COLOR, cols, rows);
      // All 8 ANSI named colors must be present.
      let expected = vec![
          "black", "red", "green", "yellow",
          "blue", "magenta", "cyan", "white",
      ];
      for c in &expected {
          assert!(
              outcome.parsed.capability_labels.iter().any(|l| l == c),
              "missing color {c:?} at {cols}x{rows}: {:?}\nGrid:\n{}",
              outcome.parsed.capability_labels, outcome.grid_text
          );
      }
      insta::assert_snapshot!(snapshot_name, outcome.grid_text);
  }

  #[test]
  fn tack_color_80x24() { run_color_at(80, 24, "tack_color_80x24"); }

  #[test]
  fn tack_color_97x33() { run_color_at(97, 33, "tack_color_97x33"); }

  #[test]
  fn tack_color_120x40() { run_color_at(120, 40, "tack_color_120x40"); }
  ```

- [ ] Wire into `mod.rs`: `pub mod color;`

- [ ] Run all 3 color scenarios. Each must pass on first run (after `INSTA_UPDATE=1` capture).

---

## 05.4 Cursor movement scenarios (size matrix)

**File(s):** `oriterm_core/tests/tack/test_menu/cursor_movement.rs`

Cursor movement is the second-highest value screen: it tests `cup`, `csr`, `hpa`, `vpa`, scroll regions, and origin mode. Same size matrix as color.

- [ ] Create `oriterm_core/tests/tack/test_menu/cursor_movement.rs`:
  ```rust
  use oriterm_test_support::tack_framework::{MenuStep, ScenarioRunner, ScenarioSpec, ScreenFacts};

  /// Parser for the cursor movement screen: looks for the cursor
  /// position labels tack draws (`cup`, `hpa`, `vpa`, `csr`, `cuu`,
  /// `cud`, `cub`, `cuf`).
  fn parse_cursor_screen(grid: &str) -> ScreenFacts {
      const CURSOR_CAPS: &[&str] = &[
          "cup", "hpa", "vpa", "csr",
          "cuu", "cud", "cub", "cuf",
      ];
      let mut found = Vec::new();
      for c in CURSOR_CAPS {
          if grid.contains(c) {
              found.push((*c).to_string());
          }
      }
      ScreenFacts {
          header_text: grid.lines().next().unwrap_or("").to_string(),
          capability_labels: found,
          notes: Vec::new(),
      }
  }

  pub const TACK_CURSOR_MOVEMENT: ScenarioSpec = ScenarioSpec {
      id: "tack_cursor_movement",
      menu_path: &[
          MenuStep { send: b"n", wait_for: "begin testing" },
          MenuStep { send: b"u", wait_for: "cursor" },  // verify key in 03 capture
      ],
      ready_anchor: "cursor",
      parser: parse_cursor_screen,
  };

  fn run_cursor_at(cols: u16, rows: u16, snapshot_name: &str) {
      if !ScenarioRunner::available() { return; }
      let outcome = ScenarioRunner::run_at(&TACK_CURSOR_MOVEMENT, cols, rows);
      // cup is the universal cursor positioning cap — must be present.
      assert!(
          outcome.parsed.capability_labels.iter().any(|c| c == "cup"),
          "expected cup at {cols}x{rows}, got {:?}",
          outcome.parsed.capability_labels
      );
      insta::assert_snapshot!(snapshot_name, outcome.grid_text);
  }

  #[test]
  fn tack_cursor_movement_80x24() { run_cursor_at(80, 24, "tack_cursor_movement_80x24"); }

  #[test]
  fn tack_cursor_movement_97x33() { run_cursor_at(97, 33, "tack_cursor_movement_97x33"); }

  #[test]
  fn tack_cursor_movement_120x40() { run_cursor_at(120, 40, "tack_cursor_movement_120x40"); }
  ```

- [ ] Wire into `mod.rs`: `pub mod cursor_movement;`

- [ ] **TPR checkpoint** — `/tpr-review` covering 05.1–05.4 (the bulk of the test menu catalog). Catches: scenario IDs that drift from snapshot file names, parsers with off-by-one bugs in capability detection, missing skip gates, brittle ready_anchors that work at one size and not another.

---

## 05.4b Additional test menu screens (pad timing, function key test, string caps)

**File(s):** `crates/oriterm_test_support/src/tack_framework/scenarios/pad_timing.rs`, `function_key_test.rs`, `string_caps.rs`, `oriterm_core/tests/tack/test_menu/pad_timing.rs`, `function_key_test.rs`, `string_caps.rs`

Tack's `n) begin testing` submenu in recent ncurses releases exposes MORE screens than just modes/ACS/color/cursor. The ncurses v6.x tack menu is:
- `b)` change specific caps
- `c)` color
- `e)` edit terminfo
- `f)` function keys
- `k)` send strings (terminfo string capabilities)
- `l)` labels
- `m)` modes (glitches)
- `o)` output
- `p)` pad timing
- `s)` subpads / alternate character sets (ACS)
- `u)` cursor movement

The plan's original scope covered 5 of these (modes, ACS, color, cursor, graphic rendition). The remaining in-scope items below are the ones that exercise real ori_term capabilities (not just tack internals). The rule: if a test menu screen exercises a capability that ori_term either implements or advertises in `extra/ori_term.info`, it IS in scope for this plan. The broken window policy in CLAUDE.md forbids scoping down — every tack test screen reachable from the begin-testing menu MUST be either a scenario or documented with a concrete reason it's excluded.

- [ ] **Pad timing (`p`)** — tack measures padding delays for a curated set of capabilities. This validates that ori_term honors millisecond padding declarations in the terminfo `$<N>` syntax. Scenario: navigate `[n] [p]`, wait for pad screen anchor, snapshot, assert the screen header. Parser extracts pad-timing numeric values if tack prints them. Add `TACK_PAD_TIMING` const + `parse_pad_timing` + test wrapper. The pad timing screen runs at 80x24 only (pad timing is size-independent).

- [ ] **Function key test (`f`)** — tack's interactive function-key probe that asks the user to press each key and records the bytes received. This is NOT automatable from tack's side (it waits for user input for every key), so it CANNOT become a snapshot scenario. Instead, cover it via Section 08's in-crate sibling test at `oriterm/src/key_encoding/terminfo_xcheck.rs` — those tests exercise the same ground (encode_key vs. terminfo sequences) through ori_term's internal encoder, which IS automatable. Document in a comment at the top of `test_menu/function_key_test.rs`:
  ```rust
  //! tack's `n) begin testing -> f) function keys` screen is interactive
  //! (blocks waiting for user keystrokes). Cannot be automated from the
  //! PTY test harness — covered instead by Section 08's in-crate sibling
  //! test at `oriterm/src/key_encoding/terminfo_xcheck.rs`, which
  //! validates the same encode_key <-> terminfo correspondence without
  //! needing a live tack process.
  ```
  Create the file as a doc-only stub so the exclusion is VISIBLE in the test tree, not silent.

- [ ] **String capabilities (`k) send strings`)** — tack sends each declared string cap and shows what the terminal does. Useful for validating `clear`, `el`, `ed`, `smcup`, `rmcup`, `is1`/`is2`/`is3` (init strings), and `rs1`/`rs2`/`rs3` (reset strings). Scenario: navigate `[n] [k]`, wait for send-strings screen, snapshot, assert known caps listed. Add `TACK_SEND_STRINGS` const + `parse_send_strings` + test wrapper.

- [ ] **Labels (`l)`** — tack lists the terminfo's label capabilities (`lf0` through `lf10`) and their declared values. Low priority because ori_term does not declare soft-labels in `extra/ori_term.info` (no physical label area). Add `TACK_LABELS` const + test wrapper that asserts the screen shows "no labels declared" or similar. Document in-code:
  ```rust
  // ori_term does not declare label caps (lf0-lf10, fsl, tsl, dsl, wsl)
  // because it has no soft-label area. tack's label screen should show
  // the capability as absent. This test asserts the absence — if
  // ori_term ever adds label support, this test will flip to ensure
  // the labels render correctly.
  ```

- [ ] **Edit terminfo (`e)`** — interactive terminfo editor. Like the function-key screen, it blocks waiting for user input and cannot be automated. Create a doc-only stub in `test_menu/edit_terminfo.rs` documenting the exclusion reason.

- [ ] **Output (`o)`** — tack's output demo, dumps terminal capability strings to the screen. Overlaps heavily with `k)` send strings — include one of the two, not both. The plan picks `k)` (send strings) as the more structured variant and documents `o)` as a duplicate in a comment.

- [ ] Wire all new modules into `oriterm_core/tests/tack/test_menu/mod.rs`:
  ```rust
  pub mod acs;
  pub mod color;
  pub mod cursor_movement;
  pub mod edit_terminfo;         // doc-only stub
  pub mod function_key_test;     // doc-only stub
  pub mod graphic_rendition;
  pub mod labels;
  pub mod modes;
  pub mod pad_timing;
  pub mod send_strings;
  ```

---

## 05.5 Determinism + size matrix verification

**File(s):** None (verification only)

The scenarios are non-trivial — each spawns a real tack child, navigates menus, captures, parses. Verify they run deterministically before closing the section.

- [ ] Run the entire test_menu submodule 10 times in a row:
  ```
  for i in $(seq 1 10); do
      timeout 150 cargo test -p oriterm_core --test tack -- test_menu || break
  done
  ```
  All 10 must pass. Any failure → file `/add-bug` immediately and treat as blocker.

- [ ] Run with `--test-threads=1` to confirm scenarios don't depend on parallelism:
  ```
  timeout 150 cargo test -p oriterm_core --test tack -- test_menu --test-threads=1
  ```
  Must pass.

- [ ] Run with `--test-threads=4` to confirm scenarios DO work in parallel:
  ```
  timeout 150 cargo test -p oriterm_core --test tack -- test_menu --test-threads=4
  ```
  Must pass. Parallel runs surface PTY/temp-dir collision bugs — `TerminfoEnv` uses `tempfile::TempDir` (unique per call) so this should work, but verify.

- [ ] Cross-compile gate: `cargo build --target x86_64-pc-windows-gnu -p oriterm_core --tests`. All test_menu modules must compile on Windows (they skip at runtime, but they MUST compile).

- [ ] Snapshot directory size check: `find oriterm_core/tests/tack/snapshots -name '*.snap' | wc -l` should equal the number of UNIQUE scenarios that snapshot (modes-family scenarios share one snapshot; ACS, graphic_rendition each have one; color and cursor each have 3 for the size matrix). Sanity-check the count matches expectations (~10-12 .snap files).

---

## 05.R Third Party Review Findings

<!-- Reserved for Codex or other external reviewers. -->

- None.

---

## 05.N Completion Checklist

- [ ] `oriterm_core/tests/tack/test_menu/modes/` (or `modes.rs`): TACK_MODES_AM/BCE/BW/KM/MIR/MSGR/XENL — 7 scenarios, all passing
- [ ] `oriterm_core/tests/tack/test_menu/acs.rs`: TACK_ACS_GRAPHIC_CHARS scenario passing
- [ ] `oriterm_core/tests/tack/test_menu/graphic_rendition.rs`: TACK_GRAPHIC_RENDITION_SGR scenario passing
- [ ] `oriterm_core/tests/tack/test_menu/color.rs`: TACK_COLOR scenario at 3 sizes (80x24, 97x33, 120x40), all passing
- [ ] `oriterm_core/tests/tack/test_menu/cursor_movement.rs`: TACK_CURSOR_MOVEMENT at 3 sizes, all passing
- [ ] `oriterm_core/tests/tack/test_menu/pad_timing.rs`: TACK_PAD_TIMING scenario passing
- [ ] `oriterm_core/tests/tack/test_menu/send_strings.rs`: TACK_SEND_STRINGS scenario passing
- [ ] `oriterm_core/tests/tack/test_menu/labels.rs`: TACK_LABELS scenario passing (asserts absence of label caps)
- [ ] `oriterm_core/tests/tack/test_menu/function_key_test.rs`: doc-only stub documenting the interactive exclusion
- [ ] `oriterm_core/tests/tack/test_menu/edit_terminfo.rs`: doc-only stub documenting the interactive exclusion
- [ ] Total scenarios: 7 (modes) + 1 (acs) + 1 (gr) + 3 (color) + 3 (cursor) + 1 (pad) + 1 (send strings) + 1 (labels) = **18 scenarios** (≥12 success criterion satisfied)
- [ ] Each parser has sibling-file unit tests (`parser/tests.rs`) covering happy path, missing labels, empty grid
- [ ] All 18 scenarios pass deterministically (10 reruns clean for the entire test_menu submodule)
- [ ] Both `--test-threads=1` and `--test-threads=4` runs pass
- [ ] Cross-compile for `x86_64-pc-windows-gnu` succeeds
- [ ] Snapshot count under `oriterm_core/tests/tack/snapshots/` matches expected unique scenarios (~10-12 .snap files)
- [ ] No file in `test_menu/` exceeds 500 lines
- [ ] `./build-all.sh` green
- [ ] `./clippy-all.sh` green
- [ ] `timeout 150 ./test-all.sh` green
- [ ] Plan annotation cleanup
- [ ] All TPR checkpoint findings resolved (see `05.R`)
- [ ] **Plan sync**:
  - [ ] This section's frontmatter `status` → `complete`
  - [ ] `00-overview.md` Quick Reference table: Section 05 marked Complete
  - [ ] `00-overview.md` Mission Success Criteria #7 ticked
  - [ ] `index.md` Section 05 status updated
- [ ] `/tpr-review` final pass clean
- [ ] `/impl-hygiene-review last commit` final pass clean (after TPR)

**Exit Criteria:** `timeout 150 cargo test -p oriterm_core --test tack -- test_menu` runs all 18 test menu scenarios (7 modes + 1 ACS + 1 graphic rendition + 3 color sizes + 3 cursor sizes + 1 pad_timing + 1 send_strings + 1 labels) to completion in under 2 minutes. Every scenario has a programmatic semantic assertion beyond the snapshot. Determinism verified across 10 reruns and both single-/multi-threaded modes. Cross-compile gate passes for Windows. The test menu catalog is complete and Section 06 (tools menu) follows the same pattern.
