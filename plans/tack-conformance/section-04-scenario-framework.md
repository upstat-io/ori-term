---
section: "04"
title: "Scenario Catalog Framework"
status: not-started
reviewed: false
goal: "Build a structured scenario catalog framework inside `crates/oriterm_test_support` (NOT inside `oriterm_core/tests/`) so both text tests in `oriterm_core/tests/tack/` AND GPU golden tests in `oriterm/src/gpu/visual_regression/tack/` (Section 07) can consume the same `ScenarioSpec`/`TackNavigator`/`ScenarioRunner`. The framework lives in `oriterm_test_support::tack_framework` from the start — no later lift needed. Prevents the fragile regex-over-whole-grid antipattern by giving every scenario a structured outcome and per-scenario assertions."
success_criteria:
  - "`crates/oriterm_test_support/src/tack_framework/mod.rs` exists and exposes `ScenarioSpec`, `TackNavigator`, `ScenarioRunner`, `ScenarioOutcome` from the workspace-internal test-support crate"
  - "Re-exported via `oriterm_test_support::tack_framework::*` so both `oriterm_core/tests/tack/` (text scenarios in Sections 05-06) and `oriterm/src/gpu/visual_regression/tack/` (GPU goldens in Section 07) can import the same types"
  - "`ScenarioSpec { id, menu_path, ready_anchor, parser }` struct holds the semantic ID, navigation steps, readiness check, and per-scenario parser"
  - "`TackNavigator::navigate(session, &menu_path)` walks tack from the main menu through each step, calling `wait_for(prompt)` between every keystroke — no fixed sleeps"
  - "`ScenarioRunner::run(spec) -> ScenarioOutcome` ties it all together: spawn tack via spawn_tack, call TackNavigator, capture grid_text, call the per-scenario parser, return ScenarioOutcome with grid + parser-extracted facts"
  - "`ScenarioRunner::run_with_session_at(spec, cols, rows) -> LiveSession` returns a wrapper holding the live `PtySession` AND the `TerminfoEnv` (so it outlives the session) — used by Section 07 GPU goldens to render the live session through the GPU pipeline"
  - "One end-to-end scenario `tack_modes_am` (modes screen, autowrap-mode test) passes: navigates `[n] -> [m]` (begin testing -> modes), waits for the modes screen anchor, captures grid, asserts via insta snapshot AND a parser-extracted assertion (e.g., \"the modes screen contains the literal `am` capability label\")"
  - "ScenarioSpec is `Send` and constructible at module scope as a `const` or `static` (so test catalogs can list scenarios in arrays)"
  - "Framework gracefully handles tack going off-script: if `wait_for(prompt, timeout)` times out at any navigation step, panic with a clear error including the current grid AND the menu_path step that failed"
  - "`timeout 150 cargo test -p oriterm_core --test tack -- tack_modes_am` passes on Linux"
  - "Satisfies mission criteria: 'tack test scenarios cover modes/glitches/...' (foundation), 'Text snapshots (insta) exist for tack screens'"
inspired_by:
  - "ori_term teseq ScenarioSpec (plans/completed/teseq-conformance/section-01-infrastructure.md:95-156 — TerminalConfig + SetupConfig + ExpectConfig pattern)"
  - "ori_term vttest menu walking (oriterm_core/tests/vttest/menu6.rs::walk_menu6_subscreens — same `wait_for` + send-keystroke + drain pattern this framework formalizes)"
  - "Alacritty ref tests (alacritty_terminal/tests/ref.rs — scenario-directory + sidecar config + grid assertion)"
depends_on: ["03"]
third_party_review:
  status: none
  updated: null
sections:
  - id: "04.1"
    title: "ScenarioSpec, MenuStep, ScreenParser types"
    status: not-started
  - id: "04.2"
    title: "TackNavigator: walk menu_path with wait_for between steps"
    status: not-started
  - id: "04.3"
    title: "ScenarioRunner: spawn_tack + navigate + capture + parse"
    status: not-started
  - id: "04.4"
    title: "End-to-end scenario tack_modes_am"
    status: not-started
  - id: "04.R"
    title: "Third Party Review Findings"
    status: not-started
  - id: "04.N"
    title: "Completion Checklist"
    status: not-started
---

# Section 04: Scenario Catalog Framework

**Status:** Not Started
**Goal:** Replace ad-hoc tack-driving code with a structured catalog framework that lives in `crates/oriterm_test_support::tack_framework`. Each scenario is described once via `ScenarioSpec` (semantic ID, menu navigation steps, readiness anchor, per-scenario parser). `TackNavigator` walks the navigation steps through `PtySession`. `ScenarioRunner` ties spawn → navigate → capture → parse together. Sections 05-08 add scenarios to the catalog without re-implementing the navigation loop. The framework is finished and proven by one end-to-end scenario at the end of this section.

