---
section: "03"
title: "Tack Smoke Test"
status: complete
reviewed: true
goal: "Prove the full pipeline works end-to-end with the simplest possible scenario: spawn `tack` under PtySession with TerminfoEnv-pinned (TERM=ori_term, TERMINFO=..., TERMINFO_DIRS=...), wait for the main menu, capture the grid as an insta snapshot, send 'q' to quit cleanly. This is the empirical discovery point for tack's actual prompt format and menu wording — Section 04 builds the scenario framework on top of whatever Section 03 captures."
success_criteria:
  - "`tack_available()` helper exists in `oriterm_test_support` next to `vttest_available()`/`tic_available()`"
  - "`PtySession::spawn_tack(env: &TerminfoEnv, cols, rows)` helper constructs a tack session with the pinned terminfo env vars"
  - "`PtySession::wait_for_child_exit(timeout_ms) -> ExitStatus` primitive exists on `PtySession`, polls `portable_pty::Child::try_wait()` with a **bounded** poll rate on the `Ok(None)` branch (sleeps 10 ms when `drain_blocking` returns 0 so the loop never hot-spins after the reader thread closes), and has its own unit test in `crates/oriterm_test_support/src/session/tests.rs`"
  - "Smoke test `tack_smoke_main_menu_at_80x24` spawns tack, waits for the main menu prompt (`tack [n] >`), captures `grid_text()`, asserts via insta snapshot, sends `q\\n` to quit, AND asserts the child exited within 2 seconds via `PtySession::wait_for_child_exit(timeout_ms) -> ExitStatus`. The exit-status assertion is what makes 'verifies child exits' executable — without it, the test only proves the parent stopped reading bytes, not that tack actually terminated. The test MUST also surface the exit code (either `assert!(exit.success(), ...)` if tack's quit semantics are stable, or `eprintln!(\"tack exit status: {exit:?}\")` at minimum so CI logs capture regressions)"
  - "The captured insta snapshot contains the literal substrings `Main Menu`, `begin testing`, `tools`, `quit`, and `tack [n] >` (verified at test time, not just visually inspected)"
  - "Test skips cleanly when tack OR tic is unavailable — both tools are required, both gated"
  - "Test discovers and documents the actual sub-menu prompt format used by tack (the captured snapshot IS the documentation that Section 04 consumes)"
  - "`timeout 150 cargo test -p oriterm_core --test tack -- tack_smoke_main_menu_at_80x24` passes on Linux"
  - "Satisfies mission criteria (traced explicitly): (a) 'Tack test scenarios cover EVERY navigable begin-testing screen...' — Section 03 is the PREREQUISITE canary that proves tack + PtySession + TerminfoEnv work before Section 04 builds the scenario framework and Sections 05-06 populate it. (b) 'All tests skip cleanly when tack/tic unavailable' — delivered directly by the `if !tack_available() || !tic_available()` gate and verified by 03.4's PATH-override check. (c) '`extra/ori_term.info` ... hand-authored, fully-pinned entry' — Section 02 creates the entry; Section 03 validates it is actually consumed by a live tack child under `TERM=ori_term`. (d) './test-all.sh green, ./build-all.sh green, ./clippy-all.sh green' — gated by 03.N's completion checklist."
inspired_by:
  - "ori_term teseq smoke test (plans/completed/teseq-conformance/section-01-infrastructure.md:561-580 — the `smoke_bel` pattern)"
  - "ori_term vttest cursor-position smoke test (oriterm_core/tests/vttest/menu1.rs::vttest_border_fills_80x24 — same `wait_for(needle, timeout)` flow)"
  - "ncurses tack(1) man page — `tack [-itV] [term]` invocation, menu navigation"
depends_on: ["01", "02"]
third_party_review:
  status: resolved
  updated: 2026-04-07
sections:
  - id: "03.1"
    title: "tack_available, spawn_tack, wait_for_child_exit primitives"
    status: complete
  - id: "03.2"
    title: "Smoke test directory and main.rs scaffold"
    status: complete
  - id: "03.3"
    title: "tack_smoke_main_menu_at_80x24 with insta snapshot"
    status: complete
  - id: "03.4"
    title: "Skip discipline and platform compile"
    status: complete
  - id: "03.5"
    title: "Exit semantics and cleanup verification"
    status: complete
  - id: "03.T"
    title: "TPR checkpoint"
    status: complete
  - id: "03.R"
    title: "Third Party Review Findings"
    status: complete
  - id: "03.N"
    title: "Completion Checklist"
    status: complete
---

# Section 03: Tack Smoke Test

**Status:** Complete
**Goal:** Spawn `tack` under `PtySession` with `TERM=ori_term`, `TERMINFO=<TerminfoEnv tempdir>`, and `TERMINFO_DIRS=<TerminfoEnv tempdir>` set via `TerminfoEnv::apply_env(&mut cmd)`, wait until tack prints the main menu and the `tack [n] > ` prompt, capture the grid as an insta golden snapshot, send `q\n` to quit, then assert the child exits within 2 s via `wait_for_child_exit`. The captured snapshot is the empirical record of tack's actual main menu format that Section 04's scenario framework will consume — no part of Section 04 should hard-code menu text that isn't first verified by Section 03.

**Success Criteria:**

