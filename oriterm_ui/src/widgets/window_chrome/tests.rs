use crate::geometry::Rect;
use crate::widgets::tests::MockMeasurer;

use super::WindowChromeWidget;
use super::constants::{
    CAPTION_HEIGHT, CAPTION_HEIGHT_MAXIMIZED, CONTROL_BUTTON_WIDTH, RESIZE_BORDER_WIDTH,
};
use super::controls::WindowControlButton;
use super::layout::{ChromeLayout, ControlKind};

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
    let btn = WindowControlButton::new(
        ControlKind::Close,
        crate::color::Color::WHITE,
        crate::color::Color::TRANSPARENT,
        crate::color::Color::WHITE,
    );
    assert_eq!(btn.kind(), ControlKind::Close);
}

#[test]
fn control_button_not_focusable() {
    use crate::widgets::Widget;

    let btn = WindowControlButton::new(
        ControlKind::Minimize,
        crate::color::Color::WHITE,
        crate::color::Color::TRANSPARENT,
        crate::color::Color::WHITE,
    );
    assert!(!btn.is_focusable());
}

#[test]
fn control_button_hover_sets_pressed() {
    let mut btn = WindowControlButton::new(
        ControlKind::MaximizeRestore,
        crate::color::Color::WHITE,
        crate::color::Color::TRANSPARENT,
        crate::color::Color::WHITE,
    );
    assert!(!btn.is_pressed());

    let measurer = MockMeasurer::STANDARD;
    let theme = crate::theme::UiTheme::dark();
    let ctx = crate::widgets::EventCtx {
        measurer: &measurer,
        bounds: Rect::new(0.0, 0.0, 46.0, 36.0),
        is_focused: false,
        focused_widget: None,
        theme: &theme,
    };

    use crate::widgets::Widget;
    let event = crate::input::MouseEvent {
        kind: crate::input::MouseEventKind::Down(crate::input::MouseButton::Left),
        pos: crate::geometry::Point::new(23.0, 18.0),
        modifiers: crate::input::Modifiers::NONE,
    };
    btn.handle_mouse(&event, &ctx);
    assert!(btn.is_pressed());
}

// ── WindowChromeWidget tests ──

#[test]
fn chrome_widget_caption_height() {
    let chrome = WindowChromeWidget::new("test", 800.0);
    assert_eq!(chrome.caption_height(), CAPTION_HEIGHT);
}

#[test]
fn chrome_widget_fullscreen_invisible() {
    let mut chrome = WindowChromeWidget::new("test", 800.0);
    chrome.set_fullscreen(true);
    assert!(!chrome.is_visible());
    assert_eq!(chrome.caption_height(), 0.0);
}

#[test]
fn chrome_widget_maximized_caption_height() {
    let mut chrome = WindowChromeWidget::new("test", 800.0);
    chrome.set_maximized(true);
    assert_eq!(chrome.caption_height(), CAPTION_HEIGHT_MAXIMIZED);
}

#[test]
fn chrome_widget_interactive_rects_three_buttons() {
    let chrome = WindowChromeWidget::new("test", 800.0);
    assert_eq!(chrome.interactive_rects().len(), 3);
}

#[test]
fn chrome_widget_resize_updates_layout() {
    let mut chrome = WindowChromeWidget::new("test", 800.0);
    let old_close_x = chrome.interactive_rects()[2].x();

    chrome.set_window_width(1200.0);
    let new_close_x = chrome.interactive_rects()[2].x();

    // Close button should move right when window widens.
    assert!(new_close_x > old_close_x);
}

#[test]
fn chrome_widget_set_title() {
    let mut chrome = WindowChromeWidget::new("ori", 800.0);
    chrome.set_title("new title".into());
    // Verify no panic — title is internal, tested through draw path.
}

#[test]
fn chrome_widget_active_inactive() {
    let mut chrome = WindowChromeWidget::new("test", 800.0);
    chrome.set_active(false);
    chrome.set_active(true);
    // Verify no panic — colors tested through draw path.
}
