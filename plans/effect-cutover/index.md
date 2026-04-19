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

**§01.1 + §01.2 are COMPLETE** — atomic landing per the §01.1 CRITICAL ATOMICITY CONSTRAINT. Core wiring (commit `c5a21ab5`) plus dev follow-ups (Phase I bug filings: `BUG-11-8..BUG-11-14`; Phase J test matrix expansion: 30+ new pins across `effect_router/`, `response_poll/`, `host_request/`, `mux_pump/`, `mux_event/` test files; Phase K /tpr-review: 11 actionable findings fixed across 3 rounds + 1 outstanding tracked via `BUG-11-13`; Phase K /impl-hygiene-review: 2 swallowed-error patterns fixed inline + 1 file-size finding tracked as `BUG-11-14`). Next: §01.3 (deletion sweep) + §01.4 (daemon HostRequest audit per Path A/B/C).

## Sections

| ID | Title | File | Status |
|----|-------|------|--------|
| 01 | Migrate mux consumer | `section-01-migrate-mux-consumer.md` | In Progress |

### Subsections (inside section 01)

| ID   | Title                                                                                                 | Status      |
|------|-------------------------------------------------------------------------------------------------------|-------------|
| 01.1 | Atomic sink swap + Effect→MuxEvent router (no intermediate silent-drop state)                          | Complete (atomic with §01.2) |
| 01.2 | Activate `PendingResponse` polling with idle-wake channel                                              | Complete (atomic with §01.1) |
| 01.3 | Delete `IoThreadEventProxy`, `LegacyEventSink`, `Event::ClipboardLoad/ColorRequest`, drain shim        | Not Started |
| 01.4 | Daemon-mode IPC for `HostRequest` — Path A (in-scope design) / B (file bug) / C (spin out plan)        | Not Started |
| 01.N | Completion checklist                                                                                   | Not Started |
