//! Bell page builder — visual bell animation and duration.

use oriterm_ui::layout::SizeSpec;
use oriterm_ui::theme::UiTheme;
use oriterm_ui::widgets::Widget;
use oriterm_ui::widgets::container::ContainerWidget;
use oriterm_ui::widgets::dropdown::DropdownWidget;
use oriterm_ui::widgets::number_input::NumberInputWidget;
use oriterm_ui::widgets::setting_row::SettingRowWidget;
use oriterm_ui::widgets::toggle::ToggleWidget;

use crate::config::{BellAnimation, Config, NotificationMode, NotifyOnCommandFinish};

use super::shared::{
    build_section_header, build_section_header_with_description, build_settings_page,
};
use super::{BELL_DURATION_VALUES, SettingsIds};

/// Builds the Bell page content widget.
pub(super) fn build_page(
    config: &Config,
    ids: &mut SettingsIds,
    theme: &UiTheme,
) -> Box<dyn Widget> {
    build_settings_page(
        "Bell & Notifications",
        "Visual bell animation and desktop notification settings",
        vec![
            build_visual_section(config, ids, theme),
            build_throttle_section(theme),
            build_notifications_section(config, ids, theme),
        ],
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

    Box::new(
        ContainerWidget::column()
            .with_width(SizeSpec::Fill)
            .with_child(build_section_header("Visual Bell", theme))
            .with_child(Box::new(anim_row))
            .with_child(Box::new(dur_row)),
    )
}

/// Throttle section: header with description (settings TBD).
fn build_throttle_section(theme: &UiTheme) -> Box<dyn Widget> {
    build_section_header_with_description(
        "Throttle",
        "Suppress repeated bells to avoid visual noise from programs that ring rapidly.",
        theme,
    )
}

/// Desktop Notifications section: notification mode + command-completion gating.
///
/// BUG-11-016 — surfaces all four `behavior.*` notification fields to the
/// settings dialog: `notification` (mode), `notify_on_command_finish`
/// (when to fire on long-running commands), `notify_command_bell` (tab-bar
/// pulse on command-complete), `notify_command_threshold_secs` (minimum
/// command duration for the completion notification path).
fn build_notifications_section(
    config: &Config,
    ids: &mut SettingsIds,
    theme: &UiTheme,
) -> Box<dyn Widget> {
    // Notification mode (None / Visual / Sound / Both).
    let mode_items = vec![
        "None".to_owned(),
        "Visual only".to_owned(),
        "Sound only".to_owned(),
        "Both (default)".to_owned(),
    ];
    let mode_idx = match config.behavior.notification {
        NotificationMode::None => 0,
        NotificationMode::Visual => 1,
        NotificationMode::Sound => 2,
        NotificationMode::Both => 3,
    };
    let mode_dropdown = DropdownWidget::new(mode_items).with_selected(mode_idx);
    ids.notification_mode_dropdown = mode_dropdown.id();

    let mode_row = SettingRowWidget::new(
        "Notification mode",
        "How OSC 9 / 99 / 777 desktop notifications are presented",
        Box::new(mode_dropdown),
        theme,
    );

    // notify_on_command_finish (Never / Unfocused / Always).
    let finish_items = vec![
        "Never".to_owned(),
        "When pane unfocused".to_owned(),
        "Always".to_owned(),
    ];
    let finish_idx = match config.behavior.notify_on_command_finish {
        NotifyOnCommandFinish::Never => 0,
        NotifyOnCommandFinish::Unfocused => 1,
        NotifyOnCommandFinish::Always => 2,
    };
    let finish_dropdown = DropdownWidget::new(finish_items).with_selected(finish_idx);
    ids.notify_on_command_finish_dropdown = finish_dropdown.id();

    let finish_row = SettingRowWidget::new(
        "Notify on command finish",
        "Show a notification when a long-running command completes",
        Box::new(finish_dropdown),
        theme,
    );

    // notify_command_threshold_secs.
    let threshold_input = NumberInputWidget::new(
        config.behavior.notify_command_threshold_secs as f32,
        1.0,
        3600.0,
        1.0,
        theme,
    );
    ids.notify_command_threshold_input = threshold_input.id();

    let threshold_row = SettingRowWidget::new(
        "Threshold (seconds)",
        "Minimum command duration to trigger a completion notification",
        Box::new(threshold_input),
        theme,
    );

    // notify_command_bell (tab-bar pulse).
    let bell_toggle = ToggleWidget::new().with_on(config.behavior.notify_command_bell);
    ids.notify_command_bell_toggle = bell_toggle.id();

    let bell_row = SettingRowWidget::new(
        "Pulse tab bar",
        "Flash the tab bar when a long-running command completes",
        Box::new(bell_toggle),
        theme,
    );

    Box::new(
        ContainerWidget::column()
            .with_width(SizeSpec::Fill)
            .with_child(build_section_header("Desktop Notifications", theme))
            .with_child(Box::new(mode_row))
            .with_child(Box::new(finish_row))
            .with_child(Box::new(threshold_row))
            .with_child(Box::new(bell_row)),
    )
}
