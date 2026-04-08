---
section: "06"
title: "Tack Scenarios: Tools Menu"
status: not-started
reviewed: false
needs_re_review_after: "04"
re_review_reason: "Section 04 (post-Agent-1 expansion) defines the framework API that Section 06 consumes. Section 06's existing code samples reference the OLDER pre-expansion API: `MenuStep { send, wait_for }` (missing `or_wait_for`), `ScenarioSpec { id, menu_path, ready_anchor, parser }` (missing `screen_id` and `quit_path`), `outcome.id` for snapshot naming (must be `outcome.snapshot_name()`), parsers/consts defined inline in the test target (must live in `oriterm_test_support::tack_framework::scenarios::*`), blind `grid.contains` for short escape-prefix markers (must use `grid_has_token`/`grid_line_starts_with`/`grid_find_field`). Section 06 MUST NOT be implemented until its code samples are rewritten against Section 04 final and a fresh `/review-plan` pass flips this `reviewed` flag back to `true`. ADDITIONAL DRIFT introduced by Section 05's Agent-1 / Agent-2 / Agent-3 review pass: (a) `PhaseSpec` + `ScenarioRunner::run_phase` / `run_phase_at` is a NEW framework primitive — Section 06 should evaluate which tools-menu screens scroll mid-run (the SGR display sweep is a candidate) and use `PhaseSpec` for those instead of `ScenarioSpec`, (b) `tack_version_supported()` is now AND-combined into `ScenarioRunner::available()` — Section 06 inherits this gate automatically and the loud-skip diagnostic is a NEW user-visible behavior, (c) `BEGIN_TESTING_INVENTORY` is the discovery pattern Section 05 introduces for the begin-testing menu — Section 06 SHOULD adopt the same pattern with a `TOOLS_MENU_INVENTORY` for the `t)` submenu (same forcing-function rationale: stop guessing keys), (d) the `cap_coverage_matrix` test in Section 05.5 uses a `CapCoverageContribution` per consuming section (Pivot 5 of /review-plan): Section 06 owns `cap_coverage/section_06.rs` and MUST add the tools-menu caps it covers (u6/u7/u8/u9, Cr/Cs, Ms, Smulx, Setulc, Sync, BD/BE/PS/PE, AX/XT, hs/dsl/fsl/tsl, Se/Ss, XF/kxIN/kxOUT, Tc, RGB, the OSC family, ENQ/ACK) to `CONTRIBUTION.covered` AND remove the matching `CONTRIBUTION.exempt` entries when its scenarios land — the matrix test fires on stale exemptions, (e) the `unverified_menu_key()` / `unverified_anchor()` runtime sentinels (replacing the original `compile_error!` placeholders) are available for Section 06 scenarios that need to be authored before their menu keys are pinned via the tools-menu inventory discovery."
goal: "Populate the scenario catalog with tack's `t) tools` submenu screens: ANSI status reports (DA/DSR/DECRQM), SGR mode display (SGR 0-79), and character set tools (G0/G1/GL/GR banks). Same dual-file layout as Section 05: const ScenarioSpec values + per-scenario parsers in `crates/oriterm_test_support/src/tack_framework/scenarios/{status_reports,sgr_modes,character_sets}.rs`, test wrapper `#[test] fn`s in `oriterm_core/tests/tack/tools_menu/*.rs`. The tools menu is where tack INSPECTS what the terminal advertises rather than testing fixed protocols, so the parsers focus on extracting the inspected report contents."
success_criteria:
  - "`oriterm_core/tests/tack/tools_menu/` contains modules: status_reports (DA/DSR/DECRQM responses), sgr_modes (SGR 0-79 sweep), character_sets (G0/G1/GL/GR banks)"
  - "At least 7 scenarios across these modules — exact list in subsection checklists"
  - "Each scenario has a custom parser when needed (e.g., `parse_da_response` extracts the DA1 reply string from the tools menu output)"
  - "Each scenario has at least one programmatic semantic assertion beyond the insta snapshot"
  - "All scenarios skip cleanly when `tack`/`tic` are unavailable via `ScenarioRunner::available()`"
  - "All scenarios pass deterministically (10 consecutive reruns)"
  - "`timeout 150 cargo test -p oriterm_core --test tack -- tools_menu` passes"
  - "Satisfies mission criterion: 'Tack tool scenarios cover: ANSI status reports, SGR modes, character sets'"
