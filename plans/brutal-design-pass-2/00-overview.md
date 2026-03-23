---
plan: "brutal-design-pass-2"
title: "Brutal Design System — Visual Fidelity Pass"
status: in-progress
references:
  - "mockups/settings-brutal.html"
  - "plans/brutal-design/"
---

# Brutal Design System — Visual Fidelity Pass

## Mission

The first brutal design pass (plans/brutal-design/) established the theme token foundation and basic widget styling. This pass closes the gap between the mockup (`mockups/settings-brutal.html`) and the actual rendered settings dialog — fixing widget shapes, layout structure, typography, and interaction details identified during visual verification.

## Scope

One section per settings tab, worked sequentially. Each section achieves visual fidelity for that tab. Global widget changes (slider, toggle, dropdown, buttons) land in Section 01 (Appearance) since they're visible there first and affect all tabs.

## Findings Reference

27 visual differences identified during brutal-design Section 06 verification. The HTML mockup (`mockups/settings-brutal.html`) serves as the visual reference — open it in a browser alongside the running app for comparison.

## Mockup CSS Source of Truth

All target values come from `mockups/settings-brutal.html` CSS variables and widget classes. The mockup is the spec — when the mockup and code disagree, the mockup wins.

## Quick Reference

| ID | Title | File | Status |
|----|-------|------|--------|
| 01 | Appearance Tab Visual Fidelity | `section-01-appearance-tab.md` | In Progress |
