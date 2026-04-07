---
section: "03"
title: "Tack Smoke Test"
status: not-started
reviewed: false
goal: "Prove the full pipeline works end-to-end with the simplest possible scenario: spawn `tack` under PtySession with TerminfoEnv-pinned (TERM=ori_term, TERMINFO=..., TERMINFO_DIRS=...), wait for the main menu, capture the grid as an insta snapshot, send 'q' to quit cleanly. This is the empirical discovery point for tack's actual prompt format and menu wording — Section 04 builds the scenario framework on top of whatever Section 03 captures."
success_criteria:
  - "`tack_available()` helper exists in `oriterm_test_support` next to `vttest_available()`/`tic_available()`"
  - "`PtySession::spawn_tack(env: &TerminfoEnv, cols, rows)` helper constructs a tack session with the pinned terminfo env vars"
  - "Smoke test `tack_smoke_main_menu_at_80x24` spawns tack, waits for the main menu prompt (`tack [n] >`), captures `grid_text()`, asserts via insta snapshot, sends `q\\n` to quit, AND asserts the child exited within 2 seconds via `PtySession::wait_for_child_exit(timeout_ms) -> ExitStatus`. The exit-status assertion is what makes 'verifies child exits' executable — without it, the test only proves the parent stopped reading bytes, not that tack actually terminated."
  - "The captured insta snapshot contains the literal substrings `Main Menu`, `begin testing`, `tools`, `quit`, and `tack [n] >` (verified at test time, not just visually inspected)"
  - "Test skips cleanly when tack OR tic is unavailable — both tools are required, both gated"
  - "Test discovers and documents the actual sub-menu prompt format used by tack (the captured snapshot IS the documentation that Section 04 consumes)"
  - "`timeout 150 cargo test -p oriterm_core --test tack -- tack_smoke_main_menu_at_80x24` passes on Linux"
  - "Satisfies mission criteria: 'Tack scenarios cover...' (foundation for Sections 05-06), 'tests skip cleanly when tack/tic unavailable'"
inspired_by:
  - "ori_term teseq smoke test (plans/completed/teseq-conformance/section-01-infrastructure.md:561-580 — the `smoke_bel` pattern)"
  - "ori_term vttest cursor-position smoke test (oriterm_core/tests/vttest/menu1.rs::vttest_border_fills_80x24 — same `wait_for(needle, timeout)` flow)"
  - "ncurses tack(1) man page — `tack [-itV] [term]` invocation, menu navigation"
depends_on: ["01", "02"]
third_party_review:
  status: none
  updated: null
sections:
  - id: "03.1"
    title: "tack_available helper and PtySession::spawn_tack"
    status: not-started
  - id: "03.2"
    title: "Smoke test directory and main.rs scaffold"
    status: not-started
  - id: "03.3"
    title: "tack_smoke_main_menu_at_80x24 with insta snapshot"
    status: not-started
  - id: "03.4"
    title: "Skip discipline and child cleanup"
    status: not-started
  - id: "03.R"
    title: "Third Party Review Findings"
    status: not-started
  - id: "03.N"
    title: "Completion Checklist"
    status: not-started
---

# Section 03: Tack Smoke Test

**Status:** Not Started
**Goal:** Spawn `tack` under `PtySession` with `TERM=ori_term`, `TERMINFO=<TerminfoEnv tempdir>`, and `TERMINFO_DIRS=<TerminfoEnv tempdir>` set via `TerminfoEnv::apply_env(&mut cmd)`, wait until tack prints the main menu and the `tack [n] > ` prompt, capture the grid as an insta golden snapshot, send `q\n` to quit, then assert the child exits within 2 s via `wait_for_child_exit`. The captured snapshot is the empirical record of tack's actual main menu format that Section 04's scenario framework will consume — no part of Section 04 should hard-code menu text that isn't first verified by Section 03.

**Success Criteria:**

