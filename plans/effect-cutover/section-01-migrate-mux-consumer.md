---
section: "01"
title: "Migrate Mux Consumer from LegacyEventSink to QueueingEffectSink"
status: not-started
reviewed: false
goal: "Replace `LegacyEventSink<IoThreadEventProxy>` with `QueueingEffectSink` as the IO thread's `Term<S>` effect sink so the IO thread subscribes to `Effect` directly. Route every `Effect` variant into the existing `MuxEvent` / `MuxNotification` stream and into `pending_responses` (for `HostRequest` variants) in the IO thread's own drain loop, add an idle-wake channel so a fulfilled `ResponseToken` immediately unblocks the `crossbeam_channel::select!` in `PaneIoThread::run`, and delete `IoThreadEventProxy`, `LegacyEventSink`, `Event::ClipboardLoad`, `Event::ColorRequest`, and the `Term::drain_notifications()` shim once all consumers are wired through `drain_into()`. This section unblocks spec-conformance Section 10.2 (which removes the `#[allow(dead_code)]` gate on `PaneIoThread::register_host_request_response` and activates the OSC 52 / OSC 10/11/12 `ResponseToken` round-trip)."
success_criteria:
  - "`oriterm_mux/src/pane/io_thread/mod.rs` declares `PaneIoThread<QueueingEffectSink>` (not `PaneIoThread<LegacyEventSink<IoThreadEventProxy>>`); both `domain/local.rs::LocalDomain::spawn_pane` and `domain/handoff/mod.rs::adopt_pane` construct `Term::new(..., QueueingEffectSink::new())` and no longer construct `LegacyEventSink::new(IoThreadEventProxy::new(..))` or `IoThreadEventProxy::new(..)` anywhere."
  - "After each VTE parse chunk and after each command batch, the IO thread calls `effect_sink.drain_into(&mut effects_buf)` into a reusable scratch `Vec<Effect>` owned by `PaneIoThread` (grows-only, never shrinks inside `draw_frame`-equivalent hot paths per `.claude/rules/impl-hygiene.md` §Data Flow) and routes every `Effect` variant into `MuxEvent` via a single canonical match in one named function — no duplicated dispatch tables."
  - "`Effect::HostRequest(HostRequest::ClipboardLoad { .. })` and `Effect::HostRequest(HostRequest::ColorQuery { .. })` are registered with `pending_responses` via `PaneIoThread::register_host_request_response(request)` — the `#[allow(dead_code, reason = \"dormant during legacy phase; activates at effect-cutover\")]` attribute on `register_host_request_response` at `oriterm_mux/src/pane/io_thread/response_poll.rs:33-36` is REMOVED and `grep -rn '#\\[allow(dead_code, reason = \"dormant during legacy phase'` in `oriterm_mux/` returns zero matches."
  - "A fulfilled `ResponseToken` causes `PaneIoThread::run` to poll `pending_responses` within one `select!` iteration with NO unrelated byte or command activity required. Concretely: a new `response_wake_rx: Receiver<()>` arm is added to BOTH `crossbeam_channel::select!` blocks in `PaneIoThread::run` (the sync-deadline arm and the no-deadline arm); when the main thread calls `ClipboardProvider::fulfill_clipboard_load(token, text)` or `ColorProvider::fulfill_color_query(token, rgb)` (new mux-side helper), the fulfill call also signals `response_wake_tx` which unblocks the `select!`. The semantic pin `response_poll_idle_wake_unblocks_select` in `oriterm_mux/src/pane/io_thread/response_poll/tests.rs` verifies this with NO PTY bytes and NO command traffic — fulfillment alone drives the PtyEffect::Write emission."
  - "`Event::ClipboardLoad(..)` and `Event::ColorRequest(..)` are DELETED from `oriterm_core/src/event/mod.rs` (the `Event` enum loses those two variants and the `Arc<dyn Fn>` closure fields); every `match event { Event::ClipboardLoad(..) => ..; Event::ColorRequest(..) => .. }` arm in the workspace is removed; exhaustive-match compile errors in `MuxEventProxy::send_event`, `IoThreadEventProxy::send_event`, the Debug impl on `Event`, and any downstream consumer are fixed — not by `_ => {}` wildcards, but by removing the arm."
  - "`LegacyEventSink`, `IoThreadEventProxy`, and `DesktopNotificationRecord` are DELETED. Files removed: `oriterm_core/src/effect/sink/legacy/mod.rs`, `oriterm_core/src/effect/sink/legacy/tests.rs`, and the entire `oriterm_core/src/effect/sink/legacy/` directory; `oriterm_mux/src/pane/io_thread/event_proxy/mod.rs` and `oriterm_mux/src/pane/io_thread/event_proxy/tests.rs`. The `pub use sink::legacy::DesktopNotificationRecord` re-export at `oriterm_core/src/effect/mod.rs:20` is removed. The `mod legacy;` and `pub use legacy::LegacyEventSink;` in `oriterm_core/src/effect/sink/mod.rs:7,11` are removed. `grep -rn 'LegacyEventSink\\|IoThreadEventProxy\\|DesktopNotificationRecord'` across the workspace returns zero hits (excluding the git history)."
  - "`Term::drain_notifications()` (the shim that drains `LegacyEventSink::pending_notifications` — currently declared at `oriterm_core/src/term/shell_state/mod.rs:218` per spec-conformance `00-overview.md:752`) is DELETED. Desktop notifications flow exclusively through `Effect::Host(HostEffect::DesktopNotification { .. })` → `drain_into` → IO thread's Effect→MuxEvent router → `MuxNotification`. A new `MuxNotification::DesktopNotification { pane_id, source, title, body }` variant is added in `oriterm_mux/src/mux_event/mod.rs` (with parallel `MuxEvent::DesktopNotification { .. }` if the existing event-pump double-indirection is kept; if single-indirection is adopted, update `in_process/event_pump.rs` accordingly — BOTH paths must land atomically to avoid registration-sync drift per `.claude/rules/impl-hygiene.md` §Registration Sync Points)."
  - "`HostEffect::ClearPendingNotifications` is now observed by `QueueingEffectSink` consumers — the IO thread's Effect→MuxEvent router sees the marker and emits `MuxNotification::ClearPendingDesktopNotifications(pane_id)` (new variant) so the main thread clears any notifications it is currently holding for that pane. Semantic pin: `clear_pending_notifications_discards_preceding` in `response_poll/tests.rs` verifies that a `DesktopNotification` followed by `ClearPendingNotifications` in the same drain batch results in NO `MuxNotification::DesktopNotification` being emitted for that pane — the router collapses them per the contract documented at `oriterm_core/src/effect/families/host.rs:42-50`."
  - "Spec-conformance Section 10.2's `response_poll_emits_pty_write_on_fulfill` test (to be written in spec-conformance Section 10) passes without modification once this section's sink migration lands — this section provides the prerequisite; Section 10.2 writes the test. Cross-reference: `plans/spec-conformance/section-10-osc-suite.md:218-221` (Option A) and `plans/spec-conformance/section-10-osc-suite.md:14` (success criterion)."
  - "`oriterm_core/tests/alloc_regression.rs` stays green — the Effect→MuxEvent router uses a reusable `Vec<Effect>` scratch buffer on `PaneIoThread` (never `Vec::new()` per drain) and the router moves strings out of `HostEffect::TitleSet { value }` / `HostEffect::CwdSet { cwd }` / `HostEffect::DesktopNotification { title, body }` variants rather than cloning."
  - "`oriterm_core/tests/rss_regression.rs` stays green — `pending_responses: Vec<PendingResponse>` is capped (see 01.3) and the `response_wake` channel is bounded-size-1 (`crossbeam_channel::bounded(1)`) so idle-wake signalling never accumulates more than one pending wake."
  - "Cross-platform build green: `./build-all.sh` runs `cargo build --target x86_64-pc-windows-gnu --release` cleanly, covering the adopt-pane path (`domain/handoff/mod.rs`) which is exercised by Windows Default Terminal handoff; Linux/macOS local build also green. `./test-all.sh` green; `./clippy-all.sh` green (zero warnings under `deny(clippy::all)` + nursery)."
  - "`/tpr-review` passes on the final state (dual-source codex + gemini, all findings resolved); `/impl-hygiene-review last commit` passes after TPR is clean."
  - "Every subsection (01.1 – 01.4) transitions `status: not-started` → `status: complete`. `00-overview.md` and `index.md` are updated in the same commit that lands 01.4 to reflect plan-complete state (mission success). Spec-conformance Section 10's `depends_on` already lists `\"effect-cutover\"` (confirmed at `plans/spec-conformance/section-10-osc-suite.md:36`); this section's completion is what unblocks that dependency — no cross-plan edit is required here."
