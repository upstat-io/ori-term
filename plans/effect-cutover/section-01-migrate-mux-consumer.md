---
section: "01"
title: "Migrate Mux Consumer from LegacyEventSink to QueueingEffectSink"
status: not-started
reviewed: false
goal: "Replace `LegacyEventSink<IoThreadEventProxy>` with `QueueingEffectSink` as the IO thread's `Term<S>` effect sink so the IO thread subscribes to `Effect` directly. Route every `Effect` variant into the existing `MuxEvent` / `MuxNotification` stream and into `pending_responses` (for `HostRequest` variants) in the IO thread's own drain loop, add an idle-wake channel so a fulfilled `ResponseToken` immediately unblocks the `crossbeam_channel::select!` in `PaneIoThread::run`, and delete `IoThreadEventProxy`, `LegacyEventSink`, `Event::ClipboardLoad`, `Event::ColorRequest`, and the `Term::drain_notifications()` shim once all consumers are wired through `drain_into()`. This section unblocks spec-conformance Section 10.2 (which removes the `#[allow(dead_code)]` gate on `PaneIoThread::register_host_request_response` and activates the OSC 52 / OSC 10/11/12 `ResponseToken` round-trip)."
success_criteria:
  - "`oriterm_mux/src/pane/io_thread/mod.rs` declares `PaneIoThread<QueueingEffectSink>` (not `PaneIoThread<LegacyEventSink<IoThreadEventProxy>>`); both `domain/local.rs::LocalDomain::spawn_pane` (current call site at `oriterm_mux/src/domain/local.rs:130-146`) and `domain/handoff/mod.rs::adopt_pane` construct `Term::new(..., QueueingEffectSink::new())` and no longer construct `LegacyEventSink::new(IoThreadEventProxy::new(..))` or `IoThreadEventProxy::new(..)` anywhere. The sink swap and the router activation land in the SAME commit (01.1) — there is NO intermediate state where effects are queued but unrouted (that would silently drop bells/title/CWD/clipboard in production; flagged by both TPR reviewers as a Broken-Window violation)."
  - "After each VTE parse chunk and after each command batch, the IO thread calls `effect_sink.drain_into(&mut effects_buf)` into a reusable scratch `Vec<Effect>` owned by `PaneIoThread` (grows-only, never shrinks inside `draw_frame`-equivalent hot paths per `.claude/rules/impl-hygiene.md` §Data Flow) and routes every `Effect` variant into `MuxEvent` via a single canonical match in one named function — no duplicated dispatch tables."
  - "`Effect::HostRequest(HostRequest::ClipboardLoad { .. })` and `Effect::HostRequest(HostRequest::ColorQuery { .. })` are registered with `pending_responses` via `PaneIoThread::register_host_request_response(request)` — the `#[allow(dead_code, reason = \"dormant during legacy phase; activates at effect-cutover\")]` attribute on `register_host_request_response` at `oriterm_mux/src/pane/io_thread/response_poll.rs:33-36` (verified present at line 35) is REMOVED and `grep -rn '#\\[allow(dead_code, reason = \"dormant during legacy phase'` in `oriterm_mux/` returns zero matches."
  - "A fulfilled `ResponseToken` causes `PaneIoThread::run` to poll `pending_responses` within one `select!` iteration with NO unrelated byte or command activity required. Concretely: a new `response_wake_rx: Receiver<()>` arm is added to BOTH `crossbeam_channel::select!` blocks in `PaneIoThread::run` (the sync-deadline arm at `oriterm_mux/src/pane/io_thread/mod.rs:141-158` and the no-deadline arm at `oriterm_mux/src/pane/io_thread/mod.rs:161-176`); when the main thread calls `PaneIoHandle::fulfill_clipboard_load(token, text)` or `PaneIoHandle::fulfill_color_query(token, rgb)` (new mux-side helper), the fulfill call also signals `response_wake_tx` which unblocks the `select!`. The semantic pin `response_poll_idle_wake_unblocks_select` in `oriterm_mux/src/pane/io_thread/response_poll/tests.rs` verifies this with NO PTY bytes and NO command traffic — fulfillment alone drives the PtyEffect::Write emission."
  - "`Event::ClipboardLoad(..)` and `Event::ColorRequest(..)` are DELETED from `oriterm_core/src/event/mod.rs` (the `Event` enum loses those two variants and the `Arc<dyn Fn>` closure fields); every `match event { Event::ClipboardLoad(..) => ..; Event::ColorRequest(..) => .. }` arm in the workspace is removed; exhaustive-match compile errors in `MuxEventProxy::send_event`, `IoThreadEventProxy::send_event`, the Debug impl on `Event`, and any downstream consumer are fixed — not by `_ => {}` wildcards, but by removing the arm."
  - "`LegacyEventSink`, `IoThreadEventProxy`, and `DesktopNotificationRecord` are DELETED. Files removed: `oriterm_core/src/effect/sink/legacy/mod.rs`, `oriterm_core/src/effect/sink/legacy/tests.rs`, and the entire `oriterm_core/src/effect/sink/legacy/` directory; `oriterm_mux/src/pane/io_thread/event_proxy/mod.rs` and `oriterm_mux/src/pane/io_thread/event_proxy/tests.rs`. The `pub use sink::legacy::DesktopNotificationRecord` re-export at `oriterm_core/src/effect/mod.rs:20` is removed. The `mod legacy;` and `pub use legacy::LegacyEventSink;` in `oriterm_core/src/effect/sink/mod.rs:7,11` are removed. `grep -rn 'LegacyEventSink\\|IoThreadEventProxy\\|DesktopNotificationRecord'` across the workspace returns zero hits (excluding the git history)."
  - "`Term::drain_notifications()` (the shim that drains `LegacyEventSink::pending_notifications` — currently declared at `oriterm_core/src/term/shell_state/mod.rs:356` inside `impl<L: EventListener + Sync> Term<LegacyEventSink<L>> { .. }`; the doc block begins at line 345) is DELETED. Desktop notifications flow exclusively through `Effect::Host(HostEffect::DesktopNotification { .. })` → `drain_into` → IO thread's Effect→MuxEvent router → `MuxNotification`. A new `MuxNotification::DesktopNotification { pane_id, source, title, body }` variant is added in `oriterm_mux/src/mux_event/mod.rs` (with parallel `MuxEvent::DesktopNotification { .. }` if the existing event-pump double-indirection is kept; if single-indirection is adopted, update `in_process/event_pump.rs` accordingly — BOTH paths must land atomically to avoid registration-sync drift per `.claude/rules/impl-hygiene.md` §Registration Sync Points)."
  - "`HostEffect::ClearPendingNotifications` is now observed by `QueueingEffectSink` consumers — the IO thread's Effect→MuxEvent router sees the marker and emits `MuxNotification::ClearPendingDesktopNotifications(pane_id)` (new variant) so the main thread clears any notifications it is currently holding for that pane. Semantic pin: `clear_pending_notifications_discards_preceding` in `response_poll/tests.rs` verifies that a `DesktopNotification` followed by `ClearPendingNotifications` in the same drain batch results in NO `MuxNotification::DesktopNotification` being emitted for that pane — the router collapses them per the contract documented at `oriterm_core/src/effect/families/host.rs:42-50`."
  - "Spec-conformance Section 10.2's `response_poll_emits_pty_write_on_fulfill` test (to be written in spec-conformance Section 10) passes without modification once this section's sink migration + router + idle-wake land — this section provides the prerequisite; Section 10.2 writes the test. Cross-reference: `plans/spec-conformance/section-10-osc-suite.md:218-221` (Option A) and `plans/spec-conformance/section-10-osc-suite.md:14` (success criterion)."
  - "Daemon-mode IPC compatibility audit complete (01.4). `ResponseToken<T>` is process-local (`Arc<Mutex<Option<T>>>` at `oriterm_core/src/effect/families/host_request.rs:54-56`) and CANNOT cross an IPC boundary; `oriterm_mux/src/protocol/messages.rs:396-401` (`NotifyClipboardLoad`) serializes only `pane_id` + `clipboard_type`, no reply token. The audit produces one of: (a) an in-scope subsection that adds a request-ID + reply-PDU design, (b) a filed `/add-bug` artifact (severity major) that tracks the daemon-mode gap, OR (c) an explicit `<!-- blocked-by: bug-tracker/BUG-XX-NNN -->` cross-link if the daemon path is deferred to a separate plan. Option (b) or (c) is only acceptable if daemon mode is NOT part of the effect-cutover mission (current plan overview does not mention daemon) AND the bug tracker entry names the blocker. Default: option (b) — file a bug in 01.4."
  - "`MuxEventProxy` audit is resolved (01.3 / 01.4). `MuxEventProxy` at `oriterm_mux/src/mux_event/mod.rs:137-257` implements `EventListener` for `Term<MuxEventProxy>`. After 01.1 the IO thread no longer uses it. If it has NO consumer elsewhere, it is DELETED in 01.3 (alongside `IoThreadEventProxy`). If daemon or another path still uses it, the plan files a bug and leaves a deprecation marker. Verification: `grep -rn 'MuxEventProxy' oriterm_mux/ oriterm/` enumerated in 01.3 Cleanup; ACTION documented inline."
  - "`Event::ChildExit` routing gap resolved before 01.3's deletion. Current state: `Event::ChildExit(code)` is emitted by the pty reader thread and matched in TWO places — `oriterm_mux/src/pane/io_thread/event_proxy/mod.rs:143-148` AND `oriterm_mux/src/mux_event/mod.rs:245`. After `IoThreadEventProxy` deletion, the effect-based path `Effect::Host(HostEffect::ChildExit { code })` must route to `MuxEvent::PaneExited { pane_id, exit_code }` via 01.1's router. If `Event::ChildExit` is ALSO emitted by any VTE handler (grep verified in 01.1 implementation), that emission is migrated to the effect path in the SAME commit — no dangling `Event::ChildExit` producer. If the reader thread still emits `Event::ChildExit` through `MuxEventProxy` on a non-IO-thread path, that path stays OR is also migrated — resolved in 01.1's Cleanup block; answer documented inline."
  - "Multi-chunk parse bursts do not accumulate unbounded effects. 01.1 calls `drain_effects_into_mux_events()` at the end of EACH chunk inside `handle_bytes_chunked` (`oriterm_mux/src/pane/io_thread/mod.rs:220-234`), not only at the end of the whole forwarded read. Verified by the ordering pin `drain_preserves_push_order_end_to_end` (intra-chunk) + a new pin `multi_chunk_parse_drains_between_chunks` that asserts `effects_buf.len()` never exceeds `EFFECTS_BUF_SOFT_CAP = 4096` across a 1 MB PTY read (bounded by the sum of MAX_PARSE_CHUNK-sized drains)."
  - "Router coverage is not hidden behind `VoidEffectSink`. The helpers in `oriterm_mux/src/pane/io_thread/tests.rs` that currently construct `PaneIoThread<VoidEffectSink>` (`make_sync_thread` at line 13 and `make_sync_thread_generic` at line 54 — `VoidEffectSink::drain_into` is a no-op so they cannot exercise the router) are either (a) migrated to `PaneIoThread<QueueingEffectSink>`, or (b) kept for generic unit tests AND a parallel `make_sync_thread_queueing()` helper is introduced so every router path has at least one concrete-sink test. `QueueingEffectSink` exercise coverage is no longer concentrated at `tests.rs:1927` (the `sync_timeout_emits_abort_effect` test); 01.1's router matrix uses it exclusively."
  - "`oriterm_core/tests/alloc_regression.rs` stays green — the Effect→MuxEvent router uses a reusable `Vec<Effect>` scratch buffer on `PaneIoThread` (never `Vec::new()` per drain) and the router moves strings out of `HostEffect::TitleSet { value }` / `HostEffect::CwdSet { cwd }` / `HostEffect::DesktopNotification { title, body }` variants rather than cloning."
  - "`oriterm_core/tests/rss_regression.rs` stays green — `pending_responses: Vec<PendingResponse>` is capped (see 01.3) and the `response_wake` channel is bounded-size-1 (`crossbeam_channel::bounded(1)`) so idle-wake signalling never accumulates more than one pending wake."
  - "Cross-platform build green: `./build-all.sh` runs `cargo build --target x86_64-pc-windows-gnu --release` cleanly, covering the adopt-pane path (`domain/handoff/mod.rs`) which is exercised by Windows Default Terminal handoff; Linux/macOS local build also green. `./test-all.sh` green; `./clippy-all.sh` green (zero warnings under `deny(clippy::all)` + nursery)."
  - "`/tpr-review` passes on the final state (dual-source codex + gemini, all findings resolved); `/impl-hygiene-review last commit` passes after TPR is clean."
  - "Every subsection (01.1 – 01.4, 01.N) transitions `status: not-started` → `status: complete`. `00-overview.md` and `index.md` are updated in the same commit that lands the final subsection to reflect plan-complete state (mission success). Spec-conformance Section 10's `depends_on` already lists `\"effect-cutover\"` (confirmed at `plans/spec-conformance/section-10-osc-suite.md:36`); this section's completion is what unblocks that dependency — no cross-plan edit is required here."
depends_on:
  - "plans/spec-conformance/section-03-effect-boundary-migration.md"
third_party_review:
  status: none
  updated: null
sections:
  - id: "01.1"
    title: "Sink swap + Effect→MuxEvent router (atomic — no intermediate commit where effects are queued but not routed)"
    status: not-started
  - id: "01.2"
    title: "Activate PendingResponse polling with idle-wake channel"
    status: not-started
  - id: "01.3"
    title: "Delete IoThreadEventProxy, LegacyEventSink, Event::ClipboardLoad/ColorRequest, MuxEventProxy (if unused), drain_notifications shim"
    status: not-started
  - id: "01.4"
    title: "Daemon-mode IPC for HostRequest — design + bug filing + follow-up section or separate plan"
    status: not-started
  - id: "01.N"
    title: "Completion Checklist"
    status: not-started
---

# Section 01: Migrate Mux Consumer

## Mission-Criterion Connection

This section's mission criteria trace upward to `00-overview.md §Goal`:

- "Migrate `oriterm_mux` IO thread from `LegacyEventSink` → `QueueingEffectSink`" → delivered atomically by 01.1 (sink swap + router land in a single commit; no intermediate state silently drops effects).
- "Delete `LegacyEventSink`, `Event::ClipboardLoad`, `Event::ColorRequest`" → delivered by 01.3.
- "Remove the `drain_notifications()` thin shim" → delivered by 01.3.
- "All consumers process effects via `drain_into()` — no separate notification drain" → delivered by 01.1 + 01.3.
- "Add an idle-wake channel so a fulfilled `ResponseToken` unblocks the IO thread's `crossbeam_channel::select!`" → delivered by 01.2.

Downward: this section also unblocks `plans/spec-conformance/section-10-osc-suite.md` success criterion 7 (line 14) — Section 10.2's removal of the `#[allow(dead_code)]` gate on `register_host_request_response` is no-op once this section lands because this section wires the live call site AND removes the gate. Section 10.2 will only need to write the round-trip `#[test]` against the already-activated path.

## Blind Spots (reviewer-surfaced — MUST be resolved in this section, not deferred)

Both `codex` and `gemini` reviewers raised these during `/review-plan` Phase 4. Each resolves inside a named subsection below — none is deferred.

1. **Atomicity of sink swap + router** — 01.1 as a standalone landable commit silently drops all effects (bells, title, CWD, clipboard requests). `oriterm_mux/src/domain/local.rs:130` and `domain/handoff/mod.rs:106` construct panes whose `QueueingEffectSink` has no drain caller yet. **Resolution:** merged into a single atomic 01.1 subsection covering BOTH sink swap AND router activation AND wake channel wiring of the drain call sites. There is no intermediate commit where effects queue without being drained.
2. **Daemon-mode IPC incompatibility** — `ResponseToken<T>` is `Arc<Mutex<Option<T>>>` (`oriterm_core/src/effect/families/host_request.rs:54-56`) — process-local, cannot cross IPC. `NotifyClipboardLoad` in `oriterm_mux/src/protocol/messages.rs:396-401` serializes only `pane_id` + `clipboard_type`, not a token. **Resolution:** 01.4 is dedicated to the daemon-mode audit: either adds a request-ID + reply-PDU design in-scope, or files a `/add-bug` artifact with a concrete repro-and-fix plan, or cross-links to a new plan. No silent deferral.
3. **`Event::ChildExit` dual-path ambiguity** — Emitted and matched in `event_proxy/mod.rs:143-148` AND `mux_event/mod.rs:245`. After 01.3's `IoThreadEventProxy` deletion, the effect-based path must absorb the child-exit signal. **Resolution:** 01.1 Cleanup block mandates a grep + documentation step naming every `Event::ChildExit` producer and consumer. If the reader thread still emits `Event::ChildExit` via `MuxEventProxy`, the plan documents whether that path survives or is migrated — before 01.3 touches the deletion set.
4. **Router under-exercise via `VoidEffectSink`** — The dominant test helpers at `oriterm_mux/src/pane/io_thread/tests.rs:13` (`make_sync_thread`) and `:54` (`make_sync_thread_generic`) use `VoidEffectSink`; `QueueingEffectSink` coverage is concentrated at `tests.rs:1927`. **Resolution:** 01.1 introduces `make_sync_thread_queueing()` AND migrates existing test helpers where the production code path they cover now runs through the router. No router variant relies solely on a `VoidEffectSink` helper.
5. **Unbounded effect accumulation across parse chunks** — `handle_bytes_chunked` at `oriterm_mux/src/pane/io_thread/mod.rs:220-234` invokes `handle_bytes` per `MAX_PARSE_CHUNK` (64 KB) slice. If the drain only fires at the end of `handle_bytes_chunked` (or only at `run()` loop top), a 1 MB forwarded read accumulates effects in the sink. **Resolution:** 01.1 places the drain call at the end of `handle_bytes`, NOT at the end of `handle_bytes_chunked` — every 64 KB slice drains its effects before the next slice enters. Pinned by `multi_chunk_parse_drains_between_chunks`.
6. **`MuxEventProxy` lifecycle** — `mux_event/mod.rs:137-257` implements `EventListener` for a now-dead IO-thread path; `mux_event/tests.rs:11,15,25,…` still references it. **Resolution:** 01.3 runs the audit; if unused in production, the struct + impl + tests are deleted; if still used, a deprecation bug is filed.
7. **Sync→async ordering change** — `DesktopNotification`, `ClearPendingNotifications`, title, CWD, bell effects today fire synchronously at VTE-byte time via `LegacyEventSink::push`. Post-migration they fire at "next drain" (end of `handle_bytes`). **Resolution:** because the drain runs at the same `handle_bytes` boundary (lines 260-283) that already exists between the byte processor and `post_parse_housekeeping`, observable ordering is preserved to the main thread (snapshot production happens AFTER `post_parse_housekeeping`, which runs AFTER the drain). Pinned by `drain_preserves_push_order_end_to_end`. If any existing timing-sensitive integration test relies on metadata-before-snapshot ordering, it must still pass — validated by running the full suite at every subsection boundary.
8. **`HostEffect::{VisualBell, AudioRequest, PrintRequest}` are functional regressions from day 1 if only logged** — real `\a` sequences fire these. **Resolution:** 01.1's Cleanup block files `/add-bug` artifacts for each (severity medium) that enumerate the concrete user-facing regression (audible bell not forwarded, print request dropped, visual-bell not driving UI flash). The bug artifact IS the resolution; leaving them as log-only is acceptable ONLY because the bug artifact tracks the user-visible gap for a separate fix.
9. **Cross-crate consumer exhaustive-match coupling** — adding `MuxEvent::DesktopNotification`, `MuxEvent::HostClipboardLoad`, `MuxEvent::HostColorQuery` and `MuxNotification::{DesktopNotification, ClearPendingDesktopNotifications}` forces exhaustive-match updates in `oriterm/src/app/**`. **Resolution:** 01.1 Files block enumerates each exhaustive-match site (via `grep -rn 'match.*MuxNotification\|match.*MuxEvent' oriterm/src/`) — every site is covered by an embedded checklist item, not a vague "update callers". Concrete enumeration happens in implementation; placeholder here.
10. **`oriterm/src/app/window_management/mod.rs:214` and `oriterm/src/app/mux_pump/mod.rs:44` call `mux.drain_notifications(&mut self.notification_buf)`** — this is the mux's `InProcessMux::drain_notifications` at `oriterm_mux/src/in_process/event_pump.rs:102`, which is DIFFERENT from the `Term::drain_notifications()` shim at `oriterm_core/src/term/shell_state/mod.rs:356` that 01.3 deletes. **Resolution:** 01.3 explicitly preserves `InProcessMux::drain_notifications` (it still drains `MuxNotification`s from the mux) and deletes ONLY the `Term` shim. Documented by the `no_drain_notifications_shim` pin which narrows its grep to `oriterm_core/src/term/`.

## Architectural Context (read before implementing)

Current state (verified against code at the time of this review):

- IO thread's `Term<S>` type parameter is `S: EffectSink + 'static`. Today `S = LegacyEventSink<IoThreadEventProxy>`. Construction sites: `oriterm_mux/src/domain/local.rs:139-145` and `oriterm_mux/src/domain/handoff/mod.rs:116-122` both call `Term::new(.., LegacyEventSink::new(IoThreadEventProxy::new(..)))`.
- `LegacyEventSink::push(effect)` maps `Effect` → legacy `Event` and forwards synchronously to its wrapped `EventListener` (`IoThreadEventProxy`), whose `send_event` then fans out to `MuxEvent` (metadata) or sets `grid_dirty` (Wakeup). Net flow today: VTE handler → `term.effect_sink().push(Effect)` → `LegacyEventSink::push` → `IoThreadEventProxy::send_event(Event)` → `mux_tx.send(MuxEvent)` + wakeup.
- `LegacyEventSink::drain_into(&mut out)` is deliberately a **no-op** (`oriterm_core/src/effect/sink/legacy/mod.rs:188-191`); there is no queue to drain, because effects are forwarded synchronously at push time.
- `PaneIoThread` already owns `pending_responses: Vec<PendingResponse>` (declared at `oriterm_mux/src/pane/io_thread/mod.rs:98`) and a `poll_pending_responses()` method (declared at `oriterm_mux/src/pane/io_thread/response_poll.rs:84`). `poll_pending_responses()` is already called inside `drain_commands()` at `oriterm_mux/src/pane/io_thread/mod.rs:211`. The only thing preventing the path from firing in production is that `register_host_request_response()` (the function that PUSHES into `pending_responses`) is gated `#[allow(dead_code, reason = "dormant during legacy phase; activates at effect-cutover")]` at `response_poll.rs:33-36` and has NO live call site.
- `ResponseToken::fulfill(value)` (at `oriterm_core/src/effect/families/host_request.rs:69`) only stores the value into the mutex-protected slot. It does NOT signal the IO thread. With the current legacy adapter this is fine — the legacy closure runs synchronously in the consumer callback and pushes `Event::PtyWrite` back out. After migration, fulfillment happens on the main thread and the IO thread must be woken so `drain_commands() → poll_pending_responses()` runs. Without a wake mechanism, a fulfilled token sits idle until unrelated PTY bytes or a command arrive to unblock `crossbeam_channel::select!`. TPR-03-001-codex flagged this during spec-conformance Section 03 close-out; this section fixes it.

Target state after this section:

- IO thread's `Term<S>` type parameter is `S = QueueingEffectSink`. `Term::effect_sink().push(effect)` queues the effect in the sink's internal `parking_lot::Mutex<Vec<Effect>>`.
- The IO thread runs `effect_sink.drain_into(&mut self.effects_buf)` after each VTE parse chunk (end of `handle_bytes`) and after each command batch (end of `drain_commands`, AFTER `poll_pending_responses()` so a fulfilled reply in this tick enters the same drain). The router function walks `effects_buf`, matches each variant, and:
  - `Effect::Pty(PtyEffect::Write { bytes, .. })` → `MuxEvent::PtyWrite { pane_id, data }` (where `data` is `String::from_utf8_lossy(&bytes).into_owned()` at the mux boundary — parallel to current `LegacyEventSink::push`'s `Event::PtyWrite` construction at `legacy/mod.rs:101-103`).
  - `Effect::Host(HostEffect::Bell)` → `MuxEvent::PaneBell(pane_id)`.
  - `Effect::Host(HostEffect::TitleSet { value })` → `MuxEvent::PaneTitleChanged { pane_id, title: value.unwrap_or_default() }`.
  - `Effect::Host(HostEffect::IconNameSet { value })` → `MuxEvent::PaneIconChanged { pane_id, icon_name: value.unwrap_or_default() }`.
  - `Effect::Host(HostEffect::CwdSet { cwd })` → `MuxEvent::PaneCwdChanged { pane_id, cwd }`.
  - `Effect::Host(HostEffect::CommandComplete { duration })` → `MuxEvent::CommandComplete { pane_id, duration }`.
  - `Effect::Host(HostEffect::ChildExit { code })` → `MuxEvent::PaneExited { pane_id, exit_code: code }`.
  - `Effect::Host(HostEffect::ClipboardStore { selection, data })` → `MuxEvent::ClipboardStore { pane_id, clipboard_type: <map selection>, text: data }`. The `ClipboardSelection` → `ClipboardType` mapping is identical to `selection_to_legacy` at `oriterm_core/src/effect/sink/legacy/mod.rs:195-201`; move that helper to the router (`effect_router/mod.rs` private fn, or promote to `oriterm_mux/src/mux_event/clipboard.rs` once a second consumer emerges) — DO NOT duplicate it across legacy and new paths. After 01.3's deletion of the legacy file, the helper lives in exactly one place.
  - `Effect::Host(HostEffect::DesktopNotification { source, title, body })` → `MuxEvent::DesktopNotification { pane_id, source, title, body }` (NEW variant — added in 01.1). Event pump forwards → `MuxNotification::DesktopNotification { pane_id, source, title, body }` (NEW variant).
  - `Effect::Host(HostEffect::ClearPendingNotifications)` → `MuxNotification::ClearPendingDesktopNotifications(pane_id)` (NEW variant). Router collapses preceding `DesktopNotification` effects for the same pane in the SAME drain batch (per the contract at `host.rs:42-50`).
  - `Effect::Host(HostEffect::VisualBell | HostEffect::AudioRequest(_) | HostEffect::PrintRequest(_))` → currently no `MuxEvent` variant. Log at `info!` and count in a `dropped_effects_debug` atomic (dev-build only) — do NOT silently drop. Filing a new `MuxEvent` variant for these is out of scope here; a `/add-bug` artifact in 01.1's Cleanup records the gap as a tracked medium-severity bug per blind-spot §8. **This is a tracked gap, not deferral** — the bug artifact IS the tracking mechanism.
  - `Effect::HostRequest(req)` → 01.1's router emits `MuxEvent::HostClipboardLoad { pane_id, selection, clipboard_char, terminator, reply }` / `MuxEvent::HostColorQuery { pane_id, prefix, index, terminator, reply }` (new variants added in 01.1) carrying the `ResponseToken<T>` the main-thread consumer needs to fulfill. 01.2 additionally wires `self.register_host_request_response(req.clone())` which enqueues a `PendingResponse` that `poll_pending_responses()` drains once the token is fulfilled. The old closure-based `MuxEvent::ClipboardLoad { .. formatter: Arc<dyn Fn(&str) -> String> .. }` is DELETED in 01.3 (the formatter was redundant with `format_clipboard_reply` at `oriterm_core/src/effect/families/host_request.rs:110` — the canonical home).
  - `Effect::Ui(UiEffect::CursorBlinkChanged { .. })` → no `MuxEvent`; fire wakeup only (parallel to legacy behavior at `event_proxy/mod.rs:150-152`).
  - `Effect::Ui(UiEffect::MouseCursorDirty)` → no `MuxEvent`; fire wakeup only.
  - `Effect::Presentation(p)` → log at `info!` level and do NOT queue (parallel to `LegacyEventSink` behavior at `legacy/mod.rs:112-117`; the atomic counter on `LegacyEventSink` is dropped — tests for it will be rewritten or removed in 01.3).
- Fulfilling a `ResponseToken` now ALSO signals an idle-wake channel so `PaneIoThread::run`'s `select!` unblocks. New type `ResponseWakeSignal(crossbeam_channel::Sender<()>)` attached to the token via a new field on `ResponseToken<T>` OR a wrapper type `WakingResponseToken<T>`. The choice between "mutate `ResponseToken`" and "wrap" is resolved in 01.2 by a small TDD — the wrapping approach is preferred because `ResponseToken<T>` lives in `oriterm_core` which is standalone (has no `crossbeam_channel` dependency today; adding one is out of the core crate's charter per `.claude/rules/oriterm_core.md` §Forbidden — "No IPC transport — lives in `oriterm_ipc`"; `crossbeam_channel` is IPC-adjacent enough to warrant keeping it at the mux boundary). Per blind-spot §2, the choice also has implications for daemon-mode — a wrapper keeps `ResponseToken<T>` shippable as process-local data and confines the non-serializable wake sender to `oriterm_mux`.