inspired_by:
  - "ori_term Section 04 framework (plans/tack-conformance/section-04-scenario-framework.md)"
  - "ori_term Section 05 catalog pattern (plans/tack-conformance/section-05-test-menu-scenarios.md — modules + parsers + size matrix)"
  - "ori_term vttest menu6 (oriterm_core/tests/vttest/menu6.rs:walk_menu6_subscreens — DA/DSR report assertions)"
  - "ncurses tack source (tools menu items: send strings, receive strings, ANSI status reports, character sets)"
depends_on: ["04", "05"]
depends_on_contract:
  - section: "05"
    contract: "Section 06 consumes Section 05's framework extensions (PhaseSpec + run_phase[_at] for any tools-menu screen that scrolls — e.g. SGR sweep), the tack_version_supported() gate (inherited automatically via ScenarioRunner::available()), the BEGIN_TESTING_INVENTORY discovery pattern (Section 06 should adopt an analogous TOOLS_MENU_INVENTORY for its `t)` submenu), and the cap_coverage_matrix CapCoverageContribution extension contract (Section 06 owns `cap_coverage/section_06.rs` and must move tools-menu caps from `exempt` to `covered` as scenarios land). Section 06 can start AFTER Section 05's M1 milestone (PhaseSpec + version gate + inventory) lands; Section 06 does NOT need to wait for Section 05's M2 milestone (color/cursor/cap-coverage matrix). The strict frontmatter ordering reflects the safer 'wait for full 05' default; the contract spec here is the granular reality."
third_party_review:
  status: none
  updated: null
sections:
  - id: "06.1"
    title: "Status reports scenarios (DA/DSR/DECRQM)"
    status: not-started
  - id: "06.2"
    title: "SGR mode scenarios (SGR 0-79 sweep)"
    status: not-started
  - id: "06.3"
    title: "Character set scenarios (G0/G1/GL/GR banks)"
    status: not-started
  - id: "06.4"
    title: "Determinism + parser unit tests"
    status: not-started
  - id: "06.R"
    title: "Third Party Review Findings"
    status: not-started
  - id: "06.N"
    title: "Completion Checklist"
    status: not-started
---

# Section 06: Tack Scenarios — Tools Menu

**Status:** Not Started — `reviewed: false`, BLOCKED on re-review after Section 04 lands.

