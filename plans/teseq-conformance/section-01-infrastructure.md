---
section: "01"
title: "TeseqHarness & Infrastructure"
status: complete
reviewed: true
goal: "Build the complete teseq test harness: scenario loading, reseq subprocess integration, structured event capture, Term feeding, and snapshot-based assertions"
success_criteria:
  - "TeseqHarness compiles and runs a single smoke test scenario end-to-end"
  - "RecordedEvent enum captures all Event variants with structured data (not Debug strings)"
  - "reseq subprocess converts .teseq files to raw bytes; graceful skip when unavailable"
  - "Sidecar TOML parsing supports terminal config, pre_feed, and assertion expectations"
  - "insta snapshots capture grid state and events for golden comparison"
  - "timeout 150 cargo test -p oriterm_core --test teseq passes with smoke scenario"
  - "Satisfies mission criteria: TeseqHarness exists, RecordedEvent enum, sidecar TOML, reseq subprocess"
inspired_by:
  - "ori_term VtTestSession (oriterm_core/tests/vttest/session.rs) — PTY session + grid extraction + insta snapshots"
  - "ori_term RecordingListener (oriterm_core/src/term/handler/tests.rs:15-40) — event capture pattern"
  - "Alacritty ref tests (alacritty_terminal/tests/ref.rs) — scenario directory + sidecar config + grid assertion"
depends_on: []
third_party_review:
  status: resolved
  updated: 2026-04-05
sections:
  - id: "01.1"
    title: "ScenarioSpec & TOML Sidecar Parser"
    status: complete
  - id: "01.2"
    title: "RecordedEvent Enum & RecordedListener"
    status: complete
  - id: "01.3"
    title: "Reseq Subprocess Adapter"
    status: complete
  - id: "01.4"
    title: "TeseqHarness Runner"
    status: complete
  - id: "01.5"
    title: "Assertion Helpers & Snapshot Infrastructure"
    status: complete
  - id: "01.6"
    title: "Smoke Test & Directory Structure"
    status: complete
  - id: "01.R"
    title: "Third Party Review Findings"
    status: complete
  - id: "01.N"
    title: "Completion Checklist"
    status: complete
---

# Section 01: TeseqHarness & Infrastructure

**Status:** Complete
**Goal:** Build the complete test harness from scenario loading through assertion checking. When this section is complete, a single `.teseq` scenario file can be loaded, compiled to bytes via `reseq`, fed through `Term<RecordedListener>`, and validated against insta golden snapshots — all with a single function call from a `#[test]` function.

**Success Criteria:**

- [x] `TeseqHarness::from_scenario(path)` loads a `.teseq` file + optional `.toml` sidecar
- [x] `RecordedEvent` enum captures all `Event` variants with structured payload data
- [x] `reseq` subprocess converts `.teseq` to raw bytes; `reseq_available()` returns false gracefully
- [x] `grid_text()`, `cursor_position()`, `events()`, `pty_writes()` inspection methods work
- [x] `insta::assert_snapshot!()` integration works for grid state and event sequences
- [x] One smoke scenario (`scenarios/c0/bel.teseq`) passes end-to-end
- [x] Satisfies mission criteria: TeseqHarness exists with all sub-components

**Context:** The existing handler unit tests use `feed()` + `RecordingListener` (capturing `format!("{event:?}")` strings) for per-sequence validation. The existing vttest tests use `VtTestSession` with `PtyResponder` for black-box PTY testing. The teseq harness occupies a middle ground: synchronous byte feeding (like handler tests) with scenario-file-driven input (like vttest), plus structured event capture that surpasses both existing approaches.

**Reference implementations:**
- **ori_term** `oriterm_core/tests/vttest/session.rs`: `VtTestSession` pattern — PTY management, `grid_text()` via `renderable_content()`, `PtyResponder` for event capture, insta snapshots with size-label naming.
- **ori_term** `oriterm_core/src/term/handler/tests.rs:15-65`: `RecordingListener`, `term_with_recorder()`, `feed()` — the synchronous byte-feed + event-capture pattern this harness extends.
- **Alacritty** `alacritty_terminal/tests/ref.rs`: Scenario directories with `size.json` + `config.json` sidecars, `alacritty.recording` binary input, `grid.json` golden grid state.

