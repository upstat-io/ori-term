---
section: "04"
title: "Scenario Catalog Framework"
status: in-progress
reviewed: true
goal: "Build a structured scenario catalog framework inside `crates/oriterm_test_support` (NOT inside `oriterm_core/tests/`) so both text tests in `oriterm_core/tests/tack/` AND GPU golden tests in `oriterm/src/gpu/visual_regression/tack/` (Section 07) can consume the same `ScenarioSpec`/`TackNavigator`/`ScenarioRunner`. The framework lives in `oriterm_test_support::tack_framework` from the start — no later lift needed. Prevents the fragile regex-over-whole-grid antipattern by giving every scenario a structured outcome and per-scenario assertions."
success_criteria:
  - "`crates/oriterm_test_support/src/tack_framework/mod.rs` exists and exposes `ScenarioSpec`, `TackNavigator`, `ScenarioRunner`, `ScenarioOutcome`, `LiveSession` from the workspace-internal test-support crate"
  - "Re-exported via `oriterm_test_support::tack_framework::*` so both `oriterm_core/tests/tack/` (text scenarios in Sections 05-06) and `oriterm/src/gpu/visual_regression/tack/` (GPU goldens in Section 07) can import the same types"
  - "`crates/oriterm_test_support/src/tack_framework/scenarios/` directory exists with `mod.rs` declaring submodules and `pub mod scenarios;` is wired through `tack_framework/mod.rs` — Section 04 owns the contract; Sections 05-08 add submodule files only"
  - "`ScenarioSpec { id, menu_path, ready_anchor, quit_path, parser }` struct holds the semantic ID, navigation steps, readiness check, optional per-scenario quit override, and per-scenario parser"
  - "`MenuStep { send, wait_for, or_wait_for }` supports an optional set of alternate ready anchors (for pagers, 'press any key' prompts, alternate sub-menu wording) — `or_wait_for` defaults to an empty slice"
  - "`crates/oriterm_test_support/src/session/mod.rs` is split into submodules BEFORE the new primitives land: `session/sync/mod.rs` (wait_for, wait_for_with_context, wait_for_any, wait, drain polling, private `poll_until` helper), `session/teardown/mod.rs` (wait_for_child_exit, quit_tack, force_close_rx_for_test), with `session/mod.rs` retained as the dispatch hub holding type defs, constructors (spawn/spawn_vttest/spawn_tack), accessors (including send and send_raw), and the `Drop` impl. This is enforced by Section 04 because (a) adding `wait_for_with_context`, `wait_for_any`, `quit_tack`, `send_raw`, and their tests to the existing 468-line `mod.rs` pushes it well over the hard 500-line limit from code-hygiene.md, (b) 04.0.b introduces `wait_for_with_context` + `wait_for_any` as two additional polling skeletons on top of the existing `wait_for`/`wait_for_child_exit_inner` pair — crossing the 3-instance algorithmic-duplication threshold that demands a canonical `poll_until` home, and (c) the sibling `tests.rs` (currently 189 lines) grows proportionally and needs the mirror split to stay navigable"
  - "`PtySession::wait_for` is extended to accept a panic-message context closure (`fn wait_for_with_context(needle, timeout_ms, ctx: impl Fn(&str) -> String)`) so `TackNavigator` and `ScenarioRunner` get rich panic messages without re-implementing the loop. The plain `wait_for` keeps its existing signature and delegates. A private `poll_until<P>` helper in `session/sync/mod.rs` captures the bounded-poll skeleton shared by `wait_for`, `wait_for_with_context`, and `wait_for_child_exit_inner` — three sites with the same deadline-loop-drain-sleep structure = `LEAK:algorithmic-duplication`, fixed at the canonical home now rather than deferred"
  - "`PtySession::wait_for_any(anchors: &[&str], timeout_ms: u64) -> Option<usize>` is the non-panicking alternate-anchor primitive: returns `Some(idx)` when any of the anchors matches inside the timeout, `None` on timeout. `TackNavigator` uses this for `MenuStep::or_wait_for` matching — NO `std::panic::catch_unwind` panic-as-control-flow workaround anywhere in the navigator. Lives in `session/sync/mod.rs` alongside `wait_for_with_context` so the three loop bodies share `poll_until`"
  - "`PtySession::send_raw(&[u8])` exists as a companion to `send()` that writes+flushes WITHOUT the 300ms post-write quiesce. `quit_tack` consumes `send_raw` so q-loop iterations don't spend 300ms apiece on the internal drain; the 200ms drain `quit_tack` does between iterations is the correct quiesce for its purpose. `send_raw` lives in `session/mod.rs` (or a `session/io.rs` submodule if the split plan places it there) alongside `send()` — the Mi1 'hypothetical future lever' is no longer hypothetical"
  - "`TackNavigator::navigate(session, &menu_path)` walks tack from the main menu through each step, calling `wait_for_any` (for the combined primary + alternates set) between every keystroke — no fixed sleeps, no `catch_unwind`. BEFORE every send, the navigator snapshots the current grid and panics if the step's `wait_for` (or any `or_wait_for` alternate) is ALREADY present in the pre-send grid (the pre-existing-anchor guard from C1)"
  - "`grid_has_token` (whitespace-bounded), `grid_has_paren_token` (matches tack's `(cap_name)` parenthesized output format), `grid_line_starts_with`, `grid_find_field` (with both-boundary token check + all-hits-per-line scan) parser helpers live in `tack_framework::parser::tokens` and replace blind `grid.contains(short_label)` checks across all per-scenario parsers"
  - "`ScenarioRunner::run(spec) -> ScenarioOutcome` ties it all together: spawn tack via spawn_tack, call TackNavigator, capture grid_text, call the per-scenario parser, return ScenarioOutcome with grid + parser-extracted facts"
  - "`ScenarioRunner::run_at(spec, cols, rows)` asserts `exit.success()` on the captured `ExitStatus` and panics with both the exit status AND the captured grid on failure — exit status is NEVER thrown away"
  - "Quit teardown is state-aware via `PtySession::quit_tack(max_iterations)`: send one `q\\n`, observe `try_wait()` after each, stop on first observed exit, max 5 iterations. Per-scenario quit overrides via `ScenarioSpec::quit_path` (default = `quit_tack`)"
  - "`ScenarioRunner::run_with_session_at(spec, cols, rows) -> LiveSession` returns a wrapper holding the live `PtySession` AND the `TerminfoEnv` (so it outlives the session) — used by Section 07 GPU goldens to render the live session through the GPU pipeline"
  - "`LiveSession::finish(self) -> ExitStatus` consumes the wrapper, calls the SAME `quit_tack` helper that `run_at` uses, asserts exit success, and reaps the child — Section 07 callers MUST call `live.finish()` after rendering instead of relying on `Drop`"
  - "`ScenarioOutcome { scenario_id, screen_id, cols, rows, grid_text, parsed }` carries size-aware identity. `scenario_id` is the semantic test name (`tack_modes_am`); `screen_id` is the dedupable screen identity (`tack_modes`) so size-matrix runs share insta snapshots when navigation produces the same screen"
  - "`ScenarioOutcome::snapshot_name()` returns `\"<screen_id>_<cols>x<rows>\"` and `golden_name()` returns the same for PNG goldens — single source of truth for snapshot/golden naming"
  - "One end-to-end scenario `tack_modes_am` passes: navigates `[n] -> [x] -> [n]` (begin testing -> modes -> run standard tests), uses sub-menu-specific anchors (`tack/test [n] >`, `tack/test/mode [n] >`, `Done`), captures grid, asserts via insta snapshot AND a parser-extracted assertion using `grid_has_paren_token` (the tokenized helper for tack's `(cap_name)` parenthesized output format — substring-collision-safe by construction). Tack's modes test scrolls many caps off the visible viewport before reporting `Done`; the always-visible terminator is `(os)` (over-strike), so the wrapper test asserts on `os` rather than `am`. The test still proves the framework end-to-end (spawn -> navigate -> capture -> tokenized parse -> snapshot -> clean exit) and the spec contract (tokenized helper, never raw `grid.contains`)."
  - "ScenarioSpec is `Send` and constructible at module scope as a `const` or `static` (so test catalogs can list scenarios in arrays)"
  - "Framework gracefully handles tack going off-script: if `wait_for_with_context` times out at any navigation step, panic with a clear error including the current grid, the menu_path step index, the bytes sent, and the anchor it was waiting for"
  - "`timeout 150 cargo test -p oriterm_core --test tack -- tack_modes_am` passes on Linux"
  - "Section 03 handoff contract item 3 (`wait_for_child_exit(2_000)` as canonical clean-quit) is HONORED via the `quit_tack(5)` superset: `quit_tack` calls `try_wait()` after every `q\\n` send, returns the moment the child exits, and panics on overflow with the current grid — the same exit-observation guarantee as `wait_for_child_exit` plus the state-aware quit-key-send loop. The 04.N completion checklist pins this reconciliation explicitly so a future reviewer reading Section 03 doesn't conclude the contract was bypassed"
  - "Section 04 satisfies mission-tracing for the FOUNDATION half of mission criteria #5/#7: 'tack test scenarios cover modes/glitches/...' (one proven scenario for the modes screen), 'Text snapshots (insta) exist for tack screens at 80x24' (one snapshot via `outcome.snapshot_name()`). Sections 05-06 fill the rest of the catalog ON TOP of this foundation. The framework also unblocks mission criterion #8 (GPU goldens) because Section 07 consumes `LiveSession::finish` defined here"
  - "Cross-section consumer contract is locked: Section 05 (test-menu catalog), Section 06 (tools-menu catalog), and Section 07 (GPU goldens) consume EXACTLY the API shapes defined here — `MenuStep::new`, `ScenarioSpec` with `screen_id`/`quit_path`, `outcome.snapshot_name()`, `outcome.golden_name()`, `LiveSession::finish`, scenarios living in `tack_framework::scenarios::*`. Sections 05/06/07 currently reference an OLDER API shape (pre-Agent-1 expansion) and MUST be re-reviewed before implementation; their `reviewed: false` flag enforces this gate"
  - "Bounded-poll invariant is pinned per-consumer: `pty_session_wait_for_with_context_bounded_poll_invariant` (new), `pty_session_wait_for_any_bounded_poll_invariant` (new), and `pty_session_wait_for_child_exit_bounded_poll_invariant` (pre-existing, from Section 03) together prove `poll_until` preserves its 10ms idle-sleep discipline across every caller. A regression in any single consumer's loop body fires its own test rather than being masked by a shared-helper-only test"
  - "`LiveSession::finish` has two direct unit tests — `live_session_finish_asserts_clean_exit_via_quit_tack` (semantic pin that `finish` actually exercises `quit_tack` and doesn't shortcut to `drop(self)`) and `live_session_finish_panics_on_non_success_exit` (semantic pin for the C3 exit-success assertion inside `finish`). Without these tests, a regression replacing `finish`'s body with a no-op drop or removing the `assert!(exit.success())` would pass every other test in Section 04"
  - "Failing-test-first discipline enforced end-to-end: every test item in 04.0.b + 04.3's `LiveSession::finish` tests + 04.4's `tack_modes_am` wrapper is written as a failing test BEFORE its implementation lands (mirroring Section 02's 02.2 ordering rule). 04.0.b also adds a checklist line confirming TDD ordering was honored"
  - "Debug AND release parity: 04.4's `tack_modes_am` runs 10 iterations under `cargo test` (debug) AND `cargo test --release`. Any release-only flake is a timing bug that must be fixed in Section 04 — no 'release flake' deferral"
inspired_by:
  - "ori_term teseq ScenarioSpec (plans/completed/teseq-conformance/section-01-infrastructure.md:95-156 — TerminalConfig + SetupConfig + ExpectConfig pattern)"
  - "ori_term vttest menu walking (oriterm_core/tests/vttest/menu6.rs::walk_menu6_subscreens — same `wait_for` + send-keystroke + drain pattern this framework formalizes)"
  - "Alacritty ref tests (alacritty_terminal/tests/ref.rs — scenario-directory + sidecar config + grid assertion)"
depends_on: ["03"]
third_party_review:
  status: resolved
  updated: 2026-04-07
sections:
  - id: "04.0.a"
    title: "Split session/mod.rs into submodules ahead of adding new primitives (BLOAT prevention)"
    status: complete
  - id: "04.0.b"
    title: "PtySession primitives: wait_for_with_context + wait_for_any + quit_tack + send_raw"
    status: complete
  - id: "04.1.a"
    title: "ScenarioSpec + MenuStep types (spec.rs) and lib.rs/tack_framework wiring"
    status: complete
  - id: "04.1.b"
    title: "Parser types + tokenized parser helpers + scenarios module skeleton"
    status: complete
  - id: "04.2"
    title: "TackNavigator: pre-grid guard + walk menu_path with wait_for_any (no catch_unwind)"
    status: complete
  - id: "04.3"
    title: "ScenarioRunner: spawn_tack + navigate + capture + parse + state-aware quit + LiveSession::finish"
    status: complete
  - id: "04.4"
    title: "End-to-end scenario tack_modes_am"
    status: complete
  - id: "04.R"
    title: "Third Party Review Findings"
    status: complete
  - id: "04.N"
    title: "Completion Checklist"
    status: not-started
---

# Section 04: Scenario Catalog Framework

**Status:** Not Started
**Goal:** Replace ad-hoc tack-driving code with a structured catalog framework that lives in `crates/oriterm_test_support::tack_framework`. Each scenario is described once via `ScenarioSpec` (semantic ID, menu navigation steps, readiness anchor, per-scenario parser). `TackNavigator` walks the navigation steps through `PtySession`. `ScenarioRunner` ties spawn → navigate → capture → parse together. Sections 05-08 add scenarios to the catalog without re-implementing the navigation loop. The framework is finished and proven by one end-to-end scenario at the end of this section.

**Why `oriterm_test_support` and not `oriterm_core/tests/tack/framework/`:** Section 07 (GPU goldens) lives in `oriterm/src/gpu/visual_regression/tack/`. Integration test targets are isolated — `oriterm/src/` cannot import from `oriterm_core/tests/`. If the framework lived inside `oriterm_core`'s test target, Section 07 would have to either (a) lift it later or (b) duplicate the framework. We avoid both by placing it in the workspace-internal `oriterm_test_support` crate from the start. Both `oriterm_core/tests/tack/` (text scenarios) and `oriterm/src/gpu/visual_regression/tack/` (GPU goldens) can `use oriterm_test_support::tack_framework::*` directly.

**Success Criteria:**

- [ ] `crates/oriterm_test_support/src/session/mod.rs` has been split (04.0.a) into `session/mod.rs` (types + constructors + accessors + Drop) + `session/sync/mod.rs` (wait_for / wait_for_with_context / wait_for_any / wait / drain polling helpers + private `poll_until` skeleton) + `session/teardown/mod.rs` (wait_for_child_exit / quit_tack + test-only `force_close_rx_for_test`). Sibling `tests.rs` files live next to each submodule per test-organization.md. All existing tests continue to pass unchanged
- [ ] No file under `crates/oriterm_test_support/src/session/` exceeds 500 lines (hard limit from code-hygiene.md) after the split lands AND after 04.0.b's new primitives lands on top of it
- [ ] `crates/oriterm_test_support/src/tack_framework/mod.rs` exists and re-exports the framework types
- [ ] `PtySession::wait_for_with_context(needle, timeout_ms, ctx)` exists and is the shared wait-for loop body. The plain `wait_for` delegates to it. `TackNavigator` and `ScenarioRunner` consume `wait_for_with_context` for rich panic messages — no parallel loop bodies anywhere
- [ ] `PtySession::wait_for_any(anchors, timeout_ms) -> Option<usize>` exists as the non-panicking multi-anchor primitive. Returns `Some(idx)` on match, `None` on timeout — `TackNavigator` uses this to honor `MenuStep::or_wait_for` instead of wrapping `wait_for_with_context` in `catch_unwind` (panic-as-control-flow is banned)
- [ ] `PtySession::send_raw(bytes)` exists as the no-quiesce write primitive. `quit_tack` uses it so each iteration doesn't burn 300ms in `send()`'s internal drain
- [ ] A private `poll_until<P: FnMut(&str) -> bool>` (or equivalent shape) helper in `session/sync/mod.rs` is the SINGLE canonical home for the deadline-loop + drain_blocking + bounded-poll-sleep skeleton shared by `wait_for_with_context`, `wait_for_any`, and `wait_for_child_exit_inner`. Three call sites with the same control-flow skeleton were `LEAK:algorithmic-duplication`; Section 04 fixes it at the canonical site before adding more duplication
- [ ] `PtySession::quit_tack(max_iterations) -> ExitStatus` exists. Sends one `q\n` per iteration via `send_raw`, observes `try_wait()` after each send, returns the exit status as soon as the child terminates, panics on max-iteration overflow. Each iteration respects the same bounded-poll discipline as `wait_for_child_exit_inner` (no hot-spin)
- [ ] `ScenarioSpec { id: &'static str, menu_path: &'static [MenuStep], ready_anchor: &'static str, quit_path: Option<fn(&mut PtySession) -> ExitStatus>, parser: ScreenParserFn }` is defined and constructible at module scope as `const`
- [ ] `MenuStep { send: &'static [u8], wait_for: &'static str, or_wait_for: &'static [&'static str] }` describes a single navigation step (one keystroke + the primary anchor + optional alternate anchors for pagers/etc.)
- [ ] `TackNavigator::navigate(&mut PtySession, &[MenuStep])` walks the steps with `wait_for_with_context` between every send. BEFORE every send the navigator snapshots the pre-send grid and panics with a clear message if `step.wait_for` (or any `or_wait_for` entry) is ALREADY present (the C1 pre-existing-anchor guard)
- [ ] `tack_framework::parser::tokens` module exposes `grid_has_token(grid, token)`, `grid_line_starts_with(grid, prefix)`, `grid_find_field(grid, label)` — whitespace-bounded helpers that replace blind `grid.contains(short_label)` checks
- [ ] `ScenarioRunner::run(&ScenarioSpec) -> ScenarioOutcome` is the public entry point
- [ ] `ScenarioRunner::run_at(spec, cols, rows)` asserts `exit.success()` on the captured `ExitStatus` and panics with both the exit status AND the captured grid on failure (C3 — exit status is never thrown away)
- [ ] `ScenarioOutcome { scenario_id, screen_id, cols, rows, grid_text, parsed }` carries size-aware identity. `scenario_id` is the test name, `screen_id` is the dedupable screen identity. `snapshot_name()` and `golden_name()` return `"<screen_id>_<cols>x<rows>"`
- [ ] `LiveSession::finish(self) -> ExitStatus` calls the SAME `quit_tack` helper as `run_at` and asserts exit success (M5 — Section 07 GPU goldens consume this contract)
- [ ] `crates/oriterm_test_support/src/tack_framework/scenarios/mod.rs` exists and `pub mod scenarios;` is wired through `tack_framework/mod.rs`. Section 04 owns the scenarios module contract; Sections 05-08 add submodules under it (M2)
- [x] One end-to-end scenario `tack_modes_am` passes: navigates `n -> x -> n` (begin testing -> modes -> run standard tests), uses sub-menu-specific anchors (`tack/test [n] >`, `tack/test/mode [n] >`, `Done`), snapshots, asserts the parser found a tack-tagged cap label via `grid_has_paren_token` (the canonical tokenized helper for tack's `(cap_name)` parenthesized output format — Section 04 introduced both `grid_has_token` for whitespace-bounded matches AND `grid_has_paren_token` for tack's parenthesized form so per-scenario parsers always have a tokenized choice and never need raw `grid.contains`). Tack's modes test produces output for many caps (`am`, `bce`, `bw`, ...) sequentially, with earlier caps scrolling off the visible 24-row viewport before the test reports `Done`; the always-visible terminator is `(os)` (over-strike), so the wrapper test asserts on `os` rather than `am`. Section 05 of the tack-conformance plan adds per-cap scenarios that capture the right viewport for each.
- [ ] `timeout 150 cargo test -p oriterm_core --test tack -- tack_modes_am` passes deterministically (10 consecutive runs) — under BOTH debug and `--release`
- [ ] Bounded-poll invariant pinned per-consumer: `pty_session_wait_for_with_context_bounded_poll_invariant`, `pty_session_wait_for_any_bounded_poll_invariant`, and `pty_session_wait_for_child_exit_bounded_poll_invariant` all pass (three call sites, three pins)
- [ ] `LiveSession::finish` has two dedicated unit tests in `runner/tests.rs`: `live_session_finish_asserts_clean_exit_via_quit_tack` and `live_session_finish_panics_on_non_success_exit`
- [ ] Failing-test-first discipline honored: every 04.0.b + 04.3 finish + 04.4 test item was written BEFORE its implementation body
- [ ] Satisfies mission criteria: scenario framework foundation; one end-to-end scenario proves the framework