- [ ] `tack_available()` exists in `oriterm_test_support` and matches `tool_available("tack", "-V")`
- [ ] `PtySession::spawn_tack(&TerminfoEnv, cols, rows)` helper exists alongside `spawn_vttest` and uses `TerminfoEnv::apply_env`
- [ ] `PtySession::wait_for_child_exit(timeout_ms) -> ExitStatus` exists alongside `wait_for` and `wait`
- [ ] `oriterm_core/tests/tack/main.rs` exists with `tack_smoke_main_menu_at_80x24` test
- [ ] The smoke test captures an insta snapshot at `oriterm_core/tests/tack/snapshots/tack_smoke_main_menu_80x24.snap`
- [ ] The captured snapshot, asserted programmatically via `assert!(grid.contains("..."))`, includes: `"Main Menu"`, `"begin testing"`, `"tools"`, `"quit"`, `"tack [n] >"`
- [ ] The smoke test sends `q\n`, then calls `session.wait_for_child_exit(2_000)` which observes the child terminating within 2 s — no zombie processes, no leaked PTY file descriptors
- [ ] The test skips when `tack` OR `tic` is unavailable, with a clear `eprintln!` message
- [ ] `timeout 150 cargo test -p oriterm_core --test tack -- tack_smoke_main_menu_at_80x24` passes on Linux
- [ ] Satisfies mission criteria: foundation for tack scenarios; cross-platform skip discipline

**Context:** Tack is interactive — it draws menus, accepts keystrokes, draws sub-menus, and so on. Before Section 04 builds the structured `ScenarioSpec`/`TackNavigator` framework, we need to PROVE that the basic spawn-and-capture loop works against ori_term's pinned terminfo entry. The smoke test is the smallest possible end-to-end exercise: spawn tack, wait for the main menu, capture, quit. If this fails, the whole tack pipeline is broken — fix it here, not later.

The smoke test also serves as the **empirical discovery mechanism** for tack's exact main menu wording. The plan overview claims the prompt is `tack [n] > ` (verified live during plan creation: `printf 'q\n' | TERM=xterm-256color tack` shows `tack [n] > ` literally). The submenu wording for `n) begin testing` and `t) tools` is observed by Section 03 once tack is running under the pinned terminfo, then Section 04 builds the scenario framework around the OBSERVED text — not the assumed text.

**Reference implementations:**
- **ori_term teseq** `plans/completed/teseq-conformance/section-01-infrastructure.md:561-580`: the `smoke_bel` test pattern — minimal end-to-end exercise that validates the harness works before any catalog content lands.
- **ori_term vttest** `oriterm_core/tests/vttest/menu1.rs::vttest_border_fills_80x24`: same `PtySession::wait_for(needle, timeout_ms)` then `grid_text()` flow we use here.
- **ncurses tack(1) man page**: invocation, term argument, menu navigation conventions.
- **Live tack capture** (verified during plan creation): `printf 'q\n' | TERM=xterm-256color tack` produces `Main Menu\n b) display basic information\n m) change modes\n t) tools\n n) begin testing\n l) start logging\n q) quit\n ?) help\n\ntack [n] > ` (whitespace approximate).

**Depends on:** Section 01 (PtySession), Section 02 (TerminfoEnv).

---

## 03.1 tack_available helper and PtySession::spawn_tack

**File(s):** `crates/oriterm_test_support/src/session/mod.rs` (Section 01 already promoted `session` to a directory module — extend `mod.rs` directly; do NOT create a sibling `session.rs`)

The smoke test (and every later tack test) needs:
1. A runtime check: is `tack` installed?
2. A spawn helper: how do we construct a `PtySession` running tack under the pinned terminfo?
3. A child-exit observation method on `PtySession` so the smoke test can assert tack actually terminated, not just that the parent stopped reading.

All three go in `oriterm_test_support` next to the equivalents from Sections 01 and 02.

- [ ] Add `tack_available()` to `crates/oriterm_test_support/src/session/mod.rs`:
  ```rust
  /// Check if `tack` (terminfo action checker, ncurses) is installed.
  ///
  /// Tack ships with ncurses on Linux/macOS, not on native Windows.
  /// Use this gate at the top of every test that spawns tack so the
  /// suite skips cleanly on platforms missing the tool.
  #[must_use]
  pub fn tack_available() -> bool {
      tool_available("tack", "-V")
  }
  ```
  Add the `pub use` line to `lib.rs` re-exports next to `tic_available`.