**Why `oriterm_test_support` and not `oriterm_core/tests/tack/framework/`:** Section 07 (GPU goldens) lives in `oriterm/src/gpu/visual_regression/tack/`. Integration test targets are isolated — `oriterm/src/` cannot import from `oriterm_core/tests/`. If the framework lived inside `oriterm_core`'s test target, Section 07 would have to either (a) lift it later or (b) duplicate the framework. We avoid both by placing it in the workspace-internal `oriterm_test_support` crate from the start. Both `oriterm_core/tests/tack/` (text scenarios) and `oriterm/src/gpu/visual_regression/tack/` (GPU goldens) can `use oriterm_test_support::tack_framework::*` directly.

**Success Criteria:**

- [ ] `crates/oriterm_test_support/src/tack_framework/mod.rs` exists and re-exports the framework types
- [ ] `ScenarioSpec { id: &'static str, menu_path: &'static [MenuStep], ready_anchor: &'static str, parser: ScreenParserFn }` is defined and constructible at module scope as `const`
- [ ] `MenuStep { send: &'static [u8], wait_for: &'static str }` describes a single navigation step (one keystroke + the prompt to wait for after)
- [ ] `TackNavigator::navigate(&mut PtySession, &[MenuStep])` walks the steps with `wait_for` between every send
- [ ] `ScenarioRunner::run(&ScenarioSpec) -> ScenarioOutcome` is the public entry point
- [ ] `ScenarioOutcome { id, grid_text, parsed: ScreenFacts }` carries the captured grid AND parser-extracted facts
- [ ] One end-to-end scenario `tack_modes_am` passes: navigates `n m`, waits for the modes screen, snapshots, asserts the parser found the `am` capability label
- [ ] `timeout 150 cargo test -p oriterm_core --test tack -- tack_modes_am` passes deterministically (10 consecutive runs)
- [ ] Satisfies mission criteria: scenario framework foundation; one end-to-end scenario proves the framework

**Context:** Without a structured framework, every tack scenario test would re-implement the same loop: spawn tack, send `n` for "begin testing", wait, send a letter for the sub-menu, wait, capture, assert. Across 20+ scenarios (Sections 05-06) that's 20+ copies of fragile navigation code. The fragility shows up two ways:
1. **Fixed sleeps**: developers add `thread::sleep(100)` between sends to "let tack settle". This races, especially in CI, and produces flaky tests.
2. **Regex over whole grid**: assertions like `assert!(grid.matches("modes").count() > 5)` are brittle and don't actually verify what they claim. A semantic assertion ("the parser found the `am` capability box") is testable in a way regex over text is not.

The framework solves both: `MenuStep::wait_for` is the deterministic synchronization primitive (replaces sleeps), and `ScreenParser` extracts structured facts from the grid (replaces regex). Section 04 builds the framework end-to-end and validates it with ONE scenario; Sections 05-08 add the rest of the catalog.

**Reference implementations:**
- **ori_term teseq** `plans/completed/teseq-conformance/section-01-infrastructure.md:95-156`: `ScenarioSpec`/`TerminalConfig`/`ExpectConfig` pattern. We adapt the structure for tack's navigation-driven model (different from teseq's byte-feeding model, but the spec-as-data approach is the same).
- **ori_term vttest** `oriterm_core/tests/vttest/menu6.rs::walk_menu6_subscreens(s, label, tag)`: existing example of `wait_for` + send-keystroke + drain pattern. The framework formalizes the same idea with `MenuStep` data instead of imperative `walk_*` functions.
- **Alacritty** `alacritty_terminal/tests/ref.rs`: scenario-directory pattern (each scenario is a directory with input + golden grid). We adopt scenario-AS-data (a `&'static [ScenarioSpec]` array) instead of scenario-AS-directory because tack scenarios share so much structure (same spawn, same end-of-menu-path navigation) that directories are noise.

**Depends on:** Section 03 (smoke test proves the spawn_tack pipeline works).

---

## 04.1 ScenarioSpec, MenuStep, ScreenParser types

**File(s):** `crates/oriterm_test_support/src/tack_framework/spec.rs`, `crates/oriterm_test_support/src/tack_framework/mod.rs`, `crates/oriterm_test_support/src/lib.rs`

The types are pure data — no I/O, no PtySession. They describe scenarios, not run them.

