---
section: "03"
title: "Reports & Response Validation"
status: complete
reviewed: true
goal: "Create scenarios that validate outbound terminal responses (DA, DSR, DECRQM private, DECRQM ANSI) with raw PtyWrite byte assertions as the canonical oracle and optional teseq debug analysis"
success_criteria:
  - "DA1 scenario validates correct PtyWrite response bytes via assert_pty_writes"
  - "DA2 and DA3 scenarios validate correct response bytes"
  - "DSR device status (5n) and cursor position (6n) scenarios validate correct response bytes"
  - "DECRQM private mode scenarios validate mode report response bytes for key DEC private modes"
  - "DECRQM ANSI mode scenarios validate mode report response bytes for ANSI modes (IRM, LNM)"
  - "Outbound response pipeline works: assert_pty_writes (canonical), assert_response_snapshot (hex golden), analyze_response (debug aid)"
  - "Satisfies mission criteria: DA1, DA2, DA3, DSR, DECRQM (private + ANSI) coverage with raw PtyWrite byte assertions"
inspired_by:
  - "ori_term handler/tests.rs — DA/DSR tests via RecordingListener + PtyWrite event capture"
  - "ori_term vttest menu6 (oriterm_core/tests/vttest/menu6.rs) — report validation via PtyResponder"
  - "ori_term status.rs (oriterm_core/src/term/handler/status.rs) — DA/DSR response generation"
depends_on: ["01", "02"]
third_party_review:
  status: resolved
  updated: 2026-04-05
sections:
  - id: "03.1"
    title: "Outbound Response Assertion Pipeline"
    status: complete
  - id: "03.2"
    title: "Device Attributes Scenarios (DA1/DA2/DA3)"
    status: complete
  - id: "03.3"
    title: "Device Status Report Scenarios (DSR)"
    status: complete
  - id: "03.4"
    title: "Private Mode Report Scenarios (DECRQM Private)"
    status: complete
  - id: "03.5"
    title: "ANSI Mode Report Scenarios (DECRQM ANSI)"
    status: complete
  - id: "03.R"
    title: "Third Party Review Findings"
    status: in-progress
  - id: "03.N"
    title: "Completion Checklist"
    status: complete
---

# Section 03: Reports & Response Validation

**Status:** Complete
**Goal:** Validate that ori_term generates correct outbound responses to terminal queries (DA, DSR, DECRQM private, DECRQM ANSI). This section uses the `RecordedEvent::PtyWrite` payloads from Section 01's harness with raw byte assertions as the canonical oracle (`assert_pty_writes`). Teseq analysis is available as an optional debug aid for human-readable output when tests fail, but is never the golden oracle.

**Success Criteria:**

- [x] Outbound response assertion pipeline works: `assert_pty_writes` for canonical raw byte checks, `assert_response_snapshot` for hex golden, `analyze_response` for optional debug
- [x] DA1 response matches expected format (`ESC [ ? 64 ; 6 ; 4 c` — VT420 class with ANSI color + sixel)
- [x] DA2 response matches expected format
- [x] DA3 response matches expected format
- [x] DSR device status response reports OK (`\x1b[0n`)
- [x] DSR cursor position response encodes correct coordinates
- [x] DECRQM private mode responses correctly report DEC private mode states (set/reset)
- [x] DECRQM ANSI mode responses correctly report ANSI mode states (IRM, LNM)
- [x] All response scenarios use `assert_pty_writes` as the canonical assertion (raw bytes, no teseq dependency)
- [x] All response scenarios also use `assert_response_snapshot` as a secondary hex golden assertion

**Context:** Terminal responses are critical for interoperability — programs like vttest, tmux, and SSH clients make decisions based on DA responses. The vttest conformance work (Section 06 of completed plan) already validated DA responses in the PTY context. This section tests the same responses through the teseq harness, adding human-readable teseq analysis as a second validation layer.

**Reference implementations:**
- **ori_term** `handler/status.rs` — `status_identify_terminal()`: DA1/DA2/DA3 response generation
- **ori_term** `handler/status.rs` — `status_device_status()`: DSR device status (arg 5 → `\x1b[0n`) and cursor position (arg 6 → `\x1b[line;colR`)
- **ori_term** `handler/status.rs` — `status_report_private_mode()`: DEC private mode DECRQM response generation
- **ori_term** `handler/status.rs` — `status_report_mode()`: ANSI mode DECRQM response generation (IRM mode 4, LNM mode 20)
- **ori_term** `handler/tests.rs` — DA/DSR tests capture events via `RecordingListener`

**Not in scope (documented):** `status_text_area_size_chars()` (CSI 18 t) and `status_decrqss()` (DECRQSS/DCS $ q) also generate PtyWrite responses but are not listed in the mission criteria. DECRQSS is flagged in Section 07's gap analysis. CSI 18 t should be added to Section 07's gap analysis as well (currently missing).

**Depends on:** Section 01 (RecordedEvent with PtyWrite payloads), Section 02 (basic scenario pattern established).

---

## 03.1 Outbound Response Assertion Pipeline

**File(s):** `oriterm_core/tests/teseq/harness/assertions.rs` (extend), `oriterm_core/tests/teseq/harness/mod.rs` (update re-exports)