**Depends on:** None (foundation section).

---

## 01.1 ScenarioSpec & TOML Sidecar Parser

**File(s):** `oriterm_core/tests/teseq/harness/loader.rs`

The loader discovers scenario files and parses optional TOML sidecars into a `ScenarioSpec` struct that drives the runner.

- [x] Create directory structure:
  ```
  oriterm_core/tests/teseq/
  ├── main.rs
  └── harness/
      ├── mod.rs
      ├── loader.rs
      ├── runner.rs
      ├── assertions.rs
      ├── reseq.rs
      └── events.rs
  ```

- [x] Define `ScenarioSpec` in `loader.rs`:
  ```rust
  /// Configuration for a single teseq test scenario.
  #[derive(Debug, Deserialize)]
  #[serde(default)]
  pub struct ScenarioSpec {
      pub terminal: TerminalConfig,
      pub setup: SetupConfig,
      pub expect: ExpectConfig,
  }

  #[derive(Debug, Deserialize)]
  #[serde(default)]
  pub struct TerminalConfig {
      pub cols: usize,       // default: 80
      pub rows: usize,       // default: 24
      pub scrollback: usize, // default: 0
      pub theme: String,     // default: "dark"
  }

  #[derive(Debug, Deserialize)]
  #[serde(default)]
  pub struct SetupConfig {
      /// Raw escape sequences to feed before the scenario.
      pub pre_feed: Vec<String>,
  }

  #[derive(Debug, Deserialize)]
  #[serde(default)]
  pub struct ExpectConfig {
      pub grid_snapshot: bool,   // default: true
      pub event_snapshot: bool,  // default: true
      pub cursor: Option<CursorExpect>,
      pub events: Vec<String>,   // expected event type names
  }

  #[derive(Debug, Deserialize)]
  pub struct CursorExpect {
      pub col: usize,
      pub line: usize,
  }
  ```

- [x] Implement `ScenarioSpec::load(teseq_path: &Path) -> ScenarioSpec`:
  - Look for a sibling `.toml` file (e.g., `bel.teseq` → `bel.toml`)
  - If `.toml` exists, parse it with `toml::from_str()`
  - If `.toml` doesn't exist, return `ScenarioSpec::default()` (80x24, dark theme, no pre-feed)
  - All fields use `#[serde(default)]` so partial TOMLs work

- [x] Add `toml` and `serde` to `oriterm_core`'s dev-dependencies in `Cargo.toml`:
  ```toml
  [dev-dependencies]
  toml = "0.8"
  serde = { version = "1", features = ["derive"] }
  ```
  Note: `insta = "1"` is already present in dev-dependencies (used by vttest). Verify it's still there.

- [x] Implement `discover_scenarios(family_dir: &Path) -> Vec<PathBuf>`:
  - Walk `family_dir` for `*.teseq` files (non-recursive within family)
  - Sort deterministically by filename
  - Return sorted list of `.teseq` file paths

---

## 01.2 RecordedEvent Enum & RecordedListener

**File(s):** `oriterm_core/tests/teseq/harness/events.rs`

Structured event capture that preserves payload data, unlike the existing `RecordingListener` which formats events to `Debug` strings.

- [x] Define `RecordedEvent` enum mirroring `oriterm_core::event::Event` but with `Clone + Debug + PartialEq`:
  ```rust
  /// Structured event capture for test assertions.
  ///
  /// Mirrors `oriterm_core::event::Event` but replaces closures with
  /// their identifying data, enabling equality comparison and snapshots.
  #[derive(Clone, Debug, PartialEq)]
  pub enum RecordedEvent {
      Wakeup,
      Bell,
      Title(String),
      ResetTitle,
      IconName(String),
      ResetIconName,
      ClipboardStore(ClipboardType, String),
      ClipboardLoad(ClipboardType),         // closure stripped
      ColorRequest(usize),                  // closure stripped
      PtyWrite(String),                     // verbatim response bytes
      CursorBlinkingChange,
      Cwd(String),                          // emitted by mux RawInterceptor, not Term<T>
      CommandComplete,                      // duration stripped (non-deterministic)
      MouseCursorDirty,
      ChildExit(i32),
  }
  ```