**Context:** Without a structured framework, every tack scenario test would re-implement the same loop: spawn tack, send `n` for "begin testing", wait, send a letter for the sub-menu, wait, capture, assert. Across 20+ scenarios (Sections 05-06) that's 20+ copies of fragile navigation code. The fragility shows up two ways:
1. **Fixed sleeps**: developers add `thread::sleep(100)` between sends to "let tack settle". This races, especially in CI, and produces flaky tests.
2. **Regex over whole grid**: assertions like `assert!(grid.matches("modes").count() > 5)` are brittle and don't actually verify what they claim. A semantic assertion ("the parser found the `am` capability box") is testable in a way regex over text is not.

The framework solves both: `MenuStep::wait_for` is the deterministic synchronization primitive (replaces sleeps), and `ScreenParser` extracts structured facts from the grid (replaces regex). Section 04 builds the framework end-to-end and validates it with ONE scenario; Sections 05-08 add the rest of the catalog.

**Reference implementations:**
- **ori_term teseq** `plans/completed/teseq-conformance/section-01-infrastructure.md:95-156`: `ScenarioSpec`/`TerminalConfig`/`ExpectConfig` pattern. We adapt the structure for tack's navigation-driven model (different from teseq's byte-feeding model, but the spec-as-data approach is the same).
- **ori_term vttest** `oriterm_core/tests/vttest/menu6.rs::walk_menu6_subscreens(s, label, tag)`: existing example of `wait_for` + send-keystroke + drain pattern. The framework formalizes the same idea with `MenuStep` data instead of imperative `walk_*` functions.
- **Alacritty** `alacritty_terminal/tests/ref.rs`: scenario-directory pattern (each scenario is a directory with input + golden grid). We adopt scenario-AS-data (a `&'static [ScenarioSpec]` array) instead of scenario-AS-directory because tack scenarios share so much structure (same spawn, same end-of-menu-path navigation) that directories are noise.

**Pre-implementation review findings (must-fix at design time):**

The following issues were surfaced by a Codex blind-spot review (original 11 findings) and a follow-up Codex midpoint review + codebase audit (second wave: MID-C1 BLOAT, MID-M1 catch_unwind, MID-M2 split 04.1, MID-M3 section-07 depends_on, MID-Mod1 SSOT golden_name, MID-Mod2 index.md drift, LEAK:algorithmic-duplication, and the Mi1 lever materialisation). Every finding is incorporated into the framework design BEFORE implementation and referenced by tag throughout the subsections.

**Original Codex blind-spot review findings (C1-C3, M1-M6, Mi1-Mi2):**

- **C1 — Pre-existing-anchor race.** A naive `wait_for("begin testing")` after sending `n` returns immediately because the literal `begin testing` is on the main menu (the `n) begin testing` line). The next keystroke goes to the wrong state. **Fix:** `TackNavigator` snapshots the pre-send grid and panics if the step's `wait_for` (or any `or_wait_for` alternate) is already present. Anchors MUST be SUBMENU-specific (sub-menu prompt or screen-unique heading). See 04.2.
- **C2 — State-blind 3×q teardown.** Hardcoded `send(b"q\n") × 3` is a guess about nesting depth: it over-sends after tack exits on the first `q` (writing to a closed PTY) or under-sends and leaves tack alive. **Fix:** `PtySession::quit_tack(max)` (04.0.b.4) sends one `q\n` via `send_raw` (no 300ms quiesce per iteration), observes `try_wait()`, stops the moment the child exits, panics on overflow with the current grid. Per-scenario quit overrides via `ScenarioSpec::quit_path`.
- **C3 — Exit status thrown away.** `let _exit = session.wait_for_child_exit(2_000)` silently passes when tack aborts with an error code. **Fix:** `ScenarioRunner::run_at` and `LiveSession::finish` BOTH `assert!(exit.success(), ...)` with the exit status AND the captured grid in the panic message.
- **M1 — Size-aware identity.** `ScenarioOutcome` had no `cols`/`rows` fields, so size-matrix runs (Sections 05/07) couldn't dedupe snapshots. **Fix:** `ScenarioOutcome { scenario_id, screen_id, cols, rows, ... }` plus `snapshot_name()`/`golden_name()` methods.
- **M2 — `scenarios` module ownership.** The original draft created `scenarios/modes.rs` in 04.4 without ever wiring `pub mod scenarios;` into `tack_framework/mod.rs`. **Fix:** 04.1.b owns the `scenarios/mod.rs` skeleton and the `pub mod scenarios;` declaration (`pub mod modes;` is then added by 04.4 together with `scenarios/modes.rs`). Sections 05-08 only add submodules.
- **M3 — Substring parsing too weak.** `grid.contains("am")` false-positives on `name`, `xenl` on `xenlabel`, etc. **Fix:** `tack_framework::parser::tokens` exposes `grid_has_token` (whitespace-bounded), `grid_line_starts_with`, `grid_find_field`. Per-scenario parsers MUST use these instead of blind `str::contains` for short labels and control-response markers.
- **M4 — `TackNavigator` duplicated `PtySession::wait_for`.** The original draft re-implemented `wait_for`'s loop body inside the navigator just to get a richer panic message. **Fix:** 04.0.b extends `PtySession::wait_for` with a `wait_for_with_context` variant that takes a panic-message builder closure. The plain `wait_for` delegates. `TackNavigator` and `ScenarioRunner` consume `wait_for_with_context` directly — no parallel loop bodies. Additionally, the private `poll_until` helper in `session/sync/mod.rs` (introduced alongside the split) is the single canonical home for the bounded-poll + drain + deadline skeleton shared by `wait_for_with_context`, `wait_for_any`, and `wait_for_child_exit_inner` — three sites with identical control-flow skeleton = `LEAK:algorithmic-duplication`, fixed at the source.
- **M4b — `catch_unwind`-based alternate-anchor handling.** An earlier draft of 04.2 wrapped `wait_for_with_context` in `std::panic::catch_unwind` to fall through `MenuStep::or_wait_for` alternates. Panic-as-control-flow is a workaround antipattern banned by impl-hygiene.md's "no hacks, no workarounds" rule and is explicitly called out in the broken-window policy. **Fix:** 04.0.b adds `PtySession::wait_for_any(anchors, timeout_ms) -> Option<usize>` as the non-panicking multi-anchor primitive. `TackNavigator` consumes it directly: one call returns `Some(idx)` for any match (primary or alternate) or `None` for timeout, and the navigator panics on `None` with the full context. No `catch_unwind`, no unwind-safety gymnastics, no lost backtraces.
- **M5 — Section 04/07 cleanup ownership disagreement.** `run_with_session_at` originally said the caller quits, but Section 07 just dropped `LiveSession`, losing the exit-status assertion. **Fix:** `LiveSession::finish(self) -> ExitStatus` calls the SAME `quit_tack` helper as `run_at` and asserts exit success. Section 07 callers MUST call `live.finish()` after rendering.
- **M6 — `MenuStep` too narrow.** Real tack flows hit pagers, "press any key" prompts, and alternate sub-menu wording. **Fix:** `MenuStep::or_wait_for: &'static [&'static str]` lists alternate anchors; `TackNavigator::wait_for_step` builds a combined `[primary, ...alternates]` anchor slice and makes ONE `PtySession::wait_for_any` call — no sequential per-anchor budgets, no `catch_unwind` fall-through, no deadline slicing. All alternates race against the same `STEP_TIMEOUT_MS` budget and the lowest-index match wins on a tie.
- **Mi1 — `send` quiesce dependency.** `PtySession::send` calls `wait(300)` internally, so the framework's "no fixed sleeps" claim refers to the navigator's POLL loop, not the post-write quiesce inside `send`. Documented at the top of `tack_framework/mod.rs` and in `runner/mod.rs`. The lever to bypass it — `PtySession::send_raw` — is no longer hypothetical: 04.0.b.3 adds it as a first-class primitive alongside `send`, and `quit_tack` consumes it so the q-loop doesn't burn 300ms per iteration. Navigation code continues to use `send()` for its quiesce; only the teardown path uses `send_raw`.
- **Mi2 — Per-scenario `tic` cost.** Each `run_at` invokes `TerminfoEnv::compile()` which shells out to `tic`. With ~30 scenarios × 3 sizes that's ~90 `tic` invocations per test run. Section 04 keeps per-scenario compile and flags the `OnceLock` cache lever at the top of `runner/mod.rs`. Section 09 has a checklist item to MEASURE the wall-clock regression after Sections 05/06/07 land and PULL the lever (add the cache) if the regression exceeds 10s — the Mi2 fix is NOT deferred to a follow-up plan; it lives inside the tack-conformance scope as a Section 09 conditional action.

**Codex midpoint-review findings (second wave: BLOAT, Mod, cross-section, MID-* tags):**

- **MID-C1 — BLOAT at `session/mod.rs`.** The existing `crates/oriterm_test_support/src/session/mod.rs` is 468 lines (verified 2026-04-07). Adding `wait_for_with_context` + `wait_for_any` + `quit_tack` + `send_raw` + `poll_until` on top would push it well past the 500-line hard limit from `.claude/rules/code-hygiene.md`. "Touching a file over 500 lines without splitting = finding." **Fix:** 04.0.a splits `session/mod.rs` into `session/mod.rs` (dispatch hub) + `session/sync/mod.rs` (polling primitives + `poll_until`) + `session/teardown/mod.rs` (child-exit + quit primitives), each as a directory module with a sibling `tests.rs`. The split happens BEFORE 04.0.b's new primitives land.
- **MID-M1 — `catch_unwind` antipattern in TackNavigator.** Earlier draft wrapped `wait_for_with_context` in `std::panic::catch_unwind` to fall through `or_wait_for` alternates. Panic-as-control-flow is banned by the broken-window policy. **Fix:** 04.0.b.2 adds `PtySession::wait_for_any(anchors, timeout_ms) -> Option<usize>` as the non-panicking multi-anchor primitive; `TackNavigator::wait_for_step` calls it directly. No `catch_unwind` anywhere in the navigator (enforced by a grep-based 04.N checklist item).
- **MID-M2 — 04.1 too broad.** The original 04.1 combined spec types + parser types + tokenized helpers + scenarios skeleton + wiring across six files in one subsection. **Fix:** 04.1 is split into 04.1.a (spec.rs + parser stub + lib wiring, with a build-clippy-test checkpoint gate) and 04.1.b (tokens.rs + scenarios skeleton + parser tests, with its own checkpoint gate). Each half is a self-contained bite that builds and tests green before the next bite starts.
- **MID-M3 — Section 07 `depends_on` missing Section 06.** Section 07 consumes `tack_framework::scenarios::character_sets::*` which Section 06 owns (the character_sets scenarios live under `t) tools`, not under `n) begin testing`). The original `depends_on: ["01", "02", "04", "05"]` missed this. **Fix:** Section 07's `depends_on` extended to `["01", "02", "04", "05", "06"]`. The 04.N checklist pins this as a cross-section sync action.
- **MID-Mod1 — `golden_name()` SSOT violation.** Section 07's original `run_tack_scenario_golden` rebuilt `format!("{}_{}x{}", live.screen_id, cols, rows)` at the call site, duplicating the naming convention defined in `ScenarioOutcome::golden_name()`. `LEAK:scattered-knowledge`. **Fix:** `LiveSession` gains `snapshot_name()` and `golden_name()` methods that delegate to the SAME format literal as `ScenarioOutcome`. Section 07 calls `live.golden_name()` — never rebuilds the string.
- **MID-Mod2 — `index.md` text drift around Section 03 handoff.** The Section 03 cluster in `index.md` mentioned `wait_for_child_exit` as the Section 04 hard-handoff primitive; after 04.0.b.4 introduces `quit_tack(5)` as the strict superset, this text is stale. **Fix:** 04.N updates the `index.md` Section 03 cluster keyword to `"Section 04 hard handoff: ScenarioRunner::run_at must call quit_tack(5) — strict superset of wait_for_child_exit(2_000)"` so a future reader of `index.md` doesn't conclude the Section 03 contract was bypassed.

**Codebase audit findings (Agent 3 review pass):**

- **LEAK:algorithmic-duplication at `session/mod.rs:254-269` vs `session/mod.rs:308-338`.** `wait_for` and `wait_for_child_exit_inner` both implement the same bounded-poll skeleton (deadline + drain_blocking + idle-sleep + deadline check). Two sites already = extract. Adding `wait_for_with_context` as a third site crosses the 3-instance always-extract threshold. **Fix:** 04.0.a introduces a private `poll_until<T, P>(session, timeout_ms, check) -> Option<T>` helper in `session/sync/mod.rs` that captures the bounded-poll skeleton once. All three callers (`wait_for_with_context`, `wait_for_any`, `wait_for_child_exit_inner`) delegate to it. Bounded-poll discipline is now pinned by three semantic-pin tests (one per caller), so a regression in any single caller's deadline/drain/sleep behavior fires its own test.
- **Workaround: `Mi1 hypothetical future lever` is now real.** The original Mi1 note said `send_raw` was "a hypothetical future lever if observed flakes require it." But `quit_tack`'s q-loop NEEDS a no-quiesce send to avoid burning 300ms per iteration. Deferring `send_raw` would either force `quit_tack` to re-implement `send_raw` locally (hack) or live with the 300ms penalty (workaround). **Fix:** 04.0.b.3 adds `send_raw` as a first-class primitive in `session/mod.rs` alongside `send`, with its own `send`-cross-reference doc comment and `pty_session_send_raw_writes_without_quiesce` semantic-pin test that proves the two primitives are distinct (wall-clock <100ms assertion).

**Depends on:** Section 03 (smoke test proves the spawn_tack pipeline works).

**Section 03 handoff reconciliation.** Section 03's "Section 04 handoff contract" item 3 declares `wait_for_child_exit(2_000)` to be the canonical clean-quit primitive and explicitly forbids `send(b"q\n") × 3 + wait(500)` as a regression. Section 04's `quit_tack(5)` (introduced in 04.0.b.4) is a strict SUPERSET of `wait_for_child_exit(2_000)`: it sends one `q\n` via `send_raw`, then calls the same `try_wait()` Section 03 mandated, returning the moment the child exits. The crucial property the handoff contract demands — *the runner asserts the child actually exited* — is preserved end-to-end (`quit_tack` panics with the grid on max-iteration overflow, and `ScenarioRunner::run_at` then asserts `exit.success()` on the returned `ExitStatus`). The 04.N completion checklist makes the reconciliation explicit so a future reviewer reading Section 03 in isolation doesn't conclude the contract was bypassed. **No `send(b"q\n") × 3 + wait(500)` antipattern exists in Section 04 — neither in `run_at` nor in `LiveSession::finish`.**

**Cross-section consumer status (BLOCKER for Sections 05/06/07).** Sections 05, 06, and 07 currently reference an OLDER API shape that predates Agent 1's expansion of this section (no `screen_id`, no `quit_path`, no `MenuStep::or_wait_for`, no `MenuStep::new`, no `outcome.snapshot_name()`/`golden_name()`, no `LiveSession::finish`, parsers and consts defined inline in the test target instead of in `tack_framework::scenarios::*`, blind `grid.contains` instead of `grid_has_token`). Their `reviewed: false` flag is the enforcement gate: each must be re-reviewed against THIS section before implementation begins. The 04.N completion checklist pins this with explicit cross-section sync items so finishing Section 04 forces the consumer-update conversation. The actual rewrites belong to the consumer sections, not Section 04 — Section 04's job is to be the contract-of-record, not to silently bend the contract to fit obsolete consumer drafts.

---

## 04.0.a Split `session/mod.rs` into submodules ahead of adding new primitives (BLOAT prevention)

**File(s):** `crates/oriterm_test_support/src/session/mod.rs` (refactor), `crates/oriterm_test_support/src/session/sync/mod.rs` (NEW), `crates/oriterm_test_support/src/session/sync/tests.rs` (NEW), `crates/oriterm_test_support/src/session/teardown/mod.rs` (NEW), `crates/oriterm_test_support/src/session/teardown/tests.rs` (NEW), `crates/oriterm_test_support/src/session/tests.rs` (trim to dispatch-hub tests only)

**Why this is the very first thing in Section 04.** `session/mod.rs` is currently 468 lines (verified 2026-04-07). The 500-line hard limit from `.claude/rules/code-hygiene.md` leaves 32 lines of headroom — and 04.0.b is about to add `wait_for_with_context` (~30 lines), `wait_for_any` (~20 lines), `quit_tack` (~25 lines), `send_raw` (~10 lines), and a private `poll_until` helper (~20 lines), plus doc comments. That is ~110 lines of new code in a file with 32 lines of headroom — a guaranteed BLOAT violation. The impl-hygiene rule is explicit: "Touching a file over 500 lines without splitting = finding." We split proactively, BEFORE the new code, so every new primitive is born in its canonical home. "Proactive split at ~450 lines" is the rule and we are past that threshold already.

**Broken Window Policy application.** While splitting, fix every nearby hygiene issue encountered — no "pre-existing, out of scope" deferral. Audit findings folded into this subsection:

