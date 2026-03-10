//! Tests for the settings form builder.

use std::collections::HashSet;

use super::{SettingsIds, build_settings_form};
use crate::config::Config;

#[test]
fn default_config_produces_five_sections() {
    let config = Config::default();
    let (form, _ids) = build_settings_form(&config);
    assert_eq!(form.sections().len(), 5);
}

#[test]
fn section_names_match_expected() {
    let config = Config::default();
    let (form, _ids) = build_settings_form(&config);
    let names: Vec<&str> = form.sections().iter().map(|s| s.title()).collect();
    assert_eq!(
        names,
        ["Appearance", "Font", "Behavior", "Terminal", "Bell"]
    );
}

#[test]
fn total_row_count_is_ten() {
    let config = Config::default();
    let (form, _ids) = build_settings_form(&config);
    let total: usize = form.sections().iter().map(|s| s.rows().len()).sum();
    assert_eq!(total, 10);
}

#[test]
fn settings_ids_all_distinct() {
    let config = Config::default();
    let (_form, ids) = build_settings_form(&config);
    let all = collect_ids(&ids);
    assert_eq!(all.len(), 10, "all 10 widget IDs must be distinct");
}

#[test]
fn appearance_section_has_two_rows() {
    let config = Config::default();
    let (form, _ids) = build_settings_form(&config);
    assert_eq!(form.sections()[0].rows().len(), 2);
}

#[test]
fn font_section_has_three_rows() {
    let config = Config::default();
    let (form, _ids) = build_settings_form(&config);
    assert_eq!(form.sections()[1].rows().len(), 3);
}

#[test]
fn behavior_section_has_one_row() {
    let config = Config::default();
    let (form, _ids) = build_settings_form(&config);
    assert_eq!(form.sections()[2].rows().len(), 1);
}

#[test]
fn terminal_section_has_two_rows() {
    let config = Config::default();
    let (form, _ids) = build_settings_form(&config);
    assert_eq!(form.sections()[3].rows().len(), 2);
}

#[test]
fn bell_section_has_two_rows() {
    let config = Config::default();
    let (form, _ids) = build_settings_form(&config);
    assert_eq!(form.sections()[4].rows().len(), 2);
}

#[test]
fn all_sections_start_expanded() {
    let config = Config::default();
    let (form, _ids) = build_settings_form(&config);
    for section in form.sections() {
        assert!(
            section.is_expanded(),
            "{} should start expanded",
            section.title()
        );
    }
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
