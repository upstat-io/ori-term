//! Tests for the settings dialog builder.

use std::collections::HashSet;

use oriterm_ui::theme::UiTheme;

use super::{SettingsIds, build_settings_dialog};
use crate::config::Config;

#[test]
fn dialog_builds_without_panic() {
    let config = Config::default();
    let theme = UiTheme::default();
    let (_content, _ids) = build_settings_dialog(&config, &theme, 0);
}

#[test]
fn settings_ids_all_distinct() {
    let config = Config::default();
    let theme = UiTheme::default();
    let (_content, ids) = build_settings_dialog(&config, &theme, 0);
    let all = collect_ids(&ids);
    // 26 fixed control IDs (25 controls + sidebar) + N scheme card IDs.
    let expected = 26 + ids.scheme_card_ids.len();
    assert_eq!(all.len(), expected, "all widget IDs must be distinct");
}

#[test]
fn content_widget_has_valid_id() {
    let config = Config::default();
    let theme = UiTheme::default();
    let (content, _ids) = build_settings_dialog(&config, &theme, 0);
    assert_ne!(content.id().raw(), 0);
}

#[test]
fn all_page_ids_are_set() {
    let config = Config::default();
    let theme = UiTheme::default();
    let (_content, ids) = build_settings_dialog(&config, &theme, 0);
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
    let (_content, ids) = build_settings_dialog(&config, &theme, 0);
    // Scheme cards are captured during colors page building.
    assert!(
        !ids.scheme_card_ids.is_empty(),
        "scheme card IDs must be captured"
    );
}

/// Regression test for TPR-11-001: sidebar_id must be captured so
/// `dispatch_dialog_settings_action` can gate `active_page` updates.
#[test]
fn sidebar_id_captured() {
    let config = Config::default();
    let theme = UiTheme::default();
    let (_content, ids) = build_settings_dialog(&config, &theme, 0);
    assert_ne!(
        ids.sidebar_id,
        oriterm_ui::widget_id::WidgetId::placeholder(),
        "sidebar_id must be non-placeholder"
    );
    // Must be distinct from any scheme card ID.
    assert!(
        !ids.scheme_card_ids.contains(&ids.sidebar_id),
        "sidebar_id must not collide with scheme card IDs"
    );
}

fn collect_ids(ids: &SettingsIds) -> HashSet<u64> {
    let mut set = HashSet::new();
    // Navigation.
    set.insert(ids.sidebar_id.raw());
    // Appearance.
    set.insert(ids.theme_dropdown.raw());
    set.insert(ids.opacity_slider.raw());
    set.insert(ids.blur_toggle.raw());
    set.insert(ids.unfocused_opacity_slider.raw());
    set.insert(ids.decorations_dropdown.raw());
    set.insert(ids.tab_bar_style_dropdown.raw());
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