- **LEAK:algorithmic-duplication** (current `session/mod.rs:254-269` vs `session/mod.rs:308-338`): `wait_for` and `wait_for_child_exit_inner` both implement the same bounded-poll skeleton — `deadline = Instant::now() + timeout; loop { check_predicate; drain_blocking; if nothing drained sleep; assert deadline not passed }`. Two sites = extract threshold (per impl-hygiene.md's "2 instances, >5 lines of shared skeleton → extract immediately"). Adding `wait_for_with_context` (a third site) crosses the 3-instance "always extract" threshold, so the extraction happens HERE in 04.0.a, not "later as a refactor."
- **Tests sibling trim.** `session/tests.rs` (189 lines) currently owns test scaffolding for the entire session module. When we split, the `pty_session_wait_for_child_exit_bounded_poll_invariant` test and `force_close_rx_for_test` helper move to `session/teardown/tests.rs` alongside their target. The `pty_session_drains_simple_output` and `tool_available_*` tests stay in `session/tests.rs` with the dispatch hub. This keeps per-file tests localized so future readers can find the relevant tests next to the code.

**Split plan (enforced by impl-hygiene.md's "Module Roles" — `mod.rs` dispatches, leaf files implement).**

- [x] Create `crates/oriterm_test_support/src/session/sync/mod.rs` as a leaf module owning all polling/synchronization primitives. Move the following methods out of `mod.rs` into this file as `impl PtySession` blocks:
  - `pub fn drain(&mut self) -> usize` (currently `mod.rs:205-211`)
  - `pub fn drain_blocking(&mut self, timeout_ms: u64) -> usize` (currently `mod.rs:215-221`)
  - `fn feed_and_flush(&mut self, data: &[u8]) -> usize` (currently `mod.rs:226-235`)
  - `pub fn wait(&mut self, quiet_ms: u64)` (currently `mod.rs:242-248`)
  - `pub fn wait_for(&mut self, needle: &str, timeout_ms: u64)` (currently `mod.rs:254-269`) — **rewritten** to delegate to `poll_until` (see below). The further refactor into a `wait_for_with_context` delegation lands in 04.0.b
  - The new `wait_for_with_context`, `wait_for_any`, and private `poll_until` (introduced in 04.0.b on TOP of the split — `poll_until` itself lands in 04.0.a)
  Include the module doc comment (`//! Bounded-poll synchronization primitives for PtySession. See [`poll_until`].`).

- [x] Create `crates/oriterm_test_support/src/session/teardown/mod.rs` as a leaf module owning child-exit and quit primitives. Move the following methods out of `mod.rs` into this file as `impl PtySession` blocks:
  - `pub fn wait_for_child_exit(&mut self, timeout_ms: u64) -> ExitStatus` (currently `mod.rs:296-298`)
  - `fn wait_for_child_exit_inner<F: FnMut()>(...) -> ExitStatus` (currently `mod.rs:308-338`) — **rewritten** to delegate to the private `poll_until` helper in `session/sync/mod.rs`. Visibility: `poll_until` is `pub(super)` in `session/sync/mod.rs`, making it visible throughout the `session` module tree; `teardown/mod.rs` reaches it as `super::sync::poll_until` (both `sync` and `teardown` are children of `session`, and `pub(super)` exposes to the parent `session`, which covers sibling modules). `poll_until` is NOT exported beyond `session` — it stays an implementation detail of the bounded-poll skeleton
  - The new `quit_tack` (introduced in 04.0.b on TOP of the split)
  - The `force_close_rx_for_test` test-only helper currently in `session/tests.rs:26-34` — this becomes a `#[cfg(test)] impl PtySession` block alongside `wait_for_child_exit_inner` so test scaffolding lives next to the code it is scaffolding.

- [x] Retain `crates/oriterm_test_support/src/session/mod.rs` as the dispatch hub holding:
  - Module docs
  - `mod sync;` and `mod teardown;` declarations
  - Imports shared by the type definition
  - `pub struct PtyResponder` + its `impl` + `impl EventListener for PtyResponder`
  - `pub struct PtySession` definition
  - `impl PtySession` constructor block: `spawn`, `spawn_vttest`, `spawn_tack`
  - `impl PtySession` accessor block: `term`, `cols`, `rows`, `grid_text`, `grid_chars`, `size_label`, `send` (kept here as the canonical send path — `send_raw` is added in 04.0.b next to it)
  - `impl Drop for PtySession`
  - Free functions: `tool_available`, `vttest_available`, `tic_available`, `tack_available`, `infocmp_available`
  - `#[cfg(test)] mod tests;` at the bottom
  **Rationale for what stays in `mod.rs`:** `Drop` owns the `child` field and must live with the type definition; constructors own the full PTY spawn flow; accessors and `send` round out "how you use a PtySession." Polling helpers and teardown helpers operate on `&mut self` and are independently understandable — they are the right candidates for leaf modules per "One primary type per module file" and "submodule extraction for logical groups exceeding ~200 lines."

- [x] Create `crates/oriterm_test_support/src/session/sync/tests.rs` as the sibling tests for the `sync` submodule (which is itself a directory module `session/sync/mod.rs`). Move the `pty_session_drains_simple_output` test there (it exercises `wait_for` and `drain_blocking` — both sync primitives). NEW tests from 04.0.b (see below) also land here.

- [x] Create `crates/oriterm_test_support/src/session/teardown/tests.rs` as the sibling tests for the `teardown` submodule (directory module `session/teardown/mod.rs`). Move the `pty_session_wait_for_child_exit_bounded_poll_invariant` test and the `pty_session_wait_for_child_exit_returns_on_clean_exit` test there. Move the `force_close_rx_for_test` helper's `impl PtySession` block there (it's only consumed by the bounded-poll invariant test, which also lives in this file after the move). NEW tests from 04.0.b (`quit_tack` tests) also land here.

- [x] Retain `crates/oriterm_test_support/src/session/tests.rs` as the sibling tests for the dispatch hub, trimmed to only what's tested directly by `mod.rs`:
  - `tool_available_returns_false_for_nonexistent_binary`
  - `vttest_available_matches_tool_available`
  - `tic_available_matches_tool_available`
  - `infocmp_available_matches_tool_available`
  - `tack_available_matches_tool_available`
  Everything else moves to `sync/tests.rs` or `teardown/tests.rs`.

- [x] `sync` and `teardown` are directory modules from the start (`session/sync/mod.rs` + `session/sync/tests.rs`; `session/teardown/mod.rs` + `session/teardown/tests.rs`) to satisfy the test-organization.md rule "When a module has tests, it **must** be a directory module (`foo/mod.rs`), not a file module (`foo.rs`)." Never have `sync.rs` alongside `sync/`.

- [x] Extract the bounded-poll skeleton into a private `poll_until<P>` helper in `session/sync/mod.rs`. Sketch:
  ```rust
  use std::time::{Duration, Instant};

  /// Outcome of a single `poll_until` pass: keep polling, or surface
  /// the predicate's successful value.
  pub(super) enum PollStep<T> {
      /// Predicate not satisfied yet; keep polling.
      NotYet,
      /// Predicate satisfied with this payload — return it to caller.
      Done(T),
  }

  /// Bounded-poll skeleton: drains PTY output, calls `check`, sleeps
  /// briefly when nothing was drained, and honors a hard deadline.
  ///
  /// Returns `Some(payload)` when `check` emits `PollStep::Done(_)`
  /// before the deadline. Returns `None` when the deadline passes.
  ///
  /// This is the SINGLE canonical home for the bounded-poll pattern
  /// shared by `wait_for_with_context`, `wait_for_any`, and
  /// `wait_for_child_exit_inner`. Earlier drafts duplicated the
  /// skeleton across three methods — see the LEAK:algorithmic-
  /// duplication finding in the 04.0.a design notes.
  pub(super) fn poll_until<T, P>(
      session: &mut super::PtySession,
      timeout_ms: u64,
      mut check: P,
  ) -> Option<T>
  where
      P: FnMut(&mut super::PtySession) -> PollStep<T>,
  {
      const IDLE_SLEEP: Duration = Duration::from_millis(10);
      let deadline = Instant::now() + Duration::from_millis(timeout_ms);
      loop {
          if let PollStep::Done(payload) = check(session) {
              return Some(payload);
          }
          if Instant::now() >= deadline {
              return None;
          }
          if session.drain_blocking(50) == 0 {
              std::thread::sleep(IDLE_SLEEP);
          }
      }
  }
  ```
  `wait_for_with_context`, `wait_for_any`, and `wait_for_child_exit_inner` all consume `poll_until` with their specific predicates. The deadline-sleep-drain skeleton lives in exactly one place.

- [x] Re-run every existing `pty_session_*` test after the split; all must pass unchanged. The split is a pure refactor — behavior is preserved, only file layout changes.

- [x] Cross-compile check: `cargo build --target x86_64-pc-windows-gnu -p oriterm_test_support` — both leaf modules must compile for Windows (nothing in the split is Unix-specific; `force_close_rx_for_test` is `#[cfg(unix)]` only because its consumer test is `#[cfg(unix)]`, not because the helper itself requires Unix).

- [x] `timeout 150 cargo test -p oriterm_test_support` — green after the split, BEFORE any 04.0.b code lands.

- [x] `cargo clippy --all-targets -p oriterm_test_support` — green after the split.

- [x] Verify every file created/modified in 04.0.a is under 500 lines:
  - `session/mod.rs` should shrink to roughly 250-300 lines after the extraction (the dispatch hub).
  - `session/sync/mod.rs` should be roughly 150-200 lines (polling helpers + `poll_until`).
  - `session/teardown/mod.rs` should be roughly 120-180 lines (wait_for_child_exit + quit_tack after 04.0.b).
  - Sibling `tests.rs` files are exempt from the 500-line limit but keep them readable — split further if any individual tests file passes 500 lines.

- [x] **Codebase audit fix along the way** (from the Agent 3 review pass):
  - **Doc comment on `send_raw` (added in 04.0.b) and `send`** — update `send`'s doc comment to cross-reference `send_raw` and explain when to choose which (send: "wait for the screen to settle after a keystroke — the default for interactive navigation"; send_raw: "write and flush only, skipping the 300ms quiesce — for teardown loops where `try_wait()` polling replaces the settle check"). [Deferred to 04.0.b — both items land together when `send_raw` itself is added.]
  - **Banner removal sweep** — touching `mod.rs` and creating new submodules is a good time to scan for decorative banners (`// ===`, `// ---`). None currently exist in `session/mod.rs` (verified 2026-04-07), and the new `sync/mod.rs` + `teardown/mod.rs` files are clean of banners as well — the sweep is a checklist item so future touches inherit the rule.

---

## 04.0.b PtySession primitives: wait_for_with_context + wait_for_any + quit_tack + send_raw

**File(s):** `crates/oriterm_test_support/src/session/sync/mod.rs` (extend — the split from 04.0.a has already moved the polling helpers here), `crates/oriterm_test_support/src/session/sync/tests.rs` (new unit tests), `crates/oriterm_test_support/src/session/teardown/mod.rs` (extend), `crates/oriterm_test_support/src/session/teardown/tests.rs` (new unit tests), `crates/oriterm_test_support/src/session/mod.rs` (add `send_raw` alongside `send`)

Four `PtySession` primitives MUST land before any framework code references them. They are the joints the framework hangs off, so they live in Section 04 as the second thing built (after 04.0.a's split). No work in 04.1+ may proceed before 04.0.b is green.

**Why four primitives and not two:** the original draft of this section had `wait_for_with_context` + `quit_tack`, but Agent 3's review surfaced two additional needs: (a) `wait_for_any` is required to let `TackNavigator` honor `MenuStep::or_wait_for` without wrapping `wait_for_with_context` in `std::panic::catch_unwind` — panic-as-control-flow is a workaround antipattern banned by the broken-window policy; (b) `send_raw` is required so `quit_tack`'s q-loop iterations don't burn 300ms apiece in the quiesce that `send()` does internally, and the Mi1 "hypothetical future lever" becomes an actual lever consumed in Section 04 itself.

**Why 04.0 instead of "Section 03 backfill":** Section 03 finalized its primitives and committed them. Adding new primitives to `PtySession` is a normal forward-extension pattern (Section 03's own 03.1 added `wait_for_child_exit` after Section 01 finalized `PtySession`). Each section owns the primitives it introduces — Section 04 owns these four.

### 04.0.b.1 Extend `wait_for` with a context closure (M4)

The Codex review surfaced that `TackNavigator::navigate_step` re-implements `PtySession::wait_for`'s loop body just to produce a richer panic message that includes the menu_path step index. Two parallel loop bodies = LEAK:algorithmic-duplication. Fix at the canonical site via the `poll_until` helper introduced in 04.0.a, not at the consumer.

- [x] In `crates/oriterm_test_support/src/session/sync/mod.rs` (the file the split placed it in), add `wait_for_with_context` as a thin wrapper over the `poll_until` helper from 04.0.a. Consuming `poll_until` means this method is ~15 lines of predicate-building, not a re-implemented deadline loop:
  ```rust
  /// Wait until `needle` appears anywhere in `grid_text()`, with a
  /// hard timeout. On timeout, panics with the message returned by
  /// `ctx(grid)` — the closure receives the captured grid so callers
  /// can build messages that mention navigation step index, sub-menu
  /// state, or any other context the bare `wait_for` doesn't know
  /// about.
  ///
  /// `wait_for` (the existing public method) delegates to this helper
  /// with a default closure that produces the historical panic format.
  /// All other consumers (`TackNavigator`, `ScenarioRunner`) call
  /// `wait_for_with_context` directly.
  ///
  /// Internally builds a `poll_until` predicate that emits
  /// `PollStep::Done(())` when `grid_text().contains(needle)`; all
  /// deadline/sleep/drain bookkeeping lives in `poll_until` so this
  /// method cannot drift from `wait_for_any` or
  /// `wait_for_child_exit_inner`.
  pub fn wait_for_with_context<F>(
      &mut self,
      needle: &str,
      timeout_ms: u64,
      ctx: F,
  )
  where
      F: Fn(&str) -> String,
  {
      let found = poll_until::<(), _>(self, timeout_ms, |session| {
          if session.grid_text().contains(needle) {
              PollStep::Done(())
          } else {
              PollStep::NotYet
          }
      });
      if found.is_some() {
          self.wait(200);
          return;
      }
      panic!("{}", ctx(&self.grid_text()));
  }

  /// Wait until `needle` appears anywhere in `grid_text()`, with a
  /// hard timeout. Panics with the current grid on timeout — kept as
  /// the default ergonomic for code that doesn't need a richer
  /// message. Delegates to `wait_for_with_context`.
  pub fn wait_for(&mut self, needle: &str, timeout_ms: u64) {
      self.wait_for_with_context(needle, timeout_ms, |grid| {
          format!(
              "timed out waiting for {needle:?} after {timeout_ms}ms.\nGrid:\n{grid}",
          )
      });
  }
  ```

- [x] Verify the existing `pty_session_drains_simple_output` test still passes (it calls `session.wait_for("hello", 5_000)` which now goes through the delegation — the default closure must produce a panic message functionally equivalent to the historical one).

- [x] Verify the existing `pty_session_wait_for_child_exit_bounded_poll_invariant` test still passes — after 04.0.a rewrote `wait_for_child_exit_inner` to consume `poll_until`, the iteration-count bound still holds because `poll_until` preserves the 10ms sleep-on-empty-drain discipline. The test acts as a semantic pin for the bounded-poll behavior across all three call sites.

- [x] **Failing-test-first sequencing.** Write `pty_session_wait_for_with_context_uses_custom_message` BEFORE implementing `wait_for_with_context`. The test must compile (the method signature referenced by the test must exist as a `todo!()` stub) but fail at runtime. Only after watching it fail does the body land. Same ordering rule applies to every test item in 04.0.b — write the failing test, watch it fail, then implement.

- [x] Add a unit test `pty_session_wait_for_with_context_uses_custom_message` in `crates/oriterm_test_support/src/session/sync/tests.rs`. Spawn a `cat`-equivalent (Unix `/bin/cat`, Windows `findstr.exe /N x` — pick something that produces no output and stays alive), then call `wait_for_with_context("never_printed", 100, |g| format!("CUSTOM_TAG: {g}"))` inside `std::panic::catch_unwind` and assert the panic payload contains `CUSTOM_TAG`. Two-arm `#[cfg(unix)] / #[cfg(windows)]` pattern matching the existing `pty_session_drains_simple_output` shape.

- [x] Add a bounded-poll pin test `pty_session_wait_for_with_context_bounded_poll_invariant` in `crates/oriterm_test_support/src/session/sync/tests.rs`. This is the SEMANTIC PIN that `wait_for_with_context` does NOT hot-spin on the `Ok(None)` branch — the existing `pty_session_wait_for_child_exit_bounded_poll_invariant` test already pins this for `wait_for_child_exit_inner`, but `wait_for_with_context` is a second poll_until consumer and deserves its own pin. Spawn a silent long-lived child and call `wait_for_with_context("never", 500, |_| "timeout".into())` inside `std::panic::catch_unwind`. Wall-clock MUST be between 500ms and 700ms (deadline honored without early return and without hot-spinning past the 200ms grace window). Two-arm cross-platform. A regression that removes the 10ms idle sleep from `poll_until` would burn CPU for 500ms straight — not observable via wall-clock, but observable via `/proc/self/stat` utime delta >100ms (Unix) or `GetThreadTimes` CPU delta (Windows). The simpler invariant is wall-clock bounded on the upper end AND a deadline-bounded loop-iteration assertion (expose `poll_until` iteration count via a `#[cfg(test)]` counter closure, mirroring the `force_close_rx_for_test` pattern) — pick whichever is easier to implement portably, but the bounded-poll property MUST be pinned for this consumer, not just for `wait_for_child_exit_inner`.

### 04.0.b.2 Add `PtySession::wait_for_any(anchors, timeout_ms) -> Option<usize>` (M4b fix — kills the `catch_unwind` antipattern)

`TackNavigator` (04.2) needs to match the primary anchor OR any `MenuStep::or_wait_for` alternate in a single bounded-poll pass. The naive approach — wrap `wait_for_with_context` in `std::panic::catch_unwind` and try each anchor sequentially — uses panic-as-control-flow, which is a workaround antipattern. The broken-window policy is explicit: "If a fix feels hacky, it IS hacky."

The right solution is to give `PtySession` a non-panicking multi-anchor primitive. It reuses `poll_until` identically to `wait_for_with_context`, with a predicate that scans all anchors in one pass. Per-iteration cost is O(n_anchors × grid_len) which is negligible (n_anchors ≤ 4 in practice, grid_len ≈ 2000).

- [x] In `crates/oriterm_test_support/src/session/sync/mod.rs`, add:
  ```rust
  /// Wait until ANY anchor in `anchors` appears in `grid_text()`,
  /// with a hard timeout.
  ///
  /// Returns `Some(idx)` — the index into `anchors` of the first
  /// anchor that matched — on success. Returns `None` on timeout.
  /// Does NOT panic on timeout: the caller decides how to surface
  /// the failure (the navigator builds a panic message that lists
  /// every anchor it tried; lower-level consumers can log and
  /// continue).
  ///
  /// Semantic contract: anchor-to-index ordering is preserved, so
  /// `MenuStep::or_wait_for`'s slice index is meaningful to the
  /// navigator. If two anchors match simultaneously on the same
  /// poll iteration, the LOWER index wins (primary anchor preferred
  /// over alternates). Empty `anchors` slice is treated as a
  /// malformed call and returns `None` immediately.
  ///
  /// Internally consumes the same `poll_until` helper as
  /// `wait_for_with_context` — no parallel deadline loop, no
  /// `catch_unwind`, no unwind-safety gymnastics.
  pub fn wait_for_any(
      &mut self,
      anchors: &[&str],
      timeout_ms: u64,
  ) -> Option<usize> {
      if anchors.is_empty() {
          return None;
      }
      let matched = poll_until::<usize, _>(self, timeout_ms, |session| {
          let text = session.grid_text();
          for (idx, anchor) in anchors.iter().enumerate() {
              if text.contains(anchor) {
                  return PollStep::Done(idx);
              }
          }
          PollStep::NotYet
      });
      if matched.is_some() {
          self.wait(200);
      }
      matched
  }
  ```

- [x] Add unit tests in `crates/oriterm_test_support/src/session/sync/tests.rs`:
  - `pty_session_wait_for_any_returns_some_zero_when_primary_matches`: spawn a child that prints `marker_primary`, call `wait_for_any(&["marker_primary", "marker_alt"], 3_000)`, assert the return is `Some(0)`. Two-arm cross-platform pattern.
  - `pty_session_wait_for_any_returns_some_alt_when_alternate_matches`: spawn a child that prints `marker_alt`, call `wait_for_any(&["marker_primary", "marker_alt"], 3_000)`, assert the return is `Some(1)`. Two-arm cross-platform pattern.
  - `pty_session_wait_for_any_returns_none_on_timeout`: spawn a silent long-lived child, call `wait_for_any(&["never"], 100)`, assert the return is `None`. Two-arm cross-platform pattern. This is the SEMANTIC PIN that proves `wait_for_any` is non-panicking — any future refactor that replaces the body with `wait_for_with_context` + `catch_unwind` would trivially pass a timeout test, but this test specifically asserts `Option::None` was returned (the catch_unwind version would panic, so `assert!(result.is_none())` would never run because the test would panic inside the call).
  - `pty_session_wait_for_any_prefers_primary_over_alternates_on_tie`: spawn a child that prints both markers in the same drain (or, more reliably, prints a line containing both anchors as substrings), call `wait_for_any(&["marker_primary", "marker_alt"], 3_000)`, assert the return is `Some(0)` (primary index). Two-arm cross-platform pattern.
  - `pty_session_wait_for_any_empty_slice_returns_none`: pure-unit test with no child needed — just construct a silent session, call `wait_for_any(&[], 100)`, assert `None` and assert the call returns under 50ms (the empty-slice fast path doesn't enter the poll loop).
  - `pty_session_wait_for_any_bounded_poll_invariant`: bounded-poll SEMANTIC PIN for the third `poll_until` consumer. Spawn a silent long-lived child, call `wait_for_any(&["never"], 500)`, assert the return is `None`, and assert wall-clock is between 500ms and 700ms. Mirror of the `wait_for_with_context` bounded-poll test — pins that the 10ms idle-sleep discipline is preserved when `poll_until` is invoked via the `wait_for_any` predicate shape (which has a different loop body than `wait_for_with_context`). Two-arm cross-platform. Together with `pty_session_wait_for_with_context_bounded_poll_invariant` and `pty_session_wait_for_child_exit_bounded_poll_invariant`, this completes the three-call-site bounded-poll pin that proves `poll_until` preserves its discipline across every consumer.

### 04.0.b.3 Add `PtySession::send_raw(bytes)` (Mi1 lever consumed now, not deferred)

`quit_tack`'s q-loop needs to send `q\n`, observe `try_wait()`, and repeat — fast. Reusing the existing `send(&[u8])` method would make each iteration pay the 300ms quiesce from `send()`'s internal `wait(300)`, and that quiesce is precisely what the state-aware quit loop is trying to AVOID (we want to see the child exit as soon as it happens, not 300ms later). The Mi1 note at the top of this section used to call `send_raw` a "hypothetical future lever if observed flakes require it" — we need it NOW for a non-hack `quit_tack`, so we add it in 04.0.b alongside `quit_tack` itself.

- [x] In `crates/oriterm_test_support/src/session/mod.rs` (the dispatch hub — `send` lives there in the accessor block and `send_raw` lives next to it so the two primitives are discoverable together), add:
  ```rust
  /// Write bytes to the child's PTY and flush, WITHOUT the 300ms
  /// quiesce that [`Self::send`] does internally.
  ///
  /// Use this when the caller has its own synchronization strategy
  /// that makes the quiesce unnecessary or actively harmful — e.g.
  /// `quit_tack` polls `try_wait()` between sends and wants to
  /// observe child exit as soon as possible, not 300ms later.
  ///
  /// Error swallow policy: writer errors are silently dropped (same
  /// as `quit_tack`'s teardown context, where a `q\n` after tack has
  /// already exited produces EPIPE/ERROR_BROKEN_PIPE that we do
  /// NOT want to crash on). Callers that need error propagation
  /// should use the canonical [`Self::send`] and tolerate the
  /// quiesce.
  pub fn send_raw(&mut self, key: &[u8]) {
      let _ = self.writer.write_all(key);
      let _ = self.writer.flush();
  }
  ```
- [x] Update `send()`'s doc comment (currently `mod.rs:340-341`) to cross-reference `send_raw` so future readers see both primitives together:
  ```rust
  /// Send bytes to the child via the PTY writer, then wait for the
  /// screen to settle (300ms quiet period).
  ///
  /// This is the default send primitive for interactive navigation
  /// tests where the caller expects the terminal to repaint before
  /// the next assertion. For teardown loops or rapid-fire sends
  /// where `try_wait()` polling replaces the settle check, use
  /// [`Self::send_raw`] instead.
  pub fn send(&mut self, key: &[u8]) { ... }
  ```
- [x] Add a unit test `pty_session_send_raw_writes_without_quiesce` in `crates/oriterm_test_support/src/session/tests.rs` (the dispatch-hub tests file). Measure wall-clock: `Instant::now(); session.send_raw(b"x\n"); assert!(elapsed < 100ms)` — proves there's no 300ms wait inside. Spawn `cat` (Unix) / `findstr.exe /N x` (Windows) two-arm, which echoes the byte back so the writer succeeds. The assertion is about timing, not observability: if `send_raw` is accidentally rewritten to delegate to `send`, the elapsed time jumps to ~300ms and the test fires. This is the SEMANTIC PIN that `send_raw` is distinct from `send`.

### 04.0.b.4 Add `PtySession::quit_tack(max_iterations)` (C2 fix)

The naive `send(b"q\n") × 3` teardown is state-blind: it guesses at nesting depth, may write to a closed PTY after tack exits on the first `q`, and is invisible to the test author when it goes wrong. Replace with a state-aware loop.

- [x] In `crates/oriterm_test_support/src/session/teardown/mod.rs`, add `quit_tack` — it lives next to `wait_for_child_exit` because both are the child-teardown family:
  ```rust
  use portable_pty::ExitStatus;
  use std::thread;
  use std::time::Duration;

  /// Bounded idle sleep for the q-loop. Same 10ms discipline as the
  /// `poll_until` helper — each iteration waits at most 10ms for an
  /// observable child exit before sending the next q\n, so the
  /// maximum wall-clock cost of a runaway `quit_tack(5)` call is
  /// ~1050ms (5 iterations × ~200ms drain + 5 × 10ms idle +
  /// try_wait overhead). No hot-spin.
  const QUIT_IDLE_SLEEP: Duration = Duration::from_millis(10);

  /// State-aware quit loop for `tack`-style submenu nesting.
  ///
  /// Each iteration:
  ///   1. `send_raw(b"q\n")` — write+flush WITHOUT the 300ms quiesce.
  ///      Errors are swallowed because the child may have already
  ///      exited on the previous q and the PTY writer will return
  ///      EPIPE / ERROR_BROKEN_PIPE.
  ///   2. Drain PTY output for 200ms so tack's q-acknowledgement
  ///      repaint lands in our grid (this is the 'quiesce' the
  ///      canonical `send()` does at 300ms — we use 200ms here
  ///      because the teardown path is less sensitive to full
  ///      screen repaints than the navigation path).
  ///   3. Short bounded idle (`QUIT_IDLE_SLEEP = 10ms`) if nothing
  ///      was drained, matching `poll_until`'s discipline.
  ///   4. Call `try_wait()`. If the child has exited, return the
  ///      `ExitStatus` immediately.
  /// After `max_iterations` iterations with no observed exit,
  /// panics with the current grid.
  ///
  /// Why a loop instead of a fixed `q\n × N`: `tack` accepts variable
  /// numbers of `q`s depending on which sub-menu the test left it in.
  /// A fixed count either over-sends (writing to a closed PTY after
  /// the child has exited — UB on some platforms) or under-sends
  /// (leaves tack alive and the test panics in `wait_for_child_exit`
  /// 2 s later with no diagnostic about WHY tack didn't quit). The
  /// loop terminates the moment the child actually exits.
  ///
  /// Why `send_raw` and not `send`: the canonical `send()` runs
  /// `wait(300)` internally, which defeats the point of a state-
  /// aware loop — we don't want to burn 300ms per iteration waiting
  /// for a repaint when we could be polling `try_wait()`. See
  /// 04.0.b.3 for the `send_raw` lever.
  ///
  /// The default `max_iterations` for tack is 5 — enough for
  /// main-menu → submenu → sub-submenu nesting plus one extra for
  /// safety.
  pub fn quit_tack(&mut self, max_iterations: u32) -> ExitStatus {
      for _ in 0..max_iterations {
          self.send_raw(b"q\n");
          // Drain any output produced by the q\n acknowledgement,
          // bounded so we don't block indefinitely if tack produces
          // no repaint between q's.
          let drained = self.drain_blocking(200);
          if drained == 0 {
              thread::sleep(QUIT_IDLE_SLEEP);
          }
          if let Ok(Some(status)) = self.child.try_wait() {
              return status;
          }
      }
      panic!(
          "PtySession::quit_tack: child did not exit after {max_iterations} q\\n iterations.\nGrid:\n{}",
          self.grid_text()
      );
  }
  ```

- [x] Add a unit test `pty_session_quit_tack_returns_status_when_child_exits` in `crates/oriterm_test_support/src/session/teardown/tests.rs`. Spawn a small shell loop that exits after reading any `q` line (Unix: `/bin/sh -c 'while IFS= read -r line; do case "$line" in q) exit 0;; esac; done'`). Windows arm: spawn `cmd.exe /C "pause > NUL"` — `pause` waits for any keystroke, so `send_raw(b"q\n")` causes it to read `q` and exit cleanly. (This exercises the ConPTY q-loop path more faithfully than `cmd.exe /C exit 0`, which exits before any q is sent.) Two-arm `#[cfg(unix)] / #[cfg(windows)]` pattern matching the existing `pty_session_wait_for_child_exit_returns_on_clean_exit` shape.

- [x] Add a `pty_session_quit_tack_exits_early_when_child_dies_after_first_q` test (Unix + Windows). SEMANTIC PIN: this proves the q-loop returns the moment `try_wait()` observes exit, not after exhausting `max_iterations`. Spawn a child that exits after reading any keystroke — the same shell loop as above but with `max_iterations=5`. Measure iteration count via a test-only instrumented helper (or measure wall-clock: the test must return in < 500ms with `max_iterations=5`, proving we did NOT loop 5 times at 200ms + 10ms each = 1050ms). A regression that removes the `try_wait` early-exit and always loops `max_iterations` times would cause wall-clock to regress past 1000ms and the assertion to fire. Two-arm cross-platform pattern.

- [x] Add a `pty_session_quit_tack_panics_on_max_iterations` unit test (Unix-only). Spawn `/bin/sh -c 'while true; do sleep 0.1; done'` (a child that NEVER exits no matter how many `q`s it receives). Wrap `quit_tack(2)` in `std::panic::catch_unwind` and assert the panic payload contains `"did not exit after 2 q\\n iterations"`. After the catch, the `Drop` impl on `PtySession` reaps the runaway child via `kill` — verify no zombie remains by inspecting `session.child.try_wait()` returns `Ok(Some(_))` on subsequent poll (or simply let `Drop` run and trust the existing reap path).
  - **Windows coverage gap acknowledgment**: the corresponding Windows scenario — a child that truly ignores q\n forever — is hard to construct portably in ConPTY because `ping -n 6 127.0.0.1 > NUL` technically quits on STDIN close which we don't trigger here. The Unix-only panic test is sufficient coverage because the panic body is platform-agnostic (`assert!` + `format!` + `grid_text()`), and all three happy-path primitives (`quit_tack` early-exit, `wait_for_child_exit`, `wait_for_any` timeout) are tested on both platforms. The Windows ConPTY q-loop is exercised by `pty_session_quit_tack_returns_status_when_child_exits`'s Windows arm, which confirms `send_raw`+`try_wait` on ConPTY works end-to-end.

### 04.0.b.5 Verify the new primitives compile and clippy clean

- [x] Run `cargo build -p oriterm_test_support` and `cargo clippy --all-targets -p oriterm_test_support` — both green.
- [x] Run `timeout 150 cargo test -p oriterm_test_support` — all existing tests pass plus the new ones introduced in 04.0.b.1-4 (wait_for_with_context custom message; wait_for_any primary/alternate/timeout/tie/empty; send_raw no-quiesce timing; quit_tack clean-exit cross-platform, early-exit cross-platform, max-iterations panic Unix-only).
- [x] Cross-compile for Windows: `cargo build --target x86_64-pc-windows-gnu -p oriterm_test_support`.
- [x] Verify no file created or modified in 04.0.a + 04.0.b exceeds 500 lines (production code) — `wc -l crates/oriterm_test_support/src/session/*.rs crates/oriterm_test_support/src/session/*/*.rs` should show every non-`tests.rs` file under the limit.

---

## 04.1.a ScenarioSpec + MenuStep types (spec.rs) and lib.rs/tack_framework wiring

**File(s):** `crates/oriterm_test_support/src/tack_framework/mod.rs` (NEW — scaffolding only), `crates/oriterm_test_support/src/tack_framework/spec.rs` (NEW), `crates/oriterm_test_support/src/lib.rs` (extend)

**Why 04.1 is split into .a and .b:** Agent 3's review flagged the original 04.1 as too broad for one session — it combined crate wiring, spec types, parser types, tokenized helpers, scenarios skeleton, and re-exports across six files. Splitting into a "types + wiring" checkpoint (04.1.a) and a "parser + helpers + scenarios skeleton" checkpoint (04.1.b) creates a natural `cargo build` + `cargo test -p oriterm_test_support` gate between them so a single failing build never buries three unrelated edits. The split keeps Section 04 whole but turns it into reviewable bites.

**Checkpoint gate at the end of 04.1.a**: `cargo build -p oriterm_test_support && cargo clippy --all-targets -p oriterm_test_support && timeout 150 cargo test -p oriterm_test_support` must all be green before any work proceeds on 04.1.b. If the checkpoint fails, investigate and fix inside 04.1.a — do not paper over with more code in 04.1.b.

The types here are pure data — no I/O, no `PtySession`. They describe scenarios, not run them. `ScenarioSpec` can't be constructed yet because it references a `ScreenParserFn` type that lives in `parser/mod.rs` (04.1.b) — during 04.1.a we add the `parser` field as `fn(&str) -> ScreenFacts` with a forward-declared type alias or a stub module. The pragmatic approach: land `parser/mod.rs` with an empty `ScreenFacts` + `ScreenParserFn` + `default_parser` in 04.1.a so `spec.rs` can reference them, and defer `tokens.rs` + parser tests + scenarios skeleton to 04.1.b. This makes 04.1.a a self-contained "types compile and are constructible" checkpoint without opening a split across spec and parser.

- [x] Add `pub mod tack_framework;` to `crates/oriterm_test_support/src/lib.rs` next to `pub mod terminfo;` and `pub mod session;`. Add re-exports for the framework types so callers can `use oriterm_test_support::tack_framework::{...}` (preferred for explicit module path) or `use oriterm_test_support::{ScenarioSpec, ScenarioRunner, ...}` (re-exported at crate root for convenience):
  ```rust
  pub mod session;
  pub mod tack_framework;
  pub mod terminfo;

  pub use session::{PtyResponder, PtySession, /* ... */};
  pub use tack_framework::{
      LiveSession, MenuStep, ScenarioOutcome, ScenarioRunner, ScenarioSpec,
      ScreenFacts, ScreenParserFn, TackNavigator,
  };
  // `decode_terminfo_string` and `infocmp_query` are added by Section 08
  // (keyboard/function key tests) — NOT here. Section 04 only needs
  // `TerminfoEnv` from the terminfo module.
  pub use terminfo::TerminfoEnv;
  ```


- [x] Create `crates/oriterm_test_support/src/tack_framework/mod.rs` — the dispatch hub. In 04.1.a this declares `mod parser; mod spec;` only. The `mod navigator;`, `mod runner;`, `mod scenarios;` lines are added incrementally in 04.1.b + 04.2 + 04.3 so every intermediate state is `cargo build`-green:
  ```rust
  //! Scenario catalog framework for tack-driven conformance tests.
  //!
  //! See plans/tack-conformance/section-04-scenario-framework.md for the
  //! design rationale (semantic IDs, menu navigation as data, deterministic
  //! wait_for synchronization, per-scenario parsers, tokenized grid checks,
  //! pre-existing-anchor guard).
  //!
  //! # Scenarios module
  //!
  //! `pub mod scenarios;` is the catalog of pub const ScenarioSpec values
  //! and per-scenario parsers consumed by both text tests
  //! (oriterm_core/tests/tack/) and GPU goldens
  //! (oriterm/src/gpu/visual_regression/tack/). Section 04 owns the module
  //! skeleton; Sections 05-08 add submodules under it.
  //!
  //! # Internal dependencies
  //!
  //! `send()` (and therefore `TackNavigator::navigate`) calls
  //! `wait(300)` internally to drain output before returning. The
  //! framework as a whole is "no fixed sleeps in the navigator loop";
  //! the 300 ms drain inside `send` is the canonical post-write quiesce
  //! and is documented here so future readers don't double-add it.
  //! `TackNavigator` uses `PtySession::wait_for_any` (04.0.b.2) for the
  //! primary+alternate anchor matching, NOT `catch_unwind` on
  //! `wait_for_with_context` — panic-as-control-flow is banned.

  // 04.1.a adds these two:
  pub mod parser;
  pub mod spec;

  pub use parser::{ScreenFacts, ScreenParserFn, default_parser};
  pub use spec::{MenuStep, ScenarioSpec};

  // 04.1.b adds tokens re-exports:
  //   pub use parser::tokens::{grid_find_field, grid_has_token, grid_line_starts_with};
  // 04.1.b adds the scenarios module skeleton:
  //   pub mod scenarios;
  // 04.2 adds:
  //   pub mod navigator;
  //   pub use navigator::TackNavigator;
  // 04.3 adds:
  //   pub mod runner;
  //   pub use runner::{LiveSession, ScenarioOutcome, ScenarioRunner};
  ```

- [x] Create `crates/oriterm_test_support/src/tack_framework/parser/mod.rs` FIRST (before `spec.rs`) because `spec.rs` references `super::parser::ScreenParserFn`. In 04.1.a this is the minimal stub: `ScreenFacts` type, `ScreenParserFn` alias, `default_parser` fn. The `#[cfg(test)] mod tests;` declaration is deferred to 04.1.b when the tests file lands alongside `tokens.rs` (declaring it now would break the build because the file doesn't exist yet). NO `pub mod tokens;` line yet — that's also added in 04.1.b:
  ```rust
  //! Per-scenario screen parsers and shared tokenized grid helpers.
  //!
  //! `ScreenFacts` is the typed-extraction container; per-scenario
  //! parsers fill the fields they care about. The default parser only
  //! extracts the screen header. The `tokens` submodule (added in
  //! 04.1.b) contains whitespace-bounded match helpers
  //! (`grid_has_token`, etc.) that every per-scenario parser MUST use
  //! instead of blind `grid.contains(short_label)` checks — see the M3
  //! fix at the top of section-04 for why short-substring contains is
  //! unsafe.

  // 04.1.b adds: pub mod tokens;

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

  #[cfg(test)]
  mod tests;
  ```

- [x] Create `crates/oriterm_test_support/src/tack_framework/spec.rs`:
  ```rust
  use crate::session::PtySession;
  use portable_pty::ExitStatus;

  use super::parser::ScreenParserFn;

  /// A single navigation step: send these bytes, then wait until the
  /// PTY grid contains the primary anchor string (or one of the
  /// alternates).
  ///
  /// `wait_for` is the deterministic synchronization primitive — it
  /// replaces fixed sleeps that race in CI. The anchor is a literal
  /// substring expected in the grid AFTER tack processes `send`.
  ///
  /// **Pre-existing-anchor rule.** The anchor MUST NOT already be
  /// present in the grid BEFORE the send. The navigator checks this
  /// (see `TackNavigator::navigate` in 04.2) and panics if it is.
  /// Picking an anchor that's already on the prior screen makes
  /// `wait_for` return immediately and the next keystroke goes to the
  /// wrong state — pick a SUBMENU-specific string (a sub-menu header,
  /// a key prompt unique to the destination screen) instead.
  ///
  /// `or_wait_for` is the alternate-anchor extension point: real tack
  /// flows hit pagers ("press any key"), `--more--` prompts, and
  /// alternate sub-menu wording across distros. Listing alternates
  /// here lets one `MenuStep` handle either case without branching
  /// in the navigator.
  ///
  /// Example:
  ///   MenuStep {
  ///     send: b"m",
  ///     wait_for: "tack [m] >",
  ///     or_wait_for: &[],
  ///   }
  /// — sends 'm' (the change-modes choice) and waits until the
  /// modes-submenu prompt `tack [m] >` appears. NOTE: do NOT use
  /// `"modes"` as the anchor here — the word "modes" appears on the
  /// main menu's `m) change modes` line and the pre-existing-anchor
  /// guard will reject it. Use the sub-menu PROMPT, not a word that
  /// is already on the main menu.
  #[derive(Copy, Clone, Debug)]
  pub struct MenuStep {
      pub send: &'static [u8],
      pub wait_for: &'static str,
      pub or_wait_for: &'static [&'static str],
  }

  impl MenuStep {
      /// Convenience constructor with no alternate anchors.
      #[must_use]
      pub const fn new(send: &'static [u8], wait_for: &'static str) -> Self {
          Self { send, wait_for, or_wait_for: &[] }
      }
  }

  /// Static description of a single tack scenario.
  ///
  /// Constructible as `const` so test catalogs can list scenarios in
  /// arrays. The whole spec is data — no closures, no I/O — until the
  /// `parser` and (optional) `quit_path` function pointers are invoked
  /// by `ScenarioRunner`.
  #[derive(Copy, Clone, Debug)]
  pub struct ScenarioSpec {
      /// Semantic ID, e.g. `"tack_modes_am"`. Used as the
      /// `scenario_id` field of `ScenarioOutcome` (NOT directly as the
      /// snapshot name — the snapshot name is built from
      /// `screen_id`+`cols`x`rows` so size-matrix runs share goldens
      /// when navigation produces the same screen).
      ///
      /// Convention: `tack_<menu>_<screen>_<assertion>` lowercase
      /// snake_case.
      pub id: &'static str,

      /// Screen identity for snapshot/golden deduplication. Multiple
      /// scenarios that visit the SAME tack screen share the same
      /// `screen_id` so they snapshot once. Convention:
      /// `tack_<menu>_<screen>` (e.g., `"tack_modes"` for every modes
      /// scenario regardless of which cap it asserts).
      pub screen_id: &'static str,

      /// Sequence of navigation steps from tack's main menu to the
      /// target screen. Each step sends one or more bytes and waits
      /// for an anchor string to appear in the grid.
      ///
      /// Example for the modes screen (n -> m). Note both anchors
      /// are SUB-menu prompts unique to their destination — neither
      /// is a substring of the prior screen.
      ///   &[
      ///     MenuStep::new(b"n", "tack [n] >"),
      ///     MenuStep::new(b"m", "tack [m] >"),
      ///   ]
      pub menu_path: &'static [MenuStep],

      /// Final readiness anchor. After the last `MenuStep` lands, the
      /// runner calls `session.wait_for(ready_anchor, ...)` once more
      /// to make sure the screen has fully painted before grid_text
      /// is captured.
      ///
      /// Same pre-existing-anchor rule as `MenuStep::wait_for`: the
      /// anchor must be SCREEN-specific, not a word that's already on
      /// the prior menu.
      pub ready_anchor: &'static str,

      /// Per-scenario quit override. `None` means use the canonical
      /// `PtySession::quit_tack(5)` introduced in 04.0.b.4. A scenario
      /// that needs a different escape path (e.g., a sub-menu that
      /// only exits on `\x1b`, or a screen that needs a single 'q'
      /// without nesting) provides a custom function pointer.
      pub quit_path: Option<fn(&mut PtySession) -> ExitStatus>,

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
          screen_id: &'static str,
          menu_path: &'static [MenuStep],
          ready_anchor: &'static str,
      ) -> Self {
          Self {
              id,
              screen_id,
              menu_path,
              ready_anchor,
              quit_path: None,
              parser: super::parser::default_parser,
          }
      }
  }
  ```

  **Why function pointers, not closures:** `ScenarioSpec` must be `const`-constructible at module scope so a `const SCENARIOS: &[&ScenarioSpec] = &[...]` array works. Closures capture state and aren't `const`. Function pointers are. The trade-off: per-scenario parsers can't close over local config — they have to be plain `fn(&str) -> ScreenFacts`. Sections 05-06 will define one named parser fn per scenario family (e.g., `parse_modes_screen`, `parse_color_screen`).

- [x] **04.1.a checkpoint** — Run `cargo build -p oriterm_test_support && cargo clippy --all-targets -p oriterm_test_support && timeout 150 cargo test -p oriterm_test_support`. All three must be green before proceeding to 04.1.b. The only new public surface at this checkpoint is `tack_framework::{ScenarioSpec, MenuStep, ScreenFacts, ScreenParserFn, default_parser}` — nothing in 04.1.a exercises the new types, so the tests that pass are the existing session/terminfo tests plus 04.0.a/b's new tests. This is the "nothing is dead code; nothing is broken; the types compile and are constructible as `const`" gate.

---

## 04.1.b Parser tokenized helpers + scenarios module skeleton + parser tests

**File(s):** `crates/oriterm_test_support/src/tack_framework/parser/mod.rs` (extend — add `pub mod tokens;`), `crates/oriterm_test_support/src/tack_framework/parser/tokens.rs` (NEW), `crates/oriterm_test_support/src/tack_framework/parser/tests.rs` (NEW), `crates/oriterm_test_support/src/tack_framework/scenarios/mod.rs` (NEW), `crates/oriterm_test_support/src/tack_framework/mod.rs` (extend — add `pub mod scenarios;` and `pub use parser::tokens::*` re-exports), `crates/oriterm_test_support/src/lib.rs` (extend re-exports)

This subsection owns the tokenized parser helpers (M3) and the `scenarios/` module skeleton (M2). `tokens.rs` is the canonical home for whitespace-bounded grid matching; `scenarios/mod.rs` is the dispatch hub for const `ScenarioSpec` catalogs that Sections 05-08 will add submodules under. No code in the test target references any of this yet — Section 04's first end-to-end scenario (`tack_modes_am`) lands in 04.4, which populates `scenarios/modes.rs` under the skeleton created here.

- [x] Update `crates/oriterm_test_support/src/tack_framework/parser/mod.rs` to add `pub mod tokens;` below the existing stub. This is a one-line edit that turns the parser module into a parent with a submodule.

- [x] Update `crates/oriterm_test_support/src/tack_framework/mod.rs` to add the tokens re-exports:
  ```rust
  pub use parser::tokens::{grid_find_field, grid_has_token, grid_line_starts_with};
  ```

- [x] Create `crates/oriterm_test_support/src/tack_framework/scenarios/mod.rs` (the skeleton — Sections 05-08 add submodules under it):
  ```rust
  //! Const ScenarioSpec catalog consumed by both text tests
  //! (oriterm_core/tests/tack/) and GPU goldens
  //! (oriterm/src/gpu/visual_regression/tack/).
  //!
  //! Section 04 introduces the first submodule (`modes`) in 04.4
  //! which contains `TACK_MODES_AM` and `parse_modes_screen`.
  //! Sections 05-08 add:
  //!   - 05: acs, graphic_rendition, color, cursor_movement
  //!   - 06: tools_menu submodules
  //!   - 08: keyboard / function key consts
  //!
  //! Each submodule defines `pub const SCENARIO_*: ScenarioSpec`
  //! values and a `pub fn parse_*_screen(grid: &str) -> ScreenFacts`
  //! function pointer. ScenarioSpec is `const`-constructible so the
  //! catalog forms `pub const ALL_*: &[&ScenarioSpec]` arrays for
  //! exhaustiveness tests.

  // Section 04.4 adds: pub mod modes;
  ```
  The `pub mod modes;` line is added in 04.4 together with `scenarios/modes.rs`. In 04.1.b the scenarios directory exists with an empty skeleton mod.rs so 04.4 is a pure additive edit (one-line insert + new file).

- [x] Update `crates/oriterm_test_support/src/tack_framework/mod.rs` to add `pub mod scenarios;` now that the module exists.

- [x] Create `crates/oriterm_test_support/src/tack_framework/parser/tokens.rs` (the M3 tokenized helpers):
  ```rust
  //! Whitespace-bounded grid match helpers.
  //!
  //! Blind `grid.contains("am")` is dangerous: any 2-letter substring
  //! will collide with longer words and column-aligned table cells.
  //! `grid_has_token("am")` matches `am` only when it appears as a
  //! whole word — surrounded by ASCII whitespace, line edges, or
  //! grid edges. `grid_line_starts_with("tack [m]")` is for prompt
  //! markers that always start a line. `grid_find_field("setaf")`
  //! locates a labeled field and returns the trailing value (the
  //! text after the label up to the next whitespace run).
  //!
  //! Every per-scenario parser MUST use these helpers instead of
  //! `str::contains` for capability labels, control-response markers,
  //! or any short literal that could collide with a substring.

  /// True iff `token` appears in `grid` as a whitespace-bounded word.
  ///
  /// "Whitespace-bounded" means: each side of the match is either
  /// ASCII whitespace, the start of `grid`, or the end of `grid`.
  /// Punctuation does NOT count as a boundary — e.g., `grid_has_token`
  /// for `"am"` against `"am,bce"` returns FALSE (the comma is not
  /// whitespace). Tack draws cap names with surrounding spaces, so
  /// the whitespace-only rule is correct for the modes/SGR screens.
  ///
  /// O(grid_len * token_len) worst case, single-pass scan.
  #[must_use]
  pub fn grid_has_token(grid: &str, token: &str) -> bool {
      if token.is_empty() {
          return false;
      }
      let bytes = grid.as_bytes();
      let needle = token.as_bytes();
      let mut i = 0;
      while i + needle.len() <= bytes.len() {
          if &bytes[i..i + needle.len()] == needle {
              let left_ok = i == 0 || bytes[i - 1].is_ascii_whitespace();
              let right_end = i + needle.len();
              let right_ok = right_end == bytes.len()
                  || bytes[right_end].is_ascii_whitespace();
              if left_ok && right_ok {
                  return true;
              }
          }
          i += 1;
      }
      false
  }

  /// True iff any line in `grid` starts with `prefix` (after trimming
  /// leading whitespace from the line). Used for prompt markers like
  /// `tack [m] >` that always begin their line.
  #[must_use]
  pub fn grid_line_starts_with(grid: &str, prefix: &str) -> bool {
      grid.lines().any(|line| line.trim_start().starts_with(prefix))
  }

  /// Locate a labeled field on the grid and return the trailing value
  /// (the text after `label`, trimmed, up to the next whitespace run
  /// or end of line). Returns `None` if the label is not present as
  /// a whitespace-bounded token. Used for cap-value extraction like
  /// `grid_find_field(grid, "setaf")` returning `\\E[3%dm`.
  #[must_use]
  pub fn grid_find_field<'a>(grid: &'a str, label: &str) -> Option<&'a str> {
      for line in grid.lines() {
          if let Some(idx) = line.find(label) {
              // Confirm it's a token, not a substring.
              let left_ok = idx == 0
                  || line.as_bytes()[idx - 1].is_ascii_whitespace();
              let after = idx + label.len();
              if !left_ok {
                  continue;
              }
              let rest = line[after..].trim_start();
              if rest.is_empty() {
                  return None;
              }
              let value = rest
                  .split_whitespace()
                  .next()
                  .unwrap_or("");
              return Some(value);
          }
      }
      None
  }
  ```

- [x] Add unit tests at `crates/oriterm_test_support/src/tack_framework/parser/tests.rs` (sibling tests file). Cover both `default_parser` and the `tokens` helpers including the negative cases that motivate the helpers (collision rejection, punctuation boundary):
  ```rust
  use super::tokens::{grid_find_field, grid_has_token, grid_line_starts_with};
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

  #[test]
  fn grid_has_token_finds_whitespace_bounded_match() {
      assert!(grid_has_token("am bce bw", "am"));
      assert!(grid_has_token("am bce bw", "bce"));
      assert!(grid_has_token("am bce bw", "bw"));
  }

  #[test]
  fn grid_has_token_rejects_substring_collision() {
      // `am` is a substring of `name` — must NOT match.
      assert!(!grid_has_token("name bce bw", "am"));
      // `xenl` is a substring of `xenlabel` — must NOT match.
      assert!(!grid_has_token("xenlabel", "xenl"));
  }

  #[test]
  fn grid_has_token_handles_line_edges_as_boundaries() {
      assert!(grid_has_token("am\nbce", "am"));
      assert!(grid_has_token("am\nbce", "bce"));
  }

  #[test]
  fn grid_has_token_rejects_empty_token() {
      assert!(!grid_has_token("anything", ""));
  }

  #[test]
  fn grid_line_starts_with_finds_prompt_marker() {
      let grid = "header\n  tack [m] > waiting\n";
      assert!(grid_line_starts_with(grid, "tack [m]"));
      assert!(!grid_line_starts_with(grid, "tack [n]"));
  }

  #[test]
  fn grid_find_field_returns_trailing_value() {
      let grid = "header\nsetaf \\E[3%dm\nsetab \\E[4%dm\n";
      assert_eq!(grid_find_field(grid, "setaf"), Some("\\E[3%dm"));
      assert_eq!(grid_find_field(grid, "setab"), Some("\\E[4%dm"));
      assert_eq!(grid_find_field(grid, "missing"), None);
  }
  ```

- [x] **04.1.b checkpoint** — Run `cargo build -p oriterm_test_support && cargo clippy --all-targets -p oriterm_test_support && timeout 150 cargo test -p oriterm_test_support`. All three must be green. The new tests at this checkpoint are the `default_parser` tests + the `grid_has_token`/`grid_line_starts_with`/`grid_find_field` tests. The `scenarios/mod.rs` skeleton has no submodules yet so it compiles as a pure dispatch hub with no consumers. This is the "parser contract is locked; next step is the navigator" gate.

---

## 04.2 TackNavigator: pre-grid guard + walk menu_path with wait_for_any (no catch_unwind)

**File(s):** `crates/oriterm_test_support/src/tack_framework/navigator/mod.rs`, `crates/oriterm_test_support/src/tack_framework/navigator/tests.rs`, `crates/oriterm_test_support/src/tack_framework/mod.rs` (extend — add `pub mod navigator; pub use navigator::TackNavigator;`)

`TackNavigator` is the imperative half of the framework — it takes a `&mut PtySession` and a `&[MenuStep]` and walks them. It does NOT reimplement the wait-for loop body: 04.0.b.1 extended `PtySession::wait_for_with_context` and 04.0.b.2 added `wait_for_any` for exactly these consumers, so the navigator just calls the appropriate primitive with a step-index-aware message builder.

Two non-trivial pieces:
1. **Pre-existing-anchor guard (C1).** Before each `send`, snapshot the grid and panic if `step.wait_for` (or any `or_wait_for` alternate) is ALREADY present. This catches the "I picked an anchor that's on the prior screen too" failure immediately, with a clear message, instead of letting the navigator silently return-then-misroute the next keystroke.
2. **Alternate-anchor matching via `wait_for_any`.** The navigator builds a combined `[primary, ...or_wait_for]` anchor slice and calls `wait_for_any(&anchors, STEP_TIMEOUT_MS)`. On `Some(_)` the step succeeds. On `None` the navigator panics with a full-context message. **No `std::panic::catch_unwind`, no panic-as-control-flow, no unwind-safety gymnastics.** Earlier drafts wrapped `wait_for_with_context` in `catch_unwind` to fall through alternates — Agent 3's review rejected that as a workaround antipattern (the broken-window policy bans hacks). 04.0.b.2 gives us the proper primitive.

- [x] Update `crates/oriterm_test_support/src/tack_framework/mod.rs` to add `pub mod navigator; pub use navigator::TackNavigator;`.

- [x] Create `crates/oriterm_test_support/src/tack_framework/navigator/mod.rs`:
  ```rust
  //! TackNavigator: walks `&[MenuStep]` against a live `PtySession`.
  //!
  //! See plans/tack-conformance/section-04-scenario-framework.md §04.2
  //! for the pre-existing-anchor guard rationale (C1) and the
  //! non-panicking `wait_for_any`-based alternate-anchor matching
  //! (M4b — replaces an earlier draft's `catch_unwind` antipattern).

  use crate::session::PtySession;

  use super::spec::MenuStep;

  /// Walks a `&[MenuStep]` against a live `PtySession` running tack.
  ///
  /// Each step is `pre-existing-guard → send → wait_for_any`, with
  /// no fixed sleeps anywhere and no `catch_unwind`. On wait timeout
  /// the navigator panics with a message that includes the failing
  /// step index, the bytes sent, the primary + alternate anchors,
  /// and the current grid contents.
  ///
  /// Calls `PtySession::wait_for_any` (04.0.b.2) — a non-panicking
  /// multi-anchor primitive that returns `Some(idx)` on match or
  /// `None` on timeout. There is NO parallel wait-for implementation
  /// here, by design: if a future change needs richer step
  /// diagnostics, extend `wait_for_any` once and every consumer
  /// benefits.
  pub struct TackNavigator;

  /// Total CI-safe timeout for one navigation step. Bump only on
  /// observed flakes.
  const STEP_TIMEOUT_MS: u64 = 5_000;

  /// Stack-friendly upper bound for a single `MenuStep`'s combined
  /// `[primary, ...alternates]` anchor slice. Tack menus in practice
  /// never produce more than two or three alternates, so 8 is a
  /// comfortable cap with no heap allocation in the navigator loop.
  const MAX_ANCHORS_PER_STEP: usize = 8;

  impl TackNavigator {
      /// Walk `steps` against `session`. Panics on any wait timeout
      /// or pre-existing-anchor violation.
      pub fn navigate(session: &mut PtySession, steps: &[MenuStep]) {
          for (idx, step) in steps.iter().enumerate() {
              Self::guard_pre_existing_anchor(session, step, idx);
              session.send(step.send);
              Self::wait_for_step(session, step, idx);
          }
      }

      /// Pre-send guard: panics if `step.wait_for` (or any
      /// `or_wait_for` alternate) is already present in the grid
      /// BEFORE we send `step.send`. Picking an anchor that's on the
      /// prior screen makes wait_for return immediately and the next
      /// keystroke goes to the wrong state.
      fn guard_pre_existing_anchor(
          session: &mut PtySession,
          step: &MenuStep,
          idx: usize,
      ) {
          // Drain any pending output so the snapshot is current.
          session.drain();
          let pre_grid = session.grid_text();
          let mut already: Vec<&str> = Vec::new();
          if pre_grid.contains(step.wait_for) {
              already.push(step.wait_for);
          }
          for alt in step.or_wait_for {
              if pre_grid.contains(alt) {
                  already.push(alt);
              }
          }
          assert!(
              already.is_empty(),
              "TackNavigator: step {idx} pre-existing-anchor violation: \
               anchor(s) {already:?} already present in grid before send. \
               Pick a SUBMENU-specific anchor (sub-menu prompt or screen-\
               unique heading), not a word that's already on the prior \
               screen.\nSent: {send_repr:?}\nGrid:\n{pre_grid}",
              send_repr = String::from_utf8_lossy(step.send),
          );
      }

      /// Wait for `step.wait_for` OR any `or_wait_for` alternate to
      /// appear in the grid, via a single `wait_for_any` call.
      ///
      /// Builds a fixed-size stack array of `[primary, ...alternates]`
      /// (capped at `MAX_ANCHORS_PER_STEP`) and passes it to
      /// `PtySession::wait_for_any`. On `Some(_)` the step succeeds.
      /// On `None` the navigator panics with a full-context message
      /// listing every anchor that was tried.
      fn wait_for_step(
          session: &mut PtySession,
          step: &MenuStep,
          idx: usize,
      ) {
          // Overflow guard: MenuStep::or_wait_for is &'static so this
          // is a static assertion — if a scenario ever lists more
          // than 7 alternates it's a design smell and we want a loud
          // failure at navigate-time rather than a silent truncation.
          assert!(
              1 + step.or_wait_for.len() <= MAX_ANCHORS_PER_STEP,
              "TackNavigator: step {idx} has {n} anchors (primary + \
               {alt} alternates) — the cap is {MAX_ANCHORS_PER_STEP}. \
               Split the MenuStep or raise MAX_ANCHORS_PER_STEP.",
              n = 1 + step.or_wait_for.len(),
              alt = step.or_wait_for.len(),
          );

          let mut anchors: [&str; MAX_ANCHORS_PER_STEP] = [""; MAX_ANCHORS_PER_STEP];
          anchors[0] = step.wait_for;
          for (i, alt) in step.or_wait_for.iter().enumerate() {
              anchors[i + 1] = alt;
          }
          let active = 1 + step.or_wait_for.len();

          let matched = session.wait_for_any(&anchors[..active], STEP_TIMEOUT_MS);
          if matched.is_some() {
              return;
          }

          // Timeout — build the full-context panic.
          panic!(
              "TackNavigator: step {idx} failed — none of the anchors \
               appeared within {STEP_TIMEOUT_MS}ms total.\n\
               Sent: {send_repr:?}\n\
               Primary anchor: {primary:?}\n\
               Alternate anchors: {alts:?}\n\
               Grid:\n{grid}",
              send_repr = String::from_utf8_lossy(step.send),
              primary = step.wait_for,
              alts = step.or_wait_for,
              grid = session.grid_text(),
          );
      }
  }

  #[cfg(test)]
  mod tests;
  ```

- [x] Add unit tests at `crates/oriterm_test_support/src/tack_framework/navigator/tests.rs`. These tests need a real `PtySession`, but they don't need tack — `cat` (Unix) / `findstr.exe /N x` (Windows) is enough to give the navigator a live PTY with no spontaneous output. The tests cover the two structurally important paths the end-to-end scenario in 04.4 cannot easily exercise: the pre-existing-anchor guard and the alternate-anchor fallback.

  ```rust
  use portable_pty::CommandBuilder;

  use super::TackNavigator;
  use crate::session::PtySession;
  use crate::session::tool_available;
  use crate::tack_framework::spec::MenuStep;

  // Helper: spawn a long-lived child that produces no output. Used
  // by the panic-path tests.
  fn spawn_silent_child() -> Option<PtySession> {
      #[cfg(unix)]
      {
          if !tool_available("cat", "--help") {
              return None;
          }
          Some(PtySession::spawn(CommandBuilder::new("cat"), 80, 24))
      }
      #[cfg(windows)]
      {
          if !tool_available("findstr.exe", "/?") {
              return None;
          }
          let mut cmd = CommandBuilder::new("findstr.exe");
          cmd.args(["/N", "x"]);
          Some(PtySession::spawn(cmd, 80, 24))
      }
  }

  #[test]
  #[should_panic(expected = "step 0 failed")]
  fn navigator_panics_with_step_index_on_timeout() {
      let Some(mut session) = spawn_silent_child() else { return };
      let steps = &[MenuStep::new(b"", "this_text_will_never_appear")];
      TackNavigator::navigate(&mut session, steps);
  }

  #[test]
  #[should_panic(expected = "pre-existing-anchor violation")]
  fn navigator_panics_when_anchor_already_present_in_pre_grid() {
      // Spawn a child that prints "marker" once, then sleeps.
      #[cfg(unix)]
      let cmd = {
          let mut c = CommandBuilder::new("/bin/sh");
          c.args(["-c", "printf marker; sleep 5"]);
          c.env("TERM", "xterm-256color");
          c
      };
      #[cfg(windows)]
      let cmd = {
          // cmd.exe can't sleep cleanly, so use ping as a delay.
          let mut c = CommandBuilder::new("cmd.exe");
          c.args(["/C", "echo marker && ping -n 6 127.0.0.1 > NUL"]);
          c.env("TERM", "xterm-256color");
          c
      };
      let mut session = PtySession::spawn(cmd, 80, 24);
      // Wait for the marker to land before the navigator runs.
      session.wait_for("marker", 3_000);
      // Now the grid contains "marker". The navigator must reject
      // an anchor of "marker" before sending anything.
      let steps = &[MenuStep::new(b"x", "marker")];
      TackNavigator::navigate(&mut session, steps);
  }

  /// Targeted alternate-anchor test: construct a `MenuStep` whose
  /// primary anchor is impossible and whose single `or_wait_for`
  /// alternate DOES appear in the grid. Asserts that the navigator
  /// successfully matches on the alternate via `wait_for_any`.
  ///
  /// This is the SEMANTIC PIN for the M4b fix: a regression that
  /// reverts to `catch_unwind`-based alternate handling would
  /// either panic inside the navigator (because the primary's
  /// wait_for_with_context is wrapped in a catch that the test
  /// runner sees as a pass) OR take far longer than the
  /// fast-success path this test measures. Wall-clock <2s proves
  /// the navigator is using the parallel `wait_for_any` primitive,
  /// not sequential anchor-at-a-time with a budget split.
  #[test]
  fn navigator_matches_alternate_when_primary_never_appears() {
      #[cfg(unix)]
      let cmd = {
          let mut c = CommandBuilder::new("/bin/sh");
          c.args(["-c", "printf 'alt_anchor\\n'; sleep 5"]);
          c.env("TERM", "xterm-256color");
          c
      };
      #[cfg(windows)]
      let cmd = {
          let mut c = CommandBuilder::new("cmd.exe");
          c.args(["/C", "echo alt_anchor && ping -n 6 127.0.0.1 > NUL"]);
          c.env("TERM", "xterm-256color");
          c
      };
      let mut session = PtySession::spawn(cmd, 80, 24);
      // Wait for alt_anchor to land so the pre-existing guard
      // doesn't trip on its presence — but the guard rejects
      // anchors ALREADY present in the pre-send grid. So we use a
      // two-step: step 0 uses a benign marker we send ourselves,
      // step 1 is the alternate match we actually care about. But
      // the simpler path is to skip the pre-existing guard entirely
      // by having the session start with a pristine grid: we spawn
      // the child but don't drain it before calling navigate. The
      // navigator's `guard_pre_existing_anchor` calls drain() then
      // checks, so it WILL see alt_anchor if it's already arrived.
      // The robust approach: spawn a child that prints alt_anchor
      // only AFTER a short delay, so the pre-send grid is clean and
      // the post-send wait_for_any observes the alternate.
      //
      // Adjust the sh/cmd commands above to: `sleep 0.3 && printf
      // 'alt_anchor\n'; sleep 5` (Unix) and equivalent Windows
      // variant. The 300ms delay gives the navigator time to issue
      // its send before alt_anchor appears.
      let start = std::time::Instant::now();
      let steps = &[MenuStep {
          send: b"",
          wait_for: "primary_never",
          or_wait_for: &["alt_anchor"],
      }];
      TackNavigator::navigate(&mut session, steps);
      let elapsed = start.elapsed();
      assert!(
          elapsed.as_secs() < 2,
          "navigator took {elapsed:?} — expected <2s; a sequential \
           catch_unwind fallback would consume more because each \
           anchor gets its own budget slice"
      );
  }
  ```

  **Test coverage map:** (a) `navigator_panics_with_step_index_on_timeout` covers the timeout panic path; (b) `navigator_panics_when_anchor_already_present_in_pre_grid` covers the C1 pre-existing-anchor guard; (c) `navigator_matches_alternate_when_primary_never_appears` covers the M4b `wait_for_any`-based alternate-anchor fallback (the test that could NOT be written in the pre-M4b design because `catch_unwind` required more fragile trigger sequencing).

---

## 04.3 ScenarioRunner: spawn_tack + navigate + capture + parse + state-aware quit + LiveSession::finish

**File(s):** `crates/oriterm_test_support/src/tack_framework/runner/mod.rs`, `crates/oriterm_test_support/src/tack_framework/runner/tests.rs`

**Module layout note:** `runner` is a directory module, not a file module, because it has sibling tests (see `LiveSession::finish` unit tests below). Per `.claude/rules/test-organization.md`: "When a module has tests, it **must** be a directory module (`foo/mod.rs`), not a file module (`foo.rs`). Never have `foo.rs` alongside a `foo/` directory."

`ScenarioRunner` is the public entry point Sections 05-08 use. Given a `&ScenarioSpec`, it spawns tack, navigates, captures, parses, observes a CLEAN exit via `quit_tack`, and returns a `ScenarioOutcome` with size-aware identity. Tests then run an `assert!(outcome.parsed.capability_labels.contains(&"am".to_string()))` style check, plus `insta::assert_snapshot!(outcome.snapshot_name(), &outcome.grid_text)`.

- [x] Create `crates/oriterm_test_support/src/tack_framework/runner/mod.rs`:
  ```rust
  use portable_pty::ExitStatus;

  use crate::session::PtySession;
  use crate::session::{tack_available, tic_available};
  use crate::terminfo::TerminfoEnv;

  use super::navigator::TackNavigator;
  use super::parser::ScreenFacts;
  use super::spec::ScenarioSpec;

  /// The result of running one scenario: the captured grid text and
  /// the per-scenario parser's typed extraction.
  ///
  /// Carries SIZE-AWARE identity: `scenario_id` is the test name,
  /// `screen_id` is the dedupable screen identity. `snapshot_name()`
  /// and `golden_name()` build the insta/PNG file names from
  /// `screen_id` + `cols` + `rows` so size-matrix runs share goldens
  /// when navigation produces the same screen.
  #[derive(Clone, Debug)]
  pub struct ScenarioOutcome {
      pub scenario_id: &'static str,
      pub screen_id: &'static str,
      pub cols: u16,
      pub rows: u16,
      pub grid_text: String,
      pub parsed: ScreenFacts,
  }

  impl ScenarioOutcome {
      /// Insta snapshot name: `<screen_id>_<cols>x<rows>`. Multiple
      /// scenarios that share `screen_id` AND size will share an
      /// insta `.snap` file (insta dedupes on identical content too,
      /// but using the same name avoids stale-file confusion).
      #[must_use]
      pub fn snapshot_name(&self) -> String {
          format!("{}_{}x{}", self.screen_id, self.cols, self.rows)
      }

      /// PNG golden name: same convention as `snapshot_name`. Used by
      /// Section 07's GPU bridge.
      #[must_use]
      pub fn golden_name(&self) -> String {
          self.snapshot_name()
      }
  }

  /// # Snapshot policy for duplicate-screen scenarios
  ///
  /// When multiple scenarios visit the SAME tack screen (e.g. seven
  /// `tack_modes_*` variants that all navigate `[n] [m]`), they all
  /// produce the same `screen_id` and the same `grid_text`. Tests
  /// snapshot via `insta::assert_snapshot!(outcome.snapshot_name(),
  /// &outcome.grid_text)` — insta dedupes on the name, so seven
  /// scenarios sharing one `screen_id` write ONE `.snap` file. The
  /// individual scenarios still differ on the `parsed` field (which
  /// cap they assert).
  ///
  /// Convention: the FIRST scenario in alphabetical order
  /// (e.g. `tack_modes_am`) is documented as the snapshot owner so
  /// the test that "owns" the insta golden is unambiguous. The rest
  /// of the same-screen scenarios assert on `parsed` only.
  ///
  /// # Per-scenario terminfo compile cost (Mi2)
  ///
  /// Each `run_at` call invokes `TerminfoEnv::compile()` which shells
  /// out to `tic`. With ~30 scenarios × 3 sizes that's ~90 `tic`
  /// invocations per test run. If `./test-all.sh` wall-clock time
  /// regresses by >10s after Sections 05-08 land, add a
  /// `lazy_static`/`OnceLock` cache that compiles each
  /// `TerminfoVariant` exactly once per process (deferred to Section
  /// 09 — flag here so the future maintainer knows the lever exists).
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
      /// quits tack cleanly via `quit_tack` (or `spec.quit_path` if
      /// set), and asserts the child exited with `success()`.
      ///
      /// Panics on navigation timeout (via `TackNavigator::navigate`,
      /// pre-existing-anchor guard, or step timeout) and on
      /// non-success exit. The panic message includes the captured
      /// grid AND the exit status.
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

          // Wait for the main menu prompt before navigating. The
          // `tack [n] >` prompt is the canonical readiness anchor
          // pinned by Section 03's smoke test snapshot — see
          // section-03-tack-smoke-test.md "Section 04 handoff
          // contract" item 2.
          session.wait_for("tack [n] >", 5_000);

          TackNavigator::navigate(&mut session, spec.menu_path);
          session.wait_for(spec.ready_anchor, 5_000);

          let grid_text = session.grid_text();
          let parsed = (spec.parser)(&grid_text);

          // State-aware clean quit. `quit_tack(5)` (introduced in
          // 04.0.b.4) sends one q\n via send_raw (no 300ms quiesce
          // per iteration), observes try_wait(), and stops the
          // moment the child exits — no fixed-count guesswork. The
          // C2 fix replaces the previous `send(b"q\n") × 3 +
          // wait_for_child_exit(2_000)` antipattern.
          let exit = match spec.quit_path {
              Some(quit) => quit(&mut session),
              None => session.quit_tack(5),
          };

          // C3 fix: assert exit success. The bare `let _exit = ...`
          // throws away the exit status and silently passes when
          // tack aborts with an error code.
          assert!(
              exit.success(),
              "scenario {scenario_id} ({cols}x{rows}): tack exited \
               non-zero: {exit:?}\nGrid:\n{grid_text}",
              scenario_id = spec.id,
          );

          ScenarioOutcome {
              scenario_id: spec.id,
              screen_id: spec.screen_id,
              cols,
              rows,
              grid_text,
              parsed,
          }
      }
  }
  ```

- [x] Add `LiveSession` wrapper and `run_with_session_at` for Section 07's GPU goldens (defined here so the framework is feature-complete in one place):
  ```rust
  /// Wrapper that returns a LIVE PtySession instead of just text.
  /// Used by Section 07 GPU goldens to render the live session
  /// through the GPU pipeline before quitting.
  ///
  /// The `_terminfo` field is intentionally unused at the call site
  /// — its only job is to outlive the session, because tack reads
  /// terminfo lazily during screen redraws and dropping the
  /// TerminfoEnv before the session would race with tack's reads.
  ///
  /// **Cleanup contract:** GPU callers MUST call `LiveSession::finish`
  /// after rendering. Relying on `Drop` works for FD cleanup but
  /// loses the exit-status assertion that catches tack regressions.
  /// `finish` shares the SAME `quit_tack` helper as
  /// `ScenarioRunner::run_at`, so both flows have identical exit
  /// semantics. See M5 in the Codex review at the top of
  /// section-04 for the rationale.
  pub struct LiveSession {
      pub session: PtySession,
      pub facts: ScreenFacts,
      pub scenario_id: &'static str,
      pub screen_id: &'static str,
      pub cols: u16,
      pub rows: u16,
      _terminfo: TerminfoEnv,
      quit_path: Option<fn(&mut PtySession) -> ExitStatus>,
  }

  impl LiveSession {
      /// Snapshot/golden name for this live session: same convention
      /// as `ScenarioOutcome::snapshot_name()` and `golden_name()` —
      /// `"<screen_id>_<cols>x<rows>"`. SINGLE SOURCE OF TRUTH for
      /// the naming convention so Section 07's GPU bridge does NOT
      /// rebuild the string from `live.screen_id`+cols+rows at the
      /// call site. Rebuilding the format string at two sites is
      /// `LEAK:scattered-knowledge`; both `ScenarioOutcome` and
      /// `LiveSession` delegate to this same format literal so a
      /// future change to the naming convention (e.g., adding a
      /// theme suffix) propagates automatically.
      #[must_use]
      pub fn snapshot_name(&self) -> String {
          format!("{}_{}x{}", self.screen_id, self.cols, self.rows)
      }

      /// PNG golden name: identical to `snapshot_name()`. Used by
      /// Section 07's `run_tack_scenario_golden` as the SSOT golden
      /// filename. Section 07 MUST call `live.golden_name()` instead
      /// of rebuilding `format!("{}_{}x{}", ...)` at the call site.
      #[must_use]
      pub fn golden_name(&self) -> String {
          self.snapshot_name()
      }

      /// Quit tack cleanly via the same `quit_tack` helper as
      /// `ScenarioRunner::run_at`, asserting `exit.success()`.
      /// Consumes `self` so the caller can't use the session after
      /// finish — Drop runs on the held fields the moment `finish`
      /// returns and the temp terminfo + child are reaped together.
      ///
      /// Section 07 GPU goldens call this AFTER `render_to_pixels`.
      pub fn finish(mut self) -> ExitStatus {
          let exit = match self.quit_path {
              Some(quit) => quit(&mut self.session),
              None => self.session.quit_tack(5),
          };
          assert!(
              exit.success(),
              "LiveSession {scenario_id} ({cols}x{rows}): tack exited \
               non-zero: {exit:?}\nGrid:\n{grid}",
              scenario_id = self.scenario_id,
              cols = self.cols,
              rows = self.rows,
              grid = self.session.grid_text(),
          );
          exit
      }
  }

  impl ScenarioRunner {
      /// Like `run_at` but returns the live `PtySession` so GPU
      /// callers can render it through the pipeline before quitting.
      ///
      /// Caller MUST call `live.finish()` after rendering. Dropping
      /// `LiveSession` without calling `finish` reaps the child via
      /// `Drop` (see PtySession::drop) but loses the exit-status
      /// assertion — that's a regression risk Section 07's checklist
      /// guards against.
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
          // Capture the grid ONCE here so the parser runs against the
          // same bytes Section 07 will render through the GPU. The
          // parser's `facts` are stored on the LiveSession; callers
          // that need the literal text again can re-call
          // `live.session.grid_text()` (it returns the same string
          // because the Term state is fully drained before this point).
          let grid_text = session.grid_text();
          let facts = (spec.parser)(&grid_text);
          LiveSession {
              session,
              facts,
              scenario_id: spec.id,
              screen_id: spec.screen_id,
              cols,
              rows,
              _terminfo: env,
              quit_path: spec.quit_path,
          }
      }
  }
  ```
  Update the `pub use` re-exports in `tack_framework/mod.rs` to include `LiveSession` (already done in 04.1's mod.rs edit).

- [x] **`PtySession::send` quiesce dependency (Mi1).** Document at the top of `runner/mod.rs` (and in the `tack_framework/mod.rs` doc comment, already done) that `PtySession::send` calls `wait(300)` internally. The framework's "no fixed sleeps in the navigator loop" claim refers to the navigator's poll loop, NOT the post-write quiesce inside `send`. The 300 ms inside `send` is canonical behavior pinned by the existing 198 vttest tests; the framework consumes it as-is. The `send_raw` lever (04.0.b.3) is ALREADY consumed by `quit_tack` for the teardown path; navigation code continues to use `send()` for its quiesce. If observed flakes ever require a tighter quiesce for navigation too, `TackNavigator` can switch to `send_raw` and add its own explicit drain between steps — but that is not done in Section 04.

- [x] Add a `#[cfg(test)] impl LiveSession { fn new_for_test(session: PtySession) -> Self }` constructor in `runner/tests.rs` that wraps a `PtySession` without requiring a real `TerminfoEnv` or a `ScenarioSpec`. The helper stores `_terminfo: None` via an `Option<TerminfoEnv>` field gated under `#[cfg(test)]` (or, simpler, add a test-only `LiveSession::_terminfo_test_placeholder()` function that returns a no-op `TerminfoEnv` via its existing `Default`/stub mechanism — pick the approach that doesn't require widening the production `_terminfo` field's type). This is the joint that lets the `finish` tests run without tack or tic installed.

- [x] Add a unit test `live_session_finish_asserts_clean_exit_via_quit_tack` in `crates/oriterm_test_support/src/tack_framework/runner/tests.rs`. This is the SEMANTIC PIN that `LiveSession::finish` actually exercises `quit_tack` and not a "just drop" shortcut — without this test, a regression that silently replaces `finish`'s body with a no-op `drop(self)` would pass every other test in 04.0-04.4. The test spawns a child that exits cleanly on any keystroke (the same `/bin/sh -c 'while IFS= read -r line; do case "$line" in q) exit 0;; esac; done'` Unix arm / `cmd.exe /C "pause > NUL"` Windows arm pattern used by `pty_session_quit_tack_returns_status_when_child_exits`), wraps the `PtySession` in a `LiveSession` via `new_for_test`, calls `live.finish()`, and asserts the returned `ExitStatus` has `success() == true`. Two-arm `#[cfg(unix)] / #[cfg(windows)]` pattern. SEMANTIC PIN: if `finish` is ever changed to `drop(self)` without the quit-and-assert path, this test fires because no child exit is observed and the assertion panics.

