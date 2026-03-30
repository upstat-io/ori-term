//! Font page builder — typeface, size, weight, ligatures, and line height.
//!
//! Builds a page with a code preview widget and two sections:
//! Typeface (family dropdown, size input, weight dropdown) and
//! Features (ligatures toggle, line height input).

use oriterm_ui::layout::SizeSpec;
use oriterm_ui::theme::UiTheme;
use oriterm_ui::widget_id::WidgetId;
use oriterm_ui::widgets::Widget;
use oriterm_ui::widgets::code_preview::CodePreviewWidget;
use oriterm_ui::widgets::container::ContainerWidget;
use oriterm_ui::widgets::dropdown::DropdownWidget;
use oriterm_ui::widgets::number_input::NumberInputWidget;
use oriterm_ui::widgets::setting_row::SettingRowWidget;
use oriterm_ui::widgets::toggle::ToggleWidget;

use crate::config::Config;

use super::SettingsIds;
use super::shared::{
    build_section_header, build_section_header_with_description, build_settings_page,
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
/// `scale_factor` and `opacity` drive the "Auto (detected)" labels in the
/// Advanced section dropdowns.
pub(super) fn build_page(
    config: &Config,
    ids: &mut SettingsIds,
    theme: &UiTheme,
    scale_factor: f64,
    opacity: f64,
) -> Box<dyn Widget> {
    build_settings_page(
        "Font",
        "Typeface, size, and text rendering settings",
        vec![
            Box::new(CodePreviewWidget::new()) as Box<dyn Widget>,
            build_typeface_section(config, ids, theme),
            build_features_section(config, ids, theme),
            build_fallback_section(theme),
            build_advanced_section(config, ids, theme, scale_factor, opacity),
        ],
        theme,
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
    let family_dropdown = DropdownWidget::new(items)
        .with_selected(family_idx)
        .with_min_width(180.0);
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

    Box::new(
        ContainerWidget::column()
            .with_width(SizeSpec::Fill)
            .with_child(build_section_header("Typeface", theme))
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

    Box::new(
        ContainerWidget::column()
            .with_width(SizeSpec::Fill)
            .with_child(build_section_header("Features", theme))
            .with_child(Box::new(liga_row))
            .with_child(Box::new(line_row)),
    )
}

/// Fallback section: header with description (settings TBD).
fn build_fallback_section(theme: &UiTheme) -> Box<dyn Widget> {
    build_section_header_with_description(
        "Fallback",
        "Used when the primary font doesn't contain a glyph (emoji, CJK, symbols).",
        theme,
    )
}

/// Create a dropdown setting row, returning the widget ID and boxed row.
fn dropdown_row(
    label: &str,
    description: &str,
    items: Vec<String>,
    selected: usize,
    theme: &UiTheme,
) -> (WidgetId, Box<dyn Widget>) {
    let dd = DropdownWidget::new(items).with_selected(selected);
    let id = dd.id();
    let row = SettingRowWidget::new(label, description, Box::new(dd), theme);
    (id, Box::new(row))
}

/// Advanced section: hinting, subpixel AA, subpixel positioning, atlas filtering.
fn build_advanced_section(
    config: &Config,
    ids: &mut SettingsIds,
    theme: &UiTheme,
    scale_factor: f64,
    opacity: f64,
) -> Box<dyn Widget> {
    use crate::font::{HintingMode, SubpixelMode};
    use crate::gpu::bind_groups::AtlasFiltering;

    let auto_h = HintingMode::from_scale_factor(scale_factor);
    let auto_s = SubpixelMode::for_display(scale_factor, opacity);
    let auto_f = AtlasFiltering::from_scale_factor(scale_factor);

    let h_auto = match auto_h {
        HintingMode::Full => "Auto (Full)",
        HintingMode::None => "Auto (None)",
    };
    let h_idx = match config.font.hinting.as_deref() {
        Some("full") => 1,
        Some("none") => 2,
        _ => 0,
    };
    let (h_id, h_row) = dropdown_row(
        "Hinting",
        "Font hinting for sharper text at low DPI",
        vec![h_auto.into(), "Full".into(), "None".into()],
        h_idx,
        theme,
    );
    ids.hinting_dropdown = h_id;

    let s_auto = match auto_s {
        SubpixelMode::Rgb => "Auto (RGB)",
        SubpixelMode::Bgr => "Auto (BGR)",
        SubpixelMode::None => "Auto (None)",
    };
    let s_idx = match config.font.subpixel_mode.as_deref() {
        Some("rgb") => 1,
        Some("bgr") => 2,
        Some("none") => 3,
        _ => 0,
    };
    let (s_id, s_row) = dropdown_row(
        "Subpixel AA",
        "LCD subpixel antialiasing for color fringe sharpening",
        vec![
            s_auto.into(),
            "RGB".into(),
            "BGR".into(),
            "None (Grayscale)".into(),
        ],
        s_idx,
        theme,
    );
    ids.subpixel_aa_dropdown = s_id;

    let sp_idx = match config.font.subpixel_positioning {
        None => 0,
        Some(true) => 1,
        Some(false) => 2,
    };
    let (sp_id, sp_row) = dropdown_row(
        "Subpixel positioning",
        "Quarter-pixel glyph placement for smoother text",
        vec![
            "Auto (Quarter-pixel)".into(),
            "Quarter-pixel".into(),
            "None".into(),
        ],
        sp_idx,
        theme,
    );
    ids.subpixel_positioning_dropdown = sp_id;

    let f_auto = match auto_f {
        AtlasFiltering::Linear => "Auto (Linear)",
        AtlasFiltering::Nearest => "Auto (Nearest)",
    };
    let f_idx = match config.font.atlas_filtering.as_deref() {
        Some("linear") => 1,
        Some("nearest") => 2,
        _ => 0,
    };
    let (f_id, f_row) = dropdown_row(
        "Atlas filtering",
        "GPU texture sampling: Linear (smooth) or Nearest (crisp)",
        vec![f_auto.into(), "Linear".into(), "Nearest".into()],
        f_idx,
        theme,
    );
    ids.atlas_filtering_dropdown = f_id;

    Box::new(
        ContainerWidget::column()
            .with_width(SizeSpec::Fill)
            .with_child(build_section_header("Advanced", theme))
            .with_child(h_row)
            .with_child(s_row)
            .with_child(sp_row)
            .with_child(f_row),
    )
}
