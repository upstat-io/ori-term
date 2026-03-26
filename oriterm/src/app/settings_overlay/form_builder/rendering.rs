//! Rendering page builder — GPU backend and text rendering settings.

use oriterm_ui::layout::SizeSpec;
use oriterm_ui::theme::UiTheme;
use oriterm_ui::widgets::Widget;
use oriterm_ui::widgets::container::ContainerWidget;
use oriterm_ui::widgets::dropdown::DropdownWidget;
use oriterm_ui::widgets::setting_row::SettingRowWidget;
use oriterm_ui::widgets::toggle::ToggleWidget;

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
        "GPU backend and text rendering options",
        vec![
            build_gpu_section(config, ids, theme),
            build_text_section(config, ids, theme),
            build_performance_section(theme),
        ],
        theme,
    )
}

/// GPU section: backend dropdown + restart notice.
fn build_gpu_section(config: &Config, ids: &mut SettingsIds, theme: &UiTheme) -> Box<dyn Widget> {
    let items = vec![
        "Auto".to_owned(),
        "Vulkan".to_owned(),
        "DirectX 12".to_owned(),
        "Metal".to_owned(),
    ];
    let idx = match config.rendering.gpu_backend {
        GpuBackend::Auto => 0,
        GpuBackend::Vulkan => 1,
        GpuBackend::DirectX12 => 2,
        GpuBackend::Metal => 3,
    };
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

/// Text section: subpixel rendering toggle.
fn build_text_section(config: &Config, ids: &mut SettingsIds, theme: &UiTheme) -> Box<dyn Widget> {
    // Subpixel mode: "rgb"/"bgr" = enabled, "none"/absent = disabled.
    let subpixel_on = config
        .font
        .subpixel_mode
        .as_ref()
        .is_some_and(|m| m != "none");
    let toggle = ToggleWidget::new().with_on(subpixel_on);
    ids.subpixel_toggle = toggle.id();

    let row = SettingRowWidget::new(
        "LCD subpixel rendering",
        "Sharper text on LCD displays (uses font subpixel mode)",
        Box::new(toggle),
        theme,
    );

    Box::new(
        ContainerWidget::column()
            .with_width(SizeSpec::Fill)
            .with_child(build_section_header("Text", theme))
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