- [x] Implement `From<&Event> for RecordedEvent` to convert live events:
  ```rust
  impl From<&Event> for RecordedEvent {
      fn from(event: &Event) -> Self {
          match event {
              Event::Wakeup => Self::Wakeup,
              Event::Bell => Self::Bell,
              Event::Title(t) => Self::Title(t.clone()),
              Event::ResetTitle => Self::ResetTitle,
              Event::IconName(n) => Self::IconName(n.clone()),
              Event::ResetIconName => Self::ResetIconName,
              Event::ClipboardStore(ty, text) => Self::ClipboardStore(*ty, text.clone()),
              Event::ClipboardLoad(ty, _) => Self::ClipboardLoad(*ty),  // closure stripped
              Event::ColorRequest(idx, _) => Self::ColorRequest(*idx),  // closure stripped
              Event::PtyWrite(s) => Self::PtyWrite(s.clone()),
              Event::CursorBlinkingChange => Self::CursorBlinkingChange,
              Event::Cwd(path) => Self::Cwd(path.clone()),
              Event::CommandComplete(_) => Self::CommandComplete,  // duration stripped (non-deterministic)
              Event::MouseCursorDirty => Self::MouseCursorDirty,
              Event::ChildExit(code) => Self::ChildExit(*code),
          }
      }
  }
  ```

- [x] Define `RecordedListener`:
  ```rust
  /// Event listener that captures structured RecordedEvents.
  #[derive(Clone)]
  pub struct RecordedListener {
      events: Arc<Mutex<Vec<RecordedEvent>>>,
  }

  impl RecordedListener {
      pub fn new() -> Self { ... }

      /// All captured events.
      pub fn events(&self) -> Vec<RecordedEvent> { ... }

      /// Only PtyWrite events (response bytes).
      pub fn pty_writes(&self) -> Vec<String> { ... }

      /// Clear all captured events (interior mutability via Arc<Mutex>).
      pub fn clear(&self) { ... }
  }

  impl EventListener for RecordedListener {
      fn send_event(&self, event: Event) {
          // EventListener::send_event takes Event by value; convert to reference for From impl.
          self.events.lock().unwrap().push(RecordedEvent::from(&event));
      }
  }
  ```

- [x] `RecordedListener` uses `Arc<Mutex<Vec<RecordedEvent>>>` — same concurrency pattern as existing `RecordingListener` but with structured data instead of strings.

- [x] **Sync mechanism:** The `From<&Event> for RecordedEvent` impl uses an exhaustive `match` on `Event`. When a new `Event` variant is added to `oriterm_core::event::Event`, the compiler will produce a non-exhaustive match error in `events.rs`, forcing the implementer to add a corresponding `RecordedEvent` variant. This is compile-time enforcement — no separate sync test needed. Add a comment above the `From` impl documenting this: `// Exhaustive match ensures RecordedEvent stays in sync with Event.`

---

## 01.3 Reseq Subprocess Adapter

**File(s):** `oriterm_core/tests/teseq/harness/reseq.rs`

Thin adapter that invokes `reseq` to compile `.teseq` files into raw terminal bytes.

- [x] Implement `reseq_available() -> bool`:
  ```rust
  /// Check if reseq is installed and accessible.
  pub fn reseq_available() -> bool {
      std::process::Command::new("reseq")
          .arg("--version")
          .stdout(std::process::Stdio::null())
          .stderr(std::process::Stdio::null())
          .status()
          .is_ok()
  }
  ```
  Pattern mirrors `vttest_available()` from `oriterm_core/tests/vttest/session.rs:232-239`.

