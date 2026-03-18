//! Rendering page builder — GPU backend and text rendering settings.

use oriterm_ui::layout::SizeSpec;
use oriterm_ui::theme::UiTheme;
use oriterm_ui::widgets::Widget;
use oriterm_ui::widgets::container::ContainerWidget;
use oriterm_ui::widgets::dropdown::DropdownWidget;
use oriterm_ui::widgets::label::{LabelStyle, LabelWidget};
use oriterm_ui::widgets::scroll::ScrollWidget;
use oriterm_ui::widgets::setting_row::SettingRowWidget;
use oriterm_ui::widgets::toggle::ToggleWidget;

use crate::config::{Config, GpuBackend};

use super::SettingsIds;
use super::appearance::{
    DESC_FONT_SIZE, PAGE_PADDING, ROW_GAP, SECTION_GAP, TITLE_FONT_SIZE, section_title,
};

/// Builds the Rendering page content widget.
pub(super) fn build_page(
    config: &Config,
    ids: &mut SettingsIds,
    theme: &UiTheme,
) -> Box<dyn Widget> {
    let header = build_header(theme);
    let gpu = build_gpu_section(config, ids, theme);
    let text = build_text_section(config, ids, theme);

    let page = ContainerWidget::column()
        .with_width(SizeSpec::Fill)
        .with_padding(PAGE_PADDING)
        .with_gap(SECTION_GAP)
        .with_child(header)
        .with_child(gpu)
        .with_child(text);

    let mut scroll = ScrollWidget::vertical(Box::new(page));
    scroll.set_height(SizeSpec::Fill);
    Box::new(scroll)
}

/// Page header.
fn build_header(theme: &UiTheme) -> Box<dyn Widget> {
    let title = LabelWidget::new("Rendering").with_style(LabelStyle {
        font_size: TITLE_FONT_SIZE,
        color: theme.fg_primary,
        ..LabelStyle::from_theme(theme)
    });
    let desc = LabelWidget::new("GPU backend and text rendering options").with_style(LabelStyle {
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

    let title = section_title("GPU", theme);
    Box::new(
        ContainerWidget::column()
            .with_width(SizeSpec::Fill)
            .with_gap(ROW_GAP)
            .with_child(title)
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

    let title = section_title("Text", theme);
    Box::new(
        ContainerWidget::column()
            .with_width(SizeSpec::Fill)
            .with_gap(ROW_GAP)
            .with_child(title)
            .with_child(Box::new(row)),
    )
}