- [ ] Add `pub mod tack_framework;` to `crates/oriterm_test_support/src/lib.rs` next to `pub mod terminfo;` and `pub mod session;`. Add re-exports for the framework types so callers can `use oriterm_test_support::tack_framework::{...}` (preferred for explicit module path) or `use oriterm_test_support::{ScenarioSpec, ScenarioRunner, ...}` (re-exported at crate root for convenience):
  ```rust
  pub mod session;
  pub mod tack_framework;
  pub mod terminfo;

  pub use session::{PtyResponder, PtySession, /* ... */};
  pub use tack_framework::{
      MenuStep, ScenarioOutcome, ScenarioRunner, ScenarioSpec,
      ScreenFacts, ScreenParserFn, TackNavigator,
  };
  // `decode_terminfo_string` and `infocmp_query` are added by Section 08
  // (keyboard/function key tests) — NOT here. Section 04 only needs
  // `TerminfoEnv` from the terminfo module.
  pub use terminfo::TerminfoEnv;
  ```


- [ ] Create `crates/oriterm_test_support/src/tack_framework/mod.rs`:
  ```rust
  //! Scenario catalog framework for tack-driven conformance tests.
  //!
  //! See plans/tack-conformance/section-04-scenario-framework.md for the
  //! design rationale (semantic IDs, menu navigation as data, deterministic
  //! wait_for synchronization, per-scenario parsers).

  pub mod navigator;
  pub mod parser;
  pub mod runner;
  pub mod spec;

  pub use navigator::TackNavigator;
  pub use parser::{ScreenFacts, ScreenParserFn, default_parser};
  pub use runner::{ScenarioOutcome, ScenarioRunner};
  pub use spec::{MenuStep, ScenarioSpec};
  ```

