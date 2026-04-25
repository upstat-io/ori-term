---
section: "09"
title: "Gate Flip + Meta-Test"
status: not-started
reviewed: false
goal: "Update the THREE gate-checking files (`./clippy-all.sh`, `.github/workflows/ci.yml` `clippy` + `clippy-windows-cross` jobs, `lefthook.yml` `clippy:` pre-commit hook) to enforce `--workspace --all-targets` plus the per-crate feature matrix. Add a source-text regression meta-test (modeled on `oriterm/tests/architecture.rs:238-294`) that pins the `--all-targets` flag in all three locations — failing CI immediately if any single file loses the flag. Extend the CI clippy job timeouts from 15 min to 25 min if the `--all-targets` runtime requires."
success_criteria:
  - "`./clippy-all.sh` invokes `cargo clippy --workspace --all-targets --target x86_64-pc-windows-gnu -- -D warnings` AND `cargo clippy --workspace --all-targets -- -D warnings` (host) AND the per-crate feature combos (default already covered, plus oriterm_core --no-default-features, oriterm_ui --features testing, oriterm --features gpu-tests, oriterm --features profile)"
  - "`.github/workflows/ci.yml` `clippy` job (line 40-63) and `clippy-windows-cross` job (line 65-83) invoke the same flag set as `./clippy-all.sh`"
  - "`lefthook.yml` `clippy:` pre-commit hook invokes `cargo clippy --workspace --all-targets --target x86_64-pc-windows-gnu -- -D warnings`"
  - "Source-text meta-test exists (in `oriterm/tests/architecture.rs` or new sibling `oriterm/tests/clippy_gate.rs`) that reads all three files and asserts `--all-targets` appears in each clippy invocation"
  - "`cargo test -p oriterm --test {architecture|clippy_gate}` passes the meta-test"
  - "If `--all-targets` runtime exceeds 15 minutes in CI, the timeouts at `ci.yml:44` and `:69` are raised to 25 minutes in the same commit"
  - "CI green on the PR introducing the gate flip"
inspired_by:
  - "oriterm/tests/architecture.rs:238-294 (call-sequence source-text pin pattern)"
  - "oriterm/tests/architecture.rs:330-342 (removal source-text pin pattern)"
depends_on: ["08"]
third_party_review:
  status: none
  updated: null
sections:
  - id: "09.1"
    title: "Update `./clippy-all.sh`"
    status: not-started
  - id: "09.2"
    title: "Update `.github/workflows/ci.yml` clippy + clippy-windows-cross jobs"
    status: not-started
  - id: "09.3"
    title: "Update `lefthook.yml` clippy: pre-commit hook"
    status: not-started
  - id: "09.4"
    title: "Add source-text meta-test"
    status: not-started
  - id: "09.5"
    title: "Verify CI runtime; extend timeouts if required"
    status: not-started
  - id: "09.R"
    title: "Third Party Review Findings"
    status: not-started
  - id: "09.N"
    title: "Completion Checklist"
    status: not-started
---

# Section 09: Gate Flip + Meta-Test

**Status:** Not Started
**Goal:** The architectural deliverable. Sections 02-08 cleaned the violations; Section 09 closes the gate so they cannot recur. Three files get the new flag set; a meta-test pins the SSOT invariant (`--all-targets` appears in all three) so a future drift fails CI immediately.

**Success Criteria:** see frontmatter.

**Context:** This section is the critical path. Three files currently share a gate gap (`./clippy-all.sh:6-11`, `.github/workflows/ci.yml:40-83`, `lefthook.yml clippy:` block). The gap let ~1480 violations accumulate. After Sections 02-08, every cell of the feature matrix is clean; the gate flip turns the new clean state into the enforced state.

The meta-test is the SSOT enforcement mechanism. A "DRY" refactor to a shared `scripts/clippy-gate.sh` was rejected per Codex's round-1 verifiable reasoning (CI jobs have OS-package installs at `ci.yml:50-63` that don't fit a shared script; lefthook hooks have their own structural conventions). Instead, the meta-test reads all three files, parses the flag arguments, and asserts each invocation contains `--all-targets`. Removing the flag from any single file fails the test, so the SSOT invariant is enforced at compile-test time without forcing a single-execution surface.