- [x] Implement `compile_teseq(teseq_path: &Path) -> Result<Vec<u8>, String>`:
  ```rust
  /// Compile a .teseq file to raw bytes via reseq subprocess.
  pub fn compile_teseq(teseq_path: &Path) -> Result<Vec<u8>, String> {
      let output = std::process::Command::new("reseq")
          .arg(teseq_path)
          .arg("-")     // output to stdout
          .output()
          .map_err(|e| format!("failed to run reseq: {e}"))?;

      if !output.status.success() {
          return Err(format!(
              "reseq failed (exit {}): {}",
              output.status,
              String::from_utf8_lossy(&output.stderr)
          ));
      }

      Ok(output.stdout)
  }
  ```

- [x] Implement `teseq_available() -> bool` (for outbound analysis in Section 03):
  ```rust
  pub fn teseq_available() -> bool {
      std::process::Command::new("teseq")
          .arg("--version")
          .stdout(std::process::Stdio::null())
          .stderr(std::process::Stdio::null())
          .status()
          .is_ok()
  }
  ```

---

## 01.4 TeseqHarness Runner

**File(s):** `oriterm_core/tests/teseq/harness/runner.rs`

The core runner: constructs `Term<RecordedListener>`, applies setup, feeds bytes, and produces a `ScenarioOutcome`.

- [x] Define `ScenarioOutcome`:
  ```rust
  /// Results from running a single teseq scenario.
  pub struct ScenarioOutcome {
      pub grid_text: String,
      pub grid_chars: Vec<Vec<char>>,
      pub cells: Vec<RenderableCell>,  // full cell data for attribute inspection (Section 05)
      pub cursor_col: usize,
      pub cursor_line: usize,
      pub events: Vec<RecordedEvent>,
      pub cols: usize,
      pub rows: usize,
      pub scrollback_len: usize,  // from RenderableContent::scrollback_len (for ED 3 validation)
  }
  ```

- [x] Implement `TeseqHarness`:
  ```rust
  /// Integration tests must use fully qualified `vte::ansi::Processor`
  /// (not a `use` re-export from `oriterm_core` internals).
  pub struct TeseqHarness {
      term: Term<RecordedListener>,
      proc: vte::ansi::Processor,
      listener: RecordedListener,
      spec: ScenarioSpec,
  }

  impl TeseqHarness {
      /// Create harness from a .teseq file path.
      pub fn from_scenario(teseq_path: &Path) -> Self {
          let spec = ScenarioSpec::load(teseq_path);
          let listener = RecordedListener::new();
          let theme = match spec.terminal.theme.as_str() {
              "light" => Theme::Light,
              _ => Theme::default(),  // Dark (default) or Unknown — both use dark palette
          };
          let mut term = Term::new(
              spec.terminal.rows,
              spec.terminal.cols,
              spec.terminal.scrollback,
              theme,
              listener.clone(),
          );
          let mut proc = vte::ansi::Processor::new();

          // Apply pre_feed setup sequences.
          for seq in &spec.setup.pre_feed {
              let bytes = parse_escape_string(seq);
              proc.advance(&mut term, &bytes);
          }

          // Clear setup events — only scenario events matter.
          listener.clear();

          Self { term, proc, listener, spec }
      }

      /// Feed the .teseq scenario through the terminal.
      ///
      /// Takes the path explicitly (same as `from_scenario`) because the harness
      /// does not store the path — keeping it stateless avoids lifetime issues.
      /// Callers pass the same `&Path` to both `from_scenario` and `run`.
      pub fn run(&mut self, teseq_path: &Path) -> ScenarioOutcome {
          let bytes = compile_teseq(teseq_path)
              .expect("reseq compilation failed");
          self.proc.advance(&mut self.term, &bytes);
          self.outcome()
      }

      /// Access the loaded scenario spec.
      pub fn spec(&self) -> &ScenarioSpec { &self.spec }

      /// Extract current terminal state as ScenarioOutcome.
      fn outcome(&self) -> ScenarioOutcome {
          let content = self.term.renderable_content();
          let grid_text = grid_text_from_content(&content);
          let grid_chars = grid_chars_from_content(&content);

          // Use RenderableCursor from content (single source of truth)
          // rather than querying grid().cursor() separately.
          ScenarioOutcome {
              grid_text,
              grid_chars,
              cells: content.cells.clone(),  // preserve for attribute inspection
              cursor_col: content.cursor.column.0,
              cursor_line: content.cursor.line,
              events: self.listener.events(),
              cols: content.cols,
              rows: content.lines,
              scrollback_len: content.scrollback_len,
          }
      }
  }
  ```