- [ ] Create `crates/oriterm_test_support/src/tack_framework/spec.rs`:
  ```rust
  use super::parser::ScreenParserFn;

  /// A single navigation step: send these bytes, then wait until the
  /// PTY grid contains this anchor string.
  ///
  /// `wait_for` is the deterministic synchronization primitive — it
  /// replaces fixed sleeps that race in CI. The anchor is a literal
  /// substring expected in the grid AFTER tack processes `send`.
  ///
  /// Example:
  ///   MenuStep { send: b"n", wait_for: "begin testing" }
  /// — sends 'n' (the main menu choice for begin-testing) and waits
  /// until the begin-testing submenu contains the literal "begin
  /// testing" (the submenu header).
  #[derive(Copy, Clone, Debug)]
  pub struct MenuStep {
      pub send: &'static [u8],
      pub wait_for: &'static str,
  }

  /// Static description of a single tack scenario.
  ///
  /// Constructible as `const` so test catalogs can list scenarios in
  /// arrays. The whole spec is data — no closures, no I/O — until the
  /// `parser` function pointer is invoked by `ScenarioRunner`.
  #[derive(Copy, Clone, Debug)]
  pub struct ScenarioSpec {
      /// Semantic ID, e.g. `"tack_modes_am"`. Used as the insta
      /// snapshot name and as the test function name (one wrapper
      /// `#[test] fn tack_modes_am() { run_scenario(&MODES_AM) }`).
      ///
      /// Convention: `tack_<menu>_<screen>` lowercase snake_case.
      pub id: &'static str,

      /// Sequence of navigation steps from tack's main menu to the
      /// target screen. Each step sends one or more bytes and waits
      /// for an anchor string to appear in the grid.
      ///
      /// Example for the modes screen (n -> m):
      ///   &[
      ///     MenuStep { send: b"n", wait_for: "begin testing" },
      ///     MenuStep { send: b"m", wait_for: "modes" },
      ///   ]
      pub menu_path: &'static [MenuStep],

      /// Final readiness anchor. After the last `MenuStep` lands, the
      /// runner calls `session.wait_for(ready_anchor, ...)` once more
      /// to make sure the screen has fully painted before grid_text
      /// is captured.
      pub ready_anchor: &'static str,

      /// Per-scenario screen parser. Takes the captured grid_text and
      /// extracts structured facts (which capability labels are
      /// present, what the cursor reports look like, etc.). The
      /// returned `ScreenFacts` is asserted by the test.
      pub parser: ScreenParserFn,
  }

  impl ScenarioSpec {
      /// Convenience constructor for tests that just snapshot and
      /// don't need a custom parser.
      #[must_use]
      pub const fn snapshot_only(
          id: &'static str,
          menu_path: &'static [MenuStep],
          ready_anchor: &'static str,
      ) -> Self {
          Self { id, menu_path, ready_anchor, parser: super::parser::default_parser }
      }
  }
  ```

- [ ] Create `crates/oriterm_test_support/src/tack_framework/parser.rs`:
  ```rust
  /// Structured facts extracted from a tack screen by a per-scenario
  /// parser. The default parser populates only `header_text`; custom
  /// parsers populate the typed fields they care about.
  #[derive(Clone, Debug, Default, PartialEq, Eq)]
  pub struct ScreenFacts {
      /// First non-blank line of the captured grid — the screen
      /// header. e.g. "modes", "ACS graphic rendition", "color".
      pub header_text: String,

      /// Capability labels found on the screen (for modes/glitches and
      /// SGR test screens that show literal cap names like `am`,
      /// `bce`, `bw`).
      pub capability_labels: Vec<String>,

      /// Free-form notes the parser wants to record. Snapshotted as
      /// part of the outcome but not asserted automatically.
      pub notes: Vec<String>,
  }

  /// Function pointer type for per-scenario screen parsers.
  ///
  /// Function pointer (not closure) so `ScenarioSpec` can be `Copy`
  /// and `const`-constructible.
  pub type ScreenParserFn = fn(&str) -> ScreenFacts;

  /// Default parser: extracts the first non-blank line as
  /// `header_text` and leaves all other fields empty. Suitable for
  /// snapshot-only scenarios that don't need typed assertions.
  #[must_use]
  pub fn default_parser(grid: &str) -> ScreenFacts {
      let header = grid
          .lines()
          .map(str::trim)
          .find(|line| !line.is_empty())
          .unwrap_or("")
          .to_string();
      ScreenFacts { header_text: header, ..ScreenFacts::default() }
  }
  ```

  **Why function pointers, not closures:** `ScenarioSpec` must be `const`-constructible at module scope so a `const SCENARIOS: &[ScenarioSpec] = &[...]` array works. Closures capture state and aren't `const`. Function pointers are. The trade-off: per-scenario parsers can't close over local config — they have to be plain `fn(&str) -> ScreenFacts`. Sections 05-06 will define one named parser fn per scenario family (e.g., `parse_modes_screen`, `parse_color_screen`).

- [ ] Add unit tests at `crates/oriterm_test_support/src/tack_framework/parser/tests.rs` (sibling tests file — restructure `parser.rs` → `parser/mod.rs` to fit the convention):
  ```rust
  use super::{default_parser, ScreenFacts};

  #[test]
  fn default_parser_extracts_first_non_blank_line_as_header() {
      let grid = "\n\nMain Menu\n b) basic\n";
      let facts = default_parser(grid);
      assert_eq!(facts.header_text, "Main Menu");
      assert!(facts.capability_labels.is_empty());
      assert!(facts.notes.is_empty());
  }

  #[test]
  fn default_parser_handles_empty_grid() {
      let facts = default_parser("");
      assert_eq!(facts.header_text, "");
  }

  #[test]
  fn default_parser_handles_all_blank_grid() {
      let facts = default_parser("\n\n   \n  \n");
      assert_eq!(facts.header_text, "");
  }
  ```

---

## 04.2 TackNavigator: walk menu_path with wait_for between steps

**File(s):** `crates/oriterm_test_support/src/tack_framework/navigator.rs`

`TackNavigator` is the imperative half of the framework — it takes a `&mut PtySession` and a `&[MenuStep]` and walks them. Failure handling is the only non-trivial part: when a `wait_for` times out, the panic message must include the menu_path step index AND the current grid so the failure tells the developer exactly where tack went off-script.

- [ ] Create `crates/oriterm_test_support/src/tack_framework/navigator.rs`:
  ```rust
  use crate::session::PtySession;

  use super::spec::MenuStep;

  /// Walks a `&[MenuStep]` against a live `PtySession` running tack.
  ///
  /// Each step is `send → wait_for`, with no fixed sleeps anywhere.
  /// On wait_for timeout, the navigator panics with a message that
  /// includes the failing step index, the bytes sent, the anchor it
  /// was waiting for, and the current grid contents.
  pub struct TackNavigator;

  impl TackNavigator {
      /// Walk `steps` against `session`. Panics on any wait_for
      /// timeout — see the panic message format in the body.
      pub fn navigate(session: &mut PtySession, steps: &[MenuStep]) {
          for (idx, step) in steps.iter().enumerate() {
              session.send(step.send);
              // Use a wait timeout long enough for tack to redraw a
              // submenu (CI-safe: 5s). Bump only if observed flakes.
              navigate_step(session, step, idx);
          }
      }
  }

  fn navigate_step(session: &mut PtySession, step: &MenuStep, idx: usize) {
      // Use a manual loop instead of session.wait_for so we can
      // produce a richer panic message including the step index.
      const STEP_TIMEOUT_MS: u64 = 5_000;
      let deadline = std::time::Instant::now() + std::time::Duration::from_millis(STEP_TIMEOUT_MS);
      loop {
          session.drain();
          let grid = session.grid_text();
          if grid.contains(step.wait_for) {
              // Drain any trailing output to make sure the screen
              // has fully painted before navigate() returns.
              session.wait(200);
              return;
          }
          if std::time::Instant::now() >= deadline {
              panic!(
                  "TackNavigator: step {idx} timed out after {STEP_TIMEOUT_MS}ms.\n\
                   Sent: {send_repr:?}\n\
                   Waiting for: {anchor:?}\n\
                   Current grid:\n{grid}",
                  send_repr = String::from_utf8_lossy(step.send),
                  anchor = step.wait_for,
                  grid = grid,
              );
          }
          // Block for up to 100ms waiting for new PTY data, then loop.
          session.drain_blocking(100);
      }
  }
  ```

- [ ] Add unit tests at `crates/oriterm_test_support/src/tack_framework/navigator/tests.rs` (restructure `navigator.rs` → `navigator/mod.rs`). These tests use a fake `PtySession` — but `PtySession` owns a real PTY child, so a true unit test isn't trivial. Two options:
  - **(a)** Skip unit tests on `TackNavigator` entirely. Cover it only via the end-to-end test in 04.4. Acceptable because the navigator is small and its behavior is exercised every time a scenario runs.
  - **(b)** Add a `#[cfg(test)] pub trait Session` abstraction so `TackNavigator::navigate` can take a test double. More flexible but adds an abstraction layer for one test.

  Recommendation: option (a). The end-to-end scenario in 04.4 IS the navigator test. Re-running 04.4 ten times is the regression check.

  Note: a single unit test for the panic message format is still useful. Test it via a synthesized `wait_for` failure in a small focused test that uses a real `PtySession` running `cat` (so the "grid" is empty and `wait_for("anchor", short_timeout)` is guaranteed to fail). This proves the panic message format is what we expect:
  ```rust
  #[test]
  #[should_panic(expected = "TackNavigator: step 0 timed out")]
  fn navigator_panics_with_step_index_on_timeout() {
      use oriterm_test_support::{PtySession, tool_available};
      use portable_pty::CommandBuilder;
      if !tool_available("cat", "--help") { return; }
      let mut session = PtySession::spawn(CommandBuilder::new("cat"), 80, 24);
      let steps = &[MenuStep { send: b"", wait_for: "this_text_will_never_appear" }];
      TackNavigator::navigate(&mut session, steps);
  }
  ```
  Note: the test must skip on Windows (no `cat`). Wrap in `if !tool_available("cat", "--help") { return; }`. Yes, this is an integration-style test even though it's a unit test — that's fine; it lives in `framework/navigator/tests.rs` next to the production code.

