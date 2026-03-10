//! Builds the settings form from current config state.
//!
//! Constructs a `FormLayout` populated with sections and controls
//! that reflect the current `Config`. Widget IDs are captured in
//! `SettingsIds` for action dispatch in Section 04.

use oriterm_ui::widget_id::WidgetId;
use oriterm_ui::widgets::Widget;
use oriterm_ui::widgets::checkbox::CheckboxWidget;
use oriterm_ui::widgets::dropdown::DropdownWidget;
use oriterm_ui::widgets::form_layout::FormLayout;
use oriterm_ui::widgets::form_row::FormRow;
use oriterm_ui::widgets::form_section::FormSection;
use oriterm_ui::widgets::toggle::ToggleWidget;

use crate::config::{BellAnimation, Config, CursorStyle, PasteWarning};

/// Opacity dropdown values (display label → actual value).
pub(in crate::app) const OPACITY_VALUES: [f32; 8] = [0.3, 0.4, 0.5, 0.6, 0.7, 0.8, 0.9, 1.0];

/// Font size dropdown values.
pub(in crate::app) const FONT_SIZE_VALUES: [f32; 15] = [
    8.0, 9.0, 10.0, 11.0, 12.0, 13.0, 14.0, 15.0, 16.0, 18.0, 20.0, 22.0, 24.0, 28.0, 32.0,
];

/// Bell duration dropdown values in milliseconds.
pub(in crate::app) const BELL_DURATION_VALUES: [u16; 7] = [0, 50, 100, 150, 200, 300, 500];

/// Widget IDs for all settings controls, used to match actions in both
/// overlay dispatch and dialog window event handling.
pub(crate) struct SettingsIds {
    pub theme_dropdown: WidgetId,
    pub opacity_dropdown: WidgetId,
    pub font_size_dropdown: WidgetId,
    pub font_weight_dropdown: WidgetId,
    pub ligatures_checkbox: WidgetId,
    pub paste_warning_dropdown: WidgetId,
    pub cursor_style_dropdown: WidgetId,
    pub cursor_blink_toggle: WidgetId,
    pub bell_animation_dropdown: WidgetId,
    pub bell_duration_dropdown: WidgetId,
}

/// Builds the complete settings form from the current config.
///
/// Returns the populated `FormLayout` and the ID map for action dispatch.
/// The caller must call `form.compute_label_widths(measurer, theme)` before
/// pushing the panel into the overlay.
pub(in crate::app) fn build_settings_form(config: &Config) -> (FormLayout, SettingsIds) {
    let (appearance, theme_id, opacity_id) = build_appearance_section(config);
    let (font, size_id, weight_id, liga_id) = build_font_section(config);
    let (behavior, paste_id) = build_behavior_section(config);
    let (terminal, style_id, blink_id) = build_terminal_section(config);
    let (bell, anim_id, dur_id) = build_bell_section(config);

    let form = FormLayout::new()
        .with_section(appearance)
        .with_section(font)
        .with_section(behavior)
        .with_section(terminal)
        .with_section(bell);

    let ids = SettingsIds {
        theme_dropdown: theme_id,
        opacity_dropdown: opacity_id,
        font_size_dropdown: size_id,
        font_weight_dropdown: weight_id,
        ligatures_checkbox: liga_id,
        paste_warning_dropdown: paste_id,
        cursor_style_dropdown: style_id,
        cursor_blink_toggle: blink_id,
        bell_animation_dropdown: anim_id,
        bell_duration_dropdown: dur_id,
    };

    (form, ids)
}

/// Appearance section: Theme dropdown, Opacity dropdown.
fn build_appearance_section(config: &Config) -> (FormSection, WidgetId, WidgetId) {
    let names = crate::scheme::builtin_names();
    let selected = names
        .iter()
        .position(|n| *n == config.colors.scheme)
        .unwrap_or(0);
    let items: Vec<String> = names.iter().map(|s| (*s).to_owned()).collect();
    let theme_dropdown = DropdownWidget::new(items).with_selected(selected);
    let theme_id = theme_dropdown.id();

    let opacity_items: Vec<String> = OPACITY_VALUES
        .iter()
        .map(|v| format!("{:.0}%", v * 100.0))
        .collect();
    let opacity_idx = OPACITY_VALUES
        .iter()
        .position(|v| (*v - config.window.opacity).abs() < 0.01)
        .unwrap_or(OPACITY_VALUES.len() - 1);
    let opacity_dropdown = DropdownWidget::new(opacity_items).with_selected(opacity_idx);
    let opacity_id = opacity_dropdown.id();

    let section = FormSection::new("Appearance")
        .with_row(FormRow::new("Theme", Box::new(theme_dropdown)))
        .with_row(FormRow::new("Opacity", Box::new(opacity_dropdown)));

    (section, theme_id, opacity_id)
}

