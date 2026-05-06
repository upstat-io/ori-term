---
plan: "BUG-08-004"
title: "BUG-08-004 — vttest LNM key encoding not testable in headless mode"
status: complete
bug:
  id: "BUG-08-004"
  severity: "low"
  subsystem: "oriterm_core/tests/vttest/session.rs"
  found: "2026-04-03"
  source: "vttest conformance audit"
references:
  - "bug-tracker/section-08-core-terminal.md"
---

# BUG-08-004 — vttest LNM key encoding not testable in headless mode

**Status:** Complete
**Severity:** low
**Resolved:** 2026-05-06

## Resolution

- Added `encode_enter_base()` as SSOT in `oriterm_core`, consumed by both the application key encoder (`legacy.rs`) and `PtySession::send_enter()`.
- Updated `walk_vttest_screens()` to use LNM-aware Enter simulation.
- vttest menu 6 sub-item 2 now shows CR+LF OK instead of bare CR failure.
