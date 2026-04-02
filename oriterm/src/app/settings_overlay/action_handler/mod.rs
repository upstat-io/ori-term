//! Settings action dispatch — maps widget actions to config updates.
//!
//! When a settings control emits a `WidgetAction`, this module matches
//! it against the known `SettingsIds` and updates the corresponding
//! config field.

use oriterm_ui::widgets::WidgetAction;

use crate::config::{
    BellAnimation, Config, CursorStyle, Decorations, GpuBackend, PasteWarning, TabBarPosition,
    TabBarStyle,
};

use super::form_builder::{BELL_DURATION_VALUES, FONT_FAMILIES, SettingsIds};

/// Weight values matching the dropdown items in the font page builder.
const FONT_WEIGHTS: [u16; 9] = [100, 200, 300, 400, 500, 600, 700, 800, 900];

/// Matches a `WidgetAction` against settings controls and updates config.
///
/// Returns `true` if the config was modified (caller should persist + apply).
pub(in crate::app) fn handle_settings_action(
    action: &WidgetAction,
    ids: &SettingsIds,
    config: &mut Config,
) -> bool {
    handle_appearance(action, ids, config)
        || handle_colors(action, ids, config)
        || handle_font(action, ids, config)
        || handle_font_advanced(action, ids, config)
        || handle_terminal(action, ids, config)
        || handle_window(action, ids, config)
        || handle_bell(action, ids, config)
        || handle_rendering(action, ids, config)
}

/// Appearance page: theme, opacity, blur, unfocused opacity, decorations, tab bar style.
fn handle_appearance(action: &WidgetAction, ids: &SettingsIds, config: &mut Config) -> bool {
    match action {
        WidgetAction::Selected { id, index } if *id == ids.theme_dropdown => {
            let names = crate::scheme::builtin_names();
            if let Some(name) = names.get(*index) {
                (*name).clone_into(&mut config.colors.scheme);
            }
            true
        }
        WidgetAction::ValueChanged { id, value } if *id == ids.opacity_slider => {
            config.window.opacity = (*value / 100.0).clamp(0.0, 1.0);
            true
        }
        WidgetAction::Toggled { id, value } if *id == ids.blur_toggle => {
            config.window.blur = *value;
            true
        }
        WidgetAction::ValueChanged { id, value } if *id == ids.unfocused_opacity_slider => {
            config.window.unfocused_opacity = (*value / 100.0).clamp(0.3, 1.0);
            true
        }
        WidgetAction::Selected { id, index } if *id == ids.decorations_dropdown => {
            config.window.decorations = match index {
                0 => Decorations::None,
                1 => Decorations::Full,
                #[cfg(target_os = "macos")]
                3 => Decorations::Buttonless,
                _ => Decorations::Transparent,
            };
            true
        }
        WidgetAction::Selected { id, index } if *id == ids.tab_bar_style_dropdown => {
            // 0=Default, 1=Compact, 2=Hidden.
            // "Hidden" maps to TabBarPosition::Hidden to avoid duplicate hidden state.
            match index {
                0 => {
                    config.window.tab_bar_style = TabBarStyle::Default;
                    if config.window.tab_bar_position == TabBarPosition::Hidden {
                        config.window.tab_bar_position = TabBarPosition::Top;
                    }
                }
                1 => {
                    config.window.tab_bar_style = TabBarStyle::Compact;
                    if config.window.tab_bar_position == TabBarPosition::Hidden {
                        config.window.tab_bar_position = TabBarPosition::Top;
                    }
                }
                _ => {
                    config.window.tab_bar_position = TabBarPosition::Hidden;
                }
            }
            true
        }
        _ => false,
    }
}

/// Colors page: scheme card selection.
fn handle_colors(action: &WidgetAction, ids: &SettingsIds, config: &mut Config) -> bool {
    match action {
        WidgetAction::Selected { id, index }
            if ids.scheme_card_ids.iter().any(|card_id| card_id == id) =>
        {
            let names = crate::scheme::builtin_names();
            if let Some(name) = names.get(*index) {
                (*name).clone_into(&mut config.colors.scheme);
            }
            true
        }
        _ => false,
    }
}

/// Font page: family, size, weight, ligatures, line height.
fn handle_font(action: &WidgetAction, ids: &SettingsIds, config: &mut Config) -> bool {
    match action {
        WidgetAction::Selected { id, index } if *id == ids.font_family_dropdown => {
            config.font.family = if *index == 0 {
                None
            } else {
                FONT_FAMILIES.get(*index).map(|s| (*s).to_owned())
            };
            true
        }
        WidgetAction::ValueChanged { id, value } if *id == ids.font_size_input => {
            config.font.size = value.clamp(8.0, 32.0);
            true
        }
        WidgetAction::Selected { id, index } if *id == ids.font_weight_dropdown => {
            if let Some(&w) = FONT_WEIGHTS.get(*index) {
                config.font.weight = w;
            }
            true
        }
        WidgetAction::Toggled { id, value } if *id == ids.ligatures_toggle => {
            if *value {
                if !config.font.features.iter().any(|f| f == "liga") {
                    config.font.features.push("liga".to_owned());
                }
                config.font.features.retain(|f| f != "-liga");
            } else {
                config.font.features.retain(|f| f != "liga");
                if !config.font.features.iter().any(|f| f == "-liga") {
                    config.font.features.push("-liga".to_owned());
                }
            }
            true
        }
        WidgetAction::ValueChanged { id, value } if *id == ids.line_height_input => {
            config.font.line_height = value.clamp(0.8, 2.0);
            true
        }
        _ => false,
    }
}

