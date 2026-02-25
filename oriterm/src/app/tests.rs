//! Tests for app-level theme resolution.

use oriterm_core::Theme;
use oriterm_ui::theme::UiTheme;

use crate::config::{Config, ThemeOverride};

use super::resolve_ui_theme_with;

// ── resolve_ui_theme_with: ThemeOverride → UiTheme mapping ──

#[test]
fn resolve_dark_override_ignores_system() {
    let mut config = Config::default();
    config.colors.theme = ThemeOverride::Dark;
    // System says Light, but override says Dark → dark theme.
    assert_eq!(
        resolve_ui_theme_with(&config, Theme::Light),
        UiTheme::dark()
    );
}

#[test]
fn resolve_light_override_ignores_system() {
    let mut config = Config::default();
    config.colors.theme = ThemeOverride::Light;
    // System says Dark, but override says Light → light theme.
    assert_eq!(
        resolve_ui_theme_with(&config, Theme::Dark),
        UiTheme::light()
    );
}

#[test]
fn resolve_auto_delegates_to_system_light() {
    let mut config = Config::default();
    config.colors.theme = ThemeOverride::Auto;
    assert_eq!(
        resolve_ui_theme_with(&config, Theme::Light),
        UiTheme::light()
    );
}

#[test]
fn resolve_auto_delegates_to_system_dark() {
    let mut config = Config::default();
    config.colors.theme = ThemeOverride::Auto;
    assert_eq!(resolve_ui_theme_with(&config, Theme::Dark), UiTheme::dark());
}

#[test]
fn resolve_auto_unknown_falls_back_to_dark() {
    let mut config = Config::default();
    config.colors.theme = ThemeOverride::Auto;
    assert_eq!(
        resolve_ui_theme_with(&config, Theme::Unknown),
        UiTheme::dark(),
    );
}
