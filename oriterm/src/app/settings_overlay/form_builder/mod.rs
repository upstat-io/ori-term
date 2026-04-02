//! Builds the settings dialog with sidebar navigation and 8 pages.
//!
//! Each page is built by its own submodule. Widget IDs are captured
//! in `SettingsIds` for action dispatch.

mod appearance;
mod bell;
mod colors;
mod font;
mod keybindings;
mod rendering;
mod shared;
mod terminal;
mod window;

use oriterm_ui::icons::IconId;
use oriterm_ui::layout::SizeSpec;
use oriterm_ui::theme::UiTheme;
use oriterm_ui::widget_id::WidgetId;
use oriterm_ui::widgets::Widget;
use oriterm_ui::widgets::container::ContainerWidget;
use oriterm_ui::widgets::page_container::PageContainerWidget;
use oriterm_ui::widgets::settings_footer::SettingsFooterWidget;
use oriterm_ui::widgets::sidebar_nav::{NavItem, NavSection, SidebarNavWidget};

use crate::config::Config;

pub(in crate::app) use font::FONT_FAMILIES;

/// Bell duration dropdown values in milliseconds.
pub(in crate::app) const BELL_DURATION_VALUES: [u16; 7] = [0, 50, 100, 150, 200, 300, 500];

/// Widget IDs for all settings controls, used to match actions in both
/// overlay dispatch and dialog window event handling.
pub(crate) struct SettingsIds {
    // Navigation.
    pub sidebar_id: WidgetId,
    // Appearance page.
    pub theme_dropdown: WidgetId,
    pub opacity_slider: WidgetId,
    pub blur_toggle: WidgetId,
    pub unfocused_opacity_slider: WidgetId,
    pub decorations_dropdown: WidgetId,
    pub tab_bar_style_dropdown: WidgetId,
    // Colors page — per-card IDs (each SchemeCard has its own ID).
    pub scheme_card_ids: Vec<WidgetId>,
    // Font page.
    pub font_family_dropdown: WidgetId,
    pub font_size_input: WidgetId,
    pub font_weight_dropdown: WidgetId,
    pub ligatures_toggle: WidgetId,
    pub line_height_input: WidgetId,
    // Terminal page.
    pub cursor_picker: WidgetId,
    pub cursor_blink_toggle: WidgetId,
    pub cursor_blink_fade_toggle: WidgetId,
    pub scrollback_input: WidgetId,
    pub shell_input: WidgetId,
    pub paste_warning_dropdown: WidgetId,
    // Window page.
    pub tab_bar_position_dropdown: WidgetId,
    pub grid_padding_input: WidgetId,
    pub restore_session_toggle: WidgetId,
    pub initial_columns_input: WidgetId,
    pub initial_rows_input: WidgetId,
    // Bell page.
    pub bell_animation_dropdown: WidgetId,
    pub bell_duration_dropdown: WidgetId,
    // Font page — Advanced section.
    pub hinting_dropdown: WidgetId,
    pub subpixel_aa_dropdown: WidgetId,
    pub subpixel_positioning_dropdown: WidgetId,
    pub atlas_filtering_dropdown: WidgetId,
    // Rendering page.
    pub gpu_backend_dropdown: WidgetId,
}

/// Builds the settings dialog with sidebar navigation and 8 pages.
///
/// Returns the content widget (sidebar + pages in a horizontal row) and the
/// ID map for action dispatch. `active_page` sets the initial page (use 0
/// for first open, or preserve the current page across rebuilds like reset).
#[expect(
    clippy::too_many_arguments,
    reason = "scale_factor + opacity needed for font Advanced section Auto labels"
)]
pub(in crate::app) fn build_settings_dialog(
    config: &Config,
    theme: &UiTheme,
    active_page: usize,
    scale_factor: f64,
    opacity: f64,
    update_info: Option<(&str, &str, &str)>,
) -> (Box<dyn Widget>, SettingsIds, (WidgetId, WidgetId, WidgetId)) {
    // Initialize IDs with placeholders; page builders overwrite their fields.
    let mut ids = SettingsIds::placeholder();

    let page_appearance = appearance::build_page(config, &mut ids, theme);
    let page_colors = colors::build_page(config, &mut ids, theme);
    let page_font = font::build_page(config, &mut ids, theme, scale_factor, opacity);
    let page_terminal = terminal::build_page(config, &mut ids, theme);
    let page_keybindings = keybindings::build_page(theme);
    let page_window = window::build_page(config, &mut ids, theme);
    let page_bell = bell::build_page(config, &mut ids, theme);
    let page_rendering = rendering::build_page(config, &mut ids, theme);

    let config_path = crate::config::config_path();
    let mut sidebar = build_sidebar(theme)
        .with_version(format!("v{}", env!("CARGO_PKG_VERSION")))
        .with_config_path(config_path.display().to_string());
    if let Some((label, tooltip, url)) = update_info {
        sidebar = sidebar.with_update_available(label, tooltip, url);
    }
    sidebar.set_active_page(active_page);
    let sidebar_id = sidebar.id();
    ids.sidebar_id = sidebar_id;

    let mut pages = PageContainerWidget::new(vec![
        page_appearance,
        page_colors,
        page_font,
        page_terminal,
        page_keybindings,
        page_window,
        page_bell,
        page_rendering,
    ])
    .with_nav_source(sidebar_id);
    pages.set_active_page(active_page);

    // Footer lives in the right column (below pages, above nothing).
    let footer = SettingsFooterWidget::new(theme);
    let footer_ids = footer.button_ids();

    // Right column: pages fill remaining space, footer is pinned at bottom.
    let right_column = ContainerWidget::column()
        .with_width(SizeSpec::Fill)
        .with_height(SizeSpec::Fill)
        .with_child(Box::new(pages))
        .with_child(Box::new(footer));

    let content = ContainerWidget::row()
        .with_width(SizeSpec::Fill)
        .with_height(SizeSpec::Fill)
        .with_child(Box::new(sidebar))
        .with_child(Box::new(right_column));

    (Box::new(content), ids, footer_ids)
}

