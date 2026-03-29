---
plan: "main-window-brutal"
title: "Main Window Brutal Design: Exhaustive Implementation Plan"
status: not-started
supersedes: []
references:
  - "mockups/main-window-brutal.html"
  - "mockups/settings-brutal.html"
  - "plans/completed/ui-css-framework/"
  - "plans/brutal-design-pass-2/"
---

# Main Window Brutal Design: Exhaustive Implementation Plan

## Mission

Apply the brutal design language to the main terminal window — transforming the tab bar from Chrome-style rounded tabs to flat, accent-barred tabs; creating a new status bar widget; and styling split pane dividers, focus borders, and window frames — to match `mockups/main-window-brutal.html` pixel-for-pixel. Every visual change must be accompanied by golden tests that lock the rendered output against regression.

## Architecture

```
mockups/main-window-brutal.html  (THE SPEC — mockup wins on any disagreement)
         │
         ├── Tab Bar ──────────────────────────────────────────────────
         │   │  36px height, flat tabs, no rounded corners
         │   │  Active: 2px accent bar on top + bottom border bleed
         │   │  Inactive: text-muted, 1px border-right separator
         │   │  Modified: 6px square accent dot
         │   │  Bottom: 2px border line
         │   │  Actions: border-left on each button
         │   │
         │   ├── oriterm_ui/src/widgets/tab_bar/constants.rs  (metrics)
         │   ├── oriterm_ui/src/widgets/tab_bar/colors.rs     (new tokens)
         │   ├── oriterm_ui/src/widgets/tab_bar/widget/draw.rs (rendering)
         │   └── oriterm_ui/src/widgets/tab_bar/widget/controls_draw.rs
         │
         ├── Status Bar ───────────────────────────────────────────────
         │   │  22px height, bg-surface, border-top 2px
         │   │  Items: shell(accent), panes, grid size | encoding, term(accent)
         │   │
         │   └── oriterm_ui/src/widgets/status_bar/  (NEW widget)
         │       ├── mod.rs          (StatusBarWidget, StatusBarData)
         │       └── tests.rs        (WidgetTestHarness tests)
         │
         ├── Split Panes ──────────────────────────────────────────────
         │   │  Divider: 2px border, accent on hover
         │   │  Focus: 2px accent outline, inset
         │   │
         │   ├── oriterm/src/app/divider_drag.rs     (hover detection)
         │   ├── oriterm/src/gpu/window_renderer/multi_pane.rs (append_dividers, append_focus_border)
         │   └── oriterm/src/config/mod.rs           (divider_px, divider_color defaults)
         │
         ├── Window Frame ─────────────────────────────────────────────
         │   │  2px border-strong around entire window (client-area drawing)
         │   │
         │   ├── oriterm/src/gpu/window_renderer/    (append_window_border)
         │   └── oriterm/src/app/chrome/mod.rs       (compute_window_layout — grid inset)
         │
         └── Golden Tests ─────────────────────────────────────────────
             │  Per-component: tab bar, status bar, dividers
             │  Composed: full main window chrome
             │  DPI: 96 + 192
             │
             └── oriterm/src/gpu/visual_regression/  (test modules)
```

## Design Principles

1. **Mockup is the spec.** When `mockups/main-window-brutal.html` and the running code disagree, the mockup wins. Every CSS property in the mockup maps to a concrete Rust field or constant. This principle was established by the CSS UI framework plan and carries forward.

2. **Golden tests lock every visual change.** Every rendering modification gets a golden test written BEFORE the implementation change (TDD). The golden test captures the current output, the implementation changes it, and the updated golden reference proves the change is correct. No visual change ships without a committed reference PNG.

3. **Per-element styling from theme tokens.** All styling derives from `UiTheme` fields. No hardcoded colors in draw code. New colors (accent bar, bar border) are added to `TabBarColors::from_theme()`, not embedded as hex literals. This was the foundational principle of the CSS UI framework plan.

## Section Dependency Graph

```
01 Tab Bar Brutal ─────────┐
                           │
02 Status Bar Widget ──────┤  (independent of 01)
                           │
03 Window Frame & Panes ───┤  (independent of 01, 02)
                           │
                           ↓
04 Production Wiring ──────┤  (needs 01, 02, 03)
                           │
                           ↓
05 Verification ───────────┘  (needs all)
```

- Sections 01, 02, 03 are **independent** and can be worked in any order.
- Section 04 requires all three foundation sections (wires everything into production).
- Section 05 requires all sections (final verification).

**Cross-section interactions:**
- **Section 01 + Section 04**: Tab bar metrics change affects the grid origin offset calculated in `oriterm/src/app/redraw/`. Section 04 updates all call sites.
- **Section 02 + Section 04**: Status bar must be wired into the window render pipeline. Section 04 handles the plumbing.

## Implementation Sequence

```
Phase 1 - Component Redesign (independent, any order)
  ├─ 01: Tab bar brutal styling + golden tests
  ├─ 02: Status bar widget + golden tests
  └─ 03: Window frame + split pane styling + golden tests
  Gate: Each component passes its individual golden tests

Phase 2 - Integration
  └─ 04: Wire status bar into production, update grid offsets,
         update existing golden references, composed golden tests
  Gate: Full window renders with all components, composed goldens pass

Phase 3 - Verification
  └─ 05: DPI scaling, cross-platform, build/clippy/test gates
  Gate: ./build-all.sh, ./clippy-all.sh, ./test-all.sh all green
```