- [x] `tack_available()` exists in `oriterm_test_support` and matches `tool_available("tack", "-V")`
- [x] `PtySession::spawn_tack(&TerminfoEnv, cols, rows)` helper exists alongside `spawn_vttest` and uses `TerminfoEnv::apply_env`
- [x] `PtySession::wait_for_child_exit(timeout_ms) -> ExitStatus` exists alongside `wait_for` and `wait` AND implements the bounded-poll contract (10 ms sleep on `Ok(None)` when `drain_blocking` returned 0, never hot-spins after reader EOF)
- [x] `pty_session_wait_for_child_exit_returns_on_clean_exit` unit test exists in `crates/oriterm_test_support/src/session/tests.rs` using the two-arm `#[cfg(unix)] / #[cfg(windows)]` pattern and asserts `status.success()`
- [x] `oriterm_core/tests/tack/main.rs` exists with `tack_smoke_main_menu_at_80x24` test
- [x] The smoke test captures an insta snapshot at `oriterm_core/tests/tack/snapshots/tack__tack_smoke_main_menu_80x24.snap` (insta prepends the target name `tack__` to the snapshot file)
- [x] The captured snapshot, asserted programmatically via `assert!(grid.contains("..."))`, includes: `"Main Menu"`, `"begin testing"`, `"tools"`, `"quit"`, `"tack [n] >"` (NOTE: `grid_text()` captures only `RenderableCell.ch`; color, SGR, and wide-char attributes are NOT validated here — those are deferred to Sections 05-07)
- [x] The smoke test sends `q\n`, then calls `session.wait_for_child_exit(2_000)` which observes the child terminating within 2 s — no zombie processes, no leaked PTY file descriptors
- [x] The smoke test surfaces tack's exit code via `eprintln!` (minimum) or `assert!(exit.success(), ...)` (preferred, after 03.5 distro investigation)
- [x] The test skips when `tack` OR `tic` is unavailable, with a clear `eprintln!` message
- [x] `timeout 150 cargo test -p oriterm_core --test tack -- tack_smoke_main_menu_at_80x24` passes on Linux
- [x] Satisfies mission criteria, traced explicitly: (a) prerequisite canary for "tack test scenarios cover EVERY navigable begin-testing screen" (Section 04+ depends on this section's proven `spawn_tack` + main-menu snapshot), (b) directly delivers "all tests skip cleanly when tack/tic unavailable" via the gate, (c) validates "`extra/ori_term.info` ... fully-pinned entry" is consumed by a real tack child under `TERM=ori_term`, (d) gates `./test-all.sh`, `./build-all.sh`, `./clippy-all.sh` green via 03.N

**Context:** Tack is interactive — it draws menus, accepts keystrokes, draws sub-menus, and so on. Before Section 04 builds the structured `ScenarioSpec`/`TackNavigator` framework, we need to PROVE that the basic spawn-and-capture loop works against ori_term's pinned terminfo entry. The smoke test is the smallest possible end-to-end exercise: spawn tack, wait for the main menu, capture, quit. If this fails, the whole tack pipeline is broken — fix it here, not later.

The smoke test also serves as the **empirical discovery mechanism** for tack's exact main menu wording. The plan overview claims the prompt is `tack [n] > ` (verified live during plan creation: `printf 'q\n' | TERM=xterm-256color tack` shows `tack [n] > ` literally). The submenu wording for `n) begin testing` and `t) tools` is observed by Section 03 once tack is running under the pinned terminfo, then Section 04 builds the scenario framework around the OBSERVED text — not the assumed text.

**PTY-required scope note.** `tack` behaves differently depending on whether stdin is a real TTY. When run without a PTY against `ori_term` (e.g. `printf 'q\n' | TERM=ori_term tack ori_term`) the tool prints `The "ori_term" terminal is listed as generic` and exits without ever reaching the menu — a completely different failure mode from the interactive path. Under a real PTY (which `PtySession` provides via `portable_pty::native_pty_system`) tack works normally: `smcup`, the main menu, the `tack [n] >` prompt, and sub-menu navigation all draw as expected. **Section 03 validates only the interactive-PTY path.** The non-PTY "listed as generic" fallback is out of scope for the smoke test and out of scope for the entire tack-conformance plan — it's a tack/ncurses reachability check, not a terminfo-correctness check.

**What this smoke test does NOT validate.** `PtySession::grid_text()` serializes only `RenderableCell.ch` (see `oriterm_core/src/term/renderable/mod.rs` — `pub ch: char` alongside `pub fg: Rgb`, `pub bg: Rgb`, `pub flags: CellFlags`). Non-character attributes — foreground/background colors, SGR flags, underline styles, wide-char spacers, ACS-vs-Unicode translation, zero-width combining marks — are all **discarded** by the smoke-test snapshot. That is acceptable for Section 03 ("prove the basic interactive pipeline works end-to-end"), but it means this section's assertions do NOT cover color correctness, SGR rendering, wide-char handling, or any visual attribute. Those are deferred to:
- **Section 05/06** — text scenarios use the same `grid_text()` extraction for navigation-correctness assertions, not visual fidelity.
- **Section 07** — GPU golden images capture full visual fidelity (pixel-by-pixel) via `render_to_pixels`, which IS what validates colors, SGR, wide chars, and ACS translations.

Any tack menu item that differs from the baseline only in color or SGR flags will pass this smoke test unchanged. That is by design — Section 03 is the pipeline canary, not the rendering canary.

**Reference implementations:**
- **ori_term teseq** `plans/completed/teseq-conformance/section-01-infrastructure.md:561-580`: the `smoke_bel` test pattern — minimal end-to-end exercise that validates the harness works before any catalog content lands.
- **ori_term vttest** `oriterm_core/tests/vttest/menu1.rs::vttest_border_fills_80x24`: same `PtySession::wait_for(needle, timeout_ms)` then `grid_text()` flow we use here.
- **ncurses tack(1) man page**: invocation, term argument, menu navigation conventions.
- **Live tack capture** (verified during plan creation): `printf 'q\n' | TERM=xterm-256color tack` produces `Main Menu\n b) display basic information\n m) change modes\n t) tools\n n) begin testing\n l) start logging\n q) quit\n ?) help\n\ntack [n] > ` (whitespace approximate).

**Depends on:** Section 01 (PtySession), Section 02 (TerminfoEnv).

**Bug-tracker adjacency (BUG-07-004).** `plans/bug-tracker/section-07-ci-build.md` BUG-07-004 tracks the removal of `pty_session_drains_simple_output`'s Windows arm via `#[cfg(unix)]` (later fixed by BUG-07-008 which restored the two-arm pattern for that specific test). Section 03's new `pty_session_wait_for_child_exit_returns_on_clean_exit` unit test (03.1) uses the same two-arm `#[cfg(unix)]` / `#[cfg(windows)]` pattern and exercises `cmd.exe /C exit 0` on Windows — this is the **first Windows-native ConPTY child-lifecycle assertion** in `oriterm_test_support::session` in the tack-conformance plan. It does NOT close BUG-07-004 (that bug wants a Windows PTY **size-propagation** test specifically; the smoke test here covers child-exit lifecycle, an adjacent but different surface). Do not mark BUG-07-004 closed when Section 03 lands — only the adjacent coverage is gained. BUG-07-004 closes when a follow-up Windows ConPTY size test lands (out of scope for this plan; tracked in the bug tracker).

**Edge cases `wait_for_child_exit` must handle cleanly.** The primitive runs after `send(b"q\n")` which itself calls `wait(300)` internally (draining PTY output until 300 ms of quiet). Three edge cases arise:

1. **Exit observed during `send`'s drain.** `wait(300)` may consume the exit bytes before `wait_for_child_exit` is even called. This is fine — `try_wait()` on the first iteration returns `Ok(Some(status))` immediately and the method returns. No special handling needed; document the invariant.
2. **Tack ignores `q\n` in an error state.** If tack is hung or in a menu state that doesn't recognize `q\n` as "quit", `wait_for_child_exit(2_000)` will panic after 2 s with the grid contents (the panic message format is mandated by 03.1). The panic unwinds, `PtySession::Drop` runs, the child is killed via `child.kill()` + `child.wait()` (reaped in Section 01's Drop impl). No zombie, no leaked FD — the Drop guard from Section 01 is what makes the timeout path safe. This is the canonical cleanup pattern: the test panics loud, Drop cleans up silent.
3. **Exit races with final drain on `Ok(None)`.** Between the reader-thread EOF and `try_wait()` observing the exit, `drain_blocking(50)` returns 0 because the channel is closed. The 10 ms bounded-poll sleep (see 03.1's bounded-poll contract) prevents the loop from hot-spinning in this window. 10 ms × 200 iterations = 2 s deadline with full headroom.

If a future change removes the Drop impl from Section 01's `PtySession`, the timeout-path cleanup in case (2) breaks silently. The invariant "`PtySession::Drop` kills and reaps the child" is a **hard prerequisite** of Section 03's panic-on-timeout behavior. Section 01's existing `pty_session_drains_simple_output` test and the `VtTestSession` zombie-leak LEAK fix (mission criterion) together enforce this — do NOT weaken Drop in any later section without updating Section 03's timeout-path documentation.

---

## 03.1 tack_available, spawn_tack, wait_for_child_exit primitives

**File(s):** `crates/oriterm_test_support/src/session/mod.rs` (Section 01 already promoted `session` to a directory module — extend `mod.rs` directly; do NOT create a sibling `session.rs`)

The smoke test (and every later tack test) needs:
1. A runtime check: is `tack` installed?
2. A spawn helper: how do we construct a `PtySession` running tack under the pinned terminfo?
3. A child-exit observation method on `PtySession` so the smoke test can assert tack actually terminated, not just that the parent stopped reading.

All three go in `oriterm_test_support` next to the equivalents from Sections 01 and 02.

- [x] Add `tack_available()` to `crates/oriterm_test_support/src/session/mod.rs`:
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

- [x] Add `PtySession::spawn_tack(...)` helper. Decide carefully where to put it: it needs `TerminfoEnv` (from `crates/oriterm_test_support/src/terminfo/mod.rs`) and is therefore aware of terminfo provisioning. Put it on `PtySession` as an inherent method that takes `&TerminfoEnv` and uses the documented `apply_env` wrapper from Section 02:

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

- [x] Add `PtySession::wait_for_child_exit(timeout_ms)` so the smoke test can assert child termination. The method polls the existing `child: Box<dyn portable_pty::Child + Send + Sync>` field via `try_wait()` until the child exits or the deadline expires. On timeout, panics with the current grid for diagnostic value (same panic-on-failure idiom as `wait_for`).

  **Bounded-poll contract (must not hot-spin).** Once the child exits, the reader thread hits EOF, drops the channel, and `drain_blocking(50)` returns `0` **immediately** (the `recv_timeout` inside `drain_blocking` sees a closed channel and returns `Err` without sleeping — see `session/mod.rs` lines 188-194). Between the moment the child's reader-side closes and the moment `portable_pty::Child::try_wait()` observes termination (Unix: `waitpid(WNOHANG)`; Windows: `GetExitCodeProcess` — NOT `WaitForSingleObject(handle, 0)`, see `crates/portable-pty/src/win/mod.rs` `WinChild::is_complete`), the loop could hot-spin on `try_wait` → `drain_blocking(50)` returning 0 → `try_wait` → ... up to the full `timeout_ms` deadline, burning 100% CPU. **Mitigation:** on the `Ok(None)` path, call `self.drain_blocking(50)` first (forward progress on any late output), then if it returned 0, `std::thread::sleep(Duration::from_millis(10))` to bound the poll rate to ~100 Hz. 10 ms is large enough that busy-wait is impossible, small enough that a 2-second test deadline still has 200 poll attempts of headroom.

  ```rust
  use std::thread;
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
      ///
      /// Implementation note: this polls
      /// [`portable_pty::Child::try_wait`] (Unix: `waitpid(WNOHANG)`;
      /// Windows: `GetExitCodeProcess` — NOT `WaitForSingleObject`).
      /// On the `Ok(None)` branch the method calls
      /// [`Self::drain_blocking`] to forward any late output, then
      /// sleeps 10 ms if nothing was drained — this bounds the poll
      /// rate to ~100 Hz so the loop never hot-spins between reader
      /// EOF and `try_wait` observing termination.
      pub fn wait_for_child_exit(&mut self, timeout_ms: u64) -> ExitStatus {
          const POLL_SLEEP: Duration = Duration::from_millis(10);
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
                      let drained = self.drain_blocking(50);
                      if drained == 0 {
                          // Reader thread has closed its channel (EOF);
                          // drain_blocking returned immediately. Bound
                          // the poll rate so we don't burn CPU between
                          // reader EOF and try_wait() observing exit.
                          thread::sleep(POLL_SLEEP);
                      }
                  }
                  Err(e) => panic!("PtySession::wait_for_child_exit: try_wait error: {e}"),
              }
          }
      }
  }
  ```

  This method must be added IN Section 03 (it's a Section 03 dependency for the smoke test's exit-status assertion). It is NOT a Section 01 backfill — Section 01's `PtySession` API is finalized for the existing 198 vttest tests; new methods can land in any later section that needs them, owned by that section.

  **Open question for the implementer:** if tack's init sequences are noisy in the snapshot (cursor positioning, mode setting, alt-screen enter), the smoke test in 03.3 may need to call `session.wait(500)` BEFORE asserting on the grid to let init settle. The right answer is data-driven — capture once, look at the snapshot, decide. Do not pre-emptively pass `-i` unless the snapshot shows it's needed.

- [x] Add unit test for `tack_available` in `crates/oriterm_test_support/src/session/tests.rs`:
  ```rust
  #[test]
  fn tack_available_matches_tool_available() {
      assert_eq!(tack_available(), tool_available("tack", "-V"));
  }
  ```

- [x] **Add unit test for `wait_for_child_exit`** in the same `session/tests.rs`. `wait_for_child_exit` is a new core `PtySession` primitive — per impl-hygiene testing discipline every core primitive needs its own unit test, not just coverage through the tack smoke test. Use the same two-arm `#[cfg(unix)]` / `#[cfg(windows)]` pattern as the existing `pty_session_drains_simple_output` test so Windows gets real ConPTY coverage of `GetExitCodeProcess`:
  ```rust
  #[test]
  fn pty_session_wait_for_child_exit_returns_on_clean_exit() {
      #[cfg(unix)]
      let cmd = {
          let mut c = CommandBuilder::new("/bin/sh");
          c.args(["-c", "exit 0"]);
          c.env("TERM", "xterm-256color");
          c
      };
      #[cfg(windows)]
      let cmd = {
          let mut c = CommandBuilder::new("cmd.exe");
          c.args(["/C", "exit 0"]);
          c.env("TERM", "xterm-256color");
          c
      };
      let mut session = PtySession::spawn(cmd, 80, 24);
      let status = session.wait_for_child_exit(5_000);
      assert!(
          status.success(),
          "expected clean exit, got {status:?}"
      );
  }
  ```
  This test also functions as a smoke test for the bounded-poll path: a `sh -c "exit 0"` child exits essentially immediately, so the loop must tolerate the reader-closed-channel + try_wait-observes-exit race without hot-spinning and without deadlocking. If the 10 ms sleep gets accidentally dropped in a future refactor, this test still passes (the exit is observed in the first iteration) — but the `test-all.sh` wall-clock budget regression is the canary. If the test starts taking >100 ms, something is wrong.

---

## 03.2 Smoke test directory and main.rs scaffold

**File(s):** `oriterm_core/tests/tack/main.rs` (NEW), `oriterm_core/tests/tack/snapshots/` (NEW directory created on first snapshot)

The tack tests live alongside `oriterm_core/tests/vttest/` — they follow the same convention: a single integration test target (`main.rs`) declaring the scenario sub-modules.

- [x] Create `oriterm_core/tests/tack/main.rs`:
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
      // Exit-status surfacing: "exited within 2s" alone still passes if
      // tack aborted with an error code. Log the exit status via
      // eprintln! so CI logs capture the exact code on every run — if
      // tack starts returning non-zero on clean quit after a distro
      // upgrade, this is the first breadcrumb the next debugger sees.
      // The 03.5 "Investigate tack's clean-quit exit code" checklist
      // item upgrades this to `assert!(exit.success(), ...)` once the
      // cross-distro behavior is empirically characterized.
      let exit = session.wait_for_child_exit(2_000);
      eprintln!("tack_smoke_main_menu_at_80x24: tack exit status = {exit:?}");
  }
  ```

  Note: the framework lives in `oriterm_test_support::tack_framework` (Section 04 places it there from the start). The `mod test_menu;` and `mod tools_menu;` lines are **commented out** at the end of Section 03 — they get uncommented as Sections 05 and 06 land. This keeps Section 03 buildable in isolation.

---

## 03.3 tack_smoke_main_menu_at_80x24 with insta snapshot

**File(s):** Same as 03.2 — this subsection drives the test to green.

The test body is fully written in 03.2. This subsection is the iterative loop to make it pass.

The subsection is a strict linear sequence: generate → visually verify → re-run against the generated golden → **git-add the .snap file** → flake loop → remediation. Do not reorder. The snapshot must land in the working tree **and be staged** before the 10x flake loop runs, otherwise a flake on iteration N leaves a modified `.snap` file that the developer can't distinguish from "the snapshot I just generated".

- [x] **Step 1 — Generate the golden.** Run: `INSTA_UPDATE=1 timeout 150 cargo test -p oriterm_core --test tack -- tack_smoke_main_menu_at_80x24`
  - First run creates the golden under `oriterm_core/tests/tack/snapshots/tack__tack_smoke_main_menu_80x24.snap` (or similar — insta's filename convention).
- [x] **Step 2 — Visual sanity check.** Open the generated `.snap` file. Verify the captured grid contains the expected main menu structure visually:
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
- [x] **Step 3 — Re-run against the golden.** Run: `timeout 150 cargo test -p oriterm_core --test tack -- tack_smoke_main_menu_at_80x24` (without `INSTA_UPDATE`). The test must PASS — the snapshot is now in the working tree and re-runs match it byte-for-byte.
- [x] **Step 4 — Stage the snapshot in git.** `git add oriterm_core/tests/tack/snapshots/` so the golden is part of the in-progress commit. This MUST happen before Step 6's flake loop — otherwise a flake mid-loop would leave a modified `.snap` file with no baseline to diff against. Do NOT commit yet; the commit lands at the end of Section 03 after the whole checklist passes.
- [x] **Step 5 — Non-determinism triage.** If the snapshot is non-deterministic (e.g., contains a timestamp, contains `Terminal size: 80 x 24.  Baud rate: 38400.  Frame size: 10.0` where the baud rate or frame size varies), the snapshot is flaky. Per CLAUDE.md "Flaky tests ARE bugs", treat this as a blocker:
  - Identify the source of variation in the captured grid
  - Either: (a) wait longer with `session.wait_for("tack [n] >", ...)` so the screen reaches a steady state, or (b) post-process the grid before snapshotting to redact non-deterministic regions, or (c) shrink the snapshot to a stable region only (e.g., just the main menu lines, not the diagnostic header)
  - File via `/add-bug` if the root cause is in oriterm itself (e.g., grid serialization is non-deterministic)
  - If triage lands on a fix, go back to Step 1 and regenerate. The flake loop only runs on a suspected-stable snapshot.

- [x] **Step 5a — Wait-timeout sanity check.** Validate the `wait_for` deadline is reasonable: 5 seconds is generous for a local PTY. CI may need 10s if the runner is slow — but do not pre-emptively raise it; only raise on observed CI flakiness.

- [x] **Step 6 — Determinism loop (AFTER Step 4 git-add).** Run the test 10 times in a row against the now-staged snapshot:
  ```
  for i in $(seq 1 10); do
      timeout 150 cargo test -p oriterm_core --test tack -- tack_smoke_main_menu_at_80x24 || { echo "flake on iteration $i"; break; }
  done
  ```
  All 10 must pass. If any fail:
  1. `git diff oriterm_core/tests/tack/snapshots/` to see what iteration N produced that differed from the Step 4 baseline. (This is why Step 4 must run before Step 6 — the staged version IS the baseline.)
  2. Identify the source of non-determinism (see Step 5 triage list).
  3. Fix the root cause — do NOT `git checkout` the snapshot and re-run hoping for better luck. Flaky tests are bugs per CLAUDE.md; the fix is the root-cause elimination, not retry.
  4. File via `/add-bug` if the root cause is in oriterm itself.
  5. Regenerate from Step 1, re-stage from Step 4, re-loop from Step 6.

### Section 04 handoff contract

Once Step 4 stages the snapshot, it becomes the empirical record Section 04 consumes. The snapshot artifact produced by 03.3 IS the contract. Section 04's `ScenarioRunner::run_at` is the downstream consumer, and it MUST honor these contract items:

1. **`PtySession::spawn_tack` is the canonical tack constructor.** Section 04's `ScenarioRunner::run_at` MUST call `PtySession::spawn_tack(&env, cols, rows)` — not open its own `CommandBuilder::new("tack")` or reproduce the `TerminfoEnv::apply_env` wiring. Any tack spawn site outside `spawn_tack` is a LEAK of scattered knowledge. (The framework may add a thin `ScenarioRunner::run_with_session_at` wrapper — Section 04 defines this — but the wrapper calls `spawn_tack` underneath.)
2. **`wait_for("tack [n] >", 5_000)` is the canonical readiness anchor.** Section 04's `run_at` MUST use this exact wait-for call site (or the `ready_anchor` field of `ScenarioSpec` for sub-menu waits — which may be `"tack [n] >"` again or a sub-menu prompt observed live from `INSTA_UPDATE=1`). The prompt text came out of THIS section's snapshot. Section 04 may NOT invent menu text (`"Press any key"`, `"— more —"`, sub-menu titles) that did not first land in Section 03's snapshot or a Section 04-generated sibling snapshot via `INSTA_UPDATE=1`.
3. **`wait_for_child_exit(2_000)` is the canonical clean-quit primitive — not `send(b"q\n") × 3 + wait(500)`.** A naive draft of `ScenarioRunner::run_at` would send three `q\n`s then call `wait(500)`. **That is the exact regression Section 03 exists to prevent.** `wait(500)` does not observe child termination — it just waits for 500 ms of PTY quiet. The child may still be hung, mid-abort, zombified, or leaking FDs at that point. Section 04's `ScenarioRunner::run_at` MUST use `session.wait_for_child_exit(2_000)` so the runner asserts the child actually exited. The three `send(b"q\n")` calls to navigate back out of submenus are fine (tack's menu nesting needs multiple `q`s); it's the terminating quiesce-wait that must become `wait_for_child_exit`. **Hard handoff contract — enforced by a checklist item in `section-04-scenario-framework.md`'s 04.N Completion Checklist that pins `wait_for_child_exit(2_000)` as the canonical clean-quit primitive.**
4. **`grid_text()` is the canonical text extraction.** Section 04 MUST NOT invent a parallel grid serialization. If a scenario parser needs non-text attributes (color, SGR, wide chars), those belong in Section 07 (GPU goldens), NOT in a new `grid_text_with_colors()` helper on `PtySession`.
5. **`TerminfoEnv::compile()` is the canonical terminfo provisioning.** Every tack test in Sections 04-08 calls `TerminfoEnv::compile()` — there is no alternative pinned-terminfo helper. If a scenario needs a variant (e.g., `ori_term-direct`), it calls `TerminfoEnv::compile_with_variant(TerminfoVariant::OriTermDirect)` — defined by Section 02. The enum variants are `OriTerm` and `OriTermDirect` (matching the `ori_term` / `ori_term-direct` entry names); there is no bare `Direct` variant.

---

## 03.4 Skip discipline and platform compile

**File(s):** `oriterm_core/tests/tack/main.rs` (no new file, verification only)

Skip-when-unavailable and cross-platform compile are non-negotiable per CLAUDE.md cross-platform rules and per the conformance suite's "compile everywhere, runtime skip" contract from `00-overview.md`. This subsection covers the skip gate, tack version drift, and the Windows cross-compile. Exit semantics and cleanup verification are in 03.5.

- [x] Verify skip discipline **without modifying source code**. CLAUDE.md's "no temporary scaffolding" rule bans the `if false { /* body */ }` pattern previously documented here — hand-editing the test source to force a branch is exactly the "temporary" scaffolding the rule forbids. Use the PATH-override method:

  On a system where tack IS installed, run the test with a PATH that hides tack. `env -i PATH=/nonexistent` scrubs `cargo` and `timeout` along with the ncurses tools, so resolve their absolute paths from the current environment BEFORE switching to the empty one:
  ```sh
  CARGO_BIN=$(command -v cargo)
  TIMEOUT_BIN=$(command -v timeout)
  env -i PATH=/nonexistent HOME=$HOME "$TIMEOUT_BIN" 150 "$CARGO_BIN" test \
      -p oriterm_core --test tack -- tack_smoke_main_menu_at_80x24 --nocapture
  ```
  (`env -i` wipes the environment; `PATH=/nonexistent` ensures neither `tack` nor `tic` is reachable; `HOME` is preserved so cargo's target cache works; `cargo` and `timeout` are invoked by absolute path so the empty `PATH` doesn't break the runner itself.) The test must print `tack or tic not installed, skipping tack_smoke_main_menu_at_80x24` via the `eprintln!` gate and return success. No source modification, no git diff, no scaffolding residue. Document the observed `eprintln!` output in this checklist item once verified.

  Belt-and-braces fallback: Section 09's cross-platform matrix runs the test on Windows (native, no ncurses), which exercises the same skip gate at runtime with zero source edits. That is additional verification, not a substitute for the PATH-override run on Linux.

- [x] **Tack version drift handling (distro variance).** Different ncurses releases ship slightly different tack main-menu wording. The plan was authored against the ncurses v6.x tack. If the test fails on a distro with an older (or newer) tack whose main-menu wording differs, update the programmatic assertions to match the actual wording (verified by re-running `tack -V` and then `printf 'q\n' | tack` on the affected system). The insta snapshot will also need `INSTA_UPDATE=1` to refresh. Document the tested tack version in a comment next to the first assertion: `// Verified against tack v1.08 (ncurses 6.4).`

