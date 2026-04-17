---
plan: effect-cutover
title: "Effect Cutover — Migrate Legacy Event Consumers to Effect"
status: not-started
---

# Effect Cutover

## Context

Section 03 of the spec-conformance plan introduced the `Effect`/`EffectSink` system and migrated all VTE handler emission sites to emit `Effect` instead of `Event`. The `LegacyEventSink` adapter bridges `Effect` → `Event` for backward compatibility. The deprecated `Event::ClipboardLoad` and `Event::ColorRequest` variants carry closures created by the adapter.

This plan completes the migration by moving each legacy consumer from `Event` subscriptions to direct `Effect` subscriptions, then deleting the adapter and deprecated variants. Once this plan lands, spec-conformance Section 10.2 can activate the dormant `response_poll` path by removing the `#[allow(dead_code)]` gate on `PaneIoThread::register_host_request_response` — that gate exists today solely because the IO thread uses `LegacyEventSink`, whose `drain_into()` is a no-op.

## Goal

- Migrate `oriterm_mux` IO thread from `LegacyEventSink` → `QueueingEffectSink` atomically with the Effect→MuxEvent/MuxNotification router (single commit — no intermediate state silently drops effects).
- Migrate `oriterm` application from `Event`-based dispatch → `Effect`-based dispatch (for the closure-based variants — `ClipboardLoad`, `ColorRequest`).
- Delete `LegacyEventSink`, `Event::ClipboardLoad`, `Event::ColorRequest`, `IoThreadEventProxy`, `DesktopNotificationRecord`, and `MuxEventProxy` (subject to audit).
- Remove the `Term::drain_notifications()` thin shim (preserve `InProcessMux::drain_notifications` — distinct method).
- All consumers process effects via `drain_into()` — no separate notification drain.
- Add an idle-wake channel so a fulfilled `ResponseToken` unblocks the IO thread's `crossbeam_channel::select!` without requiring unrelated PTY or command activity (fixes TPR-03-001-codex).
- Resolve the daemon-mode `HostRequest` IPC incompatibility by EITHER designing the request-ID + reply PDU in scope (Path A), OR filing a tracked `/add-bug` artifact with a cross-link (Path B), OR spinning out a new plan (Path C). Section 01 §01.4 executes exactly one path.

## Mission Success Criteria

Every criterion here is concrete, testable, and traces to at least one section:

1. **Atomic type-level migration + router activation** — `PaneIoThread<QueueingEffectSink>` is the only instantiation in production code AND the Effect→MuxEvent/MuxNotification router is active in the SAME commit. `LegacyEventSink::new(IoThreadEventProxy::new(..))` is not referenced anywhere in `oriterm_mux/src/`, and every effect that previously reached the main thread via `LegacyEventSink::push` now reaches it via the router. There is no intermediate landable state where effects queue without being drained. Delivered by Section 01 (01.1).
2. **Effect→MuxEvent routing lives in one place** — the Effect→MuxEvent match is in exactly one function (`PaneIoThread::drain_effects_into_mux_events` in `effect_router/mod.rs`). `grep -rn 'Effect::Host\|Effect::Pty\|Effect::HostRequest\|Effect::Ui\|Effect::Presentation' oriterm_mux/src/pane/io_thread/` shows match arms only in `effect_router/mod.rs` (and its `tests.rs`). Delivered by Section 01 (01.1).
3. **`HostRequest` round-trip is live** — the `#[allow(dead_code, reason = "dormant during legacy phase; activates at effect-cutover")]` gate on `register_host_request_response` is deleted, and a fulfilled `ResponseToken` produces a `PtyEffect::Write` reply within one `select!` iteration. Pinned by semantic test `response_poll_idle_wake_unblocks_select`. Delivered by Section 01 (01.2).
4. **Legacy scaffolding deleted** — `grep -rn 'LegacyEventSink\|IoThreadEventProxy\|DesktopNotificationRecord\|Event::ClipboardLoad\|Event::ColorRequest'` workspace-wide returns zero hits; `Term::drain_notifications()` is removed from `oriterm_core/src/term/shell_state/mod.rs`. Delivered by Section 01 (01.3).
5. **Cross-platform build green** — `./build-all.sh` runs `cargo build --target x86_64-pc-windows-gnu --release` cleanly at every subsection boundary; Linux + macOS builds also green. Delivered across 01.1–01.4.
6. **Performance invariants unchanged** — `oriterm_core/tests/alloc_regression.rs` and `oriterm_core/tests/rss_regression.rs` stay green; the `effects_buf: Vec<Effect>` scratch vector is grow-only (and drained between `MAX_PARSE_CHUNK` slices to bound accumulation); the wake channel is bounded-size-1. Delivered across 01.1–01.4.
7. **Spec-conformance Section 10.2 unblocked** — Section 10.2 can write `response_poll_emits_pty_write_on_fulfill` against already-live infrastructure after this plan lands. Delivered by Section 01 (01.2).
8. **Daemon-mode IPC for `HostRequest` is either in-scope or explicitly tracked** — `ResponseToken<T>` is process-local (`Arc<Mutex<Option<T>>>` at `oriterm_core/src/effect/families/host_request.rs:54-56`); `oriterm_mux/src/protocol/messages.rs:396-401` serializes no reply token. Section 01 (01.4) resolves this by ONE of: (a) adding a request-ID + reply-PDU design in scope, (b) filing a tracked `/add-bug` artifact with `<!-- blocked-by: -->` cross-link, or (c) spinning out a dedicated plan. No silent deferral; `00-overview.md` Goal list matches the chosen resolution. Delivered by Section 01 (01.4).

## Dependency

Requires spec-conformance Section 03 complete (Effect types, EffectSink, LegacyEventSink exist and are stable — verified: `plans/spec-conformance/section-03-effect-boundary-migration.md` status: complete).

Unblocks spec-conformance Section 10.2 (`plans/spec-conformance/section-10-osc-suite.md:14` success criterion — the OSC 52 `ResponseToken` round-trip activation). Section 10's `depends_on` already lists `"effect-cutover"` (`section-10-osc-suite.md:36`).

## Section Dependency Graph

Single-section plan: Section 01 is self-contained. Within the section, 01.1 → 01.2 → 01.3 → 01.4 → 01.N (completion). Subsections cannot reorder because:

- 01.1 is **atomic** (sink swap + router activation + new MuxEvent/MuxNotification variants) — per reviewer consensus, splitting swap and router into separate commits silently drops effects in production. 01.1 must land as a single commit.
- 01.2 adds the idle-wake channel that 01.3's `effect_cutover_final_state_full_run` integration test depends on.
- 01.3 deletes types that 01.1 + 01.2 still reference (with `#[allow(dead_code, reason = ...)]` attributes).
- 01.4 is the daemon-mode IPC audit (blind-spot §2). It runs LAST because the chosen resolution (in-scope design / file bug / spin out plan) depends on a stable post-deletion state — the audit over code that still contains `LegacyEventSink` would be noisy.