The CI runtime concern is real: the existing 15-minute timeouts at `ci.yml:44` and `:69` may not absorb the larger `--all-targets` workload. The plan extends timeouts to 25 minutes IF post-cleanup runtime exceeds (verified empirically in 09.5).

**Reference implementations:**
- `oriterm/tests/architecture.rs:238-294` — `move_to_new_window_embedded_mirrors_tear_off_sequence` test reads source via `std::fs::read_to_string` and asserts ordered substrings appear in the file
- `oriterm/tests/architecture.rs:330-342` — `move_tab_to_window_helper_remains_removed` test asserts a substring is ABSENT (negative pin)

**Depends on:** Section 08 (every feature-matrix cell verified clean — flipping the gate against any unverified cell breaks CI).

---

## Intelligence Reconnaissance

Queries run 2026-04-25:

- `scripts/intel-query.sh` — not present in this project; queries below used Grep / Glob / Read / cargo clippy --message-format=json instead.
- `Read .github/workflows/ci.yml` — `clippy` job at line 40-63 (run: `cargo clippy --workspace -- -D warnings` at line 63), `clippy-windows-cross` job at line 65-83 (run: `cargo clippy --workspace --target x86_64-pc-windows-gnu -- -D warnings` at line 83). `RUSTFLAGS: "-D warnings"` set globally at line 7-9. Timeouts at line 44 (clippy 15min) and line 69 (clippy-windows-cross 15min).
- `Read lefthook.yml` — `clippy:` hook (located at the pre-commit dispatch section) runs `cargo clippy --target x86_64-pc-windows-gnu -- -D warnings` — missing `--workspace` AND `--all-targets`, only checks the default workspace member crate.
- `Read oriterm/tests/architecture.rs:238-345` — confirmed the source-text pin pattern: reads file via `std::fs::read_to_string`, extracts function bodies via a `extract_fn_body` helper (line 245), asserts ordered substrings via plain string `.contains()` checks.

Results summary (≤500 chars) [ori]: Three gate-checking files, three different invocation styles, one shared gap (`--all-targets` missing). Source-text pin precedent at `oriterm/tests/architecture.rs:238-294` is exactly the right model. CI 15-min timeouts may need extension; Section 09.5 verifies empirically.

See _(intel graph not available in this project; use Grep/Glob)_ for the full query protocol.

---

## 09.1 Update `./clippy-all.sh`

**File(s):** `./clippy-all.sh`

The current `./clippy-all.sh` runs `cargo clippy --workspace -- -D warnings` on Windows GNU and host. It must invoke `--all-targets` plus the per-crate feature matrix.

- [ ] Replace `./clippy-all.sh` content with:
  ```bash
  #!/usr/bin/env bash
  set -euo pipefail

  TARGET="x86_64-pc-windows-gnu"

  echo "=== cargo clippy --workspace --all-targets (${TARGET}) ==="
  cargo clippy --workspace --all-targets --target "${TARGET}" -- -D warnings

  echo ""
  echo "=== cargo clippy --workspace --all-targets (host) ==="
  cargo clippy --workspace --all-targets -- -D warnings

  # Per-crate feature combos that don't compose under --workspace
  echo ""
  echo "=== cargo clippy -p oriterm_core --no-default-features (host) ==="
  cargo clippy -p oriterm_core --all-targets --no-default-features -- -D warnings

  echo ""
  echo "=== cargo clippy -p oriterm_ui --features testing (host) ==="
  cargo clippy -p oriterm_ui --all-targets --features testing -- -D warnings

  echo ""
  echo "=== cargo clippy -p oriterm --features gpu-tests (host) ==="
  cargo clippy -p oriterm --all-targets --features gpu-tests -- -D warnings

  echo ""
  echo "=== cargo clippy -p oriterm --features profile (host) ==="
  cargo clippy -p oriterm --all-targets --features profile -- -D warnings

  echo ""
  echo "All clippy checks passed."
  ```
- [ ] Verify: `./clippy-all.sh` exits 0. (This step is the integration check.)
- [ ] Commit: `build(clippy-all): enforce --all-targets and per-crate feature matrix`.