## Shared Invariants (apply across ALL subsections)

- **Crate boundary discipline** (`.claude/rules/crate-boundaries.md`): `oriterm_core` stays free of mux/IPC types. The idle-wake channel and its `Sender`/`Receiver` live in `oriterm_mux` only. The `Effect` enum stays in `oriterm_core`. Any new wrapper around `ResponseToken` that carries a channel lives in `oriterm_mux` (proposed home: `oriterm_mux/src/pane/io_thread/response_poll/mod.rs`).
- **No duplicated dispatch** (`.claude/rules/impl-hygiene.md` §LEAK:duplicated-dispatch): the Effect→MuxEvent match MUST live in exactly one function. Canonical home: `oriterm_mux/src/pane/io_thread/effect_router/mod.rs` (new directory module with a sibling `tests.rs`). Do NOT inline a second match in `handle_bytes`, `handle_sync_timeout`, or `drain_commands`.
- **No SSOT drift** (`.claude/rules/impl-hygiene.md` §SSOT): reply formatting continues to go through `format_clipboard_reply` / `format_color_reply` at `oriterm_core/src/effect/families/host_request.rs:110,126`. `register_host_request_response` already uses them; the new router does NOT format replies — it only registers the response token. A broken SSOT would look like the router computing `let reply_bytes = format!("\x1b]52;..")` inline. Grep `grep -rn 'format!("\\\\x1b\\]52\\|format!("\\\\x1b\\]4'` across `oriterm_mux/` must return zero hits after this section lands.
- **Hot-path buffer discipline** (`.claude/rules/oriterm_core.md` §Performance Invariants): the `effects_buf: Vec<Effect>` scratch vector on `PaneIoThread` is reused via `clear()` + capacity retention after each drain. No `Vec::new()` per drain. No `shrink_to_fit()` during the hot path; a `maybe_shrink()` call at `PaneIoThread::run`'s bottom of the idle arm is acceptable if measurement warrants it (out of scope here).
- **File size** (`.claude/rules/code-hygiene.md` §File Size): the new `effect_router.rs` MUST stay under 500 lines. `response_poll.rs` is currently ~100 lines; adding idle-wake plumbing keeps it well under. `mod.rs` in `pane/io_thread/` is currently at 436 lines — adding the `effects_buf: Vec<Effect>` field + `effect_router` module declaration + two lines in the drain path should keep it under 500; if it crosses, extract `run()`'s `select!` body into a submodule (the two `select!` arms with their sync-deadline logic are prime split candidates).
- **TDD discipline** (`.claude/rules/tests.md` §TDD for Bugs): every subsection writes its failing test matrix FIRST, verifies RED, then implements to GREEN. No subsection is "complete" without RED→GREEN evidence in its validation checklist.
- **Cross-platform** (`.claude/rules/tests.md` §Cross-Platform Verification): both `domain/local.rs` (POSIX) and `domain/handoff/mod.rs` (Windows conhost handoff) are touched. `cargo build --target x86_64-pc-windows-gnu --release` must succeed locally before each subsection is marked complete. The `adopt_pane` path has no platform-specific `#[cfg]` in the sink wiring itself, but the broader handoff is Windows-only; the Linux build must still compile the cross-platform stubs.

---

## 01.1 Atomic sink swap + Effect→MuxEvent router (no intermediate silent-drop state)

**Goal:** Migrate `PaneIoThread<S = LegacyEventSink<IoThreadEventProxy>>` to `PaneIoThread<S = QueueingEffectSink>` AND activate the Effect→MuxEvent/MuxNotification router in the SAME commit. These are **atomic** — there is no intermediate landable state where effects are queued but unrouted, because such a state silently drops bells, title, CWD, and clipboard traffic in production (Broken-Window violation per blind-spot §1).

Post-01.1 state: metadata events that previously reached the main thread synchronously via `LegacyEventSink::push` → `IoThreadEventProxy::send_event` → `mux_tx.send(..)` now reach the main thread via `drain_into` → `effect_router` → `mux_tx.send(..)`. `HostRequest` variants are still NOT registered into `pending_responses` — 01.2 adds that (because it requires the idle-wake channel to be useful). However `MuxEvent::HostClipboardLoad` / `MuxEvent::HostColorQuery` ARE emitted so the main thread is aware of the request — it just cannot yet drive a reply through `pending_responses` until 01.2.

**IMPORTANT — exhaustive match requirement in 01.1:** Adding `MuxNotification::HostClipboardLoad` and `MuxNotification::HostColorQuery` as new enum variants forces an exhaustive-match update in `oriterm/src/app/mux_pump/mod.rs:54` (`handle_mux_notification`). Since 01.2 is where the actual fulfillment handler is wired, 01.1 MUST add stub match arms: `MuxNotification::HostClipboardLoad { .. } => { log::debug!("HostClipboardLoad received; fulfillment not yet wired (pre-01.2)"); }` and the `HostColorQuery` equivalent. These stubs compile and allow 01.1 to land cleanly; the fulfillment logic (calling `self.mux.fulfill_host_request(..)`) replaces the stubs in 01.2. The stubs are intentionally visible (log::debug, not silent) so a user running a debug build between 01.1 and 01.2 sees the warning in logs rather than a silent drop.