depends_on:
  - "plans/spec-conformance/section-03-effect-boundary-migration.md (COMPLETE — Effect/EffectSink types and LegacyEventSink adapter exist and are stable)"
third_party_review:
  status: none
  updated: null
sections:
  - id: "01.1"
    title: "Replace IoThreadEventProxy + LegacyEventSink with QueueingEffectSink in PaneIoThread (type-level migration only, no deletion yet)"
    status: not-started
  - id: "01.2"
    title: "Implement Effect→MuxEvent/MuxNotification routing in the IO thread drain loop"
    status: not-started
  - id: "01.3"
    title: "Activate PendingResponse polling with idle-wake channel"
    status: not-started
  - id: "01.4"
    title: "Delete IoThreadEventProxy, LegacyEventSink, Event::ClipboardLoad/ColorRequest, drain_notifications shim"
    status: not-started
  - id: "01.N"
    title: "Completion Checklist"
    status: not-started
---

# Section 01: Migrate Mux Consumer

## Mission-Criterion Connection

This section's mission criteria trace upward to `00-overview.md §Goal`:

- "Migrate `oriterm_mux` IO thread from `LegacyEventSink` → `QueueingEffectSink`" → delivered by 01.1 + 01.2 + 01.3.
- "Delete `LegacyEventSink`, `Event::ClipboardLoad`, `Event::ColorRequest`" → delivered by 01.4.
- "Remove the `drain_notifications()` thin shim" → delivered by 01.4.
- "All consumers process effects via `drain_into()` — no separate notification drain" → delivered by 01.2 + 01.4.

Downward: this section also unblocks `plans/spec-conformance/section-10-osc-suite.md` success criterion 7 (line 14) — Section 10.2's removal of the `#[allow(dead_code)]` gate on `register_host_request_response` is no-op once this section lands because this section wires the live call site AND removes the gate. Section 10.2 will only need to write the round-trip `#[test]` against the already-activated path.

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
  - `Effect::Host(HostEffect::ClipboardStore { selection, data })` → `MuxEvent::ClipboardStore { pane_id, clipboard_type: <map selection>, text: data }`. The `ClipboardSelection` → `ClipboardType` mapping is identical to `selection_to_legacy` at `oriterm_core/src/effect/sink/legacy/mod.rs:195-201`; move that helper to a new canonical home at `oriterm_mux/src/mux_event/clipboard.rs` (or inline into the router) — DO NOT duplicate it across legacy and new paths. After 01.4's deletion the helper lives in exactly one place.
  - `Effect::Host(HostEffect::DesktopNotification { source, title, body })` → `MuxEvent::DesktopNotification { pane_id, source, title, body }` (NEW variant — added in 01.2). Event pump forwards → `MuxNotification::DesktopNotification { pane_id, source, title, body }` (NEW variant).
  - `Effect::Host(HostEffect::ClearPendingNotifications)` → `MuxNotification::ClearPendingDesktopNotifications(pane_id)` (NEW variant). Router collapses preceding `DesktopNotification` effects for the same pane in the SAME drain batch (per the contract at `host.rs:42-50`).
  - `Effect::Host(HostEffect::VisualBell | HostEffect::AudioRequest(_) | HostEffect::PrintRequest(_))` → currently no `MuxEvent` variant. Log at `info!` and count in a `dropped_effects_debug` atomic (dev-build only) — do NOT silently drop. Filing a new `MuxEvent` variant for these is out of scope here; a `NOTE` item in 01.2 records the gap and the TPR reviewer can either accept the log-only behavior or require a variant. **This is a tracked gap, not deferral** — see 01.2's Cleanup block.
  - `Effect::HostRequest(req)` → call `self.register_host_request_response(req.clone())` to enqueue a `PendingResponse`. The `Effect::HostRequest` variant itself does NOT map to a `MuxEvent` — the request travels to the main thread via a SEPARATE path: a new `MuxEvent::HostRequest { pane_id, request }` variant (or reuse of `ClipboardLoad` / a color analog). This section chooses to add `MuxEvent::HostClipboardLoad { pane_id, selection, reply_token }` and `MuxEvent::HostColorQuery { pane_id, prefix, index, reply_token }` because they carry the `ResponseToken<T>` the consumer needs to fulfill. The old closure-based `MuxEvent::ClipboardLoad { .. formatter: Arc<dyn Fn(&str) -> String> .. }` is DELETED in 01.4 (the formatter was redundant with `format_clipboard_reply` at `oriterm_core/src/effect/families/host_request.rs:110` — the canonical home).
  - `Effect::Ui(UiEffect::CursorBlinkChanged { .. })` → no `MuxEvent`; fire wakeup only (parallel to legacy behavior at `event_proxy/mod.rs:150-152`).
  - `Effect::Ui(UiEffect::MouseCursorDirty)` → no `MuxEvent`; fire wakeup only.
  - `Effect::Presentation(p)` → log at `info!` level and do NOT queue (parallel to `LegacyEventSink` behavior at `legacy/mod.rs:112-117`; the atomic counter on `LegacyEventSink` is dropped — tests for it will be rewritten or removed in 01.4).
- Fulfilling a `ResponseToken` now ALSO signals an idle-wake channel so `PaneIoThread::run`'s `select!` unblocks. New type `ResponseWakeSignal(crossbeam_channel::Sender<()>)` attached to the token via a new field on `ResponseToken<T>` OR a wrapper type `WakingResponseToken<T>`. The choice between "mutate `ResponseToken`" and "wrap" is resolved in 01.3 by a small TDD — the wrapping approach is preferred because `ResponseToken<T>` lives in `oriterm_core` which is standalone (has no `crossbeam_channel` dependency today; adding one is out of the core crate's charter per `.claude/rules/oriterm_core.md` §Forbidden — "No IPC transport — lives in `oriterm_ipc`"; `crossbeam_channel` is IPC-adjacent enough to warrant keeping it at the mux boundary).

## Shared Invariants (apply across ALL subsections)