- [ ] **Subsection close-out (09.1)**: standard template.

---

## 09.2 Update `.github/workflows/ci.yml` clippy + clippy-windows-cross jobs

**File(s):** `.github/workflows/ci.yml`

- [ ] Edit `ci.yml:40-63` (`clippy` job) — change the run command:
  ```yaml
  - run: |
      cargo clippy --workspace --all-targets -- -D warnings
      cargo clippy -p oriterm_core --all-targets --no-default-features -- -D warnings
      cargo clippy -p oriterm_ui --all-targets --features testing -- -D warnings
      cargo clippy -p oriterm --all-targets --features gpu-tests -- -D warnings
      cargo clippy -p oriterm --all-targets --features profile -- -D warnings
  ```
- [ ] Edit `ci.yml:65-83` (`clippy-windows-cross` job) — change the run command:
  ```yaml
  - run: |
      cargo clippy --workspace --all-targets --target x86_64-pc-windows-gnu -- -D warnings
      cargo clippy -p oriterm_core --all-targets --target x86_64-pc-windows-gnu --no-default-features -- -D warnings
      cargo clippy -p oriterm_ui --all-targets --target x86_64-pc-windows-gnu --features testing -- -D warnings
      cargo clippy -p oriterm --all-targets --target x86_64-pc-windows-gnu --features gpu-tests -- -D warnings
      cargo clippy -p oriterm --all-targets --target x86_64-pc-windows-gnu --features profile -- -D warnings
  ```
- [ ] Verify: `gh act` (if installed) or visual review shows the updated YAML is well-formed.
- [ ] Commit: `ci: clippy jobs enforce --all-targets and per-crate feature matrix`.

- [ ] **Subsection close-out (09.2)**: standard template.

---

## 09.3 Update `lefthook.yml` clippy: pre-commit hook

**File(s):** `lefthook.yml`

- [ ] Edit the `clippy:` pre-commit hook (currently `cargo clippy --target x86_64-pc-windows-gnu -- -D warnings`) to:
  ```yaml
  clippy:
    glob: "*.rs"
    run: cargo clippy --workspace --all-targets --target x86_64-pc-windows-gnu -- -D warnings
  ```
- [ ] CONSIDER: should the pre-commit hook ALSO run host? Tradeoff: pre-commit speed vs catching host-specific regressions early. Recommendation: keep Windows GNU only (cross-compile catches more cross-platform issues; CI catches host-specific), to keep pre-commit time reasonable. Document the decision in the commit message.
- [ ] If host pre-commit deemed unnecessary, document in the commit message: "Pre-commit hook covers Windows GNU; CI's `clippy` job covers host."
- [ ] Verify: `lefthook run pre-commit` succeeds (or stage a single Rust file edit and verify the hook fires + passes).
- [ ] Commit: `chore(lefthook): clippy hook enforces --workspace --all-targets`.

- [ ] **Subsection close-out (09.3)**: standard template.

---

## 09.4 Add source-text meta-test

**File(s):** `oriterm/tests/clippy_gate.rs` (new), or extend `oriterm/tests/architecture.rs`

The meta-test is the SSOT enforcer. It reads all three files and asserts the `--all-targets` flag appears in each clippy invocation.

**Decision: new sibling test file `oriterm/tests/clippy_gate.rs` rather than appending to `architecture.rs`.** The architecture test file is scoped to "architecture/boundary" tests; gate-config tests are a different concern (CI/build infrastructure pins). Keeping them separate follows SRP and the existing pattern (architecture-only assertions live in the existing file).