- [ ] Add `PtySession::spawn_tack(...)` helper. Decide carefully where to put it: it needs `TerminfoEnv` (from `crates/oriterm_test_support/src/terminfo/mod.rs`) and is therefore aware of terminfo provisioning. Put it on `PtySession` as an inherent method that takes `&TerminfoEnv` and uses the documented `apply_env` wrapper from Section 02:

  ```rust
  use crate::terminfo::TerminfoEnv;

  impl PtySession {
      /// Spawn `tack` at the given grid size, using the supplied
      /// `TerminfoEnv` to pin `TERM`, `TERMINFO`, and `TERMINFO_DIRS`.
      ///
      /// `tack` reads the terminfo entry named by `$TERM` from the
      /// directories listed in `$TERMINFO_DIRS` (or `$TERMINFO` —
      /// some ncurses consumers honor only one of the two). The
      /// `TerminfoEnv::apply_env` wrapper sets all three at once,
      /// hiding the env-var details from this call site.
      ///
      /// Mirrors `spawn_vttest(cols, rows)`. The split helper exists so
      /// the smoke test (Section 03) and the scenario catalog
      /// (Section 04+) share a single canonical tack invocation site.
      #[must_use]
      pub fn spawn_tack(env: &TerminfoEnv, cols: u16, rows: u16) -> Self {
          let mut cmd = CommandBuilder::new("tack");
          // Pass the term name as positional arg so tack picks it up
          // under both ncurses and BSD curses, regardless of which
          // env var the implementation consults first.
          cmd.arg(env.term());
          // Apply TERM/TERMINFO/TERMINFO_DIRS via the canonical wrapper.
          // No raw env-var iteration here — if TerminfoEnv learns about
          // a fourth env var tomorrow, this call site is unchanged.
          env.apply_env(&mut cmd);
          // We do NOT pass -i — tack's init sequences are part of what
          // we want to test. Comment kept here so future readers know
          // the omission is deliberate.
          Self::spawn(cmd, cols, rows)
      }
  }
  ```

- [ ] Add `PtySession::wait_for_child_exit(timeout_ms)` so the smoke test can assert child termination. The method polls the existing `child: Box<dyn portable_pty::Child + Send + Sync>` field via `try_wait()` until the child exits or the deadline expires. On timeout, panics with the current grid for diagnostic value (same panic-on-failure idiom as `wait_for`).

  ```rust
  use std::time::{Duration, Instant};
  use portable_pty::ExitStatus;

  impl PtySession {
      /// Wait until the child process exits, with a hard timeout.
      ///
      /// Returns the [`ExitStatus`] on clean exit. Panics with the
      /// current grid contents on timeout — the panic message tells
      /// the test author exactly what was on screen when the child
      /// failed to exit.
      ///
      /// Used by tack/vttest tests after sending `q\n` (or whatever
      /// the tool's quit key is) to assert the child actually
      /// terminated, not just that the parent stopped reading bytes.
      pub fn wait_for_child_exit(&mut self, timeout_ms: u64) -> ExitStatus {
          let deadline = Instant::now() + Duration::from_millis(timeout_ms);
          loop {
              match self.child.try_wait() {
                  Ok(Some(status)) => return status,
                  Ok(None) => {
                      assert!(
                          Instant::now() < deadline,
                          "child did not exit within {timeout_ms}ms.\nGrid:\n{}",
                          self.grid_text()
                      );
                      // Drain any final output while we wait.
                      self.drain_blocking(50);
                  }
                  Err(e) => panic!("PtySession::wait_for_child_exit: try_wait error: {e}"),
              }
          }
      }
  }
  ```

  This method must be added IN Section 03 (it's a Section 03 dependency for the smoke test's exit-status assertion). It is NOT a Section 01 backfill — Section 01's `PtySession` API is finalized for the existing 198 vttest tests; new methods can land in any later section that needs them, owned by that section.

  **Open question for the implementer:** if tack's init sequences are noisy in the snapshot (cursor positioning, mode setting, alt-screen enter), the smoke test in 03.3 may need to call `session.wait(500)` BEFORE asserting on the grid to let init settle. The right answer is data-driven — capture once, look at the snapshot, decide. Do not pre-emptively pass `-i` unless the snapshot shows it's needed.

- [ ] Add unit test for `tack_available`:
  ```rust
  #[test]
  fn tack_available_matches_tool_available() {
      assert_eq!(tack_available(), tool_available("tack", "-V"));
  }
  ```

---

## 03.2 Smoke test directory and main.rs scaffold

**File(s):** `oriterm_core/tests/tack/main.rs` (NEW), `oriterm_core/tests/tack/snapshots/` (NEW directory created on first snapshot)

The tack tests live alongside `oriterm_core/tests/vttest/` — they follow the same convention: a single integration test target (`main.rs`) declaring the scenario sub-modules.