**Files (state additions):**
- `oriterm_mux/src/pane/io_thread/mod.rs` — add fields to `PaneIoThread<S: EffectSink>`:
  - `effects_buf: Vec<Effect>` (grow-only scratch; alloc-regression invariant)
  - `pane_id: PaneId` (used by the router to tag outbound `MuxEvent`s)
  - `mux_tx: mpsc::Sender<MuxEvent>` (router's output channel)
  - `#[cfg(debug_assertions)] dropped_effects_debug: std::sync::atomic::AtomicU64` (counts log-only effects in dev builds; see blind-spot §8)
- `oriterm_mux/src/pane/io_thread/handle.rs:90-116` (`IoThreadConfig`) — add `pane_id: PaneId` and `mux_tx: mpsc::Sender<MuxEvent>` fields; update the struct literal in `new_with_handle` (lines 128-160) accordingly.
- `oriterm_mux/src/pane/io_thread/handle.rs` → **MUST be converted to a directory module** before a `tests.rs` sibling can be added. Per `.claude/rules/test-organization.md` §Sibling tests.rs Pattern: a flat file module (`handle.rs`) cannot have a sibling `tests.rs` file — it must first become a directory module (`handle/mod.rs`). Convert: (1) `mkdir oriterm_mux/src/pane/io_thread/handle/`, (2) rename `handle.rs` → `handle/mod.rs`, (3) create `handle/tests.rs` as the sibling test file, (4) update `mod.rs`'s `mod handle;` declaration — Rust resolves the same `mod handle;` to either `handle.rs` or `handle/mod.rs` so no import changes needed in consumers. The conversion is PART of this subsection's work, not a deferred step.

**Files (construction-site edits):**
- `oriterm_mux/src/domain/local.rs:130-146` (current: `let io_event_proxy = IoThreadEventProxy::new(...);` at line 132, `LegacyEventSink::new(io_event_proxy)` at line 144) — DELETE both calls. Replace with `Term::new(.., QueueingEffectSink::new())`. Thread `pane_id` and `mux_tx.clone()` into the subsequent `IoThreadConfig` literal so the router holds them. Remove `use oriterm_core::effect::LegacyEventSink;` (line 8) and `use crate::pane::io_thread::event_proxy::IoThreadEventProxy;` (line 17).
- `oriterm_mux/src/domain/handoff/mod.rs:106-122` (parallel construction) — same changes. `mux_tx` is already captured as `mux_tx: &mpsc::Sender<MuxEvent>` parameter — pass `mux_tx.clone()`.
- `oriterm_mux/src/domain/handoff/tests.rs` — thread through the new `pane_id` / `mux_tx` in any test that constructs through the real path.

**Files (new — effect router module):**
- `oriterm_mux/src/pane/io_thread/effect_router/mod.rs` (new directory module) — the single canonical dispatch home. Declared at the top of `pane/io_thread/mod.rs` alongside `commands`, `event_proxy`, `handle`, `handler`, `response_poll`, `snapshot` (line 11-16). Under `.claude/rules/test-organization.md` §Sibling tests.rs Pattern, the module is a directory with `mod.rs` + `tests.rs` so tests go in `effect_router/tests.rs`.
- `oriterm_mux/src/pane/io_thread/effect_router/tests.rs` (new sibling).

**Files (MuxEvent + MuxNotification variant additions — single commit, compiler-enforced exhaustive sync per `.claude/rules/impl-hygiene.md` §Registration Sync Points):**
- `oriterm_mux/src/mux_event/mod.rs` — add new `MuxEvent` variants:
  - `MuxEvent::DesktopNotification { pane_id: PaneId, source: NotificationSource, title: String, body: String }`
  - `MuxEvent::HostClipboardLoad { pane_id: PaneId, selection: ClipboardSelection, clipboard_char: u8, terminator: String, reply: ResponseToken<String> }`
  - `MuxEvent::HostColorQuery { pane_id: PaneId, prefix: String, index: usize, terminator: String, reply: ResponseToken<Rgb> }`
  - Update `impl fmt::Debug for MuxEvent` (starts near line 94) exhaustively — compiler enforces.
- `oriterm_mux/src/mux_event/mod.rs` — add new `MuxNotification` variants:
  - `MuxNotification::DesktopNotification { pane_id, source, title, body }`
  - `MuxNotification::ClearPendingDesktopNotifications(PaneId)`
  - `MuxNotification::HostClipboardLoad { pane_id: PaneId, selection: ClipboardSelection, clipboard_char: u8, terminator: String, reply: ResponseToken<String> }` — carries the token so `oriterm`'s app layer can fulfill it. **Registration sync point**: this variant MUST be added in the SAME commit as `MuxEvent::HostClipboardLoad` and the `event_pump.rs` forwarding arm per `.claude/rules/impl-hygiene.md` §Registration Sync Points.
  - `MuxNotification::HostColorQuery { pane_id: PaneId, prefix: String, index: usize, terminator: String, reply: ResponseToken<Rgb> }` — parallel to `HostClipboardLoad` for color queries. Same same-commit constraint applies.
  - Update `impl fmt::Debug for MuxNotification` (line 327) exhaustively — compiler enforces all four new variants.
- `oriterm_mux/src/in_process/event_pump.rs:24-94` (`poll_events`) — add match arms for the new `MuxEvent` variants. Title/icon/cwd/output paths are unchanged; new arms forward `DesktopNotification` → `MuxNotification::DesktopNotification`, `HostClipboardLoad` → `MuxNotification::HostClipboardLoad`, `HostColorQuery` → `MuxNotification::HostColorQuery`. The legacy closure-based `ClipboardLoad { formatter: Arc<dyn Fn> }` variant stays during 01.1 AND 01.2 and is deleted in 01.3.
- `oriterm_mux/src/pane/io_thread/mod.rs` — add `self.drain_effects_into_mux_events()` call at the end of `handle_bytes` (after `post_parse_housekeeping`, line 281-282), AND at the end of `drain_commands` (after `poll_pending_responses`, line 211), AND at the end of `handle_sync_timeout` (after `post_parse_housekeeping`, line 301). **Per blind-spot §5**, the call is inside `handle_bytes` (which is per-chunk inside `handle_bytes_chunked`), NOT only at the top of `handle_bytes_chunked` — this bounds queue growth to the effects produced by one 64 KB `MAX_PARSE_CHUNK` parse slice.

**Files (legacy suppression attributes — delete in 01.3):**
- `oriterm_core/src/effect/sink/legacy/mod.rs` — add `#[allow(dead_code, reason = "removed in effect-cutover 01.3")]` to the `LegacyEventSink` struct declaration (current line 45). After 01.1 construction edits, no code references it.
- `oriterm_mux/src/pane/io_thread/event_proxy/mod.rs` — add `#[allow(dead_code, reason = "removed in effect-cutover 01.3")]` to the `IoThreadEventProxy` struct declaration (current line 26). After 01.1 construction edits, no code references it.

**Files (exhaustive-match update sites — compiler-enforced; enumerate during implementation):**
- `oriterm/src/app/**` — `grep -rn 'match.*MuxNotification' oriterm/src/ | grep -v '//'` during implementation to enumerate exhaustive sites. Every match on `MuxNotification` gains arms for `DesktopNotification` and `ClearPendingDesktopNotifications` in this commit. Per blind-spot §9, this is NOT incidental work — each site is an embedded checklist item at implementation time. The two known call sites today (`oriterm/src/app/window_management/mod.rs:214` and `oriterm/src/app/mux_pump/mod.rs:44`) use `mux.drain_notifications()` which returns `MuxNotification`s that are then consumed elsewhere; follow the data flow into the actual exhaustive matches.
- `oriterm_mux/src/mux_event/tests.rs` — file has 525 lines (approaching 500-line proactive-split threshold). Several tests reference `MuxEventProxy` (lines 11, 15, 25, 214, 384, 437, 476, 507) and `Event::ChildExit` (lines 160, 233). Tests assume the old `Event` → `MuxEvent` mapping via `MuxEventProxy::send_event`. In 01.1 these tests are still needed (legacy path still exists). If the file crosses 500 with added Debug-pin tests for the new variants, split it into `mux_event/tests/proxy.rs` + `mux_event/tests/debug.rs` submodules per `.claude/rules/test-organization.md` §Test File Layout. BLOAT finding tracked inline.

**Tests (written FIRST — per `.claude/rules/tests.md` §TDD for Bugs — VERIFIED RED before implementation):**

Positive path:

- [ ] `pane_io_thread_accepts_queueing_effect_sink` (new in `oriterm_mux/src/pane/io_thread/tests.rs`) — new helper `make_sync_thread_queueing()` constructs `PaneIoThread<QueueingEffectSink>`; assert it compiles and `t.run()` returns on `Shutdown` without panicking. RED because the field layout doesn't yet accommodate the new fields.
- [ ] `io_thread_config_carries_pane_id_and_mux_tx` (new in `handle/tests.rs`) — `IoThreadConfig { ..., pane_id: PaneId(42), mux_tx: tx, .. }` constructs; `PaneIoThread::pane_id() == PaneId(42)` (test-only accessor); `PaneIoThread::mux_tx_for_test()` returns the sender.
- [ ] `effects_buf_is_reused_across_drains` (new) — push 10 `Effect::Pty(..)` effects, call `drain_into(&mut t.effects_buf)` twice with `clear()` in between; assert `capacity() >= 10` after the second clear. Pins alloc-regression grow-only invariant.
- [ ] **Semantic pin** — `queueing_sink_holds_effects_until_drained` — push a `Effect::Host(HostEffect::Bell)`, do NOT drain, assert the sink's queue length is 1 via a new `#[cfg(test)] pub fn queue_len_for_test(&self) -> usize` on `QueueingEffectSink` at `oriterm_core/src/effect/sink/mod.rs`. Do NOT expose via `Debug` (the derive at `sink/mod.rs:61` stays; `parking_lot::Mutex<T>: Debug` does not call `.lock()`).
- [ ] **Idempotency pin** — `multiple_drains_return_all_pushed_effects_in_order` — push 5 distinct effects, drain, assert push order matches. Push 3 more, drain again, assert new-only order is preserved. No interleaving, no drops.

Negative pins:

- [ ] **Negative pin** — `legacy_event_sink_construction_removed_from_local_domain` — runtime `include_str!("../../domain/local.rs")` asserts `!source.contains("LegacyEventSink::new")` and `!source.contains("IoThreadEventProxy::new")`. GREEN when 01.1 lands.
- [ ] **Negative pin** — `legacy_event_sink_construction_removed_from_handoff` — parallel shape for `oriterm_mux/src/domain/handoff/mod.rs`.

Router matrix (covers blind-spots §4 and §5):

- [ ] **Matrix: Effect variant × routing target** — in `effect_router/tests.rs`, one test per `HostEffect` / `PtyEffect` / `UiEffect` / `PresentationEffect` / `HostRequest` variant. Each test pushes a single Effect into a `QueueingEffectSink`, runs `PaneIoThread::drain_effects_into_mux_events()`, and asserts the expected `MuxEvent` appears on a test-side `mpsc::Receiver<MuxEvent>`. Count assertion at the bottom iterates `HostEffect::ALL_VARIANTS_FOR_TEST` (add `#[cfg(test)]` const slice of variant constructors — one entry per variant). The matrix pins that NO variant is silently dropped when a new one is added.
  - Required: `HostEffect::{Bell, VisualBell, DesktopNotification, TitleSet(Some), TitleSet(None), IconNameSet(Some), IconNameSet(None), CwdSet, AudioRequest, PrintRequest, ClipboardStore, ChildExit, CommandComplete, ClearPendingNotifications}`.
  - Required: `PtyEffect::{Write(Other), Write(DeviceStatus), Write(MouseReport)}` (verify variants against `oriterm_core/src/effect/families/pty.rs`).
  - Required: `UiEffect::{CursorBlinkChanged(true), CursorBlinkChanged(false), MouseCursorDirty}`.
  - Required: `PresentationEffect::{Begin, Commit, Abort(Timeout), Abort(BufferLimit)}` (verify shape against `oriterm_core/src/effect/families/presentation.rs`).
  - Required: `HostRequest::{ClipboardLoad, ColorQuery}` — in 01.1 the test asserts the request is emitted to `MuxEvent::HostClipboardLoad` / `MuxEvent::HostColorQuery`; 01.2 layers the `register_host_request_response` + wake assertion on top.
- [ ] **Blind-spot §4 pin** — `router_matrix_uses_queueing_sink_exclusively` — static assertion that every `#[test] fn` in `effect_router/tests.rs` constructs a `PaneIoThread<QueueingEffectSink>` (not `VoidEffectSink`). Implemented as a `#[cfg(test)]` grep-style `include_str!` test reading `effect_router/tests.rs` and asserting `VoidEffectSink` does NOT appear.
- [ ] **Blind-spot §5 pin** — `multi_chunk_parse_drains_between_chunks` — seed a byte stream whose VTE processing emits ≥5 effects per `MAX_PARSE_CHUNK` (64 KB). Feed 1 MB to `handle_bytes_chunked`. Instrument `drain_effects_into_mux_events()` with a `#[cfg(test)]` counter; assert the drain is called at least `ceil(1 MB / 64 KB) = 16` times during the single `handle_bytes_chunked` call (once per chunk). Assert `effects_buf.len()` never exceeded `EFFECTS_BUF_SOFT_CAP = 4096` (named constant per `.claude/rules/impl-hygiene.md` §Magic Numbers).

Semantic + regression pins:

- [ ] **Negative pin** — `visual_bell_is_logged_not_dropped_silently` — push `HostEffect::VisualBell`, call `drain_effects_into_mux_events()`, assert `log::Level::Info` fired via `testing_logger` (workspace dev-dep) AND `dropped_effects_debug` (the new `AtomicU64` field, `#[cfg(debug_assertions)]`) incremented by 1.
- [ ] **SSOT pin** — `title_set_none_produces_empty_title` — push `HostEffect::TitleSet { value: None }`, assert `MuxEvent::PaneTitleChanged { title: String::new() }`. Pattern matches via `value.unwrap_or_default()` inlined once at the TitleSet call site; the pin here is against DUPLICATION, not absence.
- [ ] **Ordering pin** — `drain_preserves_push_order_end_to_end` — push `[Bell, TitleSet("A"), CwdSet("/x"), Bell]`. Drain. Assert receiver sees `[PaneBell, PaneTitleChanged, PaneCwdChanged, PaneBell]` in exactly that order.
- [ ] **Collapse pin** — `clear_pending_notifications_collapses_preceding` — push `[DesktopNotification(Osc9, "A"), DesktopNotification(Osc99, "B"), ClearPendingNotifications, DesktopNotification(Osc777, "C")]`. Drain. Receiver sees `[ClearPendingDesktopNotifications, DesktopNotification(Osc777, "C")]`. Pins the contract documented at `oriterm_core/src/effect/families/host.rs:42-50`.
- [ ] **Cross-batch pin** — `clear_pending_notifications_does_not_retro_collapse_across_drains` — push `[DesktopNotification(Osc9, "A")]`, drain. Push `[ClearPendingNotifications]`, drain. Assert the main-thread-side `self.notifications` buffer still CONTAINS `DesktopNotification(Osc9, "A")` — intra-batch collapse only; cross-batch retro-clear happens on the main thread.
- [ ] **Alloc regression** — `effect_router_drain_zero_alloc_steady_state` — warm-up (two 10-effect drains to seed capacity), then within `oriterm_core::tests::alloc_counter::measure!`: push 10 effects, drain, assert 0 allocations. Uses the existing infrastructure at `oriterm_core/tests/alloc_regression.rs`.

**Implementation:**

- [ ] Add the new fields (`effects_buf`, `pane_id`, `mux_tx`, `dropped_effects_debug`) to `PaneIoThread` with appropriate defaults.
- [ ] Wire `pane_id: PaneId` and `mux_tx: mpsc::Sender<MuxEvent>` through `IoThreadConfig` and `handle.rs::new_with_handle`.
- [ ] `oriterm_mux/src/domain/local.rs:130-146`: DELETE the `IoThreadEventProxy::new(..)` + `LegacyEventSink::new(..)` construction. Replace with `Term::new(.., QueueingEffectSink::new())`. Move `pane_id` + `mux_tx.clone()` into the `IoThreadConfig` literal.
- [ ] `oriterm_mux/src/domain/handoff/mod.rs:106-122`: parallel change.
- [ ] Update `oriterm_mux/src/pane/io_thread/tests.rs`:
  - `make_sync_thread_with_term` (line 13) — populate new fields.
  - `make_sync_thread_generic` (line 54) — populate new fields.
  - Add `make_sync_thread_queueing()` returning `PaneIoThread<QueueingEffectSink>`.
  - For every existing helper or test in this file that currently uses `VoidEffectSink` and exercises a code path now routed through the effect router, migrate to the queueing helper OR duplicate the test so BOTH helpers cover the same path (answers blind-spot §4).
- [ ] Update `oriterm_mux/src/domain/handoff/tests.rs` for the new fields.
- [ ] Create `oriterm_mux/src/pane/io_thread/effect_router/mod.rs`:
  - `impl PaneIoThread<QueueingEffectSink> { pub(crate) fn drain_effects_into_mux_events(&mut self) { .. } }` — **monomorphized on QueueingEffectSink only**. Generic unit tests that use `VoidEffectSink` cannot exercise the router; that is why blind-spot §4's migration exists.
  - Router body: `self.terminal.effect_sink().drain_into(&mut self.effects_buf);` → walk `effects_buf` → match each variant → call `route_host_effect` / `route_pty_effect` / `route_ui_effect` / `route_presentation_effect` / `route_host_request` helpers. `effects_buf.clear()` at the end.
  - `route_host_request`: emits `MuxEvent::HostClipboardLoad` / `MuxEvent::HostColorQuery` with the cloned `HostRequest` variant (so the main thread holds the `ResponseToken`). Does NOT call `register_host_request_response` yet — 01.2 adds that.
  - Private helper `fn selection_to_mux_clipboard_type(s: ClipboardSelection) -> ClipboardType`. Do NOT preemptively create `mux_event/clipboard.rs` — single-caller helpers live next to their caller per `.claude/rules/impl-hygiene.md` §No Premature Abstraction.
- [ ] Add `self.drain_effects_into_mux_events()` at the end of `handle_bytes` (inside the per-chunk loop called by `handle_bytes_chunked`), the end of `drain_commands` (after `poll_pending_responses()`), and the end of `handle_sync_timeout` (after `post_parse_housekeeping`).
- [ ] Add the new `MuxEvent` + `MuxNotification` variants and update their `Debug` impls. Let the compiler drive the exhaustive-match updates in `event_pump.rs` and any `oriterm/src/app/` site — each compiler error is a concrete checklist item.
- [ ] Add `#[allow(dead_code, reason = "removed in effect-cutover 01.3")]` on `IoThreadEventProxy` and `LegacyEventSink`. Do NOT yet remove them, the `Term::drain_notifications()` shim, or `Event::ClipboardLoad/ColorRequest` — those deletions land in 01.3.

**Cleanup (woven hygiene items per `.claude/rules/impl-hygiene.md`):**

- [ ] **[BLOAT]** `oriterm_mux/src/pane/io_thread/mod.rs` — currently 435 lines (`wc -l` verified). Adding `effects_buf`, `pane_id`, `mux_tx`, `dropped_effects_debug`, the three `drain_effects_into_mux_events()` call sites, and the `mod effect_router;` declaration WILL likely push past 500. Extract the two `crossbeam_channel::select!` bodies of `run()` (lines 138-178) into a private `run_loop.rs` submodule BEFORE adding new code — proactive split at 450 per `.claude/rules/code-hygiene.md` §File Size, not a reactive split after the breach. If `run()` itself stays under the limit, split `drain_commands` and `handle_sync_timeout` helpers instead.
- [ ] **[BLOAT]** `oriterm_mux/src/mux_event/tests.rs` is 525 lines — already over the 500-line proactive-split threshold. BEFORE adding Debug pins for the 5 new variants, split this file into `mux_event/tests/proxy.rs` (MuxEventProxy send_event tests) and `mux_event/tests/debug.rs` (Debug impl pins). The split is required; skipping it is a BLOAT finding per `.claude/rules/code-hygiene.md`.
- [ ] **[BLOAT]** `oriterm_mux/src/pane/io_thread/tests.rs` is 2339 lines — massively over limit. Tests added in 01.1 MUST go into new sibling files (`effect_router/tests.rs`, `handle/tests.rs`), not into the existing 2339-line monster. Separately, file a `/add-bug` (severity major) to split `tests.rs` into per-feature tests modules; the split itself is OUT OF SCOPE for this plan but the bug-tracker artifact is mandatory per CLAUDE.md §Bug Discipline.
- [ ] **[NOTE]** File `/add-bug` artifacts for each of `HostEffect::{VisualBell, AudioRequest, PrintRequest}` — each describes a concrete user-visible regression (audible bell not forwarded, print request dropped, visual-bell not driving UI flash). Severity medium; repro recipe in each artifact. The bug-tracker entries ARE the resolution; per blind-spot §8 this is NOT deferral.
- [ ] **[LEAK:scattered-knowledge]** `ClipboardType` (legacy) vs `ClipboardSelection` (effect) live in two types. The translation helper `selection_to_legacy` at `oriterm_core/src/effect/sink/legacy/mod.rs:195-201` (cited in plan body; confirm exact line during implementation) is the SSOT today. After 01.3 deletes the legacy file, the canonical home is the private helper in `effect_router/mod.rs`. If a second consumer emerges, promote to `oriterm_mux/src/mux_event/clipboard.rs`. One helper, one home, zero duplication.
- [ ] **[DRIFT audit — Event::ChildExit]** — per blind-spot §3: `grep -rn 'Event::ChildExit' oriterm_core/ oriterm_mux/ oriterm/` during implementation. Enumerate every emitter and consumer. Document the flow in the subsection's notes before marking complete. The effect-based `HostEffect::ChildExit { code }` → `MuxEvent::PaneExited { pane_id, exit_code }` path is in the router matrix above. If `Event::ChildExit` is STILL emitted by the reader thread on a `MuxEventProxy` path, that path survives 01.1 (deletion is 01.3 after audit). If it is emitted only inside `IoThreadEventProxy::send_event`, no migration is needed — the effect path already covers it. Answer MUST be written into the plan body before 01.3 begins.
- [ ] **[DRIFT audit — MuxEventProxy]** — `grep -rn 'MuxEventProxy' oriterm_mux/ oriterm/ oriterm_core/` enumerated in the subsection's notes. Known hits today (verified): `oriterm_mux/src/mux_event/tests.rs:11,15,25,214,384,437,476,507` (tests), `oriterm_mux/src/mux_event/mod.rs:4-5,134,137,150,175,245` (declaration + impl + ChildExit arm), `oriterm_mux/src/pane/io_thread/mod.rs:50` (doc comment). After 01.1 the type is not used by the IO thread. If `mux_event/tests.rs` tests cover a path that no longer exists in production (the `MuxEventProxy::send_event(Event::*)` mapping), those tests are rewritten or removed in 01.3.

**Validation:**

- [ ] All 14 TDD tests transition RED → GREEN (5 positive, 2 negative, 5 matrix/coverage, 2 semantic/regression — this is the atomic subsection, count adjusted).
- [ ] Every existing `IoThreadEventProxy`-driven mux event (title, icon, cwd, command-complete, child-exit, bell, clipboard-store, pty-write) now fires via the router — pre-existing `oriterm_mux` suite (`timeout 150 cargo test -p oriterm_mux`) is GREEN at commit boundary (NOT just RED→GREEN across commits; the atomicity means no intermediate failing state).
- [ ] `timeout 150 cargo test -p oriterm_core --test alloc_regression` green.
- [ ] `timeout 150 cargo test -p oriterm_core --test rss_regression` green.
- [ ] `./build-all.sh` (including Windows cross-compile) green.
- [ ] `./test-all.sh` green.
- [ ] `./clippy-all.sh` green (new `#[allow(dead_code, reason = ...)]` attributes include `reason` per `.claude/rules/code-hygiene.md` §Style).
- [ ] Section 01.1 `status` → `complete` in frontmatter. 01.1 is the atomic swap + router commit. No intermediate landable state.

---

## 01.2 Activate PendingResponse polling with idle-wake channel

**Goal:** Wire `register_host_request_response` into the router (remove the `#[allow(dead_code)]` gate), add the idle-wake channel so a fulfilled `ResponseToken` drives the PTY reply even when the IO thread is blocked in `select!` with no unrelated activity, and add a mux-side fulfillment helper that signals the wake.

**Files:**
- `oriterm_mux/src/pane/io_thread/response_poll.rs` — convert to directory module at `response_poll/mod.rs` with a sibling `response_poll/tests.rs` (per `.claude/rules/test-organization.md` §Sibling tests.rs Pattern). Files after conversion:
  - `oriterm_mux/src/pane/io_thread/response_poll/mod.rs` — contains `register_host_request_response`, `poll_pending_responses`, and a new `WakeableResponseToken<T>` wrapper type.
  - `oriterm_mux/src/pane/io_thread/response_poll/tests.rs` — sibling test module for `mod.rs`.
- `oriterm_mux/src/pane/io_thread/response_poll/mod.rs:33-36` — REMOVE `#[allow(dead_code, reason = "dormant during legacy phase; activates at effect-cutover")]`. Method becomes live because `effect_router.rs::route_host_request` calls it.
- `oriterm_mux/src/pane/io_thread/mod.rs` — add `response_wake_rx: Receiver<()>` field. Add it to BOTH `crossbeam_channel::select!` blocks at lines 141-158 (sync-deadline arm) and 161-176 (no-deadline arm). On wake, the loop continues to the next iteration which drains commands → calls `poll_pending_responses()` → calls `drain_effects_into_mux_events()`.
- `oriterm_mux/src/pane/io_thread/handle.rs:90-116` (`IoThreadConfig`) — add `response_wake_rx: Receiver<()>` AND expose the paired `response_wake_tx: Sender<()>` so the main thread can signal fulfillment. Since `PaneIoHandle` already holds channels for the main thread to drive the IO thread, add `response_wake_tx: Sender<()>` to `PaneIoHandle` alongside `cmd_tx` and `byte_tx`.
- `oriterm_mux/src/pane/io_thread/handle.rs:40-87` (`impl PaneIoHandle`) — add `pub fn fulfill_clipboard_load(&self, token: &ResponseToken<String>, text: String)` and `pub fn fulfill_color_query(&self, token: &ResponseToken<Rgb>, color: Rgb)` helpers that call `token.fulfill(value)` followed by `self.response_wake_tx.try_send(()).ok()`. Use `try_send` on a bounded-size-1 channel so a pending wake is never stacked — the IO thread only needs "wake at least once; multiple fulfills in the same idle period collapse to one wake."
- `oriterm_mux/src/pane/io_thread/effect_router/mod.rs` — `route_host_request` calls `self.register_host_request_response(req.clone())` AND ALSO emits `MuxEvent::HostClipboardLoad` / `MuxEvent::HostColorQuery` with the token so the main thread can fulfill. The effect is NOT consumed — the `Effect::HostRequest(req)` is cloned because both sides (IO thread's pending-responses AND main thread's fulfillment queue) need access to the same `ResponseToken` (internal Arc-shared slot — `ResponseToken<T>` is `Clone` per `host_request.rs:53`). (Note: the module is a directory module `effect_router/mod.rs`, not a flat file `effect_router.rs` — it was declared as a directory module in 01.1's Files section.)
- `oriterm_mux/src/backend/mod.rs` — add `fn fulfill_host_request(&mut self, pane_id: PaneId, reply: HostReply)` to the `MuxBackend` trait (where `HostReply` is a new `enum { ClipboardLoad { token: ResponseToken<String>, text: String }, ColorQuery { token: ResponseToken<Rgb>, color: Rgb } }` in `oriterm_mux/src/backend/mod.rs`). **This is the required boundary** — `oriterm/src/app/` accesses the mux ONLY through the `MuxBackend` trait; it must NOT reach into `PaneIoHandle` directly (which is not exposed through any existing `MuxBackend` API today, verified by `grep -rn 'PaneIoHandle' oriterm/src/` returning zero hits). The embedded backend (`EmbeddedMux`) looks up the pane's `PaneIoHandle` from its registry and calls the appropriate `fulfill_*` method. The daemon backend (`MuxClient`) implements a stub (`Err(io::Error::other("not supported"))` or sends a reply PDU if Path A of 01.4 is chosen).
- `oriterm_mux/src/backend/embedded/mod.rs` (verified: `oriterm_mux/src/backend/embedded/` is a directory module, not a flat file — confirmed by `ls oriterm_mux/src/backend/`) — implement `MuxBackend::fulfill_host_request` by looking up `pane_id` → `PaneIoHandle` in the registry and calling `handle.fulfill_clipboard_load(token, text)` or `handle.fulfill_color_query(token, color)`.
- `oriterm/src/app/**` — main-thread consumer. Find the site where `MuxNotification::ClipboardLoad` is currently handled (today: receives the closure-based formatter and invokes it). After 01.3 that variant is deleted; 01.2 adds handling for `MuxNotification::HostClipboardLoad { pane_id, selection, clipboard_char, terminator, reply }`. The handler reads the clipboard, then calls `self.mux.fulfill_host_request(pane_id, HostReply::ClipboardLoad { token: reply, text })`. Similarly add handling for `MuxNotification::HostColorQuery`. `grep -rn 'MuxNotification::ClipboardLoad' oriterm/` during implementation to locate the site.
- `oriterm_core/src/effect/response.rs` — NO changes. `PendingResponse` stays exactly as-is. The wake mechanism is pure `oriterm_mux`-side plumbing — `ResponseToken<T>` continues to be a plain data type per the design at `oriterm_core/src/effect/families/host_request.rs:11-17`.

**Tests (written FIRST — VERIFIED RED before implementation):**

- [ ] **Semantic pin — THE critical test for this section** — `response_poll_idle_wake_unblocks_select` in `response_poll/tests.rs`. Setup using the REAL production IO-thread API surface (verified against `oriterm_mux/src/pane/io_thread/handle.rs` and existing tests at `io_thread/tests.rs:1411`): (1) Create a `crossbeam_channel::unbounded()` `(mux_tx, mux_rx)` pair to capture `MuxEvent` output. (2) Construct `PaneIoThread<QueueingEffectSink>` via the same scaffolding as `io_thread/tests.rs` — `handle.rs::new_with_handle(config)` with a `QueueingEffectSink`. (3) Obtain `byte_tx = handle.byte_sender()` (real API: `handle.rs:50`). (4) Send the OSC 52 read sequence as raw PTY bytes: `byte_tx.send(b"\x1b]52;c;?\x07".to_vec()).unwrap()`. The VTE parser decodes this as a clipboard-read request and pushes `Effect::HostRequest(HostRequest::ClipboardLoad { .. })` into the `QueueingEffectSink`. (5) Wait for the IO thread to drain the bytes by calling `mux_rx.recv_timeout(Duration::from_secs(5))` until a `MuxEvent::PaneOutput` is observed (the IO thread emits `PaneOutput` after each byte batch — this is the real synchronization signal, NOT `MuxEvent::Output` which does not exist). (6) After observing `MuxEvent::PaneOutput`, the token is registered and pending. (7) Call `handle.fulfill_clipboard_load(&token, "hello".to_string())`. (8) Assert deterministically: `mux_rx.recv_timeout(Duration::from_secs(5))` returns `Ok(MuxEvent::PtyWrite { data })` where `data` contains the base64-encoded `"hello"`. **Companion test** `response_poll_wake_is_load_bearing` — same setup but do NOT call `fulfill_clipboard_load`; after observing `MuxEvent::PaneOutput` (synchronization signal that bytes were processed), call `mux_rx.try_recv()` and assert `Err(TryRecvError::Empty)` — proving no `MuxEvent::PtyWrite` is emitted without fulfillment (deterministic negative via `try_recv`, no sleep).
- [ ] **Register-side pin** — `host_request_effect_registers_pending_response` — push `Effect::HostRequest(HostRequest::ColorQuery { index: 10, .. })` into the sink, call `drain_effects_into_mux_events()`, assert `t.pending_responses.len() == 1` after the drain. Before the gate removal, the router doesn't call `register_host_request_response`, so this is RED. After wiring, GREEN.
- [ ] **No-fulfillment pin** — `response_poll_token_requires_fulfillment` — push `Effect::HostRequest(HostRequest::ClipboardLoad { .. })`, run two full `drain_commands → poll_pending_responses → drain_effects_into_mux_events` cycles WITHOUT calling `fulfill_clipboard_load`. Assert `t.pending_responses.len() == 1` throughout (token stays pending) and NO `MuxEvent::PtyWrite` is emitted.
- [ ] **Round-trip pin** — `response_poll_emits_pty_write_on_fulfill` — push `Effect::HostRequest(HostRequest::ClipboardLoad { clipboard_char: b'c', terminator: "\x1b\\".into(), .. reply: token })` → drain → `token.fulfill("hello")` → trigger one more `drain_commands` (without wake, for this deterministic test) → assert `MuxEvent::PtyWrite { data }` where `data == format!("\x1b]52;c;{}\x1b\\", base64::engine::general_purpose::STANDARD.encode("hello"))`. The formatted bytes come from the canonical `format_clipboard_reply` — the test doesn't reimplement formatting; it asserts against the canonical helper's output.
- [ ] **Color-query pin** — `color_query_roundtrip_emits_pty_write` — parallel to above for `HostRequest::ColorQuery` with `prefix: "10"`, `index: 0`, fulfilled with `Rgb { r: 255, g: 0, b: 0 }`. Assert `MuxEvent::PtyWrite { data: "\x1b]10;rgb:ffff/0000/0000\x1b\\" }`.
- [ ] **Wake-collapse pin** — `multiple_fulfills_collapse_to_one_wake` — spawn, register two tokens, fulfill both from the main thread with `try_send` on a size-1 bounded channel. Assert the IO thread wakes at most twice (preferably once, collapsed if fulfillments interleave with the wake). Wake events are observable via an atomic counter `wake_seen: AtomicU64` bumped in the `select!` wake arm (debug-only; production can be a no-op or a log-trace).
- [ ] **Cross-platform pin** — `fulfill_compiles_on_windows_cross` — a `trybuild`-style assertion that the `PaneIoHandle::fulfill_clipboard_load` signature compiles on `x86_64-pc-windows-gnu`. This is covered by `./build-all.sh` but is called out here so a failure is diagnosed as a Windows-compat issue, not a test failure.
- [ ] **Negative pin** — `dead_code_attribute_is_removed` — runtime `#[test]` reading `include_str!("../../response_poll/mod.rs")` (or the post-conversion path) and asserting `!source.contains("dormant during legacy phase")` AND `!source.contains("#[allow(dead_code,")`. Pairs with the matching success-criterion grep.

**Implementation:**

- [ ] Convert `response_poll.rs` to a directory module: `mkdir oriterm_mux/src/pane/io_thread/response_poll`; move current body to `mod.rs` inside it; create `tests.rs` sibling. Update `pane/io_thread/mod.rs:15` — no change needed, `mod response_poll;` already refers to either a file OR a directory with `mod.rs`.
- [ ] Delete the `#[allow(dead_code, reason = "dormant during legacy phase; activates at effect-cutover")]` attribute on `register_host_request_response` (lines 33-36 of current `response_poll.rs`).
- [ ] Add `response_wake_tx: Sender<()>` and `response_wake_rx: Receiver<()>` via `crossbeam_channel::bounded(1)` in `handle.rs::new_with_handle` — mirror the pattern used for `cmd_tx` / `cmd_rx` at lines 131-132.
- [ ] Add `response_wake_rx: Receiver<()>` to `IoThreadConfig` AND to `PaneIoThread`.
- [ ] Add `response_wake_tx: Sender<()>` to `PaneIoHandle`.
- [ ] Update BOTH `crossbeam_channel::select!` blocks in `mod.rs::run` (lines 141-158 and 161-176) to add an arm: `recv(self.response_wake_rx) -> _ => { /* woken; next loop iteration drains commands */ }`. The arm body is intentionally empty — the wakeup is the signal; the next iteration of the outer `loop` does the work.
- [ ] Add `PaneIoHandle::fulfill_clipboard_load(&self, token: &ResponseToken<String>, text: String)` and `PaneIoHandle::fulfill_color_query(&self, token: &ResponseToken<Rgb>, color: Rgb)`. Each calls `token.fulfill(value)` then `self.response_wake_tx.try_send(()).ok()` (ignore `Full` — a pending wake already sufficed).
- [ ] In `effect_router.rs::route_host_request`: call `self.register_host_request_response(request.clone())` AND emit the `MuxEvent::HostClipboardLoad { .. reply: request.reply_token_clone() }` / `MuxEvent::HostColorQuery { .. }` (add a small helper on `HostRequest` to clone out the token without cloning the whole variant, OR just `clone()` the whole variant since `HostRequest: Clone` per `host_request.rs:15`).
- [ ] In `oriterm/src/app/mux_pump/mod.rs` (the `handle_mux_notification` match at `mux_pump/mod.rs:54` — do NOT reach into `PaneIoHandle` directly): add match arms for `MuxNotification::HostClipboardLoad { pane_id, selection, clipboard_char, terminator, reply }` and `MuxNotification::HostColorQuery { pane_id, prefix, index, terminator, reply }`. Each arm calls `self.mux.fulfill_host_request(pane_id, HostReply::ClipboardLoad { token: reply, text })` (or the color variant) — going through the `MuxBackend` trait boundary as established in the Files section above. **DO NOT** call `pane_handle.fulfill_clipboard_load` directly from `oriterm/src/app/` — `PaneIoHandle` is not accessible from `oriterm` (verified: `grep -rn 'PaneIoHandle' oriterm/src/` returns zero hits), and the `MuxBackend` trait is the required boundary per `.claude/rules/crate-boundaries.md`. The legacy closure-based `MuxNotification::ClipboardLoad` variant stays in place UNTIL 01.3 — both paths coexist during 01.2 to minimize blast radius.

**Cleanup (hygiene items per `.claude/rules/impl-hygiene.md`):**

- [ ] **[NOTE]** `response_wake_rx` wake-only arm intentionally has an empty body. Per `.claude/rules/impl-hygiene.md` §Defensive Code for Impossible States, this is NOT a code smell — the empty body IS the semantic: "continue loop, next iteration handles work." Add a single-line `// Woken by response fulfillment — next loop iteration drains.` comment explaining the intent.
- [ ] **[DRIFT]** `ResponseToken<T>` now has two consumers that both take ownership conceptually: `register_host_request_response` (via `HostRequest` field) and `MuxEvent::HostClipboardLoad::reply` (via the cloned variant). Both views share the same `Arc<Mutex<Option<T>>>` slot — correct by construction. Pin this with a `#[test]` `response_token_is_shared_by_clone` at `oriterm_core/src/effect/families/host_request.rs` sibling tests that clones a token, fulfills one clone, and asserts the other clone sees `is_fulfilled() == true`.

**Validation:**

- [ ] All 8 TDD tests transition RED → GREEN.
- [ ] `grep -rn '#\\[allow(dead_code, reason = "dormant during legacy phase'` in `oriterm_mux/` returns zero matches.
- [ ] `timeout 150 cargo test -p oriterm_mux` green.
- [ ] `timeout 150 cargo test -p oriterm_core --test alloc_regression` green.
- [ ] `timeout 150 cargo test -p oriterm_core --test rss_regression` green.
- [ ] `./build-all.sh` (including Windows cross-compile) green.
- [ ] `./test-all.sh` green.
- [ ] `./clippy-all.sh` green.
- [ ] Spec-conformance Section 10.2's cross-reference is satisfied: the dormant `response_poll` arm is now live. Section 10.2 can land without changing this plan.
- [ ] Section 01.2 `status` → `complete` in frontmatter.

---

## 01.3 Delete IoThreadEventProxy, LegacyEventSink, Event::ClipboardLoad/ColorRequest, MuxEventProxy (if unused), drain_notifications shim

**Goal:** Remove all legacy scaffolding. After this subsection, the Effect path is the ONLY path and nothing in the workspace references `LegacyEventSink`, `IoThreadEventProxy`, `DesktopNotificationRecord`, `Event::ClipboardLoad`, `Event::ColorRequest`, or `Term::drain_notifications()`.

**Files (deletions):**
- `oriterm_core/src/effect/sink/legacy/mod.rs` — DELETED.
- `oriterm_core/src/effect/sink/legacy/tests.rs` — DELETED.
- `oriterm_core/src/effect/sink/legacy/` directory — removed (empty after above).
- `oriterm_mux/src/pane/io_thread/event_proxy/mod.rs` — DELETED.
- `oriterm_mux/src/pane/io_thread/event_proxy/tests.rs` — DELETED.
- `oriterm_mux/src/pane/io_thread/event_proxy/` directory — removed.

**Files (edits):**
- `oriterm_core/src/effect/sink/mod.rs:7` — remove `pub mod legacy;`.
- `oriterm_core/src/effect/sink/mod.rs:11` — remove `pub use legacy::LegacyEventSink;`.
- `oriterm_core/src/effect/mod.rs:20` — remove `pub use sink::legacy::DesktopNotificationRecord;`.
- `oriterm_core/src/effect/mod.rs:21` — update `pub use sink::{..}` to remove `LegacyEventSink` from the list.
- `oriterm_core/src/event/mod.rs:42-51` — delete `Event::ClipboardLoad(..)` and `Event::ColorRequest(..)` variants (and their doc comments).
- `oriterm_core/src/event/mod.rs:76-77` — delete their Debug arms. Exhaustive match in `Debug for Event` at line 66 now covers fewer variants; compiler enforces.
- `oriterm_core/src/term/shell_state/mod.rs:345-359` — delete the `impl<L: EventListener + Sync> Term<LegacyEventSink<L>>` block that contains `Term::drain_notifications()` (verified at lines 345-359 in the current source; spec-conformance `00-overview.md:752` cited the old location). The block includes the trailing `}` — remove it in full. The body calls `self.effect_sink.drain_pending_notifications()` which goes away with `LegacyEventSink` itself.
- `oriterm_core/src/term/tests.rs:1627,1654` — delete `ris_clears_pending_notifications` and `drain_notifications_returns_empty_on_second_call` tests (they assert on the deleted shim). Their coverage is absorbed by the new `QueueingEffectSink` drain tests added in 01.1.
- `oriterm_core/src/term/handler/test_helpers/mod.rs` — **MIGRATE**, do NOT leave as-is. `term_with_recording_legacy` (line 74) and `term_with_recording_legacy_and_size` (line 83) construct `Term<LegacyEventSink<RecordingListener>>`. After 01.3, these helpers must be replaced with `Term<QueueingEffectSink>` variants (or `Term<RecordingEffectSink>` if a new test-only sink is introduced). All callers of these helpers in `oriterm_core/src/term/handler/` tests must be migrated to the new signature in this commit. Compiler drives enumeration; each error is a checklist item.
- `oriterm_core/src/image/kitty/tests.rs:271-280` — `term_with_recorder()` uses `LegacyEventSink`. Migrate to a `QueueingEffectSink`-based helper OR to `RecordingEffectSink`. Image protocol tests only assert on rendered cell output, not on events, so the migration is mechanical.
- `oriterm_core/src/image/iterm2/tests.rs:162-170` — parallel to `kitty/tests.rs`. Same migration.
- `oriterm_core/tests/teseq/harness/runner.rs:42,65` — `TeseqRunner` holds `term: Term<LegacyEventSink<RecordedListener>>`. Migrate to `QueueingEffectSink`. The teseq harness uses the term's grid output, not its event stream; the migration is mechanical. Update all dependent teseq test files (`harness/events.rs`, `workflows/mode.rs`, `workflows/edge.rs`, `csi_reports.rs`) that import and use `LegacyEventSink` — compiler drives the full list.
- `oriterm_mux/src/shell_integration/tests.rs:929-937` — `term_with_legacy_sink()` helper. Migrate to `QueueingEffectSink`. Shell integration tests assert on PTY write output, not on notifications, so the migration is mechanical.
- **Verification**: after all migrations, `grep -rn 'LegacyEventSink' oriterm_core/ oriterm_mux/ oriterm/ crates/` must return zero hits across ALL source files (not just the legacy/ directory). The `no_legacy_event_sink_references` deletion pin (below) covers this exhaustively.
- `oriterm_mux/src/pane/io_thread/mod.rs:12` — remove `pub(crate) mod event_proxy;`.
- `oriterm_mux/src/pane/io_thread/handle.rs` — remove the `IoThreadEventProxy` reference from the `grid_dirty` field doc comment at line 99-100.
- `oriterm_mux/src/mux_event/mod.rs:84-92` — delete `MuxEvent::ClipboardLoad { .. formatter: Arc<dyn Fn(&str) -> String + Send + Sync> }` (the old closure-based variant). Its replacement (`HostClipboardLoad` with `ResponseToken`) landed in 01.1.
- `oriterm_mux/src/mux_event/mod.rs:121-126` (approx, update after 01.1's edits) — delete the Debug arm for `ClipboardLoad`.
- `oriterm_mux/src/mux_event/mod.rs:311-319` (approx) — delete `MuxNotification::ClipboardLoad { .. formatter: Arc<dyn Fn> }` and its Debug arm.
- `oriterm_mux/src/in_process/event_pump.rs:82-93` — delete the `MuxEvent::ClipboardLoad` match arm.
- `oriterm_mux/src/mux_event/mod.rs:137-257` (`MuxEventProxy`) — if `MuxEventProxy` is confirmed unused in 01.1's [DRIFT audit] (blind-spot §6), delete the entire `MuxEventProxy` struct + impl block here AND delete its test fixtures at `oriterm_mux/src/mux_event/tests.rs:11,15,25,214,384,437,476,507` (the 6 tests that construct `MuxEventProxy`). The two `Event::ChildExit` tests at lines 160 and 233 either migrate to `MuxEvent::PaneExited` assertions or delete if their coverage is absorbed by the router matrix in 01.1. If `MuxEventProxy` is still used by an out-of-scope path, file a `/add-bug` (severity major) tracking its removal and leave it in place with `#[deprecated(note = "...")]`; do NOT leave dead-code without the artifact.
- `oriterm/src/app/**` — remove any lingering `MuxNotification::ClipboardLoad` handler (closure-based); only the `HostClipboardLoad` / `HostColorQuery` paths remain.
- Callers of `Term::drain_notifications()` (the DELETED shim, not the mux method): grep `grep -rn '\.drain_notifications()' oriterm_core/src/ oriterm/src/` during implementation. Note that `oriterm/src/app/window_management/mod.rs:214` and `oriterm/src/app/mux_pump/mod.rs:44` call `mux.drain_notifications(&mut self.notification_buf)` — that is the `InProcessMux` method at `oriterm_mux/src/in_process/event_pump.rs:102`, NOT the `Term` shim. It STAYS. Disambiguate each call site before deleting anything.

**Tests (written FIRST — VERIFIED RED before implementation — "red" here means the grep-based guards fail because the names still exist):**

- [ ] **Deletion pin** — `no_legacy_event_sink_references` — reads the output of `grep -rn 'LegacyEventSink' oriterm_core/ oriterm_mux/ oriterm/ crates/` and asserts zero hits (excluding `.git/`, `target/`, and this plan doc). Expressed as a `#[test]` that `std::process::Command::new("grep")`s the workspace — gracefully skips on Windows (`reseq`-style skip protocol per `.claude/rules/tests.md` §Graceful Skip Protocol). On Linux/macOS this MUST fail before 01.3's deletions, pass after.
- [ ] **Deletion pin** — `no_io_thread_event_proxy_references` — same shape, for `IoThreadEventProxy`.
- [ ] **Deletion pin** — `no_event_clipboardload_or_colorrequest_variants` — asserts `grep -n 'Event::ClipboardLoad\\|Event::ColorRequest' oriterm_core/src/ oriterm_mux/src/ oriterm/src/` returns zero hits.
- [ ] **Deletion pin** — `no_drain_notifications_shim` — asserts `grep -rn 'fn drain_notifications' oriterm_core/src/term/` returns zero hits. The `drain_notifications` name also lives on `InProcessMux::drain_notifications` (at `oriterm_mux/src/in_process/event_pump.rs:102`) — THAT method stays (it drains `MuxNotification`s from the mux). The grep narrows to `oriterm_core/src/term/` to pin ONLY the shim deletion.
- [ ] **Deletion pin** — `no_desktop_notification_record_references` — `grep -rn 'DesktopNotificationRecord' oriterm_core/ oriterm_mux/` returns zero hits.
- [ ] **Negative pin** — `event_enum_variants_exhaustive_list` — a `#[test]` that constructs every remaining `Event` variant and asserts the list matches the expected set (Wakeup, Bell, Title, ResetTitle, IconName, ResetIconName, ClipboardStore, PtyWrite, CursorBlinkingChange, Cwd, CommandComplete, MouseCursorDirty, ChildExit — note ClipboardLoad and ColorRequest are gone). Count assertion + name list. Pins that no re-introduction of the deleted variants compiles.
- [ ] **Regression pin** — `effect_cutover_final_state_full_run` — a longer integration test that spawns a `PaneIoThread<QueueingEffectSink>`, pushes an OSC 52 query (`\x1b]52;c;?\x1b\\`) through the VTE processor, fulfills the clipboard, and asserts end-to-end PTY reply. This is the smoke test proving the full pipeline works without ANY legacy scaffolding.

**Implementation:**

- [ ] Delete files/directories listed above (`rm` via implementation, tracked in the commit).
- [ ] Apply the edits listed above.
- [ ] Run the compiler repeatedly during the deletion to catch all downstream `Event::ClipboardLoad` / `Event::ColorRequest` match arms — the compiler is the driver, not grep. Fix each error by deleting the arm (NOT by adding `_ => ()`). The deletion should be SURGICAL — each arm's deletion site is a confirmed consumer of the deleted variant.
- [ ] `rustfmt` the files that had variants deleted so the Debug impl's formatting stays clean.
- [ ] `InProcessMux::drain_notifications` (at `in_process/event_pump.rs:102`) — this is DIFFERENT from the `Term::drain_notifications` shim and STAYS. Verify via re-read that nothing in 01.3's deletion set touches it.

**Cleanup (hygiene items):**

- [ ] **[WASTE]** `oriterm_core/src/effect/sink/legacy/tests.rs` had `presentation_effect_count` tests (e.g. at `legacy/tests.rs:259` based on the grep hit at `legacy/tests.rs:225,231,259,268`). Those tests die with the file. Audit: are any of those assertions portable to `QueueingEffectSink` tests? (E.g. "Presentation effects are logged, not queued" was a LegacyEventSink behavior; on QueueingEffectSink, Presentation effects ARE queued per the router's decision in 01.1.) If any LegacyEventSink test captured a semantic that survives to `QueueingEffectSink`, port it BEFORE deleting the file. If not, delete cleanly.
- [ ] **[LEAK:scattered-knowledge]** `selection_to_legacy` at `legacy/mod.rs:195-201` — the `ClipboardSelection` → `ClipboardType` helper. Verify the canonical home (promoted in 01.1 into `effect_router/mod.rs` or `mux_event/clipboard.rs`) survives this deletion and no other call site re-implemented the same mapping. `grep -rn 'ClipboardSelection.*Clipboard.*ClipboardType::Clipboard\\|ClipboardSelection.*Primary.*ClipboardType::Selection' oriterm_mux/ oriterm_core/` must return exactly ONE hit after this subsection lands.
- [ ] **[DRIFT — Event::ChildExit resolution]** — per blind-spot §3 and the 01.1 audit entry. The full trace at the time of this plan: `Event::ChildExit(code)` is matched in `IoThreadEventProxy::send_event` at `event_proxy/mod.rs:143-148` AND in `MuxEventProxy::send_event` at `mux_event/mod.rs:245`. After 01.1, the IO thread no longer wraps `Term` in an `EventListener`, so `IoThreadEventProxy::send_event` is unreachable on the IO thread. The `MuxEventProxy::send_event(Event::ChildExit)` path may survive if any non-IO-thread pane lifecycle signal still flows through `MuxEventProxy` (e.g. pane shutdown from an external source). 01.3's investigation step: (1) `grep -rn 'Event::ChildExit(' oriterm_core/ oriterm_mux/ oriterm/` enumerates every EMITTER — if the only producers are the deleted event_proxy paths, `Event::ChildExit` is dead and is DELETED alongside `ClipboardLoad`/`ColorRequest` in this subsection (update `oriterm_core/src/event/mod.rs` accordingly); (2) if any non-deleted producer remains, either that producer is migrated to `Effect::Host(HostEffect::ChildExit)` in 01.3 OR the plan files a `/add-bug` and leaves `Event::ChildExit` undeleted with the concrete bug ID inline. Document the answer IN the plan body before marking 01.3 complete — no dangling variant.

**Validation:**

- [ ] All 7 deletion/regression pins GREEN.
- [ ] `grep -rn 'LegacyEventSink\\|IoThreadEventProxy\\|DesktopNotificationRecord'` workspace-wide = 0.
- [ ] `grep -rn 'Event::ClipboardLoad\\|Event::ColorRequest'` = 0 in src files.
- [ ] `grep -rn 'fn drain_notifications' oriterm_core/src/term/` = 0.
- [ ] `timeout 150 cargo test -p oriterm_core` green (legacy tests are gone, which is intentional).
- [ ] `timeout 150 cargo test -p oriterm_mux` green.
- [ ] `timeout 150 cargo test -p oriterm` green.
- [ ] `./build-all.sh` (debug + release + Windows cross-compile) green.
- [ ] `./test-all.sh` green.
- [ ] `./clippy-all.sh` green (zero new warnings — now that `#[allow(dead_code)]` is removed from `IoThreadEventProxy` and `LegacyEventSink`, any residual dead-code offenders surface here).
- [ ] Teseq OSC clipboard/color scenarios still green: `timeout 150 cargo test -p oriterm_core --test teseq osc::` (regression gate against the OSC 0/1/2/4/10/11/12/52 basics).
- [ ] Section 01.3 `status` → `complete` in frontmatter.

---

## 01.4 Daemon-mode IPC for `HostRequest` — design + bug filing + follow-up plan

**Goal:** Resolve the daemon-mode IPC incompatibility surfaced in blind-spot §2 and architectural-risk §1. `ResponseToken<T>` is `Arc<Mutex<Option<T>>>` (`oriterm_core/src/effect/families/host_request.rs:54-56`) — process-local only. `oriterm_mux/src/protocol/messages.rs:396-401` (`NotifyClipboardLoad`) today serializes only `pane_id: PaneId` + `clipboard_type: u8`; there is no request-ID and no reply PDU. As a result, daemon-mode OSC 52 load (`\x1b]52;c;?\x1b\\`) CANNOT round-trip through the IPC transport even after 01.3 lands.

This subsection has three exit paths; it is **not** optional. Choose one and execute.

**Path A — In-scope design (recommended when daemon-mode is part of the next release):**
- [ ] Add `NotifyHostRequest { pane_id, request_id: u64, payload: HostRequestPayload }` to `oriterm_mux/src/protocol/messages.rs` (wire variant). `HostRequestPayload` is the serialize-safe projection of `HostRequest` — drops `ResponseToken`, carries enough context to rebuild the reply.
- [ ] Add `ReplyHostRequest { request_id: u64, payload: HostReplyPayload }` as the paired reply PDU.
- [ ] Server side (daemon) stores an in-memory `HashMap<u64, ResponseToken<...>>` keyed by `request_id`; incoming `ReplyHostRequest` fulfills the matching token and signals the wake channel introduced in 01.2.
- [ ] Client side (UI process) receives `NotifyHostRequest`, performs the host action (clipboard read, color query), sends `ReplyHostRequest`.
- [ ] All the above lives in this section as Files / Tests / Implementation blocks with RED→GREEN TDD discipline — essentially a new subsection rigor set.
- [ ] Document in `00-overview.md` that daemon-mode is in scope.

**Path B — File bug + cross-plan deferral (when daemon-mode is NOT part of the current release):**
- [ ] Invoke `/add-bug` with severity **major**, subsystem `oriterm_mux/protocol`, title `"HostRequest round-trip not wired through daemon IPC"`. The bug artifact MUST include: repro (run `oriterm` against daemon backend, emit `\x1b]52;c;?\x1b\\`, observe clipboard never responds), root cause (`ResponseToken<T>` process-local; `NotifyClipboardLoad` PDU has no request-ID or reply path), fix plan outline (mirror Path A).
- [ ] Add `<!-- blocked-by: plans/bug-tracker/BUG-XX-NNN -->` cross-link inside 01.4 so `/review-plan-verify` can mechanically detect the artifact.
- [ ] Update `00-overview.md` to explicitly scope daemon-mode OUT of this plan's goals.
- [ ] The bug-tracker entry is the concrete artifact — this is NOT a "deferred" checkbox. Per `CLAUDE.md §Bug Discipline`, filing via `/add-bug` IS the tracking mechanism.

**Path C — Spin out to a new plan (when the daemon work is large enough to warrant its own plan):**
- [ ] Create `plans/daemon-host-request/` (via `Skill: create-plan`) with a dedicated overview + sections covering PDU design, server-side token map, client-side reply, integration tests.
- [ ] Cross-link here via `<!-- see-also: plans/daemon-host-request/ -->`.
- [ ] Update `00-overview.md` to explicitly scope daemon-mode out of this plan and point to the new plan.
- [ ] The new plan is the concrete artifact — this is NOT deferral.

**Files (always, regardless of path):**
- `plans/effect-cutover/section-01-migrate-mux-consumer.md` — update the selected path's checklist items to `[x]` after execution; document which path was chosen and why in the validation notes.
- `plans/effect-cutover/00-overview.md` — scope-of-plan text is updated to match the chosen path.

**Tests (minimal — most work happens in Path A or in the follow-up artifact):**
- [ ] `daemon_notify_clipboard_load_has_request_id` — if Path A chosen: assert the wire format carries a non-zero `request_id`. If Path B/C: this test is tracked in the bug/follow-up plan, not here.
- [ ] `host_request_process_locality_is_documented` — a `#[cfg(test)]` test that reads `oriterm_core/src/effect/families/host_request.rs` and asserts the doc comment on `ResponseToken<T>` explicitly mentions "process-local, cannot cross IPC — see plans/effect-cutover/section-01 §01.4 for daemon-mode design". This pin keeps the documentation honest regardless of which path was chosen.

**Implementation:**
- [ ] Pick ONE of Path A, Path B, Path C. Execute the checklist for that path.
- [ ] Update `ResponseToken<T>` doc comment at `oriterm_core/src/effect/families/host_request.rs:43-52` to include the process-locality warning.
- [ ] Add the chosen path's name + rationale to this subsection's notes before marking complete.

**Cleanup (hygiene items):**
- [ ] **[NOTE]** If Path B or C is chosen, `00-overview.md §Goal` does NOT promise daemon-mode support — the goal list must be accurate. Update it explicitly rather than leaving an implicit unmet deliverable.
- [ ] **[DRIFT]** If Path A is chosen, the daemon PDU version number in `oriterm_mux/src/protocol/` must advance (compatibility-breaking change) AND any existing daemon client tests must be updated atomically. This is within 01.4's scope if Path A; no deferral.

**Validation:**
- [ ] The chosen path is executed end-to-end (Path A: code lands; Path B: bug filed + `00-overview.md` updated; Path C: new plan directory created + `00-overview.md` updated).
- [ ] `host_request_process_locality_is_documented` pin GREEN.
- [ ] `./build-all.sh` (debug + release + Windows cross-compile) green.
- [ ] `./test-all.sh` green.
- [ ] `./clippy-all.sh` green.
- [ ] Section 01.4 `status` → `complete` in frontmatter.

---

## 01.N Completion Checklist

### TDD Discipline (MUST be FIRST — per `.claude/rules/tests.md` §TDD for Bugs)

- [ ] 01.1's 14 TDD tests written and VERIFIED RED before any implementation (5 positive, 2 negative, 5 matrix/coverage, 2 semantic/regression).
- [ ] 01.2's 8 TDD tests written and VERIFIED RED before any implementation.
- [ ] 01.3's 7 deletion/regression pins written and VERIFIED RED (pre-deletion state) before any deletions land.
- [ ] 01.4's 2 pins (daemon-mode documentation) written — Path A adds its own TDD matrix inside the subsection; Path B/C files the tracking artifact.

### Ordering gate (crate dependency direction per `.claude/rules/crate-boundaries.md`)

- [ ] Changes land in this order: 01.1 (atomic sink swap + router + new `MuxEvent`/`MuxNotification` variants — single commit; spans `oriterm_mux` **AND** `oriterm` AND `oriterm_core` — `oriterm` because `mux_pump/mod.rs` has an exhaustive `match` on `MuxNotification` requiring new stub arms; `oriterm_core` because 01.1 adds `#[allow(dead_code, reason = "removed in effect-cutover 01.3")]` to `LegacyEventSink` in `oriterm_core/src/effect/sink/legacy/mod.rs:45` — the effect TYPES themselves are pre-existing and unchanged, but the dead-code suppression must land in 01.1 immediately after the construction sites stop referencing the type) → 01.2 (idle-wake channel + response_poll activation + main-thread fulfill handlers — replaces 01.1 stub arms with real `MuxBackend::fulfill_host_request` calls; spans `oriterm_mux` + `oriterm`) → 01.3 (legacy deletions — spans `oriterm_core` + `oriterm_mux` + `oriterm` in a single commit because `LegacyEventSink` is referenced from `event_proxy/mod.rs`; compiler refuses any intermediate state) → 01.4 (daemon IPC audit — edits depend on chosen path).

### Matrix coverage

- [ ] **Matrix dimensions**: Effect variant × routing target × drain entry point (handle_bytes, drain_commands, handle_sync_timeout) × sink implementation. Per blind-spot §4, `QueueingEffectSink` is the ONLY sink that exercises the router (`VoidEffectSink::drain_into` is a no-op at `oriterm_core/src/effect/sink/mod.rs:90`); the matrix uses `QueueingEffectSink` exclusively via `make_sync_thread_queueing()`.
- [ ] **Semantic pins** (at least one per subsection; the single-most-critical pin of the section is `response_poll_idle_wake_unblocks_select`):
  - [ ] `pane_io_thread_accepts_queueing_effect_sink` (01.1)
  - [ ] `effect_router_drain_zero_alloc_steady_state` (01.1)
  - [ ] `multi_chunk_parse_drains_between_chunks` (01.1) — pins blind-spot §5 (no unbounded accumulation)
  - [ ] `router_matrix_uses_queueing_sink_exclusively` (01.1) — pins blind-spot §4 (router coverage)
  - [ ] `response_poll_idle_wake_unblocks_select` (01.2) — THE critical pin
  - [ ] `response_poll_emits_pty_write_on_fulfill` (01.2)
  - [ ] `effect_cutover_final_state_full_run` (01.3)
  - [ ] `host_request_process_locality_is_documented` (01.4)
- [ ] **Negative pins** (every positive test has a paired negative):
  - [ ] `legacy_event_sink_construction_removed_from_local_domain` (01.1)
  - [ ] `legacy_event_sink_construction_removed_from_handoff` (01.1)
  - [ ] `visual_bell_is_logged_not_dropped_silently` (01.1)
  - [ ] `clear_pending_notifications_collapses_preceding` (01.1) — pins the intra-batch collapse
  - [ ] `clear_pending_notifications_does_not_retro_collapse_across_drains` (01.1) — pins the cross-batch boundary
  - [ ] `response_poll_token_requires_fulfillment` (01.2)
  - [ ] `dead_code_attribute_is_removed` (01.2)
  - [ ] `no_legacy_event_sink_references` (01.3)
  - [ ] `no_io_thread_event_proxy_references` (01.3)
  - [ ] `no_event_clipboardload_or_colorrequest_variants` (01.3)
  - [ ] `no_drain_notifications_shim` (01.3)
  - [ ] `no_desktop_notification_record_references` (01.3)
  - [ ] `event_enum_variants_exhaustive_list` (01.3)
- [ ] **Cross-pattern matrix**: the router handles THREE flow patterns — (a) synchronous push during VTE parsing (`handle_bytes` → drain at end of chunk), (b) command-driven effect emission (`drain_commands` → `poll_pending_responses` → drain), (c) sync-timeout replay (`handle_sync_timeout` → `post_parse_housekeeping` → drain). Each of the three entry points is tested for at least one `HostEffect`, one `PtyEffect`, and one `HostRequest`.
- [ ] **Count assertion**: the effect variant matrix in `effect_router/tests.rs` ends with `assert_eq!(covered_variants.len(), ALL_HOST_EFFECT_VARIANTS.len() + ALL_PTY_EFFECT_VARIANTS.len() + ALL_UI_EFFECT_VARIANTS.len() + ALL_PRESENTATION_VARIANTS.len() + ALL_HOST_REQUEST_VARIANTS.len())` so adding a new variant forces a test update per `.claude/rules/tests.md` §Matrix Testing Rule.

### Rules weaving (per `.claude/rules/impl-hygiene.md` + `.claude/rules/code-hygiene.md` + `.claude/rules/crate-boundaries.md` + `.claude/rules/oriterm_core.md` + `.claude/rules/oriterm_mux.md` + `.claude/rules/tests.md`)

- [ ] **No SSOT drift**: `Effect::HostRequest` → PTY reply formatting goes through `format_clipboard_reply` / `format_color_reply` at `oriterm_core/src/effect/families/host_request.rs:110,126`. The router does NOT format replies. Verified by: `grep -rn 'format!("\\\\x1b\\]52\\|format!("\\\\x1b\\]4\\|format!("\\\\x1b\\]10\\|format!("\\\\x1b\\]11\\|format!("\\\\x1b\\]12'` in `oriterm_mux/src/pane/` returns zero hits after this section lands. The reply formatters are called from exactly one place: `register_host_request_response` at `response_poll/mod.rs`. **Exception — daemon client formatter**: `oriterm_mux/src/backend/client/notification.rs:29` (`osc52_response_formatter`) is a pre-existing daemon-side formatter that reconstructs the OSC 52 response for IPC round-trips. This formatter is OUTSIDE the `oriterm_mux/src/pane/` grep scope and is explicitly excluded — it operates on a different code path (daemon client receiving a `NotifyClipboardLoad` PDU from the server, not the IO thread's response-poll path). The SSOT assertion is scoped to the IO-thread path only; the daemon client formatter is audited separately in 01.4.
- [ ] **No duplicated dispatch** (`.claude/rules/impl-hygiene.md` §LEAK:duplicated-dispatch): the Effect→MuxEvent match lives ONLY in `effect_router/mod.rs`. No parallel match in `handle_bytes`, `handle_sync_timeout`, `drain_commands`, or `handle_command`. Verified by: `grep -n 'Effect::Host\\|Effect::Pty\\|Effect::HostRequest\\|Effect::Ui\\|Effect::Presentation' oriterm_mux/src/pane/io_thread/` shows match arms ONLY inside `effect_router/mod.rs` (and its `tests.rs`).
- [ ] **No registration sync drift**: adding `MuxEvent::DesktopNotification`, `MuxEvent::HostClipboardLoad`, `MuxEvent::HostColorQuery` requires synchronized updates at — (1) `oriterm_mux/src/mux_event/mod.rs::MuxEvent` enum, (2) `impl fmt::Debug for MuxEvent` at line 94, (3) `oriterm_mux/src/in_process/event_pump.rs::poll_events` exhaustive match, (4) `oriterm_mux/src/mux_event/tests.rs` where Debug output is pinned. All 4 updated atomically in 01.1. Similarly for the new `MuxNotification::DesktopNotification` and `MuxNotification::ClearPendingDesktopNotifications` — (1) enum, (2) Debug impl at line 327, (3) forwarding site in event_pump, (4) any downstream consumer in `oriterm/src/app/**` that pattern-matches `MuxNotification`. **Additionally** the following daemon-side files route `MuxNotification` variants and MUST be audited for sync in the SAME commit as the enum additions: (5) `oriterm_mux/src/backend/client/notification.rs` — `pdu_to_notification()` translates wire PDUs to `MuxNotification`; new variants `HostClipboardLoad`/`HostColorQuery` are NOT reachable via the daemon PDU path (the server side emits the PDU, the client receives it — daemon fulfillment is handled separately in 01.4); verify that `pdu_to_notification` has a `_` catch-all for unknown PDUs so new variants don't cause a compile error here. (6) `oriterm_mux/src/server/notify/mod.rs` — daemon server's notification dispatch; audit for any `match` on `MuxNotification` that would become non-exhaustive. If an exhaustive match exists there, add `HostClipboardLoad { .. } => {}` and `HostColorQuery { .. } => {}` stubs (the server side routes these differently via the pending-response registry, not notification broadcast).
- [ ] **No LEAK:scattered-knowledge**: `ClipboardSelection` → `ClipboardType` translation lives at exactly one site post-01.3 (the legacy helper at `legacy/mod.rs:195-201` is deleted). `selection_to_mux_clipboard_type` in `effect_router/mod.rs` (private) is the single canonical home.
- [ ] **No file size violations** (`.claude/rules/code-hygiene.md` §File Size, 500-line limit, proactive split at 450):
  - [ ] `oriterm_mux/src/pane/io_thread/mod.rs` — currently 435 lines. 01.1 adds fields + three drain call sites + `mod effect_router;`. Proactive split at 450 required: extract `run()`'s two `crossbeam_channel::select!` bodies (lines 138-178) into a private `run_loop.rs` submodule BEFORE the line count crosses the threshold.
  - [ ] `oriterm_mux/src/pane/io_thread/effect_router/mod.rs` — new file; stay under 500 lines by extracting per-variant helpers into `effect_router/host.rs`, `effect_router/pty.rs`, etc. if the single file crosses 450.
  - [ ] `oriterm_mux/src/mux_event/mod.rs` — currently 355 lines. Adding 3 new MuxEvent variants + 2 new MuxNotification variants adds ~80 lines. Approaches 450 — proactively split `MuxNotification` into `oriterm_mux/src/mux_event/notification.rs` if it crosses 450.
  - [ ] `oriterm_mux/src/mux_event/tests.rs` — currently 525 lines (already over 500). 01.1 split into `mux_event/tests/proxy.rs` + `mux_event/tests/debug.rs` BEFORE adding new Debug pins. Split task is MANDATORY, not conditional.
  - [ ] `oriterm_mux/src/pane/io_thread/tests.rs` — currently 2339 lines (massively over). 01.1 adds new tests into SIBLING files (`effect_router/tests.rs`, `handle/tests.rs`), not this file. A separate `/add-bug` (severity major) tracks the overall split of the existing 2339-line file.
  - [ ] `oriterm_core/src/effect/sink/legacy/tests.rs` — 364 lines; approaches 500. No new additions here in 01.1; the whole file is DELETED in 01.3.
  - [ ] `oriterm_core/src/term/shell_state/mod.rs` — 362 lines; approaches 500. 01.3 DELETES 15 lines (the `drain_notifications` shim block at 345-359). Net reduction; no split needed.
  - [ ] `oriterm_core/tests/alloc_regression.rs` — 386 lines; approaches 500. 01.1 may add the `effect_router_drain_zero_alloc_steady_state` test; if that pushes past 450, split into per-subsystem files (`alloc_regression/render.rs`, `alloc_regression/effect_router.rs`).
- [ ] **Cross-platform** (`.claude/rules/tests.md` §Cross-Platform Verification): `cargo build --target x86_64-pc-windows-gnu` green at EVERY subsection boundary. The `adopt_pane` path in `domain/handoff/mod.rs` is Windows-specific-flavored but compiles on all platforms. The idle-wake channel uses `crossbeam_channel` which is cross-platform.
- [ ] **Alloc regression**: `oriterm_core/tests/alloc_regression.rs` green at EVERY subsection boundary. The `effects_buf: Vec<Effect>` scratch vector is grow-only; `drain_into` reuses its capacity per the existing contract at `sink/mod.rs:77-80`.
- [ ] **RSS regression**: `oriterm_core/tests/rss_regression.rs` green. `pending_responses: Vec<PendingResponse>` bounded — 01.2 adds a `debug_assert!(self.pending_responses.len() < MAX_PENDING_RESPONSES)` with `MAX_PENDING_RESPONSES = 64` to pin that unfulfilled tokens don't accumulate unboundedly; if a production scenario genuinely exceeds this, it is a bug filed via `/add-bug`.
- [ ] **Crate boundary discipline** (`.claude/rules/crate-boundaries.md`): `crossbeam_channel` stays out of `oriterm_core`. The wake channel lives in `oriterm_mux` only. `ResponseToken<T>` in `oriterm_core` stays a plain data type.
- [ ] **Plan-document discipline** — `plans/spec-conformance/00-overview.md` is 951 lines and `plans/spec-conformance/section-10-osc-suite.md` is 1255 lines (both flagged by the audit). This section does NOT edit those files; plan documents are not subject to the 500-line source-file limit (the limit applies to `src/**`). The audit warnings are informational — no action required here.

### Catalog + cross-section updates

- [ ] Spec-conformance Section 10.2's success criterion dependency is now satisfied: `plans/spec-conformance/section-10-osc-suite.md:14` references the dead-code gate removal. After 01.2 lands, the gate is gone and Section 10.2 can write `response_poll_emits_pty_write_on_fulfill` as a pure verification test against already-working infrastructure. No cross-plan file edit is required here (Section 10.2's `depends_on` already lists `"effect-cutover"`).
- [ ] No other plans depend on this plan's deliverables (verified by `grep -rn 'effect-cutover' plans/` — only spec-conformance Section 10 is the consumer).
- [ ] The `plans/effect-cutover/00-overview.md` Goal bullets are ALL ticked (each bullet maps to a success criterion here):
  - [ ] "Migrate `oriterm_mux` IO thread from `LegacyEventSink` → `QueueingEffectSink`" ↔ 01.1 (atomic sink swap + router) + 01.2 (wake channel live).
  - [ ] "Migrate `oriterm` application from `Event`-based dispatch → `Effect`-based dispatch" ↔ 01.2's main-thread wiring (partial — clipboard/color only, since those were the only closure-based `Event` variants).
  - [ ] "Delete `LegacyEventSink`, `Event::ClipboardLoad`, `Event::ColorRequest`" ↔ 01.3.
  - [ ] "Remove the `drain_notifications()` thin shim" ↔ 01.3.
  - [ ] "All consumers process effects via `drain_into()` — no separate notification drain" ↔ verified by 01.3's `no_drain_notifications_shim` pin.
  - [ ] "Add an idle-wake channel so a fulfilled `ResponseToken` unblocks `select!`" ↔ 01.2 (`response_poll_idle_wake_unblocks_select` pin).

### Existing test suites (regression gates)

- [ ] All existing teseq OSC tests pass — `timeout 150 cargo test -p oriterm_core --test teseq osc::`.
- [ ] All existing tack tests pass — `timeout 150 cargo test -p oriterm_core --test tack`.
- [ ] Alloc regression unchanged — `timeout 150 cargo test -p oriterm_core --test alloc_regression`.
- [ ] RSS regression unchanged — `timeout 150 cargo test -p oriterm_core --test rss_regression`.
- [ ] Control-flow / event-loop purity unchanged — `timeout 150 cargo test -p oriterm --test main_window` (if the app-level tests cover the idle-wake path; otherwise manually verify `oriterm/src/app/event_loop_helpers/tests.rs::compute_control_flow` still asserts `ControlFlow::Wait` when the pending queue is empty).
- [ ] No new `#[ignore]` annotations added in any touched test file — per `.claude/rules/tests.md` §Test Hygiene, `#[ignore]` budget is strict.

### Final verification

- [ ] `./build-all.sh` green (debug + release + Windows cross-compile via `cargo build --target x86_64-pc-windows-gnu`).
- [ ] `./test-all.sh` green (debug workspace test sweep).
- [ ] Explicit release-mode test run — `timeout 150 cargo test --workspace --release` green (release-mode alloc regressions and `#[cfg(debug_assertions)]` divergence are invisible to `./test-all.sh`).
- [ ] `./clippy-all.sh` green (zero new warnings under `deny(clippy::all)` + nursery + `dead_code = "deny"`).
- [ ] Section frontmatter `status` → `complete`; each sub-entry (01.1, 01.2, 01.3, 01.4, 01.N) → `complete`.
- [ ] `00-overview.md` `status` → `complete`; Goal bullets all ticked.
- [ ] `index.md` Section 01 row status → `Complete`.
- [ ] `plans/spec-conformance/section-10-osc-suite.md` — NO edit here; Section 10's review cycle will detect the newly unblocked state on its next `/review-plan` or `/continue-roadmap` pass.
- [ ] `/tpr-review` final (full-section) passed — dual-source codex + gemini, all findings resolved. `third_party_review.status` → `findings_accepted_by_user` (or equivalent final state per `/review-plan verify`).
- [ ] `/impl-hygiene-review last commit` passed (after `/tpr-review` is clean).

**Exit Criteria:** The IO thread's `Term<S>` uses `QueueingEffectSink`. Every `Effect` variant is routed to `MuxEvent`/`MuxNotification` in one canonical match. `HostRequest` variants register `PendingResponse` entries and a fulfilled `ResponseToken` immediately wakes the IO thread's `select!` via a dedicated wake channel. `LegacyEventSink`, `IoThreadEventProxy`, `DesktopNotificationRecord`, `Event::ClipboardLoad`, `Event::ColorRequest`, and `Term::drain_notifications()` are deleted. Spec-conformance Section 10.2 is unblocked — it can write its OSC 52 round-trip test against already-live infrastructure.