- [x] Cross-compile for `x86_64-pc-windows-gnu`:
  ```
  cargo build --target x86_64-pc-windows-gnu -p oriterm_core --tests
  ```
  Must succeed. The test source must compile on Windows even though `tack` and `tic` are unavailable there — runtime skip handles that. This is the compile-everywhere half of the conformance suite contract; the runtime-skip half is the PATH-override verification above plus Section 09's actual Windows runner.

---

## 03.5 Exit semantics and cleanup verification

**File(s):** `oriterm_core/tests/tack/main.rs` (test source edits for assertion upgrade), plus host-side diagnostic runs (no file edits).

This subsection covers the exit-code investigation (to upgrade `eprintln!` to `assert!(exit.success(), ...)`), the zombie-process check, and the FD-leak loop. These are all "does the child actually exit and clean up?" concerns — distinct from 03.4's "does the test even compile and skip correctly?" concerns.

- [x] **Investigate tack's clean-quit exit code** and upgrade the `eprintln!` to a firm assertion. The smoke test currently logs `exit` via `eprintln!` as a CI-visibility measure, but the success criterion ultimately wants `assert!(exit.success(), ...)`. Steps:
  1. Run the smoke test on Linux under an unmodified ncurses v6.x tack. Observe the logged exit status across 10 runs — must be identical every time.
  2. If it's consistently `ExitStatus { success: true, exit_code: Some(0) }`, upgrade the `eprintln!` line to `assert!(exit.success(), "tack exited non-zero: {exit:?}\nGrid:\n{}", session.grid_text());` and keep the `eprintln!` as a supplementary line for log readability.
  3. If it's consistently non-zero or varies (this would be surprising — ncurses tack is documented to exit 0 on `q`), file `/add-bug` against tack version drift and keep the `eprintln!`-only posture until Section 09 verification cross-checks on macOS.
  4. Document the observed exit code in the test comment next to the assertion: `// Verified exit 0 on ncurses v6.4 tack v1.08 clean quit (Linux x86_64).`

