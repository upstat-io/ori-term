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

- Migrate `oriterm_mux` IO thread from `LegacyEventSink` → `QueueingEffectSink`
- Migrate `oriterm` application from `Event`-based dispatch → `Effect`-based dispatch (for the closure-based variants — `ClipboardLoad`, `ColorRequest`)
- Delete `LegacyEventSink`, `Event::ClipboardLoad`, `Event::ColorRequest`, `IoThreadEventProxy`, `DesktopNotificationRecord`
- Remove the `Term::drain_notifications()` thin shim
- All consumers process effects via `drain_into()` — no separate notification drain
- Add an idle-wake channel so a fulfilled `ResponseToken` unblocks the IO thread's `crossbeam_channel::select!` without requiring unrelated PTY or command activity (fixes TPR-03-001-codex)

## Mission Success Criteria

Every criterion here is concrete, testable, and traces to at least one section:

1. **Type-level migration complete** — `PaneIoThread<QueueingEffectSink>` is the only instantiation in production code. `LegacyEventSink::new(IoThreadEventProxy::new(..))` is not referenced anywhere in `oriterm_mux/src/`. Delivered by Section 01 (01.1).
2. **Effect→MuxEvent routing lives in one place** — the Effect→MuxEvent match is in exactly one function (`PaneIoThread::drain_effects_into_mux_events` in `effect_router.rs`). `grep -rn 'Effect::Host\|Effect::Pty\|Effect::HostRequest\|Effect::Ui\|Effect::Presentation' oriterm_mux/src/pane/io_thread/` shows match arms only in `effect_router.rs` (and its `tests.rs`). Delivered by Section 01 (01.2).
3. **`HostRequest` round-trip is live** — the `#[allow(dead_code, reason = "dormant during legacy phase; activates at effect-cutover")]` gate on `register_host_request_response` is deleted, and a fulfilled `ResponseToken` produces a `PtyEffect::Write` reply within one `select!` iteration. Pinned by semantic test `response_poll_idle_wake_unblocks_select`. Delivered by Section 01 (01.3).
4. **Legacy scaffolding deleted** — `grep -rn 'LegacyEventSink\|IoThreadEventProxy\|DesktopNotificationRecord\|Event::ClipboardLoad\|Event::ColorRequest'` workspace-wide returns zero hits; `Term::drain_notifications()` is removed from `oriterm_core/src/term/shell_state/mod.rs`. Delivered by Section 01 (01.4).
5. **Cross-platform build green** — `./build-all.sh` runs `cargo build --target x86_64-pc-windows-gnu --release` cleanly at every subsection boundary; Linux + macOS builds also green. Delivered across 01.1–01.4.
6. **Performance invariants unchanged** — `oriterm_core/tests/alloc_regression.rs` and `oriterm_core/tests/rss_regression.rs` stay green; the `effects_buf: Vec<Effect>` scratch vector is grow-only; the wake channel is bounded-size-1. Delivered across 01.1–01.4.
7. **Spec-conformance Section 10.2 unblocked** — Section 10.2 can write `response_poll_emits_pty_write_on_fulfill` against already-live infrastructure after this plan lands. Delivered by Section 01 (01.3).

## Dependency

Requires spec-conformance Section 03 complete (Effect types, EffectSink, LegacyEventSink exist and are stable — verified: `plans/spec-conformance/section-03-effect-boundary-migration.md` status: complete).

Unblocks spec-conformance Section 10.2 (`plans/spec-conformance/section-10-osc-suite.md:14` success criterion — the OSC 52 `ResponseToken` round-trip activation). Section 10's `depends_on` already lists `"effect-cutover"` (`section-10-osc-suite.md:36`).

## Section Dependency Graph

Single-section plan: Section 01 is self-contained. Within the section, 01.1 → 01.2 → 01.3 → 01.4 → 01.N (completion). Subsections cannot reorder because:

- 01.2 routes `HostRequest` into `MuxEvent` variants added by 01.2 — those variants don't exist before 01.2.
- 01.3 adds the idle-wake channel that 01.4's `effect_cutover_final_state_full_run` integration test depends on.
- 01.4 deletes types that 01.1–01.3 still reference (with `#[allow(dead_code)]` attributes).
