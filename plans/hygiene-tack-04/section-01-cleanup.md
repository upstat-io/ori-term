---
section: "01"
title: "Cleanup"
status: not-started
reviewed: false
goal: "Resolve the 24 hygiene findings (8 Major + 16 Minor) from the impl-hygiene review of the Section 04 tack-conformance scenario framework slice, then delete the plan directory."
depends_on: []
third_party_review:
  status: none
  updated: null
sections:
  - id: "01.A"
    title: "Cluster A — runner/mod.rs polish"
    status: not-started
  - id: "01.B"
    title: "Cluster B — Test helper duplication (extract test_helpers.rs)"
    status: not-started
  - id: "01.C"
    title: "Cluster C — Drift + Gap"
    status: not-started
  - id: "01.D"
    title: "Sub-cluster D — Named constants"
    status: not-started
  - id: "01.E"
    title: "Sub-cluster E — Boundary error handling"
    status: not-started
  - id: "01.F"
    title: "Sub-cluster F — Surface polish"
    status: not-started
  - id: "01.G"
    title: "Sub-cluster G — Performance"
    status: not-started
  - id: "01.H"
    title: "Sub-cluster H — Optional duplication followup"
    status: not-started
  - id: "01.Z"
    title: "Cleanup"
    status: not-started
---

# Section 01: Cleanup

**Goal:** Walk every Major and Minor hygiene finding from the impl-hygiene review of
the Section 04 tack-conformance slice (`ce305091..efec3818`), fix each one, then
delete this plan directory.

**Severity ordering:** Major findings first (8), then Minor (16). Within each tier,
fix clusters together — extracting `test_helpers.rs` resolves 4 DRY findings in one
go, naming constants resolves 5 magic-number findings together, etc.

**Verification cadence:** after each cluster, run `cargo test -p oriterm_test_support`
and `cargo clippy -p oriterm_test_support --all-targets -- -D warnings`. After every
checked item, the test suite must still be green.

---

## 01.A Cluster A — runner/mod.rs polish (Major × 3)

- [ ] **LEAK-04-05** — `parse_modes_screen` re-derives `default_parser` header extraction.
      `crates/oriterm_test_support/src/tack_framework/scenarios/modes.rs:103-108` and
      `tack_framework/parser/mod.rs:43-50` both contain the byte-for-byte identical:
      ```rust
      let header = grid.lines().map(str::trim).find(|line| !line.is_empty()).unwrap_or("").to_string();
      ```
      **Fix**: extract `pub(crate) fn extract_header(grid: &str) -> String` in
      `parser/mod.rs`. Both `default_parser` and `parse_modes_screen` call it. Add a
      sibling test in `parser/tests.rs` for the helper.

- [ ] **LEAK-04-06 (residual)** — finish naming the unnamed `5_000` ms timeouts.
      `0b0806f2` introduced `MAIN_MENU_READY_TIMEOUT_MS` and `READY_ANCHOR_TIMEOUT_MS`
      in `runner/mod.rs`, but the `STEP_TIMEOUT_MS` constant in
      `tack_framework/navigator/mod.rs:30` and the `wait_for("__READY__", 5_000)`
      callsites in test helpers (`teardown/tests.rs`, `runner/tests.rs`) still inline
      `5_000`. **Fix**: audit `crates/oriterm_test_support/` for the literal `5_000`
      and replace any timeout uses with named constants. Cross-reference the existing
      navigator constant rather than introducing a duplicate.

- [ ] **LEAK-04-07 (residual)** — finish replacing `"tack [n] >"` literals.
      `0b0806f2` introduced `TACK_MAIN_MENU_PROMPT` in `runner/mod.rs` and routes
      both `prepare_and_navigate` callsites through it. The smoke test in
      `oriterm_core/tests/tack/main.rs:56` still inlines the literal:
      ```rust
      session.wait_for("tack [n] >", 5_000);
      ```
      **Fix**: re-export `TACK_MAIN_MENU_PROMPT` from `oriterm_test_support` (move it
      out of `runner/mod.rs` private scope into `tack_framework/mod.rs` or a new
      `tack_framework::constants` module). Update the smoke test to use the const.

---

## 01.B Cluster B — Test helper duplication (Major × 4 → one extraction)