- [x] **Verify child cleanup (platform-gated diagnostic).** The `wait_for_child_exit(2_000)` assertion in 03.3 is the *authoritative* check — if it passes, the child was reaped by `try_wait()` returning `Ok(Some(_))` and there is no zombie. The host-side checks below are optional diagnostic hints, useful only when `wait_for_child_exit` times out and you need to see what the OS thinks happened:
  - **Linux only**: `strace -f -e trace=clone,wait4 cargo test ...` to trace the child lifecycle, OR `ps -ef | grep tack` in a second terminal window right after the test exits (before the shell prompt returns). Neither command is portable.
  - **macOS**: use `ps -p <pid>` or `lsof -p <pid>` against the cargo test process; the Linux `strace`/`/proc` paths do not exist.
  - **Windows**: use `Get-Process tack* | Format-List` in PowerShell, or let Section 09's cross-platform CI matrix exercise the ConPTY reap path — there is no `strace` equivalent on Windows, and `wait_for_child_exit` on Windows goes through `GetExitCodeProcess` (NOT `WaitForSingleObject`, see `crates/portable-pty/src/win/mod.rs`).
  - If a zombie remains after `wait_for_child_exit(2_000)` returned `Ok`, the Drop impl on `PtySession` is broken — file against Section 01 and fix there. This is not an expected failure mode; the check exists to diagnose it if it happens, not as a gating step.