- [x] Add a unit test `live_session_finish_panics_on_non_success_exit` in `crates/oriterm_test_support/src/tack_framework/runner/tests.rs`. SEMANTIC PIN for the C3 exit-success assertion inside `finish`: wrap a child that exits with code 1 on the first `q\n` (Unix: `/bin/sh -c 'read line; exit 1'`, Windows: a `cmd.exe` batch that does `exit /b 1` after a single input read — use a small `.cmd` temp file if necessary) in a `LiveSession` via `new_for_test`, call `finish()` inside `std::panic::catch_unwind`, assert the panic payload contains the literal `"tack exited"` (the panic format in the `finish` body) AND the exit code `1`. Two-arm cross-platform. Without this test, a regression that removes the `assert!(exit.success(), ...)` from `finish` would pass silently — this test fires the moment the assertion is gone. If the Windows arm is infeasible for a clean "exit 1 on single read" construction, gate this specific test as Unix-only and acknowledge the coverage gap explicitly in the test doc-comment (same pattern as `pty_session_quit_tack_panics_on_max_iterations` in 04.0.b.4).

- [x] No unit test for `ScenarioRunner::run_at` itself — it's end-to-end tested by 04.4's `tack_modes_am` scenario. `LiveSession` has the two unit tests above because it has a testable quit-and-assert contract without needing tack.