**API drift warning (BLOCKING — DO NOT IMPLEMENT FROM THIS SECTION'S CODE SAMPLES VERBATIM).** This section was authored before Agent 1's expansion of Section 04 and references an OBSOLETE framework API. Every code sample below uses the pre-expansion shapes:

- `MenuStep { send, wait_for }` literal struct construction — the new shape is `MenuStep::new(send, wait_for)` or the full three-field literal `MenuStep { send, wait_for, or_wait_for }`. Tools-menu screens are a particularly strong fit for `or_wait_for` because the tools sub-screens often expose pager prompts and "press any key" continuations that the navigator must handle.
- `ScenarioSpec { id, menu_path, ready_anchor, parser }` literal — the new shape requires `screen_id` and `quit_path: Option<...>` fields.
- `outcome.id` for snapshot naming — the new shape is `outcome.snapshot_name()` so DA1/DA2/DSR scenarios that share the same status-reports screen can share a snapshot file.
- `parse_status_reports`, `parse_sgr_modes`, `parse_character_sets`, `parse_enq_ack`, `parse_osc_queries` defined inline in `oriterm_core/tests/tack/tools_menu/*.rs` — the new layout puts ALL `pub const TACK_TOOLS_*: ScenarioSpec` values AND their parser fns in `crates/oriterm_test_support/src/tack_framework/scenarios/<family>.rs` (workspace crate). The test target file holds ONLY `#[test] fn` wrappers and a `use oriterm_test_support::tack_framework::scenarios::<family>::*;` import. This is the SSOT principle from Section 04 — Section 07 GPU goldens reference the same const, so the const lives in the shared crate.
- `grid.contains("[?")`, `grid.contains("c")`, `grid.contains("$y")` for response-marker detection — the new shape uses `crate::tack_framework::parser::tokens::{grid_has_token, grid_line_starts_with, grid_find_field}`. Bare `grid.contains("c")` matches every lowercase `c` in the screen including the literal English word "color" — the M3 Codex finding fix from Section 04 exists exactly for this antipattern. The status-reports parser has the strongest motivation for switching: a one-character contains check is the textbook false-positive case.
- The IMPORTANT "dual-file layout" call-out in this section's preamble was authored against the OLD assumption that consts lived in the test target. Re-read it as: const ScenarioSpec values + parser fns ALWAYS live in `oriterm_test_support::tack_framework::scenarios::*`. The test target file is a thin `#[test] fn` wrapper with one `use` line.

The rewrite contract for Section 06 is fixed: keep the SCENARIO LIST and the assertion intent, replace every code sample with the new API shapes, move every const + parser fn into the workspace crate, and re-run `/review-plan` against this section to flip `reviewed: true`. Treat the code blocks below as PSEUDOCODE — they describe what to build, not literally what to type.

**Goal:** Cover tack's `t) tools` submenu with structured scenarios. Tools differ from the test menu in that they INSPECT what the terminal reports (DA/DSR/DECRQM responses, SGR mode names, character set bank state) instead of testing fixed protocols. The scenario parsers focus on extracting the report contents from the screen and asserting they match what ori_term should advertise.

**Layout reminder (same as Section 05):** const ScenarioSpec values and parser functions go in `crates/oriterm_test_support/src/tack_framework/scenarios/`. Test wrapper `#[test] fn`s go in `oriterm_core/tests/tack/tools_menu/`. The two files share nothing except the import line `use oriterm_test_support::tack_framework::scenarios::{status_reports,sgr_modes,character_sets}::*;` in the test wrapper. The const layout makes Section 07's GPU goldens reference the SAME consts.

**IMPORTANT — dual-file layout applies to EVERY subsection below.** Subsections 06.1, 06.2, 06.3 show code samples that define `pub const TACK_TOOLS_*` consts alongside `fn parse_*` parsers; those consts and parsers MUST live in `crates/oriterm_test_support/src/tack_framework/scenarios/{family}.rs`, NOT inline in `oriterm_core/tests/tack/tools_menu/{family}.rs`. The test-target files contain ONLY the `#[test] fn` wrappers and the `use oriterm_test_support::tack_framework::scenarios::{family}::*;` import. Every code block that reads `pub const ... = ScenarioSpec { ... }` in this section's code samples implicitly targets the workspace crate file, and every `#[test] fn` block targets the test target file. A single inline file is a hygiene violation (SSOT: scenario knowledge has one canonical home — `oriterm_test_support::tack_framework::scenarios`).

**Success Criteria:**

- [ ] `oriterm_core/tests/tack/tools_menu/` contains: `status_reports.rs`, `sgr_modes.rs`, `character_sets.rs`
- [ ] At least 7 scenarios — exact list in subsections
- [ ] All scenarios pass deterministically (10 consecutive reruns clean)
- [ ] All scenarios skip cleanly when tools unavailable
- [ ] `timeout 150 cargo test -p oriterm_core --test tack -- tools_menu` green
- [ ] Satisfies mission criterion #8

**Context:** The tools menu reflects how a real human uses tack to debug a terminal: launch tack, hit `t`, pick "show DA response", look at what the terminal sent back. Each tool is a one-shot inspection — there's no test pass/fail inside tack, just a captured report. Our scenario parsers extract the report contents and assert they match what ori_term's terminfo and term handler claim to support.

This is also where the existing vttest cross-validation comes in: vttest's menu6 tests assert structurally against DA/DSR/DECRQM responses (e.g., `oriterm_core/tests/vttest/menu6.rs:walk_menu6_subscreens`). The tack tools_menu scenarios should produce the SAME response strings as the vttest tests — if they diverge, one of the two test paths is wrong (or ori_term is non-deterministic in its responses, which would be a real bug).

**Reference implementations:**
- **Section 04** `plans/tack-conformance/section-04-scenario-framework.md`: framework consumed here.
- **Section 05** `plans/tack-conformance/section-05-test-menu-scenarios.md`: catalog pattern followed here.
- **ori_term vttest menu6** `oriterm_core/tests/vttest/menu6.rs`: existing DA/DSR test logic — we cross-validate against the same responses.
- **ncurses tack source** (man page): tools menu items.

**Depends on:** Section 04 (framework) + **Section 05** (PhaseSpec extension, version gate, cap_coverage_matrix extension contract — see Section 05.5b for the contract details). **Cross-validation note:** Section 06's status_reports scenarios should produce the same DA/DSR responses as the vttest menu6 tests — Section 09 verification will diff the two. <!-- reviewed: cohesion fix -->

---

## 06.1 Status reports scenarios (DA/DSR/DECRQM)

**File(s):** `oriterm_core/tests/tack/tools_menu/status_reports.rs`

The tools menu shows ANSI status reports — DA1 (Primary Device Attributes), DA2 (Secondary), DA3 (Tertiary), DSR (Device Status Report), DECRQM (Mode Query). Each tool sends the query and displays the response in the tack tools screen.

- [ ] Create `oriterm_core/tests/tack/tools_menu/status_reports.rs`:
  ```rust
  use oriterm_test_support::tack_framework::{MenuStep, ScenarioRunner, ScenarioSpec, ScreenFacts};

  /// Parser for DA/DSR/DECRQM tool screens.
  ///
  /// Tack draws the response inline. We grep for the canonical
  /// response prefixes:
  ///   DA1 → `\E[?...c` (response starts with `[?` and ends with `c`)
  ///   DA2 → `\E[>...c`
  ///   DSR cursor pos → `\E[<row>;<col>R`
  ///   DECRQM → `\E[?<mode>;<value>$y` or `\E[<mode>;<value>$y`
  ///
  /// The grid contains escape sequences with `\E` rendered or with
  /// the literal byte stripped depending on tack's display mode. We
  /// do a substring scan for the bracket characters that uniquely
  /// identify each response type.
  fn parse_status_reports(grid: &str) -> ScreenFacts {
      let mut found = Vec::new();
      // DA1 marker: [? followed by digits ending in 'c'
      if grid.contains("[?") && grid.contains("c") {
          found.push("DA1".to_string());
      }
      // DA2 marker: [> followed by digits
      if grid.contains("[>") {
          found.push("DA2".to_string());
      }
      // DSR cursor pos marker: ;<digits>R or [<digits>;<digits>R
      if grid.contains(";") && grid.contains("R") {
          found.push("DSR_CPR".to_string());
      }
      // DECRQM marker: $y in the response
      if grid.contains("$y") {
          found.push("DECRQM".to_string());
      }
      ScreenFacts {
          header_text: grid.lines().next().unwrap_or("").to_string(),
          capability_labels: found,
          notes: Vec::new(),
      }
  }

  pub const TACK_TOOLS_DA1: ScenarioSpec = ScenarioSpec {
      id: "tack_tools_da1",
      menu_path: &[
          MenuStep { send: b"t", wait_for: "tools" },
          // The DA1 tool key is observed empirically. Update after
          // first run with INSTA_UPDATE=1.
          MenuStep { send: b"d", wait_for: "DA1" },
      ],
      ready_anchor: "DA1",
      parser: parse_status_reports,
  };

  pub const TACK_TOOLS_DA2: ScenarioSpec = ScenarioSpec {
      id: "tack_tools_da2",
      menu_path: &[
          MenuStep { send: b"t", wait_for: "tools" },
          // Many tack builds have DA2 as a sub-option of the same
          // status-reports menu. Adjust after observation.
          MenuStep { send: b"D", wait_for: "DA2" },
      ],
      ready_anchor: "DA2",
      parser: parse_status_reports,
  };

  pub const TACK_TOOLS_DSR: ScenarioSpec = ScenarioSpec {
      id: "tack_tools_dsr",
      menu_path: &[
          MenuStep { send: b"t", wait_for: "tools" },
          MenuStep { send: b"s", wait_for: "DSR" },  // adjust after observation
      ],
      ready_anchor: "DSR",
      parser: parse_status_reports,
  };

  #[test]
  fn tack_tools_da1() {
      if !ScenarioRunner::available() { return; }
      let outcome = ScenarioRunner::run(&TACK_TOOLS_DA1);
      // ori_term's DA1 response begins with [? — verify the parser
      // detected it.
      assert!(
          outcome.parsed.capability_labels.contains(&"DA1".to_string()),
          "expected DA1 marker in tools_menu DA1 screen, grid:\n{}",
          outcome.grid_text
      );
      insta::assert_snapshot!(outcome.id, outcome.grid_text);
  }

  #[test]
  fn tack_tools_da2() {
      if !ScenarioRunner::available() { return; }
      let outcome = ScenarioRunner::run(&TACK_TOOLS_DA2);
      assert!(
          outcome.parsed.capability_labels.contains(&"DA2".to_string()),
          "expected DA2 marker in tools_menu DA2 screen, grid:\n{}",
          outcome.grid_text
      );
      insta::assert_snapshot!(outcome.id, outcome.grid_text);
  }

  #[test]
  fn tack_tools_dsr() {
      if !ScenarioRunner::available() { return; }
      let outcome = ScenarioRunner::run(&TACK_TOOLS_DSR);
      assert!(
          outcome.parsed.capability_labels.contains(&"DSR_CPR".to_string()),
          "expected DSR cursor position in tools_menu DSR screen, grid:\n{}",
          outcome.grid_text
      );
      insta::assert_snapshot!(outcome.id, outcome.grid_text);
  }
  ```

  **Empirical menu key discovery:** the keys for DA1/DA2/DSR/DECRQM tools are observed by running tack interactively and noting the menu letters. Section 03 captures the main menu; this section needs to extend that capture to the tools submenu. Implementer should run `printf 't\n?\nq\nq\n' | TERM=xterm-256color tack` and look at the captured tools menu output before finalizing the keys.

- [ ] Add sibling parser tests at `tools_menu/status_reports/tests.rs` (after restructuring `status_reports.rs` → `status_reports/mod.rs`):
  ```rust
  #[test]
  fn parse_status_reports_detects_da1_marker() {
      let grid = "DA1 response:\n[?6;4c\n";
      let facts = super::parse_status_reports(grid);
      assert!(facts.capability_labels.contains(&"DA1".to_string()));
  }

  #[test]
  fn parse_status_reports_detects_dsr_cpr() {
      let grid = "DSR Cursor Position:\n[12;34R\n";
      let facts = super::parse_status_reports(grid);
      assert!(facts.capability_labels.contains(&"DSR_CPR".to_string()));
  }

  #[test]
  fn parse_status_reports_handles_empty_grid() {
      let facts = super::parse_status_reports("");
      assert!(facts.capability_labels.is_empty());
  }
  ```

---

## 06.2 SGR mode scenarios (SGR 0-79 sweep)

**File(s):** `oriterm_core/tests/tack/tools_menu/sgr_modes.rs`

Tack's SGR display tool sweeps SGR 0 through 79 and shows each effect. We can't observe the visual effect from grid_text alone (text vs. color/style is the GPU's domain — Section 07), but we CAN observe the labels tack draws and the SGR numbers it lists.

- [ ] Create `oriterm_core/tests/tack/tools_menu/sgr_modes.rs`:
  ```rust
  use oriterm_test_support::tack_framework::{MenuStep, ScenarioRunner, ScenarioSpec, ScreenFacts};

  /// Parser for the SGR display tool screen.
  ///
  /// Tack draws SGR codes 0-79 with labels. We extract the numeric
  /// SGR codes that appear in the grid and record the count.
  fn parse_sgr_modes(grid: &str) -> ScreenFacts {
      let mut sgr_codes: std::collections::BTreeSet<u32> = std::collections::BTreeSet::new();
      // Scan for `SGR <num>` or bare numbers in the 0-79 range that
      // appear adjacent to the literal "SGR" header.
      for word in grid.split_whitespace() {
          if let Ok(n) = word.parse::<u32>() {
              if n < 80 {
                  sgr_codes.insert(n);
              }
          }
      }
      ScreenFacts {
          header_text: grid.lines().next().unwrap_or("").to_string(),
          capability_labels: Vec::new(),
          notes: vec![format!("sgr_codes_count={}", sgr_codes.len())],
      }
  }

  pub const TACK_TOOLS_SGR: ScenarioSpec = ScenarioSpec {
      id: "tack_tools_sgr",
      menu_path: &[
          MenuStep { send: b"t", wait_for: "tools" },
          // Update key after observing tack's tools menu.
          MenuStep { send: b"r", wait_for: "SGR" },
      ],
      ready_anchor: "SGR",
      parser: parse_sgr_modes,
  };

  #[test]
  fn tack_tools_sgr() {
      if !ScenarioRunner::available() { return; }
      let outcome = ScenarioRunner::run(&TACK_TOOLS_SGR);
      // The SGR sweep should display at least the canonical SGRs:
      // 0 (reset), 1 (bold), 4 (underline), 7 (reverse), 22 (normal
      // intensity), 24 (no underline), 27 (no reverse). Extract the
      // count from the parser's note.
      let count_note = outcome.parsed.notes.iter()
          .find(|n| n.starts_with("sgr_codes_count="))
          .expect("parser must record sgr_codes_count");
      let count: usize = count_note
          .trim_start_matches("sgr_codes_count=")
          .parse()
          .expect("integer count");
      assert!(
          count >= 7,
          "expected ≥7 SGR codes on tools sgr screen, got {count}\nGrid:\n{}",
          outcome.grid_text
      );
      insta::assert_snapshot!(outcome.id, outcome.grid_text);
  }
  ```

---

## 06.3 Character set scenarios (G0/G1/GL/GR banks)

**File(s):** `oriterm_core/tests/tack/tools_menu/character_sets.rs`

Tack's character set tools test G0/G1 designation (`\E(`, `\E)`) and GL/GR locking (SI, SO, LS2, LS3, SS2, SS3). The test screen shows the rendered characters from each bank — line-drawing chars for the DEC special graphics set, etc.

- [ ] Create `oriterm_core/tests/tack/tools_menu/character_sets.rs`:
  ```rust
  use oriterm_test_support::tack_framework::{MenuStep, ScenarioRunner, ScenarioSpec, ScreenFacts};

  /// Parser for the character set tool screen.
  ///
  /// Tack designates G0 to DEC special graphics, then draws ASCII
  /// chars that should be rendered as line-drawing (lqkx etc map to
  /// │─┌─┐). We count distinct line-drawing chars in the output.
  fn parse_character_sets(grid: &str) -> ScreenFacts {
      let mut box_chars: std::collections::HashSet<char> = std::collections::HashSet::new();
      for ch in grid.chars() {
          if ('\u{2500}'..='\u{257F}').contains(&ch) {
              box_chars.insert(ch);
          }
      }
      ScreenFacts {
          header_text: grid.lines().next().unwrap_or("").to_string(),
          capability_labels: Vec::new(),
          notes: vec![format!("box_drawing_chars={}", box_chars.len())],
      }
  }

  pub const TACK_TOOLS_G0_DEC_GRAPHICS: ScenarioSpec = ScenarioSpec {
      id: "tack_tools_g0_dec_graphics",
      menu_path: &[
          MenuStep { send: b"t", wait_for: "tools" },
          // Update key after observation.
          MenuStep { send: b"c", wait_for: "character" },
      ],
      ready_anchor: "character",
      parser: parse_character_sets,
  };

  #[test]
  fn tack_tools_g0_dec_graphics() {
      if !ScenarioRunner::available() { return; }
      let outcome = ScenarioRunner::run(&TACK_TOOLS_G0_DEC_GRAPHICS);
      let count_note = outcome.parsed.notes.iter()
          .find(|n| n.starts_with("box_drawing_chars="))
          .expect("parser must record box_drawing_chars");
      let count: usize = count_note
          .trim_start_matches("box_drawing_chars=")
          .parse()
          .expect("integer count");
      // DEC graphics designation should produce at least 4 distinct
      // box-drawing chars (corners and edges).
      assert!(
          count >= 4,
          "expected ≥4 box-drawing chars after G0 DEC designation, got {count}\nGrid:\n{}",
          outcome.grid_text
      );
      insta::assert_snapshot!(outcome.id, outcome.grid_text);
  }
  ```

- [ ] Wire all three modules into `oriterm_core/tests/tack/tools_menu/mod.rs`:
  ```rust
  //! Tack `t) tools` submenu scenarios — see Section 06.

  pub mod character_sets;
  pub mod sgr_modes;
  pub mod status_reports;
  ```

- [ ] Add `mod tools_menu;` to `oriterm_core/tests/tack/main.rs`.

- [ ] **TPR checkpoint** — `/tpr-review` covering 06.1–06.3 (the entire tools menu catalog). Catches: parser regex bugs, ready_anchor mismatches, missed `q\n` quit on the deeper tools sub-menus.

---

## 06.3b Additional tools menu screens (scan codes, enq/ack, OSC queries)

**File(s):** `crates/oriterm_test_support/src/tack_framework/scenarios/enq_ack.rs`, `osc_queries.rs`, `scan_codes.rs`, matching test wrappers under `oriterm_core/tests/tack/tools_menu/`

Tack's `t) tools` submenu in recent ncurses releases exposes MORE tools than just DA/DSR, SGR, and character sets. The full v1.08 tools menu is:
- `a)` ANSI status reports (DA/DSR/DECRQM) — covered in 06.1
- `c)` character sets — covered in 06.3
- `e)` ENQ/ACK handshake (u8/u9 caps)
- `g)` generic OSC queries (color palette, title, cursor)
- `m)` modem status (scan codes for arrows, F-keys as emitted, echoing)
- `s)` SGR display — covered in 06.2
- `x)` decompile terminfo

Per the broken-window policy, every reachable tool must be covered or have a concrete in-code exclusion stub.

- [ ] **ENQ/ACK (`e)`)** — tack sends an ENQ (0x05) via the `u8` capability and waits for the terminal's ACK reply declared in the `u9` capability. Mirrors the ori_term term handler's ENQ response. Add `TACK_TOOLS_ENQ_ACK` const + `parse_enq_ack` parser that extracts the received ACK string from the tools screen. Verify in the test assertion that the received ACK matches what `u9` declares in `extra/ori_term.info` (cross-reference with Section 02).

- [ ] **Generic OSC queries (`g)`)** — tack sends OSC 10 (get foreground color), OSC 11 (get background), OSC 4;N (get palette entry N), OSC 2 (set window title) etc. and records the responses. ori_term implements the OSC 10/11 query responses in `oriterm_core/src/term/handler/` — this scenario validates the responses end-to-end. Add `TACK_TOOLS_OSC_QUERIES` const + `parse_osc_queries` parser that extracts OSC response prefixes (`]10;rgb:` etc.). Assert at least OSC 10 and OSC 11 responses are present.

- [ ] **Scan codes / modem status (`m)`)** — tack's scan-code tool is interactive (prompts the user to press keys), overlapping with function key test in Section 05. Create a doc-only stub `tools_menu/scan_codes.rs`:
  ```rust
  //! tack's `t) tools -> m) scan codes` tool is interactive — it waits
  //! for user keystrokes and records what the terminal emits. Cannot
  //! be automated from the PTY test harness. Covered instead by
  //! Section 08's in-crate sibling test at
  //! `oriterm/src/key_encoding/terminfo_xcheck.rs`, which validates
  //! the same encode_key <-> terminfo correspondence.
  ```

- [ ] **Decompile terminfo (`x)`)** — tack runs `infocmp` internally and displays the result. Overlaps with Section 02's `ori_term_terminfo_round_trips_via_infocmp` unit test (which already verifies infocmp round-trips). Create a doc-only stub `tools_menu/decompile_terminfo.rs` documenting that the cross-check is handled by Section 02 tests directly, not through tack.

- [ ] Wire new modules into `oriterm_core/tests/tack/tools_menu/mod.rs`:
  ```rust
  pub mod character_sets;
  pub mod decompile_terminfo;  // doc-only stub
  pub mod enq_ack;
  pub mod osc_queries;
  pub mod scan_codes;          // doc-only stub
  pub mod sgr_modes;
  pub mod status_reports;
  ```

---

## 06.4 Determinism + parser unit tests

**File(s):** Sibling tests files for each module's parser

- [ ] `tools_menu/status_reports/tests.rs` covers `parse_status_reports` (already added in 06.1)
- [ ] `tools_menu/sgr_modes/tests.rs` (after restructuring `sgr_modes.rs` → `sgr_modes/mod.rs`):
  ```rust
  #[test]
  fn parse_sgr_modes_counts_unique_codes() {
      let grid = "SGR display\n0 1 4 7 22 24 27\n";
      let facts = super::parse_sgr_modes(grid);
      let count_note = facts.notes.iter()
          .find(|n| n.starts_with("sgr_codes_count="))
          .unwrap();
      assert_eq!(count_note, "sgr_codes_count=7");
  }

  #[test]
  fn parse_sgr_modes_ignores_codes_above_79() {
      let grid = "0 80 100 1\n";
      let facts = super::parse_sgr_modes(grid);
      assert_eq!(
          facts.notes[0],
          "sgr_codes_count=2"  // 0 and 1; 80 and 100 ignored
      );
  }
  ```
- [ ] `tools_menu/character_sets/tests.rs`:
  ```rust
  #[test]
  fn parse_character_sets_counts_box_drawing() {
      let grid = "DEC graphics\n┌─┐\n│ │\n└─┘\n";
      let facts = super::parse_character_sets(grid);
      let note = &facts.notes[0];
      assert!(note.starts_with("box_drawing_chars="));
      // 6 distinct chars: ┌ ─ ┐ │ └ ┘
      assert_eq!(note, "box_drawing_chars=6");
  }
  ```

- [ ] Determinism gate: 10 consecutive runs of `timeout 150 cargo test -p oriterm_core --test tack -- tools_menu` all pass. Any flake → `/add-bug` immediately.

- [ ] `--test-threads=1` and `--test-threads=4` both pass.

- [ ] Cross-compile gate: `cargo build --target x86_64-pc-windows-gnu -p oriterm_core --tests` succeeds.

---

## 06.R Third Party Review Findings

<!-- Reserved for Codex or other external reviewers. -->

- None.

---

## 06.N Completion Checklist

- [ ] `oriterm_core/tests/tack/tools_menu/status_reports/`: TACK_TOOLS_DA1/DA2/DSR (3 scenarios) all passing
- [ ] `oriterm_core/tests/tack/tools_menu/sgr_modes/`: TACK_TOOLS_SGR (1 scenario) passing
- [ ] `oriterm_core/tests/tack/tools_menu/character_sets/`: TACK_TOOLS_G0_DEC_GRAPHICS (1 scenario) passing
- [ ] `oriterm_core/tests/tack/tools_menu/enq_ack/`: TACK_TOOLS_ENQ_ACK (1 scenario) passing — cross-references Section 02's `u9` cap declaration
- [ ] `oriterm_core/tests/tack/tools_menu/osc_queries/`: TACK_TOOLS_OSC_QUERIES (1 scenario) passing — asserts OSC 10/11 responses present
- [ ] `oriterm_core/tests/tack/tools_menu/scan_codes.rs`: doc-only stub documenting the interactive exclusion
- [ ] `oriterm_core/tests/tack/tools_menu/decompile_terminfo.rs`: doc-only stub referencing Section 02's infocmp round-trip test
- [ ] At least 7 tools_menu scenarios total — combined with the 18 from Section 05, that's **25+ tack scenarios** across the catalog
- [ ] **Cap-coverage extension (cross-section sync from Section 05.5).** Section 06 owns `crates/oriterm_test_support/src/tack_framework/cap_coverage/section_06.rs`. Per Pivot 5 of Agent 3 of /review-plan, the cap-coverage matrix uses owner-partitioned `CapCoverageContribution` per section instead of a flat `EXEMPT_CAPS` constant. Section 06's task is: (a) move tools-menu caps Section 06 actually exercises (u6/u7/u8/u9, Cr/Cs, Ms, Smulx, Setulc, Sync, BD/BE/PS/PE, AX/XT, hs/dsl/fsl/tsl, Se/Ss, XF/kxIN/kxOUT, Tc, RGB, ENQ/ACK markers) FROM `section_06.rs::CONTRIBUTION.exempt` INTO `section_06.rs::CONTRIBUTION.covered` in lockstep with the scenario landing, (b) re-run `tack_cap_coverage_matrix` and confirm no stale exemptions (the negative pin fires on caps appearing in BOTH any section's `covered` AND any section's `exempt`), (c) update the doc comment at the top of `section_06.rs` to reflect that Section 06 has landed. <!-- reviewed: executability/hygiene fix -->
- [ ] Cross-validation against vttest menu6: the DA1/DA2/DSR responses captured here match the responses asserted by `oriterm_core/tests/vttest/menu6.rs`. Section 09 verification will diff them.
- [ ] Each parser has sibling-file unit tests
- [ ] All scenarios pass deterministically (10 reruns clean)
- [ ] Both `--test-threads=1` and `--test-threads=4` pass
- [ ] Cross-compile for `x86_64-pc-windows-gnu` succeeds
- [ ] No file in `tools_menu/` exceeds 500 lines
- [ ] `./build-all.sh` green
- [ ] `./clippy-all.sh` green
- [ ] `timeout 150 ./test-all.sh` green
- [ ] Plan annotation cleanup
- [ ] All TPR checkpoint findings resolved (see `06.R`)
- [ ] **Plan sync**:
  - [ ] Section frontmatter `status` → `complete`
  - [ ] `00-overview.md` Quick Reference table updated
  - [ ] `00-overview.md` Mission Success Criteria #8 ticked
  - [ ] `index.md` Section 06 updated
- [ ] `/tpr-review` final pass clean
- [ ] `/impl-hygiene-review last commit` final pass clean (after TPR)

**Exit Criteria:** `timeout 150 cargo test -p oriterm_core --test tack -- tools_menu` runs all 7 tools_menu scenarios (DA1, DA2, DSR, SGR sweep, G0 DEC graphics, ENQ/ACK, OSC queries) deterministically. Each parser has its own unit tests proving it extracts the right facts from synthesized grids. The captured DA/DSR responses match what `oriterm_core/tests/vttest/menu6.rs` already asserts. Section 06 closes with the entire tack scenario catalog at 25+ deterministic, semantically-asserted scenarios (Section 05's 18 test_menu + Section 06's 7 tools_menu).