---

## 04.3 ScenarioRunner: spawn_tack + navigate + capture + parse

**File(s):** `crates/oriterm_test_support/src/tack_framework/runner.rs`

`ScenarioRunner` is the public entry point Sections 05-08 use. Given a `&ScenarioSpec`, it spawns tack, navigates, captures, parses, and returns a `ScenarioOutcome`. Tests then run an `assert!(outcome.parsed.capability_labels.contains(&"am".to_string()))` style check, plus `insta::assert_snapshot!(outcome.id, &outcome.grid_text)`.

- [ ] Create `crates/oriterm_test_support/src/tack_framework/runner.rs`:
  ```rust
  use crate::session::PtySession;
  use crate::session::{tack_available, tic_available};
  use crate::terminfo::TerminfoEnv;

  use super::navigator::TackNavigator;
  use super::parser::ScreenFacts;
  use super::spec::ScenarioSpec;

  /// The result of running one scenario: the captured grid text and
  /// the per-scenario parser's typed extraction.
  #[derive(Clone, Debug)]
  pub struct ScenarioOutcome {
      pub id: &'static str,
      pub grid_text: String,
      pub parsed: ScreenFacts,
  }

  pub struct ScenarioRunner;

  impl ScenarioRunner {
      /// Returns true iff both `tack` and `tic` are available — call
      /// at the top of every test that runs scenarios so the test
      /// skips cleanly when the tools are missing.
      #[must_use]
      pub fn available() -> bool {
          tack_available() && tic_available()
      }

      /// Run a single scenario at the standard 80x24 size.
      ///
      /// Spawns tack via `PtySession::spawn_tack` against a fresh
      /// `TerminfoEnv`, navigates the menu_path, calls the parser,
      /// and quits tack cleanly via `q\n` before returning.
      ///
      /// Panics on navigation timeout (via `TackNavigator::navigate`)
      /// — the panic message identifies the failing step.
      #[must_use]
      pub fn run(spec: &ScenarioSpec) -> ScenarioOutcome {
          Self::run_at(spec, 80, 24)
      }

      /// Run a scenario at a specific grid size. Used by Sections
      /// 05-08 for size-matrix tests.
      #[must_use]
      pub fn run_at(spec: &ScenarioSpec, cols: u16, rows: u16) -> ScenarioOutcome {
          let env = TerminfoEnv::compile();
          let mut session = PtySession::spawn_tack(&env, cols, rows);

          // Wait for the main menu prompt before navigating.
          session.wait_for("tack [n] >", 5_000);

          TackNavigator::navigate(&mut session, spec.menu_path);
          session.wait_for(spec.ready_anchor, 5_000);

          let grid_text = session.grid_text();
          let parsed = (spec.parser)(&grid_text);

          // Quit tack cleanly so the child reaps quickly.
          // 'q' may need to be sent multiple times to back out of
          // submenus — three q's covers any nesting depth tack uses.
          session.send(b"q\n");
          session.send(b"q\n");
          session.send(b"q\n");
          session.wait(500);

          ScenarioOutcome { id: spec.id, grid_text, parsed }
      }
  }
  ```

  **Quitting depth:** the comment says "three q's covers any nesting depth." This is a guess based on tack's main-menu / submenu / sub-submenu structure. If observed runs leave zombies because `q\n` doesn't exit cleanly, raise it or check the exit status. Section 05 will exercise this — fix on first observed flake.

