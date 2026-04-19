---
section: "10"
title: "OSC Suite (full)"
status: in-progress
reviewed: true
goal: "Drive every row in `plans/spec-conformance/catalog/osc.md`, `plans/spec-conformance/catalog/shell-integration.md`, and the non-image rows of `plans/spec-conformance/catalog/iterm2.md` (SetMark, RemoteHost, CurrentDir, Copy, ReportCellSize, SetUserVar, ShellIntegrationVersion) from `implemented-unverified` / `stub` / `missing` to `verified`. Section 10 owns the ENTIRE OSC stack — Section 08's post-completion audit (`section-08 Implementation notes 2026-04-14`) recorded that tack scenarios drove ZERO OSC rows. Basic OSC rows (0, 1, 2, 4, 7, 10, 11, 12, 52) stay owned by Section 10, NOT Section 08. This includes OSC 8 hyperlinks, OSC 22/50 cursor icon/shape, OSC 9/99/777 desktop notifications, OSC 104/110/111/112 color reset, OSC 133 semantic prompt, OSC 633 VS Code shell integration, and OSC 1337 non-image sub-ops. Section 10 also lands the prerequisites that make these rows testable: a mux-internal `spec_chain_helper` (in `oriterm_mux/src/shell_integration/tests.rs`) that exercises `oriterm_mux::shell_integration::RawInterceptor` (existing production path for OSC 7/9/99/133/777; OSC 633 dispatch is added by subsection 10.4), a completed renderable observer (OSC 8 cell-metadata assertions), a Term-level mouse-cursor-icon state (OSC 22 — already landed under §10.0 partial landing 2026-04-18), and an extensible OSC 1337 sub-dispatcher (handed off to Section 14 for images). The OSC 52 `HostRequest::ClipboardLoad` → `PtyEffect::Write` response-poll pipeline is ALREADY LIVE post effect-cutover §01.1 (gate removed, `register_host_request_response` wired from `effect_router/mod.rs:194,215`, nine response_poll tests green in `oriterm_mux/src/pane/io_thread/response_poll/tests.rs`); Section 10 is a CONSUMER of that pipeline, not an activator of it — 10.2 adds OSC-52-specific spec_chain and IO-thread round-trip pins on top of the already-green pipeline."
success_criteria:
  - "Every row in `plans/spec-conformance/catalog/osc.md` is `verified` or `verified-with-deviation` (no `implemented-unverified`, no `stub`, no `missing`) — this includes the basic subset 08 left unverified (OSC 0/1/2/4/7/10/11/12/52) and the advanced subset (OSC 8/22/50/104/110/111/112/9/99/777/133/633 and the non-image OSC 1337 sub-ops)"
  - "Every row in `plans/spec-conformance/catalog/shell-integration.md` is `verified` (OSC-7-CWD, OSC-133 A/B/C/D, OSC-633 VS Code, OSC-1337-RemoteHost / CurrentDir / SetMark / SetUserVar / ReportCellSize shell-integration cross-refs, OSC-9/777 notification cross-refs)"
  - "The non-image rows of `plans/spec-conformance/catalog/iterm2.md` (ITERM2-1337-REMOTEHOST, ITERM2-1337-CURRENTDIR, ITERM2-1337-COPY, ITERM2-1337-SETMARK, ITERM2-1337-REPORTCELLSIZE, ITERM2-1337-SETUSERVAR, ITERM2-1337-SHELLINTVERSION) are `verified`. `ITERM2-1337-SHELLINTVERSION` was added to `plans/spec-conformance/catalog/iterm2.md` as a new row during the /review-plan pass that landed Section 10's ownership. `owner_section` in `plans/spec-conformance/catalog/iterm2.md:5` is `01 (bootstrap), 10 (non-image), 14 (image)` — Section 10 owns the non-image sub-ops and Section 14 owns ONLY `ITERM2-1337-FILE` + image-adjacent rows. Cross-checked against `plans/spec-conformance/section-14-iterm2-images.md:55` (which now reads \"Section 10's OSC suite covered the non-image OSC 1337 variants; this section covers the image variants\") and `plans/spec-conformance/catalog/iterm2.md:15-21` (non-image rows tagged with `Owner: Section 10 (non-image)` in each Notes cell)."
  - "OSC 7 / 9 / 99 / 133 / 633 / 777 are verified against the REAL production path (`oriterm_mux/src/shell_integration/interceptor.rs`) via spec_chain unit tests that live in `oriterm_mux/src/shell_integration/tests.rs` (sibling unit-test module — has `pub(crate)` access to `RawInterceptor` per the Rust unit-test visibility rule). Tests that need `RawInterceptor` MUST NOT live in `oriterm_mux/tests/spec_chain/` — integration test crates are separate compilation units and cannot access `pub(crate)` items in the main crate. `crates/oriterm_test_support` (`SpecHarness`) stays mux-free — no `mux_layer` API is added to `SpecHarness`. High-level-processor OSC tests (OSC 0/1/2/4/8/10/11/12/22/50/52/104/110/111/112/1337 non-image) stay in `oriterm_core/tests/spec_chain/osc/`."
  - "`observe_renderable` (crates/oriterm_test_support/src/spec_chain/observers/renderable.rs) is no longer a stub — it asserts cell hyperlink URI, cursor position, cursor shape, palette entries, and damaged lines. Every OSC 8 subsection test exercises this observer with a scenario that would FAIL if the observer remained a stub (semantic pin against `RungResult::pass(rung)` stub-behavior)"
  - "OSC 8 hyperlink rows verified — cell-attached URI survives reflow, scroll into scrollback, copy (cell metadata), and alt-screen toggle; the OSC 8 terminator (empty URI) cancels the attachment on subsequent cells; `id=<id>` parameter is preserved but does not change attachment semantics (per gist:egmontkob)"
  - "OSC 52 clipboard rows verified — `c`, `s`, `p` clipboard characters (store and load); `q` is explicitly pinned as an unsupported/dropped character (no `ClipboardSelection::q` variant exists — see `oriterm_core/src/effect/families/host.rs:108-115`). `HostRequest::ClipboardLoad` apex is verified in spec_chain (harness asserts the HostRequest is emitted); the `ResponseToken` round-trip to `PtyEffect::Write` is verified in `oriterm_mux/src/pane/io_thread/response_poll/tests.rs` (`response_poll_token_requires_fulfillment`, `response_poll_idle_wake_unblocks_select`, `two_fulfills_one_wake_drains_both_tokens`, `fulfill_immediately_after_request_still_delivers`, `cancellation_detects_after_staging_drain`, and `fulfill_before_register_still_delivers` already landed under effect-cutover §01.1 — Section 10.2's role is NOT to activate a dormant path but to add OSC-52-specific end-to-end coverage on top of the already-live pipeline). The `#[allow(dead_code, reason = \"dormant during legacy phase\")]` gate was REMOVED by effect-cutover §01.1; `register_host_request_response` is already wired from `oriterm_mux/src/pane/io_thread/effect_router/mod.rs:194` for `HostRequest::ClipboardLoad` and `effect_router/mod.rs:215` for `HostRequest::ColorQuery`. Section 10.2's scope is (a) OSC-52-specific spec_chain tests asserting `HostRequest::ClipboardLoad` emission from OSC 52 `c/s/p` load bytes and (b) a dedicated `osc52_register_poll_roundtrip` IO-thread test that drives the end-to-end pipeline from OSC 52 load bytes through the already-active `register_host_request_response` → `poll_pending_responses` → `PtyEffect::Write` path (distinct from the `HostRequest::ColorQuery`-based tests that effect-cutover §01.1 used)."
  - "OSC 9 / 99 / 777 desktop notification rows verified — `Effect::Host(HostEffect::DesktopNotification { source, title, body })` is observed with the correct `NotificationSource` discriminator (`Osc9`, `Osc99`, `Osc777`); empty-body and missing-title cases are pinned so `String::from_utf8_lossy` boundary behavior is stable"
  - "OSC 133 semantic prompt rows verified — OSC 133;A/B/C each update `PromptMarker` (`prompt`, `command`, `output` fields — `PromptMarker` has NO fourth field for D) AND drive the `PromptState` state machine correctly; OSC 133;D does NOT update `PromptMarker` — D clears `prompt_state` (back to `None`) AND emits `HostEffect::CommandComplete`. `HostEffect::CommandComplete { duration }` assertion is deterministic via an INJECTABLE clock (no wall-clock reliance). The catalog split into `OSC-133-PROMPT` (A/B/C, `state-snapshot`) and `OSC-133-CMD-COMPLETE` (D, `effect-host-command` — a NEW `ApexLayer` variant added by Section 10.4 to `crates/oriterm_test_support/src/spec_chain/scenario.rs` and `plans/spec-conformance/00-overview.md:820`, since `EffectHostNotification` is semantically wrong for CommandComplete) reflects this two-path behavior."
  - "OSC 633 VS Code shell integration rows verified against the authoritative VS Code source at `https://github.com/microsoft/vscode/blob/main/src/vs/platform/terminal/common/xterm/shellIntegrationAddon.ts` — every OSC 633 sub-command that VS Code emits is catalogued and tested; OSC 633 is currently MISSING per `plans/spec-conformance/catalog/osc.md:56`; subsection 10.4 adds its dispatch arm EXCLUSIVELY to `oriterm_mux/src/shell_integration/interceptor.rs` (the raw `vte::Parser` + `RawInterceptor` path) — the same interceptor-only architecture used for OSC 133 / 9 / 99 / 777. OSC 633 MUST NOT be added to the high-level `Processor` path (`crates/vte/src/ansi/dispatch/osc.rs`) because the high-level processor silently drops interceptor-managed sequences (per scope clarification B); adding it there would create a second dispatch path that fires when the high-level processor runs on the same bytes, producing double-handling. A negative pin (`osc633_via_high_level_processor_drops`) confirms OSC 633 bytes fed through `Processor::advance` only do NOT trigger any state change"
  - "OSC 22 cursor icon row verified — `Term` grows a `mouse_cursor_icon: Option<CursorIcon>` field (in `oriterm_core/src/term/mod.rs`) and an override of `Handler::set_mouse_cursor_icon` on `Term` that writes to it; `RenderableContent` exposes this state to the rendering consumer; OSC 22's `plans/spec-conformance/catalog/osc.md:29` row is promoted from `stub` to `verified` — the current no-op at `crates/vte/src/ansi/handler.rs:270` is replaced by real state mutation. OSC 22 and OSC 50 (cursor SHAPE) MUST use distinct Term fields — conflation would make reset semantics incorrect (OSC 22 = mouse-cursor icon; OSC 50 = text-cursor shape, already wired via `Term::set_cursor_shape`)"
  - "OSC 50 legacy cursor-shape rows verified — the `CursorShape=N` form with N ∈ {0 block, 1 beam, 2 underline} round-trips through `Term::set_cursor_shape`; DECRQSS-style query (if supported) returns the correct response; OSC 50 with unknown N is dropped via `unhandled` without mutating cursor shape (negative pin)"
  - "OSC 104 / 110 / 111 / 112 color reset rows verified — OSC 104 with zero args resets ALL 256 palette entries to the theme default; OSC 104 with explicit indices resets only those; OSC 110 / 111 / 112 reset Foreground / Background / Cursor default respectively; post-reset state matches the initial theme palette byte-for-byte; subsequent OSC 10/11/12 queries return the theme default values"
  - "OSC 1337 non-image sub-ops verified — the dispatcher at `crates/vte/src/ansi/dispatch/osc.rs:248-254` is refactored into a key=value sub-dispatcher that delegates to named handler methods (`Handler::iterm2_set_mark`, `Handler::iterm2_remote_host`, `Handler::iterm2_current_dir`, `Handler::iterm2_copy`, `Handler::iterm2_report_cell_size`, `Handler::iterm2_set_user_var`, `Handler::iterm2_shell_integration_version`) while preserving the existing `Handler::iterm2_file` arm for Section 14. Cross-cutting with Section 14 is explicitly tracked — Section 14 inherits the sub-dispatcher and adds `File=` verification on top"
  - "New OSC rows previously `missing` in `plans/spec-conformance/catalog/osc.md` (OSC-13, OSC-14, OSC-17, OSC-19, OSC-113, OSC-114, OSC-117, OSC-119, OSC-3, OSC-5, OSC-6, OSC-L, OSC-l) each have a dispatch arm, a Term handler, and a verified row. Rows the plan cannot responsibly `verify` (OSC-3 X11-only, OSC-L / OSC-l historical) are promoted to `verified-with-deviation` with a catalog note naming the deviation"
  - "All existing teseq OSC tests at `oriterm_core/tests/teseq/scenarios/osc/{osc_title,osc_icon_name,osc_clipboard,osc_color_query}.teseq` continue to pass unchanged — they are regression guards against OSC 0/1/2/4/10/11/12/52 dispatch basics"
  - "Alloc regression (`oriterm_core/tests/alloc_regression.rs`) stays green — no OSC 10/11/12 query or OSC 52 load reply path may allocate per-byte in the hot path; reply formatting goes through `format_clipboard_reply` / `format_color_reply` in `oriterm_core/src/effect/families/host_request.rs` (already the canonical home) rather than ad-hoc `format!` calls at dispatch sites"
  - "`./build-all.sh` (debug + release + Windows cross-compile via `cargo build --target x86_64-pc-windows-gnu`) green; `./test-all.sh` green (debug workspace sweep); explicit release-mode run `timeout 150 cargo test --workspace --features oriterm/gpu-tests --release` green (release-mode alloc and `#[cfg(debug_assertions)]` divergence is invisible to `./test-all.sh`); `./clippy-all.sh` green (zero new warnings under `deny(clippy::all)` + nursery)"
  - "Section's mission-criterion connection: contributes to **Verification chain complete per row** (every applicable OSC row reaches `verified` with parser → dispatch → state/effect apex green) AND **Effect/State separation enforced** (the OSC 52 `c/s/p` spec_chain + IO-thread round-trip tests pin OSC 52 as the first production OSC consumer of the response_poll pipeline that effect-cutover §01.1 brought live; Section 10 does NOT re-activate the pipeline — it adds OSC 52 as a protocol-level pin against regression)"
inspired_by:
  - "gist:egmontkob — OSC 8 hyperlink canonical spec (`https://gist.github.com/egmontkob/eb114294efbcd5adb1944c9f3cb5feda`)"
  - "Final Term proposal — OSC 133 semantic prompt (FTCS_* markers)"
  - "iTerm2 proprietary-escape-codes documentation — OSC 9, OSC 1337 (non-image sub-ops)"
  - "VS Code `shellIntegrationAddon.ts` — OSC 633 sub-commands + arguments"
  - "kitty terminal docs — OSC 777 desktop notifications (rxvt-unicode lineage)"
  - "xterm `ctlseqs.html` — OSC 0/1/2/4/7/8/10/11/12/22/50/52/104/110/111/112/3/5/13/14/17/19"
  - "wezterm `escape-sequences.md` — de-facto OSC behavior reference across variants"
  - "alacritty `crates/vte/src/ansi/dispatch/osc.rs` (upstream) — dispatcher shape this section extends"
depends_on: ["03", "08", "plans/effect-cutover/section-01-migrate-mux-consumer.md"]
third_party_review:
  status: resolved
  updated: "2026-04-18"
  rounds_completed: 30
  notes: "TPR-10-109 (SIZE_VIOLATION documented exception) triaged via /verify-tpr on 2026-04-18: ACCEPTED with blocked-task anchor. The §10.N 'Accepted audit findings' SIZE_VIOLATION entry now carries `<!-- blocked-by:anchor-migration-plan -->` + explicit unblock condition (targeted /review-plan producing split proposal + anchor-rewrite strategy). Status stays `findings` because the anchored task remains open; transitions to `resolved` when the split lands or a permanent exception is ratified. Round 30 (/review-plan → /tpr-review, 2026-04-18): four /tpr-review rounds after the Step 5 editor pass — round 0 fixed 8 residual drift/consistency items (34e7214e), round 1 fixed 6 dependency/crate-ordering items (84f11055), round 2 fixed 2 alignment items with gemini clean (f5de3ab6), round 3 fixed 3 meta count-drift items with gemini clean (ad77d3ee). Convergence achieved: all ever-verified findings resolved inline; zero outstanding - [ ] items."
sections:
  - id: "10.0"
    title: "Harness + observer + state prerequisites (spec_chain mux layer, renderable observer, Term mouse cursor icon field, OSC 1337 sub-dispatcher, injectable clock)"
    status: complete
  - id: "10.1"
    title: "OSC 8 hyperlinks — dispatch, cell metadata, reflow/scroll/copy/alt-screen survival"
    status: complete
  - id: "10.2"
    title: "OSC 52 clipboard — store + load + ResponseToken round-trip (consumer-side coverage on the already-live response_poll path)"
    status: complete
  - id: "10.3"
    title: "OSC 9 / 99 / 777 desktop notifications — NotificationSource discriminators"
    status: complete
  - id: "10.4"
    title: "OSC 133 semantic prompt (A/B/C/D) + OSC 633 VS Code shell integration"
    status: complete
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
    status: complete
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

**Status:** In Progress (10.R — Third Party Review — complete; 10.0 / 10.1 / 10.2 / 10.3 — complete; 10.4–10.9 + 10.N — not started). Matches frontmatter `status: in-progress` and the per-subsection status fields at the top of this file.

