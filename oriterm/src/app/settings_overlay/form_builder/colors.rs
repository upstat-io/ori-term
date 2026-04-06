//! Colors page builder — scheme selection and palette editor.
//!
//! Builds a page with two sections: a `SchemeCard` grid for selecting
//! color schemes, and a palette editor showing the active scheme's
//! special colors and ANSI palette.

use oriterm_ui::color::Color;
use oriterm_ui::layout::{GridColumns, SizeSpec};
use oriterm_ui::theme::UiTheme;
use oriterm_ui::widgets::Widget;
use oriterm_ui::widgets::color_swatch::{ColorSwatchGrid, SpecialColorSwatch};
use oriterm_ui::widgets::container::ContainerWidget;
use oriterm_ui::widgets::label::{LabelStyle, LabelWidget};
use oriterm_ui::widgets::panel::{PanelStyle, PanelWidget};
use oriterm_ui::widgets::scheme_card::{SchemeCardData, SchemeCardWidget};

use crate::config::Config;
use crate::scheme::{self, ColorScheme};

use super::shared::{build_section_header, build_settings_page};

/// Gap between scheme cards in the grid.
const CARD_GAP: f32 = 10.0;

/// Minimum card width for the auto-fill grid (mockup `minmax(240px, 1fr)`).
const CARD_MIN_WIDTH: f32 = 240.0;

/// Gap between special color swatches (mockup: 6px).
const SWATCH_GAP: f32 = 6.0;

/// Gap between palette subsections (special colors, ANSI rows).
const PALETTE_GAP: f32 = 10.0;

/// Padding inside the palette card (mockup: 14px).
const PALETTE_CARD_PADDING: f32 = 14.0;

/// Builds the Colors page content widget.
///
/// Displays a grid of `SchemeCardWidget`s for all built-in schemes and
/// a palette editor for the currently active scheme. Captures scheme card
/// IDs into `ids` for action dispatch.
pub(super) fn build_page(
    config: &Config,
    ids: &mut super::SettingsIds,
    theme: &UiTheme,
) -> Box<dyn Widget> {
    build_settings_page(
        "Colors",
        "Color schemes and palette customization",
        vec![
            build_schemes_section(config, ids, theme),
            build_palette_section(config, theme),
        ],
        theme,
    )
}

/// Schemes section: cards grouped by dark/light brightness.
///
/// Captures each card's `WidgetId` into `ids.scheme_card_ids` so the
/// action handler can match `Selected` actions from card clicks.
fn build_schemes_section(
    config: &Config,
    ids: &mut super::SettingsIds,
    theme: &UiTheme,
) -> Box<dyn Widget> {
    let names = scheme::builtin_names();

    // Build all cards first, partitioned by brightness.
    let mut dark_cards: Vec<SchemeCardWidget> = Vec::new();
    let mut light_cards: Vec<SchemeCardWidget> = Vec::new();
    for (i, name) in names.iter().enumerate() {
        let resolved = match scheme::find_builtin(name) {
            Some(s) => s,
            None => continue,
        };
        let brightness = resolved.brightness();
        let selected = resolved.name.eq_ignore_ascii_case(&config.colors.scheme);
        let data = scheme_to_card_data(&resolved, selected);
        let card = SchemeCardWidget::new(data, i, theme);
        ids.scheme_card_ids.push(card.id());
        if brightness == scheme::SchemeBrightness::Light {
            light_cards.push(card);
        } else {
            dark_cards.push(card);
        }
    }

    // Set the scheme group on all cards so selection deselects siblings.
    let group_ids = ids.scheme_card_ids.clone();
    for card in dark_cards.iter_mut().chain(light_cards.iter_mut()) {
        card.set_scheme_group(group_ids.clone());
    }

    let dark_grid = build_card_grid(dark_cards);
    let light_grid = build_card_grid(light_cards);

    Box::new(
        ContainerWidget::column()
            .with_width(SizeSpec::Fill)
            .with_gap(CARD_GAP)
            .with_child(build_section_header("Dark Schemes", theme))
            .with_child(dark_grid)
            .with_child(build_section_header("Light Schemes", theme))
            .with_child(light_grid),
    )
}

/// Builds an auto-fill grid of scheme cards.
fn build_card_grid(cards: Vec<SchemeCardWidget>) -> Box<dyn Widget> {
    let mut grid = ContainerWidget::grid(
        GridColumns::AutoFill {
            min_width: CARD_MIN_WIDTH,
        },
        CARD_GAP,
    )
    .with_width(SizeSpec::Fill);
    for card in cards {
        grid = grid.with_child(Box::new(card));
    }
    Box::new(grid)
}

