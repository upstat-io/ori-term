---
section: "09"
title: "Settings Content Completeness"
status: not-started
reviewed: true
third_party_review:
  status: none
  updated: null
goal: "All settings visible in the mockup Appearance page exist in the app — including the Decorations section with unfocused opacity, window decorations, and tab bar style"
depends_on: ["01", "02", "03", "05", "06", "07", "08"]
sections:
  - id: "09.1"
    title: "Missing Settings — Appearance Page"
    status: not-started
  - id: "09.2"
    title: "Missing Section — Decorations"
    status: not-started
  - id: "09.3"
    title: "Config Integration"
    status: not-started
  - id: "09.R"
    title: "Third Party Review Findings"
    status: not-started
  - id: "09.4"
    title: "Build & Verify"
    status: not-started
---

# Section 09: Settings Content Completeness

**Status:** Not Started
**Goal:** Every setting visible on the mockup's Appearance page is present in the running app. Three settings are currently missing entirely, and one section ("Decorations") does not exist. After this section, the Appearance page's content is 1:1 with the mockup.

**Production code paths:**
- Form builder: `oriterm/src/app/settings_overlay/form_builder/appearance.rs`
- Config model: `oriterm/src/config/mod.rs` (`WindowConfig` struct)
- Settings IDs: `oriterm/src/app/settings_overlay/form_builder/mod.rs` (`SettingsIds` struct)
- Save handler: `oriterm/src/app/settings_overlay/action_handler/mod.rs` (routes `WidgetAction` to config changes)

**Observable change:** Opening the settings dialog shows a "Decorations" section below "Window" with three controls: an "Unfocused opacity" slider, a "Window decorations" dropdown (None / Full), and a "Tab bar style" dropdown (Default / Compact). All controls read from and write back to Config.

---

## 09.1 Missing Settings — Appearance Page

**File(s):** `oriterm/src/app/settings_overlay/form_builder/appearance.rs`

The mockup's Appearance page has these settings that the current `build_page()` does not create:

| Setting | Section | Control | Mockup Values |
|---------|---------|---------|---------------|
| Unfocused opacity | Window | Slider 30-100% | Same range/step as regular opacity |
| Window decorations | Decorations | Dropdown | "None (Frameless)", "Full" |
| Tab bar style | Decorations | Dropdown | "Default", "Compact" |

### Current state

`build_page()` calls two section builders:
1. `build_theme_section()` — color scheme dropdown (matches mockup).
2. `build_window_section()` — opacity slider + blur toggle (matches mockup, but missing unfocused opacity).

### Changes needed

**Add "Unfocused opacity" slider to `build_window_section()`:**
- After the existing blur toggle row, add a third `SettingRowWidget`.
- Control: `SliderWidget::new().with_range(30.0, 100.0).with_step(1.0).with_value(config.window.unfocused_opacity * 100.0)`.
- Name: `"Unfocused opacity"`.
- Description: `"Window transparency when not focused (30-100%)"`.
- Store the slider's `WidgetId` as `ids.unfocused_opacity_slider`.

Note: The `unfocused_opacity` field may not exist in `WindowConfig` yet. If not, Section 09.3 adds it. Use `config.window.opacity` as the fallback initial value.

**Add `build_decorations_section()`:**
- New function matching the pattern of `build_theme_section()` / `build_window_section()`.
- Section title: `section_title("Decorations", theme)`.
- Two setting rows (see 09.2).
- Wire into `build_page()` as the third section: `vec![theme, window, decorations]`.

### Checklist

- [ ] `SliderWidget` for unfocused opacity added to `build_window_section()`.
- [ ] `ids.unfocused_opacity_slider` field added to `SettingsIds`.
- [ ] `build_decorations_section()` function created and wired into `build_page()`.
- [ ] `ids.decorations_dropdown` and `ids.tab_bar_style_dropdown` fields added to `SettingsIds`.
- [ ] Build passes (`./build-all.sh`).

---

## 09.2 Missing Section — Decorations

**File(s):** `oriterm/src/app/settings_overlay/form_builder/appearance.rs`

The mockup shows a "Decorations" section below "Window" with two dropdowns.

### Window decorations dropdown

- Name: `"Window decorations"`.
- Description: `"Window frame and title bar style"`.
- Options: `["None (Frameless)", "Full"]` on Windows/Linux; add `"Transparent"`, `"Buttonless"` on macOS.
- Initial selection: derived from `config.window.decorations`. The `Decorations` enum already exists in Config with variants `None`, `Full`, `Transparent`, and `Buttonless`.
- Conversion: `Decorations::None` -> index 0, `Decorations::Full` -> index 1 (plus platform-specific variants).

### Tab bar style dropdown

- Name: `"Tab bar style"`.
- Description: `"Visual style and density of the tab bar"`.
- Options: `["Default", "Compact"]`.
- Initial selection: derived from `config.window.tab_bar_position` or a new `tab_bar_style` field. Check `WindowConfig` for existing fields. If `TabBarStyle` enum does not exist, create it in Config (see 09.3).
- Conversion: `TabBarStyle::Default` -> index 0, `TabBarStyle::Compact` -> index 1.