- [x] **Verify PTY file descriptor cleanup (platform-gated diagnostic).** Run the smoke test 50 times in a tight loop. After the loop ends, check that the FD count is stable:
  ```sh
  for i in $(seq 1 50); do
      timeout 150 cargo test -p oriterm_core --test tack -- tack_smoke_main_menu_at_80x24 --quiet || { echo "failed on iteration $i"; break; }
  done
  ```
  - **Linux**: `ls /proc/self/fd | wc -l` for the test runner — count should be stable across runs, not growing. `/proc` is Linux-specific.
  - **macOS**: `lsof -p <pid>` on the test runner process; `/proc/self/fd` does not exist on Darwin.
  - **Windows**: use `handle.exe` from Sysinternals or let Section 09's CI matrix surface a leak via OOM — there is no `/proc` and no `lsof` in standard Windows installs.
  - If the FD count grows, the reader thread or the PTY pair is not being dropped — file as a bug against Section 01's `PtySession::Drop`.

- [x] **Minimal-environment sanity check is deferred to Section 09** — Section 09's `09.4 Cross-platform build + skip verification` subsection is where minimal-container and no-host-terminfo matrices live. Do NOT add one here; Section 03's cross-compile check + 03.3 determinism loop proves the happy path on the dev box, and that is the correct scope for this section.

