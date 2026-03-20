---
section: "02"
title: "Widget Styling — Zero Radius & Flat"
status: not-started
reviewed: false
goal: "All widget styles use 0px corner radius and no shadows, matching the brutal aesthetic"
depends_on: ["01"]
sections:
  - id: "02.1"
    title: "Zero Radius on All Widgets"
    status: not-started
  - id: "02.2"
    title: "Remove Shadows"
    status: not-started
  - id: "02.3"
    title: "Completion Checklist"
    status: not-started
---

# Section 02: Widget Styling — Zero Radius & Flat

**Status:** Not Started
**Goal:** Every widget that draws rounded corners or shadows now draws sharp corners with no shadows.

**Depends on:** Section 01 (theme tokens).

---

## 02.1 Zero Radius on All Widgets

Audit and fix every `corner_radius` / `with_radius` usage:

- [ ] `ButtonStyle::from_theme()` — set `corner_radius: 0.0`
- [ ] `ToggleStyle::from_theme()` — track and thumb use pill shape → make rectangular
- [ ] `SliderStyle::from_theme()` — `track_radius: 0.0`, `thumb_radius: 0.0`
- [ ] `ScrollbarStyle::default()` — `thumb_radius: 0.0`
- [ ] `MenuStyle::from_theme()` — `corner_radius: 0.0`
- [ ] `SchemeCardWidget` — card border radius → 0.0
- [ ] `SettingRowWidget` — `CORNER_RADIUS: 0.0`
- [ ] `SettingsPanel` — `CORNER_RADIUS: 0.0`
- [ ] `DropdownWidget` — dropdown button radius → 0.0
- [ ] `DialogWidget` (confirmation) — radius → 0.0
- [ ] Any `RectStyle::filled(...).with_radius(...)` in form builders — remove radius

---

## 02.2 Remove Shadows

- [ ] `SettingsPanel::paint()` — remove `with_shadow(Shadow { ... })` in overlay mode
- [ ] `MenuStyle::from_theme()` — `shadow_color: Color::TRANSPARENT`
- [ ] Search for all `Shadow` struct usages — set all to zero/transparent
- [ ] `UiTheme::shadow` field → fully transparent

---

## 02.3 Completion Checklist

- [ ] `grep -r 'with_radius' oriterm_ui/src/widgets/` shows only `0.0` values
- [ ] `grep -r 'Shadow' oriterm_ui/src/widgets/` shows only transparent/zero shadows
- [ ] `./test-all.sh` green
- [ ] `./clippy-all.sh` green
- [ ] `./build-all.sh` green

**Exit Criteria:** No rounded corners or shadows visible in any settings dialog element.