- **Crate boundary discipline** (`.claude/rules/crate-boundaries.md`): `oriterm_core` stays free of mux/IPC types. The idle-wake channel and its `Sender`/`Receiver` live in `oriterm_mux` only. The `Effect` enum stays in `oriterm_core`. Any new wrapper around `ResponseToken` that carries a channel lives in `oriterm_mux` (proposed home: `oriterm_mux/src/pane/io_thread/response_poll/mod.rs`).
- **No duplicated dispatch** (`.claude/rules/impl-hygiene.md` §LEAK:duplicated-dispatch): the Effect→MuxEvent match MUST live in exactly one function. Candidate home: `oriterm_mux/src/pane/io_thread/effect_router.rs` (new file). Do NOT inline a second match in `handle_bytes`, `handle_sync_timeout`, or `drain_commands`.
- **No SSOT drift** (`.claude/rules/impl-hygiene.md` §SSOT): reply formatting continues to go through `format_clipboard_reply` / `format_color_reply` at `oriterm_core/src/effect/families/host_request.rs:110,126`. `register_host_request_response` already uses them; the new router does NOT format replies — it only registers the response token. A broken SSOT would look like the router computing `let reply_bytes = format!("\x1b]52;..")` inline. Grep `grep -rn 'format!("\\\\x1b\\]52\\|format!("\\\\x1b\\]4'` across `oriterm_mux/` must return zero hits after this section lands.
- **Hot-path buffer discipline** (`.claude/rules/oriterm_core.md` §Performance Invariants): the `effects_buf: Vec<Effect>` scratch vector on `PaneIoThread` is reused via `clear()` + capacity retention after each drain. No `Vec::new()` per drain. No `shrink_to_fit()` during the hot path; a `maybe_shrink()` call at `PaneIoThread::run`'s bottom of the idle arm is acceptable if measurement warrants it (out of scope here).
- **File size** (`.claude/rules/code-hygiene.md` §File Size): the new `effect_router.rs` MUST stay under 500 lines. `response_poll.rs` is currently ~100 lines; adding idle-wake plumbing keeps it well under. `mod.rs` in `pane/io_thread/` is currently at 436 lines — adding the `effects_buf: Vec<Effect>` field + `effect_router` module declaration + two lines in the drain path should keep it under 500; if it crosses, extract `run()`'s `select!` body into a submodule (the two `select!` arms with their sync-deadline logic are prime split candidates).
- **TDD discipline** (`.claude/rules/tests.md` §TDD for Bugs): every subsection writes its failing test matrix FIRST, verifies RED, then implements to GREEN. No subsection is "complete" without RED→GREEN evidence in its validation checklist.
- **Cross-platform** (`.claude/rules/tests.md` §Cross-Platform Verification): both `domain/local.rs` (POSIX) and `domain/handoff/mod.rs` (Windows conhost handoff) are touched. `cargo build --target x86_64-pc-windows-gnu --release` must succeed locally before each subsection is marked complete. The `adopt_pane` path has no platform-specific `#[cfg]` in the sink wiring itself, but the broader handoff is Windows-only; the Linux build must still compile the cross-platform stubs.

---

## 01.1 Replace IoThreadEventProxy + LegacyEventSink with QueueingEffectSink in PaneIoThread

**Goal:** Type-level migration only — swap the generic parameter and construction. Effect routing and `register_host_request_response` wiring come in 01.2 and 01.3. At the end of 01.1, the IO thread compiles with `Term<QueueingEffectSink>`, existing tests may fail because effects are queued instead of forwarded — that is EXPECTED and the RED signal that 01.2 must GREEN.

**Files:**
- `oriterm_mux/src/pane/io_thread/mod.rs` — add `effects_buf: Vec<Effect>` field to `PaneIoThread` (initialized `Vec::new()` in `handle.rs::new_with_handle`); no type alias on `S` (keep generic so tests can still use `VoidEffectSink`).
- `oriterm_mux/src/pane/io_thread/handle.rs:128-160` (`new_with_handle`) — add `effects_buf: Vec::new()` to the struct literal on line 134-152.
- `oriterm_mux/src/domain/local.rs:139-145` — change `LegacyEventSink::new(IoThreadEventProxy::new(...))` to `QueueingEffectSink::new()`. Parameters currently passed to `IoThreadEventProxy::new` (`grid_dirty`, `pane_id`, `mux_tx`, `wakeup`) must now be threaded into `IoThreadConfig` so `PaneIoThread` itself holds them (for the 01.2 router). Add fields: `pane_id: PaneId`, `mux_tx: mpsc::Sender<MuxEvent>` to both `IoThreadConfig` and `PaneIoThread`. `grid_dirty` is already there; `wakeup` is already there.
- `oriterm_mux/src/domain/handoff/mod.rs:116-122` — same change as `local.rs`; parallel construction.
- `oriterm_mux/src/pane/io_thread/handle.rs:90-116` (`IoThreadConfig`) — add the new `pane_id` and `mux_tx` fields; update the struct literal in `new_with_handle` accordingly.
- `oriterm_mux/src/pane/io_thread/mod.rs` — add `pane_id: PaneId` and `mux_tx: mpsc::Sender<MuxEvent>` private fields to `PaneIoThread` (used by the router in 01.2).
- `oriterm_mux/src/pane/io_thread/tests.rs:60-84` (`make_sync_thread_with_term`) — update struct literal to include `effects_buf: Vec::new()`, `pane_id: PaneId(0)`, and a dummy `mux_tx` obtained from `mpsc::channel()`.
- `oriterm_mux/src/pane/io_thread/tests.rs:1837` (`make_sync_thread_generic`) — same additions; this helper is used by the `sync_timeout_emits_abort_effect` test at line 1927 which constructs a `QueueingEffectSink`-parameterized thread.
- `crates/oriterm_test_support/src/**` — spec_chain fixtures that construct `PaneIoThread`s for integration tests: audit for any constructor that needs the new fields. Use `grep -rn 'PaneIoThread\\s*{' crates/oriterm_test_support/` during implementation.

**Tests (written FIRST — per `.claude/rules/tests.md` §TDD for Bugs — VERIFIED RED before implementation):**