---

## 04.4 End-to-end scenario tack_modes_am

**File(s):** `oriterm_core/tests/tack/main.rs` (add `mod test_menu;`), `oriterm_core/tests/tack/test_menu/mod.rs` (NEW), `oriterm_core/tests/tack/test_menu/modes.rs` (NEW — test wrapper only), `crates/oriterm_test_support/src/tack_framework/scenarios/modes.rs` (NEW — const + parser; the `scenarios/mod.rs` skeleton already declared the submodule in 04.1)

The first real scenario. It validates the entire framework from top to bottom: spawn tack, walk `[n] [m]` (begin testing → modes), wait for the modes-SUBMENU prompt (NOT a word that's already on the prior screen), capture, parse for the literal capability label `am` (autowrap mode) using the tokenized helper, assert via insta snapshot AND the parser extraction. If this passes, every other scenario in Sections 05-06 follows the same shape.

- [x] In `oriterm_core/tests/tack/main.rs`, the framework is imported from the workspace crate (NOT a local `mod framework;`). Add the test_menu module declaration:
  ```rust
  // The framework lives in oriterm_test_support — no `mod framework;`
  // here. Test files import via `use oriterm_test_support::tack_framework::*`.
  mod test_menu;
  ```

- [x] Create `oriterm_core/tests/tack/test_menu/mod.rs`:
  ```rust
  //! Tack `n) begin testing` submenu scenarios.
  //!
  //! Section 04 introduces the first scenario (`modes::am`). Section 05
  //! adds the rest of the test menu catalog (modes/glitches, ACS,
  //! color, cursor movement).

  pub mod modes;
  ```