- [ ] Add `LiveSession` wrapper and `run_with_session_at` for Section 07's GPU goldens (defined here so the framework is feature-complete in one place):
  ```rust
  /// Wrapper that returns a LIVE PtySession instead of just text.
  /// Used by Section 07 GPU goldens to render the live session
  /// through the GPU pipeline before quitting.
  ///
  /// The `_terminfo` field is intentionally unused at the call site
  /// — its only job is to outlive the session, because tack reads
  /// terminfo lazily during screen redraws and dropping the
  /// TerminfoEnv before the session would race with tack's reads.
  pub struct LiveSession {
      pub session: PtySession,
      pub facts: ScreenFacts,
      _terminfo: TerminfoEnv,
  }

  impl ScenarioRunner {
      /// Like `run_at` but returns the live `PtySession` so GPU
      /// callers can render it through the pipeline before quitting.
      ///
      /// Caller is responsible for cleanly quitting tack via
      /// `live.session.send(b"q\nq\nq\n")`. Drop on `LiveSession`
      /// reaps the child process and the temp terminfo dir.
      #[must_use]
      pub fn run_with_session_at(
          spec: &ScenarioSpec,
          cols: u16,
          rows: u16,
      ) -> LiveSession {
          let env = TerminfoEnv::compile();
          let mut session = PtySession::spawn_tack(&env, cols, rows);
          session.wait_for("tack [n] >", 5_000);
          TackNavigator::navigate(&mut session, spec.menu_path);
          session.wait_for(spec.ready_anchor, 5_000);
          let grid_text = session.grid_text();
          let facts = (spec.parser)(&grid_text);
          LiveSession { session, facts, _terminfo: env }
      }
  }
  ```
  Update the `pub use` re-exports in `tack_framework/mod.rs` to include `LiveSession`.

- [ ] No unit test for `ScenarioRunner` itself — it's exercised by 04.4's end-to-end test.

- [ ] **Snapshot collision policy (must be documented in the framework):** multiple scenarios often visit the SAME tack screen and produce an identical grid (e.g. all seven `tack_modes_*` scenarios navigate `[n] [m]` and produce the same modes screen, asserting different `capability_labels`). If every scenario called `insta::assert_snapshot!(outcome.id, grid)`, the insta store would hold N identical `.snap` files — wasteful and confusing.

  The framework encodes the policy in a doc comment at the top of `ScenarioRunner`:
  ```rust
  /// # Snapshot policy for duplicate-screen scenarios
  ///
  /// When multiple scenarios visit the SAME tack screen (e.g. seven
  /// `tack_modes_*` variants that all navigate `[n] [m]`), ONE
  /// designated scenario owns the insta snapshot for that screen.
  /// The rest call `ScenarioRunner::run` and assert on
  /// `outcome.parsed` only — skipping `insta::assert_snapshot!`.
  ///
  /// Convention: the FIRST scenario in alphabetical order
  /// (e.g. `tack_modes_am`) owns the snapshot for its family. The
  /// rest are parser-only. See `oriterm_core/tests/tack/test_menu/modes.rs`
  /// for the canonical example.
  ```



---

## 04.4 End-to-end scenario tack_modes_am

**File(s):** `oriterm_core/tests/tack/main.rs` (add `mod test_menu;`), `oriterm_core/tests/tack/test_menu/mod.rs` (NEW), `oriterm_core/tests/tack/test_menu/modes.rs` (NEW — test wrapper only), `crates/oriterm_test_support/src/tack_framework/scenarios/modes.rs` (NEW — const + parser)

The first real scenario. It validates the entire framework from top to bottom: spawn tack, walk `[n] [m]` (begin testing → modes), wait for the modes screen anchor, capture, parse for the literal capability label `am` (autowrap mode), assert via insta snapshot AND the parser extraction. If this passes, every other scenario in Sections 05-06 follows the same shape.

- [ ] In `oriterm_core/tests/tack/main.rs`, the framework is imported from the workspace crate (NOT a local `mod framework;`). Add the test_menu module declaration:
  ```rust
  // The framework lives in oriterm_test_support — no `mod framework;`
  // here. Test files import via `use oriterm_test_support::tack_framework::*`.
  mod test_menu;
  ```