/// Font section: Size dropdown, Weight dropdown, Ligatures checkbox.
fn build_font_section(config: &Config) -> (FormSection, WidgetId, WidgetId, WidgetId) {
    let size_items: Vec<String> = FONT_SIZE_VALUES.iter().map(|v| format!("{v:.0}")).collect();
    let size_idx = FONT_SIZE_VALUES
        .iter()
        .position(|v| (*v - config.font.size).abs() < 0.1)
        .unwrap_or(4); // Default to 12.0
    let size_dropdown = DropdownWidget::new(size_items).with_selected(size_idx);
    let size_id = size_dropdown.id();

    let weight_items: Vec<String> = [100, 200, 300, 400, 500, 600, 700, 800, 900]
        .iter()
        .map(i32::to_string)
        .collect();
    let weight_idx = match config.font.weight {
        w if w <= 100 => 0,
        w if w <= 200 => 1,
        w if w <= 300 => 2,
        w if w <= 400 => 3,
        w if w <= 500 => 4,
        w if w <= 600 => 5,
        w if w <= 700 => 6,
        w if w <= 800 => 7,
        _ => 8,
    };
    let weight_dropdown = DropdownWidget::new(weight_items).with_selected(weight_idx);
    let weight_id = weight_dropdown.id();

    // Ligatures are enabled if "liga" is in features without a "-liga" override.
    let has_liga = config.font.features.iter().any(|f| f == "liga");
    let has_neg_liga = config.font.features.iter().any(|f| f == "-liga");
    let ligatures_on = has_liga && !has_neg_liga;
    let liga_checkbox = CheckboxWidget::new("Enabled").with_checked(ligatures_on);
    let liga_id = liga_checkbox.id();

    let section = FormSection::new("Font")
        .with_row(FormRow::new("Size", Box::new(size_dropdown)))
        .with_row(FormRow::new("Weight", Box::new(weight_dropdown)))
        .with_row(FormRow::new("Ligatures", Box::new(liga_checkbox)));

    (section, size_id, weight_id, liga_id)
}

/// Behavior section: Paste Warning dropdown.
fn build_behavior_section(config: &Config) -> (FormSection, WidgetId) {
    let paste_items = vec!["Always".to_owned(), "Never".to_owned()];
    let paste_idx = match config.behavior.warn_on_paste {
        PasteWarning::Always | PasteWarning::Threshold(_) => 0,
        PasteWarning::Never => 1,
    };
    let paste_dropdown = DropdownWidget::new(paste_items).with_selected(paste_idx);
    let paste_id = paste_dropdown.id();

    let section = FormSection::new("Behavior")
        .with_row(FormRow::new("Paste Warning", Box::new(paste_dropdown)));

    (section, paste_id)
}

/// Terminal section: Cursor Style dropdown, Cursor Blink toggle.
fn build_terminal_section(config: &Config) -> (FormSection, WidgetId, WidgetId) {
    let style_items = vec!["Block".to_owned(), "Bar".to_owned(), "Underline".to_owned()];
    let style_idx = match config.terminal.cursor_style {
        CursorStyle::Block => 0,
        CursorStyle::Bar => 1,
        CursorStyle::Underline => 2,
    };
    let style_dropdown = DropdownWidget::new(style_items).with_selected(style_idx);
    let style_id = style_dropdown.id();

    let blink_toggle = ToggleWidget::new().with_on(config.terminal.cursor_blink);
    let blink_id = blink_toggle.id();

    let section = FormSection::new("Terminal")
        .with_row(FormRow::new("Cursor Style", Box::new(style_dropdown)))
        .with_row(FormRow::new("Cursor Blink", Box::new(blink_toggle)));

    (section, style_id, blink_id)
}

/// Bell section: Animation dropdown, Duration dropdown.
fn build_bell_section(config: &Config) -> (FormSection, WidgetId, WidgetId) {
    let anim_items = vec!["EaseOut".to_owned(), "Linear".to_owned(), "None".to_owned()];
    let anim_idx = match config.bell.animation {
        BellAnimation::EaseOut => 0,
        BellAnimation::Linear => 1,
        BellAnimation::None => 2,
    };
    let anim_dropdown = DropdownWidget::new(anim_items).with_selected(anim_idx);
    let anim_id = anim_dropdown.id();

    let dur_items: Vec<String> = BELL_DURATION_VALUES
        .iter()
        .map(|v| {
            if *v == 0 {
                "Off".to_owned()
            } else {
                format!("{v}ms")
            }
        })
        .collect();
    let dur_idx = BELL_DURATION_VALUES
        .iter()
        .position(|v| *v == config.bell.duration_ms)
        .unwrap_or(3); // Default to 150ms
    let dur_dropdown = DropdownWidget::new(dur_items).with_selected(dur_idx);
    let dur_id = dur_dropdown.id();

    let section = FormSection::new("Bell")
        .with_row(FormRow::new("Animation", Box::new(anim_dropdown)))
        .with_row(FormRow::new("Duration", Box::new(dur_dropdown)));

    (section, anim_id, dur_id)
}

#[cfg(test)]
mod tests;
