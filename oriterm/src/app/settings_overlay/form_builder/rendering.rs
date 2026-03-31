//! Rendering page builder — GPU backend settings.

use oriterm_ui::layout::SizeSpec;
use oriterm_ui::theme::UiTheme;
use oriterm_ui::widgets::Widget;
use oriterm_ui::widgets::container::ContainerWidget;
use oriterm_ui::widgets::dropdown::DropdownWidget;
use oriterm_ui::widgets::setting_row::SettingRowWidget;

use crate::config::{Config, GpuBackend};

use super::SettingsIds;
use super::shared::{
    build_section_header, build_section_header_with_description, build_settings_page,
};

/// Builds the Rendering page content widget.
pub(super) fn build_page(
    config: &Config,
    ids: &mut SettingsIds,
    theme: &UiTheme,
) -> Box<dyn Widget> {
    build_settings_page(
        "Rendering",
        "GPU backend settings",
        vec![
            build_gpu_section(config, ids, theme),
            build_performance_section(theme),
        ],
        theme,
    )
}

/// GPU section: backend dropdown + restart notice.
fn build_gpu_section(config: &Config, ids: &mut SettingsIds, theme: &UiTheme) -> Box<dyn Widget> {
    let available = GpuBackend::available();
    let items: Vec<String> = available
        .iter()
        .map(|(_, label)| (*label).to_owned())
        .collect();
    let idx = available
        .iter()
        .position(|(b, _)| *b == config.rendering.gpu_backend)
        .unwrap_or(0);
    let dropdown = DropdownWidget::new(items).with_selected(idx);
    ids.gpu_backend_dropdown = dropdown.id();

    let row = SettingRowWidget::new(
        "Backend",
        "GPU rendering backend (restart required to apply)",
        Box::new(dropdown),
        theme,
    );

    Box::new(
        ContainerWidget::column()
            .with_width(SizeSpec::Fill)
            .with_child(build_section_header("GPU", theme))
            .with_child(Box::new(row)),
    )
}

/// Performance section: header with description (settings TBD).
fn build_performance_section(theme: &UiTheme) -> Box<dyn Widget> {
    build_section_header_with_description(
        "Performance",
        "Tuning options for high-throughput scenarios. Defaults are correct for most users.",
        theme,
    )
}