Add response assertion helpers. **Design principle:** Raw PtyWrite bytes are the canonical assertion (via `assert_pty_writes`). Teseq analysis is an optional supplementary debug aid (via `analyze_response`) — never the oracle. This keeps test correctness independent of teseq's output format, which is version-fragile and human-oriented.

**File size budget:** `assertions.rs` is currently 69 lines. After adding `assert_pty_writes` (~18 lines), `assert_response_snapshot` (~12 lines), and `analyze_response` (~24 lines), it will be ~125 lines — well under the 500-line limit. No split needed.

- [x] Add `use super::events::RecordedEvent;` and `use super::reseq::teseq_available;` to the imports in `assertions.rs` (needed by the new functions).

- [x] Implement `assert_pty_writes(outcome: &ScenarioOutcome, expected: &[&str])`:
  ```rust
  /// Assert PtyWrite response bytes match expected values exactly.
  ///
  /// This is the canonical response assertion — raw bytes are the oracle,
  /// not teseq output. Each entry in `expected` is compared verbatim against
  /// the corresponding PtyWrite event payload.
  pub fn assert_pty_writes(outcome: &ScenarioOutcome, expected: &[&str]) {
      let actual: Vec<&str> = outcome.events.iter()
          .filter_map(|e| match e {
              RecordedEvent::PtyWrite(s) => Some(s.as_str()),
              _ => None,
          })
          .collect();
      assert_eq!(
          actual.len(), expected.len(),
          "expected {} PtyWrite events, got {}: {:?}",
          expected.len(), actual.len(), actual
      );
      for (i, (got, want)) in actual.iter().zip(expected).enumerate() {
          assert_eq!(
              got, want,
              "PtyWrite[{i}] mismatch:\n  got:  {:02x?}\n  want: {:02x?}",
              got.as_bytes(), want.as_bytes()
          );
      }
  }
  ```

- [x] Implement `assert_response_snapshot(outcome: &ScenarioOutcome, name: &str)`:
  ```rust
  /// Snapshot PtyWrite response bytes for golden comparison.
  ///
  /// Snapshots the raw response bytes (hex-escaped for readability).
  /// This is a secondary assertion — `assert_pty_writes` is the primary
  /// canonical check. The snapshot catches unexpected format changes.
  pub fn assert_response_snapshot(outcome: &ScenarioOutcome, name: &str) {
      let pty_writes: Vec<String> = outcome.events.iter()
          .filter_map(|e| match e {
              RecordedEvent::PtyWrite(s) => Some(format!("{:02x?}", s.as_bytes())),
              _ => None,
          })
          .collect();
      insta::assert_snapshot!(format!("{name}_responses"), pty_writes.join("\n"));
  }
  ```

- [x] Implement `analyze_response(response_bytes: &str) -> Result<String, String>` (supplementary debug helper):
  ```rust
  /// Pipe response bytes through teseq for human-readable debug output.
  ///
  /// This is NOT an oracle — it is a debug aid for understanding response
  /// content when tests fail. Never use the return value as a golden
  /// assertion target. Falls back to hex dump if teseq is unavailable.
  pub fn analyze_response(response_bytes: &str) -> Result<String, String> {
      use std::io::Write as _;  // needed for write_all on ChildStdin

      if !teseq_available() {
          return Ok(format!("hex: {:02x?}", response_bytes.as_bytes()));
      }

      let mut child = std::process::Command::new("teseq")
          .stdin(std::process::Stdio::piped())
          .stdout(std::process::Stdio::piped())
          .stderr(std::process::Stdio::piped())
          .spawn()
          .map_err(|e| format!("failed to spawn teseq: {e}"))?;

      // take() returns Option<ChildStdin>; safe to unwrap because we set piped().
      // The temporary ChildStdin is dropped at statement end, closing the pipe
      // and signaling EOF to teseq.
      child.stdin.take().unwrap()
          .write_all(response_bytes.as_bytes())
          .map_err(|e| format!("failed to write to teseq: {e}"))?;

      let output = child.wait_with_output()
          .map_err(|e| format!("teseq failed: {e}"))?;

      Ok(String::from_utf8_lossy(&output.stdout).to_string())
  }
  ```

- [x] Update `harness/mod.rs` re-exports — add `assert_pty_writes`, `assert_response_snapshot`, and `analyze_response` to the `pub use assertions::{...}` line. This follows the existing pattern where all assertion helpers are re-exported for use by family modules.

---

## 03.2 Device Attributes Scenarios (DA1/DA2/DA3)

**File(s):** `oriterm_core/tests/teseq/scenarios/csi/reports/*.teseq` + `*.toml`, `oriterm_core/tests/teseq/csi_reports.rs`, `oriterm_core/tests/teseq/main.rs`

### Prerequisite: directory and family module scaffolding

- [x] Create scenario directory `oriterm_core/tests/teseq/scenarios/csi/reports/` (new subdirectory under existing `csi/`).

- [x] Add `#[derive(Clone)]` to `ScenarioSpec`, `TerminalConfig`, `SetupConfig`, `ExpectConfig`, and `CursorExpect` in `loader.rs` (needed by `run_scenario` which calls `h.spec().clone()`). Alternatively, restructure the helper to avoid the clone — choose whichever is simpler at implementation time. **Chose restructure:** `run_scenario` calls `assert_spec` while holding `&h`, then returns only the `ScenarioOutcome`. No Clone needed.

