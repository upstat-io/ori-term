---
section: "03"
title: "Reports & Response Validation"
status: not-started
reviewed: false
goal: "Create scenarios that validate outbound terminal responses (DA, DSR, DECRQM) with raw PtyWrite byte assertions as the canonical oracle and optional teseq debug analysis"
success_criteria:
  - "DA1 scenario validates correct PtyWrite response bytes via assert_pty_writes"
  - "DA2 and DA3 scenarios validate correct response bytes"
  - "DSR cursor position report scenario validates correct cursor coordinates in response bytes"
  - "DECRQM scenarios validate mode report response bytes for key modes"
  - "Outbound response pipeline works: assert_pty_writes (canonical), assert_response_snapshot (hex golden), analyze_response (debug aid)"
  - "Satisfies mission criteria: DA1, DA2, DA3, DSR, DECRQM coverage with raw PtyWrite byte assertions"
inspired_by:
  - "ori_term handler/tests.rs — DA/DSR tests via RecordingListener + PtyWrite event capture"
  - "ori_term vttest menu6 (oriterm_core/tests/vttest/menu6.rs) — report validation via PtyResponder"
  - "ori_term status.rs (oriterm_core/src/term/handler/status.rs) — DA/DSR response generation"
depends_on: ["01", "02"]
third_party_review:
  status: none
  updated: null
sections:
  - id: "03.1"
    title: "Outbound Response Assertion Pipeline"
    status: not-started
  - id: "03.2"
    title: "Device Attributes Scenarios (DA1/DA2/DA3)"
    status: not-started
  - id: "03.3"
    title: "Device Status Report Scenarios (DSR)"
    status: not-started
  - id: "03.4"
    title: "Mode Report Scenarios (DECRQM)"
    status: not-started
  - id: "03.R"
    title: "Third Party Review Findings"
    status: not-started
  - id: "03.N"
    title: "Completion Checklist"
    status: not-started
---

# Section 03: Reports & Response Validation

**Status:** Not Started
**Goal:** Validate that ori_term generates correct outbound responses to terminal queries (DA, DSR, DECRQM). This section uses the `RecordedEvent::PtyWrite` payloads from Section 01's harness with raw byte assertions as the canonical oracle (`assert_pty_writes`). Teseq analysis is available as an optional debug aid for human-readable output when tests fail, but is never the golden oracle.

**Success Criteria:**

- [ ] Outbound response assertion pipeline works: `assert_pty_writes` for canonical raw byte checks, `assert_response_snapshot` for hex golden, `analyze_response` for optional debug
- [ ] DA1 response matches expected format (`ESC [ ? 64 ; 6 ; 4 c` — VT420 class with ANSI color + sixel)
- [ ] DA2 response matches expected format
- [ ] DA3 response matches expected format
- [ ] DSR cursor position response encodes correct coordinates
- [ ] DECRQM responses correctly report mode states (set/reset)
- [ ] All response scenarios use `assert_pty_writes` as the canonical assertion (raw bytes, no teseq dependency)

**Context:** Terminal responses are critical for interoperability — programs like vttest, tmux, and SSH clients make decisions based on DA responses. The vttest conformance work (Section 06 of completed plan) already validated DA responses in the PTY context. This section tests the same responses through the teseq harness, adding human-readable teseq analysis as a second validation layer.

**Reference implementations:**
- **ori_term** `handler/status.rs` — `status_identify_terminal()`: DA1/DA2/DA3 response generation
- **ori_term** `handler/status.rs` — `status_device_status()`: DSR cursor position reporting
- **ori_term** `handler/status.rs` — `status_report_private_mode()`: DECRQM response generation
- **ori_term** `handler/tests.rs` — DA/DSR tests capture events via `RecordingListener`

**Depends on:** Section 01 (RecordedEvent with PtyWrite payloads), Section 02 (basic scenario pattern established).

---

## 03.1 Outbound Response Assertion Pipeline

**File(s):** `oriterm_core/tests/teseq/harness/assertions.rs` (extend)

Add response assertion helpers. **Design principle:** Raw PtyWrite bytes are the canonical assertion (via `assert_pty_writes`). Teseq analysis is an optional supplementary debug aid (via `analyze_response`) — never the oracle. This keeps test correctness independent of teseq's output format, which is version-fragile and human-oriented.

- [ ] Implement `assert_pty_writes(outcome: &ScenarioOutcome, expected: &[&str])`:
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

- [ ] Implement `assert_response_snapshot(outcome: &ScenarioOutcome, name: &str)`:
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

