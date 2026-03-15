//! Tests for window chrome layout and control buttons.
//!
//! `WindowChromeWidget` container tests have been removed — the unified
//! tab-in-titlebar chrome routes events through `TabBarWidget`, not
//! `WindowChromeWidget`. `ChromeLayout` and `WindowControlButton` tests
//! remain: these types are still used for layout computation and button
//! rendering within the tab bar.

use crate::geometry::{Point, Rect};
use crate::input::{Modifiers, MouseButton, MouseEvent, MouseEventKind};
use crate::theme::UiTheme;
use crate::widgets::tests::MockMeasurer;
use crate::widgets::{EventCtx, Widget};

use super::constants::{
    CAPTION_HEIGHT, CAPTION_HEIGHT_MAXIMIZED, CONTROL_BUTTON_WIDTH, RESIZE_BORDER_WIDTH,
};
use super::controls::{ControlButtonColors, WindowControlButton};
use super::layout::{ChromeLayout, ControlKind};

// ── Test helpers ──

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

/// Left mouse button press at the given position.
fn left_down(x: f32, y: f32) -> MouseEvent {
    MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        pos: Point::new(x, y),
        modifiers: Modifiers::NONE,
    }
}

// ── ChromeLayout tests ──

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
        layout
            .interactive_rects
            .iter()
            .all(|r| *r == Rect::default())
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

// ── WindowControlButton tests ──

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
fn control_button_hover_sets_pressed() {
    let mut btn = WindowControlButton::new(ControlKind::MaximizeRestore, test_button_colors());
    assert!(!btn.is_pressed());

    let measurer = MockMeasurer::STANDARD;
    let theme = UiTheme::dark();
    let ctx = EventCtx {
        measurer: &measurer,
        bounds: Rect::new(0.0, 0.0, 46.0, 36.0),
        is_focused: false,
        focused_widget: None,
        theme: &theme,
        interaction: None,
        widget_id: None,
    };

    let event = left_down(23.0, 18.0);
    btn.handle_mouse(&event, &ctx);
    assert!(btn.is_pressed());
}
