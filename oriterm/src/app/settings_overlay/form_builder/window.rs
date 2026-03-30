//! Window page builder — tab bar, padding, and startup settings.

use oriterm_ui::layout::SizeSpec;
use oriterm_ui::theme::UiTheme;
use oriterm_ui::widgets::Widget;
use oriterm_ui::widgets::container::ContainerWidget;
use oriterm_ui::widgets::dropdown::DropdownWidget;
use oriterm_ui::widgets::number_input::NumberInputWidget;
use oriterm_ui::widgets::setting_row::SettingRowWidget;
use oriterm_ui::widgets::toggle::ToggleWidget;

use crate::config::{Config, TabBarPosition};

use super::SettingsIds;
use super::shared::{build_section_header, build_settings_page};

/// Builds the Window page content widget.
pub(super) fn build_page(
    config: &Config,
    ids: &mut SettingsIds,
    theme: &UiTheme,
) -> Box<dyn Widget> {
    build_settings_page(
        "Window",
        "Window chrome, padding, and startup behavior",
        vec![
            build_chrome_section(config, ids, theme),
            build_padding_section(config, ids, theme),
            build_startup_section(config, ids, theme),
        ],
        theme,
    )
}

/// Chrome section: tab bar position.
fn build_chrome_section(
    config: &Config,
    ids: &mut SettingsIds,
    theme: &UiTheme,
) -> Box<dyn Widget> {
    let items = vec!["Top".to_owned(), "Bottom".to_owned(), "Hidden".to_owned()];
    let idx = match config.window.tab_bar_position {
        TabBarPosition::Top => 0,
        TabBarPosition::Bottom => 1,
        TabBarPosition::Hidden => 2,
    };
    let dropdown = DropdownWidget::new(items).with_selected(idx);
    ids.tab_bar_position_dropdown = dropdown.id();

    let row = SettingRowWidget::new(
        "Tab bar position",
        "Where to show the tab bar",
        Box::new(dropdown),
        theme,
    );

    Box::new(
        ContainerWidget::column()
            .with_width(SizeSpec::Fill)
            .with_child(build_section_header("Chrome", theme))
            .with_child(Box::new(row)),
    )
}

/// Padding section: grid padding.
fn build_padding_section(
    config: &Config,
    ids: &mut SettingsIds,
    theme: &UiTheme,
) -> Box<dyn Widget> {
    let input = NumberInputWidget::new(config.window.grid_padding, 0.0, 40.0, 2.0, theme);
    ids.grid_padding_input = input.id();

    let row = SettingRowWidget::new(
        "Grid padding",
        "Padding around the terminal grid in pixels",
        Box::new(input),
        theme,
    );

    Box::new(
        ContainerWidget::column()
            .with_width(SizeSpec::Fill)
            .with_child(build_section_header("Padding", theme))
            .with_child(Box::new(row)),
    )
}

/// Startup section: restore session, initial size.
fn build_startup_section(
    config: &Config,
    ids: &mut SettingsIds,
    theme: &UiTheme,
) -> Box<dyn Widget> {
    let restore = ToggleWidget::new()
        .with_on(config.window.restore_session)
        .with_disabled(true);
    ids.restore_session_toggle = restore.id();

    let restore_row = SettingRowWidget::new(
        "Restore previous session",
        "Not yet implemented — coming in a future release",
        Box::new(restore),
        theme,
    );

    let cols = NumberInputWidget::new(config.window.columns as f32, 40.0, 400.0, 10.0, theme)
        .with_input_width(44.0);
    ids.initial_columns_input = cols.id();

    let cols_row = SettingRowWidget::new(
        "Initial columns",
        "Default window width in columns",
        Box::new(cols),
        theme,
    );

    let rows = NumberInputWidget::new(config.window.rows as f32, 10.0, 100.0, 5.0, theme)
        .with_input_width(44.0);
    ids.initial_rows_input = rows.id();

    let rows_row = SettingRowWidget::new(
        "Initial rows",
        "Default window height in rows",
        Box::new(rows),
        theme,
    );

    Box::new(
        ContainerWidget::column()
            .with_width(SizeSpec::Fill)
            .with_child(build_section_header("Startup", theme))
            .with_child(Box::new(restore_row))
            .with_child(Box::new(cols_row))
            .with_child(Box::new(rows_row)),
    )
}