- [x] Implement `parse_escape_string(s: &str) -> Vec<u8>` helper:
  - Converts escape notation like `\x1b[?40h` to raw bytes
  - Handles `\x##` hex escapes, `\e` or `\x1b` for ESC, `\n`, `\r`, `\t`
  - Used for `pre_feed` strings in sidecar TOML
  - **TOML escaping note:** TOML strings use `\\` for literal backslash. So `pre_feed = ["\\x1b[?40h"]` in TOML yields the Rust string `\x1b[?40h`, which `parse_escape_string` then converts to raw bytes `[0x1b, 0x5b, 0x3f, 0x34, 0x30, 0x68]`. This double-escape layer is inherent to TOML — document it in the scenario authoring guide (Section 07.3).

- [x] Implement `grid_text_from_content()` and `grid_chars_from_content()`:
  - Same logic as `VtTestSession::grid_text()` in `session.rs:187-207`
  - Extracted as free functions for reuse without VtTestSession

---

## 01.5 Assertion Helpers & Snapshot Infrastructure

**File(s):** `oriterm_core/tests/teseq/harness/assertions.rs`

Assertion helpers that integrate with insta and provide convenience methods for common checks.

- [x] Implement grid snapshot assertion:
  ```rust
  /// Assert grid state matches insta golden snapshot.
  pub fn assert_grid_snapshot(outcome: &ScenarioOutcome, name: &str) {
      insta::assert_snapshot!(name, outcome.grid_text);
  }
  ```

- [x] Implement event snapshot assertion:
  ```rust
  /// Assert event sequence matches insta golden snapshot.
  pub fn assert_event_snapshot(outcome: &ScenarioOutcome, name: &str) {
      let event_text = outcome.events
          .iter()
          .map(|e| format!("{e:?}"))
          .collect::<Vec<_>>()
          .join("\n");
      insta::assert_snapshot!(name, event_text);
  }
  ```

- [x] Implement cursor position assertion:
  ```rust
  /// Assert cursor is at expected position.
  pub fn assert_cursor(outcome: &ScenarioOutcome, col: usize, line: usize) {
      assert_eq!(
          (outcome.cursor_col, outcome.cursor_line),
          (col, line),
          "cursor at col={}, line={} but expected col={col}, line={line}",
          outcome.cursor_col, outcome.cursor_line
      );
  }
  ```

- [x] Implement spec-driven assertion runner:
  ```rust
  /// Run all assertions specified in the ScenarioSpec.
  pub fn assert_spec(outcome: &ScenarioOutcome, spec: &ScenarioSpec, name: &str) {
      if spec.expect.grid_snapshot {
          assert_grid_snapshot(outcome, &format!("{name}_grid"));
      }
      if spec.expect.event_snapshot {
          assert_event_snapshot(outcome, &format!("{name}_events"));
      }
      if let Some(cursor) = &spec.expect.cursor {
          assert_cursor(outcome, cursor.col, cursor.line);
      }
      // Event name matching: each `expected_event` string is matched against the
      // Debug output of RecordedEvent variants. This uses `contains()` for
      // flexibility (e.g., "Bell" matches "Bell", "Title" matches "Title(...)").
      // For exact payload matching, use dedicated assertion helpers instead.
      for expected_event in &spec.expect.events {
          assert!(
              outcome.events.iter().any(|e| format!("{e:?}").contains(expected_event)),
              "expected event containing {expected_event:?} not found in {:?}",
              outcome.events
          );
      }
  }
  ```

- [x] Implement scrollback assertion:
  ```rust
  /// Assert scrollback buffer is empty (e.g., after ED 3).
  pub fn assert_scrollback_empty(outcome: &ScenarioOutcome) {
      assert_eq!(
          outcome.scrollback_len, 0,
          "expected empty scrollback, got {} lines",
          outcome.scrollback_len
      );
  }
  ```