This cluster resolves all four DRY findings via a single new module:
`crates/oriterm_test_support/src/test_helpers.rs` (gated `#[cfg(test)]`,
`pub(crate)`).

- [ ] **DRY-04-01** — `spawn_quit_on_keystroke(exit_code: i32) -> PtySession`
      duplicated byte-for-byte across `session/teardown/tests.rs:115-149` and
      `tack_framework/runner/tests.rs:43-77`. The `runner/tests.rs:40` comment
      already cross-references the teardown copy as a known smell.
      **Fix**: move to `test_helpers::spawn_quit_on_keystroke(exit_code: i32)`. Both
      consumers import it. Delete both copies.

- [ ] **DRY-04-02** — `spawn_silent_long_lived` (sync/tests.rs:11-27) and
      `spawn_silent_child` (navigator/tests.rs:10-27) do the same thing — spawn a
      long-lived silent child for timeout/bounded-poll tests — with different
      command bodies (`sleep 10` vs `cat`/`findstr`).
      **Fix**: pick one canonical helper (`sleep 10` is cleaner because it's
      universal), promote to `test_helpers::spawn_silent_long_lived()`, delete both
      copies, update consumers.

- [ ] **DRY-04-03** — panic-payload downcast block duplicated 3× across
      `sync/tests.rs:78-85`, `teardown/tests.rs:195-202`, `runner/tests.rs:127-134`:
      ```rust
      let msg = if let Some(s) = payload.downcast_ref::<String>() {
          s.clone()
      } else if let Some(s) = payload.downcast_ref::<&'static str>() {
          (*s).to_string()
      } else {
          String::from("<non-string panic payload>")
      };
      ```
      **Fix**: extract `test_helpers::panic_payload_to_string(payload: Box<dyn Any +
      Send>) -> String`. All three sites delegate.

- [ ] **DRY-04-04** — 9× `#[cfg(unix)]/#[cfg(windows)] CommandBuilder` boilerplate
      across all test files. Each block is 8-14 lines.
      **Fix**: extract `test_helpers::shell_command(unix_script: &str, windows_script:
      &str) -> CommandBuilder` (or two separate helpers if call sites prefer named
      args). All 9 sites collapse to single-line calls.

- [ ] **Verify**: after `test_helpers.rs` lands, all 4 DRY findings collapse to a
      single shared module. Run `cargo test -p oriterm_test_support` and confirm the
      test count is unchanged (47+ tests) and all are green.

---

## 01.C Cluster C — Drift + Gap (Major × 2)

- [ ] **BND-04-01** — `quit_tack` drain timeout drift between plan, test comment, and impl.
      - Plan (`section-04-scenario-framework.md:475`) says "200 ms drain".
      - Test comment (`teardown/tests.rs:159`) does arithmetic on `5 × (200 ms drain
        + 10 ms idle) ≈ 1050 ms`.
      - Impl (`session/teardown/mod.rs:124`) uses `drain_blocking(150)`.
      **Fix**: pick one value. The cleanest path: introduce
      `const INTER_Q_DRAIN_MS: u64 = 150;` in `session/teardown/mod.rs`. Update the
      test comment to derive from the constant. Update the section-04 plan body
      prose if it ships any further version.

- [ ] **BND-04-02** — `LiveSession` has no `Drop` guard / no `#[must_use]`. The
      cleanup contract is purely prose: 3 places in
      `tack_framework/runner/mod.rs:200-203, 237-241, 290-296` warn _"Caller MUST
      call `finish` after rendering"_, but a Section 07 caller can do
      `let _live = ScenarioRunner::run_with_session_at(...); render(&_live.session);`
      and silently drop the exit-status assertion via `PtySession::drop`. Per
      impl-hygiene.md § "Temporal Coupling & RAII Guards" and § "Invariant
      Explicitness", implicit invariants are invisible regressions.
      **Fix**: implement option 1 from the review:
      1. Add `finished: bool` field to `LiveSession` (defaults to `false`).
      2. `LiveSession::finish` sets `self.finished = true` before calling
         `finish_and_assert`.
      3. `impl Drop for LiveSession { fn drop(&mut self) { if !self.finished &&
         !std::thread::panicking() { panic!("LiveSession dropped without
         finish() — exit-status assertion was skipped, see runner/mod.rs"); } } }`.
      4. Add a `#[should_panic]` test in `runner/tests.rs` that constructs a
         `LiveSession` via `new_for_test`, drops it without calling `finish`, and
         asserts the panic message contains "dropped without finish".
      5. Update Section 07's plan section to call `live.finish()` even on the
         render-error path (use a `defer!`-style scoped guard if needed).