- [x] Create family module `oriterm_core/tests/teseq/csi_reports.rs` with `run_scenario` helper. Unlike basic scenario families (which only call `assert_spec`), report scenarios need additional response assertions. The helper pattern:
  ```rust
  //! CSI report and response scenarios.

  use std::path::Path;

  use super::harness::{
      self, TeseqHarness, assert_pty_writes, assert_response_snapshot, reseq_available,
  };

  /// Run a report scenario and apply spec assertions only.
  ///
  /// Callers must guard with `if !reseq_available() { return; }` before
  /// calling this. This function assumes reseq is available.
  fn run_scenario(name: &str) -> (super::harness::ScenarioOutcome, super::harness::ScenarioSpec) {
      let path = Path::new(env!("CARGO_MANIFEST_DIR"))
          .join("tests/teseq/scenarios/csi/reports")
          .join(format!("{name}.teseq"));
      let mut h = TeseqHarness::from_scenario(&path);
      let outcome = h.run(&path);
      let spec = h.spec().clone();
      harness::assert_spec(&outcome, &spec, &format!("csi_reports_{name}"));
      (outcome, spec)
  }
  ```
  **Note on `run_scenario` returning the outcome:** Unlike `c0.rs`/`csi_cursor.rs` where `run_scenario` is fire-and-forget (assertions are entirely spec-driven), report tests need the `ScenarioOutcome` back to call `assert_pty_writes` with scenario-specific expected bytes. The `Clone` derive (checklist item above) enables the `h.spec().clone()` call in this helper.

- [x] Register family module in `oriterm_core/tests/teseq/main.rs` by adding `mod csi_reports;` to the family module list (after the existing `mod esc;` line).

### DA1 scenario

- [x] Create `oriterm_core/tests/teseq/scenarios/csi/reports/da1.teseq`:
  ```
  : Esc [ c
  ```

- [x] Create `oriterm_core/tests/teseq/scenarios/csi/reports/da1.toml`:
  ```toml
  [expect]
  grid_snapshot = false
  event_snapshot = false
  events = ["PtyWrite"]
  ```
  Grid and event snapshots disabled — DA queries don't change visible content, and the canonical assertion is `assert_pty_writes`, not a snapshot of the event list.

- [x] Add `#[test] fn da1()` in `csi_reports.rs`:
  ```rust
  #[test]
  fn da1() {
      if !reseq_available() { eprintln!("reseq not installed, skipping"); return; }
      let (outcome, _) = run_scenario("da1");
      assert_pty_writes(&outcome, &["\x1b[?64;6;4c"]);
      assert_response_snapshot(&outcome, "csi_reports_da1");
  }
  ```
  Expected response: `\x1b[?64;6;4c` (VT420 class, ANSI color, sixel graphics). Source: `status_identify_terminal()` DA1 branch in `status.rs`.

### DA2 scenario

- [x] Implement `compute_da2_version() -> usize` helper in `csi_reports.rs` (not in `assertions.rs` — it is scenario-specific, not a general assertion helper). Replicates the algorithm from `crate_version_number()` in `helpers.rs`: parse `env!("CARGO_PKG_VERSION")`, strip pre-release suffix, split on `.`, reverse, and sum `part * 100^i`. Do NOT hardcode a version number — it changes on every release.
  ```rust
  /// Replicate `crate_version_number()` from `handler/helpers.rs`.
  ///
  /// `pub(super)` visibility prevents test code from calling the real function.
  /// This replica uses the same algorithm so tests track version bumps.
  fn compute_da2_version() -> usize {
      let mut result = 0usize;
      let version = env!("CARGO_PKG_VERSION");
      let version = version.split('-').next().unwrap_or(version);
      for (i, part) in version.split('.').rev().enumerate() {
          let n = part.parse::<usize>().unwrap_or(0);
          result += n * 100usize.pow(i as u32);
      }
      result
  }
  ```

- [x] **Drift mitigation unit test** — add `#[test] fn da2_version_drift_check()` in `csi_reports.rs` that feeds `\x1b[>c` directly through `Term<RecordedListener>` (bypassing teseq/reseq entirely) and verifies the PtyWrite response matches `format!("\x1b[>0;{};1c", compute_da2_version())`. This test catches algorithm drift between `crate_version_number()` and `compute_da2_version()` immediately, since both evaluate `CARGO_PKG_VERSION` at compile time. Does not require reseq — no skip guard.

- [x] Create `oriterm_core/tests/teseq/scenarios/csi/reports/da2.teseq`:
  ```
  : Esc [ > c
  ```

- [x] Create `oriterm_core/tests/teseq/scenarios/csi/reports/da2.toml`:
  ```toml
  [expect]
  grid_snapshot = false
  event_snapshot = false
  events = ["PtyWrite"]
  ```

