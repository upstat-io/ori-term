//! Settings action dispatch — maps widget actions to config updates.
//!
//! When a settings control emits a `WidgetAction`, this module matches
//! it against the known `SettingsIds` and updates the corresponding
//! config field.

use oriterm_ui::widgets::WidgetAction;

use crate::config::{BellAnimation, Config, CursorStyle, PasteWarning};

use super::form_builder::{BELL_DURATION_VALUES, FONT_SIZE_VALUES, OPACITY_VALUES, SettingsIds};

/// Weight values matching the dropdown items in `build_font_section`.
const FONT_WEIGHTS: [u16; 9] = [100, 200, 300, 400, 500, 600, 700, 800, 900];

/// Matches a `WidgetAction` against settings controls and updates config.
///
/// Returns `true` if the config was modified (caller should persist + apply).
pub(in crate::app) fn handle_settings_action(
    action: &WidgetAction,
    ids: &SettingsIds,
    config: &mut Config,
) -> bool {
    match action {
        WidgetAction::Selected { id, index } if *id == ids.theme_dropdown => {
            let names = crate::scheme::builtin_names();
            if let Some(name) = names.get(*index) {
                (*name).clone_into(&mut config.colors.scheme);
            }
            true
        }
        WidgetAction::Selected { id, index } if *id == ids.opacity_dropdown => {
            if let Some(&v) = OPACITY_VALUES.get(*index) {
                config.window.opacity = v;
            }
            true
        }
        WidgetAction::Selected { id, index } if *id == ids.font_size_dropdown => {
            if let Some(&v) = FONT_SIZE_VALUES.get(*index) {
                config.font.size = v;
            }
            true
        }
        WidgetAction::Selected { id, index } if *id == ids.font_weight_dropdown => {
            if let Some(&w) = FONT_WEIGHTS.get(*index) {
                config.font.weight = w;
            }
            true
        }
        WidgetAction::Toggled { id, value } if *id == ids.ligatures_checkbox => {
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
        WidgetAction::Selected { id, index } if *id == ids.paste_warning_dropdown => {
            config.behavior.warn_on_paste = match index {
                0 => PasteWarning::Always,
                _ => PasteWarning::Never,
            };
            true
        }
        WidgetAction::Selected { id, index } if *id == ids.cursor_style_dropdown => {
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

#[cfg(test)]
mod tests;
