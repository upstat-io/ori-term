---
section: "01"
title: "Migrate Mux Consumer from LegacyEventSink to QueueingEffectSink"
status: not-started
reviewed: false
goal: "Replace LegacyEventSink<IoThreadEventProxy> with QueueingEffectSink in the mux IO thread, routing effects directly to the mux event bus."
sections:
  - id: "01.1"
    title: "Replace IoThreadEventProxy with QueueingEffectSink in PaneIoThread"
    status: not-started
  - id: "01.2"
    title: "Implement Effect→MuxEvent routing in the IO thread drain loop"
    status: not-started
  - id: "01.3"
    title: "Activate PendingResponse polling for clipboard/color reply-return"
    status: not-started
  - id: "01.4"
    title: "Delete IoThreadEventProxy and LegacyEventSink"
    status: not-started
third_party_review:
  status: none
  updated: null
---

# Section 01: Migrate Mux Consumer

## 01.1 Replace IoThreadEventProxy with QueueingEffectSink in PaneIoThread

- [ ] Change `PaneIoThread<LegacyEventSink<IoThreadEventProxy>>` to `PaneIoThread<QueueingEffectSink>`
- [ ] Update `domain/local.rs` and `domain/handoff/mod.rs` construction sites
- [ ] Add an Effect→MuxEvent routing function that maps each Effect variant to the corresponding MuxEvent

## 01.2 Implement Effect→MuxEvent routing in the IO thread drain loop

- [ ] In the IO thread's main loop, call `effect_sink.drain_into(&mut effects)` after VTE processing
- [ ] Route each Effect to the appropriate MuxEvent via the routing function
- [ ] DesktopNotification effects are processed inline (no separate drain)

## 01.3 Activate PendingResponse polling

- [ ] Wire up `pending_responses: Vec<PendingResponse>` in PaneIoThread
- [ ] Poll fulfilled tokens in `drain_commands()` / `handle_command()`
- [ ] **Add IO thread wake mechanism for token fulfillment**: `ResponseToken::fulfill()` currently only stores into the slot — it does not wake the IO thread. When the IO thread is idle (blocking in `crossbeam_channel::select!` on `cmd_rx`/`byte_rx`), a fulfilled token will not be polled until unrelated activity (PTY output or command) wakes the thread. Add a wake channel or `Sender::send()` notification from `fulfill()` into the IO thread's `select!` set so that fulfilled tokens trigger an immediate poll. Without this, idle OSC 52 / color query replies have unbounded latency. (Surfaced by TPR-03-001-codex during Section 03 close-out.)
- [ ] Test: clipboard load round-trip produces correct PTY reply
- [ ] Test: idle-query latency — fulfill a token with no other channel activity and verify the reply is written to PTY within one poll cycle (not deferred until unrelated wakeup)

## 01.4 Delete legacy infrastructure

- [ ] Delete `IoThreadEventProxy`
- [ ] Delete `LegacyEventSink` (after all consumers migrated)
- [ ] Delete deprecated `Event::ClipboardLoad` and `Event::ColorRequest` variants
- [ ] Delete `drain_notifications()` shim
