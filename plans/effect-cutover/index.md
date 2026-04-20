---
reroute: false
name: "Effect Cutover"
full_name: "Effect Cutover — Migrate Legacy Event Consumers to Effect"
status: complete
order: 999
---

# Effect Cutover

Migrate all remaining `Event`-based consumers to subscribe to `Effect` directly, then delete the deprecated `Event::ClipboardLoad` and `Event::ColorRequest` variants and the `LegacyEventSink` adapter.

## Resume Point

**Plan complete.** All four subsections (§01.1, §01.2, §01.3, §01.4) and the §01.N completion checklist are done. The effect-cutover mission is accomplished: `PaneIoThread<QueueingEffectSink>` is the sole IO-thread sink, the Effect→MuxEvent router is live in one canonical match, legacy scaffolding (`LegacyEventSink`, `IoThreadEventProxy`, `DesktopNotificationRecord`, `Event::ClipboardLoad`, `Event::ColorRequest`, `Term::drain_notifications()`) is deleted, the idle-wake channel is active, and spec-conformance Section 10.2 is unblocked. Spec-conformance Section 10's `depends_on: ["effect-cutover"]` is satisfied.

## Sections

| ID | Title | File | Status |
|----|-------|------|--------|
| 01 | Migrate mux consumer | `section-01-migrate-mux-consumer.md` | Complete |

### Subsections (inside section 01)

| ID   | Title                                                                                                 | Status      |
|------|-------------------------------------------------------------------------------------------------------|-------------|
| 01.1 | Atomic sink swap + Effect→MuxEvent router (no intermediate silent-drop state)                          | Complete (atomic with §01.2) |
| 01.2 | Activate `PendingResponse` polling with idle-wake channel                                              | Complete (atomic with §01.1) |
| 01.3 | Delete `IoThreadEventProxy`, `LegacyEventSink`, `Event::ClipboardLoad/ColorRequest`, drain shim        | Complete |
| 01.4 | Daemon-mode IPC for `HostRequest` — Path A (in-scope design) / B (file bug) / C (spin out plan)        | Complete |
| 01.N | Completion checklist                                                                                   | Complete |