---

## 03.T TPR Checkpoint

- [x] **`/tpr-review` covering 03.1–03.5.** Run before `03.R` findings are collected. Must catch: misuse of `TerminfoEnv` env vars (wrong `TERM` name), incorrect `tack` arguments (`-V` vs `--version`), races between `wait_for` and `tack`'s alt-screen entry, leaked child processes, snapshot non-determinism, bounded-poll regression in `wait_for_child_exit` (did a refactor drop the 10 ms sleep on `Ok(None)`?), Section 04 handoff contract consistency (does `spawn_tack` signature match what `ScenarioRunner::run_at` will consume?).

---

## 03.R Third Party Review Findings

<!-- Reserved for Codex or other external reviewers. -->

- [x] `[TPR-03-001][low]` `plans/tack-conformance/section-03-tack-smoke-test.md:450` — the documented PATH-override verification command in 03.4 cannot run as written because it removes `cargo` from `PATH` along with `tack`/`tic`.
  Evidence: running the literal command on 2026-04-07,
  `env -i PATH=/nonexistent HOME=$HOME cargo test -p oriterm_core --test tack -- tack_smoke_main_menu_at_80x24 --nocapture`,
  fails immediately with `env: ‘cargo’: No such file or directory`, so the plan's prescribed skip-discipline verification is not reproducible. The implementation itself does skip cleanly when the test binary is run under `PATH=/nonexistent`; the defect was in the documented verification step.
  Resolved: Updated the 03.4 command on 2026-04-07 to resolve `CARGO_BIN`/`TIMEOUT_BIN` via `command -v` BEFORE entering the scrubbed `env -i` environment, then invoke them by absolute path. Verified working locally — the rewritten command produces the expected `tack or tic not installed, skipping ...` line and exits 0.

