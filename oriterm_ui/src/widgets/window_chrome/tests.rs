//! Tests for window chrome layout and control buttons.
//!
//! `WindowChromeWidget` container tests have been removed -- the unified
//! tab-in-titlebar chrome routes events through `TabBarWidget`, not
//! `WindowChromeWidget`. `ChromeLayout` and `WindowControlButton` tests
//! remain: these types are still used for layout computation and button
//! rendering within the tab bar.

use crate::sense::Sense;
use crate::theme::UiTheme;
use crate::widgets::Widget;

use super::constants::{
    CAPTION_HEIGHT, CAPTION_HEIGHT_MAXIMIZED, CONTROL_BUTTON_WIDTH, RESIZE_BORDER_WIDTH,
};
use super::controls::{ControlButtonColors, WindowControlButton};
use super::layout::{ChromeLayout, ChromeMode, ControlKind};

// -- Test helpers --

/// Standard button colors for tests.
fn test_button_colors() -> ControlButtonColors {
    let theme = UiTheme::dark();
    ControlButtonColors {
        fg: crate::color::Color::WHITE,
        bg: crate::color::Color::TRANSPARENT,
        hover_bg: crate::color::Color::WHITE,
        close_hover_bg: theme.close_hover_bg,
        close_pressed_bg: theme.close_pressed_bg,
    }
}

// -- ChromeLayout tests --

#[test]
fn layout_restored_caption_height() {
    let layout = ChromeLayout::compute(800.0, false, false);
    assert_eq!(layout.caption_height, CAPTION_HEIGHT);
    assert!(layout.visible);
}

#[test]
fn layout_maximized_caption_height() {
    let layout = ChromeLayout::compute(800.0, true, false);
    assert_eq!(layout.caption_height, CAPTION_HEIGHT_MAXIMIZED);
    assert!(layout.visible);
}

#[test]
fn layout_fullscreen_hidden() {
    let layout = ChromeLayout::compute(800.0, false, true);
    assert_eq!(layout.caption_height, 0.0);
    assert!(!layout.visible);
    assert!(
        layout.interactive_rects.is_empty(),
        "fullscreen hidden chrome should have no interactive rects"
    );
}

#[test]
fn layout_three_control_buttons() {
    let layout = ChromeLayout::compute(800.0, false, false);
    assert_eq!(layout.controls.len(), 3);
    assert_eq!(layout.controls[0].kind, ControlKind::Minimize);
    assert_eq!(layout.controls[1].kind, ControlKind::MaximizeRestore);
    assert_eq!(layout.controls[2].kind, ControlKind::Close);
}

#[test]
fn layout_close_button_at_right_edge() {
    let width = 1024.0;
    let layout = ChromeLayout::compute(width, false, false);
    let close = layout.controls[2].rect;
    let expected_right = width;
    let epsilon = 0.001;
    assert!((close.right() - expected_right).abs() < epsilon);
    assert_eq!(close.width(), CONTROL_BUTTON_WIDTH);
}

#[test]
fn layout_buttons_ordered_right_to_left() {
    let layout = ChromeLayout::compute(1000.0, false, false);
    let min_x = layout.controls[0].rect.x();
    let max_x = layout.controls[1].rect.x();
    let close_x = layout.controls[2].rect.x();
    assert!(min_x < max_x);
    assert!(max_x < close_x);
}

#[test]
fn layout_buttons_span_full_caption_height() {
    let layout = ChromeLayout::compute(800.0, false, false);
    for ctrl in &layout.controls {
        assert_eq!(ctrl.rect.height(), CAPTION_HEIGHT);
    }
}

#[test]
fn layout_maximized_buttons_span_full_caption_height() {
    let layout = ChromeLayout::compute(800.0, true, false);
    for ctrl in &layout.controls {
        assert_eq!(ctrl.rect.height(), CAPTION_HEIGHT_MAXIMIZED);
    }
}

#[test]
fn layout_title_rect_before_buttons() {
    let layout = ChromeLayout::compute(800.0, false, false);
    let title = layout.title_rect;
    let first_button = layout.controls[0].rect;
    assert_eq!(title.x(), RESIZE_BORDER_WIDTH);
    assert!(title.right() <= first_button.x() + 0.001);
}

#[test]
fn layout_interactive_rects_match_controls() {
    let layout = ChromeLayout::compute(800.0, false, false);
    assert_eq!(layout.interactive_rects.len(), 3);
    for (i, rect) in layout.interactive_rects.iter().enumerate() {
        assert_eq!(*rect, layout.controls[i].rect);
    }
}

#[test]
fn layout_narrow_window_title_rect_zero() {
    // Window too narrow for title (buttons take up most space).
    let width = CONTROL_BUTTON_WIDTH * 3.0 + 1.0;
    let layout = ChromeLayout::compute(width, false, false);
    assert!(layout.title_rect.width() >= 0.0);
}

