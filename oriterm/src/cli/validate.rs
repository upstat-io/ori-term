//! Config validation helpers for the CLI `validate-config` subcommand.
//!
//! Extracted from `cli/mod.rs` to keep that file under the 500-line budget.

use crate::config::{self, Config};
use crate::keybindings;

/// Validate hex color strings in the config; append errors to `errors`.
pub(super) fn validate_colors(config: &Config, errors: &mut Vec<String>) {
    let fields: &[(&str, &Option<String>)] = &[
        ("colors.foreground", &config.colors.foreground),
        ("colors.background", &config.colors.background),
        ("colors.cursor", &config.colors.cursor),
        (
            "colors.selection_foreground",
            &config.colors.selection_foreground,
        ),
        (
            "colors.selection_background",
            &config.colors.selection_background,
        ),
    ];

    for (name, value) in fields {
        if let Some(hex) = value {
            if config::parse_hex_color(hex).is_none() {
                errors.push(format!("{name}: invalid hex color {hex:?}"));
            }
        }
    }

    for (key, hex) in &config.colors.ansi {
        if config::parse_hex_color(hex).is_none() {
            errors.push(format!("colors.ansi.{key}: invalid hex color {hex:?}"));
        }
    }
    for (key, hex) in &config.colors.bright {
        if config::parse_hex_color(hex).is_none() {
            errors.push(format!("colors.bright.{key}: invalid hex color {hex:?}"));
        }
    }

    if let Some(hex) = &config.bell.color {
        if config::parse_hex_color(hex).is_none() {
            errors.push(format!("bell.color: invalid hex color {hex:?}"));
        }
    }
}

/// Validate keybinding entries; append errors to `errors`.
pub(super) fn validate_keybindings(config: &Config, errors: &mut Vec<String>) {
    for (i, kb) in config.keybind.iter().enumerate() {
        if keybindings::parse_key(&kb.key).is_none() {
            errors.push(format!("keybind[{i}]: unknown key {:?}", kb.key));
        }
        if keybindings::parse_action(&kb.action).is_none() {
            errors.push(format!("keybind[{i}]: unknown action {:?}", kb.action));
        }
    }
}