- [ ] Implement `analyze_response(response_bytes: &str) -> Result<String, String>` (supplementary debug helper):
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
      child.stdin.take().unwrap()
          .write_all(response_bytes.as_bytes())
          .map_err(|e| format!("failed to write to teseq: {e}"))?;
      // Drop stdin (implicit via take()) to signal EOF to teseq.

      let output = child.wait_with_output()
          .map_err(|e| format!("teseq failed: {e}"))?;

      Ok(String::from_utf8_lossy(&output.stdout).to_string())
  }
  ```

---

## 03.2 Device Attributes Scenarios (DA1/DA2/DA3)

**File(s):** `oriterm_core/tests/teseq/scenarios/csi/reports/*.teseq`, `oriterm_core/tests/teseq/csi_reports.rs`

- [ ] Create DA1 scenario `da1.teseq`:
  ```
  : Esc [ c
  ```
  `da1.toml`:
  ```toml
  [expect]
  events = ["PtyWrite"]
  grid_snapshot = false
  ```
  Expected response: `\x1b[?64;6;4c` (VT420 class, ANSI color, sixel graphics).
  Source: `oriterm_core/src/term/handler/status.rs` in `status_identify_terminal()`, DA1 branch.

- [ ] Create DA2 scenario `da2.teseq`:
  ```
  : Esc [ > c
  ```
  Expected response: `\x1b[>0;{version};1c` where `{version}` is computed from `CARGO_PKG_VERSION` via the same algorithm as `crate_version_number()` in `helpers.rs`. That function is `pub(super)` (not exported), so the test must replicate the version computation: parse `env!("CARGO_PKG_VERSION")`, strip pre-release suffix, split on `.`, reverse, and sum `part * 100^i`. Add a `compute_da2_version()` helper in the test harness (e.g., in `assertions.rs`). Do NOT hardcode a version number — it changes on every release. Format: terminal type 0 (VT100-compatible), version number, conformance level 1.

- [ ] Create DA3 scenario `da3.teseq`:
  ```
  : Esc [ = c
  ```
  Expected response: `\x1bP!|00000000\x1b\\` (DCS response with eight zero digits as unit ID, same as xterm default). Source: `status.rs:142`.

- [ ] Each DA scenario:
  - Validates response bytes via `assert_pty_writes` (canonical — raw byte comparison)
  - Validates response snapshot via `assert_response_snapshot` (secondary — hex golden)
  - Grid snapshot disabled (DA doesn't change visible content)
  - Optional: `analyze_response()` output printed on failure for debug readability

- [ ] Register family module `csi_reports.rs` in `main.rs`

---

## 03.3 Device Status Report Scenarios (DSR)

**File(s):** `oriterm_core/tests/teseq/scenarios/csi/reports/dsr_*.teseq`

- [ ] Create DSR cursor position scenario `dsr_cursor_home.teseq`:
  ```
  : Esc [ 6 n
  ```
  Expected response: `\x1b[1;1R` (cursor at home position, 1-based)

- [ ] Create DSR cursor after movement `dsr_cursor_moved.teseq`:
  ```
  : Esc [ 10 ; 20 H
  : Esc [ 6 n
  ```
  Expected response: `\x1b[10;20R` (cursor at row 10, col 20)

- [ ] Create DSR with origin mode `dsr_origin_mode.teseq`:
  ```
  : Esc [ ? 6 h
  : Esc [ 5 ; 20 r
  : Esc [ 3 ; 10 H
  : Esc [ 6 n
  ```
  No sidecar needed (DECOM and scroll region are set in the scenario itself).
  Expected response: `\x1b[3;10R` — cursor position relative to scroll region origin when DECOM is set. CUP 3;10 in origin mode places cursor at absolute row 7 (region start 5 + 3 - 1), col 10 (1-based). DSR reports relative position (3;10).

- [ ] Each DSR scenario validates response bytes via `assert_pty_writes` (canonical raw byte check). `assert_response_snapshot` provides secondary hex golden comparison.

- [ ] **TPR checkpoint** — `/tpr-review` covering 03.1–03.3 implementation work

---

## 03.4 Mode Report Scenarios (DECRQM)

**File(s):** `oriterm_core/tests/teseq/scenarios/csi/reports/decrqm_*.teseq`

- [ ] Create DECRQM scenario for cursor visibility `decrqm_dectcem.teseq`:
  ```
  : Esc [ ? 25 $ p
  ```
  Expected response: `\x1b[?25;1$y` (mode 25 = set = cursor visible)

- [ ] Create DECRQM after mode change `decrqm_dectcem_off.teseq`:
  ```
  : Esc [ ? 25 l
  : Esc [ ? 25 $ p
  ```
  Expected response: `\x1b[?25;2$y` (mode 25 = reset = cursor hidden)

- [ ] Create DECRQM for auto-wrap `decrqm_decawm.teseq`:
  ```
  : Esc [ ? 7 $ p
  ```
  Expected response: `\x1b[?7;1$y` (mode 7 = set = auto-wrap enabled, which is the default state).

- [ ] Register and verify: `timeout 150 cargo test -p oriterm_core --test teseq -- csi_reports`

---

## 03.R Third Party Review Findings

- None.

---

## 03.N Completion Checklist

- [ ] Outbound response pipeline: `assert_pty_writes()` (canonical), `assert_response_snapshot()` (hex golden), `analyze_response()` (debug aid)
- [ ] `compute_da2_version()` helper computes version from `CARGO_PKG_VERSION` (no hardcoded version numbers)
- [ ] DA1, DA2, DA3 scenarios created with response golden snapshots
- [ ] DSR cursor position scenarios created (home, moved, origin mode)
- [ ] DECRQM scenarios created (DECTCEM set/reset, DECAWM)
- [ ] All response scenarios use `assert_pty_writes` with raw expected bytes (canonical, no teseq dependency)
- [ ] Event snapshots validate PtyWrite events were emitted
- [ ] 10+ report scenarios pass
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

**Exit Criteria:** `timeout 150 cargo test -p oriterm_core --test teseq -- csi_reports` passes with 10+ report scenarios. Each scenario validates PtyWrite response bytes via `assert_pty_writes` (canonical raw byte comparison) and `assert_response_snapshot` (hex golden). DA1/DA2/DA3 responses match ori_term's actual response strings in `handler/status.rs`. DSR responses encode correct cursor coordinates. Teseq analysis is available as a debug aid but is not the oracle. Zero regressions.
