//! Appearance page builder — theme, opacity, and blur settings.
//!
//! Builds a page with two sections: Theme (color scheme dropdown) and
//! Window (opacity slider, blur toggle). Uses `SettingRowWidget` for
//! each control.

use oriterm_ui::layout::SizeSpec;
use oriterm_ui::theme::UiTheme;
use oriterm_ui::widgets::Widget;
use oriterm_ui::widgets::container::ContainerWidget;
use oriterm_ui::widgets::dropdown::DropdownWidget;
use oriterm_ui::widgets::setting_row::SettingRowWidget;
use oriterm_ui::widgets::slider::{SliderWidget, ValueDisplay};
use oriterm_ui::widgets::toggle::ToggleWidget;

use crate::config::Config;

use super::SettingsIds;
use super::shared::{build_section_header, build_settings_page};

/// Builds the Appearance page content widget.
///
/// Writes `theme_dropdown`, `opacity_slider`, and `blur_toggle` IDs
/// into the provided `SettingsIds`.
pub(super) fn build_page(
    config: &Config,
    ids: &mut SettingsIds,
    theme: &UiTheme,
) -> Box<dyn Widget> {
    build_settings_page(
        "Appearance",
        "Theme, transparency, and visual settings",
        vec![
            build_theme_section(config, ids, theme),
            build_window_section(config, ids, theme),
            build_decorations_section(config, ids, theme),
        ],
        theme,
    )
}

/// Theme section: color scheme dropdown with "(dark)" / "(light)" labels.
fn build_theme_section(config: &Config, ids: &mut SettingsIds, theme: &UiTheme) -> Box<dyn Widget> {
    let names = crate::scheme::builtin_names();
    let selected = names
        .iter()
        .position(|n| n.eq_ignore_ascii_case(&config.colors.scheme))
        .unwrap_or(0);
    let items: Vec<String> = names
        .iter()
        .map(|name| {
            let label = crate::scheme::find_builtin(name).map_or("", |s| s.brightness_label());
            format!("{name}{label}")
        })
        .collect();
    let dropdown = DropdownWidget::new(items).with_selected(selected);
    ids.theme_dropdown = dropdown.id();

    let row = SettingRowWidget::new(
        "Color scheme",
        "Terminal color palette and syntax theme",
        Box::new(dropdown),
        theme,
    );

    Box::new(
        ContainerWidget::column()
            .with_width(SizeSpec::Fill)
            .with_child(build_section_header("Theme", theme))
            .with_child(Box::new(row)),
    )
}

/// Window section: opacity slider + blur toggle + unfocused opacity.
fn build_window_section(
    config: &Config,
    ids: &mut SettingsIds,
    theme: &UiTheme,
) -> Box<dyn Widget> {
    // Opacity slider: 30-100% (displayed as integer percentage).
    let slider = SliderWidget::new()
        .with_range(30.0, 100.0)
        .with_step(1.0)
        .with_value(config.window.opacity * 100.0)
        .with_display(ValueDisplay::Percent);
    ids.opacity_slider = slider.id();

    let opacity_row = SettingRowWidget::new(
        "Opacity",
        "Window transparency (30–100%)",
        Box::new(slider),
        theme,
    );

    // Blur toggle.
    let toggle = ToggleWidget::new().with_on(config.window.blur);
    ids.blur_toggle = toggle.id();

    let blur_row = SettingRowWidget::new(
        "Blur behind",
        "Apply backdrop blur when window is transparent",
        Box::new(toggle),
        theme,
    );

    // Unfocused opacity slider: 30-100%.
    let unfocused_slider = SliderWidget::new()
        .with_range(30.0, 100.0)
        .with_step(1.0)
        .with_value(config.window.unfocused_opacity * 100.0)
        .with_display(ValueDisplay::Percent);
    ids.unfocused_opacity_slider = unfocused_slider.id();

    let unfocused_row = SettingRowWidget::new(
        "Unfocused opacity",
        "Dim the terminal when the window loses focus",
        Box::new(unfocused_slider),
        theme,
    );

    Box::new(
        ContainerWidget::column()
            .with_width(SizeSpec::Fill)
            .with_child(build_section_header("Window", theme))
            .with_child(Box::new(opacity_row))
            .with_child(Box::new(blur_row))
            .with_child(Box::new(unfocused_row)),
    )
}

/// Decorations section: window decorations dropdown + tab bar style dropdown.
fn build_decorations_section(
    config: &Config,
    ids: &mut SettingsIds,
    theme: &UiTheme,
) -> Box<dyn Widget> {
    use crate::config::{Decorations, TabBarStyle};

    // Window decorations dropdown.
    // macOS exposes Buttonless (hides traffic lights) as a distinct mode;
    // on other platforms it's identical to Transparent so we omit it.
    #[cfg(target_os = "macos")]
    let decoration_items = vec![
        "None (frameless)".to_owned(),
        "Full".to_owned(),
        "Transparent".to_owned(),
        "Buttonless".to_owned(),
    ];
    #[cfg(not(target_os = "macos"))]
    let decoration_items = vec![
        "None (frameless)".to_owned(),
        "Full".to_owned(),
        "Transparent".to_owned(),
    ];

    let decoration_selected = match config.window.decorations {
        Decorations::None => 0,
        Decorations::Full => 1,
        Decorations::Transparent => 2,
        #[cfg(target_os = "macos")]
        Decorations::Buttonless => 3,
        #[cfg(not(target_os = "macos"))]
        Decorations::Buttonless => 2, // maps to Transparent on non-macOS
    };
    let dec_dropdown = DropdownWidget::new(decoration_items).with_selected(decoration_selected);
    ids.decorations_dropdown = dec_dropdown.id();

    let dec_row = SettingRowWidget::new(
        "Window decorations",
        "Title bar and window border style",
        Box::new(dec_dropdown),
        theme,
    );

    // Tab bar style dropdown: Default / Compact / Hidden.
    // "Hidden" maps to TabBarPosition::Hidden rather than a separate style.
    let style_items = vec![
        "Default".to_owned(),
        "Compact".to_owned(),
        "Hidden".to_owned(),
    ];
    let style_selected = if config.window.tab_bar_position == crate::config::TabBarPosition::Hidden
    {
        2 // Hidden
    } else {
        match config.window.tab_bar_style {
            TabBarStyle::Default => 0,
            TabBarStyle::Compact => 1,
        }
    };
    let style_dropdown = DropdownWidget::new(style_items).with_selected(style_selected);
    ids.tab_bar_style_dropdown = style_dropdown.id();

    let style_row = SettingRowWidget::new(
        "Tab bar style",
        "Appearance of the tab strip",
        Box::new(style_dropdown),
        theme,
    );

    Box::new(
        ContainerWidget::column()
            .with_width(SizeSpec::Fill)
            .with_child(build_section_header("Decorations", theme))
            .with_child(Box::new(dec_row))
            .with_child(Box::new(style_row)),
    )
}
