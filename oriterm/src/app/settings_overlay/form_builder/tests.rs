//! Tests for the settings dialog builder.

use std::collections::HashSet;

use oriterm_ui::theme::UiTheme;

use super::{SettingsIds, build_settings_dialog};
use crate::config::Config;

#[test]
fn dialog_builds_without_panic() {
    let config = Config::default();
    let theme = UiTheme::default();
    let (_content, _ids) = build_settings_dialog(&config, &theme);
}

#[test]
fn settings_ids_all_distinct() {
    let config = Config::default();
    let theme = UiTheme::default();
    let (_content, ids) = build_settings_dialog(&config, &theme);
    let all = collect_ids(&ids);
    // 22 fixed control IDs + N scheme card IDs.
    let expected = 22 + ids.scheme_card_ids.len();
    assert_eq!(all.len(), expected, "all widget IDs must be distinct");
}

#[test]
fn content_widget_has_valid_id() {
    let config = Config::default();
    let theme = UiTheme::default();
    let (content, _ids) = build_settings_dialog(&config, &theme);
    assert_ne!(content.id().raw(), 0);
}

#[test]
fn all_page_ids_are_set() {
    let config = Config::default();
    let theme = UiTheme::default();
    let (_content, ids) = build_settings_dialog(&config, &theme);
    let all = collect_ids(&ids);
    // Every ID must be non-placeholder.
    assert!(
        all.iter().all(|id| *id != 0),
        "no placeholder IDs should remain"
    );
}

#[test]
fn scheme_card_ids_captured() {
    let config = Config::default();
    let theme = UiTheme::default();
    let (_content, ids) = build_settings_dialog(&config, &theme);
    // Scheme cards are captured during colors page building.
    assert!(
        !ids.scheme_card_ids.is_empty(),
        "scheme card IDs must be captured"
    );
}

fn collect_ids(ids: &SettingsIds) -> HashSet<u64> {
    let mut set = HashSet::new();
    // Appearance.
    set.insert(ids.theme_dropdown.raw());
    set.insert(ids.opacity_slider.raw());
    set.insert(ids.blur_toggle.raw());
    // Colors — per-card IDs.
    for card_id in &ids.scheme_card_ids {
        set.insert(card_id.raw());
    }
    // Font.
    set.insert(ids.font_family_dropdown.raw());
    set.insert(ids.font_size_input.raw());
    set.insert(ids.font_weight_dropdown.raw());
    set.insert(ids.ligatures_toggle.raw());
    set.insert(ids.line_height_input.raw());
    // Terminal.
    set.insert(ids.cursor_picker.raw());
    set.insert(ids.cursor_blink_toggle.raw());
    set.insert(ids.scrollback_input.raw());
    set.insert(ids.shell_input.raw());
    set.insert(ids.paste_warning_dropdown.raw());
    // Window.
    set.insert(ids.tab_bar_position_dropdown.raw());
    set.insert(ids.grid_padding_input.raw());
    set.insert(ids.restore_session_toggle.raw());
    set.insert(ids.initial_columns_input.raw());
    set.insert(ids.initial_rows_input.raw());
    // Bell.
    set.insert(ids.bell_animation_dropdown.raw());
    set.insert(ids.bell_duration_dropdown.raw());
    // Rendering.
    set.insert(ids.gpu_backend_dropdown.raw());
    set.insert(ids.subpixel_toggle.raw());
    set
}
