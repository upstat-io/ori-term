---
section: "03"
title: "Reports & Response Validation"
status: not-started
reviewed: false
goal: "Create scenarios that validate outbound terminal responses (DA, DSR, DECRQM) and establish the response analysis pipeline using teseq"
success_criteria:
  - "DA1 scenario captures correct PtyWrite response and validates via teseq analysis"
  - "DA2 and DA3 scenarios capture correct responses"
  - "DSR cursor position report scenario validates correct cursor coordinates in response"
  - "DECRQM scenarios validate mode report responses for key modes"
  - "Outbound teseq analysis pipeline works: capture PtyWrite → pipe through teseq → golden compare"
  - "Satisfies mission criteria: DA1, DA2, DA3, DSR, DECRQM coverage with outbound teseq analysis"
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
    title: "Outbound Teseq Analysis Pipeline"
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
**Goal:** Validate that ori_term generates correct outbound responses to terminal queries (DA, DSR, DECRQM). This section uses the `RecordedEvent::PtyWrite` payloads from Section 01's harness and adds a teseq analysis pipeline to produce human-readable golden files of response bytes.

**Success Criteria:**

- [ ] Outbound teseq analysis helper works: PtyWrite bytes → teseq subprocess → human-readable output
- [ ] DA1 response matches expected format (`ESC [ ? 62 ; 4 ; 6 ; 8 ; 18 ; 22 c`)
- [ ] DA2 response matches expected format
- [ ] DA3 response matches expected format
- [ ] DSR cursor position response encodes correct coordinates
- [ ] DECRQM responses correctly report mode states (set/reset)
- [ ] All response golden files capture teseq-analyzed output for human-readable diffs

**Context:** Terminal responses are critical for interoperability — programs like vttest, tmux, and SSH clients make decisions based on DA responses. The vttest conformance work (Section 06 of completed plan) already validated DA responses in the PTY context. This section tests the same responses through the teseq harness, adding human-readable teseq analysis as a second validation layer.

**Reference implementations:**
- **ori_term** `handler/status.rs:1-60`: DA1 response generation (`\x1b[?62;4;6;8;18;22c`)
- **ori_term** `handler/status.rs:60-120`: DA2 response, DA3 response
- **ori_term** `handler/status.rs:120-200`: DSR cursor position reporting
- **ori_term** `handler/tests.rs` — DA/DSR tests capture events via `RecordingListener`

**Depends on:** Section 01 (RecordedEvent with PtyWrite payloads), Section 02 (basic scenario pattern established).

---

## 03.1 Outbound Teseq Analysis Pipeline

**File(s):** `oriterm_core/tests/teseq/harness/assertions.rs` (extend)

Add a response analysis helper that pipes PtyWrite bytes through `teseq` for human-readable golden comparison.

- [ ] Implement `analyze_response(response_bytes: &str) -> Result<String, String>`:
  ```rust
  /// Pipe response bytes through teseq for human-readable analysis.
  ///
  /// Returns the teseq-annotated output (escape sequence labels, descriptions).
  /// Falls back to hex dump if teseq is unavailable.
  pub fn analyze_response(response_bytes: &str) -> Result<String, String> {
      if !teseq_available() {
          // Fallback: hex dump for CI environments without teseq
          return Ok(format!("hex: {:02x?}", response_bytes.as_bytes()));
      }

      let mut child = std::process::Command::new("teseq")
          .stdin(std::process::Stdio::piped())
          .stdout(std::process::Stdio::piped())
          .stderr(std::process::Stdio::piped())
          .spawn()
          .map_err(|e| format!("failed to spawn teseq: {e}"))?;

      child.stdin.take().unwrap()
          .write_all(response_bytes.as_bytes())
          .map_err(|e| format!("failed to write to teseq: {e}"))?;

      let output = child.wait_with_output()
          .map_err(|e| format!("teseq failed: {e}"))?;

      Ok(String::from_utf8_lossy(&output.stdout).to_string())
  }
  ```

- [ ] Implement `assert_response_snapshot(outcome: &ScenarioOutcome, name: &str)`:
  ```rust
  /// Assert PtyWrite responses match golden teseq analysis.
  pub fn assert_response_snapshot(outcome: &ScenarioOutcome, name: &str) {
      let pty_writes: Vec<_> = outcome.events.iter()
          .filter_map(|e| match e {
              RecordedEvent::PtyWrite(s) => Some(s.as_str()),
              _ => None,
          })
          .collect();

      let mut analysis = String::new();
      for (i, response) in pty_writes.iter().enumerate() {
          if i > 0 { analysis.push_str("---\n"); }
          match analyze_response(response) {
              Ok(text) => analysis.push_str(&text),
              Err(e) => analysis.push_str(&format!("ERROR: {e}\n")),
          }
      }

      insta::assert_snapshot!(format!("{name}_responses"), analysis);
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
  Expected response: `\x1b[?62;4;6;8;18;22c` (VT220 class, columns, reports, etc.)

- [ ] Create DA2 scenario `da2.teseq`:
  ```
  : Esc [ > c
  ```
  Expected response: secondary device attributes (xterm-compatible)

- [ ] Create DA3 scenario `da3.teseq`:
  ```
  : Esc [ = c
  ```
  Expected response: tertiary device attributes

- [ ] Each DA scenario:
  - Validates PtyWrite event was emitted via `assert_event_snapshot`
  - Validates response content via `assert_response_snapshot` (teseq-analyzed golden)
  - Grid snapshot disabled (DA doesn't change visible content)

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
  `dsr_origin_mode.toml`:
  ```toml
  [setup]
  pre_feed = ["\\x1b[?40h"]
  ```
  Expected response: cursor position relative to scroll region origin when DECOM is set.

- [ ] Each DSR scenario validates both the PtyWrite event content and teseq analysis.

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
  Expected response reflecting DECAWM state.

- [ ] Register and verify: `timeout 150 cargo test -p oriterm_core --test teseq -- csi_reports`

---

## 03.R Third Party Review Findings

- None.

---

## 03.N Completion Checklist

- [ ] Outbound teseq analysis pipeline: `analyze_response()` and `assert_response_snapshot()` work
- [ ] DA1, DA2, DA3 scenarios created with response golden snapshots
- [ ] DSR cursor position scenarios created (home, moved, origin mode)
- [ ] DECRQM scenarios created (DECTCEM set/reset, DECAWM)
- [ ] All response golden files show human-readable teseq output
- [ ] Event snapshots validate PtyWrite events were emitted
- [ ] 10+ report scenarios pass
- [ ] `./build-all.sh` green, `./clippy-all.sh` green
- [ ] `./test-all.sh` green — no regressions
- [ ] Plan annotation cleanup
- [ ] All TPR checkpoint findings resolved
- [ ] **Plan sync** — update plan metadata
- [ ] `/tpr-review` passed (final, full-section)
- [ ] `/impl-hygiene-review last commit` passed

**Exit Criteria:** `cargo test -p oriterm_core --test teseq -- csi_reports` passes with 10+ report scenarios. Each scenario validates both the PtyWrite event emission and the teseq-analyzed response content against golden files. DA1/DA2/DA3 responses match ori_term's documented device attributes. DSR responses encode correct cursor coordinates. Zero regressions.