- [x] Add `#[test] fn da2()` in `csi_reports.rs`:
  ```rust
  #[test]
  fn da2() {
      if !reseq_available() { eprintln!("reseq not installed, skipping"); return; }
      let (outcome, _) = run_scenario("da2");
      let expected = format!("\x1b[>0;{};1c", compute_da2_version());
      assert_pty_writes(&outcome, &[&expected]);
      assert_response_snapshot(&outcome, "csi_reports_da2");
  }
  ```
  Expected response: `\x1b[>0;{version};1c` — terminal type 0 (VT100-compatible), version number, conformance level 1.

### DA3 scenario

- [x] Create `oriterm_core/tests/teseq/scenarios/csi/reports/da3.teseq`:
  ```
  : Esc [ = c
  ```

- [x] Create `oriterm_core/tests/teseq/scenarios/csi/reports/da3.toml`:
  ```toml
  [expect]
  grid_snapshot = false
  event_snapshot = false
  events = ["PtyWrite"]
  ```

- [x] Add `#[test] fn da3()` in `csi_reports.rs`:
  ```rust
  #[test]
  fn da3() {
      if !reseq_available() { eprintln!("reseq not installed, skipping"); return; }
      let (outcome, _) = run_scenario("da3");
      assert_pty_writes(&outcome, &["\x1bP!|00000000\x1b\\"]);
      assert_response_snapshot(&outcome, "csi_reports_da3");
  }
  ```
  Expected response: `\x1bP!|00000000\x1b\\` (DCS response with eight zero digits as unit ID, same as xterm default). Source: `status_identify_terminal()` DA3 branch in `status.rs`.

### Verify DA scenarios

- [x] Run: `timeout 150 cargo test -p oriterm_core --test teseq -- csi_reports::da` — all 4 tests pass (da1, da2, da3, da2_version_drift_check). Accept insta snapshots with `INSTA_UPDATE=1` on first run.

---

## 03.3 Device Status Report Scenarios (DSR)

**File(s):** `oriterm_core/tests/teseq/scenarios/csi/reports/dsr_*.teseq` + `*.toml`, `oriterm_core/tests/teseq/csi_reports.rs` (extend)

### DSR device status (arg 5)

- [x] Create `oriterm_core/tests/teseq/scenarios/csi/reports/dsr_device_status.teseq`:
  ```
  : Esc [ 5 n
  ```

- [x] Create `oriterm_core/tests/teseq/scenarios/csi/reports/dsr_device_status.toml`:
  ```toml
  [expect]
  grid_snapshot = false
  event_snapshot = false
  events = ["PtyWrite"]
  ```

- [x] Add `#[test] fn dsr_device_status()` in `csi_reports.rs`:
  ```rust
  #[test]
  fn dsr_device_status() {
      if !reseq_available() { eprintln!("reseq not installed, skipping"); return; }
      let (outcome, _) = run_scenario("dsr_device_status");
      assert_pty_writes(&outcome, &["\x1b[0n"]);
      assert_response_snapshot(&outcome, "csi_reports_dsr_device_status");
  }
  ```
  Expected response: `\x1b[0n` (device OK). Source: `status.rs` DSR arg 5 branch.

### DSR cursor position at home (arg 6)

- [x] Create `oriterm_core/tests/teseq/scenarios/csi/reports/dsr_cursor_home.teseq`:
  ```
  : Esc [ 6 n
  ```

- [x] Create `oriterm_core/tests/teseq/scenarios/csi/reports/dsr_cursor_home.toml`:
  ```toml
  [expect]
  grid_snapshot = false
  event_snapshot = false
  events = ["PtyWrite"]
  ```

- [x] Add `#[test] fn dsr_cursor_home()` in `csi_reports.rs`:
  ```rust
  #[test]
  fn dsr_cursor_home() {
      if !reseq_available() { eprintln!("reseq not installed, skipping"); return; }
      let (outcome, _) = run_scenario("dsr_cursor_home");
      assert_pty_writes(&outcome, &["\x1b[1;1R"]);
      assert_response_snapshot(&outcome, "csi_reports_dsr_cursor_home");
  }
  ```
  Expected response: `\x1b[1;1R` (cursor at home position, 1-based).

### DSR cursor after movement (arg 6)

- [x] Create `oriterm_core/tests/teseq/scenarios/csi/reports/dsr_cursor_moved.teseq`:
  ```
  : Esc [ 10 ; 20 H
  : Esc [ 6 n
  ```

- [x] Create `oriterm_core/tests/teseq/scenarios/csi/reports/dsr_cursor_moved.toml`:
  ```toml
  [expect]
  grid_snapshot = false
  event_snapshot = false
  events = ["PtyWrite"]
  ```

- [x] Add `#[test] fn dsr_cursor_moved()` in `csi_reports.rs`:
  ```rust
  #[test]
  fn dsr_cursor_moved() {
      if !reseq_available() { eprintln!("reseq not installed, skipping"); return; }
      let (outcome, _) = run_scenario("dsr_cursor_moved");
      assert_pty_writes(&outcome, &["\x1b[10;20R"]);
      assert_response_snapshot(&outcome, "csi_reports_dsr_cursor_moved");
  }
  ```
  Expected response: `\x1b[10;20R` (cursor at row 10, col 20, both 1-based).

### DSR with origin mode (arg 6 + DECOM)

