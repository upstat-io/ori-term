//! Tests for the status bar widget.

use super::{STATUS_BAR_HEIGHT, StatusBarColors, StatusBarData, StatusBarWidget};
use crate::sense::Sense;
use crate::theme::UiTheme;
use crate::widgets::Widget;

#[test]
fn status_bar_default_data_is_all_empty() {
    let data = StatusBarData::default();
    assert!(data.shell_name.is_empty());
    assert!(data.pane_count.is_empty());
    assert!(data.grid_size.is_empty());
    assert!(data.encoding.is_empty());
    assert!(data.term_type.is_empty());
}

#[test]
fn status_bar_not_focusable() {
    let widget = StatusBarWidget::new(800.0, &UiTheme::dark());
    assert!(!widget.is_focusable());
}

#[test]
fn status_bar_sense_none() {
    let widget = StatusBarWidget::new(800.0, &UiTheme::dark());
    assert_eq!(widget.sense(), Sense::none());
}

#[test]
fn status_bar_theme_colors_dark() {
    let theme = UiTheme::dark();
    let colors = StatusBarColors::from_theme(&theme);
    assert_eq!(colors.bg, theme.bg_primary);
    assert_eq!(colors.border, theme.border);
    assert_eq!(colors.text, theme.fg_faint);
    assert_eq!(colors.accent, theme.accent);
}

#[test]
fn status_bar_theme_colors_light() {
    let theme = UiTheme::light();
    let colors = StatusBarColors::from_theme(&theme);
    assert_eq!(colors.bg, theme.bg_primary);
    assert_eq!(colors.border, theme.border);
    assert_eq!(colors.text, theme.fg_faint);
    assert_eq!(colors.accent, theme.accent);
}

#[test]
fn status_bar_data_update() {
    let mut widget = StatusBarWidget::new(800.0, &UiTheme::dark());
    widget.set_data(StatusBarData {
        shell_name: "zsh".into(),
        pane_count: "3 panes".into(),
        grid_size: "120\u{00d7}30".into(),
        encoding: "UTF-8".into(),
        term_type: "xterm-256color".into(),
    });
    // No panic — data accepted.
}

#[test]
fn status_bar_window_width_update() {
    let mut widget = StatusBarWidget::new(800.0, &UiTheme::dark());
    widget.set_window_width(1200.0);
    // No panic — width accepted.
}

#[test]
fn status_bar_height_constant() {
    assert_eq!(STATUS_BAR_HEIGHT, 22.0);
}

#[test]
fn status_bar_layout_fixed_height() {
    use crate::testing::WidgetTestHarness;

    let widget = StatusBarWidget::new(800.0, &UiTheme::dark());
    let mut h = WidgetTestHarness::new(widget);
    let scene = h.render();
    assert!(!scene.is_empty());
}

#[test]
fn status_bar_renders_scene() {
    use crate::testing::WidgetTestHarness;

    let mut widget = StatusBarWidget::new(800.0, &UiTheme::dark());
    widget.set_data(StatusBarData {
        shell_name: "zsh".into(),
        pane_count: "1 pane".into(),
        grid_size: "80\u{00d7}24".into(),
        encoding: "UTF-8".into(),
        term_type: "xterm-256color".into(),
    });
    let mut h = WidgetTestHarness::new(widget);
    let scene = h.render();
    assert!(!scene.is_empty(), "status bar should produce draw commands");
}

#[test]
fn status_bar_empty_data_no_crash() {
    use crate::testing::WidgetTestHarness;

    let widget = StatusBarWidget::new(800.0, &UiTheme::dark());
    let mut h = WidgetTestHarness::new(widget);
    let scene = h.render();
    // Background + border quads should still render.
    assert!(!scene.is_empty());
}