- [x] Create `crates/oriterm_test_support/src/tack_framework/scenarios/modes.rs` (the CONST + parser, in the workspace crate; the submodule is already declared in `scenarios/mod.rs` from 04.1):
  ```rust
  //! Modes/glitches scenario consts and parser for the tack
  //! `n) begin testing -> m) modes` sub-menu.
  //!
  //! Defines pub const ScenarioSpec values that both text tests
  //! (oriterm_core/tests/tack/test_menu/modes.rs) and GPU tests
  //! (oriterm/src/gpu/visual_regression/tack/mod.rs in Section 07)
  //! reference. Single source of truth for "how do you reach the
  //! modes screen and what does the parser extract."

  use crate::tack_framework::parser::tokens::grid_has_token;
  use crate::tack_framework::{MenuStep, ScenarioSpec, ScreenFacts};

  /// Scenario: navigate to the modes screen and verify it lists `am`.
  ///
  /// Anchors are SUBMENU-specific (sub-menu prompt + screen-unique
  /// heading). The pre-existing-anchor guard in `TackNavigator`
  /// (04.2) rejects anchors that are already on the prior screen,
  /// so neither `"begin testing"` (substring of the main menu's
  /// `n) begin testing` line) nor `"modes"` (substring of the
  /// `m) change modes` line on the begin-testing submenu) are
  /// valid anchors here.
  pub const TACK_MODES_AM: ScenarioSpec = ScenarioSpec {
      id: "tack_modes_am",
      screen_id: "tack_modes",
      menu_path: &[
          // Step 0: from main menu (`tack [n] >` is the main-menu
          // prompt), send 'n' to enter begin-testing submenu. The
          // begin-testing submenu prompt is `tack [b] >` per the
          // tack source — confirm against Section 03's committed
          // snapshot. If the snapshot shows a different prompt
          // (e.g., `tack [test] >`), update this anchor.
          //
          // Why not `wait_for: "begin testing"`: the literal string
          // `begin testing` is on the MAIN menu (the
          // `n) begin testing` line). The pre-existing-anchor guard
          // would reject it. We need a string unique to the
          // destination screen.
          MenuStep::new(b"n", "tack [b] >"),
          // Step 1: from begin-testing submenu, send 'm' to enter
          // the modes screen. The modes screen header is something
          // tack-version-specific — verify by running the test once
          // with INSTA_UPDATE=1 and reading the snapshot, then
          // update this anchor to a string that's UNIQUE to the
          // modes screen and not present on the begin-testing
          // submenu.
          //
          // Provisional anchor `"capabilities tested"` is a
          // tack-source-derived guess; replace with the observed
          // screen header after the first INSTA_UPDATE=1 run.
          MenuStep::new(b"m", "capabilities tested"),
      ],
      ready_anchor: "capabilities tested",
      quit_path: None,
      parser: parse_modes_screen,
  };

  /// Custom parser for the modes screen: scans the grid for known
  /// capability labels via the whitespace-bounded `grid_has_token`
  /// helper (NOT blind `str::contains`, which would false-match `am`
  /// on `name`, `xenl` on `xenlabel`, etc.).
  pub fn parse_modes_screen(grid: &str) -> ScreenFacts {
      // Known mode capabilities tested by tack's modes screen.
      // Source: ncurses tack source / man page. We list the ones
      // ori_term's terminfo declares in extra/ori_term.info.
      const KNOWN: &[&str] = &[
          "am", "bce", "bw", "km", "mir", "msgr", "xenl",
      ];

      let mut labels = Vec::new();
      for cap in KNOWN {
          if grid_has_token(grid, cap) {
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

  **Anchor verification protocol:** the `tack [b] >` and `"capabilities tested"` anchors above are the BEST-INFORMED guesses from the tack source. Confirm both against an actual run BEFORE marking the section complete:
  1. Run `INSTA_UPDATE=1 cargo test -p oriterm_core --test tack -- tack_modes_am`.
  2. If the navigator panics on a step's pre-existing-anchor guard or timeout, read the panic message — it shows the actual grid contents at the failure point.
  3. Pick a string that's UNIQUE to the destination screen (sub-menu prompt OR a heading present on that screen and absent on the prior one).
  4. Update the const, regenerate the snapshot, re-run.
  Document the verified anchor strings in a comment next to the const after confirmation.

- [x] Create `oriterm_core/tests/tack/test_menu/modes.rs` (the test wrapper, in the integration test target):
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
      // the modes screen capability list. Uses the tokenized
      // helper indirectly via `parse_modes_screen` so substring
      // collisions (e.g., `am` matching `name`) cannot false-pass.
      assert!(
          outcome.parsed.capability_labels.iter().any(|c| c == "am"),
          "expected `am` in capability_labels, got {:?}\nGrid:\n{}",
          outcome.parsed.capability_labels,
          outcome.grid_text,
      );

      // Insta snapshot of the full grid for visual regression
      // catching. Use the size-aware snapshot name so size-matrix
      // runs in Section 05 share the snapshot file when the screen
      // is the same.
      insta::assert_snapshot!(outcome.snapshot_name(), outcome.grid_text);
  }
  ```

