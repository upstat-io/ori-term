---
section: "10"
title: "OSC Suite (full)"
status: not-started
reviewed: false
goal: "Drive every row in `catalog/osc.md`, `catalog/shell-integration.md`, and the non-image rows of `catalog/iterm2.md` (SetMark, RemoteHost, CurrentDir, Copy, ReportCellSize, SetUserVar) from `implemented-unverified` / `stub` / `missing` to `verified`. Section 10 owns the ENTIRE OSC stack — Section 08's post-completion audit (`section-08 Implementation notes 2026-04-14`) recorded that tack scenarios drove ZERO OSC rows. Basic OSC rows (0, 1, 2, 4, 7, 10, 11, 12, 52) stay owned by Section 10, NOT Section 08. This includes OSC 8 hyperlinks, OSC 22/50 cursor icon/shape, OSC 9/99/777 desktop notifications, OSC 104/110/111/112 color reset, OSC 133 semantic prompt, OSC 633 VS Code shell integration, and OSC 1337 non-image sub-ops. Section 10 also lands the prerequisites that make these rows testable: a spec_chain harness layer that routes through `oriterm_mux::shell_integration::RawInterceptor` (the production path for OSC 7/9/99/133/633/777), a completed renderable observer (OSC 8 cell-metadata assertions), a Term-level mouse-cursor-icon state (OSC 22), an extensible OSC 1337 sub-dispatcher (handed off to Section 14 for images), and the activation of the dormant `PendingResponse` polling path (OSC 52 ResponseToken round-trip)."
success_criteria:
  - "Every row in `catalog/osc.md` is `verified` or `verified-with-deviation` (no `implemented-unverified`, no `stub`, no `missing`) — this includes the basic subset 08 left unverified (OSC 0/1/2/4/7/10/11/12/52) and the advanced subset (OSC 8/22/50/104/110/111/112/9/99/777/133/633 and the non-image OSC 1337 sub-ops)"
  - "Every row in `catalog/shell-integration.md` is `verified` (OSC-7-CWD, OSC-133 A/B/C/D, OSC-633 VS Code, OSC-1337-RemoteHost / CurrentDir / SetMark / SetUserVar / ReportCellSize shell-integration cross-refs, OSC-9/777 notification cross-refs)"
  - "The non-image rows of `catalog/iterm2.md` (ITERM2-1337-REMOTEHOST, ITERM2-1337-CURRENTDIR, ITERM2-1337-COPY, ITERM2-1337-SETMARK, ITERM2-1337-REPORTCELLSIZE, ITERM2-1337-SETUSERVAR) are `verified`; `owner_section` in `catalog/iterm2.md` front-matter is updated so Section 10 owns these rows and Section 14 owns ONLY `ITERM2-1337-FILE` + image-adjacent rows — cross-checked against the ownership conflict currently at `section-14-iterm2-images.md:55` and `catalog/iterm2.md:14`"
  - "`SpecHarness` (crates/oriterm_test_support/src/spec_chain/api.rs) gains a `mux_layer` capability that runs `RawInterceptor::osc_dispatch` on the SAME bytes before the high-level processor — OSC 7 / 9 / 99 / 133 / 633 / 777 are verified against the REAL production path (`oriterm_mux/src/shell_integration/interceptor.rs`), not against the high-level `Processor` which drops them"
  - "`observe_renderable` (crates/oriterm_test_support/src/spec_chain/observers/renderable.rs) is no longer a stub — it asserts cell hyperlink URI, cursor position, cursor shape, palette entries, and damaged lines. Every OSC 8 subsection test exercises this observer with a scenario that would FAIL if the observer remained a stub (semantic pin against `RungResult::pass(rung)` stub-behavior)"
  - "OSC 8 hyperlink rows verified — cell-attached URI survives reflow, scroll into scrollback, copy (cell metadata), and alt-screen toggle; the OSC 8 terminator (empty URI) cancels the attachment on subsequent cells; `id=<id>` parameter is preserved but does not change attachment semantics (per gist:egmontkob)"
  - "OSC 52 clipboard rows verified — both `c` / `s` / `p` / `q` clipboard characters, store and load; `HostRequest::ClipboardLoad` apex with `ResponseToken` round-trip is tested end-to-end through the activated `response_poll` path (section 10.2 removes the `#[allow(dead_code, reason = \"dormant during legacy phase\")]` gate on `PaneIoThread::register_host_request_response` and wires it into the IO thread for the spec_chain verification harness)"
  - "OSC 9 / 99 / 777 desktop notification rows verified — `Effect::Host(HostEffect::DesktopNotification { source, title, body })` is observed with the correct `NotificationSource` discriminator (`Osc9`, `Osc99`, `Osc777`); empty-body and missing-title cases are pinned so `String::from_utf8_lossy` boundary behavior is stable"
  - "OSC 133 semantic prompt rows verified — OSC 133;A/B/C/D each drive the `PromptState` state machine correctly AND update `PromptMarker`. The plan explicitly documents (in 10.4 body) that OSC 133;D does NOT write a D-field into `PromptMarker` (the struct has only `prompt` / `command` / `output`) — D clears `prompt_state` and emits `HostEffect::CommandComplete`. Any Success Criteria that asserted D 'records a marker' is rewritten to match the actual data model. The `command_start`/`finish_command` timing path uses an INJECTABLE clock so the `HostEffect::CommandComplete { duration }` assertion is deterministic (no wall-clock reliance)"
  - "OSC 633 VS Code shell integration rows verified against the authoritative VS Code source at `https://github.com/microsoft/vscode/blob/main/src/vs/workbench/contrib/terminal/browser/xterm/shellIntegrationAddon.ts` — every OSC 633 sub-command that VS Code emits is catalogued and tested; any sub-command not yet dispatched (OSC 633 is currently MISSING per `catalog/osc.md:56`) lands its dispatch arm in `crates/vte/src/ansi/dispatch/osc.rs` AND its handler in `oriterm_mux/src/shell_integration/interceptor.rs`"
  - "OSC 22 cursor icon row verified — `Term` grows a `mouse_cursor_icon: Option<CursorIcon>` field (in `oriterm_core/src/term/mod.rs`) and an override of `Handler::set_mouse_cursor_icon` on `Term` that writes to it; `RenderableContent` exposes this state to the rendering consumer; OSC 22's `catalog/osc.md:29` row is promoted from `stub` to `verified` — the current no-op at `crates/vte/src/ansi/handler.rs:270` is replaced by real state mutation. OSC 22 and OSC 50 (cursor SHAPE) MUST use distinct Term fields — conflation would make reset semantics incorrect (OSC 22 = mouse-cursor icon; OSC 50 = text-cursor shape, already wired via `Term::set_cursor_shape`)"
  - "OSC 50 legacy cursor-shape rows verified — the `CursorShape=N` form with N ∈ {0 block, 1 beam, 2 underline} round-trips through `Term::set_cursor_shape`; DECRQSS-style query (if supported) returns the correct response; OSC 50 with unknown N is dropped via `unhandled` without mutating cursor shape (negative pin)"
  - "OSC 104 / 110 / 111 / 112 color reset rows verified — OSC 104 with zero args resets ALL 256 palette entries to the theme default; OSC 104 with explicit indices resets only those; OSC 110 / 111 / 112 reset Foreground / Background / Cursor default respectively; post-reset state matches the initial theme palette byte-for-byte; subsequent OSC 10/11/12 queries return the theme default values"
  - "OSC 1337 non-image sub-ops verified — the dispatcher at `crates/vte/src/ansi/dispatch/osc.rs:248-254` is refactored into a key=value sub-dispatcher that delegates to named handler methods (`Handler::iterm2_set_mark`, `Handler::iterm2_remote_host`, `Handler::iterm2_current_dir`, `Handler::iterm2_copy`, `Handler::iterm2_report_cell_size`, `Handler::iterm2_set_user_var`, `Handler::iterm2_shell_integration_version`) while preserving the existing `Handler::iterm2_file` arm for Section 14. Cross-cutting with Section 14 is explicitly tracked — Section 14 inherits the sub-dispatcher and adds `File=` verification on top"
  - "New OSC rows previously `missing` in `catalog/osc.md` (OSC-13, OSC-14, OSC-17, OSC-19, OSC-113, OSC-114, OSC-117, OSC-119, OSC-3, OSC-5, OSC-6, OSC-L, OSC-l) each have a dispatch arm, a Term handler, and a verified row. Rows the plan cannot responsibly `verify` (OSC-3 X11-only, OSC-L / OSC-l historical) are promoted to `verified-with-deviation` with a catalog note naming the deviation"
  - "All existing teseq OSC tests at `oriterm_core/tests/teseq/scenarios/osc/{osc_title,osc_icon_name,osc_clipboard,osc_color_query}.teseq` continue to pass unchanged — they are regression guards against OSC 0/1/2/4/10/11/12/52 dispatch basics"
  - "Alloc regression (`oriterm_core/tests/alloc_regression.rs`) stays green — no OSC 10/11/12 query or OSC 52 load reply path may allocate per-byte in the hot path; reply formatting goes through `format_clipboard_reply` / `format_color_reply` in `oriterm_core/src/effect/families/host_request.rs` (already the canonical home) rather than ad-hoc `format!` calls at dispatch sites"
  - "`./build-all.sh` (debug + release + Windows cross-compile via `cargo build --target x86_64-pc-windows-gnu`) green; `./test-all.sh` green (debug workspace sweep); explicit release-mode run `timeout 150 cargo test --workspace --features oriterm/gpu-tests --release` green (release-mode alloc and `#[cfg(debug_assertions)]` divergence is invisible to `./test-all.sh`); `./clippy-all.sh` green (zero new warnings under `deny(clippy::all)` + nursery)"
  - "Section's mission-criterion connection: contributes to **Verification chain complete per row** (every applicable OSC row reaches `verified` with parser → dispatch → state/effect apex green) AND **Effect/State separation enforced** (the OSC 52 ResponseToken activation closes out Section 03's dormant `response_poll` arm)"
inspired_by:
  - "gist:egmontkob — OSC 8 hyperlink canonical spec (`https://gist.github.com/egmontkob/eb114294efbcd5adb1944c9f3cb5feda`)"
  - "Final Term proposal — OSC 133 semantic prompt (FTCS_* markers)"
  - "iTerm2 proprietary-escape-codes documentation — OSC 9, OSC 1337 (non-image sub-ops)"
  - "VS Code `shellIntegrationAddon.ts` — OSC 633 sub-commands + arguments"
  - "kitty terminal docs — OSC 777 desktop notifications (rxvt-unicode lineage)"
  - "xterm `ctlseqs.html` — OSC 0/1/2/4/7/8/10/11/12/22/50/52/104/110/111/112/3/5/13/14/17/19"
  - "wezterm `escape-sequences.md` — de-facto OSC behavior reference across variants"
  - "alacritty `crates/vte/src/ansi/dispatch/osc.rs` (upstream) — dispatcher shape this section extends"
depends_on: ["03", "08"]
third_party_review:
  status: findings
  updated: "2026-04-17"
sections:
  - id: "10.0"
    title: "Harness + observer + state prerequisites (spec_chain mux layer, renderable observer, Term mouse cursor icon field, OSC 1337 sub-dispatcher, response-poll activation, injectable clock)"
    status: not-started
  - id: "10.1"
    title: "OSC 8 hyperlinks — dispatch, cell metadata, reflow/scroll/copy/alt-screen survival"
    status: not-started
  - id: "10.2"
    title: "OSC 52 clipboard — store + load + ResponseToken round-trip (activates response_poll)"
    status: not-started
  - id: "10.3"
    title: "OSC 9 / 99 / 777 desktop notifications — NotificationSource discriminators"
    status: not-started
  - id: "10.4"
    title: "OSC 133 semantic prompt (A/B/C/D) + OSC 633 VS Code shell integration"
    status: not-started
  - id: "10.5"
    title: "OSC 22 cursor icon (new Term state) + OSC 50 cursor shape (existing)"
    status: not-started
  - id: "10.6"
    title: "OSC 104 / 110 / 111 / 112 color reset — palette + default color restoration"
    status: not-started
  - id: "10.7"
    title: "OSC 1337 non-image sub-ops (SetMark, RemoteHost, CurrentDir, Copy, ReportCellSize, SetUserVar, ShellIntegrationVersion)"
    status: not-started
  - id: "10.8"
    title: "Basic OSC rows inherited from Section 08 (0/1/2/4/7/10/11/12/52 — still implemented-unverified)"
    status: not-started
  - id: "10.9"
    title: "Missing OSC rows — dispatch, handler, and verification for OSC 3/5/6/13/14/17/19/113/114/117/119/L/l"
    status: not-started
  - id: "10.R"
    title: "Third Party Review Findings"
    status: not-started
  - id: "10.N"
    title: "Completion Checklist"
    status: not-started