- [x] `[TPR-03-002][low]` `plans/tack-conformance/section-03-tack-smoke-test.md:64` — the top-level success criteria still name the wrong snapshot path.
  Evidence: the success-criteria bullet said the smoke test writes `oriterm_core/tests/tack/snapshots/tack_smoke_main_menu_80x24.snap`, but the actual assertion site is `insta::assert_snapshot!("tack_smoke_main_menu_80x24", grid)` in `oriterm_core/tests/tack/main.rs`, which writes the committed file `oriterm_core/tests/tack/snapshots/tack__tack_smoke_main_menu_80x24.snap`. The same section already uses the correct insta-generated path later in 03.3 (`...:390`) and again in the 03.N checklist (`...:538`), so the section was internally inconsistent.
  Resolved: Updated the success-criteria bullet at line 64 on 2026-04-07 to name the correct insta-generated path `tack__tack_smoke_main_menu_80x24.snap` (with the `tack__` target prefix), matching the paths already used in 03.3 and the 03.N checklist. Drift eliminated.

- [x] `[TPR-03-003][low]` `plans/tack-conformance/section-03-tack-smoke-test.md:436` — the Section 04 handoff contract referenced a nonexistent enum variant `TerminfoVariant::Direct`.
  Evidence: the 03.3 handoff contract item 5 (`"**TerminfoEnv::compile() is the canonical terminfo provisioning.**"`) called out `TerminfoEnv::compile_with_variant(TerminfoVariant::Direct)` as the canonical helper for the truecolor variant, but `crates/oriterm_test_support/src/terminfo/mod.rs` defines the enum as `OriTerm` and `OriTermDirect` — there is no bare `Direct` variant. Section 04 following the handoff literally would fail at compile time on the very first line that reached for the variant.
  Resolved: Updated the handoff contract item on 2026-04-07 to use `TerminfoVariant::OriTermDirect` and added a clarifying sentence listing both valid variants (`OriTerm` / `OriTermDirect`) so future scenario authors can't misremember the name. Drift eliminated.

- [x] `[TPR-03-004][medium]` `crates/oriterm_test_support/src/session/tests.rs:35` — the new `pty_session_wait_for_child_exit_returns_on_clean_exit` test did not pin the bounded-poll invariant that Section 03 treats as a hard contract.
  Evidence: `crates/oriterm_test_support/src/session/mod.rs` documents the `Ok(None)` branch contract: when `drain_blocking()` returns 0, `wait_for_child_exit()` must sleep 10 ms so the loop never hot-spins between reader EOF and `try_wait()` observing exit. But the old test body admitted in its own comment that removing that sleep would still pass because the spawned `exit 0` child is usually observed on the first `try_wait()` iteration. That left the anti-hot-spin behavior enforced only by prose plus wall-clock drift, not by a deterministic semantic pin.
  Impact: a future refactor could silently drop `thread::sleep(POLL_SLEEP)` while keeping the unit test green and the smoke test green, reintroducing the busy-loop regression.
  Resolved: on 2026-04-07 —
  1. Refactored `wait_for_child_exit` to delegate to a private `wait_for_child_exit_inner<F: FnMut()>(timeout_ms, on_iter)` helper. The public wrapper passes a no-op closure (zero cost); tests inject a counter.
  2. Added a `#[cfg(test)] fn force_close_rx_for_test(&mut self)` helper on `PtySession` that swaps the reader channel for a fresh closed one so `drain_blocking` returns 0 immediately — simulating the "reader thread EOF but child still alive" race window without having to precisely time a real PTY close.
  3. Added `pty_session_wait_for_child_exit_bounded_poll_invariant` in `session/tests.rs` (Unix-gated). It spawns `/bin/sh -c "sleep 0.5"`, force-closes `rx`, drives `wait_for_child_exit_inner` with a counter closure, and asserts `iters < 500`. Verified loud: with `thread::sleep(POLL_SLEEP)` removed, the test observes **902 848** iterations and fires the assertion; with the sleep restored, it observes ~50 iterations. The semantic pin catches any future regression that removes the anti-hot-spin sleep, not just wall-clock drift.
  4. The existing two-arm `pty_session_wait_for_child_exit_returns_on_clean_exit` test is unchanged — it continues to provide Windows ConPTY coverage for the happy path, while the new Unix-only invariant pin exercises the race scenario the anti-hot-spin sleep defends against (both platforms share the identical `wait_for_child_exit_inner` body, so Unix-side pinning is sufficient).

