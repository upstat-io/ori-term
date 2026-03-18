//! Appearance page builder — theme, opacity, and blur settings.
//!
//! Builds a page with two sections: Theme (color scheme dropdown) and
//! Window (opacity slider, blur toggle). Uses `SettingRowWidget` for
//! each control.

use oriterm_ui::geometry::Insets;
use oriterm_ui::layout::SizeSpec;
use oriterm_ui::theme::UiTheme;
use oriterm_ui::widgets::Widget;
use oriterm_ui::widgets::container::ContainerWidget;
use oriterm_ui::widgets::dropdown::DropdownWidget;
use oriterm_ui::widgets::label::{LabelStyle, LabelWidget};
use oriterm_ui::widgets::scroll::ScrollWidget;
use oriterm_ui::widgets::setting_row::SettingRowWidget;
use oriterm_ui::widgets::slider::SliderWidget;
use oriterm_ui::widgets::toggle::ToggleWidget;

use crate::config::Config;

use super::SettingsIds;

/// Page content padding (shared by all page builders).
pub(super) const PAGE_PADDING: Insets = Insets::vh(16.0, 20.0);

/// Gap between sections (shared by all page builders).
pub(super) const SECTION_GAP: f32 = 24.0;

/// Gap between section title and first row.
const TITLE_ROW_GAP: f32 = 8.0;

/// Gap between rows within a section (shared by all page builders).
pub(super) const ROW_GAP: f32 = 2.0;

/// Page title font size (shared by all page builders).
pub(super) const TITLE_FONT_SIZE: f32 = 20.0;

/// Page description font size (shared by all page builders).
pub(super) const DESC_FONT_SIZE: f32 = 12.0;

/// Section header font size.
const SECTION_FONT_SIZE: f32 = 11.0;

/// Builds the Appearance page content widget.
///
/// Writes `theme_dropdown`, `opacity_slider`, and `blur_toggle` IDs
/// into the provided `SettingsIds`.
pub(super) fn build_page(
    config: &Config,
    ids: &mut SettingsIds,
    theme: &UiTheme,
) -> Box<dyn Widget> {
    let header = build_header(theme);
    let theme_section = build_theme_section(config, ids, theme);
    let window_section = build_window_section(config, ids, theme);

    let page = ContainerWidget::column()
        .with_width(SizeSpec::Fill)
        .with_padding(PAGE_PADDING)
        .with_gap(SECTION_GAP)
        .with_child(header)
        .with_child(theme_section)
        .with_child(window_section);

    let mut scroll = ScrollWidget::vertical(Box::new(page));
    scroll.set_height(SizeSpec::Fill);
    Box::new(scroll)
}

/// Page header: title + description.
fn build_header(theme: &UiTheme) -> Box<dyn Widget> {
    let title = LabelWidget::new("Appearance").with_style(LabelStyle {
        font_size: TITLE_FONT_SIZE,
        color: theme.fg_primary,
        ..LabelStyle::from_theme(theme)
    });
    let desc =
        LabelWidget::new("Theme, transparency, and visual settings").with_style(LabelStyle {
            font_size: DESC_FONT_SIZE,
            color: theme.fg_secondary,
            ..LabelStyle::from_theme(theme)
        });
    Box::new(
        ContainerWidget::column()
            .with_gap(4.0)
            .with_width(SizeSpec::Fill)
            .with_child(Box::new(title))
            .with_child(Box::new(desc)),
    )
}

/// Theme section: color scheme dropdown.
fn build_theme_section(config: &Config, ids: &mut SettingsIds, theme: &UiTheme) -> Box<dyn Widget> {
    let names = crate::scheme::builtin_names();
    let selected = names
        .iter()
        .position(|n| *n == config.colors.scheme)
        .unwrap_or(0);
    let items: Vec<String> = names.iter().map(|s| (*s).to_owned()).collect();
    let dropdown = DropdownWidget::new(items).with_selected(selected);
    ids.theme_dropdown = dropdown.id();

    let row = SettingRowWidget::new(
        "Color scheme",
        "Terminal color palette and syntax theme",
        Box::new(dropdown),
        theme,
    );

    let title = section_title("Theme", theme);
    Box::new(
        ContainerWidget::column()
            .with_width(SizeSpec::Fill)
            .with_gap(TITLE_ROW_GAP)
            .with_child(title)
            .with_child(Box::new(row)),
    )
}

/// Window section: opacity slider + blur toggle.
fn build_window_section(
    config: &Config,
    ids: &mut SettingsIds,
    theme: &UiTheme,
) -> Box<dyn Widget> {
    // Opacity slider: 30-100% (displayed as integer percentage).
    let slider = SliderWidget::new()
        .with_range(30.0, 100.0)
        .with_step(1.0)
        .with_value(config.window.opacity * 100.0);
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

    let title = section_title("Window", theme);
    Box::new(
        ContainerWidget::column()
            .with_width(SizeSpec::Fill)
            .with_gap(ROW_GAP)
            .with_child(title)
            .with_child(Box::new(opacity_row))
            .with_child(Box::new(blur_row)),
    )
}

/// Creates a small section title label (shared by all page builders).
pub(super) fn section_title(text: &str, theme: &UiTheme) -> Box<dyn Widget> {
    let label = LabelWidget::new(text.to_uppercase()).with_style(LabelStyle {
        font_size: SECTION_FONT_SIZE,
        color: theme.fg_faint,
        ..LabelStyle::from_theme(theme)
    });
    // Add top gap to separate from header (first section gets gap from parent).
    Box::new(label)
}