impl SettingsIds {
    /// Creates a `SettingsIds` with all fields set to `WidgetId::placeholder()`.
    fn placeholder() -> Self {
        Self {
            sidebar_id: WidgetId::placeholder(),
            theme_dropdown: WidgetId::placeholder(),
            opacity_slider: WidgetId::placeholder(),
            blur_toggle: WidgetId::placeholder(),
            unfocused_opacity_slider: WidgetId::placeholder(),
            decorations_dropdown: WidgetId::placeholder(),
            tab_bar_style_dropdown: WidgetId::placeholder(),
            scheme_card_ids: Vec::new(),
            font_family_dropdown: WidgetId::placeholder(),
            font_size_input: WidgetId::placeholder(),
            font_weight_dropdown: WidgetId::placeholder(),
            ligatures_toggle: WidgetId::placeholder(),
            line_height_input: WidgetId::placeholder(),
            cursor_picker: WidgetId::placeholder(),
            cursor_blink_toggle: WidgetId::placeholder(),
            cursor_blink_fade_toggle: WidgetId::placeholder(),
            scrollback_input: WidgetId::placeholder(),
            shell_input: WidgetId::placeholder(),
            paste_warning_dropdown: WidgetId::placeholder(),
            tab_bar_position_dropdown: WidgetId::placeholder(),
            grid_padding_input: WidgetId::placeholder(),
            restore_session_toggle: WidgetId::placeholder(),
            initial_columns_input: WidgetId::placeholder(),
            initial_rows_input: WidgetId::placeholder(),
            bell_animation_dropdown: WidgetId::placeholder(),
            bell_duration_dropdown: WidgetId::placeholder(),
            hinting_dropdown: WidgetId::placeholder(),
            subpixel_aa_dropdown: WidgetId::placeholder(),
            subpixel_positioning_dropdown: WidgetId::placeholder(),
            atlas_filtering_dropdown: WidgetId::placeholder(),
            gpu_backend_dropdown: WidgetId::placeholder(),
        }
    }
}

/// Builds the sidebar with 8 navigation items across 2 sections.
fn build_sidebar(theme: &UiTheme) -> SidebarNavWidget {
    SidebarNavWidget::new(
        vec![
            NavSection {
                title: "General".into(),
                items: vec![
                    NavItem {
                        label: "Appearance".into(),
                        icon: Some(IconId::Sun),
                        page_index: 0,
                    },
                    NavItem {
                        label: "Colors".into(),
                        icon: Some(IconId::Palette),
                        page_index: 1,
                    },
                    NavItem {
                        label: "Font".into(),
                        icon: Some(IconId::Type),
                        page_index: 2,
                    },
                    NavItem {
                        label: "Terminal".into(),
                        icon: Some(IconId::Terminal),
                        page_index: 3,
                    },
                    NavItem {
                        label: "Keybindings".into(),
                        icon: Some(IconId::Keyboard),
                        page_index: 4,
                    },
                    NavItem {
                        label: "Window".into(),
                        icon: Some(IconId::Window),
                        page_index: 5,
                    },
                ],
            },
            NavSection {
                title: "Advanced".into(),
                items: vec![
                    NavItem {
                        label: "Bell".into(),
                        icon: Some(IconId::Bell),
                        page_index: 6,
                    },
                    NavItem {
                        label: "Rendering".into(),
                        icon: Some(IconId::Activity),
                        page_index: 7,
                    },
                ],
            },
        ],
        theme,
    )
}

#[cfg(test)]
mod tests;