- [x] **TPR checkpoint** — `/tpr-review` covering 01.1–01.5 implementation work (4 iterations, 12 findings fixed)

---

## 01.6 Smoke Test & Directory Structure

**File(s):** `oriterm_core/tests/teseq/main.rs`, `oriterm_core/tests/teseq/scenarios/c0/bel.teseq`

Wire everything together with a smoke test that proves the full pipeline works.

- [x] Create `oriterm_core/tests/teseq/main.rs`:
  ```rust
  //! Teseq-based escape sequence conformance tests.
  //!
  //! Uses GNU teseq/reseq to author human-readable escape sequence scenarios,
  //! feeds them through `Term<RecordedListener>`, and validates terminal state
  //! against insta golden snapshots.
  //!
  //! Requires `reseq` installed (`sudo apt install teseq`).
  //! Tests gracefully skip when reseq is unavailable.
  //!
  //! # Commands
  //!
  //! - Run: `cargo test -p oriterm_core --test teseq`
  //! - Update snapshots: `INSTA_UPDATE=1 cargo test -p oriterm_core --test teseq`

  mod harness;

  // Family modules (added in Sections 02-06)
  ```

- [x] Create `harness/mod.rs` re-exporting all sub-modules:
  ```rust
  pub mod assertions;
  pub mod events;
  pub mod loader;
  pub mod reseq;
  pub mod runner;

  pub use assertions::*;
  pub use events::{RecordedEvent, RecordedListener};
  pub use loader::ScenarioSpec;
  pub use reseq::{reseq_available, compile_teseq, teseq_available};
  pub use runner::{TeseqHarness, ScenarioOutcome};
  ```

- [x] Create smoke scenario `scenarios/c0/bel.teseq`:
  ```
  |Hello|
  . BEL/^G
  | World|
  ```
  Produces raw bytes: `Hello\x07 World` (no trailing LF — no `.` after the closing `|`).

- [x] Create smoke scenario sidecar `scenarios/c0/bel.toml`:
  ```toml
  [expect]
  cursor = { col = 11, line = 0 }
  events = ["Bell"]
  ```
  Cursor: "Hello" (5 chars) + BEL (no movement) + " World" (6 chars) = col 11, line 0.

- [x] Create smoke test in `main.rs`:
  ```rust
  #[test]
  fn smoke_bel() {
      if !harness::reseq_available() {
          eprintln!("reseq not installed, skipping teseq tests");
          return;
      }
      let scenario_path = Path::new(env!("CARGO_MANIFEST_DIR"))
          .join("tests/teseq/scenarios/c0/bel.teseq");
      let mut h = TeseqHarness::from_scenario(&scenario_path);
      let outcome = h.run(&scenario_path);

      harness::assert_grid_snapshot(&outcome, "smoke_bel_grid");
      harness::assert_event_snapshot(&outcome, "smoke_bel_events");
      harness::assert_cursor(&outcome, 11, 0);
  }
  ```

- [x] Verify: `timeout 150 cargo test -p oriterm_core --test teseq` compiles and the smoke test passes
- [x] Verify: `./build-all.sh` and `./clippy-all.sh` and `timeout 150 ./test-all.sh` all pass

---

## 01.R Third Party Review Findings

<!-- Reserved for Codex or other external reviewers. -->

- [x] `[TPR-01-001][major]` `oriterm_core/src/image/cache/mod.rs:153` — Animated image replacement leaks old animation state. Replacing an existing image ID only removed image data and placements but not `animations`, `animation_frames`, `frame_starts`.
  Resolved: Fixed on 2026-04-05. Replaced inline cleanup with call to `remove_image()` which handles all animation state cleanup.
- [x] `[TPR-01-002][major]` `oriterm_core/src/term/handler/image/sixel.rs:99` — Sixel "stale placement" fix leaks superseded image payloads. `remove_by_position()` removed placements but didn't prune orphaned image data.
  Resolved: Fixed on 2026-04-05. Added orphan pruning to `remove_by_position()` — images with zero remaining placements are fully removed.