- [ ] Create `oriterm_core/tests/tack/main.rs`:
  ```rust
  //! Tack-driven terminfo conformance tests.
  //!
  //! Spawns the ncurses `tack` (Terminfo Action Checker) tool against
  //! ori_term's pinned terminfo entry (`extra/ori_term.info`, compiled
  //! at runtime via `oriterm_test_support::TerminfoEnv`), navigates
  //! tack's menus from a PTY, and snapshots the rendered grid against
  //! insta golden references.
  //!
  //! Requires `tack` and `tic` installed (`apt install ncurses-bin` on
  //! Debian/Ubuntu, `brew install ncurses` on macOS). Tests gracefully
  //! skip on systems where either tool is missing — including native
  //! Windows where ncurses is not available without WSL/MSYS2.
  //!
  //! # Commands
  //!
  //! - Run: `cargo test -p oriterm_core --test tack`
  //! - Update snapshots: `INSTA_UPDATE=1 cargo test -p oriterm_core --test tack`
  //!
  //! # Layout
  //!
  //! - `main.rs` — this file (smoke + cross-cutting tests)
  //! - `framework/` — scenario catalog (Section 04)
  //! - `test_menu/` — `n) begin testing` submenu scenarios (Section 05)
  //! - `tools_menu/` — `t) tools` submenu scenarios (Section 06)

  // Sub-module declarations for Sections 04-06 (added as those sections land):
  // The framework lives in oriterm_test_support::tack_framework — no
  // local `mod framework;` declaration needed. Test wrapper modules:
  // mod test_menu;   // Section 05
  // mod tools_menu;  // Section 06

  use oriterm_test_support::{PtySession, TerminfoEnv, tack_available, tic_available};

  /// Smoke test: spawn tack under the pinned terminfo, wait for the
  /// main menu, capture as snapshot, quit cleanly.
  ///
  /// This is the canary that proves the tack pipeline (PtySession +
  /// TerminfoEnv + tack child) works end-to-end. If it fails, no
  /// scenario test in Sections 04-06 can possibly pass — fix it here.
  #[test]
  fn tack_smoke_main_menu_at_80x24() {
      if !tack_available() || !tic_available() {
          eprintln!("tack or tic not installed, skipping tack_smoke_main_menu_at_80x24");
          return;
      }

      let env = TerminfoEnv::compile();
      let mut session = PtySession::spawn_tack(&env, 80, 24);

      // Wait for the main menu prompt to appear. The exact prompt
      // string `tack [n] > ` is documented in the tack man page and
      // verified live during plan creation.
      //
      // Race-condition note: `wait_for` uses `drain_blocking(100)` +
      // content scan; it does NOT race on DECRQSS/DA handshakes because
      // `PtySession::drain()` writes `PtyWrite` responses back to the
      // PTY BEFORE returning (the same fix applied during vttest
      // conformance — see plans/completed/vttest-conformance/
      // section-01-terminal-size.md:79-83). If tack sends a DA/DECRQSS
      // query before drawing the main menu, the response is written
      // back inside the same drain call and tack's menu draw follows
      // naturally. No fixed sleeps needed.
      session.wait_for("tack [n] >", 5_000);

      let grid = session.grid_text();

      // Programmatic assertions against the captured grid — these
      // catch silent regressions where the snapshot updates but a
      // critical menu item disappears. The substrings come from the
      // live tack capture during plan creation.
      assert!(grid.contains("Main Menu"), "main menu header missing:\n{grid}");
      assert!(grid.contains("begin testing"), "'begin testing' missing:\n{grid}");
      assert!(grid.contains("tools"), "'tools' missing:\n{grid}");
      assert!(grid.contains("quit"), "'quit' missing:\n{grid}");
      assert!(grid.contains("tack [n] >"), "prompt missing:\n{grid}");

      // Capture as an insta snapshot. The first run creates the
      // golden; later runs compare against it byte-for-byte.
      insta::assert_snapshot!("tack_smoke_main_menu_80x24", grid);

      // Send 'q' + newline to quit tack cleanly.
      session.send(b"q\n");

      // Assert tack actually exited within 2 seconds. wait_for_child_exit
      // polls portable_pty::Child::try_wait() and panics with the current
      // grid on timeout. Without this assertion the test would silently
      // pass even if tack hung after receiving 'q\n' — the parent simply
      // stops reading bytes during the implicit `wait(300)` inside
      // `send`, but the child stays alive.
      //
      // We do NOT inspect the exit status's success() flag — tack's
      // exit code on clean quit is documented as 0 in ncurses tack(1),
      // but exit status semantics drift across distros and the
      // important assertion is that the child terminated, not its
      // particular code. Drop will still reap the child either way.
      let _exit = session.wait_for_child_exit(2_000);
  }
  ```

  Note: the framework lives in `oriterm_test_support::tack_framework` (Section 04 places it there from the start). The `mod test_menu;` and `mod tools_menu;` lines are **commented out** at the end of Section 03 — they get uncommented as Sections 05 and 06 land. This keeps Section 03 buildable in isolation.

