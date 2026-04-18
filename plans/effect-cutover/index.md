---
reroute: false
name: "Effect Cutover"
full_name: "Effect Cutover — Migrate Legacy Event Consumers to Effect"
status: in-progress
order: 999
---

# Effect Cutover

Migrate all remaining `Event`-based consumers to subscribe to `Effect` directly, then delete the deprecated `Event::ClipboardLoad` and `Event::ColorRequest` variants and the `LegacyEventSink` adapter.

## Resume Point

**Commit `c5a21ab5` on `dev`** landed the §01.1+§01.2 code work as WIP. Pick up at Phase I → J → K (bug filings → remaining TDD matrix → `/tpr-review` + `/impl-hygiene-review`). See `section-01-migrate-mux-consumer.md §Session Resume Point` for the full done/remaining breakdown.

## Sections

| ID | Title | File | Status |
|----|-------|------|--------|
| 01 | Migrate mux consumer | `section-01-migrate-mux-consumer.md` | In Progress |

### Subsections (inside section 01)

| ID   | Title                                                                                                 | Status      |
|------|-------------------------------------------------------------------------------------------------------|-------------|
| 01.1 | Atomic sink swap + Effect→MuxEvent router (no intermediate silent-drop state)                          | In Progress — core code in commit `c5a21ab5`; gates pending |
| 01.2 | Activate `PendingResponse` polling with idle-wake channel                                              | In Progress — core code in commit `c5a21ab5`; gates pending |
| 01.3 | Delete `IoThreadEventProxy`, `LegacyEventSink`, `Event::ClipboardLoad/ColorRequest`, drain shim        | Not Started |
| 01.4 | Daemon-mode IPC for `HostRequest` — Path A (in-scope design) / B (file bug) / C (spin out plan)        | Not Started |
| 01.N | Completion checklist                                                                                   | Not Started |
