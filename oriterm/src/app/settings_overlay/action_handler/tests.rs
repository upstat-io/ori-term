//! Tests for the settings action handler.

use oriterm_ui::widget_id::WidgetId;
use oriterm_ui::widgets::WidgetAction;

use super::handle_settings_action;
use crate::app::settings_overlay::form_builder::{SettingsIds, build_settings_dialog};
use crate::config::Config;

fn default_ids() -> (Config, SettingsIds) {
    let config = Config::default();
    let theme = oriterm_ui::theme::UiTheme::default();
    let (_content, ids, _footer_ids) = build_settings_dialog(&config, &theme, 0, 1.0, 1.0, None);
    (config, ids)
}

// Appearance page tests.

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
fn opacity_value_changed_updates_config() {
    let (mut config, ids) = default_ids();
    let action = WidgetAction::ValueChanged {
        id: ids.opacity_slider,
        value: 75.0,
    };
    assert!(handle_settings_action(&action, &ids, &mut config));
    assert!((config.window.opacity - 0.75).abs() < f32::EPSILON);
}

#[test]
fn blur_toggled_updates_config() {
    let (mut config, ids) = default_ids();
    let action = WidgetAction::Toggled {
        id: ids.blur_toggle,
        value: false,
    };
    assert!(handle_settings_action(&action, &ids, &mut config));
    assert!(!config.window.blur);
}

#[test]
fn unfocused_opacity_value_changed_updates_config() {
    let (mut config, ids) = default_ids();
    let action = WidgetAction::ValueChanged {
        id: ids.unfocused_opacity_slider,
        value: 70.0,
    };
    assert!(handle_settings_action(&action, &ids, &mut config));
    assert!((config.window.unfocused_opacity - 0.7).abs() < f32::EPSILON);
}

#[test]
fn decorations_dropdown_updates_config() {
    let (mut config, ids) = default_ids();
    let action = WidgetAction::Selected {
        id: ids.decorations_dropdown,
        index: 1, // Full
    };
    assert!(handle_settings_action(&action, &ids, &mut config));
    assert_eq!(config.window.decorations, crate::config::Decorations::Full);
}

#[test]
fn tab_bar_style_default_preserves_position() {
    let (mut config, ids) = default_ids();
    config.window.tab_bar_position = crate::config::TabBarPosition::Bottom;
    let action = WidgetAction::Selected {
        id: ids.tab_bar_style_dropdown,
        index: 0, // Default
    };
    assert!(handle_settings_action(&action, &ids, &mut config));
    assert_eq!(
        config.window.tab_bar_style,
        crate::config::TabBarStyle::Default
    );
    assert_eq!(
        config.window.tab_bar_position,
        crate::config::TabBarPosition::Bottom,
        "selecting Default style must not change a non-hidden position"
    );
}

#[test]
fn tab_bar_style_compact_preserves_position() {
    let (mut config, ids) = default_ids();
    config.window.tab_bar_position = crate::config::TabBarPosition::Bottom;
    let action = WidgetAction::Selected {
        id: ids.tab_bar_style_dropdown,
        index: 1, // Compact
    };
    assert!(handle_settings_action(&action, &ids, &mut config));
    assert_eq!(
        config.window.tab_bar_style,
        crate::config::TabBarStyle::Compact
    );
    assert_eq!(
        config.window.tab_bar_position,
        crate::config::TabBarPosition::Bottom,
    );
}

#[test]
fn tab_bar_style_hidden_maps_to_position_hidden() {
    let (mut config, ids) = default_ids();
    let action = WidgetAction::Selected {
        id: ids.tab_bar_style_dropdown,
        index: 2, // Hidden
    };
    assert!(handle_settings_action(&action, &ids, &mut config));
    assert_eq!(
        config.window.tab_bar_position,
        crate::config::TabBarPosition::Hidden
    );
}