- [ ] Create `oriterm_core/tests/tack/test_menu/mod.rs`:
  ```rust
  //! Tack `n) begin testing` submenu scenarios.
  //!
  //! Section 04 introduces the first scenario (`modes::am`). Section 05
  //! adds the rest of the test menu catalog (modes/glitches, ACS,
  //! color, cursor movement).

  pub mod modes;
  ```

- [ ] Create `crates/oriterm_test_support/src/tack_framework/scenarios/modes.rs` (the CONST + parser, in the workspace crate):
  ```rust
  //! Modes/glitches scenario consts and parser for the tack
  //! `n) begin testing -> m) modes` sub-menu.
  //!
  //! Defines pub const ScenarioSpec values that both text tests
  //! (oriterm_core/tests/tack/test_menu/modes.rs) and GPU tests
  //! (oriterm/src/gpu/visual_regression/tack/mod.rs in Section 07)
  //! reference. Single source of truth for "how do you reach the
  //! modes screen and what does the parser extract."

  use crate::tack_framework::{MenuStep, ScenarioSpec, ScreenFacts};

  /// Scenario: navigate to the modes screen and verify it lists `am`.
  pub const TACK_MODES_AM: ScenarioSpec = ScenarioSpec {
      id: "tack_modes_am",
      menu_path: &[
          MenuStep { send: b"n", wait_for: "begin testing" },
          MenuStep { send: b"m", wait_for: "modes" },
      ],
      // After `m`, tack draws the modes screen. The exact ready
      // anchor is observed empirically — Section 03's smoke test
      // captures tack's submenu wording. Update this anchor after
      // running the test once with INSTA_UPDATE=1 and inspecting
      // the captured grid for the actual screen header.
      ready_anchor: "modes",
      parser: parse_modes_screen,
  };

  /// Custom parser for the modes screen: scans the grid for known
  /// capability labels and populates `capability_labels`.
  fn parse_modes_screen(grid: &str) -> ScreenFacts {
      // Known mode capabilities tested by tack's modes screen.
      // Source: ncurses tack source / man page. We list the ones
      // ori_term's terminfo declares in extra/ori_term.info.
      const KNOWN: &[&str] = &[
          "am", "bce", "bw", "km", "mir", "msgr", "xenl",
      ];

      let mut labels = Vec::new();
      for cap in KNOWN {
          // Match the cap name as a whole-word grep — surrounded by
          // non-word characters or grid edges. Tack draws labels
          // with surrounding whitespace, so a `contains` check is
          // sufficient as long as the cap names are unique.
          if grid.contains(cap) {
              labels.push((*cap).to_string());
          }
      }

      // Header is the first non-blank line for snapshot stability.
      let header = grid
          .lines()
          .map(str::trim)
          .find(|line| !line.is_empty())
          .unwrap_or("")
          .to_string();

      ScreenFacts {
          header_text: header,
          capability_labels: labels,
          notes: Vec::new(),
      }
  }
  ```

  Note: `parse_modes_screen` is `pub` so the test wrapper file (next bullet) can import it. As a function pointer used by `const ScenarioSpec`, it must also be a plain `fn` (no closures) — see Section 04.1's parser type discussion.