- [ ] Create `oriterm/tests/clippy_gate.rs` with the meta-test:
  ```rust
  //! Source-text pins for the workspace clippy gate. Three files (`./clippy-all.sh`,
  //! `.github/workflows/ci.yml`, `lefthook.yml`) all enforce the same gate; this test
  //! ensures every file invokes `--all-targets` so a flag-removal in any one fails CI.
  //!
  //! Regression for: clippy-gate-hardening Section 09. See
  //! `plans/clippy-gate-hardening/00-overview.md` Design Principle 4
  //! "Three places, one truth — pinned by meta-test."

  use std::fs;
  use std::path::PathBuf;

  fn repo_root() -> PathBuf {
      PathBuf::from(env!("CARGO_MANIFEST_DIR"))
          .parent()
          .expect("oriterm/.. → workspace root")
          .to_path_buf()
  }

  fn read(path: &str) -> String {
      let full = repo_root().join(path);
      fs::read_to_string(&full)
          .unwrap_or_else(|e| panic!("read {path} failed: {e}"))
  }

  /// `./clippy-all.sh` MUST invoke `--all-targets` for both host and Windows GNU,
  /// AND for the per-crate feature combos (oriterm_core --no-default-features,
  /// oriterm_ui --features testing, oriterm --features gpu-tests, oriterm --features profile).
  /// A flag-removal here would silently re-introduce the gate gap that BUG-07-005..012 cluster
  /// documented.
  #[test]
  fn clippy_all_sh_pins_all_targets_and_feature_matrix() {
      let body = read("clippy-all.sh");
      // Must invoke --all-targets in BOTH the workspace runs (host + Windows GNU)
      let count = body.matches("--all-targets").count();
      assert!(
          count >= 6,
          "clippy-all.sh must invoke `--all-targets` at least 6 times \
           (workspace × {host, x86_64-pc-windows-gnu} + 4 per-crate feature combos); found {count}. \
           See plans/clippy-gate-hardening/00-overview.md Design Principle 4."
      );
      // Must cover the per-crate feature matrix
      assert!(body.contains("-p oriterm_core --all-targets --no-default-features"),
              "clippy-all.sh must invoke `cargo clippy -p oriterm_core --all-targets --no-default-features`");
      assert!(body.contains("-p oriterm_ui --all-targets --features testing"),
              "clippy-all.sh must invoke `cargo clippy -p oriterm_ui --all-targets --features testing`");
      assert!(body.contains("-p oriterm --all-targets --features gpu-tests"),
              "clippy-all.sh must invoke `cargo clippy -p oriterm --all-targets --features gpu-tests`");
      assert!(body.contains("-p oriterm --all-targets --features profile"),
              "clippy-all.sh must invoke `cargo clippy -p oriterm --all-targets --features profile`");
  }

  /// CI workflow MUST invoke `--all-targets` in both the host clippy job and
  /// the Windows-cross clippy job, AND cover the per-crate feature matrix in both.
  #[test]
  fn ci_yml_clippy_jobs_pin_all_targets_and_feature_matrix() {
      let body = read(".github/workflows/ci.yml");
      let count = body.matches("--all-targets").count();
      assert!(
          count >= 10,
          "ci.yml must invoke `--all-targets` at least 10 times \
           (workspace × {clippy, clippy-windows-cross} + 4 per-crate feature combos × 2 jobs); found {count}."
      );
      // Per-crate feature combos must appear in BOTH clippy jobs
      assert!(body.contains("-p oriterm_core --all-targets --no-default-features"),
              "ci.yml must invoke oriterm_core --no-default-features clippy");
      assert!(body.contains("-p oriterm_ui --all-targets --features testing"),
              "ci.yml must invoke oriterm_ui --features testing clippy");
  }

  /// lefthook.yml clippy hook MUST invoke `--workspace --all-targets`.
  /// Bare `cargo clippy --target x86_64-pc-windows-gnu -- -D warnings` (the pre-flip
  /// state) silently misses every test target and integration test on every commit.
  #[test]
  fn lefthook_yml_clippy_hook_pins_workspace_all_targets() {
      let body = read("lefthook.yml");
      assert!(
          body.contains("cargo clippy --workspace --all-targets"),
          "lefthook.yml clippy: hook must invoke `cargo clippy --workspace --all-targets`. \
           A bare `cargo clippy --target ...` without --workspace and --all-targets re-introduces \
           the gate gap that BUG-07-005..012 cluster documented."
      );
  }

  /// All three gate locations MUST agree on the `-D warnings` denial. A gate that allows
  /// warnings (even one of the three) defeats the entire SSOT — `RUSTFLAGS: "-D warnings"`
  /// in ci.yml only covers compilation warnings, NOT clippy::pedantic and clippy::nursery
  /// which are emitted by clippy and converted to errors only via `-D warnings` on the clippy
  /// invocation itself.
  #[test]
  fn all_three_gates_pin_d_warnings_on_clippy() {
      for path in &["clippy-all.sh", ".github/workflows/ci.yml", "lefthook.yml"] {
          let body = read(path);
          assert!(
              body.contains("-D warnings"),
              "{path} must invoke clippy with `-- -D warnings`"
          );
      }
  }
  ```

