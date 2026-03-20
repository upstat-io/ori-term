---
section: "05"
title: "Footer & Button Styling"
status: not-started
reviewed: false
goal: "Footer buttons match mockup: ghost style for Reset/Cancel, primary accent style for Save"
depends_on: ["01"]
sections:
  - id: "05.1"
    title: "Button Styles"
    status: not-started
  - id: "05.2"
    title: "Footer Separator"
    status: not-started
  - id: "05.3"
    title: "Completion Checklist"
    status: not-started
---

# Section 05: Footer & Button Styling

**Status:** Not Started
**Goal:** Footer matches mockup: separator line in `--border-strong`, "Reset to Defaults" and "Cancel" use ghost style (transparent bg, border, muted text), "Save" uses primary accent style (solid accent bg, white text).

**File(s):** `oriterm_ui/src/widgets/settings_panel/mod.rs`

**Reference:** `mockups/settings-brutal.html` CSS classes `.footer`, `.btn-ghost`, `.btn-primary`

---

## 05.1 Button Styles

Mockup button styles:
- **Ghost** (Reset, Cancel): transparent bg, 1px `--border` border, `--text-muted` text, hover: `--bg-hover` bg
- **Primary** (Save): `--accent` bg, white text, hover: `--accent-hover` bg

- [ ] Update `ghost_style` in `SettingsPanel::build()`:
  ```rust
  let ghost_style = ButtonStyle {
      fg: theme.fg_secondary,       // --text-muted
      bg: Color::TRANSPARENT,
      hover_bg: theme.bg_hover,     // --bg-hover
      pressed_bg: theme.bg_active,  // --bg-active
      border_color: theme.border,   // --border
      border_width: 1.0,
      corner_radius: 0.0,          // brutal: sharp corners
      ..ButtonStyle::from_theme(theme)
  };
  ```

- [ ] Update Save button style:
  ```rust
  let save_style = ButtonStyle {
      fg: Color::WHITE,
      bg: theme.accent,             // --accent
      hover_bg: theme.accent_hover, // --accent-hover
      pressed_bg: theme.accent,
      border_width: 0.0,
      corner_radius: 0.0,          // brutal: sharp corners
      ..ButtonStyle::from_theme(theme)
  };
  ```

---

## 05.2 Footer Separator

Mockup: separator line is 2px `--border-strong` (not 1px `--border`).

- [ ] Update footer separator: 2px height, `--border-strong` color
- [ ] Verify separator spans full width

---

## 05.3 Completion Checklist

- [ ] Reset/Cancel buttons: ghost style (transparent bg, border, muted text)
- [ ] Save button: accent bg, white text
- [ ] All buttons: 0px corner radius
- [ ] Footer separator: 2px, --border-strong color
- [ ] `./test-all.sh` green
- [ ] `./clippy-all.sh` green
- [ ] `./build-all.sh` green

**Exit Criteria:** Footer buttons match mockup styling. Ghost vs primary distinction is visually clear.
