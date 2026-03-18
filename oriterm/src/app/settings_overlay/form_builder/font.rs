//! Font page builder — typeface, size, weight, ligatures, and line height.
//!
//! Builds a page with a code preview widget and two sections:
//! Typeface (family dropdown, size input, weight dropdown) and
//! Features (ligatures toggle, line height input).

use oriterm_ui::layout::SizeSpec;
use oriterm_ui::theme::UiTheme;
use oriterm_ui::widgets::Widget;
use oriterm_ui::widgets::code_preview::CodePreviewWidget;
use oriterm_ui::widgets::container::ContainerWidget;
use oriterm_ui::widgets::dropdown::DropdownWidget;
use oriterm_ui::widgets::label::{LabelStyle, LabelWidget};
use oriterm_ui::widgets::number_input::NumberInputWidget;
use oriterm_ui::widgets::scroll::ScrollWidget;
use oriterm_ui::widgets::setting_row::SettingRowWidget;
use oriterm_ui::widgets::toggle::ToggleWidget;

use crate::config::Config;

use super::SettingsIds;
use super::appearance::{
    DESC_FONT_SIZE, PAGE_PADDING, ROW_GAP, SECTION_GAP, TITLE_FONT_SIZE, section_title,
};

/// Common monospace font families offered in the dropdown.
///
/// "Default" maps to `None` in config (uses platform default).
/// Index 0 is always the system default; action handler uses `None` for it.
pub(in crate::app) const FONT_FAMILIES: &[&str] = &[
    "Default (System)",
    "JetBrains Mono",
    "Cascadia Code",
    "Fira Code",
    "Source Code Pro",
    "Hack",
    "Inconsolata",
    "Menlo",
    "Consolas",
    "DejaVu Sans Mono",
    "Ubuntu Mono",
    "SF Mono",
];

/// Font weight labels matching their numeric value by position.
const WEIGHT_VALUES: &[u16] = &[100, 200, 300, 400, 500, 600, 700, 800, 900];

/// Builds the Font page content widget.
///
/// Writes font-related IDs into the provided `SettingsIds`.
pub(super) fn build_page(
    config: &Config,
    ids: &mut SettingsIds,
    theme: &UiTheme,
) -> Box<dyn Widget> {
    let header = build_header(theme);
    let preview = Box::new(CodePreviewWidget::new()) as Box<dyn Widget>;
    let typeface = build_typeface_section(config, ids, theme);
    let features = build_features_section(config, ids, theme);

    let page = ContainerWidget::column()
        .with_width(SizeSpec::Fill)
        .with_padding(PAGE_PADDING)
        .with_gap(SECTION_GAP)
        .with_child(header)
        .with_child(preview)
        .with_child(typeface)
        .with_child(features);

    let mut scroll = ScrollWidget::vertical(Box::new(page));
    scroll.set_height(SizeSpec::Fill);
    Box::new(scroll)
}

/// Page header: title + description.
fn build_header(theme: &UiTheme) -> Box<dyn Widget> {
    let title = LabelWidget::new("Font").with_style(LabelStyle {
        font_size: TITLE_FONT_SIZE,
        color: theme.fg_primary,
        ..LabelStyle::from_theme(theme)
    });
    let desc =
        LabelWidget::new("Typeface, size, and text rendering settings").with_style(LabelStyle {
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

/// Typeface section: font family dropdown, size input, weight dropdown.
fn build_typeface_section(
    config: &Config,
    ids: &mut SettingsIds,
    theme: &UiTheme,
) -> Box<dyn Widget> {
    // Font family dropdown.
    let items: Vec<String> = FONT_FAMILIES.iter().map(|s| (*s).to_owned()).collect();
    let family_idx = config
        .font
        .family
        .as_ref()
        .and_then(|fam| {
            FONT_FAMILIES
                .iter()
                .position(|f| f.eq_ignore_ascii_case(fam))
        })
        .unwrap_or(0); // "Default (System)" if not found.
    let family_dropdown = DropdownWidget::new(items).with_selected(family_idx);
    ids.font_family_dropdown = family_dropdown.id();

    let family_row = SettingRowWidget::new(
        "Font family",
        "Monospace font for terminal text",
        Box::new(family_dropdown),
        theme,
    );

    // Font size input.
    let size_input = NumberInputWidget::new(config.font.size, 8.0, 32.0, 0.5, theme);
    ids.font_size_input = size_input.id();

    let size_row =
        SettingRowWidget::new("Size", "Font size in points", Box::new(size_input), theme);

    // Font weight dropdown.
    let weight_items: Vec<String> = WEIGHT_VALUES.iter().map(u16::to_string).collect();
    let weight_idx = WEIGHT_VALUES
        .iter()
        .position(|w| *w == config.font.weight)
        .unwrap_or(3); // Default to 400 (Regular).
    let weight_dropdown = DropdownWidget::new(weight_items).with_selected(weight_idx);
    ids.font_weight_dropdown = weight_dropdown.id();

    let weight_row = SettingRowWidget::new(
        "Weight",
        "Font weight (100 = Thin, 400 = Regular, 700 = Bold)",
        Box::new(weight_dropdown),
        theme,
    );

    let title = section_title("Typeface", theme);
    Box::new(
        ContainerWidget::column()
            .with_width(SizeSpec::Fill)
            .with_gap(ROW_GAP)
            .with_child(title)
            .with_child(Box::new(family_row))
            .with_child(Box::new(size_row))
            .with_child(Box::new(weight_row)),
    )
}

/// Features section: ligatures toggle, line height input.
fn build_features_section(
    config: &Config,
    ids: &mut SettingsIds,
    theme: &UiTheme,
) -> Box<dyn Widget> {
    // Ligatures toggle.
    let has_liga = config.font.features.iter().any(|f| f == "liga");
    let has_neg = config.font.features.iter().any(|f| f == "-liga");
    let ligatures_on = has_liga && !has_neg;
    let liga_toggle = ToggleWidget::new().with_on(ligatures_on);
    ids.ligatures_toggle = liga_toggle.id();

    let liga_row = SettingRowWidget::new(
        "Ligatures",
        "Combine character sequences into single glyphs",
        Box::new(liga_toggle),
        theme,
    );

    // Line height input.
    let line_input = NumberInputWidget::new(config.font.line_height, 0.8, 2.0, 0.05, theme);
    ids.line_height_input = line_input.id();

    let line_row = SettingRowWidget::new(
        "Line height",
        "Line spacing multiplier (1.0 = default)",
        Box::new(line_input),
        theme,
    );

    let title = section_title("Features", theme);
    Box::new(
        ContainerWidget::column()
            .with_width(SizeSpec::Fill)
            .with_gap(ROW_GAP)
            .with_child(title)
            .with_child(Box::new(liga_row))
            .with_child(Box::new(line_row)),
    )
}