#[test]
fn tab_bar_style_default_restores_from_hidden() {
    let (mut config, ids) = default_ids();
    config.window.tab_bar_position = crate::config::TabBarPosition::Hidden;
    let action = WidgetAction::Selected {
        id: ids.tab_bar_style_dropdown,
        index: 0, // Default
    };
    assert!(handle_settings_action(&action, &ids, &mut config));
    assert_eq!(
        config.window.tab_bar_position,
        crate::config::TabBarPosition::Top,
        "selecting Default from Hidden should restore to Top"
    );
}

#[test]
#[cfg(target_os = "macos")]
fn decorations_dropdown_buttonless_on_macos() {
    let (mut config, ids) = default_ids();
    let action = WidgetAction::Selected {
        id: ids.decorations_dropdown,
        index: 3, // Buttonless (macOS only)
    };
    assert!(handle_settings_action(&action, &ids, &mut config));
    assert_eq!(
        config.window.decorations,
        crate::config::Decorations::Buttonless
    );
}

#[test]
fn decorations_dropdown_transparent_roundtrip() {
    let (mut config, ids) = default_ids();
    let action = WidgetAction::Selected {
        id: ids.decorations_dropdown,
        index: 2, // Transparent
    };
    assert!(handle_settings_action(&action, &ids, &mut config));
    assert_eq!(
        config.window.decorations,
        crate::config::Decorations::Transparent
    );
}

// Colors page tests.

#[test]
fn scheme_card_selected_updates_scheme() {
    let (mut config, ids) = default_ids();
    assert!(
        !ids.scheme_card_ids.is_empty(),
        "scheme cards must be captured"
    );
    // Click the second scheme card.
    let action = WidgetAction::Selected {
        id: ids.scheme_card_ids[1],
        index: 1,
    };
    assert!(handle_settings_action(&action, &ids, &mut config));
    let names = crate::scheme::builtin_names();
    assert_eq!(config.colors.scheme, names[1]);
}

// Font page tests.

#[test]
fn font_family_selected_updates_config() {
    let (mut config, ids) = default_ids();
    // Index 0 = "Default (System)" → None.
    let action = WidgetAction::Selected {
        id: ids.font_family_dropdown,
        index: 0,
    };
    assert!(handle_settings_action(&action, &ids, &mut config));
    assert!(config.font.family.is_none());

    // Index 1 = "JetBrains Mono" → Some.
    let action = WidgetAction::Selected {
        id: ids.font_family_dropdown,
        index: 1,
    };
    assert!(handle_settings_action(&action, &ids, &mut config));
    assert_eq!(config.font.family.as_deref(), Some("JetBrains Mono"));
}