- [x] `[TPR-01-003][medium]` `oriterm_mux/src/pty/reader/mod.rs:96` — BUG-11-1 reader 1ms sleep was unconditional across all platforms instead of Windows/ConPTY only.
  Resolved: Fixed on 2026-04-05. Gated sleep behind `#[cfg(windows)]`.
- [x] `[TPR-01-004][major]` `oriterm_core/src/image/cache/mod.rs:322` — `remove_placements_in_region()` (ED/EL/ECH erase paths) only dropped placements without reclaiming orphaned image payloads.
  Resolved: Fixed on 2026-04-05. Added orphan pruning to `remove_placements_in_region()`, matching the fix applied to `remove_by_position()`.
- [x] `[TPR-01-005][major]` `oriterm/src/key_encoding/mod.rs:107` — Doc comment falsely listed Win32 input mode as dispatch priority #1 when it's not wired into `encode_key()`.
  Resolved: Fixed on 2026-04-05. Updated doc comment to accurately reflect that Win32 encoding is not yet dispatched; activation tracked in Section 08b.
- [x] `[TPR-01-006][medium]` `oriterm_core/tests/teseq/harness/reseq.rs:25` — `compile_teseq()` used `/dev/stdout` which is Unix-only, violating the plan's cross-platform graceful-skip contract.
  Resolved: Fixed on 2026-04-05. Switched to temp-file output (cross-platform).
- [x] `[TPR-01-007][minor]` Plan drift: `index.md` and `00-overview.md` still reported Section 01 as "Not Started" despite implementation being in progress.
  Resolved: Fixed on 2026-04-05. Updated both files to "In Progress".
- [x] `[TPR-01-008][high]` `oriterm_mux/src/pane/mod.rs:442` — signal_child() sends SIGINT to shell PGID instead of foreground PGID; e2e tests ignored.
  Resolved: Filed as BUG-11-5 on 2026-04-05. Keyboard Ctrl+C works correctly (kernel handles \x03 → SIGINT). Programmatic signal path needs tcgetpgrp fix.
- [x] `[TPR-01-009][medium]` `oriterm_core/tests/teseq/harness/reseq.rs:22` — compile_teseq() used a single shared temp file path that would race when tests run in parallel.
  Resolved: Fixed on 2026-04-05. Now generates unique temp filenames using atomic counter + thread ID.
- [x] `[TPR-01-010][low]` `oriterm_core/src/image/cache/mod.rs` — File exceeded 500-line limit (516 lines).
  Resolved: Fixed on 2026-04-05. Extracted eviction methods into `cache/eviction.rs` submodule (mod.rs now 458 lines).
- [x] `[TPR-01-011][high]` `oriterm_mux/tests/e2e.rs:1528` / `oriterm_mux/tests/e2e.rs:1609` — BUG-11-1 e2e prompt detection was hard-coded to `$`, `#`, `%`, missing `❯` and `>` prompts.
  Resolved: Fixed on 2026-04-05. Extracted `snapshot_has_prompt()` helper recognizing `$`, `#`, `%`, `>`, `❯`. All three inline prompt checks replaced.
- [x] `[TPR-01-012][medium]` `oriterm_core/src/image/cache/mod.rs:154` / `oriterm_core/src/image/tests.rs:173` — Image cache leak fixes missing semantic regression pins.
  Resolved: Fixed on 2026-04-05. Added 3 regression tests: `store_replace_reclaims_old_image_memory`, `remove_by_position_prunes_orphaned_image`, `remove_placements_in_region_prunes_orphaned_image`. All verify `image_count()` and `memory_used()` drop to zero when last placement is removed.
- [x] `[TPR-01-013][major]` `oriterm_core/src/image/cache/mod.rs:249` / `oriterm_core/src/term/handler/image/kitty.rs:183` / `oriterm_core/src/image/tests.rs:255` — The `TPR-01-002` fix over-broadened `remove_by_position()`: it now destroys image payloads whenever the removed placement was the last one, but Kitty lowercase delete specifiers are explicitly "delete placements only". `kitty_delete()` routes both `d=r` and `d=R` through `remove_by_position()`, so lowercase `d=r` now collapses into uppercase behavior whenever the targeted placement is the image's final placement.
  Resolved: Fixed on 2026-04-05. Removed auto-pruning from `remove_by_position()` — it now only removes placements. Sixel overwrite (`sixel.rs`) and Kitty uppercase delete callers explicitly call `remove_orphans()` after. Split regression test into `remove_by_position_preserves_image_data` (lowercase semantic) and `remove_by_position_then_orphan_prune` (uppercase/sixel semantic).