---

## 01.D Sub-cluster D — Named constants (Minor × 5)

All five findings extract magic numbers into named constants. Group commit.

- [ ] **LEAK-04-08** — `snapshot_name` could consume `size_label` but reimplements
      the `{cols}x{rows}` sub-format. **Fix**: optional consolidation. Either
      `scenario_name` calls `size_label` internally OR accept the duplication
      because `scenario_name` works on `(u16, u16)` tuples without a session
      reference. Lean toward "accept" — the duplication is 2 chars and the
      argument shape differs.

- [ ] **LEAK-04-09** — Hardcoded `drain_blocking(50)` and `drain_blocking(150)`.
      **Fix**: `const POLL_DRAIN_BLOCK_MS: u64 = 50;` in `session/sync/mod.rs`,
      `const QUIT_DRAIN_MS: u64 = 150;` in `session/teardown/mod.rs`. (The latter
      is the same constant as BND-04-01 if you do BND-04-01 first.)

- [ ] **HYG-04-08** — `wait(300)` quiesce inside `send` is unnamed. **Fix**:
      `const POST_SEND_QUIESCE_MS: u64 = 300;` at the top of `session/mod.rs`.

- [ ] **HYG-04-09** — `wait_for_child_exit(2_000)` in `quit_tack` Phase 2 is
      unnamed. **Fix**: `const QUIT_PHASE2_TIMEOUT_MS: u64 = 2_000;` in
      `session/teardown/mod.rs`. Cross-reference `MAIN_MENU_READY_TIMEOUT_MS` /
      `READY_ANCHOR_TIMEOUT_MS` from `runner/mod.rs` if the values diverge so a
      reader can see the rationale.

- [ ] **HYG-04-10** — Already covered by LEAK-04-09 (same `drain_blocking(50)`
      site). Mark `[x]` together.

---

## 01.E Sub-cluster E — Boundary error handling (Minor × 3)

- [ ] **BND-04-03** — `LiveSession::session` is `pub` (`runner/mod.rs:245-261`),
      exposing PtySession internals (writer, child, proc) through the wrapper
      boundary. Section 07's only legitimate need is `session.term()` for grid
      inspection, which is already public. **Fix**: change `pub session: PtySession`
      to private. Add narrow accessor methods (`live.term()`, `live.cols()`,
      `live.rows()`) for what Section 07 actually needs. Leave `facts` /
      `scenario_id` / `screen_id` / `cols` / `rows` pub if they were already pub.

- [ ] **BND-04-04** — `PtySession::send_raw` swallows BOTH write and flush errors:
      `let _ = self.writer.write_all(key); let _ = self.writer.flush();`
      (`session/mod.rs:242-245`). The teardown justification is documented but the
      method is now `pub`, so future callers may consume it expecting normal write
      semantics. **Fix**: either restrict to `pub(crate)` (teardown-only — `quit_tack`
      is the sole caller) OR change the signature to `pub fn send_raw(&mut self,
      key: &[u8]) -> io::Result<()>` and have `quit_tack` explicitly `let _ = ...` at
      the call site so the swallow is visible.

- [ ] **BND-04-05** — PTY reader thread silently drops read errors
      (`session/mod.rs:120`):
      ```rust
      match pty_reader.read(&mut buf) {
          Ok(0) | Err(_) => break,
          ...
      }
      ```
      Any read error collapses to "EOF", and a broken reader thread produces an
      empty grid forever with no diagnostic. **Fix**: distinguish `Ok(0)` (legit EOF)
      from `Err(e)` and log the error via `eprintln!("PtySession reader thread:
      {e}")` before the break. Test-support code is the one place `eprintln!` is
      legitimate per CLAUDE.md style.

---

## 01.F Sub-cluster F — Surface polish (Minor × 5)

