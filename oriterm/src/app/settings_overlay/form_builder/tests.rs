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
    assert_eq!(all.len(), 10, "all 10 widget IDs must be distinct");
}

#[test]
fn content_widget_has_valid_id() {
    let config = Config::default();
    let theme = UiTheme::default();
    let (content, _ids) = build_settings_dialog(&config, &theme);
    // Content widget should have a non-placeholder ID.
    assert_ne!(content.id().raw(), 0);
}

fn collect_ids(ids: &SettingsIds) -> HashSet<u64> {
    let mut set = HashSet::new();
    set.insert(ids.theme_dropdown.raw());
    set.insert(ids.opacity_dropdown.raw());
    set.insert(ids.font_size_dropdown.raw());
    set.insert(ids.font_weight_dropdown.raw());
    set.insert(ids.ligatures_checkbox.raw());
    set.insert(ids.paste_warning_dropdown.raw());
    set.insert(ids.cursor_style_dropdown.raw());
    set.insert(ids.cursor_blink_toggle.raw());
    set.insert(ids.bell_animation_dropdown.raw());
    set.insert(ids.bell_duration_dropdown.raw());
    set
}