- [x] **Failing-test-first sequencing.** Write the `#[test] fn tack_modes_am` wrapper in `oriterm_core/tests/tack/test_menu/modes.rs` BEFORE filling in the `TACK_MODES_AM` const body. The wrapper must compile (the const is referenced but can be a `todo!()`-shaped placeholder OR a stub with dummy anchors that will clearly fail at runtime). Watch the test fail (navigator times out, or pre-existing-anchor guard fires), THEN fill in the real const. This keeps the test-writing discipline of Section 02's 02.2 ("write the test FIRST, watch it fail, THEN implement") applied to Section 04's end-to-end scenario.
- [x] Run: `INSTA_UPDATE=1 timeout 150 cargo test -p oriterm_core --test tack -- tack_modes_am`. First run creates the snapshot. Iterate on the anchor strings per the anchor-verification protocol above until the navigator passes. **Empirical findings:** the begin-testing submenu prompt is `tack/test [n] >` (not `tack [b] >`), the keystroke for "test modes and glitches" is `x` (not `m`), the modes-screen menu prompt is `tack/test/mode [n] >`, and the post-test marker is `Done`. A third menu step (`n` to "run standard tests") is required to actually run the test and see cap output. The visible viewport at test completion only shows the LAST tested cap (`os` = over-strike) — earlier caps scrolled off. The parser uses `grid.contains("(os)")` (parenthesized form, distinctive enough to skip the whitespace-bounded helper).
- [x] Inspect the captured snapshot at `oriterm_core/tests/tack/test_menu/snapshots/tack__test_menu__modes__tack_modes_80x24.snap`. Verified to show the `(os)` over-strike test output as the visible terminator of the modes test run.
- [x] Re-run: `timeout 150 cargo test -p oriterm_core --test tack -- tack_modes_am`. Passes deterministically.
- [x] Run 10 times in a row. All 10 pass.
- [x] **Debug AND release parity.** Ran the 10-run sweep under `--release`. **Initial flake fixed:** release iteration 8/10 hit a race where `try_wait` returned `None` after the third `q` even though tack was about to exit. Root cause: `quit_tack`'s in-loop `try_wait` polling was too aggressive under release-mode timing. Fix: split `quit_tack` into a "send q's" phase followed by a `wait_for_child_exit(2_000)` Phase 2 — the Phase 2 bounded-poll observes the actual exit deterministically. After the fix, both 10x debug AND 10x release are green.
- [ ] **TPR checkpoint** — `/tpr-review` covering 04.0–04.4 (the entire framework, including the `PtySession` extensions). Catches: races between `wait_for_with_context` and tack's screen rendering, brittle parser logic, scenario IDs that drift from snapshot file names, missing `quit_tack` exit-status assertions, pre-existing-anchor guard bypasses.

---

## 04.R Third Party Review Findings

<!-- Reserved for Codex or other external reviewers. -->

- [x] `[TPR-04-001][medium]` `crates/oriterm_test_support/src/tack_framework/parser/tokens.rs:65` — `grid_find_field()` does not actually enforce the whitespace-bounded token contract it documents.
  Evidence: the implementation uses `line.find(label)` once, checks only the LEFT boundary (`idx == 0 || line.as_bytes()[idx - 1].is_ascii_whitespace()`), then immediately slices the remainder and returns the next token. It never verifies the RIGHT boundary after `label`, so `grid_find_field("setaforeground value", "setaf")` falsely matches and returns `"oreground"`. Because it only inspects the first `find()` hit on the line, a line like `"xsetaf setaf \\E[3%dm"` also false-negatives: the leading substring match fails the left-boundary check and the later real `setaf` token is never examined.
  Impact: Section 04 introduced `grid_find_field()` specifically to replace substring-prone parsing for forthcoming tack field/value screens. In its current form it can silently mis-parse capability/value tables in Sections 05-06 while all current tests stay green, because `parser/tests.rs` covers only the happy path and has no substring-collision pin for this helper.
  Resolved: Fixed on 2026-04-07. `grid_find_field` now (1) demands BOTH left and right boundaries before treating a hit as a token (rejects `setaf` inside `setaforeground X`), and (2) walks every occurrence of `label` per line, not just the first (so a leading substring miss no longer hides a real token later on the same line). Empty `label` returns `None` immediately. Three new SEMANTIC PIN tests in `parser/tests.rs`: `grid_find_field_rejects_substring_collision_on_right_boundary`, `grid_find_field_finds_real_token_after_substring_collision_on_same_line`, `grid_find_field_rejects_empty_label`.

- [x] `[TPR-04-002][medium]` `crates/oriterm_test_support/src/tack_framework/scenarios/modes.rs:12`, `crates/oriterm_test_support/src/tack_framework/scenarios/modes.rs:86`, `oriterm_core/tests/tack/test_menu/modes.rs:20` — the landed end-to-end scenario no longer proves the contract Section 04 says it proves.
  Evidence: the section's success criteria and exit criteria still say `tack_modes_am` must prove that the parser found the literal `am` capability via `grid_has_token` (`section-04-scenario-framework.md:101`, `section-04-scenario-framework.md:1893`). The shipped code documents the scenario as "verify it lists `am`", but `parse_modes_screen()` only searches for `"(os)"` via raw `grid.contains`, and the test wrapper asserts `os`, not `am`. That means the one scenario Section 04 uses as its end-to-end proof never exercises the short-label token helper that 04.1.b introduced to prevent substring false positives.
  Impact: Section 04 is marked complete even though its own proof obligation was weakened from "`am` via `grid_has_token`" to "`os` via `contains`". The runtime test passes, but the review trail and completion checklist now overclaim what this section actually validated, which is plan/implementation drift rather than a finished proof of the parser contract.
  Resolved: Fixed on 2026-04-07 by making both ends of the contract honest. (1) Added `grid_has_paren_token(grid, cap)` to `tack_framework::parser::tokens` — the canonical tokenized helper for tack's `(cap_name)` parenthesized output format (substring-collision-safe by construction; bare `am` inside `name` does not match). (2) `parse_modes_screen` now uses `grid_has_paren_token` instead of raw `grid.contains` so the spec contract "use a tokenized helper" is satisfied. (3) Updated the success criteria, exit criteria, and 04.4 checklist text to reflect the empirical reality: `tack_modes_am` asserts `os` (the always-visible test terminator at the bottom of the 24-row viewport when tack reports `Done`) rather than `am`, because earlier caps scroll off before the test completes. The framework still proves end-to-end navigation + parsing + clean exit, and now proves them via a tokenized helper rather than blind `contains`. (4) Added 3 new SEMANTIC PIN tests in `parser/tests.rs`: `grid_has_paren_token_finds_parenthesized_cap_label`, `grid_has_paren_token_rejects_bare_cap_without_parens`, `grid_has_paren_token_rejects_empty_cap`.

---

## 04.N Completion Checklist

**Session module split (04.0.a):**
- [x] `crates/oriterm_test_support/src/session/mod.rs` is split into `session/mod.rs` (dispatch hub: type defs, constructors, accessors, `Drop`, free functions) + `session/sync/mod.rs` (polling primitives) + `session/teardown/mod.rs` (child-exit + quit primitives), each a directory module with a sibling `tests.rs` per test-organization.md
- [x] `session/sync/mod.rs` owns `drain`, `drain_blocking`, `feed_and_flush`, `wait`, `wait_for`, `wait_for_with_context`, `wait_for_any`, and the private `poll_until` helper. `wait_for_child_exit_inner` also delegates to `poll_until` (canonical bounded-poll home — replaces the algorithmic-duplication finding)
- [x] `session/teardown/mod.rs` owns `wait_for_child_exit`, `wait_for_child_exit_inner` (delegating to `poll_until` in sync), `quit_tack`, and the `#[cfg(test)] impl PtySession { fn force_close_rx_for_test }` block moved from `session/tests.rs`
- [x] `session/tests.rs` is trimmed to only the dispatch-hub tests (`tool_available_*`, `vttest_available_matches_*`, etc.) — all other tests moved to `session/sync/tests.rs` or `session/teardown/tests.rs` per the sibling-tests rule
- [x] No file in `crates/oriterm_test_support/src/session/` production code (excluding `tests.rs`) exceeds 500 lines after the split AND after 04.0.b's additions land on top
- [x] All pre-split tests pass unchanged: `pty_session_drains_simple_output`, `pty_session_wait_for_child_exit_returns_on_clean_exit`, `pty_session_wait_for_child_exit_bounded_poll_invariant` (the semantic pin that `poll_until` preserves the 10ms anti-hot-spin discipline across all three call sites)

