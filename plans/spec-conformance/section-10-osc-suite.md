---
section: "10"
title: "OSC Suite (full)"
status: not-started
reviewed: false
goal: "Drive every row in `catalog/osc.md`, `catalog/shell-integration.md`, and the non-image rows of `catalog/iterm2.md` (SetMark, RemoteHost, CurrentDir, Copy, ReportCellSize, SetUserVar) from `implemented-unverified` / `stub` / `missing` to `verified`. Section 10 owns the ENTIRE OSC stack — Section 08's post-completion audit (`section-08 Implementation notes 2026-04-14`) recorded that tack scenarios drove ZERO OSC rows. Basic OSC rows (0, 1, 2, 4, 7, 10, 11, 12, 52) stay owned by Section 10, NOT Section 08. This includes OSC 8 hyperlinks, OSC 22/50 cursor icon/shape, OSC 9/99/777 desktop notifications, OSC 104/110/111/112 color reset, OSC 133 semantic prompt, OSC 633 VS Code shell integration, and OSC 1337 non-image sub-ops. Section 10 also lands the prerequisites that make these rows testable: a spec_chain harness layer that routes through `oriterm_mux::shell_integration::RawInterceptor` (existing production path for OSC 7/9/99/133/777; OSC 633 dispatch is added by subsection 10.4), a completed renderable observer (OSC 8 cell-metadata assertions), a Term-level mouse-cursor-icon state (OSC 22), an extensible OSC 1337 sub-dispatcher (handed off to Section 14 for images), and the activation of the dormant `PendingResponse` polling path (OSC 52 ResponseToken round-trip)."
success_criteria:
  - "Every row in `catalog/osc.md` is `verified` or `verified-with-deviation` (no `implemented-unverified`, no `stub`, no `missing`) — this includes the basic subset 08 left unverified (OSC 0/1/2/4/7/10/11/12/52) and the advanced subset (OSC 8/22/50/104/110/111/112/9/99/777/133/633 and the non-image OSC 1337 sub-ops)"
  - "Every row in `catalog/shell-integration.md` is `verified` (OSC-7-CWD, OSC-133 A/B/C/D, OSC-633 VS Code, OSC-1337-RemoteHost / CurrentDir / SetMark / SetUserVar / ReportCellSize shell-integration cross-refs, OSC-9/777 notification cross-refs)"
  - "The non-image rows of `catalog/iterm2.md` (ITERM2-1337-REMOTEHOST, ITERM2-1337-CURRENTDIR, ITERM2-1337-COPY, ITERM2-1337-SETMARK, ITERM2-1337-REPORTCELLSIZE, ITERM2-1337-SETUSERVAR) are `verified`; `owner_section` in `catalog/iterm2.md` front-matter (line 5) is updated so Section 10 owns these rows and Section 14 owns ONLY `ITERM2-1337-FILE` + image-adjacent rows — cross-checked against the ownership conflict currently at `section-14-iterm2-images.md:55` and `catalog/iterm2.md:15-20` (the non-image rows)"
  - "OSC 7 / 9 / 99 / 133 / 633 / 777 are verified against the REAL production path (`oriterm_mux/src/shell_integration/interceptor.rs`) via spec_chain unit tests that live in `oriterm_mux/src/shell_integration/tests.rs` (sibling unit-test module — has `pub(crate)` access to `RawInterceptor` per the Rust unit-test visibility rule). Tests that need `RawInterceptor` MUST NOT live in `oriterm_mux/tests/spec_chain/` — integration test crates are separate compilation units and cannot access `pub(crate)` items in the main crate. `crates/oriterm_test_support` (`SpecHarness`) stays mux-free — no `mux_layer` API is added to `SpecHarness`. High-level-processor OSC tests (OSC 0/1/2/4/8/10/11/12/22/50/52/104/110/111/112/1337 non-image) stay in `oriterm_core/tests/spec_chain/osc/`."
  - "`observe_renderable` (crates/oriterm_test_support/src/spec_chain/observers/renderable.rs) is no longer a stub — it asserts cell hyperlink URI, cursor position, cursor shape, palette entries, and damaged lines. Every OSC 8 subsection test exercises this observer with a scenario that would FAIL if the observer remained a stub (semantic pin against `RungResult::pass(rung)` stub-behavior)"
  - "OSC 8 hyperlink rows verified — cell-attached URI survives reflow, scroll into scrollback, copy (cell metadata), and alt-screen toggle; the OSC 8 terminator (empty URI) cancels the attachment on subsequent cells; `id=<id>` parameter is preserved but does not change attachment semantics (per gist:egmontkob)"
  - "OSC 52 clipboard rows verified — `c`, `s`, `p` clipboard characters (store and load); `q` is explicitly pinned as an unsupported/dropped character (no `ClipboardSelection::q` variant exists — see `oriterm_core/src/effect/families/host.rs:108-115`). `HostRequest::ClipboardLoad` apex is verified in spec_chain (harness asserts the HostRequest is emitted); the `ResponseToken` round-trip to `PtyEffect::Write` is verified separately in `oriterm_mux` IO-thread tests (`response_poll_roundtrip_emits_pty_write` and `response_poll_token_requires_fulfillment`). Section 10.2 removes the `#[allow(dead_code, reason = \"dormant during legacy phase\")]` gate on `PaneIoThread::register_host_request_response` and wires it into the IO thread."
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
  rounds_completed: 14
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

