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
- [ ] Test: clipboard load round-trip produces correct PTY reply

## 01.4 Delete legacy infrastructure

- [ ] Delete `IoThreadEventProxy`
- [ ] Delete `LegacyEventSink` (after all consumers migrated)
- [ ] Delete deprecated `Event::ClipboardLoad` and `Event::ColorRequest` variants
- [ ] Delete `drain_notifications()` shim
