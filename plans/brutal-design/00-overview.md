---
plan: "brutal-design"
title: "Brutal Design System — Settings Panel Visual Overhaul"
status: not-started
references:
  - "mockups/settings-brutal.html"
  - "plans/ui-framework-overhaul/"
---

# Brutal Design System — Settings Panel Visual Overhaul

## Mission

Transform the settings dialog from its current soft/rounded aesthetic to the "brutal" design language defined in `mockups/settings-brutal.html`: zero border radius, no shadows, sharp corners, monospace typography, `//` prefixed section headers with horizontal rules, and a cohesive dark color palette with precise CSS variable mappings.

## Design Language (from mockup CSS)

```css
--bg-base:       #0e0e12;    /* Sidebar background */
--bg-surface:    #16161c;    /* Content area background */
--bg-raised:     #1c1c24;    /* Cards, elevated elements */
--bg-hover:      #24242e;    /* Hover state */
--bg-active:     #2a2a36;    /* Pressed/active state */
--bg-input:      #12121a;    /* Input field background */
--border:        #2a2a36;    /* Default borders */
--border-subtle: #1e1e28;    /* Subtle borders */
--border-strong: #3a3a48;    /* Strong borders (panel edge) */
--text:          #d4d4dc;    /* Primary text */
--text-muted:    #9494a8;    /* Secondary text */
--text-faint:    #8c8ca0;    /* Faint text (section titles, labels) */
--text-bright:   #eeeeef;    /* Bright text (page titles) */
--accent:        #6d9be0;    /* Accent color (active items) */
--accent-hover:  #85ade8;    /* Accent hover */
--accent-bg:     rgba(109, 155, 224, 0.08);   /* Subtle accent background */
--accent-bg-strong: rgba(109, 155, 224, 0.14); /* Active item background */
--radius:        0px;        /* ALL corners are sharp */
--shadow:        none;       /* NO shadows */
```

## Section Dependency Graph

```
01: Theme Tokens ──→ 02: Widget Styling ──→ 03: Sidebar ──┐
                                            04: Content ───┤──→ 06: Verification
                                            05: Footer ────┘
```

- Section 01 (theme tokens) must come first — all other sections reference theme values.
- Sections 02-05 can be worked in any order after 01.
- Section 06 verifies everything.

## Implementation Sequence

```
Phase 1 - Foundation
  +-- 01: Map mockup CSS variables to UiTheme fields

Phase 2 - Widget Updates (independent, any order)
  +-- 02: Remove all corner_radius, shadows from widget styles
  +-- 03: Sidebar enhancements (// prefix, horizontal rule, active indicator)
  +-- 04: Content area (section titles, page headers, uppercase)
  +-- 05: Footer button styling (ghost vs accent)

Phase 3 - Verification
  +-- 06: Visual comparison against mockup at 100%/150%/200% DPI
```

## Estimated Effort

| Section | Est. Lines | Complexity |
|---------|-----------|------------|
| 01 Theme Tokens | ~50 | Low |
| 02 Widget Styling | ~100 | Low |
| 03 Sidebar | ~150 | Medium |
| 04 Content Area | ~100 | Low |
| 05 Footer Buttons | ~50 | Low |
| 06 Verification | ~0 (manual) | Low |
| **Total** | **~450** | |

## Quick Reference

| ID | Title | File | Status |
|----|-------|------|--------|
| 01 | Theme Token Overhaul | `section-01-theme-tokens.md` | Not Started |
| 02 | Widget Styling | `section-02-widget-styling.md` | Not Started |
| 03 | Sidebar Enhancements | `section-03-sidebar.md` | Not Started |
| 04 | Content Area Styling | `section-04-content-area.md` | Not Started |
| 05 | Footer & Button Styling | `section-05-footer-buttons.md` | Not Started |
| 06 | Verification | `section-06-verification.md` | Not Started |