- [x] Create `oriterm_core/tests/teseq/scenarios/csi/reports/dsr_origin_mode.teseq`:
  ```
  : Esc [ ? 6 h
  : Esc [ 5 ; 20 r
  : Esc [ 3 ; 10 H
  : Esc [ 6 n
  ```
  No sidecar `pre_feed` needed — DECOM and scroll region are set inline in the scenario. This tests the interaction: DECOM set, scroll region 5-20, CUP 3;10 in origin mode places cursor at absolute row 7 (region start 5 + 3 - 1), col 10 (1-based). DSR reports the relative position (3;10).

- [x] Create `oriterm_core/tests/teseq/scenarios/csi/reports/dsr_origin_mode.toml`:
  ```toml
  [expect]
  grid_snapshot = false
  event_snapshot = false
  events = ["PtyWrite"]
  ```

- [x] Add `#[test] fn dsr_origin_mode()` in `csi_reports.rs`:
  ```rust
  #[test]
  fn dsr_origin_mode() {
      if !reseq_available() { eprintln!("reseq not installed, skipping"); return; }
      let (outcome, _) = run_scenario("dsr_origin_mode");
      assert_pty_writes(&outcome, &["\x1b[3;10R"]);
      assert_response_snapshot(&outcome, "csi_reports_dsr_origin_mode");
  }
  ```
  Expected response: `\x1b[3;10R` — cursor position relative to scroll region origin when DECOM is set. Source: `status_device_status()` DSR arg 6 branch with `TermMode::ORIGIN` check.

### Verify DSR scenarios

- [x] Run: `timeout 150 cargo test -p oriterm_core --test teseq -- csi_reports::dsr` — all 4 DSR tests pass. Accept insta snapshots with `INSTA_UPDATE=1` on first run.

- [x] **TPR checkpoint** — `/tpr-review` covering 03.1-03.3 implementation work

---

## 03.4 Private Mode Report Scenarios (DECRQM Private)

**File(s):** `oriterm_core/tests/teseq/scenarios/csi/reports/decrqm_*.teseq` + `*.toml`, `oriterm_core/tests/teseq/csi_reports.rs` (extend)

### DECRQM cursor visibility (DECTCEM, mode 25, default = set)

- [x] Create `oriterm_core/tests/teseq/scenarios/csi/reports/decrqm_dectcem.teseq`:
  ```
  : Esc [ ? 25 $ p
  ```

- [x] Create `oriterm_core/tests/teseq/scenarios/csi/reports/decrqm_dectcem.toml`:
  ```toml
  [expect]
  grid_snapshot = false
  event_snapshot = false
  events = ["PtyWrite"]
  ```

- [x] Add `#[test] fn decrqm_dectcem()` in `csi_reports.rs`:
  ```rust
  #[test]
  fn decrqm_dectcem() {
      if !reseq_available() { eprintln!("reseq not installed, skipping"); return; }
      let (outcome, _) = run_scenario("decrqm_dectcem");
      assert_pty_writes(&outcome, &["\x1b[?25;1$y"]);
      assert_response_snapshot(&outcome, "csi_reports_decrqm_dectcem");
  }
  ```
  Expected response: `\x1b[?25;1$y` (mode 25 = set = cursor visible, the default state).

### DECRQM cursor visibility after reset (DECTCEM, mode 25 = reset)

- [x] Create `oriterm_core/tests/teseq/scenarios/csi/reports/decrqm_dectcem_off.teseq`:
  ```
  : Esc [ ? 25 l
  : Esc [ ? 25 $ p
  ```

- [x] Create `oriterm_core/tests/teseq/scenarios/csi/reports/decrqm_dectcem_off.toml`:
  ```toml
  [expect]
  grid_snapshot = false
  event_snapshot = false
  events = ["PtyWrite"]
  ```

- [x] Add `#[test] fn decrqm_dectcem_off()` in `csi_reports.rs`:
  ```rust
  #[test]
  fn decrqm_dectcem_off() {
      if !reseq_available() { eprintln!("reseq not installed, skipping"); return; }
      let (outcome, _) = run_scenario("decrqm_dectcem_off");
      assert_pty_writes(&outcome, &["\x1b[?25;2$y"]);
      assert_response_snapshot(&outcome, "csi_reports_decrqm_dectcem_off");
  }
  ```
  Expected response: `\x1b[?25;2$y` (mode 25 = reset = cursor hidden).

### DECRQM auto-wrap (DECAWM, mode 7, default = set)

- [x] Create `oriterm_core/tests/teseq/scenarios/csi/reports/decrqm_decawm.teseq`:
  ```
  : Esc [ ? 7 $ p
  ```

- [x] Create `oriterm_core/tests/teseq/scenarios/csi/reports/decrqm_decawm.toml`:
  ```toml
  [expect]
  grid_snapshot = false
  event_snapshot = false
  events = ["PtyWrite"]
  ```

- [x] Add `#[test] fn decrqm_decawm()` in `csi_reports.rs`:
  ```rust
  #[test]
  fn decrqm_decawm() {
      if !reseq_available() { eprintln!("reseq not installed, skipping"); return; }
      let (outcome, _) = run_scenario("decrqm_decawm");
      assert_pty_writes(&outcome, &["\x1b[?7;1$y"]);
      assert_response_snapshot(&outcome, "csi_reports_decrqm_decawm");
  }
  ```
  Expected response: `\x1b[?7;1$y` (mode 7 = set = auto-wrap enabled, which is the default state). Source: `status_report_private_mode()` with `NamedPrivateMode::LineWrap` mapped to `TermMode::LINE_WRAP`.