// -- WindowControlButton tests --

#[test]
fn control_button_kind() {
    let btn = WindowControlButton::new(ControlKind::Close, test_button_colors());
    assert_eq!(btn.kind(), ControlKind::Close);
}

#[test]
fn control_button_not_focusable() {
    let btn = WindowControlButton::new(ControlKind::Minimize, test_button_colors());
    assert!(!btn.is_focusable());
}

#[test]
fn sense_returns_click() {
    let btn = WindowControlButton::new(ControlKind::Close, test_button_colors());
    assert_eq!(btn.sense(), Sense::click());
}

#[test]
fn has_two_controllers() {
    let btn = WindowControlButton::new(ControlKind::Minimize, test_button_colors());
    assert_eq!(btn.controllers().len(), 2);
}

#[test]
fn has_visual_state_animator() {
    let btn = WindowControlButton::new(ControlKind::Close, test_button_colors());
    assert!(btn.visual_states().is_some());
}

// -- ChromeMode::Dialog layout tests --

#[test]
fn layout_dialog_mode_single_close_button() {
    let layout = ChromeLayout::compute_with_mode(800.0, false, false, ChromeMode::Dialog);
    assert_eq!(
        layout.controls.len(),
        1,
        "dialog mode should have 1 control"
    );
    assert_eq!(layout.controls[0].kind, ControlKind::Close);
    assert_eq!(layout.interactive_rects.len(), 1);
    assert_eq!(layout.caption_height, CAPTION_HEIGHT);
    assert!(layout.visible);
    assert_eq!(layout.mode, ChromeMode::Dialog);
}

#[test]
fn layout_dialog_close_at_right_edge() {
    let width = 600.0;
    let layout = ChromeLayout::compute_with_mode(width, false, false, ChromeMode::Dialog);
    let close = layout.controls[0].rect;
    let epsilon = 0.001;
    assert!((close.right() - width).abs() < epsilon);
    assert_eq!(close.width(), CONTROL_BUTTON_WIDTH);
}

#[test]
fn layout_dialog_title_wider_than_full() {
    let width = 800.0;
    let full = ChromeLayout::compute_with_mode(width, false, false, ChromeMode::Full);
    let dialog = ChromeLayout::compute_with_mode(width, false, false, ChromeMode::Dialog);
    // Dialog has 1 button vs 3, so title area should be wider.
    assert!(
        dialog.title_rect.width() > full.title_rect.width(),
        "dialog title ({}) should be wider than full title ({})",
        dialog.title_rect.width(),
        full.title_rect.width(),
    );
}

// -- Control button click cycle test --

#[test]
fn click_cycle_emits_clicked_action() {
    use std::time::Instant;

    use crate::action::WidgetAction;
    use crate::geometry::Point;
    use crate::input::{InputEvent, Modifiers, MouseButton};

    use super::WindowChromeWidget;

    let mut chrome = WindowChromeWidget::new("Test", 800.0);
    let now = Instant::now();

    // Target the close button (last control).
    let close_rect = chrome.interactive_rects().last().copied().unwrap();
    let center = Point::new(
        close_rect.x() + close_rect.width() / 2.0,
        close_rect.y() + close_rect.height() / 2.0,
    );

    // Mouse-down should not emit an action yet.
    let down = InputEvent::MouseDown {
        pos: center,
        button: MouseButton::Left,
        modifiers: Modifiers::NONE,
    };
    let result_down = chrome.dispatch_input(&down, now);
    assert!(
        result_down.actions.is_empty(),
        "mouse-down alone should not emit actions"
    );

    // Mouse-up completes the click and emits Clicked.
    let up = InputEvent::MouseUp {
        pos: center,
        button: MouseButton::Left,
        modifiers: Modifiers::NONE,
    };
    let result_up = chrome.dispatch_input(&up, now);
    assert_eq!(
        result_up.actions.len(),
        1,
        "mouse-up should emit exactly one action"
    );
    assert!(
        matches!(result_up.actions[0], WidgetAction::Clicked(_)),
        "action should be Clicked, got {:?}",
        result_up.actions[0],
    );
}

// -- WindowChromeWidget draw output test --

#[test]
fn chrome_paint_produces_scene_output() {
    use crate::testing::WidgetTestHarness;

    use super::WindowChromeWidget;

    let chrome = WindowChromeWidget::new("Paint Test", 800.0);
    let mut h = WidgetTestHarness::new(chrome);
    let scene = h.render();

    // Chrome should produce at least a caption background rect.
    let rects = crate::testing::render_assert::rects(&scene);
    assert!(
        !rects.is_empty(),
        "chrome should paint at least the caption background"
    );
}
