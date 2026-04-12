---
reroute: false
name: "Effect Cutover"
full_name: "Effect Cutover — Migrate Legacy Event Consumers to Effect"
status: queued
order: 999
---

# Effect Cutover

Migrate all remaining `Event`-based consumers to subscribe to `Effect` directly, then delete the deprecated `Event::ClipboardLoad` and `Event::ColorRequest` variants and the `LegacyEventSink` adapter.

## Sections

| ID | Title | File | Status |
|----|-------|------|--------|
| 01 | Migrate mux consumer | `section-01-migrate-mux-consumer.md` | Not Started |