### DECRQM unrecognized private mode (edge case)

- [x] Create `oriterm_core/tests/teseq/scenarios/csi/reports/decrqm_unknown.teseq`:
  ```
  : Esc [ ? 9999 $ p
  ```

- [x] Create `oriterm_core/tests/teseq/scenarios/csi/reports/decrqm_unknown.toml`:
  ```toml
  [expect]
  grid_snapshot = false
  event_snapshot = false
  events = ["PtyWrite"]
  ```

- [x] Add `#[test] fn decrqm_unknown()` in `csi_reports.rs`:
  ```rust
  #[test]
  fn decrqm_unknown() {
      if !reseq_available() { eprintln!("reseq not installed, skipping"); return; }
      let (outcome, _) = run_scenario("decrqm_unknown");
      assert_pty_writes(&outcome, &["\x1b[?9999;0$y"]);
      assert_response_snapshot(&outcome, "csi_reports_decrqm_unknown");
  }
  ```
  Expected response: `\x1b[?9999;0$y` (mode 9999 = not recognized, DECRPM value 0). Source: `status_report_private_mode()` handles `PrivateMode::Unknown(n)` by returning value 0. This edge case verifies the fallback path.

### Verify DECRQM private scenarios

- [x] Run: `timeout 150 cargo test -p oriterm_core --test teseq -- csi_reports::decrqm` — all 4 DECRQM private tests pass. Accept insta snapshots with `INSTA_UPDATE=1` on first run.

---

## 03.5 ANSI Mode Report Scenarios (DECRQM ANSI)

**File(s):** `oriterm_core/tests/teseq/scenarios/csi/reports/ansi_mode_*.teseq` + `*.toml`, `oriterm_core/tests/teseq/csi_reports.rs` (extend)

ori_term implements `status_report_mode()` for ANSI (non-private) mode queries. The response format is `\x1b[{mode};{value}$y` (no `?` prefix, unlike private mode reports). This covers IRM (insert/replace mode 4) and LNM (line feed/new line mode 20).

### ANSI mode IRM default (mode 4, default = reset)

- [x] Create `oriterm_core/tests/teseq/scenarios/csi/reports/ansi_mode_irm_default.teseq`:
  ```
  : Esc [ 4 $ p
  ```

- [x] Create `oriterm_core/tests/teseq/scenarios/csi/reports/ansi_mode_irm_default.toml`:
  ```toml
  [expect]
  grid_snapshot = false
  event_snapshot = false
  events = ["PtyWrite"]
  ```

- [x] Add `#[test] fn ansi_mode_irm_default()` in `csi_reports.rs`:
  ```rust
  #[test]
  fn ansi_mode_irm_default() {
      if !reseq_available() { eprintln!("reseq not installed, skipping"); return; }
      let (outcome, _) = run_scenario("ansi_mode_irm_default");
      assert_pty_writes(&outcome, &["\x1b[4;2$y"]);
      assert_response_snapshot(&outcome, "csi_reports_ansi_mode_irm_default");
  }
  ```
  Expected response: `\x1b[4;2$y` (mode 4 = reset = replace mode, which is the default state). Source: `status_report_mode()` in `status.rs`, `NamedMode::Insert` branch. Value 2 = reset.

### ANSI mode IRM after enable (mode 4, set)

- [x] Create `oriterm_core/tests/teseq/scenarios/csi/reports/ansi_mode_irm_set.teseq`:
  ```
  : Esc [ 4 h
  : Esc [ 4 $ p
  ```

- [x] Create `oriterm_core/tests/teseq/scenarios/csi/reports/ansi_mode_irm_set.toml`:
  ```toml
  [expect]
  grid_snapshot = false
  event_snapshot = false
  events = ["PtyWrite"]
  ```

- [x] Add `#[test] fn ansi_mode_irm_set()` in `csi_reports.rs`:
  ```rust
  #[test]
  fn ansi_mode_irm_set() {
      if !reseq_available() { eprintln!("reseq not installed, skipping"); return; }
      let (outcome, _) = run_scenario("ansi_mode_irm_set");
      assert_pty_writes(&outcome, &["\x1b[4;1$y"]);
      assert_response_snapshot(&outcome, "csi_reports_ansi_mode_irm_set");
  }
  ```
  Expected response: `\x1b[4;1$y` (mode 4 = set = insert mode active). The first sequence enables IRM via `CSI 4 h`, then the query reports it as set.

### ANSI mode LNM default (mode 20, default = reset)

- [x] Create `oriterm_core/tests/teseq/scenarios/csi/reports/ansi_mode_lnm_default.teseq`:
  ```
  : Esc [ 20 $ p
  ```

- [x] Create `oriterm_core/tests/teseq/scenarios/csi/reports/ansi_mode_lnm_default.toml`:
  ```toml
  [expect]
  grid_snapshot = false
  event_snapshot = false
  events = ["PtyWrite"]
  ```

