---
section: "22"
title: "Real-App E2E Harness"
status: not-started
reviewed: false
goal: "Build the real-application PTY recording / replay harness, the snapshot capture pipeline, and the first app smoke test (vim simple session). LANDS EARLY — sibling track to section 21, NOT depending on it. Section 25 (full-pass) depends on this section."
success_criteria:
  - "Audit input committed at `plans/spec-conformance/audits/section-22-top-down-inventory.md`. The audit input is a CORPUS MANIFEST (not an external control-sequence spec — this is an integration section). Every entry in the corpus has a corresponding harness wiring + per-entry pass criterion. `cargo run -p oriterm_test_support --bin spec-coverage-report -- --check audit-files` passes for this audit file (integration-section variant: validates corpus completeness against harness wiring). Section 09A introduced the `audits/` SSOT; this section adapts it for integration scope per `plans/spec-conformance/audits/README.md` integration-section guidance."
  - "`crates/oriterm_test_support/src/real_app_harness/mod.rs` exists with PTY recording + replay infrastructure"
  - "Snapshot capture pipeline: a recorded PTY trace replays through ori_term, the final grid state is captured as a golden snapshot, and the diff is reported on mismatch"
  - "First app smoke test passes: `vim +q` (open and quit) — drives a small set of catalog rows and produces a stable snapshot golden"
  - "Recorded sessions stored under `crates/oriterm_test_support/tests/data/real_app_captures/<app>/<scenario>.cap`"
  - "**Capture environment pinning (REQUIRED)**: every committed capture file is accompanied by a `<scenario>.env.toml` sidecar that pins: `TERM` + terminfo source + sha256, locale (`LC_ALL`/`LANG`), shell (`SHELL`), app binary name + version (output of `<app> --version`), ALL relevant config file paths + sha256 (e.g. `~/.vimrc`, `~/.config/helix/config.toml`, `~/.tmux.conf`), the input data file path + sha256 (the file opened in vim, the log file htop reads, etc.), capture command exact string, OS + kernel, and ISO-8601 timestamp. CI verifies sidecar against runtime before replay; a mismatch is an infrastructure failure with a 'capture drift detected' message, NOT a test failure."
  - "**Non-determinism scrubbing**: capture command string MUST neutralize process-ID, TTY name, timestamp, hostname, and similar runtime-varying fields. The pinning includes a `scrub_rules` list that documents every substitution applied during capture (e.g. `sed 's/pid:[0-9]+/pid:REDACTED/'`) so reviewers can audit that the capture's determinism is load-bearing."
  - "**Section is `complete` when vim smoke test passes**, NOT when all real apps are tested — section 25 owns the full-pass milestone for vim/htop/btop/tmux/aerc/helix/ncmpcpp/less/nvim"
  - "Recording instructions documented: how a developer captures a real-app PTY session for the test corpus (using `script` or `ttyrec`)"
  - "All existing tests pass without modification"
  - "`./build-all.sh`, `./test-all.sh`, `./clippy-all.sh` green debug + release"
  - "Section's mission criterion connection: contributes to **Real-app E2E milestones pass** (delivers the harness; section 25 delivers the full-pass)"
inspired_by:
  - "tack scenario framework (section 21 references this) — same PTY-driven scenario pattern"
  - "ttyrec / script utilities — standard PTY recording tools"
  - "asciinema cast format — alternative recording format if `script` doesn't capture timing"
depends_on: ["04"]
third_party_review:
  status: none
  updated: null
sections:
  - id: "22.0"
    title: "Audit input verification (BLOCKING) — commit audits/section-22-top-down-inventory.md (corpus manifest of real-app sessions)"
    status: not-started
  - id: "22.1"
    title: "Build PTY recording + replay infrastructure for real apps"
    status: not-started
  - id: "22.2"
    title: "Build snapshot capture + diff pipeline"
    status: not-started
  - id: "22.3"
    title: "Document recording instructions for the test corpus"
    status: not-started
  - id: "22.4"
    title: "Land vim simple session smoke test"
    status: not-started
  - id: "22.R"
    title: "Third Party Review Findings"
    status: not-started
  - id: "22.N"
    title: "Completion Checklist"
    status: not-started
---

# Section 22: Real-App E2E Harness

**Status:** Not Started
**Goal:** Build the real-application PTY replay harness. Sibling track to section 21 — both can land in any order. Section 25 (full-pass) builds on this section's harness.

**Success Criteria:** see frontmatter.

**Context:** Real-application testing complements notcurses-demo: notcurses is adversarial (it stresses every protocol seam in combination), whereas real apps are representative (they exercise what users actually use). The harness pattern is similar to section 21's notcurses harness — capture once, replay deterministically.

**Reference implementations:** see frontmatter.

**Depends on:** Section 04 (verification chain harness exists).

---

## 22.0 Audit input verification (BLOCKING — precedes all other subsections)

**Goal:** Verify the audit-input corpus manifest at `plans/spec-conformance/audits/section-22-top-down-inventory.md` is populated and that every entry has corresponding harness wiring + per-entry pass criterion.