---

## 03.3 tack_smoke_main_menu_at_80x24 with insta snapshot

**File(s):** Same as 03.2 — this subsection drives the test to green.

The test body is fully written in 03.2. This subsection is the iterative loop to make it pass.

- [ ] Run: `INSTA_UPDATE=1 timeout 150 cargo test -p oriterm_core --test tack -- tack_smoke_main_menu_at_80x24`
  - First run creates the golden under `oriterm_core/tests/tack/snapshots/tack__tack_smoke_main_menu_80x24.snap` (or similar — insta's filename convention).
- [ ] Open the generated `.snap` file. Verify the captured grid contains the expected main menu structure visually:
  ```
  Main Menu
   b) display basic information
   m) change modes
   t) tools
   n) begin testing
   l) start logging
   q) quit
   ?) help

  tack [n] >
  ```
  Whitespace exact match isn't required — insta diffs the full text. Only verify the menu items are present.
- [ ] Run: `timeout 150 cargo test -p oriterm_core --test tack -- tack_smoke_main_menu_at_80x24` (without `INSTA_UPDATE`). The test must PASS — the snapshot is now committed and re-runs are deterministic.
- [ ] If the snapshot is non-deterministic (e.g., contains a timestamp, contains `Terminal size: 80 x 24.  Baud rate: 38400.  Frame size: 10.0` where the baud rate or frame size varies), the snapshot is flaky. Per CLAUDE.md "Flaky tests ARE bugs", treat this as a blocker:
  - Identify the source of variation in the captured grid
  - Either: (a) wait longer with `session.wait_for("tack [n] >", ...)` so the screen reaches a steady state, or (b) post-process the grid before snapshotting to redact non-deterministic regions, or (c) shrink the snapshot to a stable region only (e.g., just the main menu lines, not the diagnostic header)
  - File via `/add-bug` if the root cause is in oriterm itself (e.g., grid serialization is non-deterministic)

- [ ] Validate the `wait_for` deadline is reasonable: 5 seconds is generous for a local PTY. CI may need 10s if the runner is slow — but do not pre-emptively raise it; only raise on observed CI flakiness.

- [ ] Run the test 10 times in a row to confirm determinism:
  ```
  for i in $(seq 1 10); do
      timeout 150 cargo test -p oriterm_core --test tack -- tack_smoke_main_menu_at_80x24 || break
  done
  ```
  All 10 must pass. If any fail, the test is flaky → file `/add-bug` and treat as blocker.

---

## 03.4 Skip discipline and child cleanup

**File(s):** `oriterm_core/tests/tack/main.rs` (no new file, verification only)

Skip-when-unavailable and clean-process-cleanup are non-negotiable per CLAUDE.md cross-platform rules and per the conformance suite's "compile everywhere, runtime skip" contract from `00-overview.md`.

- [ ] Verify skip discipline by manually causing tack to be unavailable. On a system where tack IS installed, force the gate to fail:
  ```rust
  // Temporarily replace `tack_available() || tic_available()` with `false`
  // to confirm the early-return path runs cleanly.
  if false { /* the body */ }
  ```
  Re-run the test — it must pass with the `eprintln!` message logged. Then restore the gate.

  This is a one-shot manual verification — no automated test needed since the code path is trivial.

- [ ] **Tack version drift handling (distro variance).** Different ncurses releases ship slightly different tack main-menu wording. The plan was authored against the ncurses v6.x tack. If the test fails on a distro with an older (or newer) tack whose main-menu wording differs, update the programmatic assertions to match the actual wording (verified by re-running `tack -V` and then `printf 'q\n' | tack` on the affected system). The insta snapshot will also need `INSTA_UPDATE=1` to refresh. Document the tested tack version in a comment next to the first assertion: `// Verified against tack v1.08 (ncurses 6.4).`

- [ ] Cross-compile for `x86_64-pc-windows-gnu`:
  ```
  cargo build --target x86_64-pc-windows-gnu -p oriterm_core --tests
  ```
  Must succeed. The test source must compile on Windows even though `tack` and `tic` are unavailable there — runtime skip handles that.

- [ ] Verify child cleanup:
  - Run the smoke test under `strace -f -e trace=clone,wait4` (Linux) or observe via `ps -ef | grep tack` immediately after the test ends.
  - No `tack` process should remain. `PtySession::Drop` (added in Section 01.2) calls `self.child.kill()` + `self.child.wait()` on the `child: Box<dyn Child + Send + Sync>` field to reap the process. If a zombie remains, the Drop impl is not firing — file as a bug against `PtySession` (Section 01) and fix there.

- [ ] Verify PTY file descriptor cleanup:
  - Run the smoke test 50 times in a tight loop. After the loop ends, check `/proc/self/fd | wc -l` (Linux) — the FD count should be stable, not growing. A leak here would fail this test in CI matrices that run the suite many times.

- [ ] **TPR checkpoint** — `/tpr-review` covering 03.1–03.4. Catches: misuse of `TerminfoEnv` env vars (wrong `TERM` name), incorrect `tack` arguments (`-V` vs `--version`), races between `wait_for` and `tack`'s alt-screen entry, leaked child processes, snapshot non-determinism.

---

## 03.R Third Party Review Findings

<!-- Reserved for Codex or other external reviewers. -->

- None.

---

## 03.N Completion Checklist

- [ ] `tack_available()` exists in `crates/oriterm_test_support/src/session/mod.rs` and is re-exported from `lib.rs`
- [ ] `PtySession::spawn_tack(env, cols, rows)` exists and uses `TerminfoEnv::apply_env(&mut cmd)` (NOT raw env iteration) to set `TERM`, `TERMINFO`, and `TERMINFO_DIRS` on the child
- [ ] `PtySession::wait_for_child_exit(timeout_ms) -> ExitStatus` exists in `crates/oriterm_test_support/src/session/mod.rs` and is the primitive the smoke test calls after sending `q\n`
- [ ] `oriterm_core/tests/tack/main.rs` exists and declares the smoke test
- [ ] `tack_smoke_main_menu_at_80x24` test passes on Linux (`timeout 150 cargo test -p oriterm_core --test tack`)
- [ ] Insta snapshot `oriterm_core/tests/tack/snapshots/tack__tack_smoke_main_menu_80x24.snap` exists and is committed
- [ ] Programmatic assertions inside the test verify `Main Menu`, `begin testing`, `tools`, `quit`, `tack [n] >` are present
- [ ] Test skips cleanly when `tack_available()` or `tic_available()` returns false — `eprintln!` message logged, returns Ok
- [ ] Cross-compile for `x86_64-pc-windows-gnu` succeeds (`cargo build --target x86_64-pc-windows-gnu -p oriterm_core --tests`)
- [ ] No zombie tack processes after test runs (manual `ps` check OR FD-leak loop test)
- [ ] Test runs deterministically — 10 consecutive runs all pass without flake
- [ ] `./build-all.sh` green
- [ ] `./clippy-all.sh` green
- [ ] `timeout 150 ./test-all.sh` green
- [ ] Plan annotation cleanup: no temporary scaffolding in `.rs` files
- [ ] All TPR checkpoint findings resolved (see `03.R`)
- [ ] **Plan sync**:
  - [ ] This section's frontmatter `status` → `complete`
  - [ ] `00-overview.md` Quick Reference table: Section 03 marked Complete
  - [ ] `index.md` Section 03 status updated
  - [ ] Section 04's `depends_on: ["03"]` confirmed (Section 04 builds the scenario framework on top of `spawn_tack`)
- [ ] `/tpr-review` final pass clean
- [ ] `/impl-hygiene-review last commit` final pass clean (after TPR)

**Exit Criteria:** `tack_smoke_main_menu_at_80x24` passes deterministically on Linux: `timeout 150 cargo test -p oriterm_core --test tack -- tack_smoke_main_menu_at_80x24` returns success in under 10 seconds. The captured insta snapshot contains the literal main menu structure (`Main Menu`, `begin testing`, `tools`, `quit`, `tack [n] >`). The test cross-compiles for Windows and skips cleanly on Linux/macOS without `tack` installed. No zombie processes, no leaked file descriptors. The pipeline is proven end-to-end and Section 04 can safely build the scenario framework on top.