- [x] Add `#[test] fn ansi_mode_lnm_default()` in `csi_reports.rs`:
  ```rust
  #[test]
  fn ansi_mode_lnm_default() {
      if !reseq_available() { eprintln!("reseq not installed, skipping"); return; }
      let (outcome, _) = run_scenario("ansi_mode_lnm_default");
      assert_pty_writes(&outcome, &["\x1b[20;2$y"]);
      assert_response_snapshot(&outcome, "csi_reports_ansi_mode_lnm_default");
  }
  ```
  Expected response: `\x1b[20;2$y` (mode 20 = reset = line feed mode, which is the default state). Source: `status_report_mode()`, `NamedMode::LineFeedNewLine` branch.

### ANSI mode unknown (edge case)

- [x] Create `oriterm_core/tests/teseq/scenarios/csi/reports/ansi_mode_unknown.teseq`:
  ```
  : Esc [ 99 $ p
  ```

- [x] Create `oriterm_core/tests/teseq/scenarios/csi/reports/ansi_mode_unknown.toml`:
  ```toml
  [expect]
  grid_snapshot = false
  event_snapshot = false
  events = ["PtyWrite"]
  ```

- [x] Add `#[test] fn ansi_mode_unknown()` in `csi_reports.rs`:
  ```rust
  #[test]
  fn ansi_mode_unknown() {
      if !reseq_available() { eprintln!("reseq not installed, skipping"); return; }
      let (outcome, _) = run_scenario("ansi_mode_unknown");
      assert_pty_writes(&outcome, &["\x1b[99;0$y"]);
      assert_response_snapshot(&outcome, "csi_reports_ansi_mode_unknown");
  }
  ```
  Expected response: `\x1b[99;0$y` (mode 99 = not recognized, DECRPM value 0). Source: `status_report_mode()` handles `Mode::Unknown(n)` by returning value 0.

### Verify ANSI mode scenarios

- [x] Run: `timeout 150 cargo test -p oriterm_core --test teseq -- csi_reports::ansi_mode` — all 4 ANSI mode tests pass. Accept insta snapshots with `INSTA_UPDATE=1` on first run.

- [x] **TPR checkpoint** — `/tpr-review` covering 03.4-03.5 implementation work

---

## 03.R Third Party Review Findings

- [x] `[TPR-03-001][medium]` `.github/workflows/auto-release.yml:335` — website roadmap rebuild detection regressed from push-wide to tip-commit-only.
  Resolved: Fixed on 2026-04-05. Changed `fetch-depth: 0` and diff range to `github.event.before..HEAD` to cover multi-commit pushes.
  Evidence: the old `notify-website.yml` triggered on any push touching `plans/roadmap/**`, but the consolidated job now checks only `git diff --name-only HEAD~1` after a `fetch-depth: 2` checkout.
  Impact: a multi-commit push where an earlier commit edits roadmap files but the tip commit does not will skip `oriterm-roadmap-updated`, leaving the website's roadmap content stale.
  Required plan update: diff the full pushed range (`github.event.before..github.sha`) or equivalent push payload data, then re-verify the consolidated notification behavior.
- [x] `[TPR-03-002][low]` `plans/teseq-conformance/section-03-reports.md:4` — Section 03 plan state is internally inconsistent.
  Resolved: Fixed on 2026-04-05. Updated body banner to match frontmatter status.
  Evidence: the frontmatter still says `status: in-progress`, the body still says `**Status:** Not Started`, and the completion checklist claims the frontmatter was already switched to `complete`.
  Impact: downstream readers cannot trust the section status, and the checklist records plan-sync work that the file state does not actually reflect.
  Required plan update: reconcile the frontmatter, body status banner, and completion checklist after TPR resolution so the section advertises one coherent state.
- [x] `[TPR-03-003][low]` `oriterm_core/tests/teseq/harness/assertions.rs:151` — `analyze_response()` treats a failed `teseq` subprocess as success.
  Resolved: Fixed on 2026-04-05. Added `output.status.success()` check, returns `Err` with stderr on non-zero exit.
  Evidence: the helper waits for the child process and returns `Ok(stdout)` without checking `output.status.success()`.
  Impact: when `teseq` exits non-zero, the debug helper can silently return partial or empty output instead of surfacing the failure, which makes response-analysis debugging misleading.
  Required plan update: return an `Err` on non-zero exit status and add coverage for the failing-subprocess path.
- [x] `[TPR-03-004][medium]` `.github/workflows/auto-release.yml:342` — roadmap-only pushes still dispatch the `oriterm-release-published` website event.
  Resolved: Fixed on 2026-04-05. Gated `oriterm-release-published` dispatch on `needs.publish.result == 'success'`; roadmap event remains unconditional within the job.
- [x] `[TPR-03-005][low]` `oriterm_core/tests/teseq/csi_reports.rs:262` / `oriterm_core/tests/teseq/harness/assertions.rs:155` — the non-zero-exit fix in `analyze_response()` is still unpinned by a regression test.
  Resolved: Fixed on 2026-04-05. Extracted `pipe_through_command()` from `analyze_response()` and added `pipe_through_command_returns_err_on_nonzero_exit` test using `false` command.