### Implementation

```rust
fn build_decorations_section(
    config: &Config,
    ids: &mut SettingsIds,
    theme: &UiTheme,
) -> Box<dyn Widget> {
    // Window decorations dropdown.
    // On Windows/Linux, only None and Full are meaningful.
    // On macOS, Transparent and Buttonless are also available.
    let decos_items = vec!["None (Frameless)".into(), "Full".into()];
    let decos_selected = match config.window.decorations {
        Decorations::None => 0,
        Decorations::Full | Decorations::Transparent | Decorations::Buttonless => 1,
    };
    let decos_dropdown = DropdownWidget::new(decos_items).with_selected(decos_selected);
    ids.decorations_dropdown = decos_dropdown.id();
    let decos_row = SettingRowWidget::new(
        "Window decorations",
        "Window frame and title bar style",
        Box::new(decos_dropdown),
        theme,
    );

    // Tab bar style dropdown.
    let tab_items = vec!["Default".into(), "Compact".into()];
    let tab_selected = /* derive from config */ 0;
    let tab_dropdown = DropdownWidget::new(tab_items).with_selected(tab_selected);
    ids.tab_bar_style_dropdown = tab_dropdown.id();
    let tab_row = SettingRowWidget::new(
        "Tab bar style",
        "Visual style and density of the tab bar",
        Box::new(tab_dropdown),
        theme,
    );

    let title = section_title("Decorations", theme);
    Box::new(
        ContainerWidget::column()
            .with_width(SizeSpec::Fill)
            .with_gap(ROW_GAP)
            .with_child(title)
            .with_child(Box::new(decos_row))
            .with_child(Box::new(tab_row)),
    )
}
```

### Checklist

- [ ] `build_decorations_section()` implemented.
- [ ] Section uses `section_title("Decorations", theme)` with divider.
- [ ] Two `SettingRowWidget` rows with `DropdownWidget` controls.
- [ ] Section wired into `build_page()` as third section.

---

## 09.3 Config Integration

**File(s):** `oriterm/src/config/mod.rs`, `oriterm/src/app/settings_overlay/mod.rs`

New settings must read from Config on dialog open and write back on Save.

### New Config fields

Check `WindowConfig` for existing fields. Add any that are missing:

1. **`unfocused_opacity: f32`** — Window opacity when not focused. Default `1.0`. Range `0.3..=1.0`. If this field already exists (it may, as `tab_bar_opacity` is `Option<f32>`), just wire it. If not, add it with `#[serde(default = "default_unfocused_opacity")]` and `fn default_unfocused_opacity() -> f32 { 1.0 }`.

2. **`tab_bar_style: TabBarStyle`** — Enum with `Default` and `Compact` variants. If a similar enum exists (check for `TabBarPosition`), evaluate whether to reuse or create a new one. This controls tab bar density, not position.

### Save handler wiring

In the settings overlay's save handler (the function that runs when `WidgetAction::SaveSettings` is received):

1. Read `ids.unfocused_opacity_slider` value from widget tree, divide by 100.0, store in `config.window.unfocused_opacity`.
2. Read `ids.decorations_dropdown` selected index, convert to `Decorations` enum variant, store in `config.window.decorations`.
3. Read `ids.tab_bar_style_dropdown` selected index, convert to `TabBarStyle` enum variant, store in `config.window.tab_bar_style`.

### Reset handler wiring

In the reset handler (when `WidgetAction::ResetDefaults` is received), the new fields must reset to their defaults:
- `unfocused_opacity` -> `1.0` (slider value `100`).
- `decorations` -> `Decorations::None` (dropdown index `0`).
- `tab_bar_style` -> `TabBarStyle::Default` (dropdown index `0`).

### Unsaved detection

The dirty-tracking logic that sets `WidgetAction::SettingsUnsaved(true/false)` must compare the new fields against the original Config snapshot.

### Checklist

- [ ] `unfocused_opacity` field exists in `WindowConfig` with serde default.
- [ ] `TabBarStyle` enum exists with `Default` and `Compact` variants.
- [ ] `tab_bar_style` field exists in `WindowConfig` with serde default.
- [ ] Save handler reads all three new widget values and writes to Config.
- [ ] Reset handler resets all three new fields.
- [ ] Dirty detection includes the new fields.
- [ ] Serialization round-trips correctly (add to existing config serde test if one exists).

---

## 09.R Third Party Review Findings

Reserved for findings from `/review-plan` or external review. Not actionable until populated.

---

## 09.4 Build & Verify

### Gate

```bash
./build-all.sh
./clippy-all.sh
./test-all.sh
```

### Checklist

- [ ] `./build-all.sh` passes
- [ ] `./clippy-all.sh` passes
- [ ] `./test-all.sh` passes
- [ ] Settings dialog shows "Decorations" section with two dropdowns
- [ ] Unfocused opacity slider visible in Window section
- [ ] Config serde round-trip test passes with new fields