**Integration-section scope:** This section is NOT a protocol-stack section — it does not walk a control-sequence spec source. Its "audit input" is a CORPUS MANIFEST: real-app session corpus manifest (vim, htop, btop, tmux, aerc, helix, ncmpcpp, less, nvim, etc.). The `audits/` SSOT introduced by Section 09A (per `plans/spec-conformance/audits/README.md`) adapts to integration sections by treating the corpus manifest as the top-down enumerator. The completeness check is: every corpus entry has harness wiring + a per-entry pass criterion.

**Why this exists:** Section 09A closed the bottom-up catalog gap that hid DECRQCRA via the per-section audit file pattern. Integration sections inherit the same enforcement shape — the audit file IS the corpus manifest, and `spec-coverage-report --check audit-files` validates that every entry has the required wiring (not catalog-row mapping, since integration sections don't add catalog rows).

**Files touched:**
- `plans/spec-conformance/audits/section-22-top-down-inventory.md` (NEW — stub created by Section 09A's §09A.10; populated by this subsection)

**Completion criteria:**

- [ ] Audit file is populated with every entry in the corpus manifest.
- [ ] Every entry has a `harness_wiring` reference (file path + test name) + a `pass_criterion` description.
- [ ] `cargo run -p oriterm_test_support --bin spec-coverage-report -- --check audit-files` passes (integration-section variant — validates corpus completeness, not catalog-row mapping).
- [ ] Audit file `last_walked` and `walked_by` set.

**No other subsection in this section can begin work until §22.0 is complete.**

---

## 22.1 Build PTY recording + replay infrastructure

**File(s):** `crates/oriterm_test_support/src/real_app_harness/mod.rs` (new), `crates/oriterm_test_support/src/real_app_harness/replay.rs` (new), sibling tests

- [ ] Implement `replay_real_app_capture(app: &str, scenario: &str, ori_term_session: &mut SpecHarness)` that reads the committed capture file and feeds it through the spec_chain harness
- [ ] Capture file format: raw PTY byte stream (the same format `script -c '...' /tmp/cap` produces). No timing info needed for snapshot-based testing — the final state is what matters, not the per-frame state.
- [ ] If timing matters (e.g., for animation testing), use `ttyrec` or `asciinema cast` format and replay with timing simulation. Defer this to section 25 if needed.
- [ ] Sibling tests
- [ ] **Validation**: replay infrastructure works for a sample capture.

---

## 22.2 Build snapshot capture + diff pipeline

**File(s):** `crates/oriterm_test_support/src/real_app_harness/snapshot.rs` (new)

- [ ] After replay, capture the final grid state as a snapshot (text-based, not pixel-based — real-app tests are about correctness of cell content, not pixel-perfect rendering)
- [ ] Snapshot format: insta-style grid_text (similar to teseq scenarios)
- [ ] Use insta for diffing: `assert_snapshot!(grid_text)`
- [ ] Optional: also capture a pixel golden for tests that care about visual regression (e.g., syntax highlighting)
- [ ] **Validation**: snapshot capture + diff works.

---

## 22.3 Document recording instructions for the test corpus (with environment pinning)

**File(s):** `crates/oriterm_test_support/tests/data/real_app_captures/README.md` (new), `crates/oriterm_test_support/src/real_app_harness/env_pin.rs` (new)

Real-app captures depend on terminfo, locale, shell, app version, app config files, input data files, and host OS. If any drift, a passing test silently becomes a stale-baseline test. Every committed capture ships with a sidecar TOML pinning these inputs; CI verifies before replay.

- [ ] Define `RealAppEnvPin` struct in `real_app_harness/env_pin.rs`:
  ```rust
  pub struct RealAppEnvPin {
      pub term: String,
      pub colorterm: Option<String>,
      pub terminfo_source: String,
      pub terminfo_sha256: String,
      pub lc_all: String,
      pub lang: String,
      pub shell: String,                           // e.g. "/bin/bash"
      pub shell_version: String,                   // `$SHELL --version`
      pub app_name: String,                        // e.g. "vim"
      pub app_version: String,                     // `<app> --version`
      pub app_binary_path: String,                 // `which <app>`
      pub config_files: Vec<ConfigFilePin>,        // path + sha256 per config
      pub input_data_files: Vec<InputDataFilePin>, // path + sha256 per input
      pub capture_command: String,                 // exact script -c argument
      pub scrub_rules: Vec<String>,                // sed/regex substitutions applied
      pub host_os: String,
      pub host_kernel: String,
      pub captured_at: String,                     // ISO-8601
  }
  pub struct ConfigFilePin { pub path: String, pub sha256: String }
  pub struct InputDataFilePin { pub path: String, pub sha256: String }

  impl RealAppEnvPin {
      pub fn capture_current(app: &str, config_files: &[&Path], input_files: &[&Path]) -> Result<Self, PinError>;
      pub fn load_sidecar(capture_path: &Path) -> Result<Self, PinError>;
      pub fn verify_against_runtime(&self) -> Result<(), PinMismatch>;
  }
  ```
- [ ] Document how to capture a real-app session in `README.md`:
  - Set `TERM=ori_term` (use the pinned terminfo from tack-conformance section 02)
  - Freeze the locale: `export LC_ALL=C.UTF-8 LANG=C.UTF-8`
  - Use the minimal-config flag when the app supports it (e.g. `vim -u <fixed-config>`, `tmux -f <fixed-config>`, `helix --config <fixed-config>`). Commit the fixed config alongside the capture sidecar.
  - Run `script -c '<command>' /tmp/cap`
  - Verify the capture is deterministic (re-run the command, diff the captures, fix any non-determinism by passing flags to the app OR adding a scrub rule)
  - Run `cargo xtask pin-real-app-capture <app> <scenario>` to generate the sidecar TOML with all pinned fields AND sha256 hashes of config/input files
  - Commit the capture as `<app>/<scenario>.cap` + `<app>/<scenario>.env.toml`
- [ ] Document standard scenarios for each app (vim simple session, htop snapshot, helix edit + save, tmux split + swap + detach, etc.) in `README.md` — each row includes: app, scenario name, fixed config path, input data path, expected duration, and which catalog stacks it exercises
- [ ] **Validation**: README is clear enough that a developer can capture a new scenario without asking; sidecar generation command produces valid TOML that round-trips.

---

## 22.4 Land vim simple session smoke test

**File(s):** `oriterm_core/tests/real_app/vim_simple.rs` (new), `crates/oriterm_test_support/tests/data/real_app_captures/vim/simple_session.cap` (committed), `crates/oriterm_test_support/tests/data/real_app_captures/vim/simple_session.env.toml` (committed), `crates/oriterm_test_support/tests/data/real_app_captures/vim/fixtures/vimrc` (committed), `crates/oriterm_test_support/tests/data/real_app_captures/vim/fixtures/input.txt` (committed), `crates/oriterm_test_support/tests/references/real_app/vim/simple_session.snap` (committed)

**Scenario strength**: a bare `vim +q` only exercises startup + termination — not enough catalog rows to make the smoke test meaningful. This subsection captures a scenario that hits cursor movement, SGR colors, scroll regions, line drawing, and the alternate screen — the five things that MUST work for any real vim session.

- [ ] Commit a fixed vimrc under `crates/oriterm_test_support/tests/data/real_app_captures/vim/fixtures/vimrc` with: `set nu ruler laststatus=2 background=dark syntax on nocompatible`
- [ ] Commit a fixed input file under `.../vim/fixtures/input.txt` — a small plain-text file (~30 lines) with varied characters (ASCII, UTF-8, tabs, trailing whitespace) so syntax highlighting runs
- [ ] Capture: `script -c "vim -u <path-to-fixtures>/vimrc -n <path-to-fixtures>/input.txt -c 'normal! G' -c 'normal! gg' -c '/main' -c 'q'" /tmp/vim_simple.cap` — opens the file with the fixed vimrc, jumps to end, back to top, searches for "main", quits
- [ ] Run `cargo xtask pin-real-app-capture vim simple_session` to generate the sidecar TOML pinning the fixtures + vim binary version + all the fields in `RealAppEnvPin`
- [ ] Verify the capture is deterministic (re-run twice — diff the output bytes); add scrub rules for any non-determinism (timestamps, process IDs) to the pin's `scrub_rules` field
- [ ] Commit the capture, sidecar, and fixtures
- [ ] Spec_chain test that loads the sidecar, verifies the environment, replays the capture, asserts the final `grid_text` + SGR state matches the committed snapshot
- [ ] The snapshot must cover at LEAST: cursor at the `main` search result line, line numbers visible, SGR colors from syntax highlighting, status line at the bottom with ruler info
- [ ] **Validation**: test passes; back-to-back runs produce identical snapshots; the snapshot verifiably exercises multiple catalog rows (list them in a test comment block).

---

## 22.R Third Party Review Findings

- None.

---

## 22.N Completion Checklist

- [ ] Failing test matrix written FIRST
- [ ] **Matrix dimensions**: app × scenario × snapshot type (text/pixel)
- [ ] **Semantic pin**: vim simple session test
- [ ] Real-app PTY recording + replay infrastructure exists
- [ ] Snapshot capture + diff pipeline exists
- [ ] Recording instructions documented
- [ ] vim simple session smoke test passes
- [ ] All existing tests pass
- [ ] Alloc regression unchanged
- [ ] `./build-all.sh`, `./test-all.sh`, `./clippy-all.sh` green
- [ ] Plan annotation cleanup
- [ ] Section frontmatter `status` → `complete`
- [ ] `00-overview.md` + `index.md` updated
- [ ] `/tpr-review` passed
- [ ] `/impl-hygiene-review last commit` passed (after `/tpr-review` is clean)

**Exit Criteria:** Real-app harness exists; vim smoke test passes; section 25 has the scaffolding to add the remaining apps incrementally.