`SpecHarness` at `crates/oriterm_test_support/src/spec_chain/api.rs:82-103` wraps `Processor::advance_with_observer` (high-level VTE processor). The production-path interceptor at `oriterm_mux/src/shell_integration/interceptor.rs` runs a SEPARATE raw `vte::Parser` on the SAME bytes BEFORE the high-level processor — this is the only path that currently sees OSC 7, OSC 9, OSC 99, OSC 133, and OSC 777 (the high-level `Processor::advance_with_observer` silently drops them per the interceptor's own module doc: *"The vte::ansi::Processor does not route OSC 133, OSC 9/99/777, or XTVERSION (CSI >q) to Handler trait methods"*). OSC 633 is currently `MISSING` per `catalog/osc.md:56` — subsection **10.4** adds its dispatch arm to the interceptor.

Consequence: verifying OSC 7/9/99/133/633/777 via `SpecHarness` alone would test a dispatch path that DOES NOT RUN IN PRODUCTION. The solution (adopted in Round 5, ratified in Rounds 7 + 10 + 11) is NOT to add a `mux_layer` extension to `SpecHarness` — doing so would require `oriterm_test_support` to depend on `oriterm_mux`, violating crate boundaries. Instead, subsection **10.0** adds a `spec_chain_helper` test-only module inside `oriterm_mux/src/shell_integration/tests.rs` (sibling unit-test module) that runs `RawInterceptor` + `Processor` in production order. Because this module compiles as part of the `oriterm_mux` crate, it has full `pub(crate)` access to `RawInterceptor`. **CRITICAL**: Tests that need `RawInterceptor` MUST be placed in `oriterm_mux/src/shell_integration/tests.rs` — integration tests in `oriterm_mux/tests/` are separate compilation units with no `pub(crate)` visibility. Only tests exercising purely public `oriterm_mux` APIs may live in `oriterm_mux/tests/`. `SpecHarness` remains mux-free.

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
- `crates/vte/src/ansi/dispatch/mod.rs` (**MODULE REGISTRATION**: add `#[cfg(test)] mod tests;` at the bottom of this file — required for the `dispatch/tests.rs` TDD test created in the OSC 1337 sub-dispatcher parse pin below. Without this registration, `tests.rs` will not be compiled. Currently `dispatch/mod.rs` has NO `#[cfg(test)] mod tests;` declaration, so the file must be added to the plan explicitly)
- `crates/oriterm_test_support/src/spec_chain/recording_handler.rs` (**REGISTRATION SYNC**: for every new `Handler::iterm2_*` method added to `crates/vte/src/ansi/handler.rs`, a matching delegate arm must be added here — same pattern as the existing `iterm2_file` arm at line 317. Missing arms mean the SpecHarness silently drops the new methods and spec_chain tests cannot observe them. This file is also updated in 10.7 when Term overrides land.)
- `oriterm_mux/src/pane/io_thread/response_poll.rs` (remove the `#[allow(dead_code)]` gate; add the activation call in `PaneIoThread::drain_events` or equivalent)
- `oriterm_core/src/term/shell_state/mod.rs` (modify `finish_command` signature to accept `now: Option<Instant>` — Option A timing seam)
- `oriterm_mux/src/shell_integration/interceptor.rs` (update call sites of `finish_command` to pass `None`; this file is the caller of `Term::finish_command()`)
- `oriterm_mux/src/shell_integration/tests.rs` (extend existing sibling unit-test module — mux-intercepted OSC spec_chain tests for OSC 7, OSC 9/99/777, OSC 133/633 live here because only this file has `pub(crate)` access to `RawInterceptor`; integration tests in `oriterm_mux/tests/` are separate compilation units with NO `pub(crate)` visibility and MUST NOT contain `RawInterceptor`-using tests)

**Tests (written FIRST per `.claude/rules/tests.md` §TDD for Bugs — VERIFIED RED before implementation):**

- [ ] **Failing test matrix written FIRST** — write TWO tests in `oriterm_mux/src/shell_integration/tests.rs` (sibling unit-test file) using the mux-internal `spec_chain_helper` (NOT `SpecHarness::feed()` — `oriterm_test_support` is NOT a dev-dependency of `oriterm_mux`). Test 1: run only the high-level `Processor::advance(&mut term, osc133_a_bytes)` without the `RawInterceptor` pass, and assert `term.prompt_state() == PromptState::None` (sequence was dropped). Test 2: run both parsers in production order via `spec_chain_helper::feed_mux_and_proc(&mut term, osc133_a_bytes)` and assert `term.prompt_state() == PromptState::PromptStart` (interceptor processed it). This RED→GREEN pair is the TDD proof that the mux interceptor is load-bearing. Both tests live in the sibling unit-test module and have `pub(crate)` access to `RawInterceptor` — NO `SpecHarness`, NO `oriterm_test_support` dev-dep required. Integration test home (`oriterm_mux/tests/`) MUST NOT be used for these tests because integration test crates are separate compilation units with no `pub(crate)` visibility into `oriterm_mux`.
- [ ] **Renderable stub regression pin** — `observers/tests.rs` test that constructs a `RenderableExpectation { hyperlink_at: Some((row, col, "http://example.com")) }` against a `Term` whose cell at (row, col) has a DIFFERENT URI. With the stub, the test passes; with the completed observer, the test fails. Commit the NEGATIVE test first, then complete the observer; the test flips from pass→fail, and THEN we invert the assertion so the final committed test is the semantic pin that requires the observer to actually check.
- [ ] **Term mouse cursor icon pin** — test `term_set_mouse_cursor_icon_stores_icon` at `oriterm_core/src/term/tests.rs` that (i) starts `Term` with `mouse_cursor_icon == None`, (ii) calls `Handler::set_mouse_cursor_icon(&mut term, CursorIcon::Pointer)`, (iii) asserts `term.mouse_cursor_icon() == Some(CursorIcon::Pointer)`. Failing RED before the override is added.
- [ ] **OSC 1337 sub-dispatcher parse pin** — test in `crates/vte/src/ansi/dispatch/tests.rs` (if missing, create) that feeds `\x1b]1337;SetMark\x1b\\` and asserts `Handler::iterm2_set_mark` was called. RED before the sub-dispatcher refactor lands.
- [ ] **Response-poll activation pin** — test `response_poll_emits_pty_write_on_fulfill` in `oriterm_mux/src/pane/io_thread/response_poll/tests.rs` (if `response_poll.rs` is converted to a directory module) OR in `oriterm_mux/src/pane/io_thread/tests.rs` (if kept flat) — that pushes a `HostRequest::ClipboardLoad` through `PaneIoThread::register_host_request_response`, calls `ResponseToken::fulfill("hello")`, polls, and asserts a `PtyEffect::Write` with the base64-encoded reply appears on the sink. RED until the `#[allow(dead_code)]` gate is removed and `register_host_request_response` is called from the live path. (See Files block note on `response_poll.rs` directory conversion.)
- [ ] **Injectable clock pin** — test `command_duration_uses_injected_now` that: (i) calls `Term::set_command_start(t0)` where `t0` is a fixed `Instant`, (ii) calls `term.finish_command(Some(t0 + Duration::from_millis(1500)))`, (iii) asserts the returned `Duration == 1500ms`. **Uses Option A seam only** (`fn finish_command(&mut self, now: Option<Instant>) -> Option<Duration>`) — do NOT add an `Arc<dyn Fn>` clock field to `Term` (breaks `#[derive(Debug)]` at `oriterm_core/src/term/mod.rs:113` and adds runtime overhead). RED until `finish_command` accepts the `now` parameter. No wall-clock reliance; the test is deterministic by construction.

**Implementation:**

- [ ] Add a `spec_chain_helper` test-only module in `oriterm_mux/src/shell_integration/tests.rs` (existing sibling `#[cfg(test)] mod tests;` file) that constructs a `RawInterceptor + Term` pair and runs both parsers in production order: (1) `raw_parser.advance(&mut interceptor, bytes)`, (2) `processor.advance(&mut term, bytes)`. **CRITICAL VISIBILITY NOTE**: Do NOT call `post_parse_housekeeping(evicted_before)` from this module — `post_parse_housekeeping` is a private method on `PaneIoThread` in `oriterm_mux/src/pane/io_thread/mod.rs:337` and is NOT accessible from `shell_integration/tests.rs` (they are sibling modules, not the same module or child/parent). The test helper in `shell_integration/tests.rs` does NOT need snapshot production housekeeping because it is testing interceptor behavior (state changes on `Term`), not snapshot visibility. If prompt-mark deferred side effects need verification in a specific test, call the public `Term` methods for deferred marking directly (`term.prompt_mark_pending()`, `term.mark_prompt_row()`, etc.) rather than routing through the private IO-thread method. The sibling unit-test module has `pub(crate)` access to `RawInterceptor` because it compiles as part of the `oriterm_mux` crate (unlike integration tests in `oriterm_mux/tests/`, which are separate crates with no `pub(crate)` visibility). **CRITICAL BOUNDARY**: Do NOT place tests requiring `RawInterceptor` in `oriterm_mux/tests/spec_chain/` — integration test crates cannot access `pub(crate)` items. Only tests that exercise purely-public APIs may live in `oriterm_mux/tests/`. `crates/oriterm_test_support` (`SpecHarness`) requires NO modification — no `mux_layer`, no `feed_with_mux()`, no new dependency. The `SpecHarness` remains mux-free.
- [ ] Complete `observe_renderable` to check every field in `RenderableExpectation`:
  - `cells: Option<&'static [(usize, usize, char)]>` — cell contents at specific positions (`&'static` slice, const-constructible, preserves `Copy`).
  - `hyperlink_at: Option<(usize, usize, &'static str)>` — assert cell's hyperlink URI matches (tuple of row, col, `&'static str` — const-constructible).
  - `cursor_position: Option<(usize, usize)>` — assert cursor lives where expected.
  - `cursor_shape: Option<CursorShape>` — assert `Term::cursor_shape()` matches.
  - `palette_index: Option<(usize, Rgb)>` — assert `term.palette().color(index) == expected_rgb` (correct API: `Palette::color(index: usize) -> Rgb` at `oriterm_core/src/color/palette/mod.rs:282`; do NOT use `Term::palette()[index]` — `Palette` does not implement `Index`).
  - `mouse_cursor_icon: Option<CursorIcon>` — assert `Term::mouse_cursor_icon()` matches (WHERE: new state landed in this subsection).
  - `damaged_lines: Option<&'static [usize]>` — assert renderable content reports the expected damage set (`&'static` slice, const-constructible, preserves `Copy`).
  - **Const-constructibility constraint**: ALL fields MUST be `Copy` and `const`-constructible — use `&'static` slices and `&'static str` instead of `Vec` and `String`. This preserves the `SpecScenario` const-constructible invariant (see `scenario.rs:12` module doc: *"Every field type is `const`-constructible. Slices use `&'static [u16]` / `&'static [u8]`. Expectation constructors are `const fn`."*). `Vec`/`String` fields ARE NOT permitted on `RenderableExpectation`.
- [ ] Extend `RenderableExpectation` in `scenario.rs` with the fields above; keep existing callers compatible by making fields `Option` with `#[derive(Default)]`. Retain `#[derive(Copy, Clone, Debug, Default)]` — the new fields must all be `Copy`.
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
- [ ] **Response-poll activation requires EffectSink migration (GAP):** `PaneIoThread::register_host_request_response` is gated with `#[allow(dead_code)]` because the IO thread currently uses `LegacyEventSink` whose `drain_into()` is a no-op — effects are forwarded immediately as legacy `Event`s. The `response_poll.rs` module doc explicitly states: "activates when consumers migrate to `QueueingEffectSink` (in `plans/effect-cutover/`)." Section 10 CANNOT simply remove the dead-code gate without also migrating the IO thread to `QueueingEffectSink`. **The success criteria (line 14) requires live `response_poll` path activation — Option B alone does NOT satisfy this criterion.** Two valid approaches:
  - **Option A (REQUIRED for success criterion compliance):** Coordinate Section 10.2 implementation with the effect-cutover plan: migrate the pane IO thread to `QueueingEffectSink` first, then activate `register_host_request_response` by removing its dead-code gate. The success criterion "section 10.2 removes the `#[allow(dead_code)]` gate on `PaneIoThread::register_host_request_response` and wires it into the IO thread" requires this. The response-poll test (`response_poll_emits_pty_write_on_fulfill`) only runs after the sink migration is in place.
  - **Option B (interim only — does NOT satisfy the success criterion):** For spec_chain verification of the reply FORMAT only, wire a test-only shim that injects fulfilled responses directly into the pane IO thread's `pending_responses` vec (bypassing the dead-code path). Option B may be used as an intermediate step while the effect-cutover plan lands, but the section is NOT complete until Option A's dead-code gate removal is done. Document clearly that end-to-end production behavior depends on effect-cutover and that Option B is a FORMAT-verification step only.
  The 10.0/10.2 checklist MUST call out the dependency on the IO thread's effective sink type BEFORE writing tests that assume the round-trip works end-to-end through `PaneIoThread`. If effect-cutover is blocked at implementation time, file a GAP finding and escalate — do NOT mark 10.2 complete while the dead-code gate remains.
- [ ] Make `HostEffect::CommandComplete { duration }` deterministic for testing by correcting the timing seam. **TIMING SEAM ANALYSIS (verified against code):** The duration is computed in `oriterm_core/src/term/shell_state/mod.rs:205-210`: `fn finish_command(&mut self) -> Option<Duration> { let start = self.command_start.take()?; let duration = start.elapsed(); ... }` — the call is `start.elapsed()` on an `Instant`, NOT `Instant::now()` at the interceptor. Two valid approaches to make this deterministic: **(A, preferred)** refactor `finish_command()` to accept an optional `now: Option<Instant>` parameter: `fn finish_command(&mut self, now: Option<Instant>) -> Option<Duration>`, computing `now.unwrap_or_else(Instant::now).duration_since(start)`. Production callers pass `None`; tests pass `Some(injected_instant)`. No `Arc<dyn Fn>` field needed, no `Debug` issue. **(B, alternative)** add a `clock: Option<Arc<dyn Fn() -> Instant + Send + Sync>>` field to `Term` — but this requires a `ClockFn` newtype wrapper with manual `Debug` impl (see the `#[derive(Debug)]` constraint at `oriterm_core/src/term/mod.rs:113`). Option A is preferred because it avoids the `Arc<dyn Fn>` / `Debug` complication entirely and the test-injection is at the exact right seam. **INCORRECT alternative (do NOT do this):** replacing `Instant::now()` at `oriterm_mux/src/shell_integration/interceptor.rs` — the interceptor calls `Term::set_command_start(Instant::now())` to SET the start time, but the DURATION is computed in `Term::finish_command()` via `start.elapsed()`. The interceptor is not the seam where the duration is measured.

**Validation:**

- [ ] All five TDD matrix tests transition RED → GREEN.
- [ ] The OSC 133;A scenario routed through `SpecHarness::feed()` still fails (proves the high-level processor really drops OSC 133, not just our test setup).
- [ ] The mux-layer test in `oriterm_mux/src/shell_integration/tests.rs` that runs both parsers in production order passes (proves the mux interceptor is load-bearing). The `SpecHarness` in `oriterm_core/tests/spec_chain/` has no `feed_with_mux()` method — this validation lives in the sibling unit-test module, not in `SpecHarness` and NOT in `oriterm_mux/tests/`.
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
- [ ] `osc8_survives_scrollback` — place hyperlinked text, then feed enough newlines that the row scrolls into `Grid::scrollback`. Assert the scrollback row still carries the URI on every cell. **Uses the STATE RUNG** (`term.grid().scrollback()[row][col].hyperlink()`) to inspect scrollback cells directly — `RenderableContent` does NOT expose individual scrollback rows (it has `scrollback_len: usize` for the count but no per-cell scrollback access). Do NOT use `observe_renderable` for this assertion — only viewport cells are visible through that rung.
- [ ] `osc8_terminator_cancels_attachment` — feed text, `OSC 8 ; ; uri ST`, text-A, `OSC 8 ; ; ST`, text-B. Assert text-B cells have `hyperlink_uri == None` (the empty URI terminates the attachment).
- [ ] `osc8_malformed_uri_dropped` — feed `\x1b]8;; BROKEN URI WITH SPACES \x1b\\X\x1b]8;;\x1b\\` and assert the cell carries the URI as-is (whitespace is not syntactically restricted in OSC 8 params — the terminal does not validate; it records). Negative pin: feed truncated `\x1b]8;;\x1b` (no ST) and assert no URI is attached (parser aborts on timeout / sequence boundary).
- [ ] `osc8_alt_screen_toggle_clears` — enter alt screen, attach hyperlink, leave alt screen. Assert primary screen cells are unaffected (alt-screen hyperlinks do NOT bleed).
- [ ] **Semantic pin** — `osc8_renderable_observer_not_stub` — scenario asserts `hyperlink_at: Some((0, 0, "WRONG_URI"))` against an actual URI of `"http://example.com"`. Must FAIL. If it passes, the renderable observer has regressed to the 10.0 stub.

**Implementation prerequisites (verified from catalog/osc.md):**

OSC 8 dispatch at `crates/vte/src/ansi/dispatch/osc.rs` (`b"8"` arm) already routes to `handler.set_hyperlink()`; `Term::set_hyperlink` → `Term::osc_set_hyperlink` at `oriterm_core/src/term/handler/osc.rs` already attaches URI to cells. No new dispatch work. Section 10.1 is pure verification.

**Catalog update:**

- [ ] Promote OSC-8 in `catalog/osc.md` from `implemented-unverified` → `verified`. Fill `Test chain` cell with `parser:pass dispatch:pass state:pass` + citation of `oriterm_core/tests/spec_chain/osc/hyperlinks.rs::{osc8_basic_attach, osc8_with_id, osc8_survives_reflow, osc8_survives_scrollback, osc8_terminator_cancels_attachment, osc8_malformed_uri_dropped, osc8_alt_screen_toggle_clears}`. (Token schema: `pass` / `fail` / `pending` / `missing` — NOT `passed`.)

**Validation:**

- [ ] All 8 tests pass (7 behavioral + 1 semantic pin).
- [ ] `observe_renderable` is exercised with a real expectation in every test (no test relies on rung pass-through).
- [ ] `./test-all.sh` green.

---

## 10.2 OSC 52 clipboard

**Files:**
- `oriterm_core/tests/spec_chain/osc/clipboard.rs` (new)
- `oriterm_mux/src/pane/io_thread/response_poll/tests.rs` (new — direct tests of the activated polling path). **Note**: `response_poll.rs` is currently a flat file, not a directory module. Per `.claude/rules/test-organization.md §Sibling tests.rs Pattern`, tests must live in a sibling `tests.rs`. This requires converting `response_poll.rs` → `response_poll/mod.rs` + `response_poll/tests.rs` before the test file can be created. The conversion is a 10.2 implementation prerequisite. Alternatively, place the tests in the existing `oriterm_mux/src/pane/io_thread/tests.rs` under a dedicated `#[cfg(test)]` section, which avoids the directory module conversion.
- Catalog update: `plans/spec-conformance/catalog/osc.md` (rows OSC-52-STORE, OSC-52-LOAD)

**Tests (TDD — RED first):**

- [ ] `osc52_store_clipboard_c` — feed `\x1b]52;c;SGVsbG8=\x1b\\`, assert `Effect::HostRequest(HostRequest::ClipboardLoad { .. })` is NOT emitted (this is a store, not a load), and assert the Effect-side variant is `Effect::Host(HostEffect::ClipboardStore { selection: ClipboardSelection::Clipboard, data: "Hello".into() })` — the exact field name is `data: String` (NOT `text`), as confirmed at `oriterm_core/src/effect/families/host.rs:36`. The public re-export path is `oriterm_core::effect::{HostEffect, ClipboardSelection}` (NOT the private `oriterm_core::effect::families::host` path — use the public API). **No `LegacyEventSink` assertion here** — spec_chain tests use `QueueingEffectSink`; asserting on `Event::ClipboardStore` via `LegacyEventSink` would test the wrong sink path.
- [ ] `osc52_store_clipboard_s` — same shape, `s` (selection) clipboard character, assert `selection: ClipboardSelection::Select` (NOT `Selection` — the enum variant at `oriterm_core/src/effect/families/host.rs:114` is `Select`, not `Selection`), `data: <decoded>`.
- [ ] `osc52_store_clipboard_p` — `p` (primary) clipboard character; assert `selection: ClipboardSelection::Primary`, `data: <decoded>`.
- [ ] `osc52_store_clipboard_q` — NEGATIVE PIN: `q` is NOT a valid `ClipboardSelection` variant (`ClipboardSelection` at `oriterm_core/src/effect/families/host.rs:108-115` has only `Clipboard`, `Primary`, `Select`). Feed `\x1b]52;q;SGVsbG8=\x1b\\` and assert NO `HostEffect::ClipboardStore` is emitted (the OSC 52 handler must drop unknown clipboard characters). This is a negative pin, NOT a positive test for `q` support. The success criteria (frontmatter line 14) that claims `both 'c' / 's' / 'p' / 'q' clipboard characters` is corrected by this test: `q` is tested only as a DROPPED/invalid character, not as a supported selection type.
- [ ] `osc52_load_request_fires_hostrequest` — feed `\x1b]52;c;?\x1b\\`, assert `Effect::HostRequest(HostRequest::ClipboardLoad { selection: Clipboard, clipboard_char: b'c', terminator: "\x1b\\", reply: <ResponseToken> })` is on the transcript. This is the SPEC-CHAIN assertion scope boundary — the spec_chain harness asserts the HostRequest was emitted; it does NOT simulate the IO thread's polling loop (that lives in `oriterm_mux::PaneIoThread`, which is a separate crate layer). The ResponseToken fulfillment → PtyEffect::Write round-trip is tested in `oriterm_mux/src/pane/io_thread/response_poll/tests.rs` (listed in the Files block), NOT in the spec_chain test. No `harness.poll_pending_responses()` helper is added to `SpecHarness` — doing so would force `oriterm_test_support` to depend on `oriterm_mux`'s internal `PaneIoThread`, which violates the crate boundary (see `.claude/rules/crate-boundaries.md` §crates/oriterm_test_support).
- [ ] `response_poll_roundtrip_emits_pty_write` (**in `oriterm_mux/src/pane/io_thread/response_poll/tests.rs`** if directory module conversion happened, OR in `oriterm_mux/src/pane/io_thread/tests.rs` if `response_poll.rs` stays flat — NOT in spec_chain) — construct a `PaneIoThread` (or the minimal stub thereof that holds `pending_responses`), call `register_host_request_response(request)` with a `HostRequest::ClipboardLoad { clipboard_char: b'c', terminator: "\x1b\\", reply }`, fulfill the `ResponseToken` with `reply.fulfill("example-text".into())`, call `poll_pending_responses()`, and assert the effect sink received `Effect::Pty(PtyEffect::Write { bytes })` where `bytes == format_clipboard_reply("example-text", b'c', "\x1b\\")` (base64-encoded). Uses `format_clipboard_reply` from `oriterm_core/src/effect/families/host_request.rs` — DO NOT re-implement the reply format inline (LEAK).
- [ ] **Semantic pin (in `oriterm_mux` response_poll tests, NOT spec_chain)** — `response_poll_token_requires_fulfillment` — construct a `PaneIoThread` stub, push a `HostRequest::ClipboardLoad` via `register_host_request_response`, do NOT fulfill the `ResponseToken`, call `poll_pending_responses()`. Assert NO `PtyEffect::Write` is emitted. This pins the requirement that the IO thread waits for fulfillment rather than emitting an empty reply. Lives in `oriterm_mux/src/pane/io_thread/response_poll/tests.rs` (same file as `response_poll_roundtrip_emits_pty_write`) — the spec_chain harness has no IO-thread tick mechanism, so this test MUST NOT live in spec_chain.
- [ ] `osc52_load_with_s_and_p_selections` — load with `s` and `p` characters; assert the correct `ClipboardSelection` in the `HostRequest`.
- [ ] `osc52_store_invalid_base64_dropped` — feed `\x1b]52;c;!!!invalid-base64!!!\x1b\\`, assert no `HostEffect::ClipboardStore` is emitted (store path rejects invalid base64; confirm behavior matches `oriterm_core/src/term/handler/tests/osc.rs::osc52_clipboard_load` pattern) OR assert a specific error/drop behavior — whichever the current dispatcher at `oriterm_core/src/term/handler/osc.rs::osc_clipboard_store` produces. If the current behavior is "accept garbage and store it", file `/add-bug` and document the observed behavior as the current catalog deviation.

**Catalog update:**

- [ ] OSC-52-STORE in `catalog/osc.md` → `verified` with citations for `c`, `s`, `p` clipboard characters (store); `q` documented as not supported (`ClipboardSelection` has no `q` variant — verified at `oriterm_core/src/effect/families/host.rs:108-115`).
- [ ] OSC-52-LOAD in `catalog/osc.md` → `verified` with citation of the ResponseToken round-trip test.
- [ ] `catalog/shell-integration.md` row SHINT-OSC-9-NOTIFY (cross-reference) remains pointing at `osc.md::OSC-9` (handled in 10.3).

**Validation:**

- [ ] All 7 spec_chain tests pass (4 store tests + `osc52_load_request_fires_hostrequest` + `osc52_load_with_s_and_p_selections` + `osc52_store_invalid_base64_dropped`).
- [ ] `response_poll_roundtrip_emits_pty_write` AND `response_poll_token_requires_fulfillment` in `oriterm_mux/src/pane/io_thread/response_poll/tests.rs` green — both test the IO thread path in `oriterm_mux`, never in spec_chain.
- [ ] `oriterm_core/tests/teseq/osc.rs::osc_clipboard` regression test unchanged and green.

---

## 10.3 OSC 9 / 99 / 777 desktop notifications

**Files:**
- `oriterm_mux/src/shell_integration/tests.rs` (extend sibling unit-test module — OSC 9/99/777 tests live here because `RawInterceptor` is `pub(crate)` and only accessible from sibling unit tests, NOT from `oriterm_mux/tests/` integration tests)
- Catalog updates: `catalog/osc.md` — OSC-9 and OSC-777 rows already exist (both marked `missing`; promote to `verified`); OSC-99 is NOT yet a catalog row and must be added as a new row (status `verified`). Also update `catalog/shell-integration.md` rows SHINT-OSC-9-NOTIFY, SHINT-OSC-777-NOTIFY.

**Tests (in `oriterm_mux/src/shell_integration/tests.rs` — these OSCs route through the RawInterceptor, NOT the high-level processor; must NOT be placed in `oriterm_core/tests/spec_chain/osc/` and must NOT be placed in `oriterm_mux/tests/` integration tests which have no `pub(crate)` access to `RawInterceptor`):**

- [ ] `osc9_simple_body_fires_notification` — feed `\x1b]9;Build complete\x1b\\`, assert `Effect::Host(HostEffect::DesktopNotification { source: NotificationSource::Osc9, title: "", body: "Build complete" })`. OSC 9 has no title field (Growl-style).
- [ ] `osc99_body_fires_notification_osc99_source` — feed `\x1b]99;kitty payload\x1b\\`, assert `source: NotificationSource::Osc99`. Per the interceptor at `shell_integration/interceptor.rs:124-128`, OSC 9 and 99 share the `handle_notification_simple` code path — 10.3 pins the discriminator so a future refactor cannot collapse them.
- [ ] `osc777_notify_title_body` — feed `\x1b]777;notify;Build;completed successfully\x1b\\`, assert `source: NotificationSource::Osc777, title: "Build", body: "completed successfully"`.
- [ ] `osc777_non_notify_action_dropped` — feed `\x1b]777;BAD_ACTION;title;body\x1b\\`, assert NO notification effect is emitted (the interceptor at line 143-145 filters non-`notify` actions).
- [ ] `osc9_empty_body` — feed `\x1b]9;\x1b\\`, assert `body == ""` and notification is still emitted (matches `handle_notification_simple` which accepts empty body).
- [ ] `osc777_missing_title` — feed `\x1b]777;notify;;body-only\x1b\\`, assert `title == "", body == "body-only"`.
- [ ] **Semantic pin** — `osc9_and_osc99_use_different_sources` — feed BOTH `OSC 9 ; X ST` and `OSC 99 ; Y ST` in the same scenario. Assert the two effects have DIFFERENT `NotificationSource` variants. If someone collapses the OSC 9 / 99 detection in the interceptor, this test fails immediately.
- [ ] **Negative pin** — `osc9_via_processor_without_mux_drops` — in `oriterm_mux/src/shell_integration/tests.rs`, run ONLY `Processor::advance(&mut term, osc9_bytes)` WITHOUT calling `raw_parser.advance(&mut interceptor, osc9_bytes)` first. Assert NO notification effect is emitted on the sink. This proves the mux interceptor is LOAD-BEARING for OSC 9; if someone accidentally adds OSC 9 to the high-level dispatcher too, this test fails (double-dispatch detection). NOTE: Do NOT use `SpecHarness::feed()` here — `oriterm_test_support` is NOT in `oriterm_mux`'s `[dev-dependencies]` (`oriterm_mux/Cargo.toml` only lists `tempfile = "3"` as a dev-dep). Use `Processor::advance` directly.

**Catalog update:**

- [ ] OSC-9 `catalog/osc.md` → `verified` (was `missing`). Implementation cell now cites `oriterm_mux/src/shell_integration/interceptor.rs::handle_notification_simple`.
- [ ] New row OSC-99 added to `catalog/osc.md` (status `verified`); existing OSC-777 row promoted from `missing` to `verified` (OSC-777 already exists at `catalog/osc.md:57` — do NOT create a duplicate row).
- [ ] `catalog/shell-integration.md` SHINT-OSC-9-NOTIFY and SHINT-OSC-777-NOTIFY → `verified`.

**Validation:**

- [ ] All 8 tests pass (6 behavioral + 1 semantic pin + 1 negative pin).
- [ ] `NotificationSource` enum (`oriterm_core/src/effect/families/host.rs:55-62`) remains unchanged — no new variants added in this subsection.
- [ ] **TPR checkpoint 2** — `/tpr-review` covering 10.1–10.3 + re-verification of 10.0. Catches harness / observer / ResponseToken integration issues before subsections 10.4–10.7 build on top.

---

## 10.4 OSC 133 semantic prompt + OSC 633 VS Code shell integration

**Files:**
- `oriterm_mux/src/shell_integration/tests.rs` (extend sibling unit-test module — OSC 133 + 633 tests live here because `RawInterceptor` is `pub(crate)` and not accessible from `oriterm_mux/tests/` integration tests)
- `oriterm_mux/src/shell_integration/interceptor.rs` (extend to dispatch OSC 633 sub-commands — currently NOT dispatched)
- `crates/vte/src/ansi/dispatch/osc.rs` (route OSC 633 if any part of it needs the high-level processor; otherwise leave to the raw interceptor)
- Catalog updates: `catalog/osc.md` OSC-133, OSC-633 (both currently `missing`); `catalog/shell-integration.md` SHINT-OSC-133-PROMPT, SHINT-OSC-633-VSCODE

**Tests (in `oriterm_mux/src/shell_integration/tests.rs` — OSC 133 + 633 are interceptor-handled, NOT high-level-processor-routed; MUST NOT be in `oriterm_mux/tests/` integration tests which have no `pub(crate)` visibility into `oriterm_mux`):**

### OSC 133 (Final Term semantic prompt)

- [ ] `osc133_a_sets_prompt_state` — feed `\x1b]133;A\x1b\\`. Assert `term.prompt_state() == PromptState::PromptStart` AND `term.prompt_mark_pending() == true`. Matches interceptor.rs:92-94.
- [ ] `osc133_b_sets_command_state` — feed `\x1b]133;B\x1b\\`. Assert `prompt_state == CommandStart` AND `command_start_mark_pending() == true`.
- [ ] `osc133_c_sets_output_state` — feed `\x1b]133;C\x1b\\`. Assert `prompt_state == OutputStart` AND `output_start_mark_pending() == true`. **CLOCK NOTE**: The interceptor's `b'C'` arm calls `self.term.set_command_start(std::time::Instant::now())` — there is NO injectable clock seam for this step; the start time is always a live wall-clock `Instant`. The Option A seam (`finish_command(now: Option<Instant>)`) only covers the D step where the duration is computed. Do NOT assert the specific `Instant` value stored — just assert the prompt-state transitions. The non-deterministic wall-clock issue at the C step is accepted: the meaningful determinism is at the D step (duration calculation), which uses the Option A seam.
- [ ] `osc133_d_clears_state_and_emits_command_complete` — SCOPE-CLARIFIED per scope clarification D above. **Test setup**: feed `OSC 133;C` (brings `prompt_state` to `OutputStart`; wall-clock `Instant::now()` is stored — there is no injectable seam for C). Then call `term.set_command_start(t0)` directly to overwrite the stored instant with a known `t0` (Option A seam: `finish_command(now: Option<Instant>)` only covers the D step). Then feed `OSC 133;D` with `spec_chain_helper` passing `Some(t0 + Duration::from_millis(1500))` as the `now` argument to `finish_command`. Assert:
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
- [ ] `osc22_no_parameter_is_dropped` — **NEGATIVE PIN**: feed `\x1b]22\x1b\\` (no second parameter at all, so `params.len() == 1`). The dispatcher at `osc.rs:180` gates on `b"22" if params.len() == 2` — when only one param is present, the arm does NOT match and falls to `_ => unhandled(params)`. Assert `term.mouse_cursor_icon()` is UNCHANGED. This pins that a malformed OSC 22 with no cursor-name param is silently dropped, not panicked on.
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

- [ ] `osc104_zero_args_resets_all_256_palette` — pre-populate palette: set indices 0..256 to custom colors via OSC 4 at setup. Feed `\x1b]104\x1b\\`. Assert every index 0..256 matches the initial theme palette (compare against `Palette::for_theme(Theme::default())` — `Theme` has no `.palette()` method; use `oriterm_core::color::palette::Palette::for_theme(oriterm_core::theme::Theme::default())`).
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
- `oriterm_core/src/term/mod.rs` (new Term fields: `remote_host: Option<String>`, `user_vars: HashMap<String, String>`, `shell_integration_version: Option<String>`). **RSS INVARIANT:** `user_vars` MUST be bounded to prevent unbounded memory growth under adversarial PTY output. Apply a configurable max-size cap: default 256 entries; when the cap is reached, the oldest (by insertion order — use `IndexMap` or a `VecDeque<String>` key-ring to track LRU order) entry is evicted before the new one is inserted. The RSS regression test (`oriterm_core/tests/rss_regression.rs`) MUST stay green; a `user_vars` that grows without bound per unique key fails that invariant. The size cap itself is verified by a dedicated test: `osc1337_user_vars_cap_evicts_oldest` — insert 257 distinct keys, assert the map size remains at 256 and the first-inserted key is gone.
- `plans/spec-conformance/catalog/iterm2.md` (update `owner_section` in front-matter; update per-row `Implementation` + `Verification` cells)

**Tests:**

- [ ] `osc1337_set_mark` — feed `\x1b]1337;SetMark\x1b\\`, assert `term.prompt_markers()` has a new marker with `prompt = current cursor row, command = None, output = None`. SetMark is a navigation mark equivalent to OSC 133;A's prompt boundary. Uses the same `prompt_markers` vec (SSOT).
- [ ] `osc1337_remote_host` — feed `\x1b]1337;RemoteHost=user@host.example.com\x1b\\`, assert `term.remote_host() == Some("user@host.example.com")`.
- [ ] `osc1337_current_dir` — feed `\x1b]1337;CurrentDir=/path/to/dir\x1b\\`, assert `term.cwd() == Some("/path/to/dir")`. SSOT with OSC 7 + OSC 133 (scope clarification H).
- [ ] `osc1337_copy` — feed `\x1b]1337;Copy=:SGVsbG8=\x1b\\` (the `Copy=<b64>` form), assert `Effect::Host(HostEffect::ClipboardStore { .. })` (or the equivalent store variant) with the decoded text.
- [ ] `osc1337_report_cell_size` — feed `\x1b]1337;ReportCellSize\x1b\\`, assert `Effect::Pty(PtyEffect::Write { bytes: ... })` with the expected reply format `OSC 1337 ; ReportCellSize=<H>;<W> ST` using the Term's current cell dimensions from `term.cell_pixel_height()` and `term.cell_pixel_width()` — `Term` already has `cell_pixel_width: u16` and `cell_pixel_height: u16` fields (at `oriterm_core/src/term/mod.rs:201,203`); expose public accessors if not already present. Do NOT create a new `Term::cell_size_pixels()` method — the existing fields are the SSOT.
- [ ] `osc1337_set_user_var` — feed `\x1b]1337;SetUserVar=MY_VAR=SGVsbG8=\x1b\\`, assert `term.user_var("MY_VAR") == Some("Hello")`.
- [ ] `osc1337_shell_integration_version` — feed `\x1b]1337;ShellIntegrationVersion=5\x1b\\`, assert `term.shell_integration_version() == Some("5")`.
- [ ] `osc1337_file_still_routes_to_iterm2_file` — feed a minimal `\x1b]1337;File=name=test.png;:<tiny-png-bytes>\x1b\\`, assert `Handler::iterm2_file` is still called (regression guard: the sub-dispatcher refactor from 10.0 must preserve Section 14's image path).
- [ ] `osc1337_unknown_key_dropped` — feed `\x1b]1337;NotARealKey=value\x1b\\`, assert no state mutation and the `unhandled` branch fires.
- [ ] `osc1337_user_vars_cap_evicts_oldest` — **RSS REGRESSION PIN**: insert 257 distinct `SetUserVar` keys (`KEY_0` through `KEY_256`). Assert `term.user_vars().len() == 256` (cap enforced) AND `term.user_var("KEY_0") == None` (oldest evicted). Assert `term.user_var("KEY_256") == Some(...)` (newest retained). This test MUST FAIL if `user_vars` grows unboundedly. Cross-reference 10.N RSS regression check.
- [ ] `osc1337_copy_invalid_base64_dropped` — **NEGATIVE PIN** (per `.claude/rules/tests.md §Negative Testing Protocol`): feed `\x1b]1337;Copy=:!!!not-valid-base64!!!\x1b\\`. Assert NO `HostEffect::ClipboardStore` is emitted (the Copy handler must drop invalid base64, not panic or store garbage). Mirrors the parallel `osc52_store_invalid_base64_dropped` test in 10.2 for consistency.
- [ ] `osc1337_set_user_var_invalid_base64_dropped` — **NEGATIVE PIN**: feed `\x1b]1337;SetUserVar=MY_KEY=!!!invalid!!!\x1b\\`. Assert NO entry is added to `user_vars` for `MY_KEY` (the SetUserVar handler must reject invalid base64 in the value, not store garbage). Documents the expected drop behavior; if the current dispatcher accepts invalid base64 and stores raw bytes, file `/add-bug` and update the test to assert the observed behavior as a documented deviation.
- [ ] **Semantic pin — SSOT for CWD (direction A)** — set `term.cwd()` via OSC 7 (`file:///start ST`). Feed `OSC 1337 ; CurrentDir=/other-path ST`. Assert `term.cwd() == Some("/other-path")` (last write wins; NO second CWD field). Cross-reference scope clarification §H.
- [ ] **Semantic pin — SSOT for CWD (direction B)** — set `term.cwd()` via OSC 1337 CurrentDir first (`/from-iterm2`). Then feed `OSC 7 ; file:///from-osc7 ST`. Assert `term.cwd() == Some("/from-osc7")` (OSC 7 overwrites OSC 1337 via the same canonical `Term::set_cwd` field). Matrix clamping requires BOTH directions per `.claude/rules/tests.md §Matrix Clamping` — a one-directional test misses a future regression where OSC 1337 writes a second CWD field that OSC 7 does not overwrite.

**Catalog update:**

- [ ] `catalog/iterm2.md` front-matter `owner_section` → `"01 (bootstrap), 10 (non-image), 14 (image)"`.
- [ ] Rows ITERM2-1337-REMOTEHOST, ITERM2-1337-CURRENTDIR, ITERM2-1337-COPY, ITERM2-1337-SETMARK, ITERM2-1337-REPORTCELLSIZE, ITERM2-1337-SETUSERVAR → `verified` with implementation citation to `oriterm_core/src/term/handler/mod.rs::iterm2_*`.
- [ ] ITERM2-1337-FILE stays at `implemented-unverified` (Section 14 owns its verification). Add a catalog Notes entry cross-linking Section 10's ownership of the non-image variants.
- [ ] New catalog row ITERM2-1337-SHELLINTVERSION added if missing.
- [ ] `catalog/shell-integration.md` — add or update cross-reference rows for OSC 1337 non-image sub-ops per the success criteria promise: SHINT-OSC-1337-REMOTEHOST, SHINT-OSC-1337-CURRENTDIR, SHINT-OSC-1337-SETMARK, SHINT-OSC-1337-SETUSERVAR, SHINT-OSC-1337-REPORTCELLSIZE → `verified`, each citing the corresponding `ITERM2-1337-*` row in `catalog/iterm2.md` and the `Term::iterm2_*` handler in `oriterm_core/src/term/handler/mod.rs`. These rows close out the success criterion "Every row in `catalog/shell-integration.md` is `verified` (... OSC-1337-RemoteHost / CurrentDir / SetMark / SetUserVar / ReportCellSize shell-integration cross-refs ...)".

**Plan sync to Section 14 (flow-up edit beyond single-section authority — recorded for next /review-plan):**

- `section-14-iterm2-images.md:55` currently says "Section 10's OSC suite covered the non-image OSC 1337 variants". This statement is now accurate with Section 10 owning the non-image variants — but `catalog/iterm2.md:5` (`owner_section` front-matter field) previously assigned all variants to Section 14, and `catalog/iterm2.md:15-20` (the non-image row table entries) listed those rows under Section 14. This review's flow-up edit updates `catalog/iterm2.md:5` `owner_section` to `"01 (bootstrap), 10 (non-image), 14 (image)"` (as specified in the Catalog update block above) and promotes the non-image rows. Section 14's next /review-plan will pick up the consistent state and update `section-14-iterm2-images.md:55` to cite the new catalog ownership wording if needed.

**Validation:**

- [ ] All 14 tests green: `osc1337_set_mark`, `osc1337_remote_host`, `osc1337_current_dir`, `osc1337_copy`, `osc1337_report_cell_size`, `osc1337_set_user_var`, `osc1337_shell_integration_version` (7 behavioral) + `osc1337_file_still_routes_to_iterm2_file` + `osc1337_unknown_key_dropped` + `osc1337_user_vars_cap_evicts_oldest` + `osc1337_ssot_cwd_direction_a` (OSC 7 → OSC 1337 overwrite) + `osc1337_ssot_cwd_direction_b` (OSC 1337 → OSC 7 overwrite) (2 CWD SSOT semantic pins) + `osc1337_copy_invalid_base64_dropped` + `osc1337_set_user_var_invalid_base64_dropped` (2 negative pins) = 14 total.
- [ ] OSC 1337 `File=` path unchanged; Section 14 can build on top without touching the sub-dispatcher.
- [ ] **TPR checkpoint 3** — `/tpr-review` covering 10.4–10.7 + ownership cross-check against Section 14.

---

## 10.8 Basic OSC rows (0/1/2/4/7/10/11/12/52) — inherited from Section 08

**Files:**
- `oriterm_core/tests/spec_chain/osc/basic.rs` (new — covers OSC 0/1/2 via `SpecHarness::feed()`)
- `oriterm_core/tests/spec_chain/osc/palette.rs` (new — covers OSC 4 set/query via `SpecHarness::feed()`)
- `oriterm_mux/src/shell_integration/tests.rs` (extend sibling unit-test module — covers OSC 7 via mux-layer test; OSC 7 is interceptor-handled, NOT high-level-processor-routed; must be sibling unit-test for `pub(crate)` access to `RawInterceptor`)
- `oriterm_core/tests/spec_chain/osc/default_colors.rs` (new — covers OSC 10/11/12 set/query via `SpecHarness::feed()`)
- Catalog updates: `catalog/osc.md` rows OSC-0, OSC-1, OSC-2, OSC-4-SET, OSC-4-QUERY, OSC-7, OSC-10-SET, OSC-10-QUERY, OSC-11-SET, OSC-11-QUERY, OSC-12-SET, OSC-12-QUERY; `catalog/shell-integration.md` row SHINT-OSC-7-CWD

**Scope pin from 08:** `section-08-ecma-48-baseline.md:179` recorded zero OSC coverage from tack; all rows below start at `implemented-unverified` / `stub` and end `verified` here.

**Tests (via `SpecHarness::feed()` for OSC 0/1/2/4/10/11/12 — routed through high-level processor; and via `oriterm_mux/src/shell_integration/tests.rs` sibling unit-test module for OSC 7 — interceptor-handled, requires `pub(crate)` access to `RawInterceptor`):**

### OSC 0 / 1 / 2 (title + icon name)

- [ ] `osc0_sets_title_and_icon` — feed `\x1b]0;myapp\x1b\\`, assert `term.title() == "myapp"` AND `term.icon_name() == "myapp"` (OSC 0 sets both).
- [ ] `osc1_sets_only_icon_name` — feed `\x1b]1;myicon\x1b\\`, assert `icon_name == "myicon"` AND `title` is UNCHANGED (starts empty).
- [ ] `osc2_sets_only_title` — feed `\x1b]2;mytitle\x1b\\`, assert `title == "mytitle"` AND `icon_name` is UNCHANGED.
- [ ] `osc0_empty_sets_empty_string` — feed `\x1b]0;\x1b\\`, assert both title and icon_name become the empty string `""`. **Important dispatch accuracy:** the `osc.rs` dispatcher's `b"0"` arm ALWAYS calls `handler.set_title(Some(text.clone()))` — it sends `Some("")` not `None` when the param is empty. There is NO `ResetTitle` path triggered by `OSC 0 ; ST`; `Event::ResetTitle` (now `HostEffect::TitleSet { value: None }`) is only emitted by other mechanisms (e.g. explicit reset via ESC c or the `TITLE_STACK_MAX_DEPTH` eviction path). Test assertions MUST reflect this: assert `term.title() == ""` not `term.title() == <original>`.
- [ ] `osc0_bel_and_st_terminators_both_accepted` — feed `\x1b]0;t1\x07` (BEL) AND `\x1b]0;t2\x1b\\` (ST) in sequence. Assert both update the title; the dispatcher's `bell_terminated` parameter routes correctly.
- [ ] **Cross-reference only — CSI 22;2t / 23;2t title stack**: xterm push/pop title uses **CSI 22;2t** (push) and **CSI 23;2t** (pop), dispatched from `crates/vte/src/ansi/dispatch/csi.rs`, NOT from `osc.rs`. This test does NOT belong in 10.8 and MUST NOT be placed here. The CSI window operations section (whichever roadmap section owns CSI window ops) owns this test. The cross-reference is noted here so the 10.8 reviewer knows to look for it elsewhere — Section 10.8 does NOT write this test. The title stack bound `TITLE_STACK_MAX_DEPTH = 4096` (at `oriterm_core/src/term/mod.rs:82`) is verified by the CSI section, not Section 10.

### OSC 4 (palette index)

- [ ] `osc4_set_palette_index` — feed `\x1b]4;5;rgb:ff/00/00\x1b\\`, assert `term.palette().color(5) == Rgb(0xff, 0, 0)` (`Palette::color(index)` at `oriterm_core/src/color/palette/mod.rs:282`).
- [ ] `osc4_query_palette_index` — feed `\x1b]4;5;?\x1b\\`, assert a `PtyEffect::Write` with the reply `OSC 4 ; 5 ; rgb:ffff/0000/0000 ST` (double-nibble per xterm).
- [ ] `osc4_multi_param_sets_multiple_indices` — feed `\x1b]4;1;rgb:00/ff/00;2;rgb:00/00/ff\x1b\\`, assert indices 1 and 2 are both set.
- [ ] `osc4_out_of_range_dropped` — feed `\x1b]4;999;rgb:ff/ff/ff\x1b\\`, assert no mutation.
- [ ] `osc4_invalid_color_dropped` — feed `\x1b]4;5;NOT_A_COLOR\x1b\\`, assert index 5 unchanged.

### OSC 7 (CWD — INTERCEPTOR path)

- [ ] `osc7_file_uri_sets_cwd` — (in `oriterm_mux/src/shell_integration/tests.rs` sibling unit-test module) feed `\x1b]7;file:///home/user/project\x1b\\` through the mux-layer harness (run `RawInterceptor` + `Processor` in production order via `spec_chain_helper`). Assert `term.cwd() == Some("/home/user/project")`. Uses the parse_osc7_path logic in `interceptor.rs:173-187`. NOT in `SpecHarness::feed()` — the high-level processor drops OSC 7. NOT in `oriterm_mux/tests/` — integration tests have no `pub(crate)` access to `RawInterceptor`.
- [ ] `osc7_file_uri_with_hostname` — feed `file://myhost.example.com/path/to/dir`, assert cwd is `/path/to/dir` (hostname stripped per interceptor.rs).
- [ ] `osc7_percent_decoded` — feed `file:///home/user/my%20folder`, assert cwd is `/home/user/my folder` (percent_decode in interceptor.rs:199-220).
- [ ] `osc7_emits_host_effect_cwd_set` — assert `Effect::Host(HostEffect::CwdSet { cwd: "/home/user/project" })` on the transcript.
- [ ] `osc7_relative_path_passed_through` — feed `\x1b]7;relative/path\x1b\\`. Per `strip_uri_suffix`, this passes through unchanged. Assert `cwd == Some("relative/path")`. Verify this matches production behavior; if the interceptor rejects non-URI paths in production, update the test accordingly.
- [ ] `osc7_via_high_level_processor_drops` — negative pin (lives in `oriterm_mux/src/shell_integration/tests.rs`). Run ONLY `Processor::advance(&mut term, osc7_bytes)` WITHOUT calling `raw_parser.advance(&mut interceptor, osc7_bytes)` first. Assert cwd is UNCHANGED. This pins the interceptor-only path. NOTE: Do NOT use `SpecHarness::feed()` here — `oriterm_test_support` is NOT in `oriterm_mux`'s `[dev-dependencies]` (only `tempfile = "3"` is listed). Use `Processor::advance` directly (same pattern as `osc9_via_processor_without_mux_drops` in 10.3).
- [ ] **OSC 7 double-dispatch remediation (LEAK:duplicated-dispatch):** The `b"7"` arm in `crates/vte/src/ansi/dispatch/osc.rs:69-87` calls `handler.set_working_directory()` which is a no-op default on `Term` (confirmed: `Term` does not override this method). The interceptor at `oriterm_mux/src/shell_integration/interceptor.rs:37` handles OSC 7 canonically with full URI parsing. The high-level `b"7"` arm is therefore vestigial — it calls a no-op and provides no value. The interceptor module doc (`interceptor.rs:6-9`) acknowledges this: "OSC 7 is also handled here (with proper URI parsing and percent-decoding) because `Term` does NOT override `Handler::set_working_directory` — the high-level handler default is a no-op. The interceptor is therefore the sole canonical path for CWD updates from OSC 7 (SSOT: `Term::set_cwd`)." Section 10.8 MUST remove the `b"7"` arm from `osc.rs` OR add a `// SSOT: CWD is handled by RawInterceptor; this arm is intentionally empty for parity` comment WITH an `assert!(!reachable)` / `debug_assert` semantic — leaving it silently calling a no-op creates a second apparent dispatch path that confuses future readers and could be mistakenly "fixed" to re-implement CWD logic in the wrong layer. Preferred fix: remove the arm and handle the `set_working_directory` default body more explicitly.

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

- **OSC 3** (set X11 window property) — platform-specific (X11 only). Add dispatch arm that routes to `Handler::set_x11_property(prop: &[u8], value: &[u8])`. **EFFECT NOTE**: `HostEffect::SetX11Property` does NOT exist in `oriterm_core/src/effect/families/host.rs` (the enum has no such variant). Do NOT reference this variant. The implementation must either: (a) add a new `HostEffect::SetX11Property { prop: String, value: String }` variant to the enum (update ALL match arms that consume `HostEffect` across `oriterm_eval`, `LegacyEventSink`, `QueueingEffectSink`, etc.), OR (b) keep OSC 3 as a no-op that stores state only in `Term` (add a `Term::x11_property` field queried by renderable/state rungs) without emitting any `HostEffect`. Option B is preferred for 10.9's scope to avoid a cross-crate fan-out. `Handler::set_x11_property` default on `Term` is a no-op; on Linux+X11 runtime, a future section may emit the effect. For 10.9 verification: test that the dispatch arm fires (state rung, not effect rung) and on non-X11 platforms the dispatch returns without side effects. Catalog status → `verified-with-deviation` with a note that the effect is platform-conditional.
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
- [ ] **Matrix completeness pin (SSOT: use catalog_row_id scanner, NOT function-name grep)** — use the existing `scan_test_citations` / `CoverageReport` infrastructure at `crates/oriterm_test_support/src/spec_chain/coverage/` (not a raw grep for function names). Every `SpecScenario` const in the OSC test files MUST declare `catalog_row_id: "OSC-<N>"` matching the corresponding catalog row ID. Run the coverage report (`cargo run -p oriterm_test_support --bin spec-coverage-report`) and assert every OSC catalog row has at least one citation. (Binary name is `spec-coverage-report` with hyphens, NOT `spec_coverage_report` with underscores — per `crates/oriterm_test_support/Cargo.toml:[[bin]]:name`.) Function-name grepping (`osc<N>`) would bypass this SSOT and create a second catalog-tracking mechanism that can drift from the canonical scanner.

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

<!-- Round 3 findings (2026-04-17) -->


- [x] `[TPR-10-14-codex][high]` `oriterm_mux/src/shell_integration/interceptor.rs:20` — `RawInterceptor` is `pub(crate)`, not accessible from `crates/oriterm_test_support`; 10.0 plan instruction to attach it directly in `SpecHarness` would not compile.
  Evidence: `pub(crate) struct RawInterceptor<'a, S: EffectSink>` — crate-private visibility in `oriterm_mux`.
  Impact: Any 10.0 implementation following the plan as written would fail to compile when `oriterm_test_support` tries to access `RawInterceptor`.
  Required plan update: 10.0 implementation bullet rewritten to require a `#[cfg(test)]` test hook exported from `oriterm_mux` (Option A) or a `TestInterceptor` mirroring production behavior (Option B), with Option A preferred to avoid SSOT DRIFT (FIXED).
  Basis: direct_file_inspection. Confidence: high.

- [x] `[TPR-10-15-codex][high]` `crates/oriterm_test_support/src/spec_chain/recording_handler.rs:317` — `recording_handler.rs` not listed in 10.0/10.7 Files blocks; new `Handler::iterm2_*` methods added there would not be forwarded/recorded by `RecordingHandler`, causing spec_chain tests to silently miss the new dispatch.
  Evidence: `fn iterm2_file(&mut self, params: &[&[u8]]) { self.record_other("iterm2_file"); ... }` — only `iterm2_file` is wired; no other `iterm2_*` arms exist.
  Impact: Spec_chain tests for `iterm2_set_mark`, `iterm2_remote_host`, etc. would pass even when dispatch is broken, because `RecordingHandler` would not see the new methods.
  Required plan update: `recording_handler.rs` added to 10.0 Files list with an explicit REGISTRATION SYNC note; implementation bullet added requiring delegate arms for every new `Handler::iterm2_*` method (FIXED).
  Basis: direct_file_inspection. Confidence: high.

- [x] `[TPR-10-16-codex][medium]` `plans/spec-conformance/section-10-osc-suite.md:276` — `osc52_store_clipboard_s` asserts `selection: Selection` but actual enum variant is `ClipboardSelection::Select`.
  Evidence: `pub enum ClipboardSelection { Clipboard, Primary, Select, }` at `oriterm_core/src/effect/families/host.rs:108-114` — variant is `Select`, not `Selection`.
  Impact: An implementer following the plan would write `ClipboardSelection::Selection` which fails to compile.
  Required plan update: corrected to `ClipboardSelection::Select`; also removed the `LegacyEventSink` assertion reference (spec_chain uses `QueueingEffectSink`) (FIXED).
  Basis: direct_file_inspection. Confidence: high.

- [x] `[TPR-10-17-codex][medium]` `plans/spec-conformance/section-10-osc-suite.md:269` — Test path `oriterm_mux/src/pane/io_thread/response_poll/tests.rs` requires a directory module but `response_poll.rs` is a flat file.
  Evidence: `ls oriterm_mux/src/pane/io_thread/` shows `response_poll.rs` as a flat file, not a directory. Per `.claude/rules/test-organization.md §Sibling tests.rs Pattern`, tests must be in a sibling `tests.rs` — which requires `response_poll/mod.rs + response_poll/tests.rs`.
  Impact: The plan implies tests can be created at the path without noting the directory conversion prerequisite; implementers would either skip the conversion (silently placing tests in the wrong file) or not realize the conversion is required.
  Required plan update: Files block updated with a note explaining the directory module conversion requirement; alternative of using existing `io_thread/tests.rs` documented (FIXED).
  Basis: direct_file_inspection. Confidence: high.

- [x] `[TPR-10-18-codex][low]` `plans/spec-conformance/section-10-osc-suite.md:421` — Plan cites `Theme::default().palette()` but `Theme` has no `.palette()` method; correct API is `Palette::for_theme(Theme::default())`.
  Evidence: `oriterm_core/src/theme/mod.rs` — `Theme` has only `is_dark()` and `Default`; `Palette::for_theme(theme: Theme) -> Self` is in `oriterm_core/src/color/palette/mod.rs:179`.
  Impact: An implementer following the plan would write `Theme::default().palette()` which fails to compile.
  Required plan update: corrected to `Palette::for_theme(Theme::default())` with full type paths cited (FIXED).
  Basis: direct_file_inspection. Confidence: high.

- [x] `[TPR-10-19-gemini][low]` `plans/spec-conformance/section-10-osc-suite.md:455` — `osc1337_report_cell_size` test plan references a hypothetical `Term::cell_size_pixels()` that does not exist when `Term` already has `cell_pixel_width` and `cell_pixel_height` fields.
  Evidence: `oriterm_core/src/term/mod.rs:201,203` — `cell_pixel_width: u16` and `cell_pixel_height: u16` fields exist; no `cell_size_pixels()` method present.
  Impact: Implementer may create an unnecessary new accessor instead of using the existing fields; "if available" hedge implies uncertainty where there is none.
  Required plan update: test plan updated to reference `term.cell_pixel_height()` / `term.cell_pixel_width()` accessors (expose if not already public); SSOT note added against creating a new aggregating method (FIXED).
  Basis: direct_file_inspection. Confidence: high.

<!-- Round 4 findings (2026-04-17) -->

- [x] `[TPR-10-20-codex][medium]` `plans/spec-conformance/section-10-osc-suite.md:276` — `osc52_store_clipboard_c` assertion cites nonexistent field `text: "Hello"` on `ClipboardStore`; actual field name is `data`.
  Evidence: `HostEffect::ClipboardStore { selection: ClipboardSelection, data: String }` at `oriterm_core/src/effect/families/host.rs:34-37` — field is `data`, not `text`.
  Impact: Implementer writing `HostEffect::ClipboardStore { selection: ..., text: "Hello" }` gets a compile error.
  Required plan update: corrected to `data: "Hello".into()`; `data` field name used consistently throughout 10.2 (FIXED).
  Basis: direct_file_inspection. Confidence: high.

- [x] `[TPR-10-21-codex][high]` `plans/spec-conformance/section-10-osc-suite.md:14` — Success criteria claims `q` clipboard character is verified; `ClipboardSelection` has no `q` variant.
  Evidence: `pub enum ClipboardSelection { Clipboard, Primary, Select }` at `oriterm_core/src/effect/families/host.rs:108-115` — only three variants, no `q`.
  Impact: Section cannot reach `verified` status claiming `q` support that does not and cannot exist without adding a new enum variant.
  Required plan update: success criteria corrected; `q` is now a NEGATIVE PIN (unsupported/dropped character), not a positive test; catalog update for `osc52_store_clipboard_q` revised accordingly (FIXED).
  Basis: direct_file_inspection. Confidence: high.

- [x] `[TPR-10-22-codex][medium]` `plans/spec-conformance/section-10-osc-suite.md:218` — 10.0 clock injection plan adds `Arc<dyn Fn() -> Instant + Send + Sync>` field to `Term`, which does not implement `Debug`, breaking `#[derive(Debug)]` on `Term`.
  Evidence: `#[derive(Debug)]` at `oriterm_core/src/term/mod.rs:113`; `Arc<dyn Fn>` is not `Debug`.
  Impact: Adding this field as-is would cause a compilation error and break all existing `{:?}` formatting on `Term`.
  Required plan update: 10.0 implementation bullet updated to require a `ClockFn` newtype wrapper with manual `Debug` impl before adding the field to `Term` (FIXED).
  Basis: direct_file_inspection. Confidence: high.

- [x] `[TPR-10-23-codex][medium]` `plans/spec-conformance/section-10-osc-suite.md:198` — 10.0 renderable observer plan uses `Term::palette()[index]` syntax; `Palette` does not implement `Index`.
  Evidence: `Palette::color(index: usize) -> Rgb` at `oriterm_core/src/color/palette/mod.rs:282`; no `impl Index for Palette`.
  Impact: Implementer writing `term.palette()[index]` gets a compile error; must use `term.palette().color(index)`.
  Required plan update: corrected to `term.palette().color(index) == expected_rgb` with canonical API path cited (FIXED).
  Basis: direct_file_inspection. Confidence: high.

- [x] `[TPR-10-24-gemini][low]` `plans/spec-conformance/section-10-osc-suite.md:391` — OSC 22 test matrix missing negative pin for no-parameter case (`params.len() != 2`) — existing `osc22_unknown_icon_is_dropped` only covers `from_str` failure, not the arm-miss case.
  Evidence: `b"22" if params.len() == 2` at `crates/vte/src/ansi/dispatch/osc.rs:180` — if `params.len() != 2`, the arm does not fire; no negative test pins this path.
  Impact: A regression where the guard is removed (allowing malformed OSC 22 to proceed) would go undetected.
  Required plan update: `osc22_no_parameter_is_dropped` test added as an explicit negative pin for the `params.len() == 1` case (FIXED).
  Basis: direct_file_inspection. Confidence: high.

- [x] `[TPR-10-25-gemini][low]` `plans/spec-conformance/section-10-osc-suite.md:504` — `osc0_title_stack_via_csi_t` test listed in 10.8 with ambiguous disposition ("move it or cite as cross-reference") without a firm decision, creating potential test ownership drift.
  Evidence: "This test belongs to the CSI window operations section, not the OSC matrix — move it to the appropriate section or cite it as a cross-reference here without duplicating ownership." — no clear ownership decision made.
  Impact: Future implementers may place the test in 10.8 under the impression it is an OSC test, duplicating ownership with the CSI section.
  Required plan update: item rewritten to a firm cross-reference-only note: this test is NOT Section 10's responsibility; the CSI window ops section owns it (FIXED).
  Basis: direct_file_inspection. Confidence: high.

<!-- Round 9 findings (2026-04-16) -->

- [x] `[TPR-10-42-codex][high]` `plans/spec-conformance/section-10-osc-suite.md:6` — Goal field and scope clarification B incorrectly claimed `RawInterceptor` is "the production path for OSC 7/9/99/133/633/777"; OSC 633 is currently MISSING per `catalog/osc.md:56` and is NOT handled by `RawInterceptor`.
  Evidence: `oriterm_mux/src/shell_integration/interceptor.rs:39-45` — dispatch arms are `b"7"`, `b"133"`, `b"9" | b"99"`, `b"777"` only; no `b"633"` arm exists.
  Impact: An implementer reading the goal would believe OSC 633 is on the existing interceptor path and skip the dispatch arm addition required in 10.4.
  Required plan update: Goal field corrected to describe the interceptor as the existing path for OSC 7/9/99/133/777 only; 633 dispatch noted as work 10.4 adds. Scope clarification B updated to list the current interceptor codes accurately (FIXED).
  Basis: direct_file_inspection. Confidence: high.

- [x] `[TPR-10-43-codex][high]` `plans/spec-conformance/section-10-osc-suite.md:194` — Proposed mux test helper specification omitted `post_parse_housekeeping(evicted_before)`, which production `handle_bytes()` always runs after both parser passes.
  Evidence: `oriterm_mux/src/pane/io_thread/mod.rs:260-282` — `handle_bytes` captures `evicted_before`, runs raw_parser, runs processor, then calls `self.post_parse_housekeeping(evicted_before)` (snapshot flip + eviction accounting).
  Impact: A test helper missing the housekeeping call would not produce any snapshot, making state-rung assertions invisible (no snapshot flip = stale front buffer reads stale data).
  Required plan update: Implementation bullet updated to specify the 4-step production order: (1) capture evicted_before, (2) raw_parser.advance, (3) processor.advance, (4) post_parse_housekeeping(evicted_before) (FIXED).
  Basis: direct_file_inspection. Confidence: high.

- [x] `[TPR-10-44-codex][high]` `plans/spec-conformance/section-10-osc-suite.md:196` — Proposed `RenderableExpectation` fields used `Vec<...>` and `String` types, which are not `Copy` and not const-constructible, violating the `SpecScenario` const-constructible invariant.
  Evidence: `crates/oriterm_test_support/src/spec_chain/scenario.rs:11-12` — module doc: "Every field type is `const`-constructible. Slices use `&'static [u16]` / `&'static [u8]`." `RenderableExpectation` is `#[derive(Copy, Clone, Debug, Default)]`; `Vec`/`String` fields break `Copy`.
  Impact: Adding `Vec`/`String` fields would remove `Copy` from `RenderableExpectation`, breaking all existing `const SpecScenario` declarations that embed it.
  Required plan update: All `Vec` and `String` fields replaced with `&'static` slice / `&'static str` equivalents; const-constructibility constraint explicitly documented in the implementation bullet (FIXED).
  Basis: direct_file_inspection. Confidence: high.

<!-- Round 8 findings (2026-04-17) -->

- [x] `[TPR-10-38-codex][high]` `plans/spec-conformance/section-10-osc-suite.md:192` — Implementation instruction `Add SpecHarness::with_mux_layer(self) -> Self` still present, contradicting Option A (mux tests in `oriterm_mux/tests/spec_chain/`); plus downstream subsections 10.3, 10.4, 10.8 still referenced `feed_with_mux()`.
  Evidence: Line 192 opened with "Add SpecHarness::with_mux_layer(self) -> Self…" — this API cannot exist per crate-boundary rules; `oriterm_test_support` depends only on `oriterm_core`.
  Impact: An implementer following the instruction would attempt to add an `oriterm_mux` dependency to `oriterm_test_support`, producing a compile error and a dependency cycle.
  Required plan update: Implementation instruction rewritten to describe mux-layer integration test directory setup only; all `feed_with_mux()` references in 10.0, 10.3, 10.4, 10.8 removed or replaced with `oriterm_mux/tests/spec_chain/` references (FIXED).
  Basis: direct_file_inspection. Confidence: high.

- [x] `[TPR-10-39-codex][medium]` `plans/spec-conformance/section-10-osc-suite.md:245` — `osc8_survives_scrollback` assigned to renderable rung; `RenderableContent` does not expose individual scrollback rows, only `scrollback_len: usize`.
  Evidence: `oriterm_core/src/term/renderable/mod.rs:128-168` — `RenderableContent` struct has `cells: Vec<RenderableCell>` (viewport only) and `scrollback_len: usize` (count only), no per-cell scrollback access.
  Impact: The test as written would fail to compile because `observe_renderable` cannot inspect scrollback cells.
  Required plan update: Test rewritten to use state rung (`term.grid().scrollback()[row][col].hyperlink()`) for scrollback assertions; note added explaining renderable rung is viewport-only (FIXED).
  Basis: direct_file_inspection. Confidence: high.

- [x] `[TPR-10-40-codex][medium]` `plans/spec-conformance/section-10-osc-suite.md:179` — 10.0 Files block listed `oriterm_mux/src/pane/io_thread/mod.rs` for clock injection; actual timing seam is in `shell_state/mod.rs` (finish_command signature) and `interceptor.rs` (caller update).
  Evidence: `oriterm_core/src/term/shell_state/mod.rs:205` — `fn finish_command(&mut self) -> Option<Duration>` — this is the function to modify for Option A clock injection; `io_thread/mod.rs` is a caller but not the seam.
  Impact: An implementer would modify `io_thread/mod.rs` looking for the seam, not find it, and likely implement the wrong approach.
  Required plan update: 10.0 Files block updated to cite `oriterm_core/src/term/shell_state/mod.rs` (signature change) and `oriterm_mux/src/shell_integration/interceptor.rs` (caller update); `oriterm_mux/tests/spec_chain/` added as the new mux integration test home (FIXED).
  Basis: direct_file_inspection. Confidence: high.

- [x] `[TPR-10-41-codex][low]` `plans/spec-conformance/section-10-osc-suite.md:524` — OSC 7 remediation note quoted an outdated interceptor comment: "which stores the raw URI" — the actual comment says the high-level handler is a no-op.
  Evidence: `oriterm_mux/src/shell_integration/interceptor.rs:6-9`: "OSC 7 is also handled here (with proper URI parsing and percent-decoding) because Term does NOT override Handler::set_working_directory — the high-level handler default is a no-op."
  Impact: An implementer reading the quoted text would believe the high-level handler stores a URI, when it is actually a no-op. This contradicts the correct SSOT rationale.
  Required plan update: Quoted text updated to match the actual interceptor module doc verbatim (FIXED).
  Basis: direct_file_inspection. Confidence: high.

<!-- Round 7 findings (2026-04-17) -->

- [x] `[TPR-10-35-codex][high]` `plans/spec-conformance/section-10-osc-suite.md:11` — Success criteria still promised `SpecHarness` gains a `mux_layer` capability, contradicting Option A (mux-intercepted tests in `oriterm_mux/tests/spec_chain/` with `SpecHarness` staying mux-free).
  Evidence: Line 11: "`SpecHarness` (crates/oriterm_test_support/src/spec_chain/api.rs) gains a `mux_layer` capability that runs `RawInterceptor::osc_dispatch`…" — this violates the crate boundary: `oriterm_test_support` depends only on `oriterm_core`; `RawInterceptor` is `pub(crate)` in `oriterm_mux`.
  Impact: Implementing the success criterion verbatim would require adding `oriterm_mux` as a dependency of `oriterm_test_support`, producing a dependency cycle.
  Required plan update: Success criterion rewritten: mux-intercepted OSC tests live in `oriterm_mux/tests/spec_chain/`; `SpecHarness` stays mux-free; high-level-processor tests stay in `oriterm_core/tests/spec_chain/osc/` (FIXED).
  Basis: direct_file_inspection. Confidence: high.

- [x] `[TPR-10-36-codex][medium]` `plans/spec-conformance/section-10-osc-suite.md:282` — `osc52_response_token_requires_fulfillment` negative pin placed in spec_chain context with "advance the harness ten ticks" — but spec_chain has no IO-thread tick mechanism; the test belongs in the `oriterm_mux` response_poll test layer.
  Evidence: Line 282: "negative test: emit the load request, do NOT fulfill the token, advance the harness ten ticks" — `SpecHarness` has no `poll_pending_responses()` or tick-advance API; adding one would violate the crate boundary (crate-boundaries.md §crates/oriterm_test_support).
  Impact: An implementer would attempt to add a tick-advance API to `SpecHarness`, requiring `oriterm_mux` access, producing a compile error.
  Required plan update: Test renamed `response_poll_token_requires_fulfillment`, moved explicitly to `oriterm_mux/src/pane/io_thread/response_poll/tests.rs`; validation block updated to reflect 7 spec_chain tests + 2 mux response_poll tests (FIXED).
  Basis: direct_file_inspection. Confidence: high.

- [x] `[TPR-10-37-codex][medium]` `plans/spec-conformance/section-10-osc-suite.md:188` — TDD injectable clock bullet described constructing `Term` with `Arc<dyn Fn() -> Instant + Send + Sync>` — but Option A (preferred, no Arc field) was the chosen seam; the TDD test must reflect the actual implementation approach.
  Evidence: Line 188: "constructs Term with a deterministic clock (Arc<dyn Fn() -> Instant + Send + Sync>)"; but line 218 says "No Arc<dyn Fn> field needed, no Debug issue" (Option A preferred).
  Impact: An implementer following the TDD bullet would add an Arc clock field to Term, breaking `#[derive(Debug)]` at `oriterm_core/src/term/mod.rs:113`.
  Required plan update: TDD bullet rewritten to test `finish_command(Some(t0 + 1500ms))` directly via the Option A seam, with no Arc clock field (FIXED).
  Basis: direct_file_inspection. Confidence: high.

<!-- Round 6 findings (2026-04-17) -->

- [x] `[TPR-10-32-codex][high]` `plans/spec-conformance/section-10-osc-suite.md:788` — Completion checklist crate-ordering item still routed mux-intercepted tests to `oriterm_core/tests/spec_chain/osc/*` and credited `SpecHarness mux_layer` to `oriterm_test_support`, conflicting with Round 5 fix that adopted Option A (tests in `oriterm_mux/tests/spec_chain/`).
  Evidence: `crates/oriterm_test_support` depends only on `oriterm_core`; adding a mux_layer to it violates crate-boundary rules; yet the checklist item still named that crate as the destination for mux-intercepted tests.
  Impact: An implementer following the checklist verbatim would place mux-intercepted OSC tests in `oriterm_core/tests/spec_chain/osc/*` without access to `RawInterceptor`, producing a non-compilable test module.
  Required plan update: checklist rewritten to reflect Option A — `oriterm_mux/tests/spec_chain/` for interceptor-handled OSC tests; `oriterm_test_support` receives only renderable-observer + RenderableExpectation changes; `oriterm_core/tests/spec_chain/osc/*` gets high-level-processor tests only (FIXED).
  Basis: direct_file_inspection. Confidence: high.

- [x] `[TPR-10-33-codex][high]` `plans/spec-conformance/section-10-osc-suite.md:214` — Option B described as an acceptable standalone path for spec_chain verification, contradicting success criteria that require live `#[allow(dead_code)]` gate removal on `PaneIoThread::register_host_request_response`.
  Evidence: Success criteria (line 14): "section 10.2 removes the `#[allow(dead_code, reason = \"dormant during legacy phase\")]` gate on `PaneIoThread::register_host_request_response` and wires it into the IO thread"; Option B skips the gate removal, leaving production behavior dormant.
  Impact: An implementer using Option B would satisfy the FORMAT verification test but NOT the success criterion; section would be incorrectly marked complete while the live activation remains gated.
  Required plan update: Option B reclassified as an interim FORMAT-verification step only; Option A marked REQUIRED for success criterion compliance; escalation path added for when effect-cutover is blocked at implementation time (FIXED).
  Basis: direct_file_inspection. Confidence: high.

- [x] `[TPR-10-34-codex][low]` `plans/spec-conformance/section-10-osc-suite.md:257` — Catalog test-chain token `parser:passed dispatch:passed state:passed` uses non-schema token `passed`; catalog schema uses `pass` / `fail` / `pending` / `missing`.
  Evidence: `catalog/osc.md:16-34` — all existing test-chain entries use `parser:pending dispatch:pending state:pending`; no entry uses `passed`.
  Impact: An implementer updating the catalog row would produce a malformed entry that does not conform to the schema, breaking any tooling that validates token values.
  Required plan update: `passed` → `pass` throughout the planned catalog update instruction; schema token note added inline (FIXED).
  Basis: direct_file_inspection. Confidence: high.

<!-- Round 5 findings (2026-04-17) -->

- [x] `[TPR-10-26-codex][high]` `plans/spec-conformance/section-10-osc-suite.md:192` — Preferred mux-layer approach (Option A) would require `oriterm_test_support` to call into `oriterm_mux`, violating crate boundary rules.
  Evidence: `crates/oriterm_test_support/Cargo.toml` — depends only on `oriterm_core`; adding `oriterm_mux` would create an upward dependency in `oriterm_test_support` which is not permitted.
  Impact: An implementer choosing Option A would need to add `oriterm_mux` as a dependency of `oriterm_test_support`, violating `.claude/rules/crate-boundaries.md` allowed dependency direction.
  Required plan update: Option A revised to place mux-intercepted tests in `oriterm_mux/tests/spec_chain/` (boundary-safe); Option B retains SSOT DRIFT risk warning; dependency violation explicitly documented (FIXED).
  Basis: direct_file_inspection. Confidence: high.

- [x] `[TPR-10-27-codex][medium]` `plans/spec-conformance/section-10-osc-suite.md:218` — Clock injection plan targets wrong seam (`interceptor.rs:102`); actual duration measurement is in `finish_command()` via `start.elapsed()`.
  Evidence: `oriterm_core/src/term/shell_state/mod.rs:205-210` — `fn finish_command(&mut self) { let start = self.command_start.take()?; let duration = start.elapsed(); ... }` — `elapsed()` is called here, not at `Instant::now()` in the interceptor.
  Impact: Injecting a clock at the interceptor sets the START time deterministically but does NOT control how the DURATION is measured; `start.elapsed()` still uses wall clock.
  Required plan update: plan rewritten to correct the seam — inject deterministic `now: Option<Instant>` into `finish_command()`, not into `set_command_start()` or the interceptor (FIXED).
  Basis: direct_file_inspection. Confidence: high.

- [x] `[TPR-10-28-codex][low]` `plans/spec-conformance/section-10-osc-suite.md:571` — Coverage report command uses wrong binary name (underscore vs hyphen).
  Evidence: `crates/oriterm_test_support/Cargo.toml:[[bin]] name = "spec-coverage-report"` — binary uses hyphens; plan said `cargo run --bin spec_coverage_report` (underscores).
  Impact: Command fails at runtime with "no bin named `spec_coverage_report`".
  Required plan update: corrected to `cargo run -p oriterm_test_support --bin spec-coverage-report` (FIXED).
  Basis: direct_file_inspection. Confidence: high.

- [x] `[TPR-10-29-codex][medium]` `plans/spec-conformance/section-10-osc-suite.md:774` — Registration sync check omits `recording_handler.rs` as a consumer of `Handler::iterm2_*` methods.
  Evidence: `crates/oriterm_test_support/src/spec_chain/recording_handler.rs:317` — `fn iterm2_file` is the only wired arm; missing arms cause spec_chain tests to miss new dispatch silently (established by TPR-10-15).
  Impact: `grep -rn 'fn iterm2_'` only in `crates/vte` + `oriterm_core` would miss `recording_handler.rs` sync drift.
  Required plan update: sync check expanded to include `recording_handler.rs` as a third sync point; `set_x11_property` also included (FIXED).
  Basis: direct_file_inspection. Confidence: high.

- [x] `[TPR-10-30-codex][low]` `plans/spec-conformance/section-10-osc-suite.md:773` — CWD SSOT grep too broad; `grep -rn 'cwd:'` matches comments and struct initialisations, not just field declarations.
  Evidence: `oriterm_core/src/term/mod.rs` — `cwd: None` appears in the constructor initialisation block; `cwd:` appears in doc strings. A broad grep cannot prove a single canonical field declaration.
  Impact: The verification step claims to prove SSOT but would pass even if a second `cwd` field were added in a different module.
  Required plan update: grep tightened to `grep -rn 'cwd: Option'` (field declaration) + `grep -rn 'fn set_cwd'` (mutator) — exactly one of each expected (FIXED).
  Basis: direct_file_inspection. Confidence: high.

- [x] `[TPR-10-31-gemini][medium]` `plans/spec-conformance/section-10-osc-suite.md:449` — `user_vars: HashMap<String, String>` has no size cap in the 10.7 implementation plan body, creating an RSS regression risk.
  Evidence: 10.7 Files block adds `user_vars: HashMap<String, String>` without a max-size cap; only the 10.N checklist mentions "256 entries, eviction LRU" as a requirement.
  Impact: An implementer following 10.7 alone would produce an unbounded HashMap that could exhaust memory under adversarial PTY output; the RSS regression test would catch it only at section completion, not at implementation.
  Required plan update: 10.7 Files block updated with explicit RSS invariant (256-entry cap, LRU eviction); `osc1337_user_vars_cap_evicts_oldest` regression pin test added to 10.7 test list (FIXED).
  Basis: direct_file_inspection. Confidence: high.

<!-- Round 10 findings (2026-04-17) -->

- [x] `[TPR-10-45-codex][medium]` `plans/spec-conformance/section-10-osc-suite.md:14` — Success criterion for OSC 52 said "wires it into the IO thread for the spec_chain verification harness", implying the ResponseToken round-trip runs inside the spec_chain harness; the correct boundary (per Rounds 7-8) is that spec_chain verifies HostRequest emission only, and the ResponseToken round-trip lives in oriterm_mux IO-thread tests.
  Evidence: Success criteria line 14: "wires it into the IO thread for the spec_chain verification harness" — round-trip belongs in mux IO-thread tests, not spec_chain.
  Impact: Implementer following the criterion verbatim would attempt to wire the ResponseToken round-trip through spec_chain, violating crate boundaries.
  Required plan update: Criterion rewritten to separate spec_chain scope (HostRequest emission) from oriterm_mux IO-thread scope (ResponseToken round-trip fulfillment) (FIXED).
  Basis: direct_file_inspection. Confidence: high.

- [x] `[TPR-10-46-gemini][high]` `plans/spec-conformance/section-10-osc-suite.md:116` — Scope Clarification B still said "Subsection 10.0 lands the mux_layer first" — contradicts the adopted fix (mux-intercepted tests live in `oriterm_mux/tests/spec_chain/`; no `mux_layer` API on SpecHarness).
  Evidence: Line 116: "Subsection 10.0 lands the mux_layer first. Every subsection that verifies a mux-intercepted OSC MUST opt into that layer." — there is no mux_layer API; tests live in oriterm_mux/tests/spec_chain/.
  Impact: Implementer reading Scope Clarification B would attempt to add a mux_layer extension to SpecHarness, requiring an oriterm_mux dependency on oriterm_test_support (compile error, dependency cycle).
  Required plan update: Scope Clarification B rewritten to describe the adopted solution: tests in oriterm_mux/tests/spec_chain/ with mux-internal spec_chain_helper; SpecHarness stays mux-free (FIXED).
  Basis: direct_file_inspection. Confidence: high.

- [x] `[TPR-10-47-codex][high]` `plans/spec-conformance/section-10-osc-suite.md:185` — TDD bullet used `SpecHarness::feed()` in tests placed in `oriterm_mux/tests/spec_chain/`, but `oriterm_test_support` is not in oriterm_mux's `[dev-dependencies]`, so this would not compile.
  Evidence: `oriterm_mux/Cargo.toml` [dev-dependencies]: only `tempfile = "3"` — no `oriterm_test_support`; `SpecHarness::feed()` cannot be called from oriterm_mux tests.
  Impact: An implementer following the TDD bullet would write tests that fail to compile due to the missing dev-dependency.
  Required plan update: TDD bullet rewritten to use the mux-internal `spec_chain_helper` (NOT SpecHarness); tests call `Processor::advance` and `spec_chain_helper::feed_mux_and_proc` directly (FIXED).
  Basis: direct_file_inspection. Confidence: high.

- [x] `[TPR-10-48-codex][medium]` `plans/spec-conformance/section-10-osc-suite.md:279` — Plan cited `oriterm_core::effect::families::host::{HostEffect, ClipboardSelection}` as the import path; the public re-export is `oriterm_core::effect::{HostEffect, ClipboardSelection}` — the `families::host` sub-module is private.
  Evidence: `oriterm_core/src/effect/mod.rs:14-18`: `pub use families::{... ClipboardSelection, HostEffect, ... }` — these are re-exported through the public `effect` module, not through the private `families::host` path.
  Impact: An implementer writing `use oriterm_core::effect::families::host::{HostEffect, ClipboardSelection}` gets a compile error (private module).
  Required plan update: Path corrected to the public re-export `oriterm_core::effect::{HostEffect, ClipboardSelection}` (FIXED).
  Basis: direct_file_inspection. Confidence: high.

- [x] `[TPR-10-49-codex][low]` `plans/spec-conformance/section-10-osc-suite.md:307` — 10.3 catalog update note said "newly-added OSC-99 + OSC-777 rows"; OSC-777 already exists in `catalog/osc.md` as a `missing` row, only OSC-99 is truly new.
  Evidence: `catalog/osc.md:57` — `| OSC-777 | urxvt notifications | ... | missing |` — row already present; only OSC-99 is absent from the catalog.
  Impact: Implementer following the note would attempt to add a duplicate OSC-777 row, or be confused about whether the row needs creation vs. promotion.
  Required plan update: Note corrected to distinguish: OSC-777 already exists (promote from `missing` to `verified`); OSC-99 must be added as a new row (FIXED).
  Basis: direct_file_inspection. Confidence: high.

- [x] `[TPR-10-50-codex][low]` `plans/spec-conformance/section-10-osc-suite.md:483` — 10.7 validation count "All 10 tests green" does not match the 12 test bullets in the 10.7 Tests block.
  Evidence: 10.7 Tests block has: 7 behavioral tests + `osc1337_file_still_routes_to_iterm2_file` + `osc1337_unknown_key_dropped` + `osc1337_user_vars_cap_evicts_oldest` + 2 CWD SSOT semantic pins = 12 total.
  Impact: An implementer who stops at 10 tests believes the matrix is complete when 2 tests are missing.
  Required plan update: Validation count corrected to 12 with an enumeration of all 12 test names (FIXED).
  Basis: direct_file_inspection. Confidence: high.

- [x] `[TPR-10-51-codex][low]` `plans/spec-conformance/section-10-osc-suite.md:10` — Success criterion cites `catalog/iterm2.md:14` for ownership conflict; line 14 is the FILE row (image row, not a conflict); `owner_section` is at line 5 and the non-image rows are at lines 15-20.
  Evidence: `catalog/iterm2.md:5` — `owner_section: "01 (bootstrap), 14 (verification)"` (the field to update); line 14 is the FILE row header.
  Impact: An implementer looking at `catalog/iterm2.md:14` would see the image row, not the ownership conflict location.
  Required plan update: Citation corrected to `catalog/iterm2.md:5` for `owner_section` and `catalog/iterm2.md:15-20` for the non-image rows (FIXED).
  Basis: direct_file_inspection. Confidence: high.

- [x] `[TPR-10-52-codex][high]` `plans/spec-conformance/section-10-osc-suite.md:11` — Success criteria and 10.0 implementation incorrectly claimed `oriterm_mux/tests/spec_chain/` integration tests have `pub(crate)` access to `RawInterceptor`. Integration test crates are separate compilation units with no `pub(crate)` visibility into the main crate.
  Evidence: `oriterm_mux/src/shell_integration/interceptor.rs:20` — `pub(crate) struct RawInterceptor<'a, S: EffectSink>` — `pub(crate)` is invisible to integration tests in `oriterm_mux/tests/`.
  Impact: The planned test home is non-implementable as written; tests requiring `RawInterceptor` would fail to compile.
  Required plan update: Success criteria line 11 and implementation bullets 185/194 rewritten to place `RawInterceptor`-using tests in `oriterm_mux/src/shell_integration/tests.rs` (sibling unit-test module) which has correct `pub(crate)` access (FIXED).
  Basis: direct_file_inspection. Confidence: high.

- [x] `[TPR-10-53-codex][medium]` `plans/spec-conformance/section-10-osc-suite.md:348` — `osc133_c_sets_output_state_and_records_start_instant` test asserted "inject deterministic clock" for the C step, but the interceptor's C arm calls `set_command_start(std::time::Instant::now())` with no injectable seam — the Option A seam only covers `finish_command` (D step).
  Evidence: `oriterm_mux/src/shell_integration/interceptor.rs:103-104` — `b'C' => { self.term.set_prompt_state(PromptState::OutputStart); self.term.set_command_start(std::time::Instant::now());` — hardcoded wall-clock, no injection point.
  Impact: The test would be non-deterministic or incorrect as written; the C step cannot use an injected clock without a code change not called for in the plan.
  Required plan update: Test renamed to `osc133_c_sets_output_state` and rewritten to assert only state transitions (no clock injection); a note documents that Option A seam determinism applies to D only (FIXED).
  Basis: direct_file_inspection. Confidence: high.

- [x] `[TPR-10-54-codex][medium]` `plans/spec-conformance/section-10-osc-suite.md:472` — 10.7 catalog update block updated `catalog/iterm2.md` rows but omitted the `catalog/shell-integration.md` cross-ref rows (SHINT-OSC-1337-REMOTEHOST, SHINT-OSC-1337-CURRENTDIR, etc.) promised in success criterion 2 ("Every row in `catalog/shell-integration.md` is `verified` (... OSC-1337-RemoteHost / CurrentDir / SetMark / SetUserVar / ReportCellSize shell-integration cross-refs ...)").
  Evidence: `plans/spec-conformance/section-10-osc-suite.md:9` — success criterion explicitly lists OSC-1337 shell-integration cross-refs; 10.7 catalog block had no corresponding `catalog/shell-integration.md` tasks.
  Impact: The shell-integration catalog would remain incomplete after Section 10, violating the stated success criterion.
  Required plan update: 10.7 catalog block extended with tasks to add/update SHINT-OSC-1337-* rows in `catalog/shell-integration.md` (FIXED).
  Basis: direct_file_inspection. Confidence: high.

<!-- Round 11 findings already committed per git log -->

<!-- Round 12 findings (2026-04-17) — survivor mode: codex only (gemini transport failure: no capacity) -->

- [x] `[TPR-10-55-codex][high]` `plans/spec-conformance/section-10-osc-suite.md:116` — Scope Clarification B still directed tests to `oriterm_mux/tests/spec_chain/` (integration test directory), contradicting the established fix (Round 10/11: tests needing `RawInterceptor` belong in `oriterm_mux/src/shell_integration/tests.rs` sibling unit-test module).
  Evidence: Line 116: "subsection **10.0** creates `oriterm_mux/tests/spec_chain/` with a `spec_chain_helper` module ... Every subsection that verifies a mux-intercepted OSC places its tests in `oriterm_mux/tests/spec_chain/`"
  Impact: Implementing as written would place `RawInterceptor`-using tests in integration test crates with no `pub(crate)` visibility, causing compile errors.
  Required plan update: Scope Clarification B rewritten to direct tests to `oriterm_mux/src/shell_integration/tests.rs`; all subsection Files/Tests blocks (10.3, 10.4, 10.8) and the 10.N crate-ordering checklist updated to match (FIXED).
  Basis: fresh_verification | direct_file_inspection. Confidence: high.

- [x] `[TPR-10-56-codex][medium]` `plans/spec-conformance/section-10-osc-suite.md:318` — 10.3 negative pin `osc9_via_processor_without_mux_drops` still called `SpecHarness::feed()`, which is unavailable in `oriterm_mux` test context (`oriterm_test_support` is not in `oriterm_mux`'s `[dev-dependencies]`).
  Evidence: Line 318: "route the same OSC 9 bytes through `SpecHarness::feed()` (no mux layer)" — `oriterm_mux/Cargo.toml` [dev-dependencies] only lists `tempfile = "3"`.
  Impact: An implementer following the plan would write a test that fails to compile due to missing `oriterm_test_support` dev-dependency.
  Required plan update: Negative pin rewritten to use `Processor::advance(&mut term, osc9_bytes)` directly (no `SpecHarness`), with a NOTE explaining why `SpecHarness` is unavailable in the mux context (FIXED).
  Basis: fresh_verification | direct_file_inspection. Confidence: high.

- [x] `[TPR-10-57-codex][low]` `plans/spec-conformance/section-10-osc-suite.md:323` — 10.3 catalog step said "New rows OSC-99, OSC-777 added" — but line 307 of the same section already noted "OSC-9 and OSC-777 rows already exist"; OSC-777 is a promotion, not a new row.
  Evidence: Line 307: "OSC-9 and OSC-777 rows already exist (both marked `missing`; promote to `verified`)"; Line 323: "New rows OSC-99, OSC-777 added to `catalog/osc.md`"
  Impact: An implementer adding OSC-777 as a new row would create a duplicate row, corrupting the catalog.
  Required plan update: Catalog step corrected to "New row OSC-99 added; existing OSC-777 promoted from `missing` to `verified`" (FIXED).
  Basis: direct_file_inspection. Confidence: high.

- [x] `[TPR-10-58-codex][low]` `plans/spec-conformance/section-10-osc-suite.md:480` — 10.7 flow-up note still cited `catalog/iterm2.md:14` for the ownership conflict location; the correct citation is `catalog/iterm2.md:5` for `owner_section` and `catalog/iterm2.md:15-20` for the non-image rows (same error that TPR-10-51 fixed in the success criteria at line 10, but the 10.7 body prose was not updated).
  Evidence: Line 480: "catalog/iterm2.md:14 said those variants are assigned to Section 14" — line 14 is the FILE (image) row header, not the ownership field.
  Impact: An implementer looking at `catalog/iterm2.md:14` sees the image row, not the ownership conflict location.
  Required plan update: 10.7 flow-up note corrected to cite `catalog/iterm2.md:5` for `owner_section` and `catalog/iterm2.md:15-20` for the non-image rows (FIXED).
  Basis: direct_file_inspection. Confidence: high.

<!-- Round 13 findings (2026-04-17) -->

- [x] `[TPR-10-59-codex][high]` `plans/spec-conformance/section-10-osc-suite.md:194` — 10.0 implementation bullet instructed `shell_integration/tests.rs` to call `post_parse_housekeeping(evicted_before)` — a private `fn` on `PaneIoThread` in `oriterm_mux/src/pane/io_thread/mod.rs:337`; `shell_integration` is a sibling module with no access to this method.
  Evidence: `fn post_parse_housekeeping(&mut self, evicted_before: usize)` at `oriterm_mux/src/pane/io_thread/mod.rs:337` — private method on `PaneIoThread`; `shell_integration/tests.rs` is a sibling, not part of `pane/io_thread`.
  Impact: An implementer following the plan would get a compile error: the private method is not callable from the sibling module.
  Required plan update: Bullet rewritten to (1) omit the `post_parse_housekeeping` call (test verifies `Term` state, not snapshot production), (2) note that deferred marking can be tested via public `Term` methods if needed, (3) add CRITICAL VISIBILITY NOTE explaining why the private method cannot be called from this location (FIXED).
  Basis: fresh_verification | direct_file_inspection. Confidence: high.

- [x] `[TPR-10-60-codex][medium]` `plans/spec-conformance/section-10-osc-suite.md:188` — TDD bullet plans `crates/vte/src/ansi/dispatch/tests.rs` but `dispatch/mod.rs` has no `#[cfg(test)] mod tests;` registration, so the file would not be compiled.
  Evidence: `crates/vte/src/ansi/dispatch/mod.rs` — no `#[cfg(test)] mod tests;` line; no `dispatch/tests.rs` file exists; per `.claude/rules/test-organization.md §Sibling tests.rs Pattern`, the declaration is required.
  Impact: Creating `dispatch/tests.rs` without the registration produces a file that compiles to nothing — the test runs no assertions and gives false confidence.
  Required plan update: `crates/vte/src/ansi/dispatch/mod.rs` added to 10.0 Files block with explicit instruction to add `#[cfg(test)] mod tests;` (FIXED).
  Basis: fresh_verification | direct_file_inspection. Confidence: high.

- [x] `[TPR-10-61-codex][high]` `plans/spec-conformance/section-10-osc-suite.md:563` — 10.9 OSC 3 plan references `HostEffect::SetX11Property` which does not exist in `oriterm_core/src/effect/families/host.rs`.
  Evidence: `pub enum HostEffect { Bell, VisualBell, DesktopNotification { .. }, TitleSet { .. }, IconNameSet { .. }, CwdSet { .. }, AudioRequest(..), PrintRequest(..), ClipboardStore { .. }, ChildExit { .. }, CommandComplete { .. }, ClearPendingNotifications }` — no `SetX11Property` variant.
  Impact: An implementer emitting `HostEffect::SetX11Property { .. }` in the OSC 3 handler gets a compile error; all arms that consume `HostEffect` would also need updating if the variant is added.
  Required plan update: 10.9 OSC 3 entry rewritten to clarify the variant does not exist and provide two valid paths: add the variant (with full fan-out), or use state-only (Term field) with no HostEffect emission for 10.9's scope (FIXED).
  Basis: fresh_verification | direct_file_inspection. Confidence: high.

- [x] `[TPR-10-62-codex][medium]` `plans/spec-conformance/section-10-osc-suite.md:992` — 10.N SSOT verification grep expected `cwd: Option` to appear in `term/shell_state/mod.rs`; the field is actually declared at `oriterm_core/src/term/mod.rs:147`.
  Evidence: `oriterm_core/src/term/mod.rs:147` — `cwd: Option<String>,`; `shell_state/mod.rs` contains only `set_cwd(&mut self, cwd: Option<String>)` mutator.
  Impact: An implementer running the verification grep and expecting exactly one hit in `shell_state/mod.rs` would get confused when grep returns `term/mod.rs:147` instead.
  Required plan update: SSOT check updated to cite `term/mod.rs` for the field declaration and keep `shell_state/mod.rs` only for the mutator check (FIXED).
  Basis: fresh_verification | direct_file_inspection. Confidence: high.

- [x] `[TPR-10-63-gemini][medium]` `plans/spec-conformance/section-10-osc-suite.md:460` — 10.7 OSC 1337 Copy and SetUserVar tests lack negative pins for invalid base64, violating the Negative Testing Protocol; OSC 52's parallel test plan includes `osc52_store_invalid_base64_dropped` but 10.7 has no equivalent.
  Evidence: 10.7 Tests block: no `osc1337_copy_invalid_base64_dropped` or `osc1337_set_user_var_invalid_base64_dropped` test; OSC 52's plan at line ~288 has `osc52_store_invalid_base64_dropped`.
  Impact: A regression where invalid base64 is silently stored as garbage in clipboard or user_vars would go undetected.
  Required plan update: Two negative pin tests added to 10.7: `osc1337_copy_invalid_base64_dropped` and `osc1337_set_user_var_invalid_base64_dropped`; 10.7 validation count updated from 12 to 14 (FIXED).
  Basis: direct_file_inspection. Confidence: medium.

<!-- Round 14 findings (2026-04-17) -->

- [x] `[TPR-10-64-codex][high]` `plans/spec-conformance/section-10-osc-suite.md:350` — `osc133_d_clears_state_and_emits_command_complete` test description implied C step uses injectable timestamp, contradicting the CLOCK NOTE that C always uses wall-clock `Instant::now()`.
  Evidence: "After C (clock at t0) and D (clock at t0 + 1.5s), assert:" — implies C step can be controlled at t0, but the interceptor's C arm calls `set_command_start(Instant::now())` with no injectable seam.
  Impact: An implementer following the test description would attempt to inject t0 at the C step, find no seam, and either produce a flaky test or incorrectly modify the interceptor.
  Required plan update: D test setup rewritten to call `term.set_command_start(t0)` directly after C to overwrite the stored instant, making duration computation deterministic via Option A seam (FIXED).
  Basis: fresh_verification | direct_file_inspection. Confidence: high.

- [x] `[TPR-10-65-codex][medium]` `plans/spec-conformance/section-10-osc-suite.md:530` — `osc7_via_high_level_processor_drops` negative pin said "Feed the same OSC 7 bytes via `feed()` (no mux)"; `oriterm_test_support` is not in `oriterm_mux`'s `[dev-dependencies]`, so `SpecHarness::feed()` is unavailable in the mux test context.
  Evidence: Line 530: "negative pin. Feed the same OSC 7 bytes via `feed()` (no mux)." — `oriterm_mux/Cargo.toml` [dev-dependencies] only lists `tempfile = "3"`.
  Impact: An implementer following the plan would write a test that fails to compile due to missing `oriterm_test_support` dev-dependency.
  Required plan update: Negative pin rewritten to use `Processor::advance(&mut term, osc7_bytes)` directly, with a NOTE matching the pattern used for `osc9_via_processor_without_mux_drops` in 10.3 (FIXED).
  Basis: fresh_verification | direct_file_inspection. Confidence: high.

- [x] `[TPR-10-66-codex][low]` `plans/spec-conformance/section-10-osc-suite.md:1063` — Exit criteria said "spec_chain harness routes OSC 7/9/99/133/633/777 through the real production interceptor path", which is imprecise — there is no `mux_layer` on SpecHarness; the mechanism is `oriterm_mux/src/shell_integration/tests.rs` sibling unit-test `spec_chain_helper`.
  Evidence: "The spec_chain harness routes OSC 7/9/99/133/633/777 through the real production interceptor path." — wording implies SpecHarness has mux routing capability, contradicting the adopted solution.
  Impact: A reader may believe SpecHarness has been extended with a mux_layer, contradicting the crate-boundary constraint that `oriterm_test_support` must stay mux-free.
  Required plan update: Exit criteria rewritten to name the adopted mechanism: mux-intercepted tests in `oriterm_mux/src/shell_integration/tests.rs` via sibling unit-test `spec_chain_helper`; high-level-processor tests in `oriterm_core/tests/spec_chain/osc/` (FIXED).
  Basis: direct_file_inspection. Confidence: high.

- [x] `[TPR-10-67-gemini][medium]` `plans/spec-conformance/section-10-osc-suite.md:1017` — 10.N negative pins checklist was incomplete, omitting several key negative tests from subsections 10.1, 10.2, 10.5, 10.7, and 10.8.
  Evidence: Checklist had only 5 negative pin items; subsections define 12+ negative pins including `osc7_via_high_level_processor_drops`, `osc52_store_clipboard_q`, `osc22_no_parameter_is_dropped`, `osc1337_copy_invalid_base64_dropped`, `osc1337_set_user_var_invalid_base64_dropped`, etc.
  Impact: The completion checklist would be satisfied without verifying several mandatory negative tests, allowing regression paths to go undetected.
  Required plan update: 10.N negative pins expanded to enumerate all 12 negative tests with test function names and subsection references (FIXED).
  Basis: direct_file_inspection. Confidence: medium.

- [x] `[TPR-10-68-gemini][low]` `plans/spec-conformance/section-10-osc-suite.md:487` — 10.7 validation block said "7 behavioral" without listing all 14 test names explicitly, making completeness verification harder.
  Evidence: "All 14 tests green (7 behavioral + ...)" — "7 behavioral" is abbreviated without naming the 7 tests; a count mismatch would be invisible if a test was added or removed.
  Impact: An implementer might stop at 12 tests believing the matrix is complete when 2 more are required, or miscount without explicit names.
  Required plan update: 10.7 validation block updated with all 14 explicit test names enumerated (FIXED).
  Basis: direct_file_inspection. Confidence: medium.

---

## 10.N Completion Checklist

### TDD Discipline (MUST be FIRST — per `.claude/rules/tests.md` §TDD for Bugs)

- [ ] **Failing test matrix written FIRST** — all of 10.0's harness/observer/state TDD tests are written and VERIFIED RED before any implementation lands. Then 10.1–10.9's test matrices are written and VERIFIED RED in subsection order. Skipping this invalidates the TDD contract and the section.
- [ ] **Ordering gate:** 10.0 completes before any of 10.1–10.9 starts. 10.0's TPR checkpoint 1 MUST pass before the downstream subsections are written.

### Crate ordering (per `.claude/rules/crate-boundaries.md` allowed dependency direction)

- [ ] Changes land in this order: `crates/vte` (new Handler trait methods + OSC 1337 sub-dispatcher) → `oriterm_core` (Term fields: `mouse_cursor_icon`, `remote_host`, `user_vars`, `shell_integration_version`; handler overrides) → `oriterm_mux` (remove `#[allow(dead_code)]` on `register_host_request_response`; wire live call site; interceptor extensions for OSC 633 + 1337 delegated rows where applicable; mux-intercepted OSC tests added to `oriterm_mux/src/shell_integration/tests.rs` sibling unit-test module — NOT to `oriterm_mux/tests/` integration tests, which have no `pub(crate)` access to `RawInterceptor`) → `crates/oriterm_test_support` (completed renderable observer + new `RenderableExpectation` fields; NO new `mux_layer` dependency per crate-boundary constraint — see 10.0 Option A) → high-level-processor OSC tests under `oriterm_core/tests/spec_chain/osc/*` (OSC 0/1/2/4/10/11/12/8/22/50/52/104/110/111/112/1337 non-image sub-ops).

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
  - [ ] OSC 8 terminator cancels cell attachment (`osc8_terminator_cancels_attachment`, 10.1).
  - [ ] OSC 8 renderable observer not a stub (`osc8_renderable_observer_not_stub` semantic pin, 10.1).
  - [ ] OSC 52 `q` clipboard char dropped — no `ClipboardSelection::q` variant (`osc52_store_clipboard_q`, 10.2).
  - [ ] OSC 52 load without fulfillment does NOT emit reply (`response_poll_token_requires_fulfillment`, 10.2).
  - [ ] OSC 9 via high-level processor does NOT fire notification — mux-only (`osc9_via_processor_without_mux_drops`, 10.3).
  - [ ] OSC 7 via high-level processor does NOT set cwd — interceptor-only (`osc7_via_high_level_processor_drops`, 10.8).
  - [ ] OSC 22 no-parameter case is silently dropped (`osc22_no_parameter_is_dropped`, 10.5).
  - [ ] OSC 22 unknown icon does not mutate state (`osc22_unknown_icon_is_dropped`, 10.5).
  - [ ] OSC 22 does NOT affect text cursor shape (`osc22_does_not_affect_text_cursor_shape`, 10.5).
  - [ ] OSC 50 unknown shape does not mutate state (`osc50_unknown_shape_dropped`, 10.5).
  - [ ] OSC 1337 Copy invalid base64 is dropped (`osc1337_copy_invalid_base64_dropped`, 10.7).
  - [ ] OSC 1337 SetUserVar invalid base64 is dropped (`osc1337_set_user_var_invalid_base64_dropped`, 10.7).
- [ ] **Cross-pattern matrix**: every OSC that has SET and QUERY forms has both tested in the same subsection; every OSC that has SET and RESET forms has both tested.

### Rules weaving (per `.claude/rules/impl-hygiene.md` + `.claude/rules/code-hygiene.md` + `.claude/rules/crate-boundaries.md` + `.claude/rules/oriterm_core.md` + `.claude/rules/oriterm_mux.md`)

- [ ] **No SSOT drift**: `Term::cwd` is the ONLY CWD field — OSC 7, OSC 133, and OSC 1337 CurrentDir all route through it (10.4, 10.7, 10.8). Verified by: (1) `grep -rn 'cwd: Option' oriterm_core/src/term/` returns exactly ONE field declaration (`term/mod.rs:147` — the field lives in `Term<S>` directly, NOT in `shell_state/mod.rs`; `shell_state/mod.rs` contains only the `set_cwd` mutator method); (2) `grep -rn 'fn set_cwd' oriterm_core/src/term/` returns exactly ONE function definition (in `shell_state/mod.rs:245`); (3) all `set_cwd` call sites route through `Term::set_cwd` (not direct field access). A broad `grep -rn 'cwd:'` is insufficient — it matches comments, doc strings, and struct initialisations that don't reveal whether there are TWO fields named `cwd`.
- [ ] **No registration sync drift**: new `NotificationSource` variants (none added in this section — pinned in 10.3) AND new `Handler` trait methods (iterm2_* in 10.0 / 10.7 + x11_property in 10.9) are checked for sync across ALL three consumers — `grep -rn 'fn iterm2_'` in (1) `crates/vte/src/ansi/handler.rs` (trait declaration), (2) `oriterm_core/src/term/handler/mod.rs` (Term impl), AND (3) `crates/oriterm_test_support/src/spec_chain/recording_handler.rs` (RecordingHandler delegate) — all three must have matching method entries. Missing from `recording_handler.rs` means spec_chain tests silently miss the new dispatch (per finding TPR-10-15 precedent). Also verify `set_x11_property` is synced across all three when added in 10.9.
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

**Exit Criteria:** Every OSC catalog row in `catalog/osc.md` is `verified` or `verified-with-deviation`. Every row in `catalog/shell-integration.md` is `verified`. The non-image rows of `catalog/iterm2.md` are `verified` and their ownership is cleanly assigned to Section 10. Mux-intercepted OSC verification (OSC 7/9/99/133/633/777) lives in `oriterm_mux/src/shell_integration/tests.rs` (sibling unit-test module) via the mux-internal `spec_chain_helper` — `SpecHarness` stays mux-free and the integration test directory (`oriterm_mux/tests/`) contains NO `RawInterceptor`-using tests. High-level-processor OSC tests (0/1/2/4/8/10/11/12/22/50/52/104/110/111/112/1337 non-image) live in `oriterm_core/tests/spec_chain/osc/`. OSC 52 ResponseToken round-trip runs end-to-end through the activated `response_poll` path. OSC 22 has real Term state, not a no-op stub. OSC 133;D's behavior is documented and pinned against the actual `PromptMarker` data model. The OSC suite is conformance-complete and Section 14 can pick up the OSC 1337 sub-dispatcher without refactoring it.
