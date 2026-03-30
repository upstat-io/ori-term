---
plan: "bug-tracker"
title: "Active Bug Tracker"
status: in-progress
references:
  - "CLAUDE.md"
---

# Active Bug Tracker

## Mission

Track and fix all discovered bugs across the ori_term codebase. This is a living plan — it is never marked as complete. Bugs are added as they are discovered and marked as fixed when resolved.

## How This Plan Works

- **Parallel plan**: Runs alongside the main roadmap. Never blocks normal work.
- **Never finished**: Sections and the plan itself are never marked `complete` or `status: complete`.
- **Categories as sections**: Each section represents a domain/area of the codebase.
- **Additive**: New bugs are appended to existing sections or new sections are created.
- **Fixed bugs**: Individual bug items are marked `[x]` with a resolution note, but the section stays `in-progress`.
- **Last choice in /continue-roadmap**: When presented as an option, bug fixes are always the last item in the choice list, never recommended as the top choice.

## Design Principles

- **Fix at source**: Every bug fix addresses the root cause, not a workaround.
- **No deferrals**: Per CLAUDE.md, discovered bugs are fixed immediately. This plan tracks them for visibility and prioritization when multiple bugs exist.
- **Test every fix**: Every bug fix includes a test that would have caught the bug.

## Quick Reference

| ID | Title | File | Total | Open |
|----|-------|------|-------|------|
| 01 | UI Widgets | `section-01-ui-widgets.md` | 13 | 0 |
| 02 | Settings Dialog | `section-02-settings-dialog.md` | 11 | 1 |
| 03 | UI Framework | `section-03-ui-framework.md` | 2 | 1 |
| 04 | Fonts | `section-04-fonts.md` | 13 | 7 |
| 05 | Config | `section-05-config.md` | 5 | 1 |
| 06 | Rendering & Perf | `section-06-rendering-perf.md` | 3 | 1 |
| 07 | CI & Build | `section-07-ci-build.md` | 3 | 0 (TPR resolved) |
| 08 | Core Terminal | `section-08-core-terminal.md` | 2 | 2 |
