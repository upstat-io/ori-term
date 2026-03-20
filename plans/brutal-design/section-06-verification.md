---
section: "06"
title: "Verification"
status: not-started
reviewed: false
goal: "Settings dialog visually matches mockup at 100%, 150%, and 200% DPI with no regressions"
depends_on: ["01", "02", "03", "04", "05"]
sections:
  - id: "06.1"
    title: "Visual Comparison"
    status: not-started
  - id: "06.2"
    title: "Completion Checklist"
    status: not-started
---

# Section 06: Verification

**Status:** Not Started
**Goal:** Side-by-side comparison of settings dialog against `mockups/settings-brutal.html` shows visual match at all DPI levels.

---

## 06.1 Visual Comparison

- [ ] Open mockup in browser at 100% zoom, screenshot
- [ ] Open settings dialog at 100% DPI, screenshot
- [ ] Compare: colors, spacing, typography, borders, hover states
- [ ] Verify at 150% DPI
- [ ] Verify at 200% DPI
- [ ] Verify hover transitions on setting rows
- [ ] Verify toggle, slider, dropdown styling
- [ ] Verify scheme card styling on Colors page
- [ ] Verify both overlay and dialog rendering modes

---

## 06.2 Completion Checklist

- [ ] All theme colors match mockup CSS variables
- [ ] Zero rounded corners anywhere in the dialog
- [ ] Zero shadows anywhere in the dialog
- [ ] Section titles show `// PREFIX` with horizontal rule
- [ ] Sidebar active indicator: 3px left border + accent-bg-strong
- [ ] Footer buttons: ghost vs primary distinction
- [ ] Page titles: uppercase, --text-bright
- [ ] `./test-all.sh` green
- [ ] `./clippy-all.sh` green
- [ ] `./build-all.sh` green

**Exit Criteria:** Settings dialog is visually indistinguishable from the mockup at normal viewing distance at 100% DPI.