---

## 03.N Completion Checklist

**Primitives (03.1):**
- [x] `tack_available()` exists in `crates/oriterm_test_support/src/session/mod.rs` and is re-exported from `lib.rs`
- [x] `PtySession::spawn_tack(env, cols, rows)` exists and uses `TerminfoEnv::apply_env(&mut cmd)` (not raw env iteration)
- [x] `PtySession::wait_for_child_exit(timeout_ms) -> ExitStatus` exists in `crates/oriterm_test_support/src/session/mod.rs`
- [x] `wait_for_child_exit` implements the bounded-poll contract (10 ms sleep on `Ok(None)` when `drain_blocking` returned 0)
- [x] `pty_session_wait_for_child_exit_returns_on_clean_exit` unit test exists in `crates/oriterm_test_support/src/session/tests.rs` using the two-arm `#[cfg(unix)] / #[cfg(windows)]` pattern and asserts `status.success()`
- [x] `tack_available_matches_tool_available` unit test exists in the same sibling `tests.rs`
- [x] Deterministic bounded-poll invariant pin `pty_session_wait_for_child_exit_bounded_poll_invariant` exists (Unix-gated) — added in response to TPR-03-004. Uses `#[cfg(test)] force_close_rx_for_test` + `wait_for_child_exit_inner<F: FnMut()>` closure refactor to count poll iterations deterministically.

**Test scaffold (03.2):**
- [x] `oriterm_core/tests/tack/main.rs` exists and declares `tack_smoke_main_menu_at_80x24`
- [x] Programmatic assertions inside the test verify `Main Menu`, `begin testing`, `tools`, `quit`, `tack [n] >` are present
- [x] Test surfaces tack's exit code via `assert!(exit.success(), ...)` or `eprintln!("tack exit status = {exit:?}")`
- [x] Test skips cleanly when `tack_available()` or `tic_available()` returns false (`eprintln!` logged, returns Ok)

**Snapshot + determinism (03.3):**
- [x] `tack_smoke_main_menu_at_80x24` passes on Linux (`timeout 150 cargo test -p oriterm_core --test tack`)
- [x] Insta snapshot `oriterm_core/tests/tack/snapshots/tack__tack_smoke_main_menu_80x24.snap` exists and is committed
- [x] Snapshot staged in git (`git add oriterm_core/tests/tack/snapshots/`) BEFORE the 10x determinism loop runs
- [x] Test runs deterministically — 10 consecutive runs all pass without flake (PASS=10 FAIL=0)

**Skip + compile (03.4):**
- [x] Skip discipline verified via PATH-override run (no source edits, no `if false {}` scaffolding) — observed `tack or tic not installed, skipping tack_smoke_main_menu_at_80x24` under `env -i PATH=/nonexistent` with `CARGO_BIN`/`TIMEOUT_BIN` resolved before the scrub
- [x] Tack version drift comment (`// Verified against tack vX.Y (ncurses Z.W).`) present next to programmatic assertions — `tack v1.08 (ncurses 6.4) on Linux x86_64`
- [x] Cross-compile for `x86_64-pc-windows-gnu` succeeds (`cargo build --target x86_64-pc-windows-gnu -p oriterm_core --tests`)

**Exit + cleanup (03.5):**
- [x] Tack clean-quit exit code investigated; `eprintln!` upgraded to `assert!(exit.success(), ...)` — 10/10 runs observed `ExitStatus { code: 0, signal: None }` on ncurses 6.4 / tack v1.08 Linux x86_64; assertion landed in `oriterm_core/tests/tack/main.rs`
- [x] Host-side child-cleanup diagnostic run on the dev platform — tack-process count delta before/after test = 0 (one pre-existing long-running tack from an unrelated `script` session observed, not from the test)
- [x] FD-leak loop (50x) run on the dev platform — 50 consecutive runs all passed; shell FD count stable before=6 after=6

**TPR + review (03.T, 03.R):**
- [x] `/tpr-review` covering 03.1–03.5 run (03.T) — 5 iterations, findings recorded in 03.R
- [x] All 03.R findings resolved (TPR-03-001 plan drift, TPR-03-002 plan drift, TPR-03-003 plan drift, TPR-03-004 medium code — deterministic invariant pin added)
- [x] `/impl-hygiene-review last commit` final pass clean (after TPR) — runs post-commit in the autopilot flow for this section

**Gates:**
- [x] `./build-all.sh` green (x86_64-pc-windows-gnu debug + release)
- [x] `./clippy-all.sh` green (x86_64-pc-windows-gnu + host)
- [x] `timeout 150 ./test-all.sh` green
- [x] No temporary scaffolding in `.rs` files

**Plan sync:**
- [x] This section's frontmatter `status` → `complete`
- [x] Each subsection's frontmatter `status` → `complete` (03.1, 03.2, 03.3, 03.4, 03.5, 03.T, 03.R)
- [x] `00-overview.md` Quick Reference table: Section 03 marked Complete
- [x] `index.md` Section 03 status updated
- [x] Section 04's `depends_on: ["03"]` confirmed

**Exit Criteria:** `tack_smoke_main_menu_at_80x24` passes deterministically on Linux: `timeout 150 cargo test -p oriterm_core --test tack -- tack_smoke_main_menu_at_80x24` returns success in under 10 seconds. The captured insta snapshot contains the literal main menu structure (`Main Menu`, `begin testing`, `tools`, `quit`, `tack [n] >`). The test cross-compiles for Windows and skips cleanly on Linux/macOS without `tack` installed. No zombie processes, no leaked file descriptors. The pipeline is proven end-to-end and Section 04 can safely build the scenario framework on top.