- [x] `[TPR-01-014][major]` `oriterm_core/src/image/cache/eviction.rs:50` / `oriterm_core/src/term/handler/image/sixel.rs:103` / `oriterm_core/src/term/handler/image/kitty.rs:241` — The follow-up fix now prunes orphans with a global sweep. That breaks valid Kitty deferred-placement flows where an image is intentionally stored with zero placements (`a=t`, `a=T,U=1`, or `a=p,U=1`). The existing tests `handler_unicode_placeholder_skips_placement` and `handler_unicode_placeholder_place_skips` prove zero-placement stored images are valid state, but any sixel overwrite or `d=R` delete now calls `remove_orphans()` and will delete those unrelated stored images too. A later `a=p,i=...` can then fail with `ENOENT` even though the image was never targeted for deletion.
  Resolved: Fixed on 2026-04-05. Replaced global `remove_orphans()` sweep with targeted `prune_if_orphaned(ids)` that only checks specific image IDs affected by the removal. Extracted `remove_placements_where()` higher-order helper to eliminate algorithmic duplication across all removal methods. Removed dead `remove_orphans()` method. Added `targeted_prune_preserves_unrelated_deferred_images` regression test. All Kitty uppercase, sixel, erase, and scrollback prune paths now use targeted pruning.

---

## 01.N Completion Checklist

- [x] `oriterm_core/tests/teseq/` directory exists with `main.rs` and `harness/` submodules
- [x] `ScenarioSpec` loads and parses TOML sidecars with defaults for missing fields
- [x] `RecordedEvent` enum covers all `Event` variants with structured data
- [x] `RecordedListener` implements `EventListener` and provides `events()`, `pty_writes()`, `clear()`
- [x] `reseq_available()` returns false gracefully when reseq not installed
- [x] `compile_teseq()` converts `.teseq` files to raw bytes via reseq subprocess
- [x] `TeseqHarness::from_scenario()` constructs Term with spec config and applies pre_feed
- [x] `TeseqHarness::run()` feeds compiled bytes and returns `ScenarioOutcome`
- [x] `grid_text_from_content()` and `grid_chars_from_content()` match VtTestSession behavior
- [x] Assertion helpers work: `assert_grid_snapshot`, `assert_event_snapshot`, `assert_cursor`, `assert_scrollback_empty`
- [x] Smoke test (`smoke_bel`) passes end-to-end: `.teseq` → reseq → bytes → Term → snapshot assertion
- [x] `toml` and `serde` added to dev-dependencies in `oriterm_core/Cargo.toml`
- [x] No new warnings from `./clippy-all.sh`
- [x] `./build-all.sh` green
- [x] `timeout 150 ./test-all.sh` green — no regressions
- [x] Plan annotation cleanup: no temporary scaffolding in `.rs` files
- [x] All intermediate TPR checkpoint findings resolved
- [x] **Plan sync** — update plan metadata:
  - [x] This section's frontmatter `status` → `complete`
  - [x] `00-overview.md` Quick Reference table updated
  - [x] `index.md` section status updated
- [x] `/tpr-review` passed (4 iterations, 12 findings all resolved)
- [x] `/impl-hygiene-review last commit` — covered by 4-iteration TPR review (12 findings all resolved)

**Exit Criteria:** `timeout 150 cargo test -p oriterm_core --test teseq -- smoke_bel` passes. The teseq harness loads a `.teseq` scenario, compiles it via `reseq`, feeds it through `Term<RecordedListener>`, captures grid state and events, and validates against insta golden snapshots — all in a single test function. Zero regressions in `timeout 150 ./test-all.sh`.