**Goal:** Verify EVERY OSC catalog row — basic (inherited from Section 08) and advanced (Section 10's own Phase 3 Group A expansion). Each OSC number gets a test that emits the sequence and asserts the correct apex: high-level-processor OSCs (0/1/2/4/8/10/11/12/22/50/52/104/110/111/112/1337 non-image and the new 10.9 variants) use `SpecHarness::feed()` with `observe_state` / `observe_effect` / `observe_renderable`; interceptor-handled OSCs (7/9/99/133/633/777) use the mux-layer sibling unit test in `oriterm_mux/src/shell_integration/tests.rs` which has `pub(crate)` access to `RawInterceptor` — SpecHarness does NOT route interceptor-managed sequences. This section owns the entire OSC stack plus its prerequisites: harness extensions, Term state additions, and dispatcher refactors. The OSC 52 ResponseToken pipeline (`response_poll`) was brought live by effect-cutover §01.1 (gate removed, call sites wired, tests green); Section 10 is a CONSUMER of that pipeline — §10.2 adds OSC-52-specific end-to-end coverage on top of the already-green path, it does NOT activate a dormant path.

**Structural note (size justification — documented exception to `.claude/skills/plan-audit/plan-audit.py` SIZE_VIOLATION heuristic):** Section 10 is intentionally large (352 top-level items as of 2026-04-18 Step 5 editor pass, ~1600 lines — the `plan-audit.py` SIZE_VIOLATION finding that flags sections with >20 top-level items is acknowledged and explicitly accepted here). The section is decomposed into 10.0–10.9 subsections + 10.R (TPR findings) + 10.N (completion checklist), each of which is itself a coherent execution unit with Files, Tests, Implementation, Catalog update, and Validation blocks. Splitting this file into ten sibling section files (e.g. `section-10a-harness.md`, `section-10b-osc8.md`, ...) would scatter the load-bearing Scope Clarifications (A–J) across ten files, break the stable citation anchors used by the 25+ rounds of TPR findings in 10.R, and lose the dependency ordering narrative (10.0 MUST complete before 10.1–10.9 start). The auditor's 20-item top-level heuristic is a sign to SUBSECTION, not to SPLIT — this section is already correctly subsectioned. Do not re-litigate this structure without a concrete proposal for how the cross-subsection Scope Clarifications and TPR anchors would remain citable after a split. **This exception applies to Section 10 only by virtue of its 25-round TPR history; future sections do NOT inherit this license and MUST subsection-or-split at the 500-line / 20-item thresholds.**

**Success Criteria:** see frontmatter.

---

## Scope clarifications (load-bearing — read before writing any tests)

These clarifications resolve the ambiguities reviewers surfaced during the /review-plan blind-spot pass:

### A. Section 08 did NOT verify any OSC rows

`plans/spec-conformance/section-08-ecma-48-baseline.md:179` (Implementation notes, 2026-04-14) explicitly records: *"OSC row ownership audit: tack scenarios drive zero OSC rows — all basic OSC rows (0, 1, 2, 4, 7, 10, 11, 12, 52) remain owned by Section 10."* This Section 10 therefore owns the WHOLE OSC stack. Sub-section **10.8** is a first-class deliverable, not a cleanup note.

### B. Spec_chain harness does NOT route mux-intercepted OSCs through the real production path

`SpecHarness` at `crates/oriterm_test_support/src/spec_chain/api.rs:82-103` wraps `Processor::advance_with_observer` (high-level VTE processor). The production-path interceptor at `oriterm_mux/src/shell_integration/interceptor.rs` runs a SEPARATE raw `vte::Parser` on the SAME bytes BEFORE the high-level processor — this is the only path that currently sees OSC 7, OSC 9, OSC 99, OSC 133, and OSC 777 (the high-level `Processor::advance_with_observer` silently drops them per the interceptor's own module doc: *"The vte::ansi::Processor does not route OSC 133, OSC 9/99/777, or XTVERSION (CSI >q) to Handler trait methods"*). OSC 633 is currently `MISSING` per `plans/spec-conformance/catalog/osc.md:56` — subsection **10.4** adds its dispatch arm to the interceptor.

Consequence: verifying OSC 7/9/99/133/633/777 via `SpecHarness` alone would test a dispatch path that DOES NOT RUN IN PRODUCTION. The solution (adopted in Round 5, ratified in Rounds 7 + 10 + 11) is NOT to add a `mux_layer` extension to `SpecHarness` — doing so would require `oriterm_test_support` to depend on `oriterm_mux`, violating crate boundaries. Instead, subsection **10.0** adds a `spec_chain_helper` test-only module inside `oriterm_mux/src/shell_integration/tests.rs` (sibling unit-test module) that runs `RawInterceptor` + `Processor` in production order. Because this module compiles as part of the `oriterm_mux` crate, it has full `pub(crate)` access to `RawInterceptor`. **CRITICAL**: Tests that need `RawInterceptor` MUST be placed in `oriterm_mux/src/shell_integration/tests.rs` — integration tests in `oriterm_mux/tests/` are separate compilation units with no `pub(crate)` visibility. Only tests exercising purely public `oriterm_mux` APIs may live in `oriterm_mux/tests/`. `SpecHarness` remains mux-free.

### C. The renderable observer is a no-op stub

`crates/oriterm_test_support/src/spec_chain/observers/renderable.rs:21-29` returns `RungResult::pass(RungName::Renderable)` unconditionally. Every OSC 8 hyperlink test planned against this observer would pass WITHOUT CHECKING ANYTHING — a silent false-green. Subsection **10.0** completes the observer before any OSC 8 test is written; the OSC 8 subsection **10.1** includes a semantic pin that fails if the observer regresses to the stub.

### D. `PromptMarker` has no D-field

The existing `PromptMarker` at `oriterm_core/src/term/mod.rs:60-67` has fields `prompt: usize`, `command: Option<usize>`, `output: Option<usize>` — no fourth field for OSC 133;D. The production handler at `oriterm_mux/src/shell_integration/interceptor.rs:105-112` sets `prompt_state = PromptState::None` and emits `HostEffect::CommandComplete { duration }`. Subsection **10.4**'s D-test MUST assert (i) state returns to `None`, (ii) `CommandComplete { duration }` is on the effect transcript, and (iii) `prompt_markers.last()` retains its A/B/C fields from the completed lifecycle — NOT that a D-row was written.

### E. OSC 22 is an unimplemented stub, not `implemented-unverified` — AND its state-vs-push architecture is a cross-section concern

`crates/vte/src/ansi/handler.rs:270` defined `fn set_mouse_cursor_icon(&mut self, _: CursorIcon) {}` — an empty default on the Handler trait. Section 10.0's partial landing (2026-04-18) added `Term::mouse_cursor_icon: Option<CursorIcon>` + override on `Handler::set_mouse_cursor_icon` + `RenderableContent::mouse_cursor_icon` surface + daemon-mode `PaneSnapshot::mouse_cursor_icon` transport. `plans/spec-conformance/catalog/osc.md:29` is promoted from `stub` to `verified` as part of 10.5. **PUSH-VS-POLL ARCHITECTURAL NOTE (blind-spot remediation):** the current design exposes `Term::mouse_cursor_icon()` as a polling getter that UI consumers (the native-window code that drives the OS cursor) must call on every frame or on a render signal. A push-style alternative would be for `Term` to emit `Effect::Ui(UiEffect::MouseCursorChanged(icon))` when the field mutates, so UI consumers update lazily and stay in sync without per-frame polling. Section 10.5 does NOT switch to push semantics — the current polling surface is the SSOT this section verifies and `RenderableContent::mouse_cursor_icon` + `PaneSnapshot::mouse_cursor_icon` form a consistent read-through interface that matches how `cursor_shape`, `cwd`, and `title` are already consumed. **Section 16 (mouse protocols) is the architecturally correct home for a push-vs-poll decision** — it owns the cross-cutting "what mouse-facing state does the UI consume, and via what interface" question. 10.N's final checklist enumerates this as a handoff note to Section 16 so a future reviewer has the context without re-deriving it.

### F. OSC 1337 sub-dispatcher is LIVE; 10.7 is verification + catalog closure, not dispatcher creation

**CODE REALITY CHECK (2026-04-18):** The extensible OSC 1337 sub-dispatcher already landed at `crates/vte/src/ansi/dispatch/osc.rs:248-322`. `b"1337"` now routes to `dispatch_iterm2_osc1337` (line 253), which forwards `File=` to `Handler::iterm2_file` and routes the non-image sub-ops to dedicated trait methods: `iterm2_set_mark`, `iterm2_report_cell_size`, `iterm2_remote_host`, `iterm2_current_dir`, `iterm2_copy`, `iterm2_shell_integration_version`, `iterm2_set_user_var` (matched at osc.rs:285-321). Ownership was reassigned during the /review-plan pass: `plans/spec-conformance/catalog/iterm2.md` front-matter `owner_section` is `"01 (bootstrap), 10 (non-image), 14 (image)"`; `plans/spec-conformance/catalog/iterm2.md:15-21` marks the non-image rows `Owner: Section 10 (non-image)`; `plans/spec-conformance/section-14-iterm2-images.md:55` now reads "Section 10's OSC suite covered the non-image OSC 1337 variants; this section covers the image variants".

Subsection **10.7**'s remaining scope is therefore verification-side only: write the `Term` overrides for the seven non-image sub-op trait methods (so the dispatches actually mutate Term state), add spec_chain tests for each non-image row, and promote the catalog rows to `verified`. The dispatcher refactor itself is done; 10.7 does NOT re-land it. The RecordingHandler sync (per scope clarification added below in 10.N) must include every new `iterm2_*` trait method so spec_chain tests observe the dispatch correctly.

### G. OSC 52 ResponseToken is ALREADY LIVE (activation landed under effect-cutover §01.1 — Section 10 is a consumer, not an activator)

**CODE REALITY CHECK (2026-04-18):** Effect-cutover §01.1 already removed the `#[allow(dead_code, reason = "dormant during legacy phase")]` gate on `PaneIoThread::register_host_request_response` and wired the pipeline. Verified by:
- `oriterm_mux/src/pane/io_thread/response_poll/mod.rs:33` — `pub(super) fn register_host_request_response(&mut self, request: HostRequest)` — no `#[allow(dead_code)]` attribute.
- `oriterm_mux/src/pane/io_thread/effect_router/mod.rs:194` — live consumer: `self.register_host_request_response(HostRequest::ClipboardLoad { .. })` (feeds §10.2 directly).
- `oriterm_mux/src/pane/io_thread/effect_router/mod.rs:215` — live consumer: `self.register_host_request_response(HostRequest::ColorQuery { .. })` (feeds §10.8's OSC 10/11/12 queries and §10.6's OSC 104/110/111/112 queries).
- `oriterm_mux/src/pane/io_thread/response_poll/tests.rs` — nine GREEN tests exercise the full pipeline: `response_poll_idle_wake_unblocks_select`, `response_poll_token_requires_fulfillment`, `two_fulfills_one_wake_drains_both_tokens`, `fulfill_immediately_after_request_still_delivers`, `no_wake_signal_drops_late_fulfill`, `cancellation_detects_after_staging_drain`, `cancellation_fails_when_inner_token_cloned`, `multiple_fulfills_collapse_to_one_wake`, `fulfill_before_register_still_delivers`.
- `oriterm_core/src/effect/families/host_request/tests.rs` — GREEN: `response_token_rejects_double_fulfillment`, `response_token_fulfill_succeeds_once` (single-assignment pin).
- `crates/oriterm_test_support/src/session/pty_responder/` already auto-fulfills `HostRequest::ClipboardLoad` for embedded-session tests.

Section 10.2 is therefore a pure **consumer-side verification** task — it adds OSC-52-specific spec_chain tests asserting the correct `HostRequest::ClipboardLoad` is emitted from OSC 52 load bytes, and an OSC-52-specific end-to-end IO-thread test that drives the full pipeline from OSC 52 bytes → `register_host_request_response` → `poll_pending_responses` → `PtyEffect::Write`. The OSC 10/11/12 `ColorQuery` path is the reference consumer; OSC 52 is the second independent consumer we pin.

The original prose (`#[allow(dead_code)]` gate, "dormant during legacy phase", "Section 10 activates it") is historically accurate for pre-effect-cutover §01.1 state, but the code moved past it. DO NOT attempt to remove the gate (already gone) or wire `register_host_request_response` call sites (already wired).

### H. CWD SSOT — OSC 7 and OSC 133 must write the SAME field

Both OSC 7 (set current working directory) and some OSC 133 variants (when they carry `cwd=<path>` in the parameter string — per Final Term spec) update Term's CWD. The canonical home is `Term::set_cwd(Option<String>)` at `oriterm_core/src/term/shell_state/mod.rs:244-247`. Subsection 10.4's OSC 133 tests MUST go through `Term::set_cwd` (same canonical field as 10.8's OSC 7 tests) — any second CWD field is an SSOT LEAK.

### I. OSC 9 ambiguity — iTerm2 Growl vs Kitty notification protocol

`plans/spec-conformance/catalog/osc.md:52` attributes OSC 9 to iTerm2 notifications. Kitty later introduced OSC 99 for its expanded protocol. The interceptor at `oriterm_mux/src/shell_integration/interceptor.rs:124-128` distinguishes them via `NotificationSource::Osc9` / `::Osc99`. Subsection **10.3** pins this discriminator so future rewires (e.g. if Kitty extends OSC 99 with more fields) don't collapse OSC 9 and OSC 99 into one source.

### J. Vendored VTE crate discipline (`crates/vte/`)

Per `.claude/rules/crate-boundaries.md` §`crates/vte`: *"Vendored fork of the upstream `vte` crate ... Treat as an external dependency — do not add oriterm-specific types here. If a change is genuinely needed, open an issue upstream first and vendor the patch with a clear reason in the crate's README."* Section 10 adds multiple new `Handler` trait methods to `crates/vte/src/ansi/handler.rs` (10.0: `iterm2_set_mark`, `iterm2_remote_host`, `iterm2_current_dir`, `iterm2_copy`, `iterm2_report_cell_size`, `iterm2_set_user_var`, `iterm2_shell_integration_version`; 10.9: `set_x11_property`, `set_mouse_fg_color`, `set_mouse_bg_color`, `set_highlight_bg_color`, `set_highlight_fg_color`) and refactors the `b"1337"` dispatch arm into a sub-dispatcher. These changes are unambiguously oriterm-specific (the trait methods exist to let `Term` observe OSC dispatch events that the upstream dispatcher routes to `unhandled`). Consequence: Section 10 is a **vendored-patch change**, not an upstream contribution candidate. The 10.N completion checklist MUST include: (a) a `crates/vte/README.md` (create if missing) entry recording the Section-10 patch reason and scope ("oriterm-specific OSC 1337 sub-dispatcher + iterm2_* Handler hooks + OSC 3/5/13/14/17/19 handlers; upstream-first filing not applicable — oriterm-specific protocol coverage"), (b) a note in the crate's `lib.rs` `//!` doc noting the divergence from upstream (if not already present), and (c) explicit acknowledgment that these changes will need rebase work the next time upstream `vte` is synced. Do NOT try to upstream these methods to the `vte` crate — the upstream maintainers intentionally keep the Handler trait minimal, and oriterm-specific OSC 1337 non-image sub-ops are not in scope for upstream.

---

## Dependency boundaries

**Depends on:**
- **Section 03** (Effect Boundary Migration, `status: complete`) — typed-effect prerequisite: `oriterm_core::effect::{Effect, EffectSink, QueueingEffectSink}` + `HostEffect::DesktopNotification` + `HostRequest::ClipboardLoad` + `PendingResponse` infrastructure. Section 10 consumes these types; it does NOT activate anything Section 03 left dormant.
- **`plans/effect-cutover/section-01-migrate-mux-consumer.md`** (`status: complete`, commit `b89bdf84` 2026-04-18) — wired the live `response_poll` pipeline: removed the `#[allow(dead_code)]` gate on `register_host_request_response`, added the live call sites at `oriterm_mux/src/pane/io_thread/effect_router/mod.rs:194,215`, and deleted `LegacyEventSink`. Section 10 inherits a fully-live pipeline and adds OSC-52-specific consumer-side tests on top.
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
- `oriterm_mux/src/protocol/snapshot.rs` (add `mouse_cursor_icon: Option<u8>` to `PaneSnapshot` — daemon-mode wire transport for the embedded-path state landed above; wire-encoded as a `CursorIcon` index for cross-process stability)
- `oriterm_mux/src/server/snapshot.rs` (populate `PaneSnapshot::mouse_cursor_icon` from `RenderableContent::mouse_cursor_icon`; daemon-client `from_snapshot()` path decodes it so embedded and daemon clients render identical cursor-icon state)
- `crates/vte/src/ansi/handler.rs` (add `iterm2_set_mark`, `iterm2_remote_host`, `iterm2_current_dir`, `iterm2_copy`, `iterm2_report_cell_size`, `iterm2_set_user_var`, `iterm2_shell_integration_version` default methods)
- `crates/vte/src/ansi/dispatch/osc.rs` (refactor the `b"1337"` arm into a key=value sub-dispatcher)
- `crates/vte/src/ansi/dispatch/mod.rs` (**MODULE REGISTRATION**: add `#[cfg(test)] mod tests;` at the bottom of this file — required for the `dispatch/tests.rs` TDD test created in the OSC 1337 sub-dispatcher parse pin below. Without this registration, `tests.rs` will not be compiled. Currently `dispatch/mod.rs` has NO `#[cfg(test)] mod tests;` declaration, so the file must be added to the plan explicitly)
- `crates/oriterm_test_support/src/spec_chain/recording_handler.rs` (**REGISTRATION SYNC**: for every new `Handler::iterm2_*` method added to `crates/vte/src/ansi/handler.rs`, a matching delegate arm must be added here — same pattern as the existing `iterm2_file` arm at line 317. Missing arms mean the SpecHarness silently drops the new methods and spec_chain tests cannot observe them. This file is also updated in 10.7 when Term overrides land.)
- `oriterm_mux/src/pane/io_thread/response_poll/mod.rs` — **NO EDIT REQUIRED** (per scope clarification G, effect-cutover §01.1 already removed the `#[allow(dead_code)]` gate on `register_host_request_response` and wired it from `effect_router/mod.rs:194,215`). Listed here only as a reference file for subsection 10.2's OSC-52-specific tests, which sit on top of the already-live pipeline; NO gate-removal or call-site-wiring work remains for Section 10. The `response_poll.rs → response_poll/` directory-module conversion landed in effect-cutover §01.2.
- `oriterm_core/src/term/shell_state/mod.rs` (modify `finish_command` signature to accept `now: Option<Instant>` — Option A timing seam)
- `oriterm_mux/src/shell_integration/interceptor.rs` (update call sites of `finish_command` to pass `None`; this file is the caller of `Term::finish_command()`)
- `oriterm_mux/src/shell_integration/tests.rs` (extend existing sibling unit-test module — mux-intercepted OSC spec_chain tests for OSC 7, OSC 9/99/777, OSC 133/633 live here because only this file has `pub(crate)` access to `RawInterceptor`; integration tests in `oriterm_mux/tests/` are separate compilation units with NO `pub(crate)` visibility and MUST NOT contain `RawInterceptor`-using tests)

**Tests (written FIRST per `.claude/rules/tests.md` §TDD for Bugs — VERIFIED RED before implementation):**

- [x] **Failing test matrix written FIRST** — write TWO tests in `oriterm_mux/src/shell_integration/tests.rs` (sibling unit-test file) using the mux-internal `spec_chain_helper` (NOT `SpecHarness::feed()` — `oriterm_test_support` is NOT a dev-dependency of `oriterm_mux`). Test 1: run only the high-level `Processor::advance(&mut term, osc133_a_bytes)` without the `RawInterceptor` pass, and assert `term.prompt_state() == PromptState::None` (sequence was dropped). Test 2: run both parsers in production order via `spec_chain_helper::feed_mux_and_proc(&mut term, osc133_a_bytes)` and assert `term.prompt_state() == PromptState::PromptStart` (interceptor processed it). This RED→GREEN pair is the TDD proof that the mux interceptor is load-bearing. Both tests live in the sibling unit-test module and have `pub(crate)` access to `RawInterceptor` — NO `SpecHarness`, NO `oriterm_test_support` dev-dep required. Integration test home (`oriterm_mux/tests/`) MUST NOT be used for these tests because integration test crates are separate compilation units with no `pub(crate)` visibility into `oriterm_mux`.
- [x] **Renderable stub regression pin** — `observers/tests.rs` test that constructs a `RenderableExpectation { hyperlink_at: Some((row, col, "http://example.com")) }` against a `Term` whose cell at (row, col) has URI `"http://example.com"`. **Real red-first TDD discipline**: with the current stub (`observe_renderable` returns `RungResult::pass(rung)` unconditionally), this test ALSO passes — which means it does NOT provide a red→green signal. To get real TDD: write the test with an assertion that FAILS with the stub, e.g. `assert_ne!(rung_result, RungResult::pass(rung))` BEFORE the observer is completed (this fails green→red because the stub always passes). OR equivalently, the test checks that `observe_renderable` is NOT a stub by feeding a MISMATCHED URI and asserting the result is a FAILURE: `RenderableExpectation { hyperlink_at: Some((row, col, "http://wrong.com")) }` against a cell with URI `"http://right.com"` → test asserts `rung_result == RungResult::fail(rung, ...)`. With the stub this test FAILS (stub returns pass, assertion expects fail = RED). After observer implementation it PASSES (GREEN). THIS is the correct TDD red→green pair.
- [x] **Term mouse cursor icon pin** — test `term_set_mouse_cursor_icon_stores_icon` at `oriterm_core/src/term/tests.rs` that (i) starts `Term` with `mouse_cursor_icon == None`, (ii) calls `Handler::set_mouse_cursor_icon(&mut term, CursorIcon::Pointer)`, (iii) asserts `term.mouse_cursor_icon() == Some(CursorIcon::Pointer)`. Failing RED before the override is added.
- [x] **OSC 1337 sub-dispatcher parse pin** — test in `crates/vte/src/ansi/dispatch/tests.rs` (if missing, create) that feeds `\x1b]1337;SetMark\x1b\\` and asserts `Handler::iterm2_set_mark` was called. RED before the sub-dispatcher refactor lands.
- [x] **Response-poll activation pin** — OBE by effect-cutover §01.1. Verified 2026-04-18: `oriterm_mux/src/pane/io_thread/response_poll/tests.rs` already contains nine GREEN tests covering register → fulfill → poll → `PtyEffect::Write` (`response_poll_idle_wake_unblocks_select`, `response_poll_token_requires_fulfillment`, `two_fulfills_one_wake_drains_both_tokens`, `fulfill_immediately_after_request_still_delivers`, `no_wake_signal_drops_late_fulfill`, `cancellation_detects_after_staging_drain`, `cancellation_fails_when_inner_token_cloned`, `multiple_fulfills_collapse_to_one_wake`, `fulfill_before_register_still_delivers`). §10.0 does NOT duplicate these tests. §10.2 adds OSC-52-specific coverage ON TOP of this already-green pipeline.
- [x] **Injectable clock pin** — test `command_duration_uses_injected_now` that: (i) calls `Term::set_command_start(t0)` where `t0` is a fixed `Instant`, (ii) calls `term.finish_command(Some(t0 + Duration::from_millis(1500)))`, (iii) asserts the returned `Duration == 1500ms`. **Uses Option A seam only** (`fn finish_command(&mut self, now: Option<Instant>) -> Option<Duration>`) — do NOT add an `Arc<dyn Fn>` clock field to `Term` (breaks `#[derive(Debug)]` at `oriterm_core/src/term/mod.rs:113` and adds runtime overhead). RED until `finish_command` accepts the `now` parameter. No wall-clock reliance; the test is deterministic by construction. Landed in `oriterm_core/src/term/tests.rs` as `finish_command_uses_injected_now_for_deterministic_duration`.

**Implementation:**

- [x] Add a `spec_chain_helper` test-only module in `oriterm_mux/src/shell_integration/tests.rs` (existing sibling `#[cfg(test)] mod tests;` file) that constructs a `RawInterceptor + Term<QueueingEffectSink>` pair and runs both parsers in production order. **Canonical signature (IMPLEMENT THIS VERBATIM)**:
  ```rust
  // In oriterm_mux/src/shell_integration/tests.rs (or a sub-module within):
  //
  // spec_chain_helper encapsulates the production "interceptor FIRST, processor
  // SECOND" byte-feed order so downstream tests cannot accidentally reorder the
  // two passes (a silent false-green source).
  pub(super) struct SpecChainHelper<'a> {
      pub(super) term: &'a mut Term<QueueingEffectSink>,
      pub(super) effects: EffectCaptureSink,    // captures Effect transcript
  }
  pub(super) fn feed_mux_and_proc(
      term: &mut Term<QueueingEffectSink>,
      bytes: &[u8],
  ) {
      let mut interceptor = RawInterceptor::new(term);
      let mut raw_parser = vte::Parser::new();
      raw_parser.advance(&mut interceptor, bytes);
      let mut processor = vte::ansi::Processor::new();
      processor.advance(term, bytes);
  }
  ```
  Downstream tests (10.3/10.4/10.8 mux-intercepted OSC bytes) call `feed_mux_and_proc(&mut term, osc_bytes)` to exercise the full production byte path. **CRITICAL VISIBILITY NOTE**: Do NOT call `post_parse_housekeeping(evicted_before)` from this module — `post_parse_housekeeping` is a private method on `PaneIoThread` in `oriterm_mux/src/pane/io_thread/mod.rs` and is NOT accessible from `shell_integration/tests.rs` (they are sibling modules, not the same module or child/parent). The test helper in `shell_integration/tests.rs` does NOT need snapshot production housekeeping because it is testing interceptor behavior (state changes on `Term`), not snapshot visibility. If prompt-mark deferred side effects need verification in a specific test, call the public `Term` methods for deferred marking directly (`term.prompt_mark_pending()`, `term.mark_prompt_row()`, etc.) rather than routing through the private IO-thread method. The sibling unit-test module has `pub(crate)` access to `RawInterceptor` because it compiles as part of the `oriterm_mux` crate (unlike integration tests in `oriterm_mux/tests/`, which are separate crates with no `pub(crate)` visibility). **CRITICAL BOUNDARY**: Do NOT place tests requiring `RawInterceptor` in `oriterm_mux/tests/spec_chain/` — integration test crates cannot access `pub(crate)` items. Only tests that exercise purely-public APIs may live in `oriterm_mux/tests/`. `crates/oriterm_test_support` (`SpecHarness`) requires NO modification — no `mux_layer`, no `feed_with_mux()`, no new dependency. The `SpecHarness` remains mux-free. **Existing helpers to reuse**: `oriterm_mux/src/shell_integration/tests.rs:200-220` already defines `make_term()` (Term + QueueingEffectSink construction) and `intercept()` (single-pass interceptor feed). `spec_chain_helper` extends the pattern to a dual-pass feed; do NOT duplicate `make_term()` — reuse it.
- [x] Complete `observe_renderable` at `crates/oriterm_test_support/src/spec_chain/observers/renderable.rs:21-29` (currently returns `RungResult::pass(RungName::Renderable)` unconditionally with `_expected: RenderableExpectation` ignored). The completed function MUST read every non-`None` field on `RenderableExpectation` and assert against the corresponding field on `term.renderable_content()` (Rung 4 — snapshot, not live state). Any mismatch MUST return `RungResult::fail(rung, reason)` with a specific reason string so test failures are diagnosable without re-running with a debugger. The stub's module doc at `renderable.rs:1-19` is also updated to remove any "stub" language.
  - `cells: Option<&'static [(usize, usize, char)]>` — cell contents at specific positions (`&'static` slice, const-constructible, preserves `Copy`).
  - `hyperlink_at: Option<(usize, usize, &'static str)>` — assert cell's hyperlink URI matches (tuple of row, col, `&'static str` — const-constructible).
  - `cursor_position: Option<(usize, usize)>` — assert cursor lives where expected.
  - `cursor_shape: Option<CursorShape>` — assert `Term::cursor_shape()` matches.
  - `palette_index: Option<(usize, Rgb)>` — **RUNG 4 APEX (use snapshot, not live state)**: call `term.renderable_content()` to build a `RenderableContent`, then assert `rc.palette_snapshot[index] == [expected_rgb.r, expected_rgb.g, expected_rgb.b]`. Do NOT use `term.palette().color(index)` directly — that is the Rung 3 (live-state) accessor and bypasses the snapshot path that the renderer actually uses. `palette_snapshot` is populated by `fill_palette_snapshot` in `renderable_content_into()` (`oriterm_core/src/term/snapshot.rs:181-188`), so asserting against the snapshot verifies that the renderable path correctly captures the palette mutation. (If `mouse_cursor_icon` is also being added to `RenderableContent` per line 215 below, use `rc.mouse_cursor_icon` from the same `renderable_content()` call rather than `term.mouse_cursor_icon()` directly — both for consistency and to ensure the renderable path captures the field.)
  - `mouse_cursor_icon: Option<CursorIcon>` — **RUNG 4 APEX (use snapshot)**: assert `term.renderable_content().mouse_cursor_icon` (after `mouse_cursor_icon` is added to `RenderableContent` per line 215 below). Do NOT use `term.mouse_cursor_icon()` directly — use the snapshot field so Rung 4 validates the renderable path, not the live state rung. WHERE: new state landed in this subsection.
  - `damaged_lines: Option<&'static [usize]>` — assert renderable content reports the expected damage set (`&'static` slice, const-constructible, preserves `Copy`).
  - **Const-constructibility constraint**: ALL fields MUST be `Copy` and `const`-constructible — use `&'static` slices and `&'static str` instead of `Vec` and `String`. This preserves the `SpecScenario` const-constructible invariant (see `crates/oriterm_test_support/src/spec_chain/scenario.rs:12` module doc: *"Every field type is `const`-constructible. Slices use `&'static [u16]` / `&'static [u8]`. Expectation constructors are `const fn`."*). `Vec`/`String` fields ARE NOT permitted on `RenderableExpectation`.
- [x] Extend `RenderableExpectation` in `scenario.rs` with the fields above; keep existing callers compatible by making fields `Option` with `#[derive(Default)]`. Retain `#[derive(Copy, Clone, Debug, Default)]` — the new fields must all be `Copy`.
- [x] Add `mouse_cursor_icon: Option<CursorIcon>` to `Term<S>`; initialize to `None` in `Term::new()`; add `Term::mouse_cursor_icon(&self)` accessor + `Term::set_mouse_cursor_icon(&mut self, icon: Option<CursorIcon>)` mutator (per `.claude/rules/impl-hygiene.md` §SSOT — canonical home for this knowledge is `Term`).
- [x] Override `Handler::set_mouse_cursor_icon` on `Term` in `oriterm_core/src/term/handler/mod.rs` to call `Term::set_mouse_cursor_icon(Some(icon))`. WHERE: add next to the other `Handler` trait methods, grouped with cursor-shape handlers.
- [x] Expose `mouse_cursor_icon` on `RenderableContent` (`oriterm_core/src/term/renderable/mod.rs`) so the rendering consumer can query it. Include it in `renderable_content_into()` writeback (NO allocation — the field is `Option<CursorIcon>`, which is `Copy`).
- [x] **Daemon-mode `PaneSnapshot` transport** — add `mouse_cursor_icon: Option<u8>` to `PaneSnapshot` at `oriterm_mux/src/protocol/snapshot.rs:160`, populate it in `oriterm_mux/src/server/snapshot.rs` from `RenderableContent::mouse_cursor_icon` (encode `CursorIcon` → `u8` index using a stable project-owned enum-to-index mapping matching the `OSC22_KNOWN_ICONS` slice referenced in 10.5), and decode it in the daemon-client's `from_snapshot()` path. This MUST land in 10.0 alongside the `Term`/`RenderableContent` field additions so embedded and daemon paths stay in sync — deferring to 10.5 or later creates a GAP where embedded and daemon clients render different cursor icons from the same terminal session (per `.claude/rules/impl-hygiene.md` §Gap Detection).
- [x] Refactor `crates/vte/src/ansi/dispatch/osc.rs:248-254`:
  ```rust
  b"1337" => {
      if params.len() < 2 { return unhandled(params); }
      dispatch_iterm2_osc1337(handler, &params[1..]);
  },
  ```
  where `dispatch_iterm2_osc1337` is a new private function in the same file that parses the first parameter as `key[=value]` and routes to the appropriate `Handler::iterm2_*` method. The existing `File=` case goes through this dispatcher — it calls `handler.iterm2_file(&params[1..])` when the key is `File`. Preserves current behavior, adds extensibility.
- [x] Add default no-op methods to the `Handler` trait in `crates/vte/src/ansi/handler.rs` for every new sub-op: `iterm2_set_mark`, `iterm2_remote_host(path: &[u8])`, `iterm2_current_dir(path: &[u8])`, `iterm2_copy(data: &[u8])`, `iterm2_report_cell_size()`, `iterm2_set_user_var(name: &[u8], value: &[u8])`, `iterm2_shell_integration_version(version: &[u8])`. Defaults are empty bodies (drop semantics) — 10.7 overrides each on `Term`.
- [x] **Vendored-patch breadcrumbs (per scope clarification J)** — every oriterm-added edit inside `crates/vte/` MUST carry a line-local `// VENDORED PATCH (oriterm): <reason>` comment on the first added line (trait method, dispatch arm, or helper function). This makes the patch surface `grep`-able so the next upstream `vte` rebase can enumerate the divergence deterministically. Apply to every new `iterm2_*` method declaration in `crates/vte/src/ansi/handler.rs`, the new `dispatch_iterm2_osc1337` helper in `crates/vte/src/ansi/dispatch/osc.rs`, and any future 10.9 Handler methods (`set_x11_property`, `set_mouse_fg_color`, `set_mouse_bg_color`, `set_highlight_bg_color`, `set_highlight_fg_color`). Validation: `grep -rn 'VENDORED PATCH (oriterm)' crates/vte/` returns a line for each new Handler method + the sub-dispatcher helper. Additionally, 10.7 adds a `// TODO: Section 14 owns image sub-ops` breadcrumb in the `unhandled` arm of the sub-dispatcher (at the location that `File=` falls through to, so a reader searching for image routing sees the cross-section pointer immediately).
- [x] **Response-poll activation requires EffectSink migration (GAP) — OBE 2026-04-18.** Effect-cutover §01.1 landed the `QueueingEffectSink` migration AND removed the `#[allow(dead_code)]` gate on `register_host_request_response`. Effect-cutover §01.2 landed the idle-wake `select!` arm (see `response_poll_idle_wake_unblocks_select` at `response_poll/tests.rs:103`) so fulfilled tokens wake the outer loop even during idle periods. `LegacyEventSink` has been deleted from `oriterm_core` (the only remaining reference is in `oriterm_core/tests/effect_cutover_deletion_pins.rs` which pins the type's absence). Section 10 does NOT re-do any of this work. The 10.0 checklist items that assumed "dead-code gate to be removed", "sink migration to be coordinated", or "idle-wake channel to be added" are all marked [x] with cross-refs to the landing commits. 10.2's tests sit on top of the already-live pipeline.
- [x] Make `HostEffect::CommandComplete { duration }` deterministic for testing by correcting the timing seam. **TIMING SEAM ANALYSIS (verified against code):** The duration is computed in `oriterm_core/src/term/shell_state/mod.rs:205-210`: `fn finish_command(&mut self) -> Option<Duration> { let start = self.command_start.take()?; let duration = start.elapsed(); ... }` — the call is `start.elapsed()` on an `Instant`, NOT `Instant::now()` at the interceptor. Two valid approaches to make this deterministic: **(A, preferred)** refactor `finish_command()` to accept a `now: Option<Instant>` parameter: `fn finish_command(&mut self, now: Option<Instant>) -> Option<Duration>`, computing `now.unwrap_or_else(Instant::now).duration_since(start)`. Production callers pass `None`; tests pass `Some(injected_instant)`. No `Arc<dyn Fn>` field needed, no `Debug` issue. **(B, alternative)** add a `clock: Option<Arc<dyn Fn() -> Instant + Send + Sync>>` field to `Term` — but this requires a `ClockFn` newtype wrapper with manual `Debug` impl (see the `#[derive(Debug)]` constraint at `oriterm_core/src/term/mod.rs:113`). Option A is preferred because it avoids the `Arc<dyn Fn>` / `Debug` complication entirely and the test-injection is at the exact right seam. **INCORRECT alternative (do NOT do this):** replacing `Instant::now()` at `oriterm_mux/src/shell_integration/interceptor.rs` — the interceptor calls `Term::set_command_start(Instant::now())` to SET the start time, but the DURATION is computed in `Term::finish_command()` via `start.elapsed()`. The interceptor is not the seam where the duration is measured.

**Validation:**

- [x] All five TDD matrix tests transition RED → GREEN.
- [x] The OSC 133;A scenario routed through `SpecHarness::feed()` still fails (proves the high-level processor really drops OSC 133, not just our test setup).
- [x] The mux-layer test in `oriterm_mux/src/shell_integration/tests.rs` that runs both parsers in production order passes (proves the mux interceptor is load-bearing). The `SpecHarness` in `oriterm_core/tests/spec_chain/` has no `feed_with_mux()` method — this validation lives in the sibling unit-test module, not in `SpecHarness` and NOT in `oriterm_mux/tests/`.
- [x] `renderable.rs` NO LONGER contains `RungResult::pass(RungName::Renderable)` as the only return — grep for the string `"Stub: always passes"` returns zero matches.
- [x] `grep -rn '#\[allow(dead_code, reason = \"dormant during legacy phase'` in `oriterm_mux/` returns zero matches (the gate was removed by effect-cutover §01.1; verified 2026-04-18).
- [x] `./build-all.sh` + `./test-all.sh` + `./clippy-all.sh` green *(on the partial state below)*.
- [x] **TPR checkpoint 1** — `/tpr-review` covering 10.0 only. Harness API MUST stabilize here before downstream subsections build on it. Ran 2026-04-18/19, exit_reason=user_accepted_at_iter_cap_reached, 5 findings fixed inline across commits 9604351e/2ba96455/7e2d9a2d, zero outstanding.

**Implementation notes (2026-04-18) — §10.0 partial landing:**

Five of the six implementation bullets landed on 2026-04-18: the OSC 1337 sub-dispatcher refactor + 7 new `Handler::iterm2_*` methods (`crates/vte`), the `Term::mouse_cursor_icon` field + `Handler::set_mouse_cursor_icon` override + `RenderableContent` snapshot writeback (`oriterm_core`), the Option A `finish_command(now: Option<Instant>)` seam (`oriterm_core` + `oriterm_mux` caller), and the daemon-mode `PaneSnapshot::mouse_cursor_icon` transport with a project-owned `OSC22_KNOWN_ICONS` index table (`oriterm_mux::protocol::snapshot` + `oriterm_mux::server::snapshot` + `oriterm::gpu::extract::from_snapshot`). 14 TDD tests GREEN: 9 OSC 1337 dispatch tests in `crates/vte/src/ansi/dispatch/tests.rs`, 3 Term mouse-cursor-icon tests + 2 injectable-clock tests in `oriterm_core/src/term/tests.rs`. All three scripts (`./build-all.sh`, `./test-all.sh`, `./clippy-all.sh`) green.

**Implementation notes update (2026-04-18 Step 5 editor pass):** effect-cutover §01 landed fully (commit `b89bdf84 docs(effect-cutover): close §01.N + complete the entire plan`). The previously-blocked items are now either (a) OBE — landed as part of effect-cutover itself — or (b) unblocked and ready to resume:

1. **Response-poll activation** — **OBE.** Gate removed, pipeline wired (`effect_router/mod.rs:194,215`), nine response_poll tests green, idle-wake channel landed. Section 10 does NOT duplicate this work.
2. **`spec_chain_helper` + load-bearing OSC 133;A test** — **UNBLOCKED.** Still needs implementing in 10.0. Now unblocked because `oriterm_mux` uses `QueueingEffectSink` in production, so the sibling unit-test module can construct `Term<QueueingEffectSink>` + `RawInterceptor` + `Processor` without contending with the vanished `LegacyEventSink`. See `oriterm_mux/src/shell_integration/tests.rs:196-205` for the production `make_term()` / `intercept()` helpers that already demonstrate the pattern; `spec_chain_helper` is a thin wrapper that feeds BOTH `raw_parser.advance(&mut interceptor, bytes)` AND `processor.advance(&mut term, bytes)` in production order.
3. **`observe_renderable` completion** — **UNBLOCKED.** Stub still returns `RungResult::pass(RungName::Renderable)` unconditionally at `observers/renderable.rs:21-29`. Ready to implement.
4. **`RenderableExpectation` extension** — **UNBLOCKED.** `scenario.rs`'s unit-struct (`pub struct RenderableExpectation;` at line 341) needs the 7 const-constructible fields (`cells`, `hyperlink_at`, `cursor_position`, `cursor_shape`, `palette_index`, `mouse_cursor_icon`, `damaged_lines`).

All four items are now in-progress work for Section 10; none are blocked on external plans.

**Implementation notes update (2026-04-19 — §10.0 complete):** All remaining implementation bullets landed across three commits: `9604351e` (initial close-out: spec_chain_helper + TDD pair, observe_renderable with per-field check_* helpers, RenderableExpectation 7-field extension, stub-regression pin matrix, vendored-patch breadcrumbs), `2ba96455` (RecordingHandler registration-sync for all 7 oriterm-added Handler::iterm2_* methods plus initial SpecHarness pin test), and `7e2d9a2d` (table-driven sync test covering all 7 OSC 1337 non-image sub-ops with matrix-completeness count assertion plus decorative-banner cleanup across observers/tests.rs and shell_integration/tests.rs). TPR checkpoint 1 ran 3 rounds: codex raised 4 findings (medium + medium + high + medium), gemini raised 1 (low) — all 5 verified against code, fixed inline, user accepted clean at iter-cap. `./build-all.sh` + `./test-all.sh` + `./clippy-all.sh` all green on host + Windows cross-compile target.

---

## 10.1 OSC 8 hyperlinks

**Files:**
- `oriterm_core/tests/spec_chain/osc/hyperlinks.rs` (new — registered as `mod hyperlinks;` inside `spec_chain/osc/mod.rs`)
- `oriterm_core/tests/spec_chain/osc/mod.rs` (new module aggregator)
- `oriterm_core/tests/spec_chain/main.rs` (add `mod osc;`)
- Catalog update: `plans/spec-conformance/catalog/osc.md` (row OSC-8)

**Tests (TDD — RED first):**

- [x] Spec_chain test `osc8_basic_attach` — feed `\x1b]8;;https://example.com\x1b\\Hello\x1b]8;;\x1b\\` (set URI, text, clear URI). Assert cells 0..5 of current row carry `hyperlink_uri == Some("https://example.com")` using `hyperlink_at: Some((row, 0, "https://example.com"))` in `RenderableExpectation`. For the "no hyperlink after clear" assertion, use the **state rung apex** directly (`term.grid()[Line(row as i32)][Column(5)].hyperlink() == None`) — `RenderableExpectation` does not have a negative-hyperlink field (`hyperlink_absent_at`); the schema only supports `hyperlink_at` as a positive match. The state-rung check is the canonical pin here. A `hyperlink_absent_at: Option<(usize, usize)>` schema addition to `RenderableExpectation` is NOT required for this test — the state-rung assertion is sufficient and 10.0's schema extensions stay scoped to the positive-match fields enumerated in the 10.0 Implementation block.
- [x] `osc8_with_id` — feed `\x1b]8;id=foo;https://example.com\x1b\\X\x1b]8;;\x1b\\`. Assert cell 0 has the URI. **Important apex constraint:** `RenderableCell` at `oriterm_core/src/term/renderable/mod.rs` only carries `hyperlink_uri: Option<String>` — the hyperlink `id` is NOT exposed on the renderable surface. To verify the `id` is preserved in cell metadata, use the **state rung apex** (read `term.grid()[Line(row as i32)][Column(col)].hyperlink()` where `Line` is `oriterm_core::index::Line` and `Column` is `oriterm_core::index::Column` — Grid implements `Index<Line>` returning `&Row`, and `Row` implements `Index<Column>` returning `&Cell`; do NOT use bare `grid[row][col]` with `usize` indices, which fails to compile because `Grid` only accepts `Line` and `Row` only accepts `Column`) rather than `observe_renderable`. Verify that `cell.hyperlink().unwrap().id == Some("foo")` at the state rung. Then test that two separate attach/clear cycles with the same `id` both carry `id == Some("foo")` (confirming `id` does not get cleared between cycles). The renderable rung assertion covers only the URI presence; the state rung assertion covers the `id`.
- [x] `osc8_survives_reflow` — place hyperlinked text at row 0. Resize grid from 80 to 40 columns. Assert the wrapped cells (now spread across row 0 and row 1) ALL carry the same URI. (This catches the reflow-drops-metadata regression pattern from the alacritty / wezterm code history.)
- [x] `osc8_survives_scrollback` — place hyperlinked text, then feed enough newlines that the row scrolls into `Grid::scrollback`. Assert the scrollback row still carries the URI on every cell. **Uses the STATE RUNG** via `ScrollbackBuffer::get(index)` — `ScrollbackBuffer` does NOT implement `Index` (no `[row]` bracket syntax); use `term.grid().scrollback().get(idx).unwrap()` to get a `&Row`, then index by `Column` via `row[Column(col)]` (Row implements `Index<Column>`, not `Index<usize>`). `RenderableContent` does NOT expose individual scrollback rows (it has `scrollback_len: usize` for the count but no per-cell scrollback access). Do NOT use `observe_renderable` for this assertion — only viewport cells are visible through that rung.
- [x] `osc8_terminator_cancels_attachment` — feed text, `OSC 8 ; ; uri ST`, text-A, `OSC 8 ; ; ST`, text-B. Assert text-B cells have `hyperlink_uri == None` (the empty URI terminates the attachment).
- [x] `osc8_malformed_uri_dropped` — feed `\x1b]8;; BROKEN URI WITH SPACES \x1b\\X\x1b]8;;\x1b\\` and assert the cell carries the URI as-is (whitespace is not syntactically restricted in OSC 8 params — the terminal does not validate; it records). Negative pin: feed truncated `\x1b]8;;\x1b` (no ST) and assert no URI is attached (parser aborts on timeout / sequence boundary).
- [x] `osc8_alt_screen_toggle_clears` — enter alt screen, attach hyperlink, leave alt screen. Assert primary screen cells are unaffected (alt-screen hyperlinks do NOT bleed).
- [x] **Semantic pin** — `osc8_renderable_observer_not_stub` — scenario asserts `hyperlink_at: Some((0, 0, "WRONG_URI"))` against an actual URI of `"http://example.com"`. Must FAIL. If it passes, the renderable observer has regressed to the 10.0 stub.

**Implementation prerequisites (verified from catalog/osc.md):**

OSC 8 dispatch at `crates/vte/src/ansi/dispatch/osc.rs` (`b"8"` arm) already routes to `handler.set_hyperlink()`; `Term::set_hyperlink` → `Term::osc_set_hyperlink` at `oriterm_core/src/term/handler/osc.rs` already attaches URI to cells. No new dispatch work. Section 10.1 is pure verification.

**Catalog update:**

- [x] Promote OSC-8 in `plans/spec-conformance/catalog/osc.md` from `implemented-unverified` → `verified`. Fill `Test chain` cell with `parser:pass dispatch:pass state:pass` + citation of `oriterm_core/tests/spec_chain/osc/hyperlinks.rs::{osc8_basic_attach, osc8_with_id, osc8_survives_reflow, osc8_survives_scrollback, osc8_terminator_cancels_attachment, osc8_malformed_uri_dropped, osc8_alt_screen_toggle_clears}`. (Token schema: `pass` / `fail` / `pending` / `missing` — NOT `passed`.)

**Validation:**

- [x] All 8 tests pass (7 behavioral + 1 semantic pin).
- [x] `observe_renderable` is exercised with a real expectation in every test (no test relies on rung pass-through).
- [x] `./build-all.sh` + `./test-all.sh` + `./clippy-all.sh` green after 10.1's changes (per CLAUDE.md "run these after every change"; section-level `/tpr-review` + `/impl-hygiene-review` are gated at 10.N's Final Verification).

---

## 10.2 OSC 52 clipboard

**Files:**
- `oriterm_core/tests/spec_chain/osc/clipboard.rs` (new — OSC 52 spec_chain tests; HostRequest::ClipboardLoad / HostEffect::ClipboardStore apex assertions)
- `oriterm_mux/src/pane/io_thread/response_poll/tests.rs` (existing — ADD `osc52_register_poll_roundtrip` bytes-in-to-pty-write OSC-52-specific IO-thread test alongside the already-green nine tests; the directory-module conversion landed in effect-cutover §01.2)
- `crates/oriterm_test_support/src/session/pty_responder/mod.rs` (existing — already auto-fulfills `HostRequest::ClipboardLoad`; referenced as the canonical fulfillment pattern the OSC 52 IO-thread test mirrors)
- Catalog update: `plans/spec-conformance/catalog/osc.md` (rows OSC-52-STORE, OSC-52-LOAD)

**Tests (TDD — RED first):**

- [x] `osc52_store_clipboard_c` — feed `\x1b]52;c;SGVsbG8=\x1b\\`, assert `Effect::HostRequest(HostRequest::ClipboardLoad { .. })` is NOT emitted (this is a store, not a load), and assert the Effect-side variant is `Effect::Host(HostEffect::ClipboardStore { selection: ClipboardSelection::Clipboard, data: "Hello".into() })` — the exact field name is `data: String` (NOT `text`), as confirmed at `oriterm_core/src/effect/families/host.rs:36`. The public re-export path is `oriterm_core::effect::{HostEffect, ClipboardSelection}` (NOT the private `oriterm_core::effect::families::host` path — use the public API). **No `LegacyEventSink` assertion here** — spec_chain tests use `QueueingEffectSink`; asserting on `Event::ClipboardStore` via `LegacyEventSink` would test the wrong sink path.
- [x] `osc52_store_clipboard_s` — same shape, `s` (selection) clipboard character, assert `selection: ClipboardSelection::Select` (NOT `Selection` — the enum variant at `oriterm_core/src/effect/families/host.rs:114` is `Select`, not `Selection`), `data: <decoded>`.
- [x] `osc52_store_clipboard_p` — `p` (primary) clipboard character; assert `selection: ClipboardSelection::Primary`, `data: <decoded>`.
- [x] `osc52_store_clipboard_q` — NEGATIVE PIN: `q` is NOT a valid `ClipboardSelection` variant (`ClipboardSelection` at `oriterm_core/src/effect/families/host.rs:108-115` has only `Clipboard`, `Primary`, `Select`). Feed `\x1b]52;q;SGVsbG8=\x1b\\` and assert NO `HostEffect::ClipboardStore` is emitted (the OSC 52 handler must drop unknown clipboard characters). This is a negative pin, NOT a positive test for `q` support. The success criteria (frontmatter line 14) that claims `both 'c' / 's' / 'p' / 'q' clipboard characters` is corrected by this test: `q` is tested only as a DROPPED/invalid character, not as a supported selection type.
- [x] `osc52_load_request_fires_hostrequest` — feed `\x1b]52;c;?\x1b\\`, assert `Effect::HostRequest(HostRequest::ClipboardLoad { selection: Clipboard, clipboard_char: b'c', terminator: "\x1b\\", reply: <ResponseToken> })` is on the transcript. This is the SPEC-CHAIN assertion scope boundary — the spec_chain harness asserts the HostRequest was emitted; it does NOT simulate the IO thread's polling loop (that lives in `oriterm_mux::PaneIoThread`, which is a separate crate layer). The ResponseToken fulfillment → PtyEffect::Write round-trip is tested in `oriterm_mux/src/pane/io_thread/response_poll/tests.rs` (listed in the Files block), NOT in the spec_chain test. No `harness.poll_pending_responses()` helper is added to `SpecHarness` — doing so would force `oriterm_test_support` to depend on `oriterm_mux`'s internal `PaneIoThread`, which violates the crate boundary (see `.claude/rules/crate-boundaries.md` §crates/oriterm_test_support).
- [x] `osc52_register_poll_roundtrip` — new OSC-52-specific bytes-in-to-pty-write test in `oriterm_mux/src/pane/io_thread/response_poll/tests.rs` (sibling to the already-green nine tests that use `HostRequest::ColorQuery` as their driver). Spawn a `PaneIoThread<QueueingEffectSink>`, feed OSC 52 load bytes `\x1b]52;c;?\x1b\\` into the pane's byte-rx, wait for `HostRequest::ClipboardLoad` on `mux_rx` via the existing `await_host_clipboard_load` helper at `response_poll/tests.rs:58`, call `reply.fulfill("example-text".into())`, then wait for the corresponding `PtyEffect::Write` containing `format_clipboard_reply("example-text", b'c', "\x1b\\")`. Uses `format_clipboard_reply` from `oriterm_core/src/effect/families/host_request/mod.rs:110` — DO NOT re-implement the reply format inline (LEAK). This test is the OSC-52 counterpart to the already-green `HostRequest::ColorQuery`-driven tests; it proves the OSC-52 dispatch → effect-router → register → poll → PTY-write chain works end-to-end on the actual OSC 52 wire bytes, not just on a synthetic `HostRequest::ClipboardLoad` insertion.
- [x] **Semantic pin — token requires fulfillment before reply emitted** — `response_poll_token_requires_fulfillment` ALREADY GREEN at `oriterm_mux/src/pane/io_thread/response_poll/tests.rs:138` (landed in effect-cutover §01.1). §10.2 cites it as a regression guard; adds no duplicate test.
- [x] **Negative pin — `ResponseToken::fulfill` is single-assignment (first-write-wins)** — `response_token_rejects_double_fulfillment` + `response_token_fulfill_succeeds_once` ALREADY GREEN at `oriterm_core/src/effect/families/host_request/tests.rs:29,42` (landed in effect-cutover §01.2, commit `c5a21ab5`). §10.2 cites them as regression guards; adds no duplicate test.
- [x] **Embedded-backend fulfillment pin (blind-spot #6 remediation)** — `osc52_embedded_backend_fulfills_via_session_pty_responder` — in `crates/oriterm_test_support/src/session/pty_responder/tests.rs` (sibling unit-test file), construct a `PtyResponder` (already the canonical embedded-path fulfillment adapter per `crates/oriterm_test_support/src/session/pty_responder/mod.rs:153`), push an `Effect::HostRequest(HostRequest::ClipboardLoad { .. })` originating from OSC 52 bytes, assert the responder auto-fulfills the token with the embedded clipboard contents and emits `PtyEffect::Write` with the correct base64-encoded reply. This pins the embedded-session consumer seam so OSC 52 round-trip is verified in BOTH the daemon path (`PaneIoThread`) AND the embedded path (`PtyResponder`) — SSOT for OSC 52 ResponseToken fulfillment cannot silently drift between the two consumers.
- [x] `osc52_load_with_s_and_p_selections` — load with `s` and `p` characters; assert the correct `ClipboardSelection` in the `HostRequest`.
- [x] `osc52_store_invalid_base64_dropped` — feed `\x1b]52;c;!!!invalid-base64!!!\x1b\\`, assert no `HostEffect::ClipboardStore` is emitted (store path rejects invalid base64; confirm behavior matches `oriterm_core/src/term/handler/tests/osc.rs::osc52_clipboard_load` pattern) OR assert a specific error/drop behavior — whichever the current dispatcher at `oriterm_core/src/term/handler/osc.rs::osc_clipboard_store` produces. If the current behavior is "accept garbage and store it", file `/add-bug` and document the observed behavior as the current catalog deviation.

**Catalog update:**

- [x] OSC-52-STORE in `plans/spec-conformance/catalog/osc.md` → `verified` with citations for `c`, `s`, `p` clipboard characters (store); `q` documented as not supported (`ClipboardSelection` has no `q` variant — verified at `oriterm_core/src/effect/families/host.rs:108-115`). **CATALOG METADATA UPDATE REQUIRED**: The current `Implementation` cell says "Emits `Event::ClipboardStore`" and the `Notes` wording is stale. Rewrite `Implementation` to cite `HostEffect::ClipboardStore { selection, data }` via `QueueingEffectSink` (public path: `oriterm_core::effect::HostEffect`); rewrite `Notes` to remove the old `Event::` wording. The `Apex layer` value `effect-clipboard` is already schema-correct (per `plans/spec-conformance/00-overview.md:820` canonical `ApexLayer` enum) — do NOT change it to `effect-host` (not a valid schema value). Do NOT mark the row `verified` while the Implementation cell still says `Event::ClipboardStore` — a DRIFT between the catalog and the actual code.
- [x] OSC-52-LOAD in `plans/spec-conformance/catalog/osc.md` → `verified` with citation of the ResponseToken round-trip test. **CATALOG METADATA UPDATE REQUIRED**: The current `Implementation` cell says "Emits `Event::ClipboardLoad` with a response-formatting closure" and the `Notes` wording is stale. Rewrite `Implementation` to cite `HostRequest::ClipboardLoad { selection, reply: ResponseToken }` via `QueueingEffectSink`; rewrite `Notes` to describe the `ResponseToken` fulfillment → `PtyEffect::Write` path via `PaneIoThread::poll_pending_responses`. Do NOT mark the row `verified` while the Implementation cell still references the old `Event::ClipboardLoad` closure path.
- [x] `plans/spec-conformance/catalog/shell-integration.md` row SHINT-OSC-9-NOTIFY (cross-reference) remains pointing at `osc.md::OSC-9` (handled in 10.3).

**Validation:**

- [x] All 7 spec_chain tests pass (4 store tests + `osc52_load_request_fires_hostrequest` + `osc52_load_with_s_and_p_selections` + `osc52_store_invalid_base64_dropped`).
- [x] `osc52_register_poll_roundtrip` green in `oriterm_mux/src/pane/io_thread/response_poll/tests.rs` (NEW in §10.2 — the OSC-52-bytes-in-to-pty-write test). Pre-existing `response_poll_token_requires_fulfillment` + the eight other response_poll tests remain green (regression guards; landed under effect-cutover §01.1).
- [x] `response_token_rejects_double_fulfillment` + `response_token_fulfill_succeeds_once` remain green at `oriterm_core/src/effect/families/host_request/tests.rs:29,42` (regression guards; landed under effect-cutover §01.2).
- [x] `osc52_embedded_backend_fulfills_via_session_pty_responder` green in `crates/oriterm_test_support/src/session/pty_responder/tests.rs` — embedded-path OSC 52 round-trip verification.
- [x] **IO-thread regression scan** — §10.2 adds an OSC-52-specific register-poll-fulfill pathway through `PaneIoThread` on top of the already-live pipeline. Explicit re-runs to prevent IO-thread regressions from the added wiring: `timeout 150 cargo test -p oriterm_mux pane::io_thread::tests`, `timeout 150 cargo test -p oriterm_mux pane::io_thread::response_poll::tests`, `timeout 150 cargo test -p oriterm_mux pane::io_thread::effect_router::tests`. Any failure in these modules is a regression introduced by 10.2 and blocks the subsection from marking complete, independent of the whole-workspace `./test-all.sh` result.
- [x] `oriterm_core/tests/teseq/osc.rs::osc_clipboard` regression test unchanged and green.
- [x] `./build-all.sh` + `./test-all.sh` + `./clippy-all.sh` green after 10.2's changes (per CLAUDE.md "run these after every change"; section-level `/tpr-review` + `/impl-hygiene-review` are gated at 10.N's Final Verification).

---

## 10.3 OSC 9 / 99 / 777 desktop notifications

**Files:**
- `oriterm_mux/src/shell_integration/tests.rs` (extend sibling unit-test module — OSC 9/99/777 tests live here because `RawInterceptor` is `pub(crate)` and only accessible from sibling unit tests, NOT from `oriterm_mux/tests/` integration tests)
- Catalog updates: `plans/spec-conformance/catalog/osc.md` — OSC-9 and OSC-777 rows already exist (both marked `missing`; promote to `verified`); OSC-99 is NOT yet a catalog row and must be added as a new row (status `verified-with-deviation`, per `osc99_metadata_body_form_pins_current_behavior` below — the Kitty two-parameter-form metadata field is dropped by the current interceptor). Also update `plans/spec-conformance/catalog/shell-integration.md` rows SHINT-OSC-9-NOTIFY, SHINT-OSC-777-NOTIFY.

**Tests (in `oriterm_mux/src/shell_integration/tests.rs` — these OSCs route through the RawInterceptor, NOT the high-level processor; must NOT be placed in `oriterm_core/tests/spec_chain/osc/` and must NOT be placed in `oriterm_mux/tests/` integration tests which have no `pub(crate)` access to `RawInterceptor`):**

- [x] `osc9_simple_body_fires_notification` — feed `\x1b]9;Build complete\x1b\\`, assert `Effect::Host(HostEffect::DesktopNotification { source: NotificationSource::Osc9, title: "", body: "Build complete" })`. OSC 9 has no title field (Growl-style).
- [x] `osc99_default_payload_routes_to_title` — feed Kitty-conformant `\x1b]99;;kitty payload\x1b\\` (two semicolons mandatory per Kitty's OSC 99 spec — see `~/projects/reference_repos/console_repos/kitty/docs/desktop-notifications.rst` line 18 + 25-26), assert `source: Osc99, title: "kitty payload", body: ""`. Per Kitty spec line 472-474 the default `p=title` routes the payload at `params[2]` into the `title` field (NOT `body`). 10.3 pins the source discriminator so a future refactor cannot collapse the OSC 9 / OSC 99 arms in `handle_notification_simple`.
- [x] `osc99_metadata_form_default_p_routes_payload_to_title` — feed `\x1b]99;i=1:t=info;hello\x1b\\` (Kitty two-parameter form with metadata that does NOT include a `p=` key); assert `source: Osc99, title: "hello", body: ""`. Default `p=title` still applies; metadata keys (`i`, `t`) are silently discarded per the catalog deviation.
- [x] `osc99_p_body_routes_payload_to_body` — feed `\x1b]99;p=body;hello\x1b\\`; assert `source: Osc99, title: "", body: "hello"`. Pins the only metadata key the implementation actually parses (`p=`).
- [x] `osc99_empty_payload_drops_notification` — feed `\x1b]99;;\x1b\\` (Kitty-conformant but empty payload); assert NO notification is emitted. Per Kitty spec "A notification with not title and no body is ignored."
- [x] `osc99_unsupported_payload_kind_drops_notification` — feed `\x1b]99;p=close;something\x1b\\` (or `p=icon|?|alive|buttons|<unknown>`); assert NO notification is emitted. Per Kitty spec "Terminal emulators should ignore payloads of unknown type to allow for future expansion of this protocol."

(Implementation history: TPR checkpoint 2 round 0 → round 1 surfaced two layered correctness gaps. Round 0 caught `handle_notification_simple` reading body from `params[1]` for both OSC 9 and OSC 99 — broken for Kitty-conformant `OSC 99 ;; body ST`. Round 1 caught the deeper issue: even after splitting `body_idx` per source, OSC 99 was hardcoding payload into `body` instead of honouring Kitty's default `p=title` semantics. The §10.3 fix landed both layers: introduced `parse_osc99_payload_kind` to honour the `p=` metadata key (`title` default, `body`, or drop on unknown), routes payload into the correct field, and drops empty / unknown-type notifications per the spec. Catalog OSC-99 deviation now correctly scoped to "metadata keys other than `p` are opaque".)
- [x] `osc777_notify_title_body` — feed `\x1b]777;notify;Build;completed successfully\x1b\\`, assert `source: NotificationSource::Osc777, title: "Build", body: "completed successfully"`.
- [x] `osc777_non_notify_action_dropped` — feed `\x1b]777;BAD_ACTION;title;body\x1b\\`, assert NO notification effect is emitted (the interceptor at line 143-145 filters non-`notify` actions).
- [x] `osc9_empty_body` — feed `\x1b]9;\x1b\\`, assert `body == ""` and notification is still emitted (matches `handle_notification_simple` which accepts empty body).
- [x] `osc777_missing_title` — feed `\x1b]777;notify;;body-only\x1b\\`, assert `title == "", body == "body-only"`.
- [x] **Semantic pin** — `osc9_and_osc99_use_different_sources` — feed BOTH `OSC 9 ; X ST` and `OSC 99 ; Y ST` in the same scenario. Assert the two effects have DIFFERENT `NotificationSource` variants. If someone collapses the OSC 9 / 99 detection in the interceptor, this test fails immediately.
- [x] **Negative pin** — `osc9_via_processor_without_mux_drops` — in `oriterm_mux/src/shell_integration/tests.rs`, run ONLY `Processor::advance(&mut term, osc9_bytes)` WITHOUT calling `raw_parser.advance(&mut interceptor, osc9_bytes)` first. Assert NO notification effect is emitted on the sink. This proves the mux interceptor is LOAD-BEARING for OSC 9; if someone accidentally adds OSC 9 to the high-level dispatcher too, this test fails (double-dispatch detection). NOTE: Do NOT use `SpecHarness::feed()` here — `oriterm_test_support` is NOT in `oriterm_mux`'s `[dev-dependencies]` (`oriterm_mux/Cargo.toml` only lists `tempfile = "3"` as a dev-dep). Use `Processor::advance` directly.

**Catalog update:**

- [x] OSC-9 `plans/spec-conformance/catalog/osc.md` → `verified` (was `missing`). Implementation cell now cites `oriterm_mux/src/shell_integration/interceptor.rs::handle_notification_simple`.
- [x] New row OSC-99 added to `plans/spec-conformance/catalog/osc.md` (status `verified-with-deviation` — Kitty's `p=` payload-type metadata key is honoured (`p=title` default, `p=body`, `p=close|icon|?|alive|buttons` and unknown values drop the notification); empty payloads drop per Kitty's "ignored if no title and no body" rule; metadata keys OTHER than `p` (`i`, `d`, `e`, `t`, `f`, `n`, `o`, `s`, `u`, ...) are recognised as opaque and discarded — chunking, base64, urgency, sound, and notification-type filtering are not honoured). Existing OSC-777 row promoted from `missing` to `verified` (OSC-777 already exists at `plans/spec-conformance/catalog/osc.md:57` — do NOT create a duplicate row).
- [x] `plans/spec-conformance/catalog/shell-integration.md` SHINT-OSC-9-NOTIFY and SHINT-OSC-777-NOTIFY → `verified`. NEW SHINT-OSC-99-NOTIFY cross-reference row added (was missing — `osc.md::OSC-99` had been added without the `shell-integration.md` cross-reference, a DRIFT finding caught by TPR round 0).

**Validation:**

- [x] All 12 tests pass (10 behavioral + 1 semantic pin + 1 negative pin). Behavioral count grew from the original 7 → 10 across two TPR checkpoint 2 rounds: round 0 added `osc99_metadata_body_form_pins_current_behavior` (deviation pin); round 1 surfaced Kitty `p=` semantic gaps and the §10.3 fix added `osc99_p_body_routes_payload_to_body`, `osc99_empty_payload_drops_notification`, and `osc99_unsupported_payload_kind_drops_notification`. The two original OSC 99 tests were renamed (`osc99_body_fires_notification_osc99_source` → `osc99_default_payload_routes_to_title`; `osc99_metadata_body_form_pins_current_behavior` → `osc99_metadata_form_default_p_routes_payload_to_title`) to reflect the corrected behavior.
- [x] `NotificationSource` enum (`oriterm_core/src/effect/families/host.rs:55-62`) remains unchanged — no new variants added in this subsection.
- [x] `./build-all.sh` + `./test-all.sh` + `./clippy-all.sh` green after 10.3's changes (per CLAUDE.md "run these after every change").
- [x] **TPR checkpoint 2** — `/tpr-review` covering 10.1–10.3 + re-verification of 10.0. Three rounds (codex+gemini parallel dispatch). Round 0: 4 verified findings — substantive OSC 99 `body_idx` bug (codex high), missing SHINT-OSC-99-NOTIFY drift (codex medium), §10.3 frontmatter status drift (codex low), cross-crate `drain_desktop_notifications` duplication (gemini low → filed as `BUG-11-15`). Round 1: 4 verified findings — Kitty `p=` default-title semantics not honoured (codex medium), §10.3 OSC 99 tests pin wrong title/body (codex medium), `osc9_and_osc99_use_different_sources` non-conformant input + missing body assertions (gemini medium), missing conformant empty-body test (gemini low). Round 2: 1 verified finding — §10.3 catalog-update bullet still describes superseded round-0 deviation (codex low). All 9 actionable findings (rounds 0-2) fixed inline; gemini round 0 F1 deferred via `BUG-11-15`. Round 2 closed with gemini clean and codex's sole finding fixed → effective convergence.

---

## 10.4 OSC 133 semantic prompt + OSC 633 VS Code shell integration

**Files:**
- `oriterm_mux/src/shell_integration/tests.rs` (extend sibling unit-test module — OSC 133 + 633 tests live here because `RawInterceptor` is `pub(crate)` and not accessible from `oriterm_mux/tests/` integration tests)
- `oriterm_mux/src/shell_integration/interceptor.rs` (extend to dispatch OSC 633 sub-commands exclusively here — currently NOT dispatched; adds `b"633"` arm to `osc_dispatch` match, mirroring the OSC 133 pattern; DO NOT add OSC 633 to `crates/vte/src/ansi/dispatch/osc.rs` high-level path — that would create a second dispatch path that fires on the same bytes, producing double-handling; per scope clarification B, the high-level processor silently drops interceptor-managed sequences)
- `oriterm_core/src/term/mod.rs` (**DRIFT FIX**: add `last_command_line: Option<String>` field for OSC 633 E sub-command; add `Term::last_command_line(&self) -> Option<&str>` accessor; add `Term::set_last_command_line(&mut self, line: Option<String>)` mutator — canonical home for this knowledge is `Term` per SSOT; same file that owns `mouse_cursor_icon` and `cwd`)
- Catalog updates: `plans/spec-conformance/catalog/osc.md` OSC-133, OSC-633 (both currently `missing`); `plans/spec-conformance/catalog/shell-integration.md` SHINT-OSC-133-PROMPT, SHINT-OSC-633-VSCODE

**NOTE**: OSC 633 E is interceptor-only. The interceptor calls `term.set_last_command_line(Some(line))` directly on the `Term` struct — NOT through the `Handler` trait dispatch path. Therefore `set_last_command_line` is NOT added to `crates/vte/src/ansi/handler.rs` as a trait method, and there is NO recording_handler delegate for it. The `Handler` trait is the high-level processor interface; the interceptor calls `Term` methods directly. Adding a Handler trait method would create a second (vestigial) dispatch path, contradicting scope clarification B.

**Tests (in `oriterm_mux/src/shell_integration/tests.rs` — OSC 133 + 633 are interceptor-handled, NOT high-level-processor-routed; MUST NOT be in `oriterm_mux/tests/` integration tests which have no `pub(crate)` visibility into `oriterm_mux`):**

### OSC 133 (Final Term semantic prompt)

- [x] `osc133_a_sets_prompt_state` — feed `\x1b]133;A\x1b\\`. Assert `term.prompt_state() == PromptState::PromptStart` AND `term.prompt_mark_pending() == true`. Matches interceptor.rs:92-94.
- [x] `osc133_b_sets_command_state` — feed `\x1b]133;B\x1b\\`. Assert `prompt_state == CommandStart` AND `command_start_mark_pending() == true`.
- [x] `osc133_c_sets_output_state` — feed `\x1b]133;C\x1b\\`. Assert `prompt_state == OutputStart` AND `output_start_mark_pending() == true`. **CLOCK NOTE**: The interceptor's `b'C'` arm calls `self.term.set_command_start(std::time::Instant::now())` — there is NO injectable clock seam for this step; the start time is always a live wall-clock `Instant`. The Option A seam (`finish_command(now: Option<Instant>)`) only covers the D step where the duration is computed. Do NOT assert the specific `Instant` value stored — just assert the prompt-state transitions. The non-deterministic wall-clock issue at the C step is accepted: the meaningful determinism is at the D step (duration calculation), which uses the Option A seam.
- [x] `osc133_d_clears_state_and_emits_command_complete` — SCOPE-CLARIFIED per scope clarification D above. **Test setup**: Feed `OSC 133;A`, then `OSC 133;B`, then `OSC 133;C` via spec_chain_helper (full A→B→C lifecycle — brings `prompt_state` to `OutputStart`; wall-clock `Instant::now()` is stored as the command start). **CRITICAL**: After each feed, the deferred-mark helpers (`term.mark_prompt_row()`, `term.mark_command_start_row()`, `term.mark_output_start_row()`) MUST be called in sequence to populate the `PromptMarker` in `prompt_markers` — the interceptor sets pending flags, but the marks are only written when these methods are invoked (this is what `post_parse_housekeeping` does in production). Without calling these helpers, `term.prompt_markers()` will be empty and the D-step assertions below will fail for the wrong reason. If the `spec_chain_helper` replicates the full 4-step production flow (including `post_parse_housekeeping`), these calls happen automatically; if the helper omits housekeeping, call the deferred-mark methods explicitly after each feed. The interceptor's D arm calls `self.term.finish_command()` which — after the Option A refactor — is `finish_command(None)`, computing `None.unwrap_or_else(Instant::now).duration_since(start)`. Because the interceptor always passes `None`, the exact duration asserted via the feed path will be non-deterministic (roughly 0ms). **DO NOT assert an exact duration via the feed path** — the interceptor has no injectable `now` seam. Instead: (1) assert state transitions and presence of `CommandComplete` effect; (2) assert the duration is a `Duration` (>= zero); (3) if exact-duration verification is needed, test `term.finish_command(Some(t0 + Duration::from_millis(1500)))` DIRECTLY as a shell_state unit test in `oriterm_core/src/term/shell_state/tests.rs` — NOT via the interceptor feed path. This split is required because the interceptor cannot relay a `now` argument through the VTE byte-feed path. Assert:
  - `term.prompt_state() == PromptState::None` (interceptor.rs:106-107 sets it).
  - `Effect::Host(HostEffect::CommandComplete { .. })` is on the transcript (interceptor.rs:108-111). Do NOT assert the exact duration value from the feed path (non-deterministic — the interceptor calls `finish_command(None)` which uses wall-clock elapsed). Assert only that the effect is present and that `duration >= Duration::ZERO`.
  - `term.prompt_markers().last()` still has its A/B/C fields populated (D does NOT mutate the existing marker; it closes out the command lifecycle). This assertion is only valid if the deferred-mark helpers were called during setup — see setup note above.
  - **NO D-field exists on `PromptMarker`** — the plan pins this by asserting `assert_matches!(term.prompt_markers().last().unwrap(), PromptMarker { prompt: _, command: Some(_), output: Some(_) })` (exhaustive match — if a future field is added, this test MUST be updated explicitly).
- [x] `osc133_a_without_b_does_not_record_command` — feed `OSC 133;A`, then call `term.mark_prompt_row()` to flush the pending mark (required — the interceptor sets the pending flag, but the mark only lands when the helper is invoked), then feed `OSC 133;A` again and call `term.mark_prompt_row()` again. Assert TWO `PromptMarker`s exist with `command = None, output = None` on each. (Deferred-mark helpers must be called after each A feed; without them `prompt_markers` stays empty and the TWO-marker assertion fails for the wrong reason.)
- [x] `osc133_command_complete_without_c_is_noop` — feed `OSC 133;D` without a preceding C. Assert NO `HostEffect::CommandComplete` is emitted (interceptor.rs:107's `term.finish_command()` returns `None` when `command_start` is unset). No deferred-mark setup needed here — the assertion is about absence of an effect, not presence of a marker.
- [x] `osc133_full_lifecycle_records_markers` — feed A, call `term.mark_prompt_row()`; feed B (type text), call `term.mark_command_start_row()`; feed C (type command), call `term.mark_output_start_row()`; feed D (type output). Assert the `prompt_markers` vec has one marker with all three of `prompt`, `command`, `output` set to distinct absolute rows. (Deferred-mark helpers MUST be called after A/B/C respectively to flush pending flags into `PromptMarker` — mirrors the production `post_parse_housekeeping` path that is intentionally not callable from the sibling test module.)
- [x] **Semantic pin — CWD SSOT** — if Final Term OSC 133 parameters carry `cwd=<path>`, assert the CWD is written through `Term::set_cwd` (same function OSC 7 uses). NOT through a second CWD field. Cross-reference to scope clarification H. Currently the interceptor at `handle_osc133` does NOT parse `cwd=<path>` params; if the VS Code / Final Term spec requires it, 10.4 adds the parsing AND the SSOT assertion. **N/A for OSC 133**: Final Term spec does not define `cwd=<path>` on OSC 133 sub-letters; ori_term's interceptor does not parse them. The SSOT is enforced on OSC 633's `P;Cwd=` route via `osc633_p_cwd_sets_term_cwd` — same `Term::set_cwd` call OSC 7 uses.

### OSC 633 (VS Code shell integration)

- [x] Read and cite the authoritative VS Code source for OSC 633 sub-commands. **URL UPDATE**: The file moved from `src/vs/workbench/contrib/terminal/browser/xterm/shellIntegrationAddon.ts` (returns 404 as of 2026-04-17) to `https://github.com/microsoft/vscode/blob/main/src/vs/platform/terminal/common/xterm/shellIntegrationAddon.ts` (verified 200 OK). Use the current path: `src/vs/platform/terminal/common/xterm/shellIntegrationAddon.ts`. Also update the success-criteria frontmatter URL (line 17) to use the same corrected path. As of the most-recent reviewed catalog (`plans/spec-conformance/catalog/osc.md:56` labels OSC-633 as `missing`), the common sub-commands are: `A` (prompt start), `B` (command start), `C` (command executed), `D` (command finished), `E` (command line — the raw typed command), `P;<key>=<value>` (property setting — Cwd, IsWindows, etc.).
- [x] Add dispatch + interceptor arms for each VS Code sub-command above. VS Code's semantic overlaps OSC 133, so the implementation wiring may reuse the OSC 133 handlers with VS Code-specific parameter parsing (in particular, `P;Cwd=<path>` should route through `Term::set_cwd` — SSOT with OSC 7).
- [x] `osc633_a_sets_prompt_state` through `osc633_d_emits_command_complete` — matrix mirroring OSC 133 A-D tests with OSC 633's exact syntax.
- [x] `osc633_p_cwd_sets_term_cwd` — feed `\x1b]633;P;Cwd=/home/user/project\x1b\\`. Assert `term.cwd() == Some("/home/user/project")`.
- [x] `osc633_e_records_command_line` — VS Code's `E` sub-command carries the raw command text. Add `Term::last_command_line: Option<String>` field and expose via `term.last_command_line()`. Feed `\x1b]633;E;git status\x1b\\`, assert `term.last_command_line() == Some("git status")`. This sub-command is REQUIRED for `verified` status: OSC-633 is enumerated in the section success criteria with no carve-outs. If the implementation cannot be completed in 10.4 because the VS Code source reveals additional complexity, the sub-command MUST be explicitly filed via `/add-bug` AND OSC-633 catalog status updated to `verified-with-deviation` with a catalog note naming the deviation — the catalog row MUST NOT be marked `verified` while the E sub-command is unimplemented. No silent deferral.
- [x] **Negative pin** — `osc633_via_high_level_processor_drops` — run ONLY `Processor::advance(&mut term, osc633_a_bytes)` WITHOUT calling `raw_parser.advance(&mut interceptor, osc633_a_bytes)` first. Assert `term.prompt_state() == PromptState::None` (NO state change). This confirms OSC 633 dispatch is interceptor-only — the high-level Processor silently drops it. If someone accidentally adds a `b"633"` arm to `crates/vte/src/ansi/dispatch/osc.rs`, this test will fail (double-dispatch detection). NOTE: Do NOT use `SpecHarness::feed()` here — `oriterm_test_support` is NOT in `oriterm_mux`'s `[dev-dependencies]`. Use `Processor::advance` directly (same pattern as `osc9_via_processor_without_mux_drops` in 10.3).

**Catalog update:**

- [x] OSC-133 `plans/spec-conformance/catalog/osc.md`: **Split the single `OSC-133` row into two rows before marking verified.** The A/B/C subops drive `PromptState` state machine (apex: `state-snapshot`); the D subop emits `HostEffect::CommandComplete`. **New apex layer**: `HostEffect::CommandComplete` does NOT map to any existing `ApexLayer` variant — `EffectHostNotification` is documented as "Apex: desktop notification (OSC 9/99/777)" (`crates/oriterm_test_support/src/spec_chain/scenario.rs:96-97`) and is INCORRECT for CommandComplete. Section 10.4 MUST add a new `EffectHostCommand` variant to `ApexLayer` in `crates/oriterm_test_support/src/spec_chain/scenario.rs`, matching the existing non-visual pattern: add `/// Apex: shell command lifecycle signal (OSC 133;D / CommandComplete). EffectHostCommand,` alongside the other effect-host variants, and add `ApexLayer::EffectHostCommand => Self::Effect` in the `from_apex` match arm. ALSO update `plans/spec-conformance/00-overview.md:820` to add `effect-host-command` to the canonical `ApexLayer` enum value list. New catalog rows: `OSC-133-PROMPT` (subops A/B/C, apex `state-snapshot`) and `OSC-133-CMD-COMPLETE` (subop D, apex `effect-host-command`). Implementation cells cite the interceptor + handler paths for each. The single-row `OSC-133` entry in the catalog is deleted. Also update `plans/spec-conformance/catalog/shell-integration.md::SHINT-OSC-133-PROMPT` to reference only the A/B/C rows (`state-snapshot` apex); add a new `SHINT-OSC-133-CMD-COMPLETE` cross-reference row for D (`effect-host-command` apex).
- [x] OSC-633 `plans/spec-conformance/catalog/osc.md` → `verified` (add implementation citations).
- [x] `plans/spec-conformance/catalog/shell-integration.md` SHINT-OSC-133-PROMPT → `verified` (apex `state-snapshot`); SHINT-OSC-133-CMD-COMPLETE → `verified` (new row, added in same edit pass as the OSC-133-CMD-COMPLETE catalog row above; apex `effect-host-command` — requires the new `EffectHostCommand` variant per the bullet above); SHINT-OSC-633-VSCODE → `verified`.

**Validation:**

- [x] OSC 133 A-D + edge cases green.
- [x] OSC 633 sub-command matrix green.
- [x] Injected clock removes flakiness from duration assertions.
- [x] `./build-all.sh` + `./test-all.sh` + `./clippy-all.sh` green after 10.4's changes (per CLAUDE.md "run these after every change").

---

## 10.5 OSC 22 cursor icon + OSC 50 cursor shape

**Files:**
- `oriterm_core/tests/spec_chain/osc/cursor.rs` (new — combines 22 + 50)
- `oriterm_core/src/term/mod.rs` (already extended in 10.0 with `mouse_cursor_icon`)
- Catalog updates: `plans/spec-conformance/catalog/osc.md` OSC-22, OSC-50

**Tests:**

### OSC 22 (mouse cursor icon, iTerm2)

- [ ] `osc22_pointer_sets_cursor_icon` — feed `\x1b]22;pointer\x1b\\`, assert `term.mouse_cursor_icon() == Some(CursorIcon::Pointer)`. Uses the Term field + Handler override from 10.0.
- [ ] `osc22_all_known_icons_matrix` — iterate through every known OSC 22 cursor name string. `cursor_icon 1.2.0` does NOT provide a `CursorIcon::all()` or iterator over variants (confirmed: the crate only exposes `CursorIcon::name()` and `FromStr` parsing). The test defines its OWN name-tagged slice (distinct from the wire-transport constant `oriterm_mux::protocol::snapshot::OSC22_KNOWN_ICONS` which is `&[CursorIcon]` without names — used for stable u8 indexing on the daemon wire): `const OSC22_TEST_CURSOR_NAMES: &[(&str, CursorIcon)] = &[("pointer", CursorIcon::Pointer), ("crosshair", CursorIcon::Crosshair), ...]` covering the ~30 variants from the CSS Basic UI / xterm spec. Feed `OSC 22 ; <name> ST` for each entry; assert each is stored. Self-verifying completeness pin: `assert_eq!(count, OSC22_TEST_CURSOR_NAMES.len())` — the project-owned name slice is the SSOT for which names are exercised BY THIS TEST; the wire `OSC22_KNOWN_ICONS` is the SSOT for which icons are transportable across the daemon boundary. Cross-check: `OSC22_TEST_CURSOR_NAMES` SHOULD be a superset of the CursorIcon variants in `oriterm_mux::protocol::snapshot::OSC22_KNOWN_ICONS` so every wire-transportable icon has a matching name-test, but the test matrix may exceed the wire slice (some variants may not be wire-stable yet) — that asymmetry is expected and not a DRIFT finding.
- [ ] `osc22_unknown_icon_is_dropped` — feed `\x1b]22;not-a-real-cursor\x1b\\`, assert `term.mouse_cursor_icon()` is UNCHANGED (the `CursorIcon::from_str` error path in the dispatcher at `crates/vte/src/ansi/dispatch/osc.rs:184` logs and drops — no state mutation).
- [ ] `osc22_no_parameter_is_dropped` — **NEGATIVE PIN**: feed `\x1b]22\x1b\\` (no second parameter at all, so `params.len() == 1`). The dispatcher at `crates/vte/src/ansi/dispatch/osc.rs:180` gates on `b"22" if params.len() == 2` — when only one param is present, the arm does NOT match and falls to `_ => unhandled(params)`. Assert `term.mouse_cursor_icon()` is UNCHANGED. This pins that a malformed OSC 22 with no cursor-name param is silently dropped, not panicked on.
- [ ] `osc22_reset_behavior` — OSC 22 does not have a spec'd reset form. Document this in the catalog; pin behavior: passing an explicit "default" name (if `CursorIcon::Default` exists) restores the default.
- [ ] **Semantic pin** — `osc22_does_not_affect_text_cursor_shape` — set `term.cursor_shape()` to `Beam` via OSC 50, then fire OSC 22 with `pointer`. Assert `term.cursor_shape() == Beam` (unchanged). Cross-reference scope clarification §I / blind-spot #5 — OSC 22 (mouse icon) and OSC 50 (text shape) are different fields.
- [ ] **Daemon-mode `PaneSnapshot` transport (scheduled in 10.0)**: the `mouse_cursor_icon` field is added to `PaneSnapshot` at `oriterm_mux/src/protocol/snapshot.rs:160` and wired through `oriterm_mux/src/server/snapshot.rs` in subsection 10.0 alongside the embedded-path `Term` / `RenderableContent` additions (see 10.0 Files block and Implementation bullet). 10.5 OSC 22 tests MUST assert the daemon path works END-TO-END across BOTH server-side snapshot production AND client-side decode:
  - **Server-side pin**: `osc22_daemon_snapshot_carries_cursor_icon` — fire OSC 22 at the `Term`, build a `PaneSnapshot` via `server::snapshot`, assert the resulting snapshot's `mouse_cursor_icon` matches the icon set.
  - **Client-side decode pin (initial extract)**: `osc22_daemon_snapshot_decode_first_frame` — build a `PaneSnapshot` with `mouse_cursor_icon = Some(<encoded>)`, call `extract_frame_from_snapshot(snapshot, viewport, cell_size)`, assert `frame.content.mouse_cursor_icon == Some(<decoded>)`. This catches the initial-frame decode path (`snapshot_to_renderable()` at `oriterm/src/gpu/extract/from_snapshot/mod.rs:62-97`) which is separate from the refill path.
  - **Client-side decode pin (refill)**: `osc22_daemon_snapshot_decode_refill` — call `extract_frame_from_snapshot_into(snapshot, &mut out, ...)` on an existing `FrameInput`, assert `out.content.mouse_cursor_icon == Some(<decoded>)`. This catches the refill path (`snapshot_to_renderable_into()` at `oriterm/src/gpu/extract/from_snapshot/mod.rs:103-141`).
  - Together these three pins are the rung-4 verification that the daemon path is live end-to-end — server build, wire transport, and BOTH client decode paths. A test that only asserts `PaneSnapshot::mouse_cursor_icon` would not catch a bug where the client-side decode path fails to populate `RenderableContent::mouse_cursor_icon`.

### OSC 50 (cursor shape, URxvt legacy)

- [ ] `osc50_cursor_shape_block` — feed `\x1b]50;CursorShape=0\x1b\\`, assert `term.cursor_shape() == CursorShape::Block`.
- [ ] `osc50_cursor_shape_beam` — `CursorShape=1` → `Beam`.
- [ ] `osc50_cursor_shape_underline` — `CursorShape=2` → `Underline`.
- [ ] `osc50_unknown_shape_dropped` — feed `CursorShape=9`, assert no change (dispatch arm returns `unhandled` per `dispatch/osc.rs:194-199`).
- [ ] `osc50_malformed_prefix_dropped` — feed `\x1b]50;BADTHING\x1b\\`, assert no change.

**Catalog update:**

- [ ] OSC-22 `plans/spec-conformance/catalog/osc.md` → `verified` (was `stub` — we added the Term field + override in 10.0, so the sequence now has observable state). **CATALOG METADATA UPDATE REQUIRED**: The current `Implementation` cell says "`Handler::set_mouse_cursor_icon` default impl" and `Apex layer` says `effect-host-notification` — both are stale (the no-op default is replaced by a real Term override and the effect is state, not a notification). Rewrite `Implementation` to cite `Term::set_mouse_cursor_icon` in `oriterm_core/src/term/handler/mod.rs`; rewrite `Apex layer` to `state-snapshot`; rewrite `Notes` to replace "effect is dropped" with the new state field description. Do NOT mark the row `verified` while these cells still describe the stub no-op.
- [ ] OSC-50 `plans/spec-conformance/catalog/osc.md` → `verified` (was `implemented-unverified`).

**Validation:**

- [ ] OSC 22 and OSC 50 tests green and do NOT interfere with each other.
- [ ] The `mouse_cursor_icon` field is queryable via `renderable_content()` — a rendering consumer can update the OS cursor on icon change.
- [ ] `./build-all.sh` + `./test-all.sh` + `./clippy-all.sh` green after 10.5's changes (per CLAUDE.md "run these after every change").

---

## 10.6 OSC 104 / 110 / 111 / 112 color reset

**Files:**
- `oriterm_core/tests/spec_chain/osc/color_reset.rs` (new)
- Catalog update: `plans/spec-conformance/catalog/osc.md` rows OSC-104, OSC-110, OSC-111, OSC-112

**Tests:**

- [ ] `osc104_zero_args_resets_all_256_palette` — pre-populate palette: set indices 0..256 to custom colors via OSC 4 at setup. Feed `\x1b]104\x1b\\`. Assert every index 0..256 matches the initial theme palette (compare against `Palette::for_theme(Theme::default())` — `Theme` has no `.palette()` method; use `oriterm_core::color::palette::Palette::for_theme(oriterm_core::theme::Theme::default())`).
- [ ] `osc104_specific_indices_resets_only_those` — set indices 0, 5, 10 to custom colors. Feed `\x1b]104;5;10\x1b\\`. Assert index 0 is still the custom color; indices 5 and 10 are restored to theme defaults; indices 1–4, 6–9, 11–255 are at theme defaults (no collateral damage).
- [ ] `osc104_invalid_index_dropped` — feed `\x1b]104;999;abc\x1b\\`, assert the `parse_number` failure path at `dispatch/osc.rs:231-234` routes to `unhandled` and no palette entry is mutated.
- [ ] `osc110_resets_default_foreground` — set OSC 10 to red, feed `\x1b]110\x1b\\`. Assert default fg matches theme default fg (queryable via `term.palette().foreground()` — NOT `Term::color()`, which does not exist; the Palette API is at `oriterm_core/src/color/palette/mod.rs:253`).
- [ ] `osc111_resets_default_background` — same pattern for Background; use `term.palette().background()`.
- [ ] `osc112_resets_cursor_color` — same pattern for Cursor; use `term.palette().cursor_color()`.
- [ ] `osc_reset_round_trip_with_query` — after each reset, feed OSC 10/11/12 ` ; ?` (query form) and assert `Effect::HostRequest(HostRequest::ColorQuery { ... })` is emitted (same apex as `osc10_query_replies_rgb` — HostRequest::ColorQuery is the spec_chain boundary; the reply `format_color_reply` output is tested in `oriterm_mux` IO-thread response-fulfillment tests — NOT here).
- [ ] **Semantic pin** — `osc104_reset_marks_grid_dirty` — observe damage after OSC 104 (palette change should mark all visible rows dirty per `Term::set_color` which marks grid dirty). Negative pin: if damage isn't set, rendering won't repaint the reset palette — semantic regression.

**Catalog update:**

- [ ] OSC-104, OSC-110, OSC-111, OSC-112 in `plans/spec-conformance/catalog/osc.md` → `verified`.

**Validation:**

- [ ] All 8 tests green.
- [ ] Color-reset round-trip confirms OSC 10/11/12 query returns the theme default after 110/111/112.
- [ ] `./build-all.sh` + `./test-all.sh` + `./clippy-all.sh` green after 10.6's changes (per CLAUDE.md "run these after every change").

---

## 10.7 OSC 1337 non-image sub-ops (handoff from Section 14)

**Files:**
- `oriterm_core/tests/spec_chain/osc/iterm2_non_image.rs` (new)
- `oriterm_core/src/term/handler/mod.rs` (implement `Handler::iterm2_set_mark`, `iterm2_remote_host`, `iterm2_current_dir`, `iterm2_copy`, `iterm2_report_cell_size`, `iterm2_set_user_var`, `iterm2_shell_integration_version` on `Term`)
- `oriterm_core/Cargo.toml` (**NEW DEPENDENCY**: add `indexmap = "2"` under `[dependencies]`. Verified against current Cargo.toml: `indexmap` is NOT currently a dependency of `oriterm_core` or any workspace crate. Adding a new external dependency requires justification per `.claude/rules/impl-hygiene.md` §Dependencies: `user_vars` needs insertion-order preservation for the FIFO eviction policy; `std::collections::HashMap` does NOT preserve insertion order, and manually tracking insertion order alongside a `HashMap` would duplicate the index. `indexmap` is a well-established dependency used widely across the Rust ecosystem for this exact pattern, and keeping the eviction O(1) for `shift_remove_index(0)` + O(1) for `insert` requires a data structure purpose-built for ordered maps. If the workspace later adopts a different ordered-map library, this is a single-call-site swap.)
- `oriterm_core/src/term/mod.rs` (new Term fields: `remote_host: Option<String>`, `user_vars: IndexMap<String, String>`, `shell_integration_version: Option<String>`). **RSS INVARIANT:** `user_vars` MUST be bounded to prevent unbounded memory growth under adversarial PTY output. Apply a configurable max-size cap: default 256 entries; when the cap is reached, the oldest-by-insertion-order entry is evicted before the new one is inserted (**FIFO/insertion-order eviction** — use `IndexMap<String, String>` from the `indexmap` crate, which preserves insertion order; evict by calling `map.shift_remove_index(0)` on the first entry before inserting). Import: `use indexmap::IndexMap;` at the top of `term/mod.rs`. The RSS regression test (`oriterm_core/tests/rss_regression.rs`) MUST stay green; a `user_vars` that grows without bound per unique key fails that invariant. The size cap itself is verified by a dedicated test: `osc1337_user_vars_cap_evicts_oldest` — insert 257 distinct keys, assert the map size remains at 256 and the first-inserted key is gone. **Eviction policy is FIFO (insertion-order), NOT LRU (access-order)** — accessing a key does not refresh its eviction position; only the insertion order matters. This matches the test pin: `KEY_0` (first inserted) is evicted when the cap is exceeded. **Re-insert semantics**: calling `user_vars.insert(existing_key, new_value)` on `IndexMap` REPLACES the value but does NOT update the insertion position (same as `HashMap`) — if the caller needs "touching a key refreshes position", they must `shift_remove` + `insert`. For OSC 1337 SetUserVar the simpler "replace in place, do not refresh" semantic is correct (the use case is "record user variables", not "LRU cache").
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
- [ ] `osc1337_unknown_file_subop_safely_ignored` — **NEGATIVE PIN (blind-spot #9 remediation for Section 14 de-risking)**: feed `\x1b]1337;File=name=foo;UnknownFileAttr=bar;:<tiny-png-bytes>\x1b\\`. Assert that `Handler::iterm2_file` IS still called (the `File=` prefix routes through the main arm) AND that the unknown `UnknownFileAttr=` sub-key is silently absorbed by the payload parser (no panic, no state corruption, `term.iterm2_files()` — if that accessor exists in Section 14 — records the recognized attrs only). This pins that unknown `File=` sub-keys do NOT crash the sub-dispatcher and do NOT cause the whole `File=` payload to be dropped. It is a 10.7 de-risking test for Section 14's image-handoff work: Section 14 inherits this contract and will add the payload-level assertion that the recognized `File=` attrs (`name`, `size`, `width`, `height`, `preserveAspectRatio`, `inline`) round-trip correctly. Without this pin in 10.7, a future `iterm2_file` parser refactor could silently break `File=` dispatch for any payload containing a newer iTerm2 attribute.
- [ ] `osc1337_user_vars_cap_evicts_oldest` — **RSS REGRESSION PIN**: insert 257 distinct `SetUserVar` keys (`KEY_0` through `KEY_256`). Assert `term.user_vars().len() == 256` (cap enforced) AND `term.user_var("KEY_0") == None` (oldest evicted). Assert `term.user_var("KEY_256") == Some(...)` (newest retained). This test MUST FAIL if `user_vars` grows unboundedly. Cross-reference 10.N RSS regression check.
- [ ] `osc1337_user_vars_reinsert_does_not_refresh_position` — **SEMANTIC PIN**: start with cap 3 entries (test-only helper that overrides the default 256 for this pin). Insert keys `A`, `B`, `C`. Re-insert `A` with a new value. Insert `D`. Assert the evicted key is `A` (NOT `B`) — proving that re-inserting an existing key does NOT refresh its eviction position per the documented "replace in place, do not refresh" semantic. This pin documents the deliberate non-LRU behavior so a future implementer who sees "cap reached, need to evict" cannot silently change `insert` to `shift_remove + insert` without breaking this test.
- [ ] `osc1337_copy_invalid_base64_dropped` — **NEGATIVE PIN** (per `.claude/rules/tests.md §Negative Testing Protocol`): feed `\x1b]1337;Copy=:!!!not-valid-base64!!!\x1b\\`. Assert NO `HostEffect::ClipboardStore` is emitted (the Copy handler must drop invalid base64, not panic or store garbage). Mirrors the parallel `osc52_store_invalid_base64_dropped` test in 10.2 for consistency.
- [ ] `osc1337_set_user_var_invalid_base64_dropped` — **NEGATIVE PIN**: feed `\x1b]1337;SetUserVar=MY_KEY=!!!invalid!!!\x1b\\`. Assert NO entry is added to `user_vars` for `MY_KEY` (the SetUserVar handler must reject invalid base64 in the value, not store garbage). Documents the expected drop behavior; if the current dispatcher accepts invalid base64 and stores raw bytes, file `/add-bug` and update the test to assert the observed behavior as a documented deviation.
- [ ] **Semantic pin — SSOT for CWD (direction A)** — set `term.cwd()` via OSC 7 (`file:///start ST`). Feed `OSC 1337 ; CurrentDir=/other-path ST`. Assert `term.cwd() == Some("/other-path")` (last write wins; NO second CWD field). Cross-reference scope clarification §H.
- [ ] **Semantic pin — SSOT for CWD (direction B)** — set `term.cwd()` via OSC 1337 CurrentDir first (`/from-iterm2`). Then feed `OSC 7 ; file:///from-osc7 ST`. Assert `term.cwd() == Some("/from-osc7")` (OSC 7 overwrites OSC 1337 via the same canonical `Term::set_cwd` field). Matrix clamping requires BOTH directions per `.claude/rules/tests.md §Matrix Clamping` — a one-directional test misses a future regression where OSC 1337 writes a second CWD field that OSC 7 does not overwrite.

**Catalog update:**

- [ ] `plans/spec-conformance/catalog/iterm2.md` front-matter `owner_section` → `"01 (bootstrap), 10 (non-image), 14 (image)"`.
- [ ] Rows ITERM2-1337-REMOTEHOST, ITERM2-1337-CURRENTDIR, ITERM2-1337-COPY, ITERM2-1337-SETMARK, ITERM2-1337-REPORTCELLSIZE, ITERM2-1337-SETUSERVAR → `verified` with implementation citation to `oriterm_core/src/term/handler/mod.rs::iterm2_*`.
- [ ] ITERM2-1337-FILE stays at `implemented-unverified` (Section 14 owns its verification). Add a catalog Notes entry cross-linking Section 10's ownership of the non-image variants.
- [ ] New catalog row `ITERM2-1337-SHELLINTVERSION` added to `plans/spec-conformance/catalog/iterm2.md`: sequence `` `OSC 1337 ; ShellIntegrationVersion=<version> BEL|ST` ``, description "Report shell integration version string", implementation cites `Handler::iterm2_shell_integration_version` (`crates/vte/src/ansi/handler.rs`) → `Term::iterm2_shell_integration_version` (`oriterm_core/src/term/handler/mod.rs`), apex `state-snapshot`, verification → `verified`. This row MUST appear in the catalog before 10.7 marks itself complete — the section success criteria explicitly include it.
- [ ] `plans/spec-conformance/catalog/shell-integration.md` — add or update cross-reference rows for OSC 1337 non-image sub-ops per the success criteria promise: SHINT-OSC-1337-REMOTEHOST, SHINT-OSC-1337-CURRENTDIR, SHINT-OSC-1337-SETMARK, SHINT-OSC-1337-SETUSERVAR, SHINT-OSC-1337-REPORTCELLSIZE → `verified`, each citing the corresponding `ITERM2-1337-*` row in `plans/spec-conformance/catalog/iterm2.md` and the `Term::iterm2_*` handler in `oriterm_core/src/term/handler/mod.rs`. These rows close out the success criterion "Every row in `plans/spec-conformance/catalog/shell-integration.md` is `verified` (... OSC-1337-RemoteHost / CurrentDir / SetMark / SetUserVar / ReportCellSize shell-integration cross-refs ...)".

**Plan sync to Section 14 (flow-up edit beyond single-section authority — partially APPLIED during /review-plan Step 5 editor pass 2026-04-18):**

- `plans/spec-conformance/catalog/iterm2.md:5` (`owner_section` front-matter) has been updated to `"01 (bootstrap), 10 (non-image), 14 (image)"` by the /review-plan editor pass on 2026-04-18, and the table's non-image rows (`ITERM2-1337-REMOTEHOST`, `ITERM2-1337-CURRENTDIR`, `ITERM2-1337-COPY`, `ITERM2-1337-SETMARK`, `ITERM2-1337-REPORTCELLSIZE`, `ITERM2-1337-SETUSERVAR`) have been retagged to cite Section 10.0's landed sub-dispatcher and Section 10.7's pending verification work. The new `ITERM2-1337-SHELLINTVERSION` row was also added in that pass (status `implemented-unverified`, awaiting 10.7's `Term` override + verification). `ITERM2-1337-FILE` stays `implemented-unverified` with an explicit "Owner: Section 14 (image)" note and a cross-link to Section 10's ownership of non-image siblings. Subsection 10.7 tasks in this plan are now pure verification/state-field work against the already-stamped ownership; do NOT re-flip those owner annotations. `plans/spec-conformance/section-14-iterm2-images.md:55` already reads "Section 10's OSC suite covered the non-image OSC 1337 variants" — Section 14's next /review-plan can leave the prose as-is or tighten wording to cite the updated `owner_section` field, but no functional edit to Section 14 is required from this pass.

**Validation:**

- [ ] All 16 tests green: `osc1337_set_mark`, `osc1337_remote_host`, `osc1337_current_dir`, `osc1337_copy`, `osc1337_report_cell_size`, `osc1337_set_user_var`, `osc1337_shell_integration_version` (7 behavioral) + `osc1337_file_still_routes_to_iterm2_file` + `osc1337_unknown_key_dropped` + `osc1337_unknown_file_subop_safely_ignored` (blind-spot #9 Section 14 de-risking pin) + `osc1337_user_vars_cap_evicts_oldest` + `osc1337_user_vars_reinsert_does_not_refresh_position` (IndexMap re-insert semantic pin) + `osc1337_ssot_cwd_direction_a` (OSC 7 → OSC 1337 overwrite) + `osc1337_ssot_cwd_direction_b` (OSC 1337 → OSC 7 overwrite) (2 CWD SSOT semantic pins) + `osc1337_copy_invalid_base64_dropped` + `osc1337_set_user_var_invalid_base64_dropped` (2 negative pins) = 16 total.
- [ ] OSC 1337 `File=` path unchanged; Section 14 can build on top without touching the sub-dispatcher.
- [ ] `./build-all.sh` + `./test-all.sh` + `./clippy-all.sh` green after 10.7's changes (per CLAUDE.md "run these after every change").
- [ ] **TPR checkpoint 3** — `/tpr-review` covering 10.4–10.7 + ownership cross-check against Section 14.

---

## 10.8 Basic OSC rows (0/1/2/4/7/10/11/12/52) — inherited from Section 08

**Files:**
- `oriterm_core/tests/spec_chain/osc/basic.rs` (new — covers OSC 0/1/2 via `SpecHarness::feed()`)
- `oriterm_core/tests/spec_chain/osc/palette.rs` (new — covers OSC 4 set/query via `SpecHarness::feed()`)
- `oriterm_mux/src/shell_integration/tests.rs` (extend sibling unit-test module — covers OSC 7 via mux-layer test; OSC 7 is interceptor-handled, NOT high-level-processor-routed; must be sibling unit-test for `pub(crate)` access to `RawInterceptor`)
- `oriterm_core/tests/spec_chain/osc/default_colors.rs` (new — covers OSC 10/11/12 set/query via `SpecHarness::feed()`)
- Catalog updates: `plans/spec-conformance/catalog/osc.md` rows OSC-0, OSC-1, OSC-2, OSC-4-SET, OSC-4-QUERY, OSC-7, OSC-10-SET, OSC-10-QUERY, OSC-11-SET, OSC-11-QUERY, OSC-12-SET, OSC-12-QUERY; `plans/spec-conformance/catalog/shell-integration.md` row SHINT-OSC-7-CWD

**Scope pin from 08:** `plans/spec-conformance/section-08-ecma-48-baseline.md:179` recorded zero OSC coverage from tack; all rows below start at `implemented-unverified` / `stub` and end `verified` here.

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
- [ ] `osc4_query_palette_index` — feed `\x1b]4;5;?\x1b\\`, assert `Effect::HostRequest(HostRequest::ColorQuery { prefix, index, .. })` is emitted where `prefix == "4;5"` and `index == 5` (the OSC 4 dispatcher at `crates/vte/src/ansi/dispatch/osc.rs:108` builds `let prefix = format!("4;{index}")` — confirmed — so prefix encodes both the OSC number AND the palette index; `HostRequest::ColorQuery` has fields `prefix: String`, `index: usize`, `terminator: String`, `reply: ResponseToken<Rgb>` — there is NO `ColorQueryTarget` enum; the reply bytes `OSC 4 ; 5 ; rgb:ffff/0000/0000 ST` are produced by the consumer and tested separately in `oriterm_mux` IO-thread tests — the spec_chain scope boundary is HostRequest emission, NOT the reply write).
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
- [ ] **OSC 7 double-dispatch remediation (LEAK:duplicated-dispatch):** The `b"7"` arm in `crates/vte/src/ansi/dispatch/osc.rs:69-87` calls `handler.set_working_directory()` which is a no-op default on `Term` (confirmed: `Term` does not override this method). The interceptor at `oriterm_mux/src/shell_integration/interceptor.rs:37` handles OSC 7 canonically with full URI parsing. The high-level `b"7"` arm is therefore vestigial — it calls a no-op and provides no value. The interceptor module doc (`interceptor.rs:6-9`) acknowledges this: "OSC 7 is also handled here (with proper URI parsing and percent-decoding) because `Term` does NOT override `Handler::set_working_directory` — the high-level handler default is a no-op. The interceptor is therefore the sole canonical path for CWD updates from OSC 7 (SSOT: `Term::set_cwd`)." Section 10.8 MUST remove the `b"7"` arm from `osc.rs`. Rationale: the arm is vestigial (calls a no-op), and leaving it creates a second apparent dispatch path that confuses future readers and could be mistakenly "fixed" to re-implement CWD logic in the wrong layer. Do NOT add `assert!(!reachable)` or `debug_assert` — the arm DOES fire on valid OSC 7 input (it just calls a no-op handler); asserting unreachability would panic on valid user input, violating `.claude/rules/impl-hygiene.md §Panic & Assertion`. The only valid options are: (a) delete the arm entirely, or (b) replace the arm body with a `// SSOT: CWD is handled exclusively by RawInterceptor; see interceptor.rs. This arm intentionally calls a no-op.` comment — with NO assertion. Preferred fix: delete the arm.

### OSC 10 / 11 / 12 (default colors)

- [ ] `osc10_sets_default_foreground` — feed `\x1b]10;rgb:de/ad/be\x1b\\`, assert `term.palette().foreground() == Rgb(0xde, 0xad, 0xbe)` (use `Palette::foreground()` at `oriterm_core/src/color/palette/mod.rs:253` — `Term::color()` does not exist; the method is on `Palette`, not `Term`).
- [ ] `osc11_sets_default_background` — OSC 11.
- [ ] `osc12_sets_cursor_color` — OSC 12.
- [ ] `osc10_query_replies_rgb` — feed `\x1b]10;?\x1b\\`, assert `Effect::HostRequest(HostRequest::ColorQuery { prefix, index, .. })` is emitted where `prefix == "10"` and `index == 256` (`NamedColor::Foreground as usize == 256`; `HostRequest::ColorQuery` has fields `prefix: String`, `index: usize`, `terminator: String`, `reply: ResponseToken<Rgb>` — there is NO `ColorQueryTarget` enum; the OSC dispatcher maps `OSC 10 → NamedColor::Foreground as usize`, `OSC 11 → NamedColor::Background as usize` (257), `OSC 12 → NamedColor::Cursor as usize` (258) via `let offset = dynamic_code as usize - 10; let index = NamedColor::Foreground as usize + offset` — confirmed at `crates/vte/src/ansi/dispatch/osc.rs:151-152`; same `HostRequest::ColorQuery` apex as OSC 4 queries; the reply `OSC 10 ; rgb:dede/adad/bebe ST` is produced by the consumer and tested separately in `oriterm_mux` IO-thread response-fulfillment tests).
- [ ] `osc10_multi_param_walks_named_colors` — per the osc.md Notes column, the multi-param form walks `NamedColor::Foreground..Cursor`. Feed `\x1b]10;rgb:10/10/10;rgb:20/20/20;rgb:30/30/30\x1b\\`, assert Foreground == `#101010`, Background == `#202020`, Cursor == `#303030`.

### OSC 52 already covered in 10.2

**Catalog updates:**

- [ ] Every row named at the top of 10.8 is promoted from `implemented-unverified` / `stub` to `verified`. **OSC-7 METADATA UPDATE REQUIRED**: The current `plans/spec-conformance/catalog/osc.md` OSC-7 `Implementation` cell reads "`osc::dispatch` (`crates/vte/src/ansi/dispatch/osc.rs`) — `b"7"` arm → `Handler::set_working_directory` default impl". After the 10.8 remediation removes the `b"7"` arm, this Implementation description is wrong. Rewrite it to cite the canonical path: `oriterm_mux/src/shell_integration/interceptor.rs` — `RawInterceptor::osc_dispatch` `b"7"` arm → `parse_osc7_path` → `Term::set_cwd`; rewrite `Notes` to state the high-level `b"7"` arm was removed (vestigial). Do NOT mark OSC-7 `verified` while the Implementation cell still describes the deleted high-level arm.
- [ ] `plans/spec-conformance/catalog/shell-integration.md` SHINT-OSC-7-CWD → `verified` (was `stub`; now the interceptor actually writes CWD). Rewrite the SHINT-OSC-7-CWD `Implementation` cross-reference to point to the interceptor path (`oriterm_mux/src/shell_integration/interceptor.rs`) rather than the high-level arm.

**Validation:**

- [ ] All ~25 tests green across the 5 files.
- [ ] Existing teseq tests (`osc_title.teseq`, `osc_icon_name.teseq`, `osc_color_query.teseq`, `osc_clipboard.teseq`) still green — these stay as regression guards.
- [ ] `./build-all.sh` + `./test-all.sh` + `./clippy-all.sh` green after 10.8's changes (per CLAUDE.md "run these after every change").

---

## 10.9 OSC rows currently `missing` — dispatch + handler + verification

**Files:**
- `crates/vte/src/ansi/dispatch/osc.rs` (add dispatch arms for OSC 3, 5, 6, 13, 14, 17, 19, 113, 114, 117, 119, L, l)
- `crates/vte/src/ansi/handler.rs` (add default Handler trait methods)
- `oriterm_core/src/term/handler/mod.rs` (**OVERRIDE HOME**: `impl Handler for Term<S>` lives here at line 30 — add the Handler method overrides for the new OSC variants here, NOT in `handler/osc.rs`. `handler/osc.rs` contains helper methods on `impl<S: EffectSink> Term<S>` — the two files serve different purposes: `mod.rs` is the trait impl, `osc.rs` is the helper impl. Any `fn set_x11_property`, `fn set_mouse_fg_color`, etc. that override Handler defaults belong in `handler/mod.rs`. If the implementation logic is complex, extract helpers into `handler/osc.rs` and call them from `mod.rs`.)
- `oriterm_core/src/term/handler/osc.rs` (OSC helper implementations — complex logic extracted from the Handler overrides; new fields `tab_title_color`, `mouse_fg_color`, `mouse_bg_color`, etc. accessed via helpers here)
- `oriterm_core/src/term/mod.rs` (new Term fields for 10.9: `x11_property`, `tab_title_color`, `mouse_fg_color`, `mouse_bg_color`, `highlight_bg_color`, `highlight_fg_color` — plus any other state-carrying fields for the new OSC variants)
- `crates/oriterm_test_support/src/spec_chain/recording_handler.rs` (**REGISTRATION SYNC**: every new `Handler::set_x11_property`, `Handler::set_mouse_fg_color`, etc. method added to `crates/vte/src/ansi/handler.rs` MUST have a matching delegate arm here — same pattern as existing arms at lines ~320+. Missing arms cause spec_chain tests to silently miss the new dispatch.)
- `oriterm_core/tests/spec_chain/osc/missing_rows.rs` (new — verifies each added variant)
- Catalog updates: `plans/spec-conformance/catalog/osc.md` rows OSC-3, OSC-5-SET, OSC-5-QUERY, OSC-6, OSC-13-SET/QUERY, OSC-14-SET/QUERY, OSC-17-SET/QUERY, OSC-19-SET/QUERY, OSC-113, OSC-114, OSC-117, OSC-119, OSC-L, OSC-l

**Per-row analysis (per `plans/spec-conformance/catalog/osc.md:37-56`):**

- **OSC 3** (set X11 window property) — platform-specific (X11 only). Per xterm ctlseqs: `OSC 3 ; Pt BEL|ST` where `Pt` is a SINGLE string payload in the form `prop=value` (to set the property), or just `prop` (to DELETE the property). The payload is NOT two semicolon-delimited fields — the parser MUST split `Pt` on the first `=` to recover `prop` and optional `value`. Add a dispatch arm that routes to `Handler::set_x11_property(payload: &[u8])` (single-payload signature; the handler does the `=` split internally). The catalog grammar row at `plans/spec-conformance/catalog/osc.md:49` (currently `OSC 3 ; prop ; value BEL|ST`) MUST be rewritten to `OSC 3 ; Pt BEL|ST` with a Notes cell explaining the `Pt = prop[=value]` split — that catalog update is scheduled below. **EFFECT NOTE (post effect-cutover §01.1 landing):** `HostEffect::SetX11Property` does NOT exist in `oriterm_core/src/effect/families/host.rs` (the enum has no such variant). Do NOT reference this variant. With `LegacyEventSink` DELETED from the workspace (effect-cutover §01.3, commit `0d05ca25`), there is NO exhaustive `HostEffect` match in the codebase — `QueueingEffectSink::push()` queues opaquely (`oriterm_core/src/effect/sink/mod.rs`). The implementation must therefore choose: (a) add a new `HostEffect::SetX11Property { prop: String, value: Option<String> }` variant — because no exhaustive match consumer exists post-§01.3, adding the variant is a pure additive change (no match arms need updating); only consumer-side test files that explicitly enumerate variants (e.g. `oriterm_core/tests/effect_cutover_deletion_pins.rs`) would need review, OR (b) keep OSC 3 as state-only — add a `Term::x11_properties: IndexMap<String, Option<String>>` field queried by renderable/state rungs, with `Option<String>::None` encoding the bare-`prop` delete form, without emitting any `HostEffect`. Option B is preferred for 10.9's scope to avoid expanding the effect surface prematurely; Option A is preferred if/when Section 17+ or Section 26 needs to act on the property change. `Handler::set_x11_property` default on `Term` is a no-op; on Linux+X11 runtime, a future section may emit the effect. For 10.9 verification: test that the dispatch arm fires (state rung, not effect rung) with BOTH `prop=value` (set) and bare `prop` (delete) payload forms, and on non-X11 platforms the dispatch returns without side effects. Catalog status → `verified-with-deviation` with a note that the effect is platform-conditional and that the `Pt` payload is parsed into `prop[=value]`.
- **OSC 5** (change/query special color: highlight/bold) — add dispatch + handler; test set + query round-trip. **Catalog note**: OSC 5 requires two catalog rows: `OSC-5-SET` (set path, apex `state-snapshot`) and `OSC-5-QUERY` (query path returns color value via PTY response, apex `effect-pty-write`). The existing single `OSC-5` row in `plans/spec-conformance/catalog/osc.md` (currently `state-snapshot`) must be replaced with these two rows — the set-only apex would be incorrect for the query test. Pattern matches OSC-13-SET/QUERY split. `verified` on both rows.
- **OSC 6** — the OSC 6 code number is AMBIGUOUS in the ecosystem: xterm ctlseqs assigns it to "Enable/disable Special Color Number Ps" (a boolean toggle for the OSC 5 special-color slots), while iTerm2 uses it for "Change Title Tab Color" (a color-setting sequence for the tab title bar). These are incompatible protocols sharing the same OSC number. ori_term follows wezterm's convention (confirmed at `~/projects/reference_repos/console_repos/wezterm/docs/escape-sequences.md:392` — "iTerm2 Change Title Tab Color") and implements the **iTerm2 interpretation**: OSC 6 sets the tab title color via a `Term::tab_title_color` field. The xterm "enable/disable special color" semantic is NOT supported — if a program sends `OSC 6 ; 0 ST` (xterm-style disable) and a later `OSC 5 ; Ps ; spec` arrives, ori_term will apply the OSC 5 color without checking an enable bit. This scope decision is documented on the OSC 6 catalog row. Tests: feed `\x1b]6;rgb:aa/bb/cc\x1b\\`, assert `term.tab_title_color() == Some(Rgb(0xaa, 0xbb, 0xcc))`; negative pin `osc6_xterm_disable_form_is_treated_as_color_parse_failure` — feed `\x1b]6;0\x1b\\` (the xterm disable form), assert `term.tab_title_color()` is UNCHANGED (the color parser rejects "0" as not a valid RGB spec, so the sequence is a no-op; this deliberately documents that ori_term does NOT interpret the xterm enable/disable semantic). `verified`.
- **OSC 13 / 113** (mouse fg color set/reset) — add dispatch + handler pair. Test set via `OSC 13 ; rgb:... ST`, query via `OSC 13 ; ? ST`, reset via `OSC 113 ST`. `verified`.
- **OSC 14 / 114** (mouse background color — NOT Tektronix) — same pattern as 13/113. Per xterm ctlseqs: OSC 14 is specifically "Change/Query Mouse Background Color". The prior "Tektronix" label was incorrect — Tektronix colors live at OSC 15/16/18 (Tektronix fg/bg/cursor) and are a separate concern from mouse colors; see the Tektronix scope note below.
- **Tektronix color rows (OSC 15, OSC 16, OSC 18, and their resets OSC 115 / 116 / 118) — intentionally omitted from Section 10 scope**. xterm ctlseqs defines these as Tektronix-emulator-specific foreground / background / cursor colors. ori_term does NOT emulate the Tektronix 4014 graphics terminal (it is a modern pty terminal emulator, same category as Alacritty / WezTerm / Ghostty — none of which implement Tektronix emulation either; confirmed in the reference table at `~/projects/reference_repos/console_repos/wezterm/docs/escape-sequences.md:384-405` which lists OSC 10/11/12 but NOT OSC 15/16/18). Deliberately omitted from `plans/spec-conformance/catalog/osc.md` — consumers that rely on Tektronix emulation are not in ori_term's target audience. The catalog Notes cell on OSC 13/14 SHOULD explicitly state "Tektronix color rows (OSC 15/16/18) are intentionally omitted — see Section 10.9 scope note" so a future contributor reading the catalog doesn't conclude the omission is an oversight.
- **OSC 17 / 117** (highlight bg color, selection bg) — same pattern; integrates with existing selection rendering state.
- **OSC 19 / 119** (highlight fg color) — same pattern.
- **OSC L** / **OSC l** (Sun console aliases for OSC 1 / 2) — add dispatch arms that alias to `Term::set_icon_name` (L) and `Term::set_title` (l). Test the aliasing. `verified`.

**Tests:**

- [ ] One test per variant — ~22 tests total (set + query + reset pairs).
- [ ] Cross-reset consistency: set OSC 13, reset via OSC 113, verify it returns to default.
- [ ] **Negative pins (MANDATORY per `.claude/rules/tests.md` §Negative Testing Protocol)**:
  - `osc5_invalid_color_dropped` — feed `\x1b]5;NOT_A_COLOR\x1b\\`, assert no state mutation (special-color handler must drop invalid specs).
  - `osc6_xterm_disable_form_is_treated_as_color_parse_failure` — feed `\x1b]6;0\x1b\\` (the xterm-ctlseqs disable form, which ori_term does NOT implement — see OSC 6 scope decision above). Assert `term.tab_title_color()` is UNCHANGED. Documents the deliberate decision to follow iTerm2's OSC 6 (tab color) over xterm's OSC 6 (special-color enable/disable).
  - `osc13_invalid_rgb_dropped` — feed `\x1b]13;GARBAGE\x1b\\`, assert `term.mouse_fg_color()` is unchanged.
  - `osc14_invalid_rgb_dropped` — feed `\x1b]14;GARBAGE\x1b\\`, assert `term.mouse_bg_color()` is unchanged.
  - `osc17_invalid_rgb_dropped` — feed `\x1b]17;GARBAGE\x1b\\`, assert `term.highlight_bg_color()` is unchanged.
  - `osc19_invalid_rgb_dropped` — feed `\x1b]19;GARBAGE\x1b\\`, assert `term.highlight_fg_color()` is unchanged.
  - `osc3_set_with_value` — feed `\x1b]3;FOO=bar\x1b\\`, assert `Term::x11_property("FOO") == Some(Some("bar"))` (or the equivalent state-field accessor on non-X11 platforms behind `#[cfg]`). Pins the `prop=value` payload form per xterm ctlseqs.
  - `osc3_delete_without_value` — feed `\x1b]3;FOO\x1b\\` (bare `prop`, no `=`), assert the OSC-3 delete path fires: `Term::x11_property("FOO") == Some(None)` (entry present with a None value encoding the delete), OR the key is absent entirely depending on the concrete state-field shape chosen in 10.0. Pins the bare-`prop` delete form.
  - `osc3_non_x11_platform_no_panic` — on non-X11 platforms (macOS, Windows), feed BOTH `\x1b]3;FOO=bar\x1b\\` (set) and `\x1b]3;FOO\x1b\\` (delete), assert no panic. Term's OSC-3 state field is absent by `#[cfg(all(unix, not(target_os = "macos")))]` gate, so the negative pin is a compile-time absence + runtime no-panic. The `HostEffect::SetX11Property` variant does NOT exist and the pin MUST NOT reference it — asserting the absence of a non-existent variant would not compile.
  - `osc_l_empty_sets_empty_title` — feed `\x1b]l;\x1b\\` (OSC l alias for OSC 2), assert `term.title() == ""` and no panic (mirrors `osc0_empty_sets_empty_string` edge case for the alias).
  - Add corresponding entries to the 10.N negative-pins checklist.
- [ ] **Matrix completeness pin (SSOT: use catalog_row_id scanner, NOT function-name grep)** — use the existing `scan_test_citations` / `CoverageReport` infrastructure at `crates/oriterm_test_support/src/spec_chain/coverage/` (not a raw grep for function names). Every `SpecScenario` const in the OSC test files MUST declare `catalog_row_id: "OSC-<N>"` matching the corresponding catalog row ID. Run the coverage report (`cargo run -p oriterm_test_support --bin spec-coverage-report`) and assert every OSC catalog row has at least one citation. (Binary name is `spec-coverage-report` with hyphens, NOT `spec_coverage_report` with underscores — per `crates/oriterm_test_support/Cargo.toml:[[bin]]:name`.) Function-name grepping (`osc<N>`) would bypass this SSOT and create a second catalog-tracking mechanism that can drift from the canonical scanner.

**Catalog updates:**

- [ ] Every `missing` row named above → `verified` or `verified-with-deviation`.

**Validation:**

- [ ] Every `missing` OSC catalog row covered by 10.9 is promoted to `verified` or `verified-with-deviation`.
- [ ] Matrix completeness pin (above) passes — catalog row IDs cover every OSC variant added.
- [ ] `./build-all.sh` + `./test-all.sh` + `./clippy-all.sh` green after 10.9's changes (per CLAUDE.md "run these after every change").

---

## 10.R Third Party Review Findings

<!-- Round 29 findings (2026-04-18) — /review-plan Step 5 editor pass; all fixed inline -->

- [x] `[EDITOR-29-1][high]` Goal paragraph + success criterion + Scope Clarification G + §10.0 Files/Tests/Implementation/Validation blocks + §10.2 Files/Tests/Validation blocks + §10.N structural note + §10.N Accepted audit findings — all described OSC 52 response-poll "activation" as pending work, but effect-cutover §01 landed fully (commit `b89bdf84 docs(effect-cutover): close §01.N + complete the entire plan`). The `#[allow(dead_code, reason = "dormant during legacy phase")]` gate on `PaneIoThread::register_host_request_response` was REMOVED; `register_host_request_response` is wired from `effect_router/mod.rs:194,215`; nine response_poll tests are green at `oriterm_mux/src/pane/io_thread/response_poll/tests.rs` (including `response_poll_idle_wake_unblocks_select` which pins the idle-wake channel); `LegacyEventSink` is DELETED from `oriterm_core` per effect-cutover §01.3 (commit `0d05ca25`). PLAN/CODE DRIFT across ~15 bullets.
  Evidence: `grep -rn '#\[allow(dead_code, reason = "dormant during legacy phase"' oriterm_mux/` → 0 matches; `grep -rn LegacyEventSink oriterm_core/src/` → 0 matches (only `effect_cutover_deletion_pins.rs` retains a reference, pinning the deletion); `oriterm_mux/src/pane/io_thread/response_poll/mod.rs:33` — `pub(super) fn register_host_request_response` with no allow attribute; `oriterm_mux/src/pane/io_thread/effect_router/mod.rs:194` — `self.register_host_request_response(HostRequest::ClipboardLoad { .. })` live.
  Impact: Plan described ~15 hours of work that is already done. An implementer following the plan would spend time trying to remove a gate that is gone, wire call sites that are wired, and add idle-wake channels that landed. Section 10.2's success criterion "removes the `#[allow(dead_code)]` gate on `PaneIoThread::register_host_request_response` and wires it into the IO thread" is unachievable-because-already-done.
  Required plan update: (a) Goal paragraph rewritten to describe §10.2 as a CONSUMER of the already-live pipeline, not an activator. (b) Success criterion for OSC 52 rewritten to drop the gate-removal language and focus on OSC-52-specific spec_chain + IO-thread coverage on top of the already-live pipeline. (c) Scope Clarification G rewritten with a CODE REALITY CHECK block enumerating the live state with file:line evidence. (d) §10.0 Files block for `response_poll.rs` marked "NO EDIT REQUIRED". (e) §10.0 Response-poll activation pin TDD test marked `[x]` OBE with citations to the nine already-green tests. (f) §10.0 "Response-poll activation requires EffectSink migration (GAP)" bullet marked `[x]` OBE with the landing commit citations. (g) §10.0 Validation grep check marked `[x]` with verification date. (h) §10.0 Implementation notes "Four items remain BLOCKED" rewritten to "UNBLOCKED" — three of the four items are now ready to resume; the response-poll one is OBE. (i) §10.2 Files block updated to reference the existing `response_poll/tests.rs` as the home for a single NEW OSC-52-specific round-trip test (`osc52_register_poll_roundtrip`). (j) §10.2 Tests block — `response_poll_roundtrip_emits_pty_write` replaced by OSC-52-specific `osc52_register_poll_roundtrip` which drives the round-trip from actual OSC 52 wire bytes rather than a synthetic `HostRequest::ClipboardLoad` insertion (distinct contribution on top of the already-green tests); `response_poll_token_requires_fulfillment` + `response_token_rejects_double_fulfillment` + `response_token_fulfill_succeeds_once` marked `[x]` already-green regression guards. (k) §10.2 new embedded-backend fulfillment pin `osc52_embedded_backend_fulfills_via_session_pty_responder` added to pin the embedded `PtyResponder` path symmetrically with the daemon `PaneIoThread` path. (l) §10.2 Validation block updated — regression scan targets changed to response_poll/tests + effect_router/tests; stale "flips response polling from dormant to live" language removed. (m) §10.N Accepted audit findings SIZE_VIOLATION count updated 308 → 352. (n) Structural note top-of-file count updated 306 → 352. (o) §10.0 `spec_chain_helper` canonical signature specified as an in-code block. (p) §10.0 `observe_renderable` completion spec clarified to Rung 4 (snapshot) semantics. (q) Scope Clarification E expanded with push-vs-poll architectural note + Section 16 handoff. (r) §10.7 Tests + Validation extended with `osc1337_unknown_file_subop_safely_ignored` (Section 14 de-risking per blind-spot #9). (s) §10.N negative-pins checklist updated for new OSC-1337 pin. (t) §10.N catalog-section extended with Section 16 push-vs-poll handoff note. (u) OSC 3 EFFECT NOTE at line 647 rewritten to reflect `LegacyEventSink` deletion — no exhaustive HostEffect match exists post-§01.3 so adding a new variant is purely additive.
  Basis: fresh_verification | direct_file_inspection | git_log_inspection. Confidence: high.

<!-- Round 28 findings (2026-04-18) — /review-plan Step 6 /tpr-review round 2; both fixed inline -->

- [x] `[TPR-10-120-codex][medium]` `oriterm/src/gpu/extract/from_snapshot/tests.rs:642` — Round-1 negative pin (`snapshot_to_renderable_none_icon_stays_none`) only covered fresh-extract path; did NOT pin the stale-value-reuse case on the refill path. Without this, a future refill implementation that only assigned when source is `Some` would leak the previous frame's icon into the current frame.
  Evidence: `fn snapshot_to_renderable_none_icon_stays_none() { let mut snap = test_snapshot(); let content = snapshot_to_renderable(&snap); assert_eq!(content.mouse_cursor_icon, None); }` — only tests the fresh-extract branch.
  Impact: The negative-pin coverage claim in TPR-10-116's resolution is incomplete; stale-value reuse on refill is a distinct failure mode from the fresh-extract bug that was fixed.
  Required plan update: Added two new regression tests — `snapshot_to_renderable_into_clears_stale_icon` and `extract_frame_from_snapshot_into_clears_stale_icon` — that seed `RenderableContent` / `FrameInput` with `Some(icon)`, call the `*_into` refill with a snapshot carrying `None`, and assert the old icon is cleared (FIXED in this round — 5 regression tests total; all GREEN).
  Basis: fresh_verification | direct_file_inspection. Confidence: high.

- [x] `[TPR-10-121-gemini][medium]` `oriterm_mux/src/protocol/snapshot.rs:345` vs `plans/spec-conformance/section-10-osc-suite.md:426` — Two different entities were sharing the same name `OSC22_KNOWN_ICONS`: the code-side wire constant (`pub const OSC22_KNOWN_ICONS: &[CursorIcon]` — plain slice of icons, for stable u8 indexing on the daemon wire) and the test-matrix slice described in 10.5 (`const OSC22_KNOWN_ICONS: &[(&str, CursorIcon)]` — name-tagged tuples for iterating cursor-name strings in `osc22_all_known_icons_matrix`).
  Evidence: `pub const OSC22_KNOWN_ICONS: &[CursorIcon] = &[CursorIcon::Default, CursorIcon::ContextMenu, ...];` at `oriterm_mux/src/protocol/snapshot.rs:345`.
  Impact: Developer reading the plan would instantiate a `&[(&str, CursorIcon)]` slice also called `OSC22_KNOWN_ICONS`, shadowing or conflicting with the existing wire constant. The two slices are legitimately different (wire-transport vs test-matrix), but the naming collision is a SSOT violation.
  Required plan update: Renamed the test-matrix slice in the plan to `OSC22_TEST_CURSOR_NAMES` with explicit contrast against the wire-side `OSC22_KNOWN_ICONS`. Added a cross-check note that `OSC22_TEST_CURSOR_NAMES` should be a superset of the wire slice's icons so every wire-transportable icon has a matching name-test (FIXED in this round).
  Basis: fresh_verification | direct_file_inspection. Confidence: high.

<!-- Round 27 findings (2026-04-18) — /review-plan Step 6 /tpr-review round 1; 2 fixed inline, 2 dropped as false positives -->

- [x] `[TPR-10-116-codex][high]` `oriterm/src/gpu/extract/from_snapshot/mod.rs:89-96` — Initial daemon snapshot extract path (`snapshot_to_renderable()`) constructed `RenderableContent::default()` and populated fields but NEVER assigned `mouse_cursor_icon`, so first-frame daemon clients always rendered with `None` regardless of what the wire snapshot carried. The refill path (`snapshot_to_renderable_into()` at line 138) correctly decoded it — asymmetric drift between the two paths.
  Evidence: `let mut content = RenderableContent::default(); content.cells = cells; content.cursor = cursor; content.display_offset = snapshot.display_offset as usize; ... content.all_dirty = true; content` — `mouse_cursor_icon` never assigned.
  Impact: A user whose shell fires OSC 22 during the initial prompt would not see the cursor icon on the first frame under daemon mode; the icon would only appear on the next refill. This is a real user-visible bug in the §10.0 partial landing.
  Required plan update: CODE FIX: add `content.mouse_cursor_icon = snapshot.mouse_cursor_icon.and_then(oriterm_mux::protocol::snapshot::decode_cursor_icon);` to `snapshot_to_renderable()` (FIXED in this round — landed at `oriterm/src/gpu/extract/from_snapshot/mod.rs:97-99`). Regression tests `snapshot_to_renderable_populates_mouse_cursor_icon`, `snapshot_to_renderable_into_populates_mouse_cursor_icon`, and `snapshot_to_renderable_none_icon_stays_none` added at `oriterm/src/gpu/extract/from_snapshot/tests.rs` (all GREEN).
  Basis: fresh_verification | direct_file_inspection. Confidence: high.

- [x] `[TPR-10-117-codex][medium]` `plans/spec-conformance/section-10-osc-suite.md:431` — 10.5 daemon pin only asserted `PaneSnapshot::mouse_cursor_icon` round-trips through `server::snapshot`; did NOT cover the client-side decode path. A bug in `snapshot_to_renderable()` that dropped `mouse_cursor_icon` on the first-frame extract would have been invisible to the pin as originally written (see TPR-10-116 for a real instance of this exact bug).
  Evidence: "add `osc22_daemon_snapshot_carries_cursor_icon` — fire OSC 22 at the `Term`, build a `PaneSnapshot` via `server::snapshot`, assert the resulting snapshot's `mouse_cursor_icon` matches the icon set."
  Impact: Rung-4 verification claim in the success criteria would hold on the server side but fail silently on the client decode path. The TPR-10-116 bug would not have been caught by 10.5's tests as written.
  Required plan update: Expanded 10.5 daemon pin to three tests — server-side (`osc22_daemon_snapshot_carries_cursor_icon`), client-side initial extract (`osc22_daemon_snapshot_decode_first_frame` against `snapshot_to_renderable()` / `extract_frame_from_snapshot()`), and client-side refill (`osc22_daemon_snapshot_decode_refill` against `snapshot_to_renderable_into()` / `extract_frame_from_snapshot_into()`) with a note that any one-path-only pin would miss the other (FIXED in this round).
  Basis: fresh_verification | direct_file_inspection. Confidence: high.

- [x] `[TPR-10-118-gemini][medium]` (REJECTED — false positive) — claimed line 484 references `oriterm_core/src/term/handler/osc.rs (set_working_directory) — 3 new tests` and `oriterm_core/src/term/shell_state/mod.rs (Cwd struct) — 1 new test`.
  Evidence: `grep -n 'set_working_directory|Cwd struct' plans/spec-conformance/section-10-osc-suite.md` returned no matches in the claimed shape; line 484 is blank (section separator); line 485+ is 10.7's Files block describing the OSC 1337 sub-ops implementation. The plan references `handler.set_working_directory` in OSC 7 audit-history contexts (lines 749, 780, 910) but not in the shape gemini quoted. No `Cwd struct` exists in the plan or codebase.
  Impact: None — evidence did not match the cited file.
  Required plan update: None — dropped at verification per /tpr-review §4 (gemini LOWER trust: claims not confirmed against actual file content).
  Basis: fresh_verification | direct_file_inspection. Confidence: high (in the rejection).

- [x] `[TPR-10-119-gemini][critical]` (REJECTED — false positive) — claimed line 12 success criterion incorrectly asserts that `observe_renderable` IS NO LONGER A STUB when the code is still a stub.
  Evidence: Line 12 appears in the `success_criteria:` YAML frontmatter which describes the end state the section must achieve when complete. All success criteria in spec-conformance sections are written in present-tense end-state form. The plan's 10.0 partial-landing notes (line 253-254) explicitly list "observe_renderable completion" as BLOCKED on effect-cutover §01.1 — the plan is internally consistent (success criterion describes the target, landing notes record current state).
  Impact: None — success criteria describe the END STATE, not the CURRENT STATE.
  Required plan update: None — dropped at verification per /tpr-review §4 (gemini LOWER trust: confuses end-state success criteria with current-state claims).
  Basis: fresh_verification | direct_file_inspection. Confidence: high (in the rejection).

<!-- Round 26 findings (2026-04-18) — /review-plan Step 6 /tpr-review pass; all fixed inline except TPR-10-109 filed as open -->

- [x] `[TPR-10-106-codex][high]` `plans/spec-conformance/section-08-ecma-48-baseline.md:11` — Section 08 success criterion still claimed it verified basic OSC rows via converted tack scenarios.
  Evidence: "Every basic OSC row (0, 1, 2, 4, 7, 10, 11, 12, 52) is `verified` in `catalog/osc.md` — these rows are verified by converting tack section 06's direct-VTE cap cross-checks into spec_chain tests (subsections 08.1-08.2)."
  Impact: A reader scanning Section 08's success criteria would believe basic OSC rows are owned by Section 08, contradicting Section 10's mission and the Section 08 post-completion audit which recorded zero OSC coverage from tack scenarios.
  Required plan update: Section 08 success criterion rewritten to assign basic OSC row ownership to Section 10 explicitly, citing the post-completion audit note (FIXED in this round).
  Basis: fresh_verification | direct_file_inspection. Confidence: high.

- [x] `[TPR-10-107-codex][medium]` `plans/spec-conformance/section-10-osc-suite.md:188` — 10.0's `response_poll.rs` Files directive was self-contradictory: it said to "add the activation call in `PaneIoThread::drain_commands`" but immediately noted "which already calls `self.poll_pending_responses()` at line 211".
  Evidence: Line 188 (before fix): "remove the `#[allow(dead_code)]` gate; add the activation call in `PaneIoThread::drain_commands` — the real method at `oriterm_mux/src/pane/io_thread/mod.rs:194` — which already calls `self.poll_pending_responses()` at line 211".
  Impact: Implementer would waste time looking for a poll-call wiring task that is already done (verified `self.poll_pending_responses();` at `oriterm_mux/src/pane/io_thread/mod.rs:211`). The real task is just dead-code gate removal + wiring the live `register_host_request_response` call site from the OSC 52 handler path.
  Required plan update: Line 188 rewritten to drop the "add the activation call" phrasing and focus on dead-code gate removal + `register_host_request_response` wiring (FIXED in this round).
  Basis: fresh_verification | direct_file_inspection. Confidence: high.

- [x] `[TPR-10-108-codex][low]` `plans/spec-conformance/section-10-osc-suite.md:248` — Partial-landing note arithmetic: claimed "11 TDD tests GREEN" but the itemized breakdown is 9 + 3 + 2 = 14.
  Evidence: "11 TDD tests GREEN: 9 OSC 1337 dispatch tests in `crates/vte/src/ansi/dispatch/tests.rs`, 3 Term mouse-cursor-icon tests + 2 injectable-clock tests in `oriterm_core/src/term/tests.rs`."
  Impact: Count mismatch erodes audit-trail trust for the partial-landing block.
  Required plan update: Total corrected from 11 to 14 (FIXED in this round).
  Basis: fresh_verification | direct_file_inspection. Confidence: high.

- [x] `[TPR-10-109-codex][medium]` `plans/spec-conformance/section-10-osc-suite.md:1434` — SIZE_VIOLATION (308 top-level items) remains as a documented exception per the Structural Note's anchor-preservation rationale; reviewer suggested moving audit-history (10.R) and closeout detail (10.N) into subordinate files to meet the 20-item cap.
  Evidence: Line 1434 accepts the 308-item SIZE_VIOLATION with rationale "the 20-item heuristic is a 'subsection or split' prompt and this section has taken the subsection path. Do not re-litigate without a concrete TPR-anchor-preservation plan."
  Impact: Section 10 continues to carry a standing size-exception that other sections do not inherit. Anchor-preservation considerations are load-bearing: 25+ rounds of TPR citations in 10.R reference line numbers in this file, and splitting would break those references. The recommended fix (move 10.R / 10.N to subordinate files) is plausible but requires an anchor-migration plan before execution.
  Resolution: Accepted on 2026-04-18 with blocked-task anchor. The SIZE_VIOLATION is real and not a false positive. The fix (split 10.R / 10.N into subordinate files) is legitimately blocked because it would break 25+ rounds of TPR citation anchors unless executed under an anchor-migration strategy. The existing `- [ ]` entry at §10.N ("Accepted audit findings (documented exceptions)", line 1541) is the canonical tracking artifact; it has been updated in the same commit to carry a `<!-- blocked-by:anchor-migration-plan -->` anchor and an explicit unblock condition — a targeted `/review-plan` follow-up that produces an anchor-migration strategy (split proposal + anchor-rewrite plan) MUST land before executing the split. `/review-bugs` and `/fix-next-bug` can now pick up the anchored task directly from §10.N. The `third_party_review.status` remains `findings` (not `resolved`) because the anchored task is still open; status transitions to `resolved` only when the split executes or a permanent exception is ratified.
  Basis: fresh_verification | direct_file_inspection. Confidence: medium.

- [x] `[TPR-10-110-codex][medium]` `plans/spec-conformance/section-10-osc-suite.md:287,320,352,401,440,468,514,578` — Subsection Validation blocks (10.1–10.9) did not enforce `./build-all.sh` + `./clippy-all.sh` gates after subsection changes, only `./test-all.sh`.
  Evidence: 10.1 Validation (pre-fix): `- [ ] ./test-all.sh green.` — no build-all or clippy-all check. Similar shape across 10.2–10.9.
  Impact: CLAUDE.md mandates `./build-all.sh` + `./clippy-all.sh` + `./test-all.sh` "after EVERY change"; subsection-level validation that only gates test-all lets cross-compile and clippy regressions slip until 10.N Final Verification.
  Required plan update: Added a uniform `- [ ] ./build-all.sh + ./test-all.sh + ./clippy-all.sh green after this subsection's changes` line to each of 10.1–10.9 Validation blocks, with a note that section-level `/tpr-review` + `/impl-hygiene-review` remain gated at 10.N's Final Verification (FIXED in this round). Per-subsection `/tpr-review` / `/impl-hygiene-review` was NOT added — those are section-level gates per Plan-section rigor, and multiplying them per subsection has no substantive benefit over a single section-level run. If a narrower per-subsection TPR is useful it is enumerated in the existing "TPR checkpoint 1/2/3" items at 10.0/10.3/10.7.
  Basis: fresh_verification | direct_file_inspection. Confidence: high.

- [x] `[TPR-10-111-gemini][medium]` `plans/spec-conformance/section-10-osc-suite.md:606` — Banned phrase "OUT OF SCOPE" used in Tektronix scope note.
  Evidence: "Tektronix color rows (OSC 15, OSC 16, OSC 18, and their resets OSC 115 / 116 / 118) — OUT OF SCOPE for Section 10" and follow-on text "are out of scope".
  Impact: `.claude/rules/impl-hygiene.md` §Banned Phrases bars the use of "out of scope" as framing; the phrase routes reviewer attention around a deliberate decision rather than stating it positively.
  Required plan update: Rewrote to "intentionally omitted from Section 10 scope" and follow-on to "intentionally omitted" with the existing rationale preserved (FIXED in this round).
  Basis: fresh_verification | direct_file_inspection. Confidence: high.

- [x] `[TPR-10-112-gemini][low]` `plans/spec-conformance/section-10-osc-suite.md:1435` — Banned phrase "out of scope" used in BLOAT_RISK audit-resolution for plan docs.
  Evidence: "splitting is out of scope for Section 10 (each would require its own dedicated /review-plan cycle)."
  Impact: Same Banned Phrases violation as TPR-10-111.
  Required plan update: Rewrote to "splitting is outside Section 10's ownership" with the same rationale intact (FIXED in this round).
  Basis: fresh_verification | direct_file_inspection. Confidence: high.

- [x] `[TPR-10-113-gemini][low]` `plans/spec-conformance/section-10-osc-suite.md:1366` — Stale CWD field citation: `oriterm_core/src/term/mod.rs:147` (actual line is 148 post §10.0 partial landing).
  Evidence: `wc -l oriterm_core/src/term/mod.rs` → 488 lines; `grep -n 'cwd: Option' oriterm_core/src/term/mod.rs` → line 148.
  Impact: Developer running the field-anchored grep would see it at line 148 and incorrectly suspect drift between plan and code.
  Required plan update: Line citation updated from 147 to 148 (FIXED in this round).
  Basis: fresh_verification | direct_file_inspection. Confidence: high.

- [x] `[TPR-10-114-gemini][informational]` `plans/spec-conformance/section-10-osc-suite.md:1371,1372` — Stale file-line counts in the "No file size violations (source files)" validation block: `oriterm_core/src/term/mod.rs` cited as 468 lines (actual: 488) and `oriterm_core/src/term/handler/mod.rs` cited as 438 lines (actual: 442).
  Evidence: `wc -l oriterm_core/src/term/mod.rs oriterm_core/src/term/handler/mod.rs` → 488 / 442.
  Impact: Count drift makes the "Count projected new lines BEFORE landing" guardrail harder to apply accurately.
  Required plan update: Cited counts updated to 488 / 442 with a "(post §10.0 partial landing)" annotation and the landed methods enumerated so future counts account for the delta (FIXED in this round).
  Basis: fresh_verification | direct_file_inspection. Confidence: high.

- [x] `[TPR-10-115-gemini][low]` `plans/spec-conformance/section-10-osc-suite.md:10` — Section 14 ownership-conflict citation stale: the frontmatter success criterion cited "the ownership conflict currently at `plans/spec-conformance/section-14-iterm2-images.md:55` and `plans/spec-conformance/catalog/iterm2.md:15-20`" but the conflict has been resolved (catalog `owner_section` now reads `01 (bootstrap), 10 (non-image), 14 (image)` and Section 14 line 55 explicitly defers non-image 1337 to Section 10).
  Evidence: `plans/spec-conformance/catalog/iterm2.md:5` → `"01 (bootstrap), 10 (non-image), 14 (image)"`; `plans/spec-conformance/section-14-iterm2-images.md:55` → "Section 10's OSC suite covered the non-image OSC 1337 variants; this section covers the image variants."
  Impact: Referring to a resolved state as "currently" creates false drift signal in future reviews.
  Required plan update: Success criterion rewritten to reference the resolved state ("`owner_section` ... is `01 (bootstrap), 10 (non-image), 14 (image)`") with positive cross-checks rather than "ownership conflict" language (FIXED in this round).
  Basis: fresh_verification | direct_file_inspection. Confidence: high.

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

- [x] `[TPR-10-42-codex][high]` `plans/spec-conformance/section-10-osc-suite.md:6` — Goal field and scope clarification B incorrectly claimed `RawInterceptor` is "the production path for OSC 7/9/99/133/633/777"; OSC 633 is currently MISSING per `plans/spec-conformance/catalog/osc.md:56` and is NOT handled by `RawInterceptor`.
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
  Evidence: `plans/spec-conformance/catalog/osc.md:16-34` — all existing test-chain entries use `parser:pending dispatch:pending state:pending`; no entry uses `passed`.
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

- [x] `[TPR-10-49-codex][low]` `plans/spec-conformance/section-10-osc-suite.md:307` — 10.3 catalog update note said "newly-added OSC-99 + OSC-777 rows"; OSC-777 already exists in `plans/spec-conformance/catalog/osc.md` as a `missing` row, only OSC-99 is truly new.
  Evidence: `plans/spec-conformance/catalog/osc.md:57` — `| OSC-777 | urxvt notifications | ... | missing |` — row already present; only OSC-99 is absent from the catalog.
  Impact: Implementer following the note would attempt to add a duplicate OSC-777 row, or be confused about whether the row needs creation vs. promotion.
  Required plan update: Note corrected to distinguish: OSC-777 already exists (promote from `missing` to `verified`); OSC-99 must be added as a new row (FIXED).
  Basis: direct_file_inspection. Confidence: high.

- [x] `[TPR-10-50-codex][low]` `plans/spec-conformance/section-10-osc-suite.md:483` — 10.7 validation count "All 10 tests green" does not match the 12 test bullets in the 10.7 Tests block.
  Evidence: 10.7 Tests block has: 7 behavioral tests + `osc1337_file_still_routes_to_iterm2_file` + `osc1337_unknown_key_dropped` + `osc1337_user_vars_cap_evicts_oldest` + 2 CWD SSOT semantic pins = 12 total.
  Impact: An implementer who stops at 10 tests believes the matrix is complete when 2 tests are missing.
  Required plan update: Validation count corrected to 12 with an enumeration of all 12 test names (FIXED).
  Basis: direct_file_inspection. Confidence: high.

- [x] `[TPR-10-51-codex][low]` `plans/spec-conformance/section-10-osc-suite.md:10` — Success criterion cites `plans/spec-conformance/catalog/iterm2.md:14` for ownership conflict; line 14 is the FILE row (image row, not a conflict); `owner_section` is at line 5 and the non-image rows are at lines 15-20.
  Evidence: `plans/spec-conformance/catalog/iterm2.md:5` — `owner_section: "01 (bootstrap), 14 (verification)"` (the field to update); line 14 is the FILE row header.
  Impact: An implementer looking at `plans/spec-conformance/catalog/iterm2.md:14` would see the image row, not the ownership conflict location.
  Required plan update: Citation corrected to `plans/spec-conformance/catalog/iterm2.md:5` for `owner_section` and `plans/spec-conformance/catalog/iterm2.md:15-20` for the non-image rows (FIXED).
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

- [x] `[TPR-10-54-codex][medium]` `plans/spec-conformance/section-10-osc-suite.md:472` — 10.7 catalog update block updated `plans/spec-conformance/catalog/iterm2.md` rows but omitted the `plans/spec-conformance/catalog/shell-integration.md` cross-ref rows (SHINT-OSC-1337-REMOTEHOST, SHINT-OSC-1337-CURRENTDIR, etc.) promised in success criterion 2 ("Every row in `plans/spec-conformance/catalog/shell-integration.md` is `verified` (... OSC-1337-RemoteHost / CurrentDir / SetMark / SetUserVar / ReportCellSize shell-integration cross-refs ...)").
  Evidence: `plans/spec-conformance/section-10-osc-suite.md:9` — success criterion explicitly lists OSC-1337 shell-integration cross-refs; 10.7 catalog block had no corresponding `plans/spec-conformance/catalog/shell-integration.md` tasks.
  Impact: The shell-integration catalog would remain incomplete after Section 10, violating the stated success criterion.
  Required plan update: 10.7 catalog block extended with tasks to add/update SHINT-OSC-1337-* rows in `plans/spec-conformance/catalog/shell-integration.md` (FIXED).
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
  Evidence: Line 307: "OSC-9 and OSC-777 rows already exist (both marked `missing`; promote to `verified`)"; Line 323: "New rows OSC-99, OSC-777 added to `plans/spec-conformance/catalog/osc.md`"
  Impact: An implementer adding OSC-777 as a new row would create a duplicate row, corrupting the catalog.
  Required plan update: Catalog step corrected to "New row OSC-99 added; existing OSC-777 promoted from `missing` to `verified`" (FIXED).
  Basis: direct_file_inspection. Confidence: high.

- [x] `[TPR-10-58-codex][low]` `plans/spec-conformance/section-10-osc-suite.md:480` — 10.7 flow-up note still cited `plans/spec-conformance/catalog/iterm2.md:14` for the ownership conflict location; the correct citation is `plans/spec-conformance/catalog/iterm2.md:5` for `owner_section` and `plans/spec-conformance/catalog/iterm2.md:15-20` for the non-image rows (same error that TPR-10-51 fixed in the success criteria at line 10, but the 10.7 body prose was not updated).
  Evidence: Line 480: "catalog/iterm2.md:14 said those variants are assigned to Section 14" — line 14 is the FILE (image) row header, not the ownership field.
  Impact: An implementer looking at `plans/spec-conformance/catalog/iterm2.md:14` sees the image row, not the ownership conflict location.
  Required plan update: 10.7 flow-up note corrected to cite `plans/spec-conformance/catalog/iterm2.md:5` for `owner_section` and `plans/spec-conformance/catalog/iterm2.md:15-20` for the non-image rows (FIXED).
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

<!-- Round 15 findings (2026-04-17) — survivor mode: codex only (gemini transport failure: file reads returned empty) -->

- [x] `[TPR-10-69-codex][high]` `plans/spec-conformance/section-10-osc-suite.md:17` — Success criterion implied OSC 633 dispatch lands in BOTH `crates/vte/src/ansi/dispatch/osc.rs` (high-level processor path) AND `oriterm_mux/src/shell_integration/interceptor.rs`, creating a duplicated-dispatch LEAK.
  Evidence: "lands its dispatch arm in `crates/vte/src/ansi/dispatch/osc.rs` AND its handler in `oriterm_mux/src/shell_integration/interceptor.rs`" — `RawInterceptor` implements `vte::Perform` directly, not via the high-level `osc.rs` dispatch; adding to `osc.rs` would mean the high-level processor also fires on OSC 633 bytes, producing double-handling.
  Impact: Adding OSC 633 to both paths would cause double-dispatch on every OSC 633 sequence in production: interceptor fires first (via raw parser), then the high-level Processor fires again (via osc.rs arm) — both on the same bytes.
  Required plan update: Success criterion rewritten to specify OSC 633 dispatch goes EXCLUSIVELY to the interceptor (`oriterm_mux/src/shell_integration/interceptor.rs`); explicitly forbids adding a `b"633"` arm to `crates/vte/src/ansi/dispatch/osc.rs`; negative pin `osc633_via_high_level_processor_drops` added to the 10.4 test list (FIXED).
  Basis: fresh_verification | direct_file_inspection. Confidence: high.

- [x] `[TPR-10-70-codex][medium]` `plans/spec-conformance/section-10-osc-suite.md:179` — 10.0 Files block cited `PaneIoThread::drain_events` as the hook site for the response-poll activation call; this method does not exist — the real method is `drain_commands()` at `oriterm_mux/src/pane/io_thread/mod.rs:194`, which already calls `self.poll_pending_responses()` at line 211.
  Evidence: `oriterm_mux/src/pane/io_thread/mod.rs:194` — `fn drain_commands(&mut self)` is the real method; no `drain_events` method exists in `PaneIoThread`.
  Impact: An implementer searching for `drain_events` to add the activation call would find nothing and either create a new method (wrong) or be confused about where to wire the call.
  Required plan update: `drain_events` replaced with `drain_commands`; note added that `poll_pending_responses()` is already called at `mod.rs:211` inside `drain_commands` — the activation may only require removing the `#[allow(dead_code)]` gate on `register_host_request_response` and verifying the existing call site is reachable (FIXED).
  Basis: fresh_verification | direct_file_inspection. Confidence: high.

- [x] `[TPR-10-71-codex][medium]` `plans/spec-conformance/section-10-osc-suite.md:366` — OSC 633 E sub-command directs adding `Term::last_command_line: Option<String>` field but `oriterm_core/src/term/mod.rs` was NOT listed in 10.4's Files block — a DRIFT violation; the field addition, accessor, and mutator need canonical file tracking.
  Evidence: 10.4 Files block (lines 337-341) listed only `oriterm_mux/src/shell_integration/tests.rs`, `oriterm_mux/src/shell_integration/interceptor.rs`, `crates/vte/src/ansi/dispatch/osc.rs`, and catalog files — `oriterm_core/src/term/mod.rs` (where the field lives) was absent.
  Impact: An implementer following the 10.4 plan would modify `Term` without the Files block listing it as a touch target, risking missing the accessor/mutator additions and causing the RecordingHandler REGISTRATION SYNC drift (also absent from the 10.4 Files block).
  Required plan update: `oriterm_core/src/term/mod.rs`, `oriterm_core/src/term/handler/mod.rs`, and `crates/oriterm_test_support/src/spec_chain/recording_handler.rs` added to 10.4 Files block with DRIFT and REGISTRATION SYNC notes (FIXED).
  Basis: fresh_verification | direct_file_inspection. Confidence: high.

- [x] `[TPR-10-72-codex][medium]` `plans/spec-conformance/section-10-osc-suite.md:566` — OSC 3 EFFECT NOTE listed `oriterm_eval` as a `HostEffect` consumer when `oriterm_eval` does not exist as a crate in this workspace; real consumers are `oriterm_core/src/effect/sink/legacy/mod.rs` and `oriterm_mux` event processing.
  Evidence: `oriterm_eval` does not appear in any `Cargo.toml` in the workspace; real `HostEffect` match arms are at `oriterm_core/src/effect/sink/legacy/mod.rs:104-145`.
  Impact: An implementer adding `HostEffect::SetX11Property` per Option A would search `oriterm_eval` for match arms to update, find nothing, and miss the real consumers — producing compile errors on `non-exhaustive patterns`.
  Required plan update: `oriterm_eval` removed; replaced with the real consumers: `oriterm_core/src/effect/sink/legacy/mod.rs:104-145` and `oriterm_mux` event-processing match arms (FIXED).
  Basis: fresh_verification | direct_file_inspection. Confidence: high.

<!-- Round 16 findings (2026-04-17) — survivor mode: codex only (gemini transport failure: model not found / no capacity both attempts) -->

- [x] `[TPR-10-73-codex][high]` `plans/spec-conformance/section-10-osc-suite.md:366` — OSC 633 high-level drop negative pin `osc633_via_high_level_processor_drops` was promised in R15 fixed entry (TPR-10-69) but never added to the concrete 10.4 test bullets or the 10.N negative-pin checklist.
  Evidence: 10.4 test list ends at `osc633_e_records_command_line` with no `osc633_via_high_level_processor_drops` bullet; 10.N checklist had no OSC 633 negative pin entry.
  Impact: The double-dispatch regression guard is absent; someone adding a `b"633"` arm to the high-level osc.rs would go undetected.
  Required plan update: `osc633_via_high_level_processor_drops` bullet added to 10.4 test list; corresponding item added to 10.N negative-pins checklist (FIXED).
  Basis: fresh_verification | direct_file_inspection. Confidence: high.

- [x] `[TPR-10-74-codex][high]` `plans/spec-conformance/section-10-osc-suite.md:341` — 10.4 Files block listed `Handler::set_last_command_line` override on Term in `handler/mod.rs` and a recording_handler delegate, but OSC 633 is interceptor-only; the interceptor calls `term.set_last_command_line()` directly (not via Handler trait dispatch), so no trait method or recording_handler arm is needed.
  Evidence: `oriterm_mux/src/shell_integration/interceptor.rs:31-66` — interceptor implements `vte::Perform::osc_dispatch` and calls `self.term.<method>()` directly; no Handler trait dispatch occurs.
  Impact: Adding `Handler::set_last_command_line` to the trait would create a vestigial dispatch path contradicting the interceptor-only architecture.
  Required plan update: Lines 341-342 removed from 10.4 Files block; explanatory NOTE added clarifying interceptor-only access pattern (FIXED).
  Basis: fresh_verification | direct_file_inspection. Confidence: high.

- [x] `[TPR-10-75-codex][high]` `plans/spec-conformance/section-10-osc-suite.md:352` — OSC 133 D test described `spec_chain_helper` passing `Some(t0 + 1500ms)` as `now` to `finish_command`, but the interceptor calls `finish_command(None)` — there is no mechanism to inject `now` via the byte-feed path; the exact-duration assertion would be non-deterministic or require a test-only injection not described in the plan.
  Evidence: `oriterm_mux/src/shell_integration/interceptor.rs:109` — `if let Some(duration) = self.term.finish_command()` — after Option A refactor this becomes `finish_command(None)`; `None.unwrap_or_else(Instant::now)` uses wall-clock.
  Impact: Test asserting `duration == 1500ms` via the interceptor feed path would be flaky.
  Required plan update: Test rewritten to NOT assert exact duration from the feed path; instead asserts `CommandComplete { .. }` presence and `duration >= Duration::ZERO`; exact-duration pin moved to a direct shell_state unit test (FIXED).
  Basis: fresh_verification | direct_file_inspection. Confidence: high.

- [x] `[TPR-10-76-codex][medium]` `plans/spec-conformance/section-10-osc-suite.md:533` — OSC 7 remediation offered `assert!(!reachable)` / `debug_assert` as a valid option for the vestigial `b"7"` arm; the arm DOES fire on valid OSC 7 bytes, so this assertion would panic on valid user input.
  Evidence: `crates/vte/src/ansi/dispatch/osc.rs:69` — `b"7" => { ... handler.set_working_directory(Some(uri)); }` — arm fires on any `OSC 7` sequence received.
  Impact: An implementer adding `assert!(!reachable)` would ship production code that panics on valid terminal input, violating impl-hygiene.md §Panic & Assertion.
  Required plan update: `assert!(!reachable)` / `debug_assert` option removed; only valid options are delete the arm or leave a SSOT comment with no assertion (FIXED).
  Basis: fresh_verification | direct_file_inspection. Confidence: high.

- [x] `[TPR-10-77-codex][medium]` `plans/spec-conformance/section-10-osc-suite.md:568` — OSC 3 EFFECT NOTE listed `QueueingEffectSink` as a match-arm consumer requiring update when adding `HostEffect::SetX11Property`; `QueueingEffectSink::push()` queues effects opaquely and has no variant-matching logic.
  Evidence: `oriterm_core/src/effect/sink/mod.rs:72-75` — `impl EffectSink for QueueingEffectSink { fn push(&self, effect: Effect) { self.queue.lock().push(effect); } }` — no match on `HostEffect`.
  Impact: An implementer spending time auditing `QueueingEffectSink` for match arms to update would find nothing and be confused about whether the fan-out was complete.
  Required plan update: `QueueingEffectSink` removed from the consumer list; note added that it queues opaquely and needs no update when `HostEffect` gains new variants (FIXED).
  Basis: fresh_verification | direct_file_inspection. Confidence: high.

<!-- Round 17 findings (2026-04-17) — survivor mode: codex only (gemini transport failure: model not found / no capacity both attempts) -->

<!-- Round 18 findings (2026-04-17) — survivor mode: codex only (gemini transport failure: model not found / no capacity both attempts) -->

- [x] `[TPR-10-82-codex][medium]` `plans/spec-conformance/section-10-osc-suite.md:292` — 10.2 catalog update only says "→ verified" for OSC-52-STORE and OSC-52-LOAD, leaving stale `Implementation`/`Apex layer`/`Notes` cells in `plans/spec-conformance/catalog/osc.md` that still reference the old `Event::ClipboardStore` / `Event::ClipboardLoad` closure wording.
  Evidence: `plans/spec-conformance/catalog/osc.md:31-32` — `Emits Event::ClipboardStore` / `Emits Event::ClipboardLoad with a response-formatting closure` — both cells reference the pre-Effect-boundary API that no longer reflects production code.
  Impact: An implementer reading the catalog after 10.2 would see correct verification status but incorrect implementation description, causing confusion and potential DRIFT against future catalog consumers.
  Required plan update: 10.2 catalog block extended with explicit CATALOG METADATA UPDATE steps to rewrite Implementation/Apex/Notes cells to the current `HostEffect::ClipboardStore` / `HostRequest::ClipboardLoad + ResponseToken` path before marking the rows `verified` (FIXED).
  Basis: direct_file_inspection. Confidence: high.

- [x] `[TPR-10-83-codex][medium]` `plans/spec-conformance/section-10-osc-suite.md:548` — 10.8 catalog update only says "every row → verified" without explicitly updating the OSC-7 `Implementation` cell, which still cites the high-level `b"7"` arm that 10.8 removes.
  Evidence: `plans/spec-conformance/catalog/osc.md:21` — `b"7"` arm → `Handler::set_working_directory` default impl` — this is the arm 10.8 deletes; if the catalog update omits rewriting this cell, it becomes a DRIFT finding.
  Impact: The catalog row for OSC-7 would describe a dispatch path that no longer exists after Section 10 implementation, creating a false reference.
  Required plan update: 10.8 catalog block extended with explicit OSC-7 METADATA UPDATE steps to rewrite Implementation/Notes to the interceptor-only path after the high-level arm is removed (FIXED).
  Basis: direct_file_inspection. Confidence: high.

- [x] `[TPR-10-84-codex][low]` `plans/spec-conformance/section-10-osc-suite.md:413` — 10.5 catalog update only says "OSC-22 → verified" without updating the `Implementation` and `Apex layer` cells that still describe the stub no-op.
  Evidence: `plans/spec-conformance/catalog/osc.md:29` — `Handler::set_mouse_cursor_icon default impl` / `Apex layer: effect-host-notification` — stale after 10.5 adds a real Term override.
  Impact: The catalog row for OSC-22 would show `verified` status but incorrect implementation description referencing the deleted stub.
  Required plan update: 10.5 catalog block extended with explicit METADATA UPDATE steps to rewrite Implementation/Apex/Notes cells to the `Term::set_mouse_cursor_icon` + `state-snapshot` path before promoting to `verified` (FIXED).
  Basis: direct_file_inspection. Confidence: high.

- [x] `[TPR-10-78-codex][high]` `plans/spec-conformance/section-10-osc-suite.md:352` — `osc133_d_clears_state_and_emits_command_complete` test setup only fed `OSC 133;C`, but the marker assertions (`prompt_markers().last()` has A/B/C fields) require a full A→B→C lifecycle AND deferred mark helpers to have been invoked; with only C fed, `prompt_markers` is empty and the assertions would panic on `unwrap()`.
  Evidence: `oriterm_core/src/term/shell_state/mod.rs:56-103` — `mark_prompt_row()`, `mark_command_start_row()`, `mark_output_start_row()` are the only code paths that push entries to `prompt_markers`; they require the respective pending flags set by A/B/C AND explicit invocation (done by `post_parse_housekeeping` in production).
  Impact: A test written following the old plan setup would fail at `prompt_markers().last().unwrap()` because the vec is empty, masking the real D-semantics under a setup bug.
  Required plan update: Setup changed to feed full A→B→C and call deferred mark helpers before feeding D; a note explains the production vs test deferred-mark path (FIXED).
  Basis: fresh_verification | direct_file_inspection. Confidence: high.

- [x] `[TPR-10-79-codex][high]` `plans/spec-conformance/section-10-osc-suite.md:563` — 10.9 Files block cited `oriterm_core/src/term/handler/osc.rs` as the target for "override on Term for state-mutating variants"; the actual `impl Handler for Term<S>` is in `handler/mod.rs:30`, not `handler/osc.rs` (which contains helper methods on `impl<S: EffectSink> Term<S>`).
  Evidence: `oriterm_core/src/term/handler/mod.rs:30` — `impl<S: EffectSink> Handler for Term<S> {` — the trait impl is here; `handler/osc.rs:19` — `impl<S: EffectSink> Term<S> {` — no Handler impl.
  Impact: An implementer following the plan would add Handler overrides to `osc.rs`, where they would not compile as trait impl methods.
  Required plan update: 10.9 Files block updated to direct Handler method overrides to `handler/mod.rs`; `handler/osc.rs` cited for helper logic; `term/mod.rs` added for new Term fields; `recording_handler.rs` added with REGISTRATION SYNC note (FIXED).
  Basis: fresh_verification | direct_file_inspection. Confidence: high.

- [x] `[TPR-10-80-codex][medium]` `plans/spec-conformance/section-10-osc-suite.md:569` — OSC 3 EFFECT NOTE still cited "the HostEffect match arm in `oriterm_mux` event processing" as a required fan-out update; no such exhaustive `HostEffect` match arm exists in `oriterm_mux` (only constructors in `interceptor.rs`).
  Evidence: `grep -n "HostEffect" oriterm_mux/src/ -r` returns only `interceptor.rs` constructors and `tests.rs` — no match arm; the only exhaustive consumer is `oriterm_core/src/effect/sink/legacy/mod.rs:104-145`.
  Impact: An implementer searching `oriterm_mux` for a `HostEffect` match to update when adding `SetX11Property` would find nothing, creating confusion about whether the fan-out is complete.
  Required plan update: False `oriterm_mux` consumer reference removed; note clarified that `LegacyEventSink` is the only in-tree exhaustive `HostEffect` match requiring update (FIXED).
  Basis: fresh_verification | direct_file_inspection. Confidence: high.

- [x] `[TPR-10-81-codex][medium]` `plans/spec-conformance/section-10-osc-suite.md:1096` — 10.N crate-ordering checklist listed only `mouse_cursor_icon`, `remote_host`, `user_vars`, `shell_integration_version` as Term fields; `last_command_line` (added by 10.4 for OSC 633 E) and `tab_title_color` (added by 10.9 for OSC 6) were omitted, plus the full set of 10.9 Term fields.
  Evidence: Section 10.4 at line 340 adds `last_command_line: Option<String>` to `term/mod.rs`; section 10.9 at line 571 adds `Term::tab_title_color`; neither appears in the crate-ordering checklist.
  Impact: An implementer following the checklist as a completion gate would not realize these fields also need to land in `oriterm_core` before the downstream crates build on them.
  Required plan update: Checklist expanded to enumerate all Term fields this section adds, including `last_command_line` and the 10.9 OSC fields (FIXED).
  Basis: fresh_verification | direct_file_inspection. Confidence: high.

<!-- Round 19 findings (2026-04-17) — survivor mode: codex only (gemini transport failure: no capacity both attempts) -->

- [x] `[TPR-10-85-codex][high]` `plans/spec-conformance/section-10-osc-suite.md:249` — `osc8_survives_scrollback` cited `term.grid().scrollback()[row][col]` as the access pattern; `ScrollbackBuffer` does NOT implement `Index` (only `.get(index) -> Option<&Row>`), and `Row` implements `Index<Column>` not `Index<usize>`, so `[col: usize]` on a row also fails to compile.
  Evidence: `oriterm_core/src/grid/ring/mod.rs:90` — `pub fn get(&self, index: usize) -> Option<&Row>` — no `impl Index` on `ScrollbackBuffer`; `oriterm_core/src/grid/row/mod.rs:175` — `impl Index<Column> for Row` — takes `Column`, not `usize`.
  Impact: An implementer following the plan would write `scrollback()[row][col]` which fails to compile at both dimensions.
  Required plan update: Replaced `scrollback()[row][col]` with `scrollback().get(idx).unwrap()` for the row, then `row[Column(col)]` for the column; noted that `Column` is a newtype `Column(usize)` (FIXED).
  Basis: fresh_verification | direct_file_inspection. Confidence: high.

- [x] `[TPR-10-86-codex][medium]` `plans/spec-conformance/section-10-osc-suite.md:357` — `osc133_a_without_b_does_not_record_command` and `osc133_full_lifecycle_records_markers` assert on `prompt_markers()` but did not mention calling the deferred-mark helpers (`term.mark_prompt_row()`, `term.mark_command_start_row()`, `term.mark_output_start_row()`) after each feed; without those calls `prompt_markers` remains empty and the assertions panic on `unwrap()` for the wrong reason.
  Evidence: `oriterm_core/src/term/shell_state/mod.rs:56,80,94` — `pub fn mark_prompt_row(&mut self)` / `mark_command_start_row` / `mark_output_start_row` — these are the only paths that push to `prompt_markers`; the interceptor only sets pending flags.
  Impact: A test written without the deferred-mark calls would fail at `prompt_markers().last().unwrap()` with the vec empty, masking the real test intent.
  Required plan update: Deferred-mark helper calls added to both test bullets; explanatory note mirrors the existing D-test setup note (FIXED).
  Basis: fresh_verification | direct_file_inspection. Confidence: high.

- [x] `[TPR-10-87-codex][medium]` `plans/spec-conformance/section-10-osc-suite.md:583` — 10.9 test block had no negative pins, violating `.claude/rules/tests.md` §Negative Testing Protocol (MANDATORY); no must-reject tests existed for malformed color specs, invalid RGB values, or platform-conditional OSC 3 behavior.
  Evidence: 10.9 Tests block (lines 583-585) — only three bullets: count, cross-reset, completeness scan; no negative pins for invalid inputs.
  Impact: A regression where the OSC 13/14/17/19 handlers accept garbage color specs would go undetected; OSC 3 on Windows could panic instead of no-op.
  Required plan update: Seven negative pin tests added to 10.9 with explicit test names; entries noted for 10.N negative-pins checklist (FIXED).
  Basis: direct_file_inspection. Confidence: high.

- [x] `[TPR-10-88-codex][medium]` `plans/spec-conformance/section-10-osc-suite.md:36` — `depends_on: ["03", "08"]` does not list the effect-cutover plan, but Section 10.2's body explicitly states 10.2 CANNOT complete without the effect-cutover plan migrating the IO thread to `QueueingEffectSink`.
  Evidence: Line 218: "Section 10 CANNOT simply remove the dead-code gate without also migrating the IO thread to `QueueingEffectSink`... Coordinate Section 10.2 implementation with the effect-cutover plan" — but `depends_on` at line 36 lists only `["03", "08"]`.
  Impact: An implementer checking section metadata for prerequisites would not be alerted to the effect-cutover dependency, potentially starting 10.2 without the prerequisite in place.
  Required plan update: `depends_on` updated to include `"effect-cutover"` to match the documented dependency in the 10.2 body (FIXED).
  Basis: direct_file_inspection. Confidence: high.

<!-- Round 21 findings (2026-04-17) -->

- [x] `[TPR-10-89-codex][high]` `plans/spec-conformance/section-10-osc-suite.md:208` — `observe_renderable` plan uses Rung 3 (live-state) accessors for `palette_index` and `mouse_cursor_icon` instead of Rung 4 (`RenderableContent` snapshot) accessors.
  Evidence: Line 208 directs `term.palette().color(index)` for `palette_index`, bypassing `RenderableContent::palette_snapshot`; line 209 directs `Term::mouse_cursor_icon()`, bypassing `RenderableContent::mouse_cursor_icon` (added at line 215). `RenderableContent` already has `palette_snapshot: Vec<[u8; 3]>` populated by `fill_palette_snapshot()` in `renderable_content_into()` (`oriterm_core/src/term/snapshot.rs:181-188`).
  Impact: `observe_renderable` would test live `Term` state (Rung 3) not the renderable snapshot path (Rung 4). A bug in `fill_palette_snapshot` or the `mouse_cursor_icon` writeback path would produce a false-green Rung 4 result.
  Required plan update: Rewritten to use `term.renderable_content()` snapshot path — `palette_snapshot[index]` and `rc.mouse_cursor_icon` from the snapshot (FIXED in this round).
  Basis: fresh_verification | direct_file_inspection. Confidence: high.

- [x] `[TPR-10-90-codex][medium]` `plans/spec-conformance/section-10-osc-suite.md:403` — OSC 22 `mouse_cursor_icon` is added to `Term` and `RenderableContent` (embedded path) but `PaneSnapshot` (`oriterm_mux/src/protocol/snapshot.rs:160`) has no corresponding field, leaving a GAP in the daemon-mode transport path.
  Evidence: `PaneSnapshot` struct (lines 160-188) carries `cwd`, `title`, `palette`, `cursor` but no `mouse_cursor_icon` field. In daemon mode the client renders from `PaneSnapshot` and has no `Term`. Daemon-mode clients would be blind to OSC 22 cursor icon changes.
  Impact: Embedded and daemon mode clients render different cursor icons from the same terminal session — SSOT violation in the mux wire protocol.
  Required plan update: GAP task added to 10.5 OSC 22 block directing `mouse_cursor_icon: Option<u8>` (wire-encoded) to be added to `PaneSnapshot`, populated in `oriterm_mux/src/server/snapshot.rs`, and decoded on the daemon-client side — same subsection as the Term field addition so the paths stay in sync (FIXED in this round).
  Basis: fresh_verification | direct_file_inspection. Confidence: high.

<!-- Round 25 findings (2026-04-17) — all fixed inline -->

- [x] `[TPR-10-103-codex][high]` `plans/spec-conformance/section-10-osc-suite.md:382` — `OSC-133-CMD-COMPLETE` catalog row specified `effect-host-notification` as the apex, but `EffectHostNotification` in the `ApexLayer` enum (`crates/oriterm_test_support/src/spec_chain/scenario.rs`) is documented as "Apex: desktop notification (OSC 9/99/777)" — semantically wrong for `HostEffect::CommandComplete`.
  Evidence: `ApexLayer` enum at `crates/oriterm_test_support/src/spec_chain/scenario.rs:97`: `EffectHostNotification` documented as "Apex: desktop notification (OSC 9/99/777)". No `EffectHostCommand` variant exists in the enum.
  Impact: Using `EffectHostNotification` for `CommandComplete` would map the OSC 133;D test apex to the desktop-notification path, producing a false-green when the CommandComplete effect fires through the wrong discriminator.
  Required plan update: Catalog update rewrote `effect-host-notification` to `effect-host-command`; Section 10.4 task added to create a new `EffectHostCommand` variant in `ApexLayer` (and register in `from_apex`, `plans/spec-conformance/00-overview.md:820`); success criterion line 16 updated to reference `effect-host-command` as the new variant (FIXED in this round).
  Basis: fresh_verification | direct_file_inspection. Confidence: high.

- [x] `[TPR-10-104-codex][medium]` `plans/spec-conformance/section-10-osc-suite.md:99` — Goal paragraph said "Section 10's tests will exercise OSC 7/9/99/133/633/777 through `SpecHarness::feed()`", conflating the high-level `Processor` path with the mux `RawInterceptor` path. Interceptor-handled OSCs (7/9/99/133/633/777) CANNOT be verified through `SpecHarness::feed()` because the spec harness runs only the high-level Processor; they require sibling unit tests in `oriterm_mux/src/shell_integration/tests.rs`.
  Evidence: Line 99 (before fix): goal stated all OSC tests go through `SpecHarness::feed()` without distinguishing interceptor-handled vs processor-handled OSCs.
  Impact: Developer would write interceptor-handled OSC tests against `SpecHarness::feed()`, where they would silently pass (bytes are consumed without effect) rather than exercising the real production path.
  Required plan update: Line 99 rewritten to explicitly distinguish high-level-processor OSCs (0/1/2/4/8/10/11/12/22/50/52/104/110/111/112/1337 non-image → `oriterm_core/tests/spec_chain/osc/`) from interceptor-handled OSCs (7/9/99/133/633/777 → `oriterm_mux/src/shell_integration/tests.rs` sibling unit tests) (FIXED in this round).
  Basis: fresh_verification | direct_file_inspection. Confidence: high.

- [x] `[TPR-10-105-codex][medium]` `plans/spec-conformance/section-10-osc-suite.md:194` — Renderable observer TDD step described a "pass→fail→invert" pattern that is not true red-first TDD. The described sequence would have the stub PASS the first invocation (counting as GREEN before any implementation), negating the TDD contract.
  Evidence: Line 194 (before fix): described running the test first with the stub (expecting pass), then inverting the assertion to confirm it fails — this is not a red-first workflow.
  Impact: Developer could write a test that passes against the stub without detecting that the stub masks a real implementation gap; the TDD contract would be nominally satisfied while the semantic pin is ineffective.
  Required plan update: Line 194 rewritten to describe proper red-first discipline: write a test with a MISMATCHED URI (e.g., `"wrong://uri"`) that FAILS against the stub (RED), implement the observer to read the actual URI, confirm the test passes (GREEN) (FIXED in this round).
  Basis: fresh_verification | direct_file_inspection. Confidence: high.

<!-- Round 24 findings (2026-04-17) — all fixed inline -->

- [x] `[TPR-10-99-codex][high]` `plans/spec-conformance/section-10-osc-suite.md:533` — OSC 4 query test expected `prefix == "4"` but the dispatcher builds `prefix = format!("4;{index}")` (confirmed at `crates/vte/src/ansi/dispatch/osc.rs:108`), so the actual prefix for palette index 5 is `"4;5"` not `"4"`.
  Evidence: `let prefix = format!("4;{index}");` at `crates/vte/src/ansi/dispatch/osc.rs:108`.
  Impact: A test written with `prefix == "4"` would FAIL at runtime since the actual emitted prefix is `"4;5"`.
  Required plan update: Line 533 corrected to `prefix == "4;5"` with dispatcher line citation (FIXED in this round).
  Basis: fresh_verification | direct_file_inspection. Confidence: high.

- [x] `[TPR-10-100-codex][medium]` `plans/spec-conformance/section-10-osc-suite.md:16` — OSC 133 success criterion wording says "OSC 133;A/B/C/D each drive the PromptState state machine correctly AND update PromptMarker" — the phrase "AND update PromptMarker" falsely implies D updates PromptMarker, contradicting the documented D-behavior.
  Evidence: Success criterion: "OSC 133;A/B/C/D each drive the `PromptState` state machine correctly AND update `PromptMarker`". The interceptor handler (`interceptor.rs:107-114`) clears prompt_state and emits CommandComplete on D — NO PromptMarker update.
  Impact: Implementer would expect D to write a PromptMarker entry — misunderstanding the data model.
  Required plan update: Success criterion rewritten to separate A/B/C (update PromptMarker) from D (clear state, emit CommandComplete); catalog split mentioned (FIXED in this round).
  Basis: fresh_verification | direct_file_inspection. Confidence: high.

- [x] `[TPR-10-101-codex][low]` `plans/spec-conformance/section-10-osc-suite.md:1298` — CWD SSOT checklist grep pattern `grep -rn 'cwd: Option' oriterm_core/src/term/` returns TWO matches (field declaration at `mod.rs:147` AND parameter at `shell_state/mod.rs:245`), contradicting the stated "returns exactly ONE field declaration".
  Evidence: `grep -rn 'cwd: Option' oriterm_core/src/term/` → matches `mod.rs:147` AND `shell_state/mod.rs:245`.
  Impact: Developer running the checklist grep would see 2 results, falsely conclude there are 2 CWD fields, and fail the SSOT check even though only one is a struct field.
  Required plan update: Grep command replaced with field-anchored pattern `grep -rn '^[[:space:]]*cwd: Option<String>,$' oriterm_core/src/term/mod.rs` that matches only the struct field declaration (FIXED in this round).
  Basis: fresh_verification | direct_file_inspection. Confidence: high.

- [x] `[TPR-10-102-codex][medium]` `plans/spec-conformance/section-10-osc-suite.md:384` — Catalog update for 10.4 promotes SHINT-OSC-133-PROMPT and SHINT-OSC-633-VSCODE but omits SHINT-OSC-133-CMD-COMPLETE, which is the new shell-integration cross-reference row for D added in the same edit.
  Evidence: Line 384: "`SHINT-OSC-133-PROMPT → verified; SHINT-OSC-633-VSCODE → verified`" — no mention of SHINT-OSC-133-CMD-COMPLETE.
  Impact: Without an explicit promotion step, the new SHINT-OSC-133-CMD-COMPLETE row would remain at `missing` status after Section 10 is implemented.
  Required plan update: Line 384 updated to include `SHINT-OSC-133-CMD-COMPLETE → verified` (FIXED in this round).
  Basis: fresh_verification | direct_file_inspection. Confidence: high.

<!-- Round 23 findings (2026-04-17) — all fixed inline -->

- [x] `[TPR-10-94-gemini][high]` `plans/spec-conformance/section-10-osc-suite.md:533` — OSC 4 query test assertion references a fabricated `ColorQueryTarget::PaletteIndex(5)` enum that does not exist in the codebase.
  Evidence: Line 533: `Effect::HostRequest(HostRequest::ColorQuery { index: ColorQueryTarget::PaletteIndex(5) })`. `HostRequest::ColorQuery` has fields `prefix: String`, `index: usize`, `terminator: String`, `reply: ResponseToken<Rgb>` (confirmed at `oriterm_core/src/effect/families/host_request.rs:35-40`). There is no `ColorQueryTarget` enum anywhere in the workspace.
  Impact: Developer would attempt to match a non-existent enum variant, causing a compilation error.
  Required plan update: Replaced with `Effect::HostRequest(HostRequest::ColorQuery { prefix, index, .. })` where `prefix == "4"` and `index == 5`, with a comment clarifying the actual struct fields (FIXED in this round).
  Basis: fresh_verification | direct_file_inspection. Confidence: high.

- [x] `[TPR-10-95-gemini][high]` `plans/spec-conformance/section-10-osc-suite.md:553` — OSC 10 query test assertion references a fabricated `ColorQueryTarget::NamedColor(NamedColor::Foreground)` enum that does not exist.
  Evidence: Line 553: `Effect::HostRequest(HostRequest::ColorQuery { index: ColorQueryTarget::NamedColor(NamedColor::Foreground) })`. Same root cause as TPR-10-94: `HostRequest::ColorQuery` uses `index: usize`, not a `ColorQueryTarget` enum. OSC 10 maps to `NamedColor::Foreground as usize = 256` via `let offset = dynamic_code as usize - 10; let index = NamedColor::Foreground as usize + offset` (`crates/vte/src/ansi/dispatch/osc.rs:151-152`).
  Impact: Developer would attempt to match a non-existent enum variant, causing a compilation error.
  Required plan update: Replaced with `Effect::HostRequest(HostRequest::ColorQuery { prefix, index, .. })` where `prefix == "10"` and `index == 256`, with index derivation context from the dispatcher confirmed at `crates/vte/src/ansi/dispatch/osc.rs:151-152` (FIXED in this round).
  Basis: fresh_verification | direct_file_inspection. Confidence: high.

- [x] `[TPR-10-96-codex][high]` `plans/spec-conformance/section-10-osc-suite.md:382` — OSC-133 catalog update directs marking the single `OSC-133` row `verified` without splitting for the D subop effect apex.
  Evidence: Line 382 (before fix): `OSC-133 catalog/osc.md → verified`. The single `OSC-133` catalog row has `state-snapshot` apex, but D emits `HostEffect::CommandComplete` — a host effect, not state. A/B/C are state-snapshot; D is effect-host-notification (nearest canonical apex).
  Impact: Implementing as written would produce an incorrect catalog row that misrepresents the D subop apex, hiding the effect path from the verification chain.
  Required plan update: Line 382 rewritten to require splitting `OSC-133` into `OSC-133-PROMPT` (A/B/C, `state-snapshot`) and `OSC-133-CMD-COMPLETE` (D, `effect-host-notification`); `SHINT-OSC-133-PROMPT` cross-reference updated; new `SHINT-OSC-133-CMD-COMPLETE` row added (FIXED in this round).
  Basis: fresh_verification | direct_file_inspection. Confidence: high.

- [x] `[TPR-10-97-codex][medium]` `plans/spec-conformance/section-10-osc-suite.md:585` — OSC 5 per-row analysis promises "set + query round-trip. verified" but the catalog row is `OSC-5` (state-snapshot) — a query round-trip requires an `effect-pty-write` apex row.
  Evidence: Catalog update list at line 580 listed `OSC-5` as a single row; per-row at line 585 promises a query path that would need `effect-pty-write` apex per the natural-apex invariant.
  Impact: OSC 5 query verification would be filed under the wrong apex, missing the effect path in the test chain.
  Required plan update: Catalog update list corrected to `OSC-5-SET` and `OSC-5-QUERY` separate rows; per-row note updated to explain the split and the correct apex for each (FIXED in this round).
  Basis: fresh_verification | direct_file_inspection. Confidence: high.

- [x] `[TPR-10-98-codex][medium]` `plans/spec-conformance/section-10-osc-suite.md:493` — `ITERM2-1337-SHELLINTVERSION` is added in the 10.7 catalog update but not included in the iterm2.md success criteria row list.
  Evidence: Success criteria (line 10 before fix) listed 6 non-image rows but omitted `ITERM2-1337-SHELLINTVERSION`; the 10.7 body at line 493 says "add if missing" without specifying the row fields; the catalog currently has no such row.
  Impact: The row could be omitted at implementation time since it is not in the enumerated success criteria; the catalog scanner would miss it.
  Required plan update: Success criteria updated to include `ITERM2-1337-SHELLINTVERSION` as a 7th non-image row; catalog update instruction updated with explicit row fields (sequence, description, apex, verification target) (FIXED in this round).
  Basis: fresh_verification | direct_file_inspection. Confidence: high.

<!-- Round 22 findings (2026-04-17) — all fixed inline -->

- [x] `[TPR-10-91-codex][high]` `plans/spec-conformance/section-10-osc-suite.md:295` — `response_poll_duplicate_fulfill_last_wins` test assigned to `host_request.rs`'s sibling `tests.rs`, but `oriterm_core/src/effect/families/host_request.rs` is a flat file and cannot have a sibling `tests.rs` without first converting to a directory module.
  Evidence: Line 295: `Lives in \`oriterm_core/src/effect/families/host_request.rs\`'s sibling \`tests.rs\` (if absent, create)`. `host_request.rs` exists as a flat file with no directory structure.
  Impact: Developer would have no valid location to place the test per the sibling `tests.rs` pattern, leading to either an inline test (banned) or a misplaced test.
  Required plan update: Line 295 rewritten to require converting `host_request.rs` to `host_request/mod.rs` + `host_request/tests.rs` first; line 308 validation updated to reference the correct path `host_request/tests.rs` (FIXED in this round).
  Basis: fresh_verification | direct_file_inspection. Confidence: high.

- [x] `[TPR-10-92-codex][medium]` `plans/spec-conformance/section-10-osc-suite.md:301` — OSC-52-STORE catalog update instructs rewriting `Apex layer` to `effect-host`, which is not a schema-valid value in the `ApexLayer` enum.
  Evidence: Line 301: `rewrite \`Apex layer\` to \`effect-host\``. `plans/spec-conformance/00-overview.md:820` lists valid values: `effect-clipboard`, `effect-host-title`, `effect-host-notification` — `effect-host` is not listed.
  Impact: The catalog row would carry an invalid `Apex layer` value, violating the frozen catalog schema from Section 04.7.
  Required plan update: Line 301 rewritten to preserve `effect-clipboard` (already the correct valid apex layer for OSC 52) and only update `Implementation` + `Notes` columns (FIXED in this round).
  Basis: fresh_verification | direct_file_inspection. Confidence: high.

- [x] `[TPR-10-93-codex][low]` `plans/spec-conformance/section-10-osc-suite.md:308` — Validation clause hardcodes the duplicate-fulfill test in `host_request.rs` sibling `tests.rs` while the body already allows two placements (`response_poll/tests.rs` or `io_thread/tests.rs`) for the first two tests — a DRIFT between the Files block and the validation text.
  Evidence: Line 308: `the duplicate-fulfill pin in \`oriterm_core\`'s \`host_request.rs\` sibling \`tests.rs\`` while line 293 says `OR in \`oriterm_mux/src/pane/io_thread/tests.rs\` if \`response_poll.rs\` stays flat`.
  Impact: Ambiguous canonical home for the test; implementation could put it in an inconsistent location.
  Required plan update: Line 308 rewritten to use `host_request/tests.rs` (after directory module conversion) and explicitly allow the two placements already documented for `response_poll_roundtrip_emits_pty_write` (FIXED in this round).
  Basis: fresh_verification | direct_file_inspection. Confidence: high.

---

## 10.N Completion Checklist

### TDD Discipline (MUST be FIRST — per `.claude/rules/tests.md` §TDD for Bugs)

- [ ] **Failing test matrix written FIRST** — all of 10.0's harness/observer/state TDD tests are written and VERIFIED RED before any implementation lands. Then 10.1–10.9's test matrices are written and VERIFIED RED in subsection order. Skipping this invalidates the TDD contract and the section.
- [ ] **Ordering gate:** 10.0 completes before any of 10.1–10.9 starts. 10.0's TPR checkpoint 1 MUST pass before the downstream subsections are written.

### Crate ordering (per `.claude/rules/crate-boundaries.md` allowed dependency direction)

- [ ] Changes land in this order: `crates/vte` (new Handler trait methods beyond the 7 non-image `iterm2_*` defaults that already landed + OSC 1337 sub-dispatcher is already in place — see scope clarification F) → `oriterm_core` (ALL Term fields this section adds: `mouse_cursor_icon` (10.0/10.5 — already landed), `remote_host` (10.7), `user_vars` (10.7), `shell_integration_version` (10.7), `last_command_line` (10.4 — OSC 633 E sub-command), `tab_title_color` (10.9 — OSC 6), plus whatever Term fields 10.9's OSC 3/5/13/14/17/19/113/114/117/119 analysis determines are needed (e.g. `x11_property`, `mouse_fg_color`, `mouse_bg_color`, `highlight_bg_color`, `highlight_fg_color`); the 7 non-image `iterm2_*` Term overrides land in `handler/mod.rs` under §10.7 — currently only `set_mouse_cursor_icon` + `iterm2_file` exist on `Term`; helpers go in `handler/osc.rs`) → `oriterm_mux` (interceptor extensions for OSC 633 + 1337 delegated rows where applicable; mux-intercepted OSC tests added to `oriterm_mux/src/shell_integration/tests.rs` sibling unit-test module — NOT to `oriterm_mux/tests/` integration tests, which have no `pub(crate)` access to `RawInterceptor`; the `response_poll` pipeline is already live post effect-cutover §01.1 — no `#[allow(dead_code)]` removal or call-site wiring is in Section 10's scope) → `crates/oriterm_test_support` (completed renderable observer + new `RenderableExpectation` fields; registration sync on `recording_handler.rs` for all 7 non-image `iterm2_*` + new 10.9 Handler methods — these are NOT yet in `recording_handler.rs` today; NO new `mux_layer` dependency per crate-boundary constraint — see 10.0 Option A) → high-level-processor OSC tests under `oriterm_core/tests/spec_chain/osc/*` (OSC 0/1/2/4/10/11/12/8/22/50/52/104/110/111/112/1337 non-image sub-ops).

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
  - [ ] `ResponseToken::fulfill` is single-assignment; duplicate-fulfill returns `Err(AlreadyFulfilled)` (`response_poll_duplicate_fulfill_rejected`, 10.2).
  - [ ] OSC 9 via high-level processor does NOT fire notification — mux-only (`osc9_via_processor_without_mux_drops`, 10.3).
  - [ ] OSC 133 via high-level processor does NOT drive PromptState — interceptor-only (`osc133_via_high_level_processor_drops`, 10.4 — parallel to OSC 633 / OSC 7 interceptor-only pins; scope clarification B covers the invariant).
  - [ ] OSC 633 via high-level processor does NOT trigger state change — interceptor-only (`osc633_via_high_level_processor_drops`, 10.4).
  - [ ] OSC 7 via high-level processor does NOT set cwd — interceptor-only (`osc7_via_high_level_processor_drops`, 10.8).
  - [ ] OSC 22 no-parameter case is silently dropped (`osc22_no_parameter_is_dropped`, 10.5).
  - [ ] OSC 22 unknown icon does not mutate state (`osc22_unknown_icon_is_dropped`, 10.5).
  - [ ] OSC 22 does NOT affect text cursor shape (`osc22_does_not_affect_text_cursor_shape`, 10.5).
  - [ ] OSC 50 unknown shape does not mutate state (`osc50_unknown_shape_dropped`, 10.5).
  - [ ] OSC 1337 Copy invalid base64 is dropped (`osc1337_copy_invalid_base64_dropped`, 10.7).
  - [ ] OSC 1337 SetUserVar invalid base64 is dropped (`osc1337_set_user_var_invalid_base64_dropped`, 10.7).
  - [ ] OSC 1337 File= with unknown sub-key is safely absorbed — no panic, no dispatch dropping (`osc1337_unknown_file_subop_safely_ignored`, 10.7 — Section 14 de-risking pin per blind-spot #9).
  - [ ] OSC 5 invalid color spec is dropped — no state mutation (`osc5_invalid_color_dropped`, 10.9).
  - [ ] OSC 13 invalid RGB is dropped — `mouse_fg_color()` unchanged (`osc13_invalid_rgb_dropped`, 10.9).
  - [ ] OSC 14 invalid RGB is dropped — `mouse_bg_color()` unchanged (`osc14_invalid_rgb_dropped`, 10.9).
  - [ ] OSC 17 invalid RGB is dropped — `highlight_bg_color()` unchanged (`osc17_invalid_rgb_dropped`, 10.9).
  - [ ] OSC 19 invalid RGB is dropped — `highlight_fg_color()` unchanged (`osc19_invalid_rgb_dropped`, 10.9).
  - [ ] OSC 3 on non-X11 platform does not panic and the `Term::x11_property` field is absent by `#[cfg]` gate — no `HostEffect::SetX11Property` variant exists or is referenced (`osc3_non_x11_platform_no_panic`, 10.9).
  - [ ] OSC l with empty param sets title to `""` and does not panic (`osc_l_empty_sets_empty_title`, 10.9).
- [ ] **Cross-pattern matrix**: every OSC that has SET and QUERY forms has both tested in the same subsection; every OSC that has SET and RESET forms has both tested.

### Rules weaving (per `.claude/rules/impl-hygiene.md` + `.claude/rules/code-hygiene.md` + `.claude/rules/crate-boundaries.md` + `.claude/rules/oriterm_core.md` + `.claude/rules/oriterm_mux.md`)

- [ ] **No SSOT drift**: `Term::cwd` is the ONLY CWD field — OSC 7, OSC 133, and OSC 1337 CurrentDir all route through it (10.4, 10.7, 10.8). Verified by: (1) `grep -rn '^[[:space:]]*cwd: Option<String>,$' oriterm_core/src/term/mod.rs` returns exactly ONE field declaration at `oriterm_core/src/term/mod.rs:148` (use this FIELD-ANCHORED pattern, NOT `grep -rn 'cwd: Option'` which also matches the `set_cwd` parameter at `oriterm_core/src/term/shell_state/mod.rs:245`); (2) `grep -rn 'fn set_cwd' oriterm_core/src/term/` returns exactly ONE function definition (in `oriterm_core/src/term/shell_state/mod.rs:245`); (3) all `set_cwd` call sites route through `Term::set_cwd` (not direct field access).
- [ ] **No registration sync drift**: new `NotificationSource` variants (none added in this section — pinned in 10.3) AND new `Handler` trait methods added across Section 10 subsections are checked for sync across ALL three consumers — (1) `crates/vte/src/ansi/handler.rs` (trait declaration), (2) `oriterm_core/src/term/handler/mod.rs` (Term impl), AND (3) `crates/oriterm_test_support/src/spec_chain/recording_handler.rs` (RecordingHandler delegate). All three must have matching method entries. Missing from `recording_handler.rs` means spec_chain tests silently miss the new dispatch (per finding TPR-10-15 precedent). **Full method checklist for this section**: `iterm2_set_mark`, `iterm2_remote_host`, `iterm2_current_dir`, `iterm2_copy`, `iterm2_report_cell_size`, `iterm2_set_user_var`, `iterm2_shell_integration_version` (10.0/10.7) + `set_mouse_cursor_icon` (10.0/10.5) + `set_x11_property` (10.9) + `set_mouse_fg_color`, `set_mouse_bg_color`, `set_highlight_bg_color`, `set_highlight_fg_color` (10.9). Run `grep -rn 'fn set_mouse_cursor_icon\|fn set_mouse_fg_color\|fn set_mouse_bg_color\|fn set_highlight_bg_color\|fn set_highlight_fg_color\|fn set_x11_property\|fn iterm2_'` across all three paths and confirm each method name appears exactly once in each consumer file.
- [ ] **No LEAK**: reply formatting for OSC 52 + OSC 4/10/11/12 queries goes through `format_clipboard_reply` / `format_color_reply` at `oriterm_core/src/effect/families/host_request.rs:110,126` — the canonical home; NO ad-hoc `format!` at dispatch or handler sites.
- [ ] **No file size violations (source files)**: per `.claude/rules/code-hygiene.md` §File Size, source files (non-`tests.rs`) stay under 500 lines. Three files Section 10 touches are already at or near the limit and will grow under the plan — each MUST be split if the Section-10 additions push over 500 lines:
  - **`crates/vte/src/ansi/dispatch/osc.rs`** — 258 lines today; the OSC 1337 sub-dispatcher extraction (10.0) and the new OSC 3/5/6/13/14/17/19/113/114/117/119/L/l arms (10.9) grow it. If it crosses 500, split by OSC family under the existing `dispatch/` pattern: `dispatch/osc/mod.rs` + `dispatch/osc/color.rs` (4/10/11/12/104/110/111/112/13/14/17/19/113/114/117/119) + `dispatch/osc/notifications.rs` (9/99/777) + `dispatch/osc/shell_integration.rs` (133/633) + `dispatch/osc/iterm2.rs` (1337 sub-dispatcher + delegations).
  - **`oriterm_core/src/term/mod.rs`** — 488 lines today (post §10.0 partial landing). Section 10 adds a non-trivial set of new fields (`mouse_cursor_icon` — already landed, `last_command_line`, `remote_host`, `user_vars`, `shell_integration_version`, `tab_title_color` plus 10.9 OSC state fields `x11_property`, `mouse_fg_color`, `mouse_bg_color`, `highlight_bg_color`, `highlight_fg_color`). Count projected new lines BEFORE landing; if over 500, extract the OSC-state bundle into a dedicated submodule `oriterm_core/src/term/osc_state/` (mod.rs with a struct + accessors, embedded as a single field on `Term<S>`) per `.claude/rules/code-hygiene.md` §File Size. This preserves SSOT (Term still owns the bundle) while keeping the file under 500 lines.
  - **`oriterm_core/src/term/handler/mod.rs`** — 442 lines today (post §10.0 partial landing). Section 10 adds Term overrides for: `iterm2_set_mark`, `iterm2_remote_host`, `iterm2_current_dir`, `iterm2_copy`, `iterm2_report_cell_size`, `iterm2_set_user_var`, `iterm2_shell_integration_version` (all 7 pending in §10.7 — the vte trait DEFAULTS already exist at `crates/vte/src/ansi/handler.rs:334-356` but the Term OVERRIDES do not exist yet; only `set_mouse_cursor_icon` at `handler/mod.rs:387` and `iterm2_file` at `handler/mod.rs:427` are landed today), `set_x11_property`, `set_mouse_fg_color`, `set_mouse_bg_color`, `set_highlight_bg_color`, `set_highlight_fg_color` (5 pending in §10.9). Count projected new lines BEFORE landing; if over 500, move the OSC overrides into `handler/osc.rs` (which already exists for OSC helpers) and leave `handler/mod.rs` as the `impl Handler for Term<S>` shell that delegates to helper methods defined in `handler/osc.rs`. Preserves SSOT (trait impl still lives with Term) without the file-size violation.
- [ ] **Test file hygiene guidance (soft recommendation — NOT a rule-enforced gate)**: per `.claude/rules/code-hygiene.md` §File Size, `tests.rs` sibling files are exempt from the 500-line limit. This recommendation is therefore NOT a rule-backed completion gate — it is project-local hygiene guidance for Section 10 to keep the biggest test files readable. The four test files below are already at multiples of the source-file limit today. When Section 10 adds new tests, prefer placing them in a newly-created submodule sibling (aggregator pattern) over appending to the existing oversized file. If a split is more disruptive than the added tests warrant (e.g., breaks too many citation links in the catalog), appending is acceptable and does NOT block the subsection — but the author SHOULD leave a "split candidate" note for a future hygiene pass:
  - **`oriterm_core/src/term/tests.rs`** — already well over the source-file limit (run `wc -l` for the current count — it drifts per commit; tests.rs is exempt from the 500-line limit). Section 10.0's `term_set_mouse_cursor_icon_stores_icon` test lands here per the TDD bullet. Preferred: extract existing tests into submodules by subject — `term/tests/cursor.rs`, `term/tests/mode.rs`, `term/tests/palette.rs`, `term/tests/shell_state.rs`, etc. — via a `term/tests/mod.rs` aggregator. Then add 10.0's new test to the appropriate submodule (likely `term/tests/cursor.rs` for `mouse_cursor_icon`). Acceptable alternative: append to the existing file and flag a "split candidate" note; this does NOT block 10.0 completion.
  - **`oriterm_mux/src/pane/io_thread/tests.rs`** — already well over the source-file limit (run `wc -l` for the current count; tests.rs is exempt from the 500-line limit). Section 10.2's OSC-52-specific response-poll tests (`osc52_register_poll_roundtrip`, `osc52_embedded_backend_fulfills_via_session_pty_responder`) land in the existing `oriterm_mux/src/pane/io_thread/response_poll/tests.rs` submodule — the directory-module conversion is already complete (effect-cutover §01.1) and response_poll already has its own sibling test file with nine green tests. DO NOT append new OSC-52 tests to the monolithic `io_thread/tests.rs`. Section 10.2 extends `response_poll/tests.rs`.
  - **`oriterm_mux/src/shell_integration/tests.rs`** — already over the source-file limit (run `wc -l` for the current count; tests.rs is exempt from the 500-line limit). Section 10 heavily extends this with the `spec_chain_helper` + OSC 7/9/99/133/633/777 mux tests across subsections 10.0, 10.3, 10.4, 10.8. Preferred: extract existing tests into `shell_integration/tests/` submodules by OSC family — `tests/osc7.rs`, `tests/osc133.rs`, `tests/osc9_99_777.rs`, `tests/osc633.rs` — via a `shell_integration/tests/mod.rs` aggregator that ALSO re-exports the `spec_chain_helper` test-only module so downstream test files can call it. The `spec_chain_helper` itself lives at `shell_integration/tests/spec_chain_helper.rs` and is `pub(super) use`d from the aggregator. All new Section 10 mux tests go into the appropriate per-OSC submodule, not the monolithic `tests.rs`. Acceptable alternative: append to `tests.rs` and flag a "split candidate" note; this does NOT block the subsection.
  - **`oriterm_core/src/term/handler/tests/osc.rs`** — already over the source-file limit (run `wc -l` for the current count; tests.rs / test-module files are exempt from the 500-line limit). Section 10 cites this as a regression guard but does NOT add new tests here. Preferred: extract into `handler/tests/osc/` submodule tree — `osc/title.rs` (OSC 0/1/2), `osc/color.rs` (OSC 4/10/11/12/104/110/111/112), `osc/hyperlink.rs` (OSC 8), `osc/clipboard.rs` (OSC 52), `osc/cursor.rs` (OSC 22/50), `osc/iterm2.rs` (OSC 1337), via an `osc/mod.rs` aggregator. Same-commit rename; citations in the catalog `Test chain` cells updated to the new paths. Acceptable alternative: leave as-is and flag for a future hygiene pass; this does NOT block Section 10 close.
  - **`crates/oriterm_test_support/src/spec_chain/scenario.rs`** — 457 lines today (91% of limit). Section 10.0 adds 6 new fields to `RenderableExpectation` (`cells`, `hyperlink_at`, `cursor_position`, `cursor_shape`, `palette_index`, `mouse_cursor_icon`, `damaged_lines`) AND a new `ApexLayer::EffectHostCommand` variant. Count projected new lines BEFORE landing; if the additions push over 500, split `scenario.rs` by concern: `scenario/mod.rs` (module aggregator + `SpecScenario` struct) + `scenario/expectations.rs` (`RenderableExpectation`, `StateExpectation`, `EffectExpectation`) + `scenario/apex.rs` (`ApexLayer` enum + `from_apex` mapping). Preserves SSOT (single `SpecScenario` construction site) while keeping each file under 500 lines.
- [ ] **Section plan file size**: `plans/spec-conformance/section-10-osc-suite.md` is well over the 500-line plan-doc soft cap enforced by `.claude/skills/plan-audit/plan-audit.py` (run `wc -l` for the current count — the number drifts each editor pass and is not load-bearing). The Structural Note at the top of this section explicitly defends against splitting because of TPR anchor stability + Scope Clarifications A–J dependencies. NOTE this as an accepted exception for Section 10 ONLY (not a general license); future sections at or above 1000 lines MUST split into `section-NN-a.md` / `section-NN-b.md` sibling files UNLESS they carry a comparable TPR anchor-stability + scope-clarification load. The 10.N exit criteria do NOT require this section file to be split — the exception is made once, at Section 10 close, with the rationale that the 25+ rounds of TPR findings in 10.R would be citation-broken by a split and that no future section should inherit this exception by precedent.
- [ ] **Cross-platform**: OSC 3 (X11 property) has `#[cfg]` branches for Linux-X11 vs macOS vs Windows per `.claude/rules/tests.md` §Cross-Platform Verification. Every branch has a counterpart; Windows cross-compile via `cargo build --target x86_64-pc-windows-gnu` green.
- [ ] **Alloc regression unchanged** — OSC 10/11/12 query reply formatting is not on the hot render path (per `.claude/rules/oriterm_core.md` §Performance Invariants the hot path is `renderable_content_into()` and snapshot flip); but OSC reply formatting still must not leak allocations per frame. `oriterm_core/tests/alloc_regression.rs` green.
- [ ] **RSS regression** — OSC 52 store + OSC 1337 SetUserVar accumulate state (clipboard history, user vars). Bound the growth: `user_vars: IndexMap<String, String>` has a configurable max-size cap (default 256 entries, **eviction FIFO/insertion-order** — oldest inserted entry evicted first, NOT access-order LRU; implemented via `IndexMap::shift_remove_index(0)`); clipboard-store state is owned by the consumer, not by Term. `oriterm_core/tests/rss_regression.rs` green.

### Catalog + cross-section updates

- [ ] Every row promoted per the success criteria list has a `Test chain` citation pointing at the specific `#[test]` function by path + name.
- [ ] `plans/spec-conformance/catalog/iterm2.md` front-matter `owner_section` updated (10.7).
- [ ] `plans/spec-conformance/catalog/osc.md` ownership notes updated where Section 08 and Section 10 split responsibility — clarify Section 10 owns ALL OSC rows (per scope clarification A).
- [ ] `plans/spec-conformance/catalog/osc.md:49` OSC-3 grammar cell rewritten from `OSC 3 ; prop ; value BEL|ST` to `OSC 3 ; Pt BEL|ST`, with a Notes-cell addition explaining `Pt = prop[=value]` (bare `prop` deletes the property; `prop=value` sets it). The current catalog grammar misrepresents the xterm ctlseqs payload as two semicolon-delimited fields — this would implement a non-standard dispatcher that rejects conformant OSC 3 payloads. Required before 10.9 marks OSC-3 `verified-with-deviation`.
- [ ] `plans/spec-conformance/catalog/osc.md:10` header wording updated. The header today reads "One row per OSC numeric sub-op dispatched by `crates/vte/src/ansi/dispatch/osc.rs::dispatch`." After Section 10 lands, this is false: OSC 7 / 9 / 99 / 133 / 633 / 777 are dispatched by `oriterm_mux::shell_integration::RawInterceptor` (the mux-layer canonical path), not by `crates/vte/src/ansi/dispatch/osc.rs::dispatch`. Rewrite the header to state: "One row per OSC numeric sub-op. Rows dispatched by `crates/vte/src/ansi/dispatch/osc.rs::dispatch` cover the high-level processor path (OSC 0/1/2/4/8/10/11/12/22/50/52/104/110/111/112/1337 non-image + 10.9 additions). Rows dispatched by `oriterm_mux::shell_integration::RawInterceptor` cover the interceptor-owned mux path (OSC 7/9/99/133/633/777). Every row's `Implementation` cell names its canonical dispatcher. OSC 1337 File= lives in `iterm2.md`. OSC terminator is BEL (0x07) or ST (`ESC \\`); the dispatcher selects the echoed form via `bell_terminated`." Required so the catalog header matches the actual routing map after Section 10 finalizes SSOT between the two dispatch paths.
- [ ] `plans/spec-conformance/section-14-iterm2-images.md:55` wording is consistent with the catalog update — flow-up review gate. If `reviewed: true` is on section 14, it MUST be flipped to `false` by the `/review-plan verify` step because this section changed a shared catalog row. The `/review-plan verify` step handles this automatically via the reviewed-gate machinery; the plan is NOT to manually edit `section-14.md` here.
- [ ] **Vendored VTE patch record (per scope clarification J)**: `crates/vte/README.md` updated (create if missing) with the Section-10 patch scope: new `Handler` methods (`iterm2_set_mark`, `iterm2_remote_host`, `iterm2_current_dir`, `iterm2_copy`, `iterm2_report_cell_size`, `iterm2_set_user_var`, `iterm2_shell_integration_version`, `set_x11_property`, `set_mouse_fg_color`, `set_mouse_bg_color`, `set_highlight_bg_color`, `set_highlight_fg_color`) + `b"1337"` sub-dispatcher refactor. Note: "oriterm-specific protocol coverage, not upstreamable — needs rebase on next upstream `vte` sync". This MUST land in the same commit as the `handler.rs` trait additions; a patch-without-record is a crate-boundary hygiene violation.
- [ ] **Vendored-patch breadcrumbs in code** (per 10.0 Implementation block) — verification grep: `grep -rn 'VENDORED PATCH (oriterm)' crates/vte/` returns one line for each new oriterm-added Handler method (at least: 7 `iterm2_*` methods from 10.0 + `set_mouse_cursor_icon` note at its oriterm-specific override site if modified + any 10.9 additions `set_x11_property` / `set_mouse_fg_color` / `set_mouse_bg_color` / `set_highlight_bg_color` / `set_highlight_fg_color`) plus one for the `dispatch_iterm2_osc1337` helper. Missing breadcrumbs are a DRIFT finding against scope clarification J and block section close.

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
- [ ] `plans/spec-conformance/00-overview.md` Quick Reference + mission success criterion **"Verification chain complete per row"** incremented (checkboxes for promoted rows); the **"Effect/State separation enforced"** criterion gets a note that §10.2 adds OSC-52-specific consumer-side coverage on top of the already-live `response_poll` pipeline (activation landed under effect-cutover §01.1, not in §10.2).
- [ ] `plans/spec-conformance/00-overview.md` Section Dependency Graph cross-references updated if any new cross-section interaction was discovered (e.g. Section 22 real-app harness benefits from OSC 633 being `verified`).
- [ ] `plans/spec-conformance/index.md` section 10 status updated from "Not Started" to "Complete"; quick-ref lines updated with the final tests.
- [ ] Cross-links added to Section 14: when Section 14 is next picked up for /continue-roadmap, the overview should note that Section 10's 10.7 landed the sub-dispatcher and the non-image OSC 1337 rows.
- [ ] Cross-links added to Section 16 (mouse protocols): Section 16's reviewer inherits the OSC 22 push-vs-poll architectural decision per scope clarification E. The handoff note is: "Section 10 exposed `Term::mouse_cursor_icon` as a polling getter consumed via `RenderableContent::mouse_cursor_icon` and `PaneSnapshot::mouse_cursor_icon` (embedded + daemon paths both read-through). If Section 16 decides UI consumers should receive a push-style `Effect::Ui(UiEffect::MouseCursorChanged)` notification instead of polling, it owns the migration: the polling surface must stay until the push consumers are in place so mid-migration consumers are not stranded."
- [ ] `/tpr-review` final (full-section) passed — dual-source codex + gemini, all findings resolved.
- [ ] `/impl-hygiene-review last commit` passed (after `/tpr-review` is clean).

### Accepted audit findings (documented exceptions)

The `.claude/skills/plan-audit/plan-audit.py` scanner reports the following findings against Section 10. Each is accepted as either a false-positive of a mechanical heuristic or a documented exception with rationale:

- [ ] **DEAD_PATH for files Section 10 creates** — planned-file references like `oriterm_core/tests/spec_chain/osc/hyperlinks.rs`, `oriterm_core/tests/spec_chain/osc/clipboard.rs`, `oriterm_core/tests/spec_chain/osc/iterm2_non_image.rs`, `oriterm_core/tests/spec_chain/osc/cursor.rs`, `oriterm_core/tests/spec_chain/osc/color_reset.rs`, `oriterm_core/tests/spec_chain/osc/basic.rs`, `oriterm_core/tests/spec_chain/osc/palette.rs`, `oriterm_core/tests/spec_chain/osc/default_colors.rs`, `oriterm_core/tests/spec_chain/osc/missing_rows.rs`, `oriterm_core/tests/spec_chain/osc/mod.rs`, `crates/vte/src/ansi/dispatch/tests.rs`, `oriterm_mux/src/pane/io_thread/response_poll/tests.rs`, `oriterm_core/src/effect/families/host_request/mod.rs`, `oriterm_core/src/effect/families/host_request/tests.rs`. These are destinations Section 10 creates during implementation; the audit tool has no way to distinguish "file to be created" from "stale reference to a deleted file." Verifying at section close: every file in this list MUST exist with tests at section completion.
- [ ] **DEAD_PATH for bare filenames in TPR history** — `handler/mod.rs`, `handler/osc.rs`, `interceptor.rs`, `mod.rs`, `shell_state/mod.rs`, `term/mod.rs` appear in the TPR findings history block (10.R) as quoted evidence lines from past review rounds. Rewriting TPR history to add full-path prefixes would violate the "TPR findings are a permanent audit trail" contract. These are accepted false positives; each TPR finding is marked `[x]` FIXED with the live prose corrected in-place.
- [ ] **SIZE_VIOLATION (352 top-level items as of 2026-04-18 Step 5 editor pass — up from 308 at TPR-10-109 acceptance)** <!-- blocked-by:anchor-migration-plan --> — documented exception in the Structural Note at the top of this section. Section 10 is correctly subsectioned into 10.0–10.9 + 10.R + 10.N; the 20-item heuristic is a "subsection or split" prompt and this section has taken the subsection path. Do not re-litigate without a concrete TPR-anchor-preservation plan. **This exception applies to Section 10 only**; future sections do NOT inherit this license. **Unblock condition:** a targeted `/review-plan` pass that produces (a) a concrete split proposal (e.g. extract 10.R into `section-10R-tpr-findings.md` and 10.N into `section-10N-completion.md`, or keep 10.N inline and extract 10.R only, or ratify the exception permanently), AND (b) an anchor-rewrite strategy that preserves the 25+ rounds of TPR citation line-number references already in 10.R (e.g. mechanical line-offset map + checked-in redirect table, or adoption of symbolic anchors instead of line numbers). This task is picked up by `/review-bugs` / `/fix-next-bug` from the TPR-10-109 resolution pointer (see §10.R).
- [ ] **BLOAT_RISK for plan docs** — `plans/spec-conformance/00-overview.md`, `plans/spec-conformance/index.md`, `plans/spec-conformance/section-08-ecma-48-baseline.md`, `plans/spec-conformance/section-10-osc-suite.md` are all above the soft cap (exact counts drift; consult `wc -l` when triaging). These are plan-doc files with cross-section reference weight; splitting is outside Section 10's ownership (each target file is owned by its own `/review-plan` cycle). Flagged for follow-up `/review-plan` sessions that own those files; Section 10 does not modify 00-overview.md, index.md, or section-08 beyond the cross-references called out in 10.N.
- [ ] **BLOAT_RISK for `.claude/rules/impl-hygiene.md`** (529 lines) — this is a coding-standards rule file that Section 10 CITES but does not MODIFY. The rules file owners (not Section 10) would split it. Flagged as NOTE for cross-plan awareness; not actionable from Section 10.
- [ ] **BLOAT_RISK for `.claude/skills/plan-audit/plan-audit.py`** (614 lines) — the plan-audit script itself, cited in the Structural Note. Not Section 10's concern; `.claude/skills/` cleanup is a separate effort.
- [ ] **BLOAT_RISK for source / test files Section 10 touches** — handled above in the "No file size violations (source files)" item (rule-backed hard gate for `.rs` sources) and "Test file hygiene guidance" item (soft recommendation for `tests.rs` siblings, which are exempt from the 500-line limit per `.claude/rules/code-hygiene.md` §File Size). Each source file at/near the limit has an explicit split plan before extending; test files have a preferred-split + acceptable-append alternative.

**Net position after audit triage**: 1 critical (BROKEN_DEP on `effect-cutover`) → FIXED (converted to cross-plan path reference). ~20 DEAD_PATH findings (planned-files + TPR history) → ACCEPTED false positives of the mechanical scanner. ~19 BLOAT_RISK findings → each has a split plan in 10.N's "No file size violations" items OR is explicitly out-of-scope (plan-doc + rule files). 1 SIZE_VIOLATION → documented exception with rationale in the Structural Note.

**Exit Criteria:** Every OSC catalog row in `plans/spec-conformance/catalog/osc.md` is `verified` or `verified-with-deviation`. Every row in `plans/spec-conformance/catalog/shell-integration.md` is `verified`. The non-image rows of `plans/spec-conformance/catalog/iterm2.md` are `verified` and their ownership is cleanly assigned to Section 10. Mux-intercepted OSC verification (OSC 7/9/99/133/633/777) lives in `oriterm_mux/src/shell_integration/tests.rs` (sibling unit-test module) via the mux-internal `spec_chain_helper` — `SpecHarness` stays mux-free and the integration test directory (`oriterm_mux/tests/`) contains NO `RawInterceptor`-using tests. High-level-processor OSC tests (0/1/2/4/8/10/11/12/22/50/52/104/110/111/112/1337 non-image) live in `oriterm_core/tests/spec_chain/osc/`. OSC 52 ResponseToken round-trip runs end-to-end through the live `response_poll` path (activated under effect-cutover §01.1; Section 10.2 is consumer-side coverage). OSC 22 has real Term state, not a no-op stub. OSC 133;D's behavior is documented and pinned against the actual `PromptMarker` data model. The OSC suite is conformance-complete and Section 14 can pick up the OSC 1337 sub-dispatcher without refactoring it.