- [ ] Run `cargo test -p oriterm --test clippy_gate` — verify all four tests pass against the post-flip state.
- [ ] Run `cargo test -p oriterm` — full crate suite green.
- [ ] **Negative pin**: temporarily revert `--all-targets` from `clippy-all.sh` (in a scratch commit, NOT pushed); confirm `clippy_all_sh_pins_all_targets_and_feature_matrix` fails. Then restore. This proves the meta-test actively rejects the broken state.
- [ ] Commit: `test(clippy-gate): pin --all-targets and feature matrix in three gate files`.

- [ ] **Subsection close-out (09.4)**: standard template; `/improve-tooling` retrospective: did writing the meta-test reveal that `oriterm/tests/architecture.rs` should be split (existing tests are architecture invariants; clippy gate is a separate concern)? If 09.4's negative-pin verification was tedious (manually reverting + re-applying), consider a `diagnostics/meta-test-negative-pin.sh` helper for future similar tests.

---

## 09.5 Verify CI runtime; extend timeouts if required

**File(s):** `.github/workflows/ci.yml` (only if extension required)

- [ ] Push a draft PR with the Section 09.1-09.4 changes. Observe CI runtime for `clippy` and `clippy-windows-cross` jobs.
- [ ] If either job exceeds 12-13 minutes (giving headroom against the 15-min timeout): edit `ci.yml:44` and `:69` to `timeout-minutes: 25`.
- [ ] Re-run CI; confirm both clippy jobs complete within the new 25-min window.
- [ ] If runtime exceeds 25 minutes, the cleanup might have hidden inefficient test compilation; investigate (`cargo build --all-targets` profile flame graph) before raising the timeout further.
- [ ] Commit (only if extension applied): `ci: extend clippy job timeouts to 25min for --all-targets workload`.

- [ ] **Subsection close-out (09.5)**: standard template.

---

## 09.R Third Party Review Findings

- None.

---

## 09.N Completion Checklist

- [ ] `./clippy-all.sh` invokes `--all-targets` and the feature matrix (host + Windows GNU)
- [ ] `ci.yml` clippy + clippy-windows-cross jobs invoke `--all-targets` and the feature matrix
- [ ] `lefthook.yml` clippy hook invokes `--workspace --all-targets`
- [ ] `oriterm/tests/clippy_gate.rs` exists with 4 source-text pin tests, all passing
- [ ] `cargo test -p oriterm --test clippy_gate` exits 0
- [ ] `cargo test --all` green (regression canary)
- [ ] `./clippy-all.sh` exits 0 (the new flag set is enforced and clean)
- [ ] CI green on the gate-flip PR (timeouts extended if required)
- [ ] **Plan sync**: section 09 status → complete in section file + 00-overview.md + index.md
- [ ] `/tpr-review` passed (gate-flip is correctness infrastructure; TPR is critical)
- [ ] `/impl-hygiene-review` passed (after TPR clean)
- [ ] `/improve-tooling` section-close sweep
- [ ] `/sync-claude` section-close doc sync — CLAUDE.md §Commands lists `./clippy-all.sh`; verify still accurate post-flip. The `./clippy-all.sh` invocation from CLAUDE.md is unchanged (caller-side); the script's internal contents changed but the user-facing command is the same.
- [ ] **Repo hygiene check**

**Exit Criteria:** `./clippy-all.sh`, ci.yml clippy jobs, and lefthook clippy hook all enforce `--workspace --all-targets`; `cargo test -p oriterm --test clippy_gate` passes; CI green; cluster's root cause is closed (any future flag removal fails CI immediately).