# TPR checkpoints are placed to break the section into reviewable chunks before
# the final sweep at 10.N. The harness/state prerequisites in 10.0 MUST TPR-pass
# before the remaining subsections are written against them, because downstream
# sections depend on the harness API shape being stable.
#
# Checkpoint 1 — after 10.0 (harness + observer + state + sub-dispatcher land).
#   Covers 10.0 only. Catches harness API drift before subsections build on it.
# Checkpoint 2 — after 10.3 (hyperlinks + clipboard + notifications).
#   Covers 10.1–10.3 + re-verifies 10.0. Catches HostRequest / ResponseToken /
#   renderable-observer integration issues before 10.4–10.7 build on top.
# Checkpoint 3 — after 10.7 (shell integration + cursor + color reset + iterm2).
#   Covers 10.4–10.7. Catches Term-state LEAK findings (mouse_cursor_icon,
#   PromptMarker D-field documentation) + OSC 1337 sub-dispatcher correctness
#   before the ownership handoff to Section 14.
# Final — 10.N (full section TPR + impl-hygiene).
---

# Section 10: OSC Suite (full)

**Status:** Not Started
**Goal:** Verify EVERY OSC catalog row — basic (inherited from Section 08) and advanced (Section 10's own Phase 3 Group A expansion). Each OSC number gets a spec_chain test that emits the sequence, observes the apex (state mutation via `observe_state` OR effect transcript via `observe_effect` OR cell metadata via `observe_renderable`), and asserts. This section owns the entire OSC stack plus its prerequisites: harness extensions, Term state additions, dispatcher refactors, and the activation of dormant infrastructure (`response_poll`) that makes OSC 52 testable end-to-end.

**Success Criteria:** see frontmatter.

---

## Scope clarifications (load-bearing — read before writing any tests)

These clarifications resolve the ambiguities reviewers surfaced during the /review-plan blind-spot pass:

### A. Section 08 did NOT verify any OSC rows

`plans/spec-conformance/section-08-ecma-48-baseline.md:179` (Implementation notes, 2026-04-14) explicitly records: *"OSC row ownership audit: tack scenarios drive zero OSC rows — all basic OSC rows (0, 1, 2, 4, 7, 10, 11, 12, 52) remain owned by Section 10."* This Section 10 therefore owns the WHOLE OSC stack. Sub-section **10.8** is a first-class deliverable, not a cleanup note.

### B. Spec_chain harness does NOT route mux-intercepted OSCs through the real production path

`SpecHarness` at `crates/oriterm_test_support/src/spec_chain/api.rs:82-103` wraps `Processor::advance_with_observer` (high-level VTE processor). The production-path interceptor at `oriterm_mux/src/shell_integration/interceptor.rs` runs a SEPARATE raw `vte::Parser` on the SAME bytes BEFORE the high-level processor — this is the only path that sees OSC 7, OSC 9, OSC 99, OSC 133, OSC 633, and OSC 777 (the high-level `Processor::advance_with_observer` silently drops them per the interceptor's own module doc: *"The vte::ansi::Processor does not route OSC 133, OSC 9/99/777, or XTVERSION (CSI >q) to Handler trait methods"*).

Consequence: verifying OSC 7/9/99/133/633/777 via `SpecHarness` without a `mux_layer` extension would test a dispatch path that DOES NOT RUN IN PRODUCTION. Subsection **10.0** lands the `mux_layer` first. Every subsection that verifies a mux-intercepted OSC MUST opt into that layer.

### C. The renderable observer is a no-op stub

`crates/oriterm_test_support/src/spec_chain/observers/renderable.rs:21-29` returns `RungResult::pass(RungName::Renderable)` unconditionally. Every OSC 8 hyperlink test planned against this observer would pass WITHOUT CHECKING ANYTHING — a silent false-green. Subsection **10.0** completes the observer before any OSC 8 test is written; the OSC 8 subsection **10.1** includes a semantic pin that fails if the observer regresses to the stub.

### D. `PromptMarker` has no D-field

The existing `PromptMarker` at `oriterm_core/src/term/mod.rs:60-67` has fields `prompt: usize`, `command: Option<usize>`, `output: Option<usize>` — no fourth field for OSC 133;D. The production handler at `oriterm_mux/src/shell_integration/interceptor.rs:105-112` sets `prompt_state = PromptState::None` and emits `HostEffect::CommandComplete { duration }`. Subsection **10.4**'s D-test MUST assert (i) state returns to `None`, (ii) `CommandComplete { duration }` is on the effect transcript, and (iii) `prompt_markers.last()` retains its A/B/C fields from the completed lifecycle — NOT that a D-row was written.

### E. OSC 22 is an unimplemented stub, not `implemented-unverified`

`crates/vte/src/ansi/handler.rs:270` defines `fn set_mouse_cursor_icon(&mut self, _: CursorIcon) {}` — an empty default on the Handler trait. `Term` at `oriterm_core/src/term/mod.rs` does NOT override it. `catalog/osc.md:29` labels OSC 22 as `stub` accordingly. Subsection **10.5** is not a test-writing exercise — it adds the Term field, the override, and the renderable surface BEFORE writing verification tests.

### F. OSC 1337 parser is monolithic; sub-op ownership is currently tangled

`crates/vte/src/ansi/dispatch/osc.rs:248-254` only routes `File=` to `Handler::iterm2_file` and drops every other sub-op to `unhandled`. `catalog/iterm2.md:15-20` assigns RemoteHost / CurrentDir / Copy / SetMark / ReportCellSize / SetUserVar to Section 14; `section-14-iterm2-images.md:55` says Section 10 already covered them. The canonical resolution (discussed with reviewers): **Section 10 owns all non-image OSC 1337 sub-ops** (the non-image rows are NOT image work). Subsection **10.7** lands the extensible sub-dispatcher and verifies non-image variants. Section 14's `owner_section` in `catalog/iterm2.md` front-matter is updated to `"01 (bootstrap), 10 (non-image), 14 (image)"`. The ownership conflict line at `section-14-iterm2-images.md:55` is updated through the flow-up edit this review authorizes (whole-plan scope would fix `section-14` directly; single-section scope notes the required update in the 10.N completion checklist — Section 14's reviewer will pick it up on its next /review-plan).

### G. OSC 52 ResponseToken is dormant; Section 10 activates it

`oriterm_mux/src/pane/io_thread/response_poll.rs:33-36` is `#[allow(dead_code, reason = "dormant during legacy phase; activates at effect-cutover")]`. The Section 03 migration brought the `Effect::HostRequest` emission online but kept the polling arm dormant because no consumer subscribed. Section 10 is the first consumer — verifying `HostRequest::ClipboardLoad` end-to-end requires the IO thread to actually poll the fulfilled token and emit the `Effect::Pty(PtyEffect::Write { .. })` reply. Subsection **10.2** removes the dead-code gate and wires the call sites.

### H. CWD SSOT — OSC 7 and OSC 133 must write the SAME field

Both OSC 7 (set current working directory) and some OSC 133 variants (when they carry `cwd=<path>` in the parameter string — per Final Term spec) update Term's CWD. The canonical home is `Term::set_cwd(Option<String>)` at `oriterm_core/src/term/shell_state/mod.rs:244-247`. Subsection 10.4's OSC 133 tests MUST go through `Term::set_cwd` (same canonical field as 10.8's OSC 7 tests) — any second CWD field is an SSOT LEAK.

### I. OSC 9 ambiguity — iTerm2 Growl vs Kitty notification protocol

`catalog/osc.md:52` attributes OSC 9 to iTerm2 notifications. Kitty later introduced OSC 99 for its expanded protocol. The interceptor at `oriterm_mux/src/shell_integration/interceptor.rs:124-128` distinguishes them via `NotificationSource::Osc9` / `::Osc99`. Subsection **10.3** pins this discriminator so future rewires (e.g. if Kitty extends OSC 99 with more fields) don't collapse OSC 9 and OSC 99 into one source.

---

## Dependency boundaries

**Depends on:**
- **Section 03** (Effect Boundary Migration, `status: complete`) — `oriterm_core::effect::{Effect, EffectSink, QueueingEffectSink}` + `HostEffect::DesktopNotification` + `HostRequest::ClipboardLoad` + `PendingResponse` infrastructure. Section 10 ACTIVATES the dormant response-poll arm that Section 03 left in place for this section to light up.
- **Section 08** (ECMA-48 baseline, `status: complete`) — basic CSI + SGR behavior is verified, so OSC consumers that observe post-OSC grid state can rely on it. Section 08 ALSO verified the 8-bit C1 control `0x9D` → OSC state entry; subsection 10.8 can use 0x9D as a parser alias in parser rung tests.

**Does not depend on:**
- Section 04 harness *extensions* for visual rungs (5–8) — OSC is non-visual except for OSC 8 cursor/hyperlink rendering, which uses the state/effect apexes at rungs 3–4, not the GPU rungs.
- Sections 11 / 12 / 13 — graphics stacks do not interact with OSC beyond OSC 1337 (image), which is Section 14's concern, not Section 10's.

**Produces outputs consumed by:**
- **Section 14** inherits the OSC 1337 sub-dispatcher landed in 10.7. Section 14 adds only the `File=` arm verification + image rows.
- **Section 16** (mouse) — OSC 22 (mouse cursor icon) lives in Section 10; Section 16 may read `Term::mouse_cursor_icon` when reporting cursor state to the host.
- **Section 21 / 24** (notcurses harness) — real applications use OSC 8 + OSC 52 + OSC 133 routinely; Section 10's verification gate is what lets those sections run real apps without false-green tests.

---

## 10.0 Harness + observer + state prerequisites

**Files:**
- `crates/oriterm_test_support/src/spec_chain/api.rs` (extend)
- `crates/oriterm_test_support/src/spec_chain/observers/renderable.rs` (complete the stub)
- `crates/oriterm_test_support/src/spec_chain/observers/mod.rs` (expose new assertions)
- `crates/oriterm_test_support/src/spec_chain/scenario.rs` (extend `RenderableExpectation`)
- `oriterm_core/src/term/mod.rs` (add `mouse_cursor_icon` field)
- `oriterm_core/src/term/handler/mod.rs` (override `Handler::set_mouse_cursor_icon`)
- `oriterm_core/src/term/renderable/mod.rs` (expose `mouse_cursor_icon` on `RenderableContent`)
- `crates/vte/src/ansi/handler.rs` (add `iterm2_set_mark`, `iterm2_remote_host`, `iterm2_current_dir`, `iterm2_copy`, `iterm2_report_cell_size`, `iterm2_set_user_var`, `iterm2_shell_integration_version` default methods)
- `crates/vte/src/ansi/dispatch/osc.rs` (refactor the `b"1337"` arm into a key=value sub-dispatcher)
- `oriterm_mux/src/pane/io_thread/response_poll.rs` (remove the `#[allow(dead_code)]` gate; add the activation call in `PaneIoThread::drain_events` or equivalent)
- `oriterm_mux/src/pane/io_thread/mod.rs` (inject the clock source for `set_command_start` timing tests)

**Tests (written FIRST per `.claude/rules/tests.md` §TDD for Bugs — VERIFIED RED before implementation):**

- [ ] **Failing test matrix written FIRST** — `spec_chain/tests.rs` harness tests that feed OSC 133;A through `SpecHarness::feed()` MUST fail in a way that proves the high-level processor drops the sequence (the test fails because the expected state change does not occur). After 10.0 lands `SpecHarness::with_mux_layer()`, the same scenario routed through `feed_with_mux()` passes. This failing-then-passing pair is the TDD proof that the `mux_layer` is actually running.
- [ ] **Renderable stub regression pin** — `observers/tests.rs` test that constructs a `RenderableExpectation { hyperlink_at: Some((row, col, "http://example.com")) }` against a `Term` whose cell at (row, col) has a DIFFERENT URI. With the stub, the test passes; with the completed observer, the test fails. Commit the NEGATIVE test first, then complete the observer; the test flips from pass→fail, and THEN we invert the assertion so the final committed test is the semantic pin that requires the observer to actually check.
- [ ] **Term mouse cursor icon pin** — test `term_set_mouse_cursor_icon_stores_icon` at `oriterm_core/src/term/tests.rs` that (i) starts `Term` with `mouse_cursor_icon == None`, (ii) calls `Handler::set_mouse_cursor_icon(&mut term, CursorIcon::Pointer)`, (iii) asserts `term.mouse_cursor_icon() == Some(CursorIcon::Pointer)`. Failing RED before the override is added.
- [ ] **OSC 1337 sub-dispatcher parse pin** — test in `crates/vte/src/ansi/dispatch/tests.rs` (if missing, create) that feeds `\x1b]1337;SetMark\x1b\\` and asserts `Handler::iterm2_set_mark` was called. RED before the sub-dispatcher refactor lands.
- [ ] **Response-poll activation pin** — test `response_poll_emits_pty_write_on_fulfill` in `oriterm_mux/src/pane/io_thread/response_poll/tests.rs` (new) that pushes a `HostRequest::ClipboardLoad` through `PaneIoThread::register_host_request_response`, calls `ResponseToken::fulfill("hello")`, polls, and asserts a `PtyEffect::Write` with the base64-encoded reply appears on the sink. RED until the `#[allow(dead_code)]` gate is removed and `register_host_request_response` is called from the live path.
- [ ] **Injectable clock pin** — test `command_duration_uses_injected_clock` that constructs `Term` with a deterministic clock (`Arc<dyn Fn() -> Instant + Send + Sync>`), feeds `OSC 133;C` then (advance clock 1.5s) `OSC 133;D`, asserts `HostEffect::CommandComplete { duration: 1500ms }`. Without the injectable clock, the assertion is flaky (wall-clock elapsed). RED until the clock is injected.

**Implementation:**

- [ ] Add `SpecHarness::with_mux_layer(self) -> Self` that attaches a `RawInterceptor` wrapper. In `feed_with_mux()`, run the raw `vte::Parser` with a borrowed `RawInterceptor` on the bytes FIRST, then run the high-level `Processor::advance_with_observer` on the SAME bytes. This matches the production order in `oriterm_mux/src/pane/io_thread/mod.rs::handle_bytes()` which runs `self.raw_parser.advance(&mut interceptor, bytes)` before `self.processor.advance(&mut self.terminal, bytes)`. The harness MUST mirror this order exactly — running the interceptor AFTER the processor would be wrong and untestable via the TDD failing-then-passing pair.
- [ ] Complete `observe_renderable` to check every field in `RenderableExpectation`:
  - `cells: Option<Vec<(row, col, ch)>>` — cell contents at specific positions.
  - `hyperlink_at: Option<(row, col, expected_uri: String)>` — assert cell's hyperlink URI matches.
  - `cursor_position: Option<(row, col)>` — assert cursor lives where expected.
  - `cursor_shape: Option<CursorShape>` — assert `Term::cursor_shape()` matches.
  - `palette_index: Option<(index, expected_rgb: Rgb)>` — assert `Term::palette()[index]` matches.
  - `mouse_cursor_icon: Option<CursorIcon>` — assert `Term::mouse_cursor_icon()` matches (WHERE: new state landed in this subsection).
  - `damaged_lines: Option<Vec<Line>>` — assert renderable content reports the expected damage set.
- [ ] Extend `RenderableExpectation` in `scenario.rs` with the fields above; keep existing callers compatible by making fields `Option` with `#[derive(Default)]`.
- [ ] Add `mouse_cursor_icon: Option<CursorIcon>` to `Term<S>`; initialize to `None` in `Term::new()`; add `Term::mouse_cursor_icon(&self)` accessor + `Term::set_mouse_cursor_icon(&mut self, icon: Option<CursorIcon>)` mutator (per `.claude/rules/impl-hygiene.md` §SSOT — canonical home for this knowledge is `Term`).
- [ ] Override `Handler::set_mouse_cursor_icon` on `Term` in `oriterm_core/src/term/handler/mod.rs` to call `Term::set_mouse_cursor_icon(Some(icon))`. WHERE: add next to the other `Handler` trait methods, grouped with cursor-shape handlers.
- [ ] Expose `mouse_cursor_icon` on `RenderableContent` (`oriterm_core/src/term/renderable/mod.rs`) so the rendering consumer can query it. Include it in `renderable_content_into()` writeback (NO allocation — the field is `Option<CursorIcon>`, which is `Copy`).
- [ ] Refactor `crates/vte/src/ansi/dispatch/osc.rs:248-254`:
  ```rust
  b"1337" => {
      if params.len() < 2 { return unhandled(params); }
      dispatch_iterm2_osc1337(handler, &params[1..]);
  },
  ```
  where `dispatch_iterm2_osc1337` is a new private function in the same file that parses the first parameter as `key[=value]` and routes to the appropriate `Handler::iterm2_*` method. The existing `File=` case goes through this dispatcher — it calls `handler.iterm2_file(&params[1..])` when the key is `File`. Preserves current behavior, adds extensibility.
- [ ] Add default no-op methods to the `Handler` trait in `crates/vte/src/ansi/handler.rs` for every new sub-op: `iterm2_set_mark`, `iterm2_remote_host(path: &[u8])`, `iterm2_current_dir(path: &[u8])`, `iterm2_copy(data: &[u8])`, `iterm2_report_cell_size()`, `iterm2_set_user_var(name: &[u8], value: &[u8])`, `iterm2_shell_integration_version(version: &[u8])`. Defaults are empty bodies (drop semantics) — 10.7 overrides each on `Term`.
- [ ] **Response-poll activation requires EffectSink migration (GAP):** `PaneIoThread::register_host_request_response` is gated with `#[allow(dead_code)]` because the IO thread currently uses `LegacyEventSink` whose `drain_into()` is a no-op — effects are forwarded immediately as legacy `Event`s. The `response_poll.rs` module doc explicitly states: "activates when consumers migrate to `QueueingEffectSink` (in `plans/effect-cutover/`)." Section 10 CANNOT simply remove the dead-code gate without also migrating the IO thread to `QueueingEffectSink`. Two valid approaches:
  - **Option A (preferred if effect-cutover is close):** Coordinate Section 10.2 implementation with the effect-cutover plan: migrate the pane IO thread to `QueueingEffectSink` first, then activate `register_host_request_response`. The response-poll test (`response_poll_emits_pty_write_on_fulfill`) only runs after the sink migration is in place.
  - **Option B (scope-bounded):** For spec_chain verification only, wire a test-only shim that injects fulfilled responses directly into the pane IO thread's `pending_responses` vec (bypassing the dead-code path) — this verifies the reply FORMAT without requiring the sink migration. Document clearly that end-to-end production behavior depends on effect-cutover.
  Whichever option is chosen, the 10.0/10.2 checklist MUST call out the dependency on the IO thread's effective sink type BEFORE writing tests that assume the round-trip works end-to-end through `PaneIoThread`.
- [ ] Replace `std::time::Instant::now()` at `oriterm_mux/src/shell_integration/interceptor.rs:102` with a clock-source call routed through `Term` — `Term` grows an optional `clock: Arc<dyn Fn() -> Instant + Send + Sync>` field (default `Arc::new(Instant::now)` in production; tests inject a deterministic one). WHERE: clock field added in `oriterm_core/src/term/mod.rs`; `Term::set_command_start(start)` uses the clock's tick when `start` is `None` and the current design calls `Instant::now()` internally. Preserve production behavior by keeping a `Term::new_default_clock()` constructor; deterministic tests use `Term::with_clock(clock_fn)`.

**Validation:**

- [ ] All five TDD matrix tests transition RED → GREEN.
- [ ] The OSC 133;A scenario routed through `SpecHarness::feed()` still fails (proves the high-level processor really drops OSC 133, not just our test setup).
- [ ] The same scenario through `SpecHarness::feed_with_mux()` passes (proves the mux layer runs).
- [ ] `renderable.rs` NO LONGER contains `RungResult::pass(RungName::Renderable)` as the only return — grep for the string `"Stub: always passes"` returns zero matches.
- [ ] `grep -rn '#\[allow(dead_code, reason = \"dormant during legacy phase'` in `oriterm_mux/` returns zero matches (the gate is removed).
- [ ] `./build-all.sh` + `./test-all.sh` + `./clippy-all.sh` green.
- [ ] **TPR checkpoint 1** — `/tpr-review` covering 10.0 only. Harness API MUST stabilize here before downstream subsections build on it.

---

## 10.1 OSC 8 hyperlinks

**Files:**
- `oriterm_core/tests/spec_chain/osc/hyperlinks.rs` (new — registered as `mod hyperlinks;` inside `spec_chain/osc/mod.rs`)
- `oriterm_core/tests/spec_chain/osc/mod.rs` (new module aggregator)
- `oriterm_core/tests/spec_chain/main.rs` (add `mod osc;`)
- Catalog update: `plans/spec-conformance/catalog/osc.md` (row OSC-8)

**Tests (TDD — RED first):**

- [ ] Spec_chain test `osc8_basic_attach` — feed `\x1b]8;;https://example.com\x1b\\Hello\x1b]8;;\x1b\\` (set URI, text, clear URI). Assert cells 0..5 of current row carry `hyperlink_uri == Some("https://example.com")`; subsequent cells after the clear carry `hyperlink_uri == None`. Uses the completed `observe_renderable` from 10.0 with `hyperlink_at: Some((row, 0, "https://example.com"))` + a negative assertion at (row, 5).
- [ ] `osc8_with_id` — feed `\x1b]8;id=foo;https://example.com\x1b\\X\x1b]8;;\x1b\\`. Assert cell 0 has the URI. **Important apex constraint:** `RenderableCell` at `oriterm_core/src/term/renderable/mod.rs` only carries `hyperlink_uri: Option<String>` — the hyperlink `id` is NOT exposed on the renderable surface. To verify the `id` is preserved in cell metadata, use the **state rung apex** (read `grid[row][col].hyperlink()` via `Term` directly) rather than `observe_renderable`. Verify that `cell.hyperlink().unwrap().id == Some("foo")` at the state rung. Then test that two separate attach/clear cycles with the same `id` both carry `id == Some("foo")` (confirming `id` does not get cleared between cycles). The renderable rung assertion covers only the URI presence; the state rung assertion covers the `id`.
- [ ] `osc8_survives_reflow` — place hyperlinked text at row 0. Resize grid from 80 to 40 columns. Assert the wrapped cells (now spread across row 0 and row 1) ALL carry the same URI. (This catches the reflow-drops-metadata regression pattern from the alacritty / wezterm code history.)
- [ ] `osc8_survives_scrollback` — place hyperlinked text, then feed enough newlines that the row scrolls into `Grid::scrollback`. Assert the scrollback row still carries the URI on every cell. Uses `grid.scrollback()[...]` via the completed renderable observer.
- [ ] `osc8_terminator_cancels_attachment` — feed text, `OSC 8 ; ; uri ST`, text-A, `OSC 8 ; ; ST`, text-B. Assert text-B cells have `hyperlink_uri == None` (the empty URI terminates the attachment).
- [ ] `osc8_malformed_uri_dropped` — feed `\x1b]8;; BROKEN URI WITH SPACES \x1b\\X\x1b]8;;\x1b\\` and assert the cell carries the URI as-is (whitespace is not syntactically restricted in OSC 8 params — the terminal does not validate; it records). Negative pin: feed truncated `\x1b]8;;\x1b` (no ST) and assert no URI is attached (parser aborts on timeout / sequence boundary).
- [ ] `osc8_alt_screen_toggle_clears` — enter alt screen, attach hyperlink, leave alt screen. Assert primary screen cells are unaffected (alt-screen hyperlinks do NOT bleed).
- [ ] **Semantic pin** — `osc8_renderable_observer_not_stub` — scenario asserts `hyperlink_at: Some((0, 0, "WRONG_URI"))` against an actual URI of `"http://example.com"`. Must FAIL. If it passes, the renderable observer has regressed to the 10.0 stub.

**Implementation prerequisites (verified from catalog/osc.md):**

OSC 8 dispatch at `crates/vte/src/ansi/dispatch/osc.rs` (`b"8"` arm) already routes to `handler.set_hyperlink()`; `Term::set_hyperlink` → `Term::osc_set_hyperlink` at `oriterm_core/src/term/handler/osc.rs` already attaches URI to cells. No new dispatch work. Section 10.1 is pure verification.

**Catalog update:**

- [ ] Promote OSC-8 in `catalog/osc.md` from `implemented-unverified` → `verified`. Fill `Test chain` cell with `parser:passed dispatch:passed state:passed` + citation of `oriterm_core/tests/spec_chain/osc/hyperlinks.rs::{osc8_basic_attach, osc8_with_id, osc8_survives_reflow, osc8_survives_scrollback, osc8_terminator_cancels_attachment, osc8_malformed_uri_dropped, osc8_alt_screen_toggle_clears}`.

**Validation:**

- [ ] All 8 tests pass (7 behavioral + 1 semantic pin).
- [ ] `observe_renderable` is exercised with a real expectation in every test (no test relies on rung pass-through).
- [ ] `./test-all.sh` green.

---

## 10.2 OSC 52 clipboard

**Files:**
- `oriterm_core/tests/spec_chain/osc/clipboard.rs` (new)
- `oriterm_mux/src/pane/io_thread/response_poll/tests.rs` (new — direct tests of the activated polling path)
- Catalog update: `plans/spec-conformance/catalog/osc.md` (rows OSC-52-STORE, OSC-52-LOAD)

**Tests (TDD — RED first):**

- [ ] `osc52_store_clipboard_c` — feed `\x1b]52;c;SGVsbG8=\x1b\\`, assert `Effect::HostRequest(HostRequest::ClipboardLoad { .. })` is NOT emitted (this is a store, not a load), and assert the legacy `Event::ClipboardStore` (via the `LegacyEventSink` adapter) records the decoded `"Hello"` string with `selection: Clipboard`. For the Effect-side assertion, verify the effect family is `Effect::Host(HostEffect::ClipboardStore { .. })` or the equivalent store-side variant named in Section 03's type (verify `oriterm_core/src/effect/families/host.rs` exact variant name — the test cites the variant by path).
- [ ] `osc52_store_clipboard_s` — same shape, `s` (selection) clipboard character, assert `selection: Selection`.
- [ ] `osc52_store_clipboard_p` — `p` (primary) clipboard character.
- [ ] `osc52_store_clipboard_q` — `q` (secondary) clipboard character, if supported; else negative pin that this character is dropped.
- [ ] `osc52_load_request_fires_hostrequest` — feed `\x1b]52;c;?\x1b\\`, assert `Effect::HostRequest(HostRequest::ClipboardLoad { selection: Clipboard, clipboard_char: b'c', terminator: "\x1b\\", reply: <ResponseToken> })` is on the transcript. This is the SPEC-CHAIN assertion scope boundary — the spec_chain harness asserts the HostRequest was emitted; it does NOT simulate the IO thread's polling loop (that lives in `oriterm_mux::PaneIoThread`, which is a separate crate layer). The ResponseToken fulfillment → PtyEffect::Write round-trip is tested in `oriterm_mux/src/pane/io_thread/response_poll/tests.rs` (listed in the Files block), NOT in the spec_chain test. No `harness.poll_pending_responses()` helper is added to `SpecHarness` — doing so would force `oriterm_test_support` to depend on `oriterm_mux`'s internal `PaneIoThread`, which violates the crate boundary (see `.claude/rules/crate-boundaries.md` §crates/oriterm_test_support).
- [ ] `response_poll_roundtrip_emits_pty_write` (**in `oriterm_mux/src/pane/io_thread/response_poll/tests.rs`**, NOT in spec_chain) — construct a `PaneIoThread` (or the minimal stub thereof that holds `pending_responses`), call `register_host_request_response(request)` with a `HostRequest::ClipboardLoad { clipboard_char: b'c', terminator: "\x1b\\", reply }`, fulfill the `ResponseToken` with `reply.fulfill("example-text".into())`, call `poll_pending_responses()`, and assert the effect sink received `Effect::Pty(PtyEffect::Write { bytes })` where `bytes == format_clipboard_reply("example-text", b'c', "\x1b\\")` (base64-encoded). Uses `format_clipboard_reply` from `oriterm_core/src/effect/families/host_request.rs` — DO NOT re-implement the reply format inline (LEAK).
- [ ] **Semantic pin** — `osc52_response_token_requires_fulfillment` — negative test: emit the load request, do NOT fulfill the token, advance the harness ten ticks. Assert NO `PtyEffect::Write` is emitted. This pins the requirement that the terminal waits for fulfillment rather than emitting an empty reply.
- [ ] `osc52_load_with_s_and_p_selections` — load with `s` and `p` characters; assert the correct `ClipboardSelection` in the `HostRequest`.
- [ ] `osc52_store_invalid_base64_dropped` — feed `\x1b]52;c;!!!invalid-base64!!!\x1b\\`, assert no `HostEffect::ClipboardStore` is emitted (store path rejects invalid base64; confirm behavior matches `oriterm_core/src/term/handler/tests/osc.rs::osc52_clipboard_load` pattern) OR assert a specific error/drop behavior — whichever the current dispatcher at `oriterm_core/src/term/handler/osc.rs::osc_clipboard_store` produces. If the current behavior is "accept garbage and store it", file `/add-bug` and document the observed behavior as the current catalog deviation.

**Catalog update:**

- [ ] OSC-52-STORE in `catalog/osc.md` → `verified` with citations for all 4 clipboard characters (c/s/p/q).
- [ ] OSC-52-LOAD in `catalog/osc.md` → `verified` with citation of the ResponseToken round-trip test.
- [ ] `catalog/shell-integration.md` row SHINT-OSC-9-NOTIFY (cross-reference) remains pointing at `osc.md::OSC-9` (handled in 10.3).

**Validation:**

- [ ] All 8 tests pass (4 spec_chain store tests + `osc52_load_request_fires_hostrequest` + `osc52_response_token_requires_fulfillment` + `osc52_load_with_s_and_p_selections` + `osc52_store_invalid_base64_dropped`).
- [ ] `response_poll_roundtrip_emits_pty_write` in `oriterm_mux/src/pane/io_thread/response_poll/tests.rs` green — exercises `PendingResponse::poll` via `PaneIoThread::register_host_request_response` + `poll_pending_responses` (the IO thread path stays in the `oriterm_mux` crate, not in spec_chain).
- [ ] `oriterm_core/tests/teseq/osc.rs::osc_clipboard` regression test unchanged and green.

---

## 10.3 OSC 9 / 99 / 777 desktop notifications

**Files:**
- `oriterm_core/tests/spec_chain/osc/notifications.rs` (new)
- Catalog updates: `catalog/osc.md` rows OSC-9, and newly-added OSC-99 + OSC-777 rows (they are `missing` today); `catalog/shell-integration.md` rows SHINT-OSC-9-NOTIFY, SHINT-OSC-777-NOTIFY

**Tests (via `SpecHarness::feed_with_mux()` — these OSCs route through the RawInterceptor, NOT the high-level processor):**

- [ ] `osc9_simple_body_fires_notification` — feed `\x1b]9;Build complete\x1b\\`, assert `Effect::Host(HostEffect::DesktopNotification { source: NotificationSource::Osc9, title: "", body: "Build complete" })`. OSC 9 has no title field (Growl-style).
- [ ] `osc99_body_fires_notification_osc99_source` — feed `\x1b]99;kitty payload\x1b\\`, assert `source: NotificationSource::Osc99`. Per the interceptor at `shell_integration/interceptor.rs:124-128`, OSC 9 and 99 share the `handle_notification_simple` code path — 10.3 pins the discriminator so a future refactor cannot collapse them.
- [ ] `osc777_notify_title_body` — feed `\x1b]777;notify;Build;completed successfully\x1b\\`, assert `source: NotificationSource::Osc777, title: "Build", body: "completed successfully"`.
- [ ] `osc777_non_notify_action_dropped` — feed `\x1b]777;BAD_ACTION;title;body\x1b\\`, assert NO notification effect is emitted (the interceptor at line 143-145 filters non-`notify` actions).
- [ ] `osc9_empty_body` — feed `\x1b]9;\x1b\\`, assert `body == ""` and notification is still emitted (matches `handle_notification_simple` which accepts empty body).
- [ ] `osc777_missing_title` — feed `\x1b]777;notify;;body-only\x1b\\`, assert `title == "", body == "body-only"`.
- [ ] **Semantic pin** — `osc9_and_osc99_use_different_sources` — feed BOTH `OSC 9 ; X ST` and `OSC 99 ; Y ST` in the same scenario. Assert the two effects have DIFFERENT `NotificationSource` variants. If someone collapses the OSC 9 / 99 detection in the interceptor, this test fails immediately.
- [ ] **Negative pin** — `osc9_via_processor_without_mux_drops` — route the same OSC 9 bytes through `SpecHarness::feed()` (no mux layer). Assert NO notification effect is emitted. This proves the mux layer is LOAD-BEARING for OSC 9; if someone accidentally adds OSC 9 to the high-level dispatcher too, this test fails (double-dispatch detection).

**Catalog update:**

- [ ] OSC-9 `catalog/osc.md` → `verified` (was `missing`). Implementation cell now cites `oriterm_mux/src/shell_integration/interceptor.rs::handle_notification_simple`.
- [ ] New rows OSC-99, OSC-777 added to `catalog/osc.md` with the same citation.
- [ ] `catalog/shell-integration.md` SHINT-OSC-9-NOTIFY and SHINT-OSC-777-NOTIFY → `verified`.

**Validation:**

- [ ] All 8 tests pass (6 behavioral + 1 semantic pin + 1 negative pin).
- [ ] `NotificationSource` enum (`oriterm_core/src/effect/families/host.rs:55-62`) remains unchanged — no new variants added in this subsection.
- [ ] **TPR checkpoint 2** — `/tpr-review` covering 10.1–10.3 + re-verification of 10.0. Catches harness / observer / ResponseToken integration issues before subsections 10.4–10.7 build on top.

---

## 10.4 OSC 133 semantic prompt + OSC 633 VS Code shell integration

**Files:**
- `oriterm_core/tests/spec_chain/osc/shell_integration.rs` (new)
- `oriterm_mux/src/shell_integration/interceptor.rs` (extend to dispatch OSC 633 sub-commands — currently NOT dispatched)
- `crates/vte/src/ansi/dispatch/osc.rs` (route OSC 633 if any part of it needs the high-level processor; otherwise leave to the raw interceptor)
- Catalog updates: `catalog/osc.md` OSC-133, OSC-633 (both currently `missing`); `catalog/shell-integration.md` SHINT-OSC-133-PROMPT, SHINT-OSC-633-VSCODE

**Tests (via `SpecHarness::feed_with_mux()` — OSC 133 + 633 are interceptor-handled):**

### OSC 133 (Final Term semantic prompt)

- [ ] `osc133_a_sets_prompt_state` — feed `\x1b]133;A\x1b\\`. Assert `term.prompt_state() == PromptState::PromptStart` AND `term.prompt_mark_pending() == true`. Matches interceptor.rs:92-94.
- [ ] `osc133_b_sets_command_state` — feed `\x1b]133;B\x1b\\`. Assert `prompt_state == CommandStart` AND `command_start_mark_pending() == true`.
- [ ] `osc133_c_sets_output_state_and_records_start_instant` — inject deterministic clock (from 10.0), feed `\x1b]133;C\x1b\\`. Assert `prompt_state == OutputStart` AND `output_start_mark_pending() == true` AND the injected clock's tick is stored via `term.set_command_start(<tick>)`.
- [ ] `osc133_d_clears_state_and_emits_command_complete` — SCOPE-CLARIFIED per scope clarification D above. After C (clock at t0) and D (clock at t0 + 1.5s), assert:
  - `term.prompt_state() == PromptState::None` (interceptor.rs:106-107 sets it).
  - `Effect::Host(HostEffect::CommandComplete { duration: Duration::from_millis(1500) })` is on the transcript (interceptor.rs:108-111).
  - `term.prompt_markers().last()` still has its A/B/C fields populated (D does NOT mutate the existing marker; it closes out the command lifecycle).
  - **NO D-field exists on `PromptMarker`** — the plan pins this by asserting `assert_matches!(term.prompt_markers().last().unwrap(), PromptMarker { prompt: _, command: Some(_), output: Some(_) })` (exhaustive match — if a future field is added, this test MUST be updated explicitly).
- [ ] `osc133_a_without_b_does_not_record_command` — feed `OSC 133;A` then new prompt (A again). Assert TWO `PromptMarker`s exist with `command = None, output = None` on each.
- [ ] `osc133_command_complete_without_c_is_noop` — feed `OSC 133;D` without a preceding C. Assert NO `HostEffect::CommandComplete` is emitted (interceptor.rs:107's `term.finish_command()` returns `None` when `command_start` is unset).
- [ ] `osc133_full_lifecycle_records_markers` — feed A, type text, B, type command, C, type output, D. Assert the `prompt_markers` vec has one marker with all three of `prompt`, `command`, `output` set to distinct absolute rows.
- [ ] **Semantic pin — CWD SSOT** — if Final Term OSC 133 parameters carry `cwd=<path>`, assert the CWD is written through `Term::set_cwd` (same function OSC 7 uses). NOT through a second CWD field. Cross-reference to scope clarification H. Currently the interceptor at `handle_osc133` does NOT parse `cwd=<path>` params; if the VS Code / Final Term spec requires it, 10.4 adds the parsing AND the SSOT assertion.

### OSC 633 (VS Code shell integration)

- [ ] Read and cite the exact source at `https://github.com/microsoft/vscode/blob/main/src/vs/workbench/contrib/terminal/browser/xterm/shellIntegrationAddon.ts` to enumerate VS Code's OSC 633 sub-commands. As of the most-recent reviewed catalog (`catalog/osc.md:56` labels OSC-633 as `missing`), the common sub-commands are: `A` (prompt start), `B` (command start), `C` (command executed), `D` (command finished), `E` (command line — the raw typed command), `P;<key>=<value>` (property setting — Cwd, IsWindows, etc.).
- [ ] Add dispatch + interceptor arms for each VS Code sub-command above. VS Code's semantic overlaps OSC 133, so the implementation wiring may reuse the OSC 133 handlers with VS Code-specific parameter parsing (in particular, `P;Cwd=<path>` should route through `Term::set_cwd` — SSOT with OSC 7).
- [ ] `osc633_a_sets_prompt_state` through `osc633_d_emits_command_complete` — matrix mirroring OSC 133 A-D tests with OSC 633's exact syntax.
- [ ] `osc633_p_cwd_sets_term_cwd` — feed `\x1b]633;P;Cwd=/home/user/project\x1b\\`. Assert `term.cwd() == Some("/home/user/project")`.
- [ ] `osc633_e_records_command_line` — VS Code's `E` sub-command carries the raw command text. Add `Term::last_command_line: Option<String>` field and expose via `term.last_command_line()`. Feed `\x1b]633;E;git status\x1b\\`, assert `term.last_command_line() == Some("git status")`. This sub-command is NOT optional for `verified` status: OSC-633 is enumerated in the section success criteria with no carve-outs. If the implementation cannot be completed in 10.4 because the VS Code source reveals additional complexity, the sub-command MUST be explicitly filed via `/add-bug` AND OSC-633 catalog status updated to `verified-with-deviation` with a catalog note naming the deviation — the catalog row MUST NOT be marked `verified` while the E sub-command is unimplemented. No silent deferral.

**Catalog update:**

- [ ] OSC-133 `catalog/osc.md` → `verified`. Implementation cell cites interceptor + handler paths.
- [ ] OSC-633 `catalog/osc.md` → `verified` (add implementation citations).
- [ ] `catalog/shell-integration.md` SHINT-OSC-133-PROMPT → `verified`; SHINT-OSC-633-VSCODE → `verified`.

**Validation:**

- [ ] OSC 133 A-D + edge cases green.
- [ ] OSC 633 sub-command matrix green.
- [ ] Injected clock removes flakiness from duration assertions.

---

## 10.5 OSC 22 cursor icon + OSC 50 cursor shape

**Files:**
- `oriterm_core/tests/spec_chain/osc/cursor.rs` (new — combines 22 + 50)
- `oriterm_core/src/term/mod.rs` (already extended in 10.0 with `mouse_cursor_icon`)
- Catalog updates: `catalog/osc.md` OSC-22, OSC-50

**Tests:**

### OSC 22 (mouse cursor icon, iTerm2)

- [ ] `osc22_pointer_sets_cursor_icon` — feed `\x1b]22;pointer\x1b\\`, assert `term.mouse_cursor_icon() == Some(CursorIcon::Pointer)`. Uses the Term field + Handler override from 10.0.
- [ ] `osc22_all_known_icons_matrix` — iterate through every known OSC 22 cursor name string. `cursor_icon 1.2.0` does NOT provide a `CursorIcon::all()` or iterator over variants (confirmed: the crate only exposes `CursorIcon::name()` and `FromStr` parsing). Instead, maintain a project-owned static slice of cursor name strings: `const OSC22_KNOWN_ICONS: &[(&str, CursorIcon)] = &[("pointer", CursorIcon::Pointer), ("crosshair", CursorIcon::Crosshair), ...]` covering the ~30 variants from the CSS Basic UI / xterm spec. Feed `OSC 22 ; <name> ST` for each entry; assert each is stored. Self-verifying completeness pin: `assert_eq!(count, OSC22_KNOWN_ICONS.len())` — the project-owned slice is the SSOT for which names are supported, and the count assertion proves every cell was visited.
- [ ] `osc22_unknown_icon_is_dropped` — feed `\x1b]22;not-a-real-cursor\x1b\\`, assert `term.mouse_cursor_icon()` is UNCHANGED (the `CursorIcon::from_str` error path in the dispatcher at `crates/vte/src/ansi/dispatch/osc.rs:184` logs and drops — no state mutation).
- [ ] `osc22_reset_behavior` — OSC 22 does not have a spec'd reset form. Document this in the catalog; pin behavior: passing an explicit "default" name (if `CursorIcon::Default` exists) restores the default.
- [ ] **Semantic pin** — `osc22_does_not_affect_text_cursor_shape` — set `term.cursor_shape()` to `Beam` via OSC 50, then fire OSC 22 with `pointer`. Assert `term.cursor_shape() == Beam` (unchanged). Cross-reference scope clarification §I / blind-spot #5 — OSC 22 (mouse icon) and OSC 50 (text shape) are different fields.

### OSC 50 (cursor shape, URxvt legacy)

- [ ] `osc50_cursor_shape_block` — feed `\x1b]50;CursorShape=0\x1b\\`, assert `term.cursor_shape() == CursorShape::Block`.
- [ ] `osc50_cursor_shape_beam` — `CursorShape=1` → `Beam`.
- [ ] `osc50_cursor_shape_underline` — `CursorShape=2` → `Underline`.
- [ ] `osc50_unknown_shape_dropped` — feed `CursorShape=9`, assert no change (dispatch arm returns `unhandled` per `dispatch/osc.rs:194-199`).
- [ ] `osc50_malformed_prefix_dropped` — feed `\x1b]50;BADTHING\x1b\\`, assert no change.

**Catalog update:**

- [ ] OSC-22 `catalog/osc.md` → `verified` (was `stub` — we added the Term field + override in 10.0, so the sequence now has observable state).
- [ ] OSC-50 `catalog/osc.md` → `verified` (was `implemented-unverified`).

**Validation:**

- [ ] OSC 22 and OSC 50 tests green and do NOT interfere with each other.
- [ ] The `mouse_cursor_icon` field is queryable via `renderable_content()` — a rendering consumer can update the OS cursor on icon change.

---

## 10.6 OSC 104 / 110 / 111 / 112 color reset

**Files:**
- `oriterm_core/tests/spec_chain/osc/color_reset.rs` (new)
- Catalog update: `catalog/osc.md` rows OSC-104, OSC-110, OSC-111, OSC-112

**Tests:**

- [ ] `osc104_zero_args_resets_all_256_palette` — pre-populate palette: set indices 0..256 to custom colors via OSC 4 at setup. Feed `\x1b]104\x1b\\`. Assert every index 0..256 matches the initial theme palette (compare against `Theme::default().palette()`).
- [ ] `osc104_specific_indices_resets_only_those` — set indices 0, 5, 10 to custom colors. Feed `\x1b]104;5;10\x1b\\`. Assert index 0 is still the custom color; indices 5 and 10 are restored to theme defaults; indices 1–4, 6–9, 11–255 are at theme defaults (no collateral damage).
- [ ] `osc104_invalid_index_dropped` — feed `\x1b]104;999;abc\x1b\\`, assert the `parse_number` failure path at `dispatch/osc.rs:231-234` routes to `unhandled` and no palette entry is mutated.
- [ ] `osc110_resets_default_foreground` — set OSC 10 to red, feed `\x1b]110\x1b\\`. Assert default fg matches theme default fg (queryable via `term.palette().foreground()` — NOT `Term::color()`, which does not exist; the Palette API is at `oriterm_core/src/color/palette/mod.rs:253`).
- [ ] `osc111_resets_default_background` — same pattern for Background; use `term.palette().background()`.
- [ ] `osc112_resets_cursor_color` — same pattern for Cursor; use `term.palette().cursor_color()`.
- [ ] `osc_reset_round_trip_with_query` — after each reset, feed OSC 10/11/12 ` ; ?` (query form) and assert the reply PtyWrite contains the theme default RGB (uses `format_color_reply` from 10.2's canonical home).
- [ ] **Semantic pin** — `osc104_reset_marks_grid_dirty` — observe damage after OSC 104 (palette change should mark all visible rows dirty per `Term::set_color` which marks grid dirty). Negative pin: if damage isn't set, rendering won't repaint the reset palette — semantic regression.

**Catalog update:**

- [ ] OSC-104, OSC-110, OSC-111, OSC-112 in `catalog/osc.md` → `verified`.

**Validation:**

- [ ] All 8 tests green.
- [ ] Color-reset round-trip confirms OSC 10/11/12 query returns the theme default after 110/111/112.

---

## 10.7 OSC 1337 non-image sub-ops (handoff from Section 14)

**Files:**
- `oriterm_core/tests/spec_chain/osc/iterm2_non_image.rs` (new)
- `oriterm_core/src/term/handler/mod.rs` (implement `Handler::iterm2_set_mark`, `iterm2_remote_host`, `iterm2_current_dir`, `iterm2_copy`, `iterm2_report_cell_size`, `iterm2_set_user_var`, `iterm2_shell_integration_version` on `Term`)
- `oriterm_core/src/term/mod.rs` (new Term fields: `remote_host: Option<String>`, `user_vars: HashMap<String, String>`, `shell_integration_version: Option<String>`)
- `plans/spec-conformance/catalog/iterm2.md` (update `owner_section` in front-matter; update per-row `Implementation` + `Verification` cells)

**Tests:**

- [ ] `osc1337_set_mark` — feed `\x1b]1337;SetMark\x1b\\`, assert `term.prompt_markers()` has a new marker with `prompt = current cursor row, command = None, output = None`. SetMark is a navigation mark equivalent to OSC 133;A's prompt boundary. Uses the same `prompt_markers` vec (SSOT).
- [ ] `osc1337_remote_host` — feed `\x1b]1337;RemoteHost=user@host.example.com\x1b\\`, assert `term.remote_host() == Some("user@host.example.com")`.
- [ ] `osc1337_current_dir` — feed `\x1b]1337;CurrentDir=/path/to/dir\x1b\\`, assert `term.cwd() == Some("/path/to/dir")`. SSOT with OSC 7 + OSC 133 (scope clarification H).
- [ ] `osc1337_copy` — feed `\x1b]1337;Copy=:SGVsbG8=\x1b\\` (the `Copy=<b64>` form), assert `Effect::Host(HostEffect::ClipboardStore { .. })` (or the equivalent store variant) with the decoded text.
- [ ] `osc1337_report_cell_size` — feed `\x1b]1337;ReportCellSize\x1b\\`, assert `Effect::Pty(PtyEffect::Write { bytes: ... })` with the expected reply format `OSC 1337 ; ReportCellSize=<H>;<W> ST` using the Term's current cell dimensions (pulled from the harness's terminal size or from a new `Term::cell_size_pixels()` if cell pixel sizes are available at the core level).
- [ ] `osc1337_set_user_var` — feed `\x1b]1337;SetUserVar=MY_VAR=SGVsbG8=\x1b\\`, assert `term.user_var("MY_VAR") == Some("Hello")`.
- [ ] `osc1337_shell_integration_version` — feed `\x1b]1337;ShellIntegrationVersion=5\x1b\\`, assert `term.shell_integration_version() == Some("5")`.
- [ ] `osc1337_file_still_routes_to_iterm2_file` — feed a minimal `\x1b]1337;File=name=test.png;:<tiny-png-bytes>\x1b\\`, assert `Handler::iterm2_file` is still called (regression guard: the sub-dispatcher refactor from 10.0 must preserve Section 14's image path).
- [ ] `osc1337_unknown_key_dropped` — feed `\x1b]1337;NotARealKey=value\x1b\\`, assert no state mutation and the `unhandled` branch fires.
- [ ] **Semantic pin — SSOT for CWD (direction A)** — set `term.cwd()` via OSC 7 (`file:///start ST`). Feed `OSC 1337 ; CurrentDir=/other-path ST`. Assert `term.cwd() == Some("/other-path")` (last write wins; NO second CWD field). Cross-reference scope clarification §H.
- [ ] **Semantic pin — SSOT for CWD (direction B)** — set `term.cwd()` via OSC 1337 CurrentDir first (`/from-iterm2`). Then feed `OSC 7 ; file:///from-osc7 ST`. Assert `term.cwd() == Some("/from-osc7")` (OSC 7 overwrites OSC 1337 via the same canonical `Term::set_cwd` field). Matrix clamping requires BOTH directions per `.claude/rules/tests.md §Matrix Clamping` — a one-directional test misses a future regression where OSC 1337 writes a second CWD field that OSC 7 does not overwrite.

**Catalog update:**

- [ ] `catalog/iterm2.md` front-matter `owner_section` → `"01 (bootstrap), 10 (non-image), 14 (image)"`.
- [ ] Rows ITERM2-1337-REMOTEHOST, ITERM2-1337-CURRENTDIR, ITERM2-1337-COPY, ITERM2-1337-SETMARK, ITERM2-1337-REPORTCELLSIZE, ITERM2-1337-SETUSERVAR → `verified` with implementation citation to `oriterm_core/src/term/handler/mod.rs::iterm2_*`.
- [ ] ITERM2-1337-FILE stays at `implemented-unverified` (Section 14 owns its verification). Add a catalog Notes entry cross-linking Section 10's ownership of the non-image variants.
- [ ] New catalog row ITERM2-1337-SHELLINTVERSION added if missing.

**Plan sync to Section 14 (flow-up edit beyond single-section authority — recorded for next /review-plan):**

- `section-14-iterm2-images.md:55` currently says "Section 10's OSC suite covered the non-image OSC 1337 variants". This statement is now accurate with Section 10 owning the non-image variants — but `catalog/iterm2.md:14` said those variants are assigned to Section 14. This review's flow-up edit aligns the catalog (above) and the `catalog/iterm2.md` front-matter `owner_section`. Section 14's next /review-plan will pick up the consistent state and update `section-14-iterm2-images.md:55` to cite the new catalog ownership wording if needed.

**Validation:**

- [ ] All 10 tests green.
- [ ] OSC 1337 `File=` path unchanged; Section 14 can build on top without touching the sub-dispatcher.
- [ ] **TPR checkpoint 3** — `/tpr-review` covering 10.4–10.7 + ownership cross-check against Section 14.

---

## 10.8 Basic OSC rows (0/1/2/4/7/10/11/12/52) — inherited from Section 08

**Files:**
- `oriterm_core/tests/spec_chain/osc/basic.rs` (new — covers OSC 0/1/2)
- `oriterm_core/tests/spec_chain/osc/palette.rs` (new — covers OSC 4 set/query)
- `oriterm_core/tests/spec_chain/osc/cwd.rs` (new — covers OSC 7 via `feed_with_mux`)
- `oriterm_core/tests/spec_chain/osc/default_colors.rs` (new — covers OSC 10/11/12 set/query)
- Catalog updates: `catalog/osc.md` rows OSC-0, OSC-1, OSC-2, OSC-4-SET, OSC-4-QUERY, OSC-7, OSC-10-SET, OSC-10-QUERY, OSC-11-SET, OSC-11-QUERY, OSC-12-SET, OSC-12-QUERY; `catalog/shell-integration.md` row SHINT-OSC-7-CWD

**Scope pin from 08:** `section-08-ecma-48-baseline.md:179` recorded zero OSC coverage from tack; all rows below start at `implemented-unverified` / `stub` and end `verified` here.

**Tests (via `SpecHarness::feed()` for OSC 0/1/2/4/10/11/12 — routed through high-level processor — and `feed_with_mux()` for OSC 7):**

### OSC 0 / 1 / 2 (title + icon name)

- [ ] `osc0_sets_title_and_icon` — feed `\x1b]0;myapp\x1b\\`, assert `term.title() == "myapp"` AND `term.icon_name() == "myapp"` (OSC 0 sets both).
- [ ] `osc1_sets_only_icon_name` — feed `\x1b]1;myicon\x1b\\`, assert `icon_name == "myicon"` AND `title` is UNCHANGED (starts empty).
- [ ] `osc2_sets_only_title` — feed `\x1b]2;mytitle\x1b\\`, assert `title == "mytitle"` AND `icon_name` is UNCHANGED.
- [ ] `osc0_empty_sets_empty_string` — feed `\x1b]0;\x1b\\`, assert both title and icon_name become the empty string `""`. **Important dispatch accuracy:** the `osc.rs` dispatcher's `b"0"` arm ALWAYS calls `handler.set_title(Some(text.clone()))` — it sends `Some("")` not `None` when the param is empty. There is NO `ResetTitle` path triggered by `OSC 0 ; ST`; `Event::ResetTitle` (now `HostEffect::TitleSet { value: None }`) is only emitted by other mechanisms (e.g. explicit reset via ESC c or the `TITLE_STACK_MAX_DEPTH` eviction path). Test assertions MUST reflect this: assert `term.title() == ""` not `term.title() == <original>`.
- [ ] `osc0_bel_and_st_terminators_both_accepted` — feed `\x1b]0;t1\x07` (BEL) AND `\x1b]0;t2\x1b\\` (ST) in sequence. Assert both update the title; the dispatcher's `bell_terminated` parameter routes correctly.
- [ ] `osc0_title_stack_via_csi_t` — xterm push/pop title uses **CSI 22;2t** (push) and **CSI 23;2t** (pop), NOT OSC. These are xterm window operations dispatched from `crates/vte/src/ansi/dispatch/csi.rs`, not from `osc.rs`. Test `ESC[22;2t` → push + `ESC[23;2t` → pop using the CSI rung, and assert the title stack is bounded at `TITLE_STACK_MAX_DEPTH` (4096 per `oriterm_core/src/term/mod.rs:82`). This test belongs to the CSI window operations section, not the OSC matrix — move it to the appropriate section or cite it as a cross-reference here without duplicating ownership.

### OSC 4 (palette index)

- [ ] `osc4_set_palette_index` — feed `\x1b]4;5;rgb:ff/00/00\x1b\\`, assert `term.palette().color(5) == Rgb(0xff, 0, 0)` (`Palette::color(index)` at `oriterm_core/src/color/palette/mod.rs:282`).
- [ ] `osc4_query_palette_index` — feed `\x1b]4;5;?\x1b\\`, assert a `PtyEffect::Write` with the reply `OSC 4 ; 5 ; rgb:ffff/0000/0000 ST` (double-nibble per xterm).
- [ ] `osc4_multi_param_sets_multiple_indices` — feed `\x1b]4;1;rgb:00/ff/00;2;rgb:00/00/ff\x1b\\`, assert indices 1 and 2 are both set.
- [ ] `osc4_out_of_range_dropped` — feed `\x1b]4;999;rgb:ff/ff/ff\x1b\\`, assert no mutation.
- [ ] `osc4_invalid_color_dropped` — feed `\x1b]4;5;NOT_A_COLOR\x1b\\`, assert index 5 unchanged.

### OSC 7 (CWD — INTERCEPTOR path)

- [ ] `osc7_file_uri_sets_cwd` — feed `\x1b]7;file:///home/user/project\x1b\\` via `feed_with_mux`. Assert `term.cwd() == Some("/home/user/project")`. Uses the parse_osc7_path logic in `interceptor.rs:173-187`.
- [ ] `osc7_file_uri_with_hostname` — feed `file://myhost.example.com/path/to/dir`, assert cwd is `/path/to/dir` (hostname stripped per interceptor.rs).
- [ ] `osc7_percent_decoded` — feed `file:///home/user/my%20folder`, assert cwd is `/home/user/my folder` (percent_decode in interceptor.rs:199-220).
- [ ] `osc7_emits_host_effect_cwd_set` — assert `Effect::Host(HostEffect::CwdSet { cwd: "/home/user/project" })` on the transcript.
- [ ] `osc7_relative_path_passed_through` — feed `\x1b]7;relative/path\x1b\\`. Per `strip_uri_suffix`, this passes through unchanged. Assert `cwd == Some("relative/path")`. Verify this matches production behavior; if the interceptor rejects non-URI paths in production, update the test accordingly.
- [ ] `osc7_via_high_level_processor_drops` — negative pin. Feed the same OSC 7 bytes via `feed()` (no mux). Assert cwd is UNCHANGED. This pins the interceptor-only path.
- [ ] **OSC 7 double-dispatch remediation (LEAK:duplicated-dispatch):** The `b"7"` arm in `crates/vte/src/ansi/dispatch/osc.rs:69-87` calls `handler.set_working_directory()` which is a no-op default on `Term` (confirmed: `Term` does not override this method). The interceptor at `oriterm_mux/src/shell_integration/interceptor.rs:37` handles OSC 7 canonically with full URI parsing. The high-level `b"7"` arm is therefore vestigial — it calls a no-op and provides no value. The interceptor comment (`interceptor.rs:6-8`) acknowledges this: "OSC 7 is also handled here instead of through the high-level `Handler::set_working_directory`, which stores the raw URI." Section 10.8 MUST remove the `b"7"` arm from `osc.rs` OR add a `// SSOT: CWD is handled by RawInterceptor; this arm is intentionally empty for parity` comment WITH an `assert!(!reachable)` / `debug_assert` semantic — leaving it silently calling a no-op creates a second apparent dispatch path that confuses future readers and could be mistakenly "fixed" to re-implement CWD logic in the wrong layer. Preferred fix: remove the arm and handle the `set_working_directory` default body more explicitly.

### OSC 10 / 11 / 12 (default colors)

- [ ] `osc10_sets_default_foreground` — feed `\x1b]10;rgb:de/ad/be\x1b\\`, assert `term.palette().foreground() == Rgb(0xde, 0xad, 0xbe)` (use `Palette::foreground()` at `oriterm_core/src/color/palette/mod.rs:253` — `Term::color()` does not exist; the method is on `Palette`, not `Term`).
- [ ] `osc11_sets_default_background` — OSC 11.
- [ ] `osc12_sets_cursor_color` — OSC 12.
- [ ] `osc10_query_replies_rgb` — feed `\x1b]10;?\x1b\\`, assert PtyEffect::Write with `OSC 10 ; rgb:dede/adad/bebe ST`.
- [ ] `osc10_multi_param_walks_named_colors` — per the osc.md Notes column, the multi-param form walks `NamedColor::Foreground..Cursor`. Feed `\x1b]10;rgb:10/10/10;rgb:20/20/20;rgb:30/30/30\x1b\\`, assert Foreground == `#101010`, Background == `#202020`, Cursor == `#303030`.

### OSC 52 already covered in 10.2

**Catalog updates:**

- [ ] Every row named at the top of 10.8 is promoted from `implemented-unverified` / `stub` to `verified`.
- [ ] `catalog/shell-integration.md` SHINT-OSC-7-CWD → `verified` (was `stub`; now the interceptor actually writes CWD).

**Validation:**

- [ ] All ~25 tests green across the 5 files.
- [ ] Existing teseq tests (`osc_title.teseq`, `osc_icon_name.teseq`, `osc_color_query.teseq`, `osc_clipboard.teseq`) still green — these stay as regression guards.

---

## 10.9 OSC rows currently `missing` — dispatch + handler + verification

**Files:**
- `crates/vte/src/ansi/dispatch/osc.rs` (add dispatch arms for OSC 3, 5, 6, 13, 14, 17, 19, 113, 114, 117, 119, L, l)
- `crates/vte/src/ansi/handler.rs` (add default Handler trait methods)
- `oriterm_core/src/term/handler/osc.rs` (override on Term for state-mutating variants)
- `oriterm_core/tests/spec_chain/osc/missing_rows.rs` (new — verifies each added variant)
- Catalog updates: `catalog/osc.md` rows OSC-3, OSC-5, OSC-6, OSC-13-SET/QUERY, OSC-14-SET/QUERY, OSC-17-SET/QUERY, OSC-19-SET/QUERY, OSC-113, OSC-114, OSC-117, OSC-119, OSC-L, OSC-l

**Per-row analysis (per `catalog/osc.md:37-56`):**

- **OSC 3** (set X11 window property) — platform-specific (X11 only). Add dispatch arm that routes to `Handler::set_x11_property(prop: &[u8], value: &[u8])`. Term's default implementation drops on non-X11; on Linux + X11 runtime it may emit `HostEffect::SetX11Property`. For 10.9 verification: test that the dispatch arm fires and on non-X11 platforms the dispatch returns without side effects. Catalog status → `verified-with-deviation` with a note that the effect is platform-conditional.
- **OSC 5** (change/query special color: highlight/bold) — add dispatch + handler; test set + query round-trip. `verified`.
- **OSC 6** (title tab color, iTerm2) — add dispatch + handler; new `Term::tab_title_color` field; test set + query. `verified`.
- **OSC 13 / 113** (mouse fg color set/reset) — add dispatch + handler pair. Test set via `OSC 13 ; rgb:... ST`, query via `OSC 13 ; ? ST`, reset via `OSC 113 ST`. `verified`.
- **OSC 14 / 114** (mouse bg color, Tektronix) — same pattern as 13/113.
- **OSC 17 / 117** (highlight bg color, selection bg) — same pattern; integrates with existing selection rendering state.
- **OSC 19 / 119** (highlight fg color) — same pattern.
- **OSC L** / **OSC l** (Sun console aliases for OSC 1 / 2) — add dispatch arms that alias to `Term::set_icon_name` (L) and `Term::set_title` (l). Test the aliasing. `verified`.

**Tests:**

- [ ] One test per variant — ~22 tests total (set + query + reset pairs).
- [ ] Cross-reset consistency: set OSC 13, reset via OSC 113, verify it returns to default.
- [ ] **Matrix completeness pin (SSOT: use catalog_row_id scanner, NOT function-name grep)** — use the existing `scan_test_citations` / `CoverageReport` infrastructure at `crates/oriterm_test_support/src/spec_chain/coverage/` (not a raw grep for function names). Every `SpecScenario` const in the OSC test files MUST declare `catalog_row_id: "OSC-<N>"` matching the corresponding catalog row ID. Run the coverage report (`cargo run --bin spec_coverage_report`) and assert every OSC catalog row has at least one citation. Function-name grepping (`osc<N>`) would bypass this SSOT and create a second catalog-tracking mechanism that can drift from the canonical scanner.

**Catalog updates:**

- [ ] Every `missing` row named above → `verified` or `verified-with-deviation`.

---

## 10.R Third Party Review Findings

- [x] `[TPR-10-1-codex][high]` `plans/spec-conformance/section-10-osc-suite.md:190` — Harness mux_layer implementation instruction had wrong interceptor/processor ordering.
  Evidence: plan said "after the high-level call, run the raw parser" but production order is interceptor FIRST.
  Impact: Implementing as written would produce a harness that runs the interceptor in the wrong order vs production.
  Required plan update: rewritten at 10.0 implementation bullet to run interceptor before processor (FIXED).
  Basis: fresh_verification | direct_file_inspection. Confidence: high.

- [x] `[TPR-10-2-codex][high]` `oriterm_mux/src/pane/io_thread/response_poll.rs:33` — Response-poll activation cannot proceed without EffectSink migration.
  Evidence: IO thread uses `LegacyEventSink` whose `drain_into()` is no-op; `register_host_request_response` cannot be wired until QueueingEffectSink migration.
  Impact: Plan's 10.2 test for full ResponseToken round-trip via PaneIoThread would fail silently with LegacyEventSink.
  Required plan update: 10.0/10.2 now document the dependency on effect-cutover plan and provide Option A/B approaches (FIXED).
  Basis: fresh_verification | direct_file_inspection. Confidence: high.

- [x] `[TPR-10-3-codex][medium]` `oriterm_core/src/term/renderable/mod.rs:46` — OSC 8 hyperlink `id` not exposed on RenderableCell; osc8_with_id test must use state rung.
  Evidence: `RenderableCell` only has `hyperlink_uri: Option<String>`, no `id` field; `Cell::hyperlink()` carries `id: Option<String>`.
  Impact: Test assertions about hyperlink id "in cell metadata" via renderable observer would be silently incomplete.
  Required plan update: osc8_with_id rewritten to use state rung for id check, renderable rung for URI (FIXED).
  Basis: fresh_verification | direct_file_inspection. Confidence: high.

- [x] `[TPR-10-4-codex][medium]` `oriterm_core/src/term/handler/tests/osc.rs:60` — Title push/pop is CSI 22;2t / CSI 23;2t, not OSC.
  Evidence: `feed(&mut t, b"\x1b[22;2t")` — push_title dispatched from CSI, not from OSC dispatcher.
  Impact: osc0_push_pop_title in the OSC 0/1/2 matrix would be in the wrong test file and wrong subsection.
  Required plan update: `osc0_push_pop_title` renamed to `osc0_title_stack_via_csi_t`, correctly attributed to CSI window ops rung (FIXED).
  Basis: fresh_verification | direct_file_inspection. Confidence: high.

- [x] `[TPR-10-5-codex][medium]` `crates/oriterm_test_support/src/spec_chain/scenario.rs:15` — 10.9 completeness pin bypasses catalog_row_id SSOT with function-name grep.
  Evidence: `pub catalog_row_id: &'static str` — SpecScenario carries catalog_row_id; scan_test_citations reads it canonically.
  Impact: Function-name grep would create a second test-tracking mechanism that can drift from the coverage scanner.
  Required plan update: completeness pin rewritten to use `scan_test_citations` / `CoverageReport` infrastructure (FIXED).
  Basis: fresh_verification | direct_file_inspection. Confidence: high.

- [x] `[TPR-10-6-codex][medium]` `crates/vte/src/ansi/dispatch/osc.rs:53` — osc0_empty_resets expects ResetTitle but dispatcher sends Some("") not None.
  Evidence: `handler.set_title(Some(text.clone()))` — always wraps in Some(); empty param → `Some("")`, not `None`.
  Impact: Test assertion `Event::ResetTitle` is emitted would fail; empty title sets `term.title() == ""`.
  Required plan update: test renamed `osc0_empty_sets_empty_string` with correct assertion (FIXED).
  Basis: fresh_verification | direct_file_inspection. Confidence: high.

- [x] `[TPR-10-7-gemini][high]` `crates/vte/src/ansi/dispatch/osc.rs:69` — OSC 7 double-dispatch: high-level arm calls no-op default; interceptor does real work.
  Evidence: `b"7" => { ... handler.set_working_directory(Some(uri)); }` calls a no-op default; Term does not override.
  Impact: Vestigial arm creates a false second dispatch path; future implementors could mistakenly add CWD logic to the wrong layer.
  Required plan update: 10.8 now includes a task to remove the `b"7"` arm from osc.rs or add an explicit SSOT comment (FIXED).
  Basis: fresh_verification | direct_file_inspection. Confidence: high.

<!-- Round 2 findings (2026-04-17) -->

- [x] `[TPR-10-8-codex][high]` `plans/spec-conformance/section-10-osc-suite.md:279` — OSC 52 spec-chain test plans `harness.poll_pending_responses()` delegating to `PaneIoThread`, which would force `oriterm_test_support` to depend on `oriterm_mux` internals — a crate boundary violation.
  Evidence: `harness.poll_pending_responses()` delegating to `PaneIoThread::poll_pending_responses` — `SpecHarness` wraps `Term<QueueingEffectSink>` and contains no `PaneIoThread`.
  Impact: Adding this helper would add a `oriterm_mux` dependency to `oriterm_test_support`, violating `.claude/rules/crate-boundaries.md §crates/oriterm_test_support`.
  Required plan update: spec_chain scope boundary clarified — `osc52_load_request_fires_hostrequest` asserts HostRequest emission only; `response_poll_roundtrip_emits_pty_write` moved to `oriterm_mux` IO thread tests where `PaneIoThread` is in scope (FIXED).
  Basis: direct_file_inspection. Confidence: high.

- [x] `[TPR-10-9-codex][medium]` `plans/spec-conformance/section-10-osc-suite.md:424` — Plan cites `Term::color()` for `NamedColor::Foreground` which does not exist; the real API is on `Palette`, not `Term`.
  Evidence: `Term::color()` does not exist on `oriterm_core::Term`; method is `term.palette().color(index)`, `term.palette().foreground()`, `term.palette().background()`, `term.palette().cursor_color()` (at `oriterm_core/src/color/palette/mod.rs:253,258,274,282`).
  Impact: An implementer following the plan would write `term.color(NamedColor::Foreground)` which fails to compile.
  Required plan update: all `term.color(...)` references replaced with `term.palette().color(index)` / `.foreground()` / `.background()` / `.cursor_color()` (FIXED).
  Basis: direct_file_inspection. Confidence: high.

- [x] `[TPR-10-10-codex][medium]` `plans/spec-conformance/section-10-osc-suite.md:360` — OSC 633 E sub-command has a deferral escape hatch that contradicts the `verified` catalog status requirement.
  Evidence: "If this is beyond scope for 10.4, defer the E sub-command to Section 22 — but do NOT leave it silently unhandled" vs catalog update "OSC-633 → `verified`".
  Impact: Deferring E without downgrading the catalog status would result in a falsely `verified` row with incomplete sub-command coverage.
  Required plan update: escape hatch removed; plan now requires E to be implemented or explicitly filed via `/add-bug` AND OSC-633 downgraded to `verified-with-deviation` if E is deferred (FIXED).
  Basis: direct_file_inspection. Confidence: high.

- [x] `[TPR-10-11-codex][low]` `plans/spec-conformance/section-10-osc-suite.md:388` — `CursorIcon::all()` does not exist in cursor-icon 1.2.0; completeness pin would fail to compile.
  Evidence: cursor-icon 1.2.0 exposes only `CursorIcon::name()` and `FromStr`; no `all()` or variant iterator.
  Impact: The completeness assertion `assert_eq!(count, CursorIcon::all().count())` does not compile.
  Required plan update: replaced with a project-owned `OSC22_KNOWN_ICONS` slice as the SSOT; count pin asserts against `OSC22_KNOWN_ICONS.len()` (FIXED).
  Basis: direct_file_inspection of cursor-icon 1.2.0 source. Confidence: high.

- [x] `[TPR-10-12-gemini][low]` `oriterm_mux/src/shell_integration/interceptor.rs:7` — Interceptor module doc comment incorrectly states that the high-level `Handler::set_working_directory` "stores the raw URI"; the `Term` default implementation is a no-op.
  Evidence: `fn set_working_directory(&mut self, _: Option<String>) {}` (handler.rs:28) — empty default; `Term` does not override it.
  Impact: Future readers may believe the high-level handler stores CWD data, obscuring the interceptor's role as the sole canonical CWD path.
  Required plan update: comment corrected in `interceptor.rs` to accurately state `Term` does not override the handler default and the interceptor is the sole canonical path (FIXED in source file per §7 fix policy).
  Basis: direct_file_inspection. Confidence: high.

- [x] `[TPR-10-13-gemini][medium]` `plans/spec-conformance/section-10-osc-suite.md:460` — CWD SSOT semantic pin only tests OSC 7 → OSC 1337 direction; missing symmetrical test (OSC 1337 → OSC 7 overwrite).
  Evidence: Plan pin: "set term.cwd() via OSC 7. Feed OSC 1337 ; CurrentDir=<different-path>. Assert new path." Reverse direction absent.
  Impact: A future regression where OSC 1337 writes a separate CWD field not overwritten by OSC 7 would go undetected.
  Required plan update: symmetrical direction B test added (OSC 1337 first, then OSC 7 overwrites) per `.claude/rules/tests.md §Matrix Clamping` (FIXED).
  Basis: direct_file_inspection. Confidence: high.

---

## 10.N Completion Checklist

### TDD Discipline (MUST be FIRST — per `.claude/rules/tests.md` §TDD for Bugs)

- [ ] **Failing test matrix written FIRST** — all of 10.0's harness/observer/state TDD tests are written and VERIFIED RED before any implementation lands. Then 10.1–10.9's test matrices are written and VERIFIED RED in subsection order. Skipping this invalidates the TDD contract and the section.
- [ ] **Ordering gate:** 10.0 completes before any of 10.1–10.9 starts. 10.0's TPR checkpoint 1 MUST pass before the downstream subsections are written.

### Crate ordering (per `.claude/rules/crate-boundaries.md` allowed dependency direction)

- [ ] Changes land in this order: `crates/vte` (new Handler trait methods + OSC 1337 sub-dispatcher) → `oriterm_core` (Term fields: `mouse_cursor_icon`, `remote_host`, `user_vars`, `shell_integration_version`; handler overrides) → `oriterm_mux` (remove `#[allow(dead_code)]` on `register_host_request_response`; wire live call site; interceptor extensions for OSC 633 + 1337 delegated rows where applicable) → `crates/oriterm_test_support` (SpecHarness `mux_layer` + completed renderable observer + new RenderableExpectation fields) → tests under `oriterm_core/tests/spec_chain/osc/*`.

### Matrix coverage

- [ ] **Matrix dimensions**: OSC number × sub-command / parameter × apex layer (parser, dispatch, state, effect, renderable) × routing layer (high-level processor OR raw interceptor).
- [ ] **Semantic pins**:
  - [ ] OSC 8 hyperlink cell metadata survives reflow + scroll + alt-screen (10.1).
  - [ ] OSC 52 ResponseToken round-trip emits PtyWrite (10.2).
  - [ ] OSC 9 / 99 use distinct NotificationSource (10.3).
  - [ ] OSC 133;D does NOT write a D-field on PromptMarker (10.4) — exhaustive match pins the data model.
  - [ ] OSC 22 and OSC 50 use distinct Term fields (10.5).
  - [ ] OSC 104 reset marks grid dirty (10.6).
  - [ ] OSC 1337 CurrentDir = OSC 7 = OSC 133 CWD = Term::cwd (10.7 SSOT pin).
  - [ ] Renderable observer is not a stub (10.1 semantic pin).
- [ ] **Negative pins**:
  - [ ] OSC 8 terminator cancels cell attachment (10.1).
  - [ ] OSC 9 via high-level processor does NOT fire notification — mux-only (10.3).
  - [ ] OSC 52 load without fulfillment does NOT emit reply (10.2).
  - [ ] OSC 22 unknown icon does not mutate state (10.5).
  - [ ] OSC 50 unknown shape does not mutate state (10.5).
- [ ] **Cross-pattern matrix**: every OSC that has SET and QUERY forms has both tested in the same subsection; every OSC that has SET and RESET forms has both tested.

### Rules weaving (per `.claude/rules/impl-hygiene.md` + `.claude/rules/code-hygiene.md` + `.claude/rules/crate-boundaries.md` + `.claude/rules/oriterm_core.md` + `.claude/rules/oriterm_mux.md`)

- [ ] **No SSOT drift**: `Term::cwd` is the ONLY CWD field — OSC 7, OSC 133, and OSC 1337 CurrentDir all route through it (10.4, 10.7, 10.8). Verified by `grep -rn 'cwd:' oriterm_core/src/term/` — returns the single canonical location.
- [ ] **No registration sync drift**: new `NotificationSource` variants (none added in this section — pinned in 10.3) AND new `Handler` trait methods (iterm2_* in 10.0 / 10.7 + x11_property in 10.9) are checked for sync across all consumers — `grep -rn 'fn iterm2_'` in `crates/vte` + `oriterm_core` returns matching pairs per method.
- [ ] **No LEAK**: reply formatting for OSC 52 + OSC 4/10/11/12 queries goes through `format_clipboard_reply` / `format_color_reply` at `oriterm_core/src/effect/families/host_request.rs:110,126` — the canonical home; NO ad-hoc `format!` at dispatch or handler sites.
- [ ] **No file size violations**: per `.claude/rules/code-hygiene.md` §File Size, source files (non-`tests.rs`) stay under 500 lines. `crates/vte/src/ansi/dispatch/osc.rs` is currently under the limit; the OSC 1337 sub-dispatcher extraction (10.0) and the new OSC 3/5/6/13/14/17/19/113/114/117/119/L/l arms (10.9) MUST NOT push it over. If approaching the limit, split by OSC family (e.g., `dispatch/osc/color.rs`, `dispatch/osc/notifications.rs`, `dispatch/osc/shell_integration.rs`) per the existing `dispatch/` pattern.
- [ ] **Cross-platform**: OSC 3 (X11 property) has `#[cfg]` branches for Linux-X11 vs macOS vs Windows per `.claude/rules/tests.md` §Cross-Platform Verification. Every branch has a counterpart; Windows cross-compile via `cargo build --target x86_64-pc-windows-gnu` green.
- [ ] **Alloc regression unchanged** — OSC 10/11/12 query reply formatting is not on the hot render path (per `.claude/rules/oriterm_core.md` §Performance Invariants the hot path is `renderable_content_into()` and snapshot flip); but OSC reply formatting still must not leak allocations per frame. `oriterm_core/tests/alloc_regression.rs` green.
- [ ] **RSS regression** — OSC 52 store + OSC 1337 SetUserVar accumulate state (clipboard history, user vars). Bound the growth: `user_vars: HashMap<String, String>` has a configurable max-size cap (default 256 entries, eviction LRU); clipboard-store state is owned by the consumer, not by Term. `oriterm_core/tests/rss_regression.rs` green.

### Catalog + cross-section updates

- [ ] Every row promoted per the success criteria list has a `Test chain` citation pointing at the specific `#[test]` function by path + name.
- [ ] `catalog/iterm2.md` front-matter `owner_section` updated (10.7).
- [ ] `catalog/osc.md` ownership notes updated where Section 08 and Section 10 split responsibility — clarify Section 10 owns ALL OSC rows (per scope clarification A).
- [ ] `plans/spec-conformance/section-14-iterm2-images.md:55` wording is consistent with the catalog update — flow-up review gate. If `reviewed: true` is on section 14, it MUST be flipped to `false` by the `/review-plan verify` step because this section changed a shared catalog row. The `/review-plan verify` step handles this automatically via the reviewed-gate machinery; the plan is NOT to manually edit `section-14.md` here.

### Existing test suites (regression gates)

- [ ] All existing teseq OSC tests pass (`timeout 150 cargo test -p oriterm_core --test teseq osc::`) — confirms the OSC 0/1/2/4/10/11/12/52 basics still dispatch correctly through their existing paths.
- [ ] All existing tack tests pass (`timeout 150 cargo test -p oriterm_core --test tack`).
- [ ] Alloc regression unchanged (`timeout 150 cargo test -p oriterm_core --test alloc_regression`).
- [ ] RSS regression unchanged (`timeout 150 cargo test -p oriterm_core --test rss_regression`).

### Final verification

- [ ] `./build-all.sh` green (debug + release + Windows cross-compile from WSL — `cargo build --target x86_64-pc-windows-gnu` per `.claude/rules/tests.md` §Cross-Platform Verification).
- [ ] `./test-all.sh` green (debug workspace test sweep).
- [ ] Explicit release-mode test run: `timeout 150 cargo test --workspace --features oriterm/gpu-tests --release` green — required because `./test-all.sh` only covers debug; release-mode alloc regressions and `#[cfg(debug_assertions)]` divergence are invisible to it.
- [ ] `./clippy-all.sh` green — no new warnings under `deny(clippy::all)` + nursery.
- [ ] Section frontmatter `status` → `complete`; each sub-entry (`10.0` through `10.9` + `10.R` + `10.N`) → `complete`.
- [ ] `00-overview.md` Quick Reference + mission success criterion **"Verification chain complete per row"** incremented (checkboxes for promoted rows); the **"Effect/State separation enforced"** criterion gets a note that 10.2 activated the dormant response-poll arm.
- [ ] `00-overview.md` Section Dependency Graph cross-references updated if any new cross-section interaction was discovered (e.g. Section 22 real-app harness benefits from OSC 633 being `verified`).
- [ ] `index.md` section 10 status updated from "Not Started" to "Complete"; quick-ref lines updated with the final tests.
- [ ] Cross-links added to Section 14: when Section 14 is next picked up for /continue-roadmap, the overview should note that Section 10's 10.7 landed the sub-dispatcher and the non-image OSC 1337 rows.
- [ ] `/tpr-review` final (full-section) passed — dual-source codex + gemini, all findings resolved.
- [ ] `/impl-hygiene-review last commit` passed (after `/tpr-review` is clean).

**Exit Criteria:** Every OSC catalog row in `catalog/osc.md` is `verified` or `verified-with-deviation`. Every row in `catalog/shell-integration.md` is `verified`. The non-image rows of `catalog/iterm2.md` are `verified` and their ownership is cleanly assigned to Section 10. The spec_chain harness routes OSC 7/9/99/133/633/777 through the real production interceptor path. OSC 52 ResponseToken round-trip runs end-to-end through the activated `response_poll` path. OSC 22 has real Term state, not a no-op stub. OSC 133;D's behavior is documented and pinned against the actual `PromptMarker` data model. The OSC suite is conformance-complete and Section 14 can pick up the OSC 1337 sub-dispatcher without refactoring it.
