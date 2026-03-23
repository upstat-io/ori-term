//! Bell page builder — visual bell animation and duration.

use oriterm_ui::layout::SizeSpec;
use oriterm_ui::theme::UiTheme;
use oriterm_ui::widgets::Widget;
use oriterm_ui::widgets::container::ContainerWidget;
use oriterm_ui::widgets::dropdown::DropdownWidget;
use oriterm_ui::widgets::setting_row::SettingRowWidget;

use crate::config::{BellAnimation, Config};

use super::appearance::{ROW_GAP, build_settings_page, section_title};
use super::{BELL_DURATION_VALUES, SettingsIds};

/// Builds the Bell page content widget.
pub(super) fn build_page(
    config: &Config,
    ids: &mut SettingsIds,
    theme: &UiTheme,
) -> Box<dyn Widget> {
    build_settings_page(
        "BELL",
        "Visual bell animation settings",
        vec![build_visual_section(config, ids, theme)],
        theme,
    )
}

/// Visual Bell section: animation + duration dropdowns.
fn build_visual_section(
    config: &Config,
    ids: &mut SettingsIds,
    theme: &UiTheme,
) -> Box<dyn Widget> {
    let anim_items = vec![
        "Ease Out".to_owned(),
        "Linear".to_owned(),
        "None".to_owned(),
    ];
    let anim_idx = match config.bell.animation {
        BellAnimation::EaseOut => 0,
        BellAnimation::Linear => 1,
        BellAnimation::None => 2,
    };
    let anim_dropdown = DropdownWidget::new(anim_items).with_selected(anim_idx);
    ids.bell_animation_dropdown = anim_dropdown.id();

    let anim_row = SettingRowWidget::new(
        "Animation",
        "Visual bell easing curve",
        Box::new(anim_dropdown),
        theme,
    );

    let dur_items: Vec<String> = BELL_DURATION_VALUES
        .iter()
        .map(|v| {
            if *v == 0 {
                "Off".to_owned()
            } else {
                format!("{v}ms")
            }
        })
        .collect();
    let dur_idx = BELL_DURATION_VALUES
        .iter()
        .position(|v| *v == config.bell.duration_ms)
        .unwrap_or(3); // Default to 150ms.
    let dur_dropdown = DropdownWidget::new(dur_items).with_selected(dur_idx);
    ids.bell_duration_dropdown = dur_dropdown.id();

    let dur_row = SettingRowWidget::new(
        "Duration",
        "How long the visual bell flash lasts",
        Box::new(dur_dropdown),
        theme,
    );

    let title = section_title("Visual Bell", theme);
    Box::new(
        ContainerWidget::column()
            .with_width(SizeSpec::Fill)
            .with_gap(ROW_GAP)
            .with_child(title)
            .with_child(Box::new(anim_row))
            .with_child(Box::new(dur_row)),
    )
}