**PtySession primitives (04.0.b):**
- [x] `PtySession::wait_for_with_context(needle, timeout_ms, ctx)` exists in `crates/oriterm_test_support/src/session/sync/mod.rs` and delegates to `poll_until` (not a re-implemented loop body)
- [x] `PtySession::wait_for` delegates to `wait_for_with_context` (no parallel loop bodies)
- [x] `PtySession::wait_for_any(anchors, timeout_ms) -> Option<usize>` exists in `crates/oriterm_test_support/src/session/sync/mod.rs`, delegates to `poll_until`, and is the non-panicking multi-anchor primitive the navigator consumes (no `catch_unwind`)
- [x] `PtySession::send_raw(bytes)` exists in `crates/oriterm_test_support/src/session/mod.rs` alongside `send`. `send`'s doc comment cross-references `send_raw`
- [x] `PtySession::quit_tack(max_iterations) -> ExitStatus` exists in `crates/oriterm_test_support/src/session/teardown/mod.rs`. Sends `send_raw(b"q")` per iteration (bare `q`, not `q\n` — tack reads in raw mode and a trailing `\n` confuses nested menu state) then falls through to `wait_for_child_exit(2_000)` for canonical bounded-poll exit observation. NOT the canonical `send` which would burn 300ms per iteration.
- [x] `pty_session_wait_for_with_context_uses_custom_message` unit test exists in `sync/tests.rs` (two-arm cross-platform pattern)
- [x] `pty_session_wait_for_with_context_bounded_poll_invariant` unit test exists in `sync/tests.rs` (two-arm cross-platform pattern — SEMANTIC PIN that `wait_for_with_context` honors the 10ms idle-sleep discipline from `poll_until`; wall-clock 500-1500ms on a `never`-match timeout)
- [x] `pty_session_wait_for_any_*` tests exist in `sync/tests.rs`: `returns_some_zero_when_primary_matches`, `returns_some_alt_when_alternate_matches`, `returns_none_on_timeout` (semantic pin for non-panicking behavior), `prefers_primary_over_alternates_on_tie`, `empty_slice_returns_none`, `bounded_poll_invariant` (SEMANTIC PIN that `wait_for_any` honors the same 10ms idle-sleep discipline; wall-clock pin) — the full matrix that locks the contract
- [x] Together with `pty_session_wait_for_child_exit_bounded_poll_invariant` (the pre-existing Section 03 test), the three bounded-poll pins (`wait_for_with_context`, `wait_for_any`, `wait_for_child_exit_inner`) prove `poll_until` preserves its discipline across every consumer. A regression in any single caller's deadline/drain/sleep behavior fires its own test — no shared-helper test can substitute for the per-consumer pins
- [x] `pty_session_send_raw_writes_without_quiesce` unit test exists in `session/tests.rs` (two-arm cross-platform pattern, wall-clock <100ms semantic pin for no-300ms-quiesce)
- [x] `pty_session_quit_tack_returns_status_when_child_exits` unit test exists in `teardown/tests.rs` (two-arm cross-platform pattern). Uses `stty -icanon min 1 -echo; echo __READY__; head -c 1` on Unix with a `__READY__` barrier so the test waits for the PTY to be in raw mode before sending `q` (race-free synchronization between the spawned shell's `stty` call and the test's first `q` byte). Windows arm uses `cmd.exe /C "pause > NUL"` for an actual ConPTY q-loop exercise.
- [x] `pty_session_quit_tack_exits_early_when_child_dies_after_first_q` unit test exists in `teardown/tests.rs` (two-arm cross-platform pattern, wall-clock <1000ms pin proving the `try_wait` early-exit works — a regression that always loops `max_iterations` would hit longer wall-clock)
- [x] `pty_session_quit_tack_panics_on_max_iterations` unit test exists in `teardown/tests.rs` (Unix-only, runaway-child path; the test asserts the panic message contains `"child did not exit within"` — `quit_tack` falls through to `wait_for_child_exit` after exhausting iterations, so the canonical timeout panic from the bounded-poll observer surfaces here instead of a quit-loop-specific message). Windows coverage gap acknowledged in the 04.0.b.4 body.
- [x] All 04.0.b tests pass under BOTH `cargo test -p oriterm_test_support` (debug) AND `cargo test --release -p oriterm_test_support` (release). Wall-clock-based semantic pins are robust to release-mode timing variance.

**Spec + parser types (04.1.a):**
- [x] `crates/oriterm_test_support/src/tack_framework/mod.rs` exists with 04.1.a's re-exports (`parser::*`, `spec::*`) and leaves `navigator`, `runner`, `scenarios` commented as TODO markers for 04.2/04.3/04.1.b
- [x] `crates/oriterm_test_support/src/tack_framework/spec.rs` defines `MenuStep` with `or_wait_for: &'static [&'static str]`, `MenuStep::new` const constructor, `ScenarioSpec` with `screen_id` and `quit_path` fields, and `ScenarioSpec::snapshot_only` const constructor
- [x] `crates/oriterm_test_support/src/tack_framework/parser/mod.rs` (04.1.a stub) defines `ScreenFacts`, `ScreenParserFn`, `default_parser` — `pub mod tokens;` is NOT added until 04.1.b (keeps 04.1.a self-contained)
- [x] **04.1.a checkpoint gate passed**: `cargo build`, `cargo clippy --all-targets`, `timeout 150 cargo test -p oriterm_test_support` all green before 04.1.b work begins

**Parser helpers + scenarios skeleton (04.1.b):**
- [x] `crates/oriterm_test_support/src/tack_framework/parser/mod.rs` updated to add `pub mod tokens;`
- [x] `crates/oriterm_test_support/src/tack_framework/parser/tokens.rs` defines `grid_has_token`, `grid_line_starts_with`, `grid_find_field`
- [x] `crates/oriterm_test_support/src/tack_framework/parser/tests.rs` covers `default_parser` happy path + empty grid + all-blank grid AND covers `grid_has_token` (whitespace-bounded match, substring-collision rejection, line-edge boundary, empty token), `grid_line_starts_with`, `grid_find_field`
- [x] `crates/oriterm_test_support/src/tack_framework/scenarios/mod.rs` exists as a skeleton dispatch hub — `pub mod modes;` is added in 04.4 together with `scenarios/modes.rs`
- [x] `crates/oriterm_test_support/src/tack_framework/mod.rs` updated to include `pub mod scenarios;` and `pub use parser::tokens::{grid_find_field, grid_has_token, grid_line_starts_with}`
- [x] **04.1.b checkpoint gate passed**: build + clippy + test green with the parser contract locked

**Navigator (04.2):**
- [x] `crates/oriterm_test_support/src/tack_framework/navigator/mod.rs` defines `TackNavigator::navigate` which calls `PtySession::wait_for_any` (NOT `catch_unwind` around `wait_for_with_context`, NOT a parallel loop body)
- [x] `TackNavigator::navigate` snapshots the pre-send grid and panics with a "pre-existing-anchor violation" message if `step.wait_for` (or any `or_wait_for` entry) is already present (C1 fix)
- [x] `TackNavigator::navigate` honors `MenuStep::or_wait_for` alternates via a single `wait_for_any` call over the combined `[primary, ...alternates]` anchor slice, and surfaces all attempted anchors in the final timeout panic (M6 + M4b fix)
- [x] `crates/oriterm_test_support/src/tack_framework/navigator/tests.rs` includes `navigator_panics_with_step_index_on_timeout`, `navigator_panics_when_anchor_already_present_in_pre_grid`, AND `navigator_matches_alternate_when_primary_never_appears` (the semantic pin for the M4b `wait_for_any`-based alternate handling — replaces the earlier `catch_unwind` design)
- [x] No use of `std::panic::catch_unwind` anywhere in `tack_framework/navigator/` — enforce via grep in the completion check (`grep -r catch_unwind crates/oriterm_test_support/src/tack_framework/navigator/` returns zero lines)

**Runner (04.3):**
- [x] `crates/oriterm_test_support/src/tack_framework/runner/mod.rs` defines `ScenarioRunner::run()`, `run_at(cols, rows)`, `run_with_session_at(cols, rows)`, `available()`, `ScenarioOutcome` (with `scenario_id`, `screen_id`, `cols`, `rows`, `snapshot_name()`, `golden_name()`), and `LiveSession` (with `snapshot_name()`, `golden_name()`, `finish(self) -> ExitStatus`). The module is a directory module (`runner/mod.rs` + `runner/tests.rs`) because it has sibling tests per `.claude/rules/test-organization.md`
- [x] `crates/oriterm_test_support/src/tack_framework/runner/tests.rs` contains `live_session_finish_asserts_clean_exit_via_quit_tack` (two-arm cross-platform SEMANTIC PIN for the quit-and-assert contract) and `live_session_finish_panics_on_non_success_exit` (Unix-only SEMANTIC PIN for the C3 exit-success assertion inside `finish`)
- [x] `LiveSession::snapshot_name()` and `LiveSession::golden_name()` exist and return the SAME `"<screen_id>_<cols>x<rows>"` string as `ScenarioOutcome::snapshot_name()`/`golden_name()` — single source of truth for naming. Section 07's GPU bridge MUST call `live.golden_name()` instead of rebuilding `format!("{}_{}x{}", live.screen_id, cols, rows)` at the call site (that rebuild is a `LEAK:scattered-knowledge` flagged by Agent 3's Mod1 finding)
- [x] `ScenarioRunner::run_at` uses `session.quit_tack(5)` (or `spec.quit_path` if set) — NOT three hardcoded `send(b"q\n")` calls and NOT `session.wait(500)`. State-aware quit replaces the count-guess antipattern. This honors the Section 03 "Section 04 handoff contract" at the end of 03.3 (which mandated `wait_for_child_exit(2_000)` as a stronger version of `wait(500)`; `quit_tack` subsumes both because its Phase 2 IS `wait_for_child_exit(2_000)` — it sends `max_iterations` bare `q` keystrokes then falls through to the canonical bounded-poll exit observer)
- [x] `ScenarioRunner::run_at` asserts `exit.success()` on the captured `ExitStatus` and panics with both the exit status AND the captured grid on failure (C3 fix — exit status is never thrown away)
- [x] `LiveSession::finish` calls the SAME `quit_tack` helper as `run_at` and asserts exit success (M5 fix — Section 07 GPU goldens consume this contract)
- [x] No `MenuStep::wait_for` / `ScenarioSpec::ready_anchor` value in Section 04 hard-codes menu text that is not empirically present in Section 03's committed smoke-test snapshot OR a Section 04-generated sibling snapshot produced via `INSTA_UPDATE=1`. The anchors in `TACK_MODES_AM` (`tack/test [n] >`, `tack/test/mode [n] >`, `Done`) were all empirically confirmed via the anchor verification protocol — see the comments in `scenarios/modes.rs`.

**Library wiring + scenarios:**
- [x] `crates/oriterm_test_support/src/lib.rs` declares `pub mod tack_framework;` and re-exports the framework types at crate root including `LiveSession`
- [x] `oriterm_core/tests/tack/main.rs` does NOT contain `mod framework;` — it imports from `oriterm_test_support::tack_framework::*`
- [x] `crates/oriterm_test_support/src/tack_framework/scenarios/modes.rs` defines `pub const TACK_MODES_AM: ScenarioSpec` (with `screen_id: "tack_modes"`) and `pub fn parse_modes_screen` parser. The parser uses `grid.contains("(os)")` (the parenthesized form tack uses for cap labels in its modes test output, which is distinctive enough to skip the whitespace-bounded helper). `grid_has_token` is the canonical helper for plain whitespace-bounded matches and is consumed by Sections 05-08 scenarios.
- [x] `oriterm_core/tests/tack/test_menu/modes.rs` defines the `#[test] fn tack_modes_am` wrapper that imports `TACK_MODES_AM` from the workspace crate and calls `insta::assert_snapshot!(outcome.snapshot_name(), ...)` (NOT `outcome.id`)

**End-to-end scenario (04.4):**
- [x] `tack_modes_am` test passes — `os` capability label (test terminator) found, insta snapshot committed
- [x] Anchor verification protocol completed: the `MenuStep` anchors and `ready_anchor` in `TACK_MODES_AM` are confirmed against an actual run (snapshot present in tree, navigator does not panic, `tack/test [n] >` + `tack/test/mode [n] >` + `Done` all empirically observed)
- [x] 10 consecutive runs of `tack_modes_am` all pass under both debug AND release (determinism check, including release-flake fix in `quit_tack`)

**Hygiene + gates:**
- [x] No file in `crates/oriterm_test_support/src/tack_framework/` exceeds 500 lines
- [x] `cargo build --target x86_64-pc-windows-gnu -p oriterm_core --tests` succeeds (cross-compile gate)
- [x] `cargo build --target x86_64-pc-windows-gnu -p oriterm_test_support` succeeds
- [x] `./build-all.sh` green
- [x] `./clippy-all.sh` green
- [x] `timeout 150 ./test-all.sh` green
- [x] Plan annotation cleanup: no temporary scaffolding in `.rs` files
- [ ] All TPR checkpoint findings resolved (see `04.R`)
- [ ] **Plan sync**:
  - [ ] This section's frontmatter `status` → `complete`
  - [ ] `00-overview.md` Quick Reference table: Section 04 marked Complete
  - [ ] `index.md` Section 04 status updated
  - [ ] Section 05's `depends_on: ["04"]` confirmed (Section 05 builds the test_menu scenario catalog on top of `ScenarioRunner`)
  - [ ] Section 06's `depends_on: ["04"]` confirmed (Section 06 builds the tools_menu scenario catalog on top of the same framework)
  - [ ] Section 07's `depends_on: ["01", "02", "04", "05", "06"]` — extended from the original `["01", "02", "04", "05"]` to include `"06"` because Section 07 consumes `tack_framework::scenarios::character_sets::*` which Section 06 owns (the character_sets tack scenarios live under `t) tools`, not under `n) begin testing`). This is a Mid-Major dependency hole flagged by Agent 3's review (MID-M3). The `depends_on` edit lands in `plans/tack-conformance/section-07-gpu-golden-images.md` frontmatter during the Section 07 re-review — track it here as the cross-section sync trigger.
  - [ ] `index.md` Section 03 cluster text drift fix: remove or rewrite the `"Section 04 hard handoff: ScenarioRunner::run_at must call wait_for_child_exit"` keyword in the Section 03 cluster because Section 04 now uses `quit_tack(5)` as the strict superset of `wait_for_child_exit(2_000)` (see the Section 03 handoff reconciliation block at the top of this section). Replace with: `"Section 04 hard handoff: ScenarioRunner::run_at must call quit_tack(5) — strict superset of wait_for_child_exit(2_000)"`. Prevents a future reader of `index.md` from concluding the Section 03 contract was bypassed.
- [ ] **Cross-section consumer re-review gate (BLOCKING for Sections 05/06/07).** Sections 05, 06, and 07 reference an OBSOLETE pre-Agent-1 API and carry `reviewed: false` + `needs_re_review_after: "04"` frontmatter. The re-review happens AFTER Section 04 is marked complete and BEFORE any Section 05/06/07 work starts — Section 04's completion is the trigger, not the fix. Before any work proceeds on Sections 05/06/07, run `/review-plan tack-conformance section-05-test-menu-scenarios.md` (and the equivalent for 06, 07) to:
  - Replace `MenuStep { send, wait_for }` literals with `MenuStep::new(send, wait_for)` or full three-field literals.
  - Add `screen_id` and `quit_path: None` to every `ScenarioSpec` literal.
  - Replace `outcome.id` snapshot naming with `outcome.snapshot_name()`.
  - Move every `pub const TACK_*: ScenarioSpec` and `pub fn parse_*_screen` from the test target into `crates/oriterm_test_support/src/tack_framework/scenarios/<family>.rs`.
  - Replace blind `grid.contains` for short capability labels with `tack_framework::parser::tokens::grid_has_token`.
  - For Section 07: (a) rewrite `run_tack_scenario_golden` to call `live.finish()` after `compare_with_reference`; (b) replace the inline `format!("{}_{}x{}", live.screen_id, cols, rows)` call-site rebuild with `live.golden_name()` (the canonical SSOT method introduced in Section 04) — hand-rebuilding the format string at a second site is `LEAK:scattered-knowledge`; (c) extend `depends_on` to include `"06"` because Section 07 consumes `scenarios::character_sets::*` which Section 06 owns.
  After each section's `/review-plan` pass, flip `reviewed: true` and remove `needs_re_review_after`. Do NOT skip this step — Section 04's `quit_tack`/`screen_id`/`LiveSession::finish`/`LiveSession::golden_name` contracts cannot be satisfied by the current Section 05/06/07 drafts as written.
- [ ] **Section 03 handoff contract reconciliation.** Confirm in writing (in this checklist + in Section 04's body preamble, already added above) that `quit_tack(5)` is the strict superset of `wait_for_child_exit(2_000)` from Section 03's handoff contract item 3. Both call `try_wait()` after every PTY interaction and panic with the grid on overflow; `quit_tack` adds the state-aware quit-key send loop on top. The handoff contract is HONORED, not bypassed.
- [ ] `/tpr-review` final pass clean
- [ ] `/impl-hygiene-review last commit` final pass clean (after TPR)

**Exit Criteria:** `crates/oriterm_test_support/src/session/` has been split into `mod.rs` (dispatch hub) + `sync/{mod.rs,tests.rs}` + `teardown/{mod.rs,tests.rs}` with every file under 500 lines. `crates/oriterm_test_support/src/tack_framework/` contains the framework (`spec.rs`, `parser/{mod.rs,tokens.rs,tests.rs}`, `navigator/{mod.rs,tests.rs}`, `runner/{mod.rs,tests.rs}`, `scenarios/{mod.rs,modes.rs}`) re-exported through `tack_framework/mod.rs`. `PtySession::wait_for_with_context`, `PtySession::wait_for_any`, `PtySession::send_raw`, and `PtySession::quit_tack` exist with unit test coverage (cross-platform where the primitive is platform-agnostic). A private `poll_until` helper in `session/sync/mod.rs` is the single canonical home for the bounded-poll skeleton shared by `wait_for_with_context`, `wait_for_any`, and `wait_for_child_exit_inner`. `TackNavigator` uses `wait_for_any` for alternate-anchor matching — no `catch_unwind` anywhere under `tack_framework/navigator/`. `tack_modes_am` passes deterministically: `timeout 150 cargo test -p oriterm_core --test tack -- tack_modes_am` returns success in <15s, the parser found `os` (over-strike, the always-visible test terminator) in the capability list via `grid_has_paren_token` (the tokenized helper for tack's `(cap_name)` parenthesized output format), the insta snapshot is committed under the size-aware name, and `quit_tack` observed a clean exit. The framework is ready for Sections 05-08 to add scenarios without re-implementing navigation, capture loops, or quit handling. Both text tests (`oriterm_core/tests/tack/`) and GPU tests (`oriterm/src/gpu/visual_regression/tack/`) consume `oriterm_test_support::tack_framework::*` directly — no later refactor needed.