/// Font page — Advanced section: hinting, subpixel AA, subpixel positioning, atlas filtering.
fn handle_font_advanced(action: &WidgetAction, ids: &SettingsIds, config: &mut Config) -> bool {
    match action {
        WidgetAction::Selected { id, index } if *id == ids.hinting_dropdown => {
            config.font.hinting = match index {
                0 => None,
                1 => Some("full".to_owned()),
                _ => Some("none".to_owned()),
            };
            true
        }
        WidgetAction::Selected { id, index } if *id == ids.subpixel_aa_dropdown => {
            config.font.subpixel_mode = match index {
                0 => None,
                1 => Some("rgb".to_owned()),
                2 => Some("bgr".to_owned()),
                _ => Some("none".to_owned()),
            };
            true
        }
        WidgetAction::Selected { id, index } if *id == ids.subpixel_positioning_dropdown => {
            config.font.subpixel_positioning = match index {
                0 => None,
                1 => Some(true),
                _ => Some(false),
            };
            true
        }
        WidgetAction::Selected { id, index } if *id == ids.atlas_filtering_dropdown => {
            config.font.atlas_filtering = match index {
                0 => None,
                1 => Some("linear".to_owned()),
                _ => Some("nearest".to_owned()),
            };
            true
        }
        _ => false,
    }
}

/// Terminal page: cursor, scrollback, shell, paste warning.
fn handle_terminal(action: &WidgetAction, ids: &SettingsIds, config: &mut Config) -> bool {
    match action {
        WidgetAction::Selected { id, index } if *id == ids.cursor_picker => {
            config.terminal.cursor_style = match index {
                0 => CursorStyle::Block,
                1 => CursorStyle::Bar,
                _ => CursorStyle::Underline,
            };
            true
        }
        WidgetAction::Toggled { id, value } if *id == ids.cursor_blink_toggle => {
            config.terminal.cursor_blink = *value;
            true
        }
        WidgetAction::Toggled { id, value } if *id == ids.cursor_blink_fade_toggle => {
            config.terminal.cursor_blink_fade = *value;
            true
        }
        WidgetAction::ValueChanged { id, value } if *id == ids.scrollback_input => {
            config.terminal.scrollback = (*value as usize).min(100_000);
            true
        }
        WidgetAction::TextChanged { id, text } if *id == ids.shell_input => {
            config.terminal.shell = if text.is_empty() {
                None
            } else {
                Some(text.clone())
            };
            true
        }
        WidgetAction::Selected { id, index } if *id == ids.paste_warning_dropdown => {
            config.behavior.warn_on_paste = match index {
                0 => PasteWarning::Always,
                _ => PasteWarning::Never,
            };
            true
        }
        _ => false,
    }
}

/// Window page: tab bar, padding, startup.
fn handle_window(action: &WidgetAction, ids: &SettingsIds, config: &mut Config) -> bool {
    match action {
        WidgetAction::Selected { id, index } if *id == ids.tab_bar_position_dropdown => {
            config.window.tab_bar_position = match index {
                0 => TabBarPosition::Top,
                1 => TabBarPosition::Bottom,
                _ => TabBarPosition::Hidden,
            };
            true
        }
        WidgetAction::ValueChanged { id, value } if *id == ids.grid_padding_input => {
            config.window.grid_padding = value.clamp(0.0, 40.0);
            true
        }
        WidgetAction::Toggled { id, value } if *id == ids.restore_session_toggle => {
            config.window.restore_session = *value;
            true
        }
        WidgetAction::ValueChanged { id, value } if *id == ids.initial_columns_input => {
            config.window.columns = (*value as usize).clamp(40, 400);
            true
        }
        WidgetAction::ValueChanged { id, value } if *id == ids.initial_rows_input => {
            config.window.rows = (*value as usize).clamp(10, 100);
            true
        }
        _ => false,
    }
}

/// Bell page: animation, duration.
fn handle_bell(action: &WidgetAction, ids: &SettingsIds, config: &mut Config) -> bool {
    match action {
        WidgetAction::Selected { id, index } if *id == ids.bell_animation_dropdown => {
            config.bell.animation = match index {
                0 => BellAnimation::EaseOut,
                1 => BellAnimation::Linear,
                _ => BellAnimation::None,
            };
            true
        }
        WidgetAction::Selected { id, index } if *id == ids.bell_duration_dropdown => {
            if let Some(&v) = BELL_DURATION_VALUES.get(*index) {
                config.bell.duration_ms = v;
            }
            true
        }
        _ => false,
    }
}

/// Rendering page: GPU backend.
fn handle_rendering(action: &WidgetAction, ids: &SettingsIds, config: &mut Config) -> bool {
    match action {
        WidgetAction::Selected { id, index } if *id == ids.gpu_backend_dropdown => {
            config.rendering.gpu_backend = GpuBackend::available()
                .get(*index)
                .map_or(GpuBackend::Auto, |(b, _)| *b);
            true
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests;