- [ ] **HYG-04-01** — `feed_and_flush` private helper sandwiched between public
      methods in `sync/mod.rs:86-95`. Per `code-hygiene.md` § "Impl Block Method
      Ordering": private helpers go LAST. **Fix**: move `feed_and_flush` to after
      `wait_for_any` at the bottom of the impl block.

- [ ] **HYG-04-02** — `tack_framework/spec.rs:1` is missing `//!` module doc.
      Every other file in `tack_framework/` has one. **Fix**: add a `//!` block
      explaining that the file owns the `MenuStep` + `ScenarioSpec` data types
      (pure data, no I/O).

- [ ] **HYG-04-03** — `parse_modes_screen` (`scenarios/modes.rs:88`) is missing
      `#[must_use]`. Compare to `default_parser` which correctly has it. **Fix**:
      add `#[must_use]`.

- [ ] **HYG-04-04** — `LiveSession::finish` is missing `#[must_use]`. Weaker
      finding than BND-04-02 (which adds the Drop bomb), but if the user opts for
      `#[must_use]` ALONGSIDE Drop, this gets the lint coverage. **Fix**: add
      `#[must_use = "the returned ExitStatus may carry diagnostic info; explicit
      ignore via let _ = ..."]`.

- [ ] **HYG-04-07** — `session/sync/mod.rs:7-10` module doc cross-references a
      plan file (`plans/tack-conformance/section-04-scenario-framework.md`) that
      will be archived to `plans/completed/` once Section 04 is done. The link
      becomes stale at archive time. **Fix**: inline the rationale in 1-2
      sentences and remove the path reference. The "see plan" pattern fails the
      "self-contained code" test.

---

## 01.G Sub-cluster G — Performance (Minor × 1)

- [ ] **HYG-04-05** — `grid_text()` builds `Vec<Vec<char>>` via `grid_chars()`,
      then allocates a `String` per row, then `push_str`s into a final `String`.
      Called once per `poll_until` iteration — ~500 times per 5-s timeout case.
      Test-suite wall-clock impact only, but real. **Fix**: write directly into a
      single `String::with_capacity((lines + 1) * cols)` inside `grid_text()`,
      bypassing `grid_chars()`. Keep `grid_chars` for consumers that actually need
      the 2D form. Add a benchmark or wall-clock pin if the change is contentious.

---

## 01.H Sub-cluster H — Optional duplication followup (Informational × 1)

- [ ] **DRY-04-05** — Three bounded-poll wall-clock invariant tests
      (`pty_session_wait_for_with_context_bounded_poll_invariant`,
      `pty_session_wait_for_any_bounded_poll_invariant`,
      `pty_session_wait_for_child_exit_bounded_poll_invariant`) share an
      almost-identical wall-clock skeleton. The duplication is INTENTIONAL — each
      pin is load-bearing per the section 04 success criteria ("a regression in any
      single consumer's loop body fires its own test"). **Decision**: leave
      duplicated. Mark `[x]` resolved with rejection rationale.

---

## 01.Z Cleanup (BLOCKING — runs LAST)

- [ ] Run `timeout 150 cargo test -p oriterm_test_support` — green
- [ ] Run `timeout 150 cargo test -p oriterm_core --test tack` — green
- [ ] Run `cargo clippy -p oriterm_test_support --all-targets -- -D warnings` — green
- [ ] Run `./test-all.sh` — green
- [ ] Run `./clippy-all.sh` — green
- [ ] Run `./build-all.sh` — green
- [ ] Run 10 consecutive `cargo test --release -p oriterm_core --test tack` — all 10 pass
- [ ] Commit final cleanup batch via `/commit-push`
- [ ] **Delete this plan directory**: `rm -rf plans/hygiene-tack-04/`
- [ ] Commit the deletion with `chore: archive completed hygiene-tack-04 cleanup plan`
- [ ] Push

---

**Exit Criteria:** Every `[ ]` above is `[x]`, the entire `plans/hygiene-tack-04/`
directory has been deleted, and the deletion has been committed and pushed. The
slice has been hygiene-clean across all 4 review passes (LEAK/SSOT, Algorithmic
DRY, Boundary/Flow, Surface) since the last cleanup batch landed.