- [ ] `pane_io_thread_accepts_queueing_effect_sink` (new in `oriterm_mux/src/pane/io_thread/tests.rs`) — constructs a `PaneIoThread<QueueingEffectSink>` via a new helper `make_sync_thread_queueing()` and asserts it compiles and `t.run()` returns on `Shutdown` without panicking. RED because today the type alias on `local.rs` is `LegacyEventSink<IoThreadEventProxy>`; the helper cannot construct `PaneIoThread<QueueingEffectSink>` because the field types don't line up. GREEN once the field additions are in place.
- [ ] `io_thread_config_carries_pane_id_and_mux_tx` (new in `handle.rs`'s sibling `tests.rs` — create if missing) — `IoThreadConfig { ..., pane_id: PaneId(42), mux_tx: tx, .. }` constructs without error; `PaneIoThread::pane_id() == PaneId(42)` and `PaneIoThread::mux_tx_for_test()` returns the sender (test-only accessor gated `#[cfg(test)]`). RED before the fields are added.
- [ ] `effects_buf_is_reused_across_drains` (new) — push 10 `Effect::Pty(..)` effects, call `drain_into(&mut t.effects_buf)` twice (the second time after `effects_buf.clear()`), assert `t.effects_buf.capacity() >= 10` after the second clear. Pins the alloc-regression invariant that `effects_buf` is grow-only.
- [ ] **Negative pin** — `legacy_event_sink_construction_removed_from_local_domain` — a compile-time assertion via `trybuild` (if available; else runtime `grep`) that `oriterm_mux/src/domain/local.rs` no longer references `LegacyEventSink` nor `IoThreadEventProxy`. If `trybuild` is not set up, add an `#[test]` that reads the file via `include_str!("../../domain/local.rs")` and asserts `!source.contains("LegacyEventSink::new")` and `!source.contains("IoThreadEventProxy::new")`. GREEN when 01.1 lands.
- [ ] **Negative pin** — `legacy_event_sink_construction_removed_from_handoff` — same shape as above, for `oriterm_mux/src/domain/handoff/mod.rs`.
- [ ] **Semantic pin** — `queueing_sink_holds_effects_until_drained` — push a `Effect::Host(HostEffect::Bell)`, do NOT drain, assert the sink's internal queue length is 1 (via a `#[cfg(test)]` accessor `QueueingEffectSink::queue_len_for_test`). Do NOT rely on `drain_into` side effects to observe queue length; add the direct accessor (minimal exposure; `#[cfg(test)]` gated; lives at `oriterm_core/src/effect/sink/mod.rs`). Do NOT implement `Debug` for `QueueingEffectSink` in a way that calls `.lock()` on observation — `debug_struct("QueueingEffectSink").finish_non_exhaustive()` is sufficient (see existing `#[derive(Debug, Default)]` at `sink/mod.rs:61` — the derive uses `parking_lot::Mutex<T>: Debug` which already works).
- [ ] **Idempotency pin** — `multiple_drains_return_all_pushed_effects_in_order` — push 5 distinct effects, drain into `Vec<Effect>`, assert the order matches push order (per the ordering contract at `oriterm_core/src/effect/sink/mod.rs:30-38`). Then push 3 more, drain again, assert those 3 come out in order. No interleaving, no drops.

**Implementation:**

- [ ] Add `effects_buf: Vec<Effect>` field to `PaneIoThread` (initialized empty; no capacity hint — the first drain seeds capacity).
- [ ] Add `pane_id: PaneId` and `mux_tx: mpsc::Sender<MuxEvent>` fields to `PaneIoThread` and `IoThreadConfig`. Wire through `handle.rs::new_with_handle`.
- [ ] In `domain/local.rs:132-145`: DELETE the `IoThreadEventProxy::new(..)` call and the `LegacyEventSink::new(..)` wrap. Replace with `Term::new(.., QueueingEffectSink::new())`. Move `pane_id` and `mux_tx` into the subsequent `IoThreadConfig` literal.
- [ ] In `domain/handoff/mod.rs:109-122`: parallel change. The `mux_tx` is already captured via the `mux_tx: &mpsc::Sender<MuxEvent>` parameter — pass `mux_tx.clone()` into `IoThreadConfig`.
- [ ] Update `oriterm_mux/src/pane/io_thread/tests.rs::make_sync_thread_with_term` and `make_sync_thread_generic` to populate the new fields. Add helper `make_sync_thread_queueing()` that returns `PaneIoThread<QueueingEffectSink>`.
- [ ] Update `domain/handoff/tests.rs` to thread through any `pane_id` / `mux_tx` additions.
- [ ] **Do NOT yet remove** `IoThreadEventProxy`, `LegacyEventSink`, `Event::ClipboardLoad`, `Event::ColorRequest`, or the `drain_notifications` shim. Those deletions land in 01.4. At 01.1's end the two types are unused but still compile; `cargo check` will flag `dead_code` — that is OK because both types have `pub` visibility and are re-exported; the dead-code warnings appear only after 01.1's construction-site edits remove their last in-crate consumers. Add `#[allow(dead_code, reason = "removed in effect-cutover 01.4")]` at the type declaration level so clippy stays clean. 01.4's deletion removes both the allow attributes and the types themselves.

**Validation:**

- [ ] `cargo build -p oriterm_mux` green.
- [ ] `cargo build --target x86_64-pc-windows-gnu` green (cross-platform gate).
- [ ] `./test-all.sh` green EXCEPT for any tests in `oriterm_mux` that asserted on the presence of `MuxEvent::PaneTitleChanged` or similar mux events fired during parsing (those may RED because effects are now queued and not yet routed). Record which tests RED in this subsection's notes; they GREEN in 01.2.
- [ ] `./clippy-all.sh` green (zero new warnings).
- [ ] Seven TDD tests all pass (1 positive compile test, 2 negative pins, 3 invariant pins, 1 idempotency pin; the `pane_io_thread_accepts_queueing_effect_sink` plus the six above).
- [ ] Section 01.1 `status` → `complete` in frontmatter.

---

## 01.2 Implement Effect→MuxEvent/MuxNotification routing in the IO thread drain loop

**Goal:** Wire the drain loop. After this subsection, metadata events that previously reached the main thread synchronously via `LegacyEventSink::push` → `IoThreadEventProxy::send_event` → `mux_tx.send(..)` again reach the main thread — but now asynchronously via `drain_into` → `effect_router` → `mux_tx.send(..)`. This is where the existing test suite flips from RED (01.1's side-effect) back to GREEN.

**Files:**
- `oriterm_mux/src/pane/io_thread/effect_router.rs` (new — registered as `mod effect_router;` in `pane/io_thread/mod.rs` alongside `commands`, `event_proxy`, etc.).
- `oriterm_mux/src/pane/io_thread/effect_router/tests.rs` (new sibling per `.claude/rules/test-organization.md` §Sibling tests.rs Pattern; `mod effect_router;` becomes `pub(crate) mod effect_router;` with `#[cfg(test)] mod tests;` at the bottom of `effect_router/mod.rs` — see 01.2's BLOAT avoidance note below).
- `oriterm_mux/src/mux_event/mod.rs` — add new variants:
  - `MuxEvent::DesktopNotification { pane_id: PaneId, source: NotificationSource, title: String, body: String }`
  - `MuxEvent::HostClipboardLoad { pane_id: PaneId, selection: ClipboardSelection, clipboard_char: u8, terminator: String, reply: ResponseToken<String> }`
  - `MuxEvent::HostColorQuery { pane_id: PaneId, prefix: String, index: usize, terminator: String, reply: ResponseToken<Rgb> }`
  - Update the `impl fmt::Debug for MuxEvent` at line 94 exhaustively — compiler enforces.
- `oriterm_mux/src/mux_event/mod.rs` — add new `MuxNotification` variants:
  - `MuxNotification::DesktopNotification { pane_id, source, title, body }`
  - `MuxNotification::ClearPendingDesktopNotifications(PaneId)`
  - Update `impl fmt::Debug for MuxNotification` at line 327 exhaustively.
- `oriterm_mux/src/in_process/event_pump.rs:24-94` (`poll_events`) — add match arms for the new `MuxEvent` variants. Title/icon/cwd/output paths are unchanged from current behavior; new arms forward `DesktopNotification` and `HostClipboardLoad`/`HostColorQuery` into `self.notifications`. Arms for `HostClipboardLoad`/`HostColorQuery` go into a new `MuxNotification::HostClipboardLoad` / `MuxNotification::HostColorQuery` variant OR collapse into existing `ClipboardLoad` with an `enum ReplyMode { Closure(Arc<..>), Token(ResponseToken<String>) }` — **resolved below**: use distinct variants to avoid the enum-with-enum noise; the legacy `ClipboardLoad { formatter: Arc<dyn Fn> }` variant stays during 01.2 and is deleted in 01.4.
- `oriterm_mux/src/pane/io_thread/mod.rs` — in `handle_bytes` (after the `self.processor.advance` call, around line 270), add `self.drain_effects_into_mux_events()` (new method on `PaneIoThread` declared in `effect_router.rs`). In `drain_commands` (line 194-212), add the same call AFTER `poll_pending_responses()` — this ordering ensures a response fulfilled this tick enters the same drain cycle as its originating request-side effects. Also in `handle_sync_timeout` (line 291), add the call after `post_parse_housekeeping`.
- `oriterm_mux/src/mux_event/clipboard.rs` (new, small — ~40 lines) OR inline in `effect_router.rs`: `fn selection_to_legacy(s: ClipboardSelection) -> ClipboardType`. This is moved from `oriterm_core/src/effect/sink/legacy/mod.rs:195-201` (the legacy helper). The legacy copy is DELETED in 01.4 along with the rest of `LegacyEventSink`.
- `oriterm_mux/src/pane/io_thread/event_proxy/mod.rs` — DO NOT delete yet (lives until 01.4). But strip it of the metadata-forwarding arms since the router now owns that dispatch. However the simpler choice: leave `IoThreadEventProxy` alone (it's now dead because no `Term` wraps an `EventListener` anymore — the `IoThreadEventProxy`'s `send_event` is never called). Add `#[allow(dead_code, reason = "removed in effect-cutover 01.4")]` at the `IoThreadEventProxy` struct declaration. This avoids a large cross-cutting edit during 01.2 and concentrates the deletion in 01.4.

**Tests (written FIRST — VERIFIED RED before implementation):**

- [ ] **Matrix: Effect variant × routing target** — in `effect_router/tests.rs`, write one test per `HostEffect` / `PtyEffect` / `UiEffect` / `Presentation` / `HostRequest` variant. Each test pushes a single Effect into a `QueueingEffectSink`, runs `PaneIoThread::drain_effects_into_mux_events()`, and asserts the expected `MuxEvent` appears on a test-side `mpsc::Receiver<MuxEvent>`. Count assertion at the bottom iterates `HostEffect::ALL_VARIANTS_FOR_TEST` (add a `#[cfg(test)]` const slice of variant constructors — one entry per variant). Pins that no variant is silently dropped when a new one is added.
  - Variants required: `HostEffect::{Bell, VisualBell, DesktopNotification, TitleSet(Some), TitleSet(None), IconNameSet(Some), IconNameSet(None), CwdSet, AudioRequest, PrintRequest, ClipboardStore, ChildExit, CommandComplete, ClearPendingNotifications}`.
  - Variants required: `PtyEffect::{Write(Other), Write(DeviceStatus), Write(MouseReport)}` (per the `PtyWriteKind` enum at `oriterm_core/src/effect/families/pty.rs`).
  - Variants required: `UiEffect::{CursorBlinkChanged(true), CursorBlinkChanged(false), MouseCursorDirty}`.
  - Variants required: `PresentationEffect::{Begin, Commit, Abort(Timeout), Abort(BufferLimit)}` (match against the actual enum shape at `oriterm_core/src/effect/families/presentation.rs` — verify against code when writing).
  - Variants required: `HostRequest::{ClipboardLoad, ColorQuery}` — these route via `register_host_request_response` (tested in 01.3; in 01.2 the test asserts the request is also shoveled into `MuxEvent::HostClipboardLoad` / `MuxEvent::HostColorQuery` so the main thread can fulfill).
- [ ] **Negative pin** — `visual_bell_is_logged_not_dropped_silently` — push `HostEffect::VisualBell`, call `drain_effects_into_mux_events()`, assert `log::Level::Info` fired a record via a `testing_logger` crate (or local stub). Also asserts the `dropped_effects_debug` atomic counter (new `#[cfg(debug_assertions)]` field on `PaneIoThread`) incremented by exactly 1.
- [ ] **SSOT pin** — `title_set_none_produces_empty_title_via_single_helper` — push `HostEffect::TitleSet { value: None }`, assert `MuxEvent::PaneTitleChanged { title: String::new() }`. Then `grep -rn 'value.unwrap_or_default()' oriterm_mux/src/pane/io_thread/effect_router.rs` returns exactly one hit (for title) — NOT three (title, icon, elsewhere). The router MUST use a single inline `unwrap_or_default()` pattern at the one call site per variant; no helper function is needed because the pattern is 1 line. The SSOT pin here is against DUPLICATION, not ABSENCE.
- [ ] **Ordering pin** — `drain_preserves_push_order_end_to_end` — push `[Bell, Title("A"), CwdSet("/x"), Bell]` into the sink in that order. Drain. Assert the receiver sees `[PaneBell, PaneTitleChanged, PaneCwdChanged, PaneBell]` in EXACTLY that order.
- [ ] **Collapse pin** — `clear_pending_notifications_collapses_preceding` — push `[DesktopNotification(src=Osc9, "A"), DesktopNotification(src=Osc99, "B"), ClearPendingNotifications, DesktopNotification(src=Osc777, "C")]`. Drain. Assert the receiver sees `[ClearPendingDesktopNotifications, DesktopNotification(Osc777, "C")]` — the two preceding notifications in the SAME drain batch are discarded; the one AFTER the clear is preserved. This pins the contract at `host.rs:42-50`.
- [ ] **Cross-batch pin** — `clear_pending_notifications_does_not_retro_collapse_across_drains` — push `[DesktopNotification(Osc9, "A")]`, drain (receiver sees it). Then push `[ClearPendingNotifications]`, drain (receiver sees `[ClearPendingDesktopNotifications]`). Assert the main-thread-side `self.notifications` buffer still CONTAINS `DesktopNotification(Osc9, "A")` — the retro-collapse happens on the main thread when it processes `ClearPendingDesktopNotifications`, NOT in the IO thread's router. This pins that the router's collapse is intra-batch only (to keep the router pure); cross-batch retro-clearing is the main thread's concern.
- [ ] **Alloc regression** — `effect_router_drain_zero_alloc_steady_state` — warm up (two drains with 10 Effects each to grow `effects_buf` capacity). Then inside a `oriterm_core::tests::alloc_counter::measure!` block: push 10 effects, drain, assert `0` allocations inside the drain. Uses the existing alloc-regression infrastructure at `oriterm_core/tests/alloc_regression.rs` (confirmed present per CLAUDE.md §Performance Invariants).

**Implementation:**

- [ ] Create `oriterm_mux/src/pane/io_thread/effect_router.rs` with:
  - `impl PaneIoThread<QueueingEffectSink>` (concrete — not generic; the router only makes sense for queueing sinks)
    - `fn drain_effects_into_mux_events(&mut self)` — the ONLY dispatch function. It calls `self.terminal.effect_sink().drain_into(&mut self.effects_buf)`, then walks `effects_buf`, matching each variant and calling helper methods `route_host_effect`, `route_pty_effect`, `route_ui_effect`, `route_presentation_effect`, `route_host_request`. The main function is ≤20 lines; helpers own the per-variant match arms. `effects_buf.clear()` at the end.
    - **Important generic boundary**: some tests still construct `PaneIoThread<VoidEffectSink>` (e.g. `make_sync_thread`). The `VoidEffectSink` path never accumulates effects (`drain_into` is a no-op per `sink/mod.rs:90`), so those tests won't exercise the router. To avoid monomorphization errors, the router impl block is gated `impl PaneIoThread<QueueingEffectSink>` — a monomorphized method only exists on the queueing-sink version. Test helpers that need to exercise the router must use the queueing sink. Generic tests (e.g. `handle_bytes` unit tests) keep using `VoidEffectSink` and skip the router.
  - `selection_to_mux_clipboard_type(s: ClipboardSelection) -> ClipboardType` helper — a private function in `effect_router.rs` at first; if a second consumer emerges in 01.4, promote to `oriterm_mux/src/mux_event/clipboard.rs`. Do NOT preemptively create the `clipboard.rs` file — per `.claude/rules/impl-hygiene.md` §No Premature Abstraction, single-caller helpers live next to their caller.
- [ ] Add new `MuxEvent` variants listed in the Files block. Update the Debug impl. Update any exhaustive match in `in_process/event_pump.rs::poll_events` (compiler will force this).
- [ ] Add new `MuxNotification` variants listed in the Files block. Update the Debug impl at line 327. Update the event-pump forwarding arms.
- [ ] Add the three `self.drain_effects_into_mux_events()` call sites in `mod.rs` (`handle_bytes` end, `drain_commands` end after `poll_pending_responses`, `handle_sync_timeout` end after `post_parse_housekeeping`).
- [ ] Add `#[allow(dead_code, reason = "removed in effect-cutover 01.4")]` on `IoThreadEventProxy` (struct decl in `event_proxy/mod.rs:26`) and on `LegacyEventSink` (struct decl in `legacy/mod.rs:45`). Both are dead after 01.1's construction-site edits and stay dead through 01.4.

**Cleanup (woven hygiene items per `.claude/rules/impl-hygiene.md`):**

- [ ] **[BLOAT]** `oriterm_mux/src/pane/io_thread/mod.rs` — currently 436 lines (`wc -l` verified). Adding `effects_buf`, `pane_id`, `mux_tx` fields, the three `drain_effects_into_mux_events()` call sites, and the `mod effect_router;` declaration may push past 500. If so, extract the `select!` body of `run()` into a private `run_loop.rs` submodule. Do NOT leave the file >500 lines unsplit — that is an immediate BLOAT finding per `.claude/rules/code-hygiene.md` §File Size.
- [ ] **[NOTE]** `Effect::Host(HostEffect::{VisualBell, AudioRequest, PrintRequest})` have no `MuxEvent` variant. Router logs at `info!` + increments a debug-only drop counter. This is a tracked gap — out of scope for this plan, but MUST be filed as a BUG via `/add-bug` with severity `medium` before 01.2 is marked complete, per `CLAUDE.md §Bug Discipline`. The bug artifact is the tracked form of "known gap" — this is NOT deferral.
- [ ] **[DRIFT]** `MuxEventProxy` at `oriterm_mux/src/mux_event/mod.rs:137-257` still implements `EventListener` for a now-dead path. It is NOT used on the IO thread anymore (post 01.1 the IO thread's `Term` uses `QueueingEffectSink` directly). Audit: is `MuxEventProxy` used by any other code path? `grep -rn 'MuxEventProxy' oriterm_mux/` during 01.2. If unused → delete in 01.4 along with `IoThreadEventProxy`. If used by an out-of-scope consumer, file a follow-up bug and leave it in place with a deprecation note.
- [ ] **[LEAK:scattered-knowledge]** `ClipboardType` (legacy) vs `ClipboardSelection` (effect) is the same information in two types. The translation helper `selection_to_legacy` (currently at `legacy/mod.rs:195-201`) is the SSOT. After 01.4 the legacy copy is deleted; the helper's permanent home is `oriterm_mux/src/pane/io_thread/effect_router.rs` (private) UNTIL a second consumer appears. At that point it graduates to `oriterm_mux/src/mux_event/clipboard.rs` (public). This is a conscious SSOT choice: one helper, one home, zero duplication.

**Validation:**

- [ ] All 7 TDD tests transition RED → GREEN.
- [ ] Every existing `IoThreadEventProxy`-driven mux event (title, icon, cwd, command-complete, child-exit, bell, clipboard-store, pty-write) now fires via the router. Verified by re-running the pre-existing `oriterm_mux` test suite: `timeout 150 cargo test -p oriterm_mux` green.
- [ ] `timeout 150 cargo test -p oriterm_core --test alloc_regression` green.
- [ ] `./build-all.sh` (including Windows cross-compile) green.
- [ ] `./test-all.sh` green.
- [ ] `./clippy-all.sh` green.
- [ ] Section 01.2 `status` → `complete` in frontmatter.

---

## 01.3 Activate PendingResponse polling with idle-wake channel

**Goal:** Wire `register_host_request_response` into the router (remove the `#[allow(dead_code)]` gate), add the idle-wake channel so a fulfilled `ResponseToken` drives the PTY reply even when the IO thread is blocked in `select!` with no unrelated activity, and add a mux-side fulfillment helper that signals the wake.

**Files:**
- `oriterm_mux/src/pane/io_thread/response_poll.rs` — convert to directory module at `response_poll/mod.rs` with a sibling `response_poll/tests.rs` (per `.claude/rules/test-organization.md` §Sibling tests.rs Pattern). Files after conversion:
  - `oriterm_mux/src/pane/io_thread/response_poll/mod.rs` — contains `register_host_request_response`, `poll_pending_responses`, and a new `WakeableResponseToken<T>` wrapper type.
  - `oriterm_mux/src/pane/io_thread/response_poll/tests.rs` — sibling test module for `mod.rs`.
- `oriterm_mux/src/pane/io_thread/response_poll/mod.rs:33-36` — REMOVE `#[allow(dead_code, reason = "dormant during legacy phase; activates at effect-cutover")]`. Method becomes live because `effect_router.rs::route_host_request` calls it.
- `oriterm_mux/src/pane/io_thread/mod.rs` — add `response_wake_rx: Receiver<()>` field. Add it to BOTH `crossbeam_channel::select!` blocks at lines 141-158 (sync-deadline arm) and 161-176 (no-deadline arm). On wake, the loop continues to the next iteration which drains commands → calls `poll_pending_responses()` → calls `drain_effects_into_mux_events()`.
- `oriterm_mux/src/pane/io_thread/handle.rs:90-116` (`IoThreadConfig`) — add `response_wake_rx: Receiver<()>` AND expose the paired `response_wake_tx: Sender<()>` so the main thread can signal fulfillment. Since `PaneIoHandle` already holds channels for the main thread to drive the IO thread, add `response_wake_tx: Sender<()>` to `PaneIoHandle` alongside `cmd_tx` and `byte_tx`.
- `oriterm_mux/src/pane/io_thread/handle.rs:40-87` (`impl PaneIoHandle`) — add `pub fn fulfill_clipboard_load(&self, token: &ResponseToken<String>, text: String)` and `pub fn fulfill_color_query(&self, token: &ResponseToken<Rgb>, color: Rgb)` helpers that call `token.fulfill(value)` followed by `self.response_wake_tx.try_send(()).ok()`. Use `try_send` on a bounded-size-1 channel so a pending wake is never stacked — the IO thread only needs "wake at least once; multiple fulfills in the same idle period collapse to one wake."
- `oriterm_mux/src/pane/io_thread/effect_router.rs` — `route_host_request` calls `self.register_host_request_response(req.clone())` AND ALSO emits `MuxEvent::HostClipboardLoad` / `MuxEvent::HostColorQuery` with the token so the main thread can fulfill. The effect is NOT consumed — the `Effect::HostRequest(req)` is cloned because both sides (IO thread's pending-responses AND main thread's fulfillment queue) need access to the same `ResponseToken` (internal Arc-shared slot — `ResponseToken<T>` is `Clone` per `host_request.rs:53`).
- `oriterm/src/app/**` — main-thread consumer. Find the site where `MuxNotification::ClipboardLoad` is currently handled (today: receives the closure-based formatter and invokes it). After 01.4 that variant is deleted; 01.3 adds handling for `MuxNotification::HostClipboardLoad { pane_id, selection, clipboard_char, terminator, reply }`. The handler reads the clipboard, then calls `pane_handle.fulfill_clipboard_load(&reply, text)` on the paired `PaneIoHandle`. `grep -rn 'MuxNotification::ClipboardLoad' oriterm/` during implementation to locate the site.
- `oriterm_core/src/effect/response.rs` — NO changes. `PendingResponse` stays exactly as-is. The wake mechanism is pure `oriterm_mux`-side plumbing — `ResponseToken<T>` continues to be a plain data type per the design at `oriterm_core/src/effect/families/host_request.rs:11-17`.

**Tests (written FIRST — VERIFIED RED before implementation):**

- [ ] **Semantic pin — THE critical test for this section** — `response_poll_idle_wake_unblocks_select` in `response_poll/tests.rs`. Setup: spawn `PaneIoThread<QueueingEffectSink>::spawn()`; handler pushes `Effect::HostRequest(HostRequest::ClipboardLoad { ..., reply: token })` via `term.effect_sink().push(..)`; the spawn handle runs `drain_commands()` which calls `poll_pending_responses()` — token is registered but not fulfilled. No PTY bytes are sent. No commands are sent. On the main thread, call `handle.fulfill_clipboard_load(&token, "hello".to_string())`. Assert within 100ms (`std::thread::sleep(Duration::from_millis(100))` is the upper bound) the IO thread has drained and pushed `MuxEvent::PtyWrite { .. data: "\x1b]52;c;..." }` into the mux event receiver. Also assert that with the idle-wake DISABLED (via a test-only toggle or a separate test), the same scenario never produces `MuxEvent::PtyWrite` within 500ms — pinning that the wake is load-bearing, not incidental.
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
- [ ] In `oriterm/src/app/**` (wherever `MuxNotification::ClipboardLoad` is handled today — locate via grep during implementation): handle `MuxNotification::HostClipboardLoad` by reading clipboard text and calling `pane_handle.fulfill_clipboard_load(&reply_token, text)`. Similarly add handling for `MuxNotification::HostColorQuery` (queries the palette via the existing main-thread path and calls `fulfill_color_query`). The legacy closure-based `MuxNotification::ClipboardLoad` variant stays in place UNTIL 01.4 — both paths coexist during 01.3 to minimize blast radius.

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
- [ ] Section 01.3 `status` → `complete` in frontmatter.

---

## 01.4 Delete IoThreadEventProxy, LegacyEventSink, Event::ClipboardLoad/ColorRequest, drain_notifications shim

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
- `oriterm_core/src/term/shell_state/mod.rs:218` — delete `Term::drain_notifications()` (confirmed location per `plans/spec-conformance/00-overview.md:752`). Also inspect the function body at that line to see if it references any fields — delete those too if orphaned.
- `oriterm_mux/src/pane/io_thread/mod.rs:12` — remove `pub(crate) mod event_proxy;`.
- `oriterm_mux/src/pane/io_thread/handle.rs` — remove the `IoThreadEventProxy` reference from the `grid_dirty` field doc comment at line 99-100.
- `oriterm_mux/src/mux_event/mod.rs:84-92` — delete `MuxEvent::ClipboardLoad { .. formatter: Arc<dyn Fn(&str) -> String + Send + Sync> }` (the old closure-based variant). Its replacement (`HostClipboardLoad` with `ResponseToken`) landed in 01.2.
- `oriterm_mux/src/mux_event/mod.rs:121-126` (approx, update after 01.2's edits) — delete the Debug arm for `ClipboardLoad`.
- `oriterm_mux/src/mux_event/mod.rs:311-319` (approx) — delete `MuxNotification::ClipboardLoad { .. formatter: Arc<dyn Fn> }` and its Debug arm.
- `oriterm_mux/src/in_process/event_pump.rs:82-93` — delete the `MuxEvent::ClipboardLoad` match arm.
- `oriterm_mux/src/mux_event/mod.rs:137-257` (`MuxEventProxy`) — if `MuxEventProxy` is confirmed unused in 01.2's [DRIFT] audit, delete the entire `MuxEventProxy` struct + impl block here. If it is still used by an out-of-scope path, leave it (a separate bug will track its removal).
- `oriterm/src/app/**` — remove any lingering `MuxNotification::ClipboardLoad` handler (closure-based); only the `HostClipboardLoad` / `HostColorQuery` paths remain.
- `oriterm_core/src/term/renderable/**` or wherever the `drain_notifications()` shim's caller lives — find via grep during implementation; delete callers.

**Tests (written FIRST — VERIFIED RED before implementation — "red" here means the grep-based guards fail because the names still exist):**

- [ ] **Deletion pin** — `no_legacy_event_sink_references` — reads the output of `grep -rn 'LegacyEventSink' oriterm_core/ oriterm_mux/ oriterm/ crates/` and asserts zero hits (excluding `.git/`, `target/`, and this plan doc). Expressed as a `#[test]` that `std::process::Command::new("grep")`s the workspace — gracefully skips on Windows (`reseq`-style skip protocol per `.claude/rules/tests.md` §Graceful Skip Protocol). On Linux/macOS this MUST fail before 01.4's deletions, pass after.
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
- [ ] `InProcessMux::drain_notifications` (at `in_process/event_pump.rs:102`) — this is DIFFERENT from the `Term::drain_notifications` shim and STAYS. Verify via re-read that nothing in 01.4's deletion set touches it.

**Cleanup (hygiene items):**

- [ ] **[WASTE]** `oriterm_core/src/effect/sink/legacy/tests.rs` had `presentation_effect_count` tests (e.g. at `legacy/tests.rs:259` based on the grep hit at `legacy/tests.rs:225,231,259,268`). Those tests die with the file. Audit: are any of those assertions portable to `QueueingEffectSink` tests? (E.g. "Presentation effects are logged, not queued" was a LegacyEventSink behavior; on QueueingEffectSink, Presentation effects ARE queued per the router's decision in 01.2.) If any LegacyEventSink test captured a semantic that survives to `QueueingEffectSink`, port it BEFORE deleting the file. If not, delete cleanly.
- [ ] **[LEAK:scattered-knowledge]** `selection_to_legacy` at `legacy/mod.rs:195-201` — the `ClipboardSelection` → `ClipboardType` helper. Verify the canonical home (promoted in 01.2 into `effect_router.rs` or `mux_event/clipboard.rs`) survives this deletion and no other call site re-implemented the same mapping. `grep -rn 'ClipboardSelection.*Clipboard.*ClipboardType::Clipboard\\|ClipboardSelection.*Primary.*ClipboardType::Selection' oriterm_mux/ oriterm_core/` must return exactly ONE hit after this subsection lands.
- [ ] **[DRIFT]** `Event` enum's `ChildExit` variant routes to `MuxEvent::PaneExited`. Still the only path? Verify: `grep -rn 'Event::ChildExit' oriterm_mux/ oriterm_core/ oriterm/` — exactly one consumer, which is `IoThreadEventProxy::send_event::Event::ChildExit` at `event_proxy/mod.rs:143-148`. After event_proxy is deleted, `ChildExit` has NO consumer but might still be emitted by the VTE handler. **Investigation step**: is `Event::ChildExit` ever pushed? Grep `grep -rn 'Event::ChildExit' oriterm_core/src/` — if only the variant declaration hits, `ChildExit` is dead and should be deleted along with `ClipboardLoad` / `ColorRequest`. If the VTE handler pushes it, the router must route it to `MuxEvent::PaneExited` (this MIGHT already be handled via `Effect::Host(HostEffect::ChildExit)` in 01.2's router). Resolve before 01.4 is marked complete — do NOT leave a dangling Event variant.

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
- [ ] Section 01.4 `status` → `complete` in frontmatter.

---

## 01.N Completion Checklist

### TDD Discipline (MUST be FIRST — per `.claude/rules/tests.md` §TDD for Bugs)

- [ ] 01.1's 7 TDD tests written and VERIFIED RED before any implementation.
- [ ] 01.2's 7 TDD tests written and VERIFIED RED before any implementation.
- [ ] 01.3's 8 TDD tests written and VERIFIED RED before any implementation.
- [ ] 01.4's 7 deletion/regression pins written and VERIFIED RED (pre-deletion state) before any deletions land.

### Ordering gate (crate dependency direction per `.claude/rules/crate-boundaries.md`)

- [ ] Changes land in this order: `oriterm_core` (event variant deletions, Term shim deletion in 01.4 — atomic) → `oriterm_mux` (router, response_poll module conversion, new MuxEvent variants — 01.1 through 01.3) → `oriterm` (main-thread consumer updates — 01.3's fulfill-site wiring). The file deletions in 01.4 span `oriterm_core` (legacy sink) and `oriterm_mux` (event_proxy) but must land in a single commit because the compiler refuses any intermediate state — `LegacyEventSink` referenced in `event_proxy/mod.rs` and both deleted together.

### Matrix coverage

- [ ] **Matrix dimensions**: Effect variant × routing target × drain entry point (handle_bytes, drain_commands, handle_sync_timeout) × sink implementation (VoidEffectSink for generic unit tests, QueueingEffectSink for integration tests).
- [ ] **Semantic pins** (at least one per subsection; the single-most-critical pin of the section is `response_poll_idle_wake_unblocks_select`):
  - [ ] `pane_io_thread_accepts_queueing_effect_sink` (01.1)
  - [ ] `effect_router_drain_zero_alloc_steady_state` (01.2)
  - [ ] `response_poll_idle_wake_unblocks_select` (01.3) — THE critical pin
  - [ ] `response_poll_emits_pty_write_on_fulfill` (01.3)
  - [ ] `effect_cutover_final_state_full_run` (01.4)
- [ ] **Negative pins** (every positive test has a paired negative):
  - [ ] `legacy_event_sink_construction_removed_from_local_domain` (01.1)
  - [ ] `legacy_event_sink_construction_removed_from_handoff` (01.1)
  - [ ] `visual_bell_is_logged_not_dropped_silently` (01.2)
  - [ ] `clear_pending_notifications_collapses_preceding` (01.2) — pins the intra-batch collapse
  - [ ] `clear_pending_notifications_does_not_retro_collapse_across_drains` (01.2) — pins the cross-batch boundary
  - [ ] `response_poll_token_requires_fulfillment` (01.3)
  - [ ] `dead_code_attribute_is_removed` (01.3)
  - [ ] `no_legacy_event_sink_references` (01.4)
  - [ ] `no_io_thread_event_proxy_references` (01.4)
  - [ ] `no_event_clipboardload_or_colorrequest_variants` (01.4)
  - [ ] `no_drain_notifications_shim` (01.4)
  - [ ] `no_desktop_notification_record_references` (01.4)
  - [ ] `event_enum_variants_exhaustive_list` (01.4)
- [ ] **Cross-pattern matrix**: the router handles BOTH flow patterns — (a) synchronous push during VTE parsing (`handle_bytes` → drain at end), (b) command-driven effect emission (`drain_commands` → `poll_pending_responses` → drain), (c) sync-timeout replay (`handle_sync_timeout` → `post_parse_housekeeping` → drain). Each of the three entry points is tested for at least one `HostEffect`, one `PtyEffect`, and one `HostRequest`.

### Rules weaving (per `.claude/rules/impl-hygiene.md` + `.claude/rules/code-hygiene.md` + `.claude/rules/crate-boundaries.md` + `.claude/rules/oriterm_core.md` + `.claude/rules/oriterm_mux.md` + `.claude/rules/tests.md`)

- [ ] **No SSOT drift**: `Effect::HostRequest` → PTY reply formatting goes through `format_clipboard_reply` / `format_color_reply` at `oriterm_core/src/effect/families/host_request.rs:110,126`. The router does NOT format replies. Verified by: `grep -rn 'format!("\\\\x1b\\]52\\|format!("\\\\x1b\\]4\\|format!("\\\\x1b\\]10\\|format!("\\\\x1b\\]11\\|format!("\\\\x1b\\]12'` in `oriterm_mux/` returns zero hits after this section lands. The reply formatters are called from exactly one place: `register_host_request_response` at `response_poll/mod.rs`.
- [ ] **No duplicated dispatch** (`.claude/rules/impl-hygiene.md` §LEAK:duplicated-dispatch): the Effect→MuxEvent match lives ONLY in `effect_router.rs`. No parallel match in `handle_bytes`, `handle_sync_timeout`, `drain_commands`, or `handle_command`. Verified by: `grep -n 'Effect::Host\\|Effect::Pty\\|Effect::HostRequest\\|Effect::Ui\\|Effect::Presentation' oriterm_mux/src/pane/io_thread/` shows match arms ONLY inside `effect_router.rs` (and its `tests.rs`).
- [ ] **No registration sync drift**: adding `MuxEvent::DesktopNotification`, `MuxEvent::HostClipboardLoad`, `MuxEvent::HostColorQuery` requires synchronized updates at — (1) `oriterm_mux/src/mux_event/mod.rs::MuxEvent` enum, (2) `impl fmt::Debug for MuxEvent` at line 94, (3) `oriterm_mux/src/in_process/event_pump.rs::poll_events` exhaustive match, (4) `oriterm_mux/src/mux_event/tests.rs` where Debug output is pinned. All 4 updated atomically. Similarly for the new `MuxNotification::DesktopNotification` and `MuxNotification::ClearPendingDesktopNotifications` — (1) enum, (2) Debug impl at line 327, (3) forwarding site in event_pump, (4) any downstream consumer in `oriterm/src/app/**` that pattern-matches `MuxNotification`.
- [ ] **No LEAK:scattered-knowledge**: `ClipboardSelection` → `ClipboardType` translation lives at exactly one site post-01.4 (the legacy helper at `legacy/mod.rs:195-201` is deleted). `selection_to_mux_clipboard_type` in `effect_router.rs` (private) is the single canonical home.
- [ ] **No file size violations** (`.claude/rules/code-hygiene.md` §File Size, 500-line limit, proactive split at 450):
  - [ ] `oriterm_mux/src/pane/io_thread/mod.rs` — if crosses 500 lines after 01.2's additions, split `run()`'s `select!` body into `run_loop.rs`.
  - [ ] `oriterm_mux/src/pane/io_thread/effect_router.rs` — new file; stay under 500 lines by extracting per-variant helpers into `effect_router/host.rs`, `effect_router/pty.rs`, etc. ONLY if the single file crosses 450.
  - [ ] `oriterm_mux/src/mux_event/mod.rs` — currently ~356 lines (`wc -l` verified). Adding 3 new MuxEvent variants + 2 new MuxNotification variants adds ~80 lines. Approaches 450 — proactively split `MuxNotification` into `oriterm_mux/src/mux_event/notification.rs` if it crosses 450.
- [ ] **Cross-platform** (`.claude/rules/tests.md` §Cross-Platform Verification): `cargo build --target x86_64-pc-windows-gnu` green at EVERY subsection boundary. The `adopt_pane` path in `domain/handoff/mod.rs` is Windows-specific-flavored but compiles on all platforms. The idle-wake channel uses `crossbeam_channel` which is cross-platform.
- [ ] **Alloc regression**: `oriterm_core/tests/alloc_regression.rs` green at EVERY subsection boundary. The `effects_buf: Vec<Effect>` scratch vector is grow-only; `drain_into` reuses its capacity per the existing contract at `sink/mod.rs:77-80`.
- [ ] **RSS regression**: `oriterm_core/tests/rss_regression.rs` green. `pending_responses: Vec<PendingResponse>` bounded — 01.3 adds a `debug_assert!(self.pending_responses.len() < MAX_PENDING_RESPONSES)` with `MAX_PENDING_RESPONSES = 64` to pin that unfulfilled tokens don't accumulate unboundedly; if a production scenario genuinely exceeds this, it is a bug filed via `/add-bug`.
- [ ] **Crate boundary discipline** (`.claude/rules/crate-boundaries.md`): `crossbeam_channel` stays out of `oriterm_core`. The wake channel lives in `oriterm_mux` only. `ResponseToken<T>` in `oriterm_core` stays a plain data type.

### Catalog + cross-section updates

- [ ] Spec-conformance Section 10.2's success criterion dependency is now satisfied: `plans/spec-conformance/section-10-osc-suite.md:14` references the dead-code gate removal. After 01.3 lands, the gate is gone and Section 10.2 can write `response_poll_emits_pty_write_on_fulfill` as a pure verification test against already-working infrastructure. No cross-plan file edit is required here (Section 10.2's `depends_on` already lists `"effect-cutover"`).
- [ ] No other plans depend on this plan's deliverables (verified by `grep -rn 'effect-cutover' plans/` — only spec-conformance Section 10 is the consumer).
- [ ] The old `plans/effect-cutover/00-overview.md` Goal bullets are ALL ticked (each bullet maps to a success criterion here):
  - [ ] "Migrate `oriterm_mux` IO thread from `LegacyEventSink` → `QueueingEffectSink`" ↔ 01.1+01.2+01.3.
  - [ ] "Migrate `oriterm` application from `Event`-based dispatch → `Effect`-based dispatch" ↔ 01.3's main-thread wiring (partial — clipboard/color only, since those were the only closure-based `Event` variants).
  - [ ] "Delete `LegacyEventSink`, `Event::ClipboardLoad`, `Event::ColorRequest`" ↔ 01.4.
  - [ ] "Remove the `drain_notifications()` thin shim" ↔ 01.4.
  - [ ] "All consumers process effects via `drain_into()` — no separate notification drain" ↔ verified by 01.4's `no_drain_notifications_shim` pin.

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