- [ ] Create `oriterm_core/tests/tack/test_menu/modes.rs` (the test wrapper, in the integration test target):
  ```rust
  //! Test wrappers for the modes scenarios. Const ScenarioSpecs and
  //! parsers live in oriterm_test_support::tack_framework::scenarios::modes.
  //! This file just defines `#[test] fn` wrappers that invoke
  //! ScenarioRunner against those consts.

  use oriterm_test_support::tack_framework::ScenarioRunner;
  use oriterm_test_support::tack_framework::scenarios::modes::TACK_MODES_AM;

  #[test]
  fn tack_modes_am() {
      if !ScenarioRunner::available() {
          eprintln!("tack or tic not installed, skipping tack_modes_am");
          return;
      }

      let outcome = ScenarioRunner::run(&TACK_MODES_AM);

      // Programmatic semantic assertion: the parser found `am` in
      // the modes screen capability list.
      assert!(
          outcome.parsed.capability_labels.iter().any(|c| c == "am"),
          "expected `am` in capability_labels, got {:?}\nGrid:\n{}",
          outcome.parsed.capability_labels,
          outcome.grid_text,
      );

      // Insta snapshot of the full grid for visual regression catching.
      insta::assert_snapshot!(outcome.id, outcome.grid_text);
  }
  ```

- [ ] Run: `INSTA_UPDATE=1 timeout 150 cargo test -p oriterm_core --test tack -- tack_modes_am`. First run creates the snapshot.
- [ ] Inspect the captured snapshot at `oriterm_core/tests/tack/snapshots/tack__test_menu__modes__tack_modes_am.snap` (or similar — insta's path convention follows module hierarchy). Verify it shows the modes screen and that `am` is visible in the capability list. If the screen header is something other than "modes" (e.g., "Test modes and glitches"), update `ready_anchor` to match the actual header text.
- [ ] Re-run: `timeout 150 cargo test -p oriterm_core --test tack -- tack_modes_am`. Must PASS deterministically.
- [ ] Run 10 times in a row. All must pass.
- [ ] **TPR checkpoint** — `/tpr-review` covering 04.1–04.4 (the entire framework). Catches: races between `wait_for` and tack's screen rendering, brittle parser logic, scenario IDs that drift from snapshot file names, missing `q\n` quit cleanups.

---

## 04.R Third Party Review Findings

<!-- Reserved for Codex or other external reviewers. -->

- None.

---

## 04.N Completion Checklist

- [ ] `crates/oriterm_test_support/src/tack_framework/mod.rs` exists with all expected re-exports
- [ ] `crates/oriterm_test_support/src/tack_framework/spec.rs` defines `MenuStep`, `ScenarioSpec`, `ScenarioSpec::snapshot_only` const constructor
- [ ] `crates/oriterm_test_support/src/tack_framework/parser/mod.rs` defines `ScreenFacts`, `ScreenParserFn`, `default_parser`
- [ ] `crates/oriterm_test_support/src/tack_framework/parser/tests.rs` (sibling tests) covers `default_parser` happy path, empty grid, all-blank grid
- [ ] `crates/oriterm_test_support/src/tack_framework/navigator/mod.rs` defines `TackNavigator` with rich panic messages on timeout
- [ ] `crates/oriterm_test_support/src/tack_framework/navigator/tests.rs` includes the `should_panic` test for the timeout error format
- [ ] `crates/oriterm_test_support/src/tack_framework/runner.rs` defines `ScenarioRunner::run()`, `run_at(cols, rows)`, `run_with_session_at(cols, rows)`, `available()`, `ScenarioOutcome`, and `LiveSession`
- [ ] `crates/oriterm_test_support/src/lib.rs` declares `pub mod tack_framework;` and re-exports the framework types at crate root
- [ ] `oriterm_core/tests/tack/main.rs` does NOT contain `mod framework;` — it imports from `oriterm_test_support::tack_framework::*`
- [ ] `crates/oriterm_test_support/src/tack_framework/scenarios/modes.rs` defines `pub const TACK_MODES_AM: ScenarioSpec` and `pub fn parse_modes_screen` parser
- [ ] `oriterm_core/tests/tack/test_menu/modes.rs` defines the `#[test] fn tack_modes_am` wrapper that imports `TACK_MODES_AM` from the workspace crate
- [ ] `tack_modes_am` test passes — `am` capability label found, insta snapshot committed
- [ ] 10 consecutive runs of `tack_modes_am` all pass (determinism check)
- [ ] No file in `crates/oriterm_test_support/src/tack_framework/` exceeds 500 lines
- [ ] `cargo build --target x86_64-pc-windows-gnu -p oriterm_core --tests` succeeds (cross-compile gate)
- [ ] `./build-all.sh` green
- [ ] `./clippy-all.sh` green
- [ ] `timeout 150 ./test-all.sh` green
- [ ] Plan annotation cleanup: no temporary scaffolding in `.rs` files
- [ ] All TPR checkpoint findings resolved (see `04.R`)
- [ ] **Plan sync**:
  - [ ] This section's frontmatter `status` → `complete`
  - [ ] `00-overview.md` Quick Reference table: Section 04 marked Complete
  - [ ] `index.md` Section 04 status updated
  - [ ] Section 05's `depends_on: ["04"]` confirmed (Section 05 builds the test_menu scenario catalog on top of `ScenarioRunner`)
  - [ ] Section 06's `depends_on: ["04"]` confirmed (Section 06 builds the tools_menu scenario catalog on top of the same framework)
- [ ] `/tpr-review` final pass clean
- [ ] `/impl-hygiene-review last commit` final pass clean (after TPR)

**Exit Criteria:** `crates/oriterm_test_support/src/tack_framework/` contains the four-file framework (`spec`, `parser`, `navigator`, `runner`) re-exported through `mod.rs`. `tack_modes_am` passes deterministically: `timeout 150 cargo test -p oriterm_core --test tack -- tack_modes_am` returns success in <15s, the parser found `am` in the capability list, and the insta snapshot is committed. The framework is ready for Sections 05-08 to add scenarios without re-implementing navigation or capture loops. Both text tests (`oriterm_core/tests/tack/`) and GPU tests (`oriterm/src/gpu/visual_regression/tack/`) consume `oriterm_test_support::tack_framework::*` directly — no later refactor needed.
