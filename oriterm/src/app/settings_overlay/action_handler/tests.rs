//! Tests for the settings action handler.

use oriterm_ui::widget_id::WidgetId;
use oriterm_ui::widgets::WidgetAction;

use super::handle_settings_action;
use crate::app::settings_overlay::form_builder::{SettingsIds, build_settings_form};
use crate::config::Config;

fn default_ids() -> (Config, SettingsIds) {
    let config = Config::default();
    let (_form, ids) = build_settings_form(&config);
    (config, ids)
}

#[test]
fn theme_selected_updates_scheme() {
    let (mut config, ids) = default_ids();
    let action = WidgetAction::Selected {
        id: ids.theme_dropdown,
        index: 1,
    };
    assert!(handle_settings_action(&action, &ids, &mut config));
    let names = crate::scheme::builtin_names();
    assert_eq!(config.colors.scheme, names[1]);
}

#[test]
fn opacity_selected_updates_config() {
    let (mut config, ids) = default_ids();
    let action = WidgetAction::Selected {
        id: ids.opacity_dropdown,
        index: 2, // 50%
    };
    assert!(handle_settings_action(&action, &ids, &mut config));
    assert!((config.window.opacity - 0.5).abs() < f32::EPSILON);
}

#[test]
fn font_size_selected_updates_config() {
    let (mut config, ids) = default_ids();
    let action = WidgetAction::Selected {
        id: ids.font_size_dropdown,
        index: 8, // 16.0
    };
    assert!(handle_settings_action(&action, &ids, &mut config));
    assert!((config.font.size - 16.0).abs() < f32::EPSILON);
}

#[test]
fn font_weight_selected_updates_config() {
    let (mut config, ids) = default_ids();
    let action = WidgetAction::Selected {
        id: ids.font_weight_dropdown,
        index: 6, // 700
    };
    assert!(handle_settings_action(&action, &ids, &mut config));
    assert_eq!(config.font.weight, 700);
}

#[test]
fn ligatures_toggled_off_removes_liga() {
    let (mut config, ids) = default_ids();
    // Default has "liga" in features.
    let action = WidgetAction::Toggled {
        id: ids.ligatures_checkbox,
        value: false,
    };
    assert!(handle_settings_action(&action, &ids, &mut config));
    assert!(!config.font.features.iter().any(|f| f == "liga"));
    assert!(config.font.features.iter().any(|f| f == "-liga"));
}

#[test]
fn ligatures_toggled_on_adds_liga() {
    let (mut config, ids) = default_ids();
    // Start with liga removed.
    config.font.features.retain(|f| f != "liga");
    config.font.features.push("-liga".to_owned());

    let action = WidgetAction::Toggled {
        id: ids.ligatures_checkbox,
        value: true,
    };
    assert!(handle_settings_action(&action, &ids, &mut config));
    assert!(config.font.features.iter().any(|f| f == "liga"));
    assert!(!config.font.features.iter().any(|f| f == "-liga"));
}

#[test]
fn cursor_blink_toggled_updates_config() {
    let (mut config, ids) = default_ids();
    let action = WidgetAction::Toggled {
        id: ids.cursor_blink_toggle,
        value: false,
    };
    assert!(handle_settings_action(&action, &ids, &mut config));
    assert!(!config.terminal.cursor_blink);
}

#[test]
fn bell_duration_selected_updates_config() {
    let (mut config, ids) = default_ids();
    let action = WidgetAction::Selected {
        id: ids.bell_duration_dropdown,
        index: 5, // 300ms
    };
    assert!(handle_settings_action(&action, &ids, &mut config));
    assert_eq!(config.bell.duration_ms, 300);
}

#[test]
fn unknown_widget_id_returns_false() {
    let (mut config, ids) = default_ids();
    let action = WidgetAction::ValueChanged {
        id: WidgetId::next(),
        value: 1.0,
    };
    assert!(!handle_settings_action(&action, &ids, &mut config));
}

#[test]
fn paste_warning_selected_updates_config() {
    let (mut config, ids) = default_ids();
    let action = WidgetAction::Selected {
        id: ids.paste_warning_dropdown,
        index: 1, // Never
    };
    assert!(handle_settings_action(&action, &ids, &mut config));
    assert_eq!(
        config.behavior.warn_on_paste,
        crate::config::PasteWarning::Never
    );
}