**Why this order:**
- Phase 1 sections are pure additions/modifications to isolated components. Each can be tested independently via WidgetTestHarness or visual regression.
- Phase 2 must wait for all components because it wires them together and updates composed golden tests.
- Phase 3 is the final gate.

## Metrics (Current State)

| Area | Key Files | Current LOC |
|------|-----------|-------------|
| Tab bar widget | `oriterm_ui/src/widgets/tab_bar/` (17 files) | ~2,200 |
| Tab bar colors | `oriterm_ui/src/widgets/tab_bar/colors.rs` | ~59 |
| Tab bar constants | `oriterm_ui/src/widgets/tab_bar/constants.rs` | ~128 |
| Window chrome | `oriterm_ui/src/widgets/window_chrome/` (5 files) | ~780 |
| Theme | `oriterm_ui/src/theme/mod.rs` | ~172 |
| Visual regression | `oriterm/src/gpu/visual_regression/` (9 files) | ~1,200 |
| Multi-pane redraw | `oriterm/src/app/redraw/multi_pane/` (4 files) | ~550 |
| Draw helpers | `oriterm/src/app/redraw/draw_helpers.rs` | 211 |
| Window renderer multi-pane | `oriterm/src/gpu/window_renderer/multi_pane.rs` | 227 |
| Chrome layout | `oriterm/src/app/chrome/mod.rs` | 380 |
| Chrome tests | `oriterm/src/app/chrome/tests.rs` | 335 |
| Config | `oriterm/src/config/mod.rs` | 391 |
| Config tests | `oriterm/src/config/tests.rs` | ~2070+ |
| Window context | `oriterm/src/app/window_context.rs` | 139 |
| Single-pane redraw | `oriterm/src/app/redraw/mod.rs` | 463 |

## Estimated Effort

| Section | Est. Lines Changed | Complexity | Depends On |
|---------|-------------------|------------|------------|
| 01 Tab Bar Brutal | ~200 modified (draw.rs, colors.rs, constants.rs, drag_draw.rs), ~250 new (tests, new fields), ~130 moved (draw_helpers.rs extraction) | Medium | -- |
| 02 Status Bar Widget | ~300 new (mod.rs + tests.rs), ~150 new (golden tests) | Medium | -- |
| 03 Window Frame & Panes | ~100 modified (multi_pane.rs, config/mod.rs, config/tests.rs), ~80 new (tests) | Medium | -- |
| 04 Production Wiring | ~300 modified (chrome/mod.rs, chrome/tests.rs, window_context.rs, redraw/mod.rs, multi_pane/mod.rs, draw_helpers.rs, init, window_management, config) + ~200 new (golden tests) | High | 01, 02, 03 |
| 05 Verification | ~50 new (test matrix, DPI tests) | Low | 01-04 |
| **Total** | **~1,800** | | |

## Known Bugs (Pre-existing)

| Bug | Root Cause | Fix Location | Status |
|-----|-----------|-------------|--------|
| `ACTIVE_TAB_RADIUS` is 8.0 (Chrome-style) | Legacy design choice | Section 01.2 | Will fix |
| No status bar exists | Not yet implemented | Section 02 | Fixed (StatusBarWidget implemented) |
| Tab bar height (46px) doesn't match mockup (36px) | Pre-brutal metrics | Section 01.1 | Will fix |
| `bar_bg` and `active_bg` are swapped vs mockup | `bar_bg` maps to `bg_secondary` (bg-base) but mockup tab-bar bg is `bg-surface` (`bg_primary`); `active_bg` maps to `bg_primary` (bg-surface) but mockup active tab is `bg-base` (`bg_secondary`) | Section 01.3 + 01.4 | Will fix |
| Default divider color (#505050) doesn't match mockup (#2a2a36) | Neutral gray default, not theme-derived | Section 03.1 | Fixed |
| Default divider width (1px) doesn't match mockup (2px) | `PaneConfig::default().divider_px` is 1.0 | Section 03.1 | Fixed |
| chrome/tests.rs hardcodes `46.0` | All `compute_window_layout` tests pass `46.0` as the tab bar height literal | Section 04.2 | Will fix |
| Tab bar close button opacity has no `0.6` codepath for active | Active tab currently always shows close at `content_opacity` (1.0), not 0.6 | Section 01.5 | Will fix |
| Focus border is hardcoded 2px physical, not DPI-scaled | `append_focus_border` uses `let border = 2.0_f32` in physical pixels. At 2x DPI this is only 1 logical pixel, not matching the mockup's 2 logical pixels | Section 03.2 | Fixed |
| Default focus border color (#6495ED) doesn't match mockup (#6d9be0) | `DEFAULT_FOCUS_BORDER_COLOR` is cornflower blue, mockup uses `--accent` | Section 03.1 | Fixed |

## Quick Reference

| ID | Title | File | Status |
|----|-------|------|--------|
| 01 | Tab Bar Brutal Styling | `section-01-tab-bar.md` | Complete |
| 02 | Status Bar Widget | `section-02-status-bar.md` | Complete |
| 03 | Window Frame & Split Pane Styling | `section-03-frame-panes.md` | Complete |
| 04 | Production Wiring & Composed Tests | `section-04-wiring.md` | Not Started |
| 05 | Verification | `section-05-verification.md` | Not Started |