#[test]
fn font_size_value_changed_updates_config() {
    let (mut config, ids) = default_ids();
    let action = WidgetAction::ValueChanged {
        id: ids.font_size_input,
        value: 16.0,
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
    let action = WidgetAction::Toggled {
        id: ids.ligatures_toggle,
        value: false,
    };
    assert!(handle_settings_action(&action, &ids, &mut config));
    assert!(!config.font.features.iter().any(|f| f == "liga"));
    assert!(config.font.features.iter().any(|f| f == "-liga"));
}

#[test]
fn ligatures_toggled_on_adds_liga() {
    let (mut config, ids) = default_ids();
    config.font.features.retain(|f| f != "liga");
    config.font.features.push("-liga".to_owned());

    let action = WidgetAction::Toggled {
        id: ids.ligatures_toggle,
        value: true,
    };
    assert!(handle_settings_action(&action, &ids, &mut config));
    assert!(config.font.features.iter().any(|f| f == "liga"));
    assert!(!config.font.features.iter().any(|f| f == "-liga"));
}

#[test]
fn line_height_value_changed_updates_config() {
    let (mut config, ids) = default_ids();
    let action = WidgetAction::ValueChanged {
        id: ids.line_height_input,
        value: 1.5,
    };
    assert!(handle_settings_action(&action, &ids, &mut config));
    assert!((config.font.line_height - 1.5).abs() < f32::EPSILON);
}

// Terminal page tests.

#[test]
fn cursor_picker_selected_updates_style() {
    let (mut config, ids) = default_ids();
    let action = WidgetAction::Selected {
        id: ids.cursor_picker,
        index: 1, // Bar
    };
    assert!(handle_settings_action(&action, &ids, &mut config));
    assert_eq!(
        config.terminal.cursor_style,
        crate::config::CursorStyle::Bar
    );
}

#[test]
fn scrollback_value_changed_updates_config() {
    let (mut config, ids) = default_ids();
    let action = WidgetAction::ValueChanged {
        id: ids.scrollback_input,
        value: 50_000.0,
    };
    assert!(handle_settings_action(&action, &ids, &mut config));
    assert_eq!(config.terminal.scrollback, 50_000);
}

#[test]
fn shell_text_changed_updates_config() {
    let (mut config, ids) = default_ids();
    let action = WidgetAction::TextChanged {
        id: ids.shell_input,
        text: "/usr/bin/fish".to_owned(),
    };
    assert!(handle_settings_action(&action, &ids, &mut config));
    assert_eq!(config.terminal.shell.as_deref(), Some("/usr/bin/fish"));

    // Empty text → None.
    let action = WidgetAction::TextChanged {
        id: ids.shell_input,
        text: String::new(),
    };
    assert!(handle_settings_action(&action, &ids, &mut config));
    assert!(config.terminal.shell.is_none());
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

// Window page tests.

#[test]
fn tab_bar_position_selected_updates_config() {
    let (mut config, ids) = default_ids();
    let action = WidgetAction::Selected {
        id: ids.tab_bar_position_dropdown,
        index: 2, // Hidden
    };
    assert!(handle_settings_action(&action, &ids, &mut config));
    assert_eq!(
        config.window.tab_bar_position,
        crate::config::TabBarPosition::Hidden
    );
}

#[test]
fn grid_padding_value_changed_updates_config() {
    let (mut config, ids) = default_ids();
    let action = WidgetAction::ValueChanged {
        id: ids.grid_padding_input,
        value: 8.0,
    };
    assert!(handle_settings_action(&action, &ids, &mut config));
    assert!((config.window.grid_padding - 8.0).abs() < f32::EPSILON);
}

#[test]
fn gpu_backend_selected_updates_config() {
    let (mut config, ids) = default_ids();
    let action = WidgetAction::Selected {
        id: ids.gpu_backend_dropdown,
        index: 1, // Second entry: platform-dependent (Vulkan on Linux, Metal on macOS, etc.)
    };
    assert!(handle_settings_action(&action, &ids, &mut config));
    let expected = crate::config::GpuBackend::available()[1].0;
    assert_eq!(config.rendering.gpu_backend, expected);
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

// Font Advanced page tests.

#[test]
fn hinting_dropdown_auto_sets_none() {
    let (mut config, ids) = default_ids();
    config.font.hinting = Some("full".to_owned());
    let action = WidgetAction::Selected {
        id: ids.hinting_dropdown,
        index: 0,
    };
    assert!(handle_settings_action(&action, &ids, &mut config));
    assert_eq!(config.font.hinting, None);
}

#[test]
fn hinting_dropdown_full() {
    let (mut config, ids) = default_ids();
    let action = WidgetAction::Selected {
        id: ids.hinting_dropdown,
        index: 1,
    };
    assert!(handle_settings_action(&action, &ids, &mut config));
    assert_eq!(config.font.hinting.as_deref(), Some("full"));
}

#[test]
fn hinting_dropdown_none() {
    let (mut config, ids) = default_ids();
    let action = WidgetAction::Selected {
        id: ids.hinting_dropdown,
        index: 2,
    };
    assert!(handle_settings_action(&action, &ids, &mut config));
    assert_eq!(config.font.hinting.as_deref(), Some("none"));
}

#[test]
fn subpixel_aa_dropdown_auto_sets_none() {
    let (mut config, ids) = default_ids();
    config.font.subpixel_mode = Some("rgb".to_owned());
    let action = WidgetAction::Selected {
        id: ids.subpixel_aa_dropdown,
        index: 0,
    };
    assert!(handle_settings_action(&action, &ids, &mut config));
    assert_eq!(config.font.subpixel_mode, None);
}

#[test]
fn subpixel_aa_dropdown_rgb() {
    let (mut config, ids) = default_ids();
    let action = WidgetAction::Selected {
        id: ids.subpixel_aa_dropdown,
        index: 1,
    };
    assert!(handle_settings_action(&action, &ids, &mut config));
    assert_eq!(config.font.subpixel_mode.as_deref(), Some("rgb"));
}

#[test]
fn subpixel_aa_dropdown_bgr() {
    let (mut config, ids) = default_ids();
    let action = WidgetAction::Selected {
        id: ids.subpixel_aa_dropdown,
        index: 2,
    };
    assert!(handle_settings_action(&action, &ids, &mut config));
    assert_eq!(config.font.subpixel_mode.as_deref(), Some("bgr"));
}

#[test]
fn subpixel_aa_dropdown_none_grayscale() {
    let (mut config, ids) = default_ids();
    let action = WidgetAction::Selected {
        id: ids.subpixel_aa_dropdown,
        index: 3,
    };
    assert!(handle_settings_action(&action, &ids, &mut config));
    assert_eq!(config.font.subpixel_mode.as_deref(), Some("none"));
}

#[test]
fn subpixel_positioning_dropdown_auto() {
    let (mut config, ids) = default_ids();
    config.font.subpixel_positioning = Some(true);
    let action = WidgetAction::Selected {
        id: ids.subpixel_positioning_dropdown,
        index: 0,
    };
    assert!(handle_settings_action(&action, &ids, &mut config));
    assert_eq!(config.font.subpixel_positioning, None);
}

#[test]
fn subpixel_positioning_dropdown_quarter_pixel() {
    let (mut config, ids) = default_ids();
    let action = WidgetAction::Selected {
        id: ids.subpixel_positioning_dropdown,
        index: 1,
    };
    assert!(handle_settings_action(&action, &ids, &mut config));
    assert_eq!(config.font.subpixel_positioning, Some(true));
}

#[test]
fn subpixel_positioning_dropdown_off() {
    let (mut config, ids) = default_ids();
    let action = WidgetAction::Selected {
        id: ids.subpixel_positioning_dropdown,
        index: 2,
    };
    assert!(handle_settings_action(&action, &ids, &mut config));
    assert_eq!(config.font.subpixel_positioning, Some(false));
}

#[test]
fn atlas_filtering_dropdown_auto() {
    let (mut config, ids) = default_ids();
    config.font.atlas_filtering = Some("linear".to_owned());
    let action = WidgetAction::Selected {
        id: ids.atlas_filtering_dropdown,
        index: 0,
    };
    assert!(handle_settings_action(&action, &ids, &mut config));
    assert_eq!(config.font.atlas_filtering, None);
}

#[test]
fn atlas_filtering_dropdown_linear() {
    let (mut config, ids) = default_ids();
    let action = WidgetAction::Selected {
        id: ids.atlas_filtering_dropdown,
        index: 1,
    };
    assert!(handle_settings_action(&action, &ids, &mut config));
    assert_eq!(config.font.atlas_filtering.as_deref(), Some("linear"));
}

#[test]
fn atlas_filtering_dropdown_nearest() {
    let (mut config, ids) = default_ids();
    let action = WidgetAction::Selected {
        id: ids.atlas_filtering_dropdown,
        index: 2,
    };
    assert!(handle_settings_action(&action, &ids, &mut config));
    assert_eq!(config.font.atlas_filtering.as_deref(), Some("nearest"));
}

#[test]
fn subpixel_toggle_removed() {
    // Regression guard: subpixel_toggle was removed from SettingsIds.
    // A random Toggled action should not match any rendering handler.
    let (mut config, ids) = default_ids();
    let action = WidgetAction::Toggled {
        id: WidgetId::next(),
        value: true,
    };
    assert!(!handle_settings_action(&action, &ids, &mut config));
}