- [x] `[TPR-03-006][medium]` `oriterm_core/tests/teseq/csi_reports.rs:271` — the new non-zero-exit regression test is Unix-only.
  Resolved: Fixed on 2026-04-05. Added `args` parameter to `pipe_through_command()` and made test cross-platform: `false` on Unix, `cmd /C exit 1` on Windows via `#[cfg(unix)]`/`#[cfg(windows)]` branches.
- [x] `[TPR-03-007][low]` `.github/workflows/auto-release.yml:350` — the roadmap-notify fix still falls back to tip-commit-only detection for zero-before pushes.
  Resolved: Fixed on 2026-04-05. Changed zero-before fallback from `HEAD~1` to `$(git hash-object -t tree /dev/null)` (empty tree), ensuring the full push is diffed on initial or recreated branch pushes.
- [x] `[TPR-03-008][medium]` `.github/workflows/auto-release.yml:331` — `notify-website` now suppresses roadmap rebuilds whenever `publish` fails.
  Resolved: Fixed on 2026-04-05. Removed `publish` result gate from job-level `if` condition. The job now runs after `prepare` succeeds regardless of `publish` outcome. Release event dispatch is already independently gated on `needs.publish.result == 'success'` inside the step.

---

## 03.N Completion Checklist

- [x] Outbound response pipeline: `assert_pty_writes()` (canonical), `assert_response_snapshot()` (hex golden), `analyze_response()` (debug aid) added to `assertions.rs`
- [x] New assertion functions re-exported in `harness/mod.rs`
- [x] `ScenarioSpec` and related types derive `Clone` in `loader.rs` (or `run_scenario` restructured to avoid clone) — chose restructure: `run_scenario` calls `assert_spec` before returning outcome
- [x] `assertions.rs` stays under 500 lines (155 lines)
- [x] `compute_da2_version()` helper computes version from `CARGO_PKG_VERSION` in `csi_reports.rs` (no hardcoded version numbers)
- [x] `da2_version_drift_check` unit test validates `compute_da2_version()` matches production `crate_version_number()` output (no reseq required)
- [x] DA1, DA2, DA3 scenarios created with `.teseq` + `.toml` sidecars + response golden snapshots
- [x] DSR scenarios created: device status (5n), cursor home (6n), cursor moved (6n after CUP), origin mode (6n after DECOM+DECSTBM)
- [x] DECRQM private mode scenarios created: DECTCEM set (default), DECTCEM reset (after CSI ?25l), DECAWM (default), unknown private mode (9999)
- [x] DECRQM ANSI mode scenarios created: IRM default (mode 4), IRM set (after CSI 4h), LNM default (mode 20), unknown ANSI mode (99)
- [x] All response scenarios use `assert_pty_writes` with raw expected bytes (canonical, no teseq dependency)
- [x] All response scenarios also call `assert_response_snapshot` for hex golden comparison (secondary assertion)
- [x] All `.toml` sidecars disable `grid_snapshot` and `event_snapshot` (report scenarios don't test grid state)
- [x] Family module `csi_reports.rs` registered in `main.rs` as `mod csi_reports;`
- [x] `csi_reports.rs` stays under 500 lines (257 lines)
- [x] `scenarios/csi/reports/` directory contains 15 `.teseq` files + 15 `.toml` sidecars
- [x] 16+ report tests pass: DA1, DA2, DA3, DA2 drift check, DSR device status, DSR cursor home, DSR cursor moved, DSR origin mode, DECRQM DECTCEM, DECRQM DECTCEM off, DECRQM DECAWM, DECRQM unknown, ANSI IRM default, ANSI IRM set, ANSI LNM default, ANSI unknown (16 total)
- [x] `./build-all.sh` green, `./clippy-all.sh` green
- [x] `timeout 150 ./test-all.sh` green — no regressions
- [x] Plan annotation cleanup
- [x] All TPR checkpoint findings resolved
- [x] **Plan sync** — update plan metadata:
  - [x] This section's frontmatter `status` → `complete`, subsection statuses updated
  - [x] `00-overview.md` Quick Reference table status updated for this section
  - [x] `00-overview.md` mission success criteria checkboxes updated (check off any now satisfied)
  - [x] `index.md` section status updated
  - [x] Next section's `depends_on` verified — no stale assumptions from this section's work
- [x] `/tpr-review` passed (final, full-section) — clean on iteration 3 (2026-04-05)
- [x] `/impl-hygiene-review last commit` passed — clean (2026-04-05)

**Exit Criteria:** `timeout 150 cargo test -p oriterm_core --test teseq -- csi_reports` passes with 16 report tests (15 teseq-based scenarios + 1 DA2 drift-check unit test). Each teseq scenario validates PtyWrite response bytes via `assert_pty_writes` (canonical raw byte comparison) and `assert_response_snapshot` (hex golden). DA1/DA2/DA3 responses match ori_term's actual response strings in `handler/status.rs`. DSR device status reports OK. DSR cursor position responses encode correct coordinates (including origin mode relative reporting). DECRQM private mode responses report correct DEC private mode states. DECRQM ANSI mode responses report correct ANSI mode states (IRM, LNM, unknown). Teseq analysis is available as a debug aid but is not the oracle. Zero regressions.