/// Palette editor: special colors + ANSI color grids for the active scheme.
fn build_palette_section(config: &Config, theme: &UiTheme) -> Box<dyn Widget> {
    let active = scheme::find_builtin(&config.colors.scheme);

    let palette_title = format!(
        "Palette — {}",
        active.as_ref().map_or("Unknown", |s| s.name.as_str())
    );
    let title = build_section_header(&palette_title, theme);

    let (specials, normal_ansi, bright_ansi) = match active {
        Some(ref s) => (
            build_special_colors(s, theme),
            build_ansi_grid(&s.ansi[..8], theme),
            build_ansi_grid(&s.ansi[8..16], theme),
        ),
        None => (
            Box::new(LabelWidget::new("No scheme selected")) as Box<dyn Widget>,
            Box::new(LabelWidget::new("")) as Box<dyn Widget>,
            Box::new(LabelWidget::new("")) as Box<dyn Widget>,
        ),
    };

    let normal_label = LabelWidget::new("Normal").with_style(LabelStyle {
        font_size: 11.0,
        color: theme.fg_faint,
        line_height: None,
        ..LabelStyle::from_theme(theme)
    });
    let bright_label = LabelWidget::new("Bright").with_style(LabelStyle {
        font_size: 11.0,
        color: theme.fg_faint,
        line_height: None,
        ..LabelStyle::from_theme(theme)
    });

    // Wrap palette in a card (mockup `.color-editor`): bg_raised, 2px border,
    // 14px padding. PanelWidget provides bg + border + padding over a child.
    let inner = ContainerWidget::column()
        .with_width(SizeSpec::Fill)
        .with_gap(PALETTE_GAP)
        .with_child(specials)
        .with_child(Box::new(normal_label))
        .with_child(normal_ansi)
        .with_child(Box::new(bright_label))
        .with_child(bright_ansi);

    let card_style = PanelStyle {
        bg: theme.bg_card,
        border_color: theme.border,
        border_width: 2.0,
        corner_radius: 0.0,
        padding: oriterm_ui::geometry::Insets::all(PALETTE_CARD_PADDING),
        shadow: None,
    };
    let card = PanelWidget::new(Box::new(inner)).with_style(card_style);

    Box::new(
        ContainerWidget::column()
            .with_width(SizeSpec::Fill)
            .with_child(title)
            .with_child(Box::new(card)),
    )
}

/// Four special color swatches: Foreground, Background, Cursor, Selection.
fn build_special_colors(scheme: &ColorScheme, theme: &UiTheme) -> Box<dyn Widget> {
    let sel_bg = scheme.selection_bg.unwrap_or(scheme.fg);
    let swatches = vec![
        Box::new(SpecialColorSwatch::new(
            "Foreground",
            rgb_to_color(scheme.fg),
            theme,
        )) as Box<dyn Widget>,
        Box::new(SpecialColorSwatch::new(
            "Background",
            rgb_to_color(scheme.bg),
            theme,
        )),
        Box::new(SpecialColorSwatch::new(
            "Cursor",
            rgb_to_color(scheme.cursor),
            theme,
        )),
        Box::new(SpecialColorSwatch::new(
            "Selection",
            rgb_to_color(sel_bg),
            theme,
        )),
    ];

    Box::new(
        ContainerWidget::grid(GridColumns::Fixed(4), SWATCH_GAP)
            .with_width(SizeSpec::Fill)
            .with_children(swatches),
    )
}

/// 8-column ANSI color grid from a slice of `Rgb` values.
fn build_ansi_grid(colors: &[oriterm_core::Rgb], theme: &UiTheme) -> Box<dyn Widget> {
    let ui_colors: Vec<Color> = colors.iter().map(|c| rgb_to_color(*c)).collect();
    Box::new(ColorSwatchGrid::new(ui_colors, theme))
}

/// Convert `oriterm_core::Rgb` (u8 channels) to `oriterm_ui::Color` (f32).
fn rgb_to_color(rgb: oriterm_core::Rgb) -> Color {
    Color::from_rgb_u8(rgb.r, rgb.g, rgb.b)
}

/// Convert a `ColorScheme` to `SchemeCardData`.
fn scheme_to_card_data(scheme: &ColorScheme, selected: bool) -> SchemeCardData {
    let ansi: [Color; 8] = std::array::from_fn(|i| rgb_to_color(scheme.ansi[i]));
    SchemeCardData {
        name: scheme.name.clone(),
        bg: rgb_to_color(scheme.bg),
        fg: rgb_to_color(scheme.fg),
        ansi,
        selected,
    }
}
