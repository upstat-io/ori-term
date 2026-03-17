use crate::geometry::{Point, Rect};
use crate::input::{HoverEvent, Key, KeyEvent, Modifiers, MouseButton, MouseEvent, MouseEventKind};
use crate::layout::BoxContent;
use crate::widgets::tests::MockMeasurer;
use crate::widgets::{CaptureRequest, EventCtx, LayoutCtx, Widget, WidgetAction, WidgetResponse};

use super::{CheckboxStyle, CheckboxWidget};

static MEASURER: MockMeasurer = MockMeasurer::STANDARD;

fn event_ctx() -> EventCtx<'static> {
    EventCtx {
        measurer: &MEASURER,
        bounds: Rect::new(0.0, 0.0, 200.0, 20.0),
        is_focused: true,
        focused_widget: None,
        theme: &super::super::tests::TEST_THEME,
        interaction: None,
        widget_id: None,
        frame_requests: None,
    }
}

fn left_down() -> MouseEvent {
    MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        pos: Point::new(5.0, 5.0),
        modifiers: Modifiers::NONE,
    }
}

fn left_up() -> MouseEvent {
    MouseEvent {
        kind: MouseEventKind::Up(MouseButton::Left),
        pos: Point::new(5.0, 5.0),
        modifiers: Modifiers::NONE,
    }
}

fn space_key() -> KeyEvent {
    KeyEvent {
        key: Key::Space,
        modifiers: Modifiers::NONE,
    }
}

#[test]
fn default_state() {
    let cb = CheckboxWidget::new("Accept");
    assert!(!cb.is_checked());
    assert!(!cb.is_disabled());
    assert!(cb.is_focusable());
}

#[test]
fn with_checked_builder() {
    let cb = CheckboxWidget::new("X").with_checked(true);
    assert!(cb.is_checked());
}

#[test]
fn layout_dimensions() {
    let cb = CheckboxWidget::new("Check");
    let m = MockMeasurer::new();
    let ctx = LayoutCtx {
        measurer: &m,
        theme: &super::super::tests::TEST_THEME,
    };
    let layout = cb.layout(&ctx);
    let s = CheckboxStyle::default();

    if let BoxContent::Leaf {
        intrinsic_width,
        intrinsic_height,
    } = &layout.content
    {
        // "Check" = 5 * 8 = 40, box = 16, gap = 8 → 64.
        assert_eq!(*intrinsic_width, s.box_size + s.gap + 40.0);
        // max(box_size=16, line_height=16) = 16.
        assert_eq!(*intrinsic_height, 16.0);
    } else {
        panic!("expected leaf layout");
    }
}

#[test]
fn click_toggles() {
    let mut cb = CheckboxWidget::new("X");
    let ctx = event_ctx();

    // Down acquires capture, Up inside bounds toggles.
    let r = cb.handle_mouse(&left_down(), &ctx);
    assert_eq!(r.capture, CaptureRequest::Acquire);
    let r = cb.handle_mouse(&left_up(), &ctx);
    assert!(cb.is_checked());
    assert_eq!(r.capture, CaptureRequest::Release);
    assert_eq!(
        r.action,
        Some(WidgetAction::Toggled {
            id: cb.id(),
            value: true,
        })
    );

    let r = cb.handle_mouse(&left_down(), &ctx);
    assert_eq!(r.capture, CaptureRequest::Acquire);
    let r = cb.handle_mouse(&left_up(), &ctx);
    assert!(!cb.is_checked());
    assert_eq!(r.capture, CaptureRequest::Release);
    assert_eq!(
        r.action,
        Some(WidgetAction::Toggled {
            id: cb.id(),
            value: false,
        })
    );
}

#[test]
fn space_toggles() {
    let mut cb = CheckboxWidget::new("X");
    let ctx = event_ctx();

    let r = cb.handle_key(space_key(), &ctx);
    assert!(cb.is_checked());
    assert_eq!(
        r.action,
        Some(WidgetAction::Toggled {
            id: cb.id(),
            value: true,
        })
    );
}

#[test]
fn disabled_ignores_events() {
    let mut cb = CheckboxWidget::new("X").with_disabled(true);
    let ctx = event_ctx();

    assert!(!cb.is_focusable());

    let r = cb.handle_mouse(&left_down(), &ctx);
    assert_eq!(r, WidgetResponse::ignored());
    let r = cb.handle_mouse(&left_up(), &ctx);
    assert_eq!(r, WidgetResponse::ignored());

    let r = cb.handle_key(space_key(), &ctx);
    assert_eq!(r, WidgetResponse::ignored());

    let r = cb.handle_hover(HoverEvent::Enter, &ctx);
    assert_eq!(r, WidgetResponse::ignored());
}

#[test]
fn hover_returns_paint() {
    let mut cb = CheckboxWidget::new("X");
    let ctx = event_ctx();

    let r = cb.handle_hover(HoverEvent::Enter, &ctx);
    assert_eq!(r.response, crate::input::EventResponse::RequestPaint);

    let r = cb.handle_hover(HoverEvent::Leave, &ctx);
    assert_eq!(r.response, crate::input::EventResponse::RequestPaint);
}

#[test]
fn set_checked_programmatic() {
    let mut cb = CheckboxWidget::new("X");
    cb.set_checked(true);
    assert!(cb.is_checked());
    cb.set_checked(false);
    assert!(!cb.is_checked());
}

#[test]
fn enter_key_does_not_toggle() {
    let mut cb = CheckboxWidget::new("X");
    let ctx = event_ctx();

    // Only Space toggles a checkbox, not Enter.
    let r = cb.handle_key(
        KeyEvent {
            key: Key::Enter,
            modifiers: Modifiers::NONE,
        },
        &ctx,
    );
    assert_eq!(r, WidgetResponse::ignored());
    assert!(!cb.is_checked());
}

#[test]
fn right_click_ignored() {
    let mut cb = CheckboxWidget::new("X");
    let ctx = event_ctx();

    let right_click = MouseEvent {
        kind: MouseEventKind::Up(MouseButton::Right),
        pos: Point::new(5.0, 5.0),
        modifiers: Modifiers::NONE,
    };
    let r = cb.handle_mouse(&right_click, &ctx);
    assert_eq!(r, WidgetResponse::ignored());
    assert!(!cb.is_checked());
}

#[test]
fn rapid_toggle_sequence() {
    let mut cb = CheckboxWidget::new("X");
    let ctx = event_ctx();

    // Toggle 4 times rapidly (each toggle is Down + Up).
    for i in 0..4 {
        cb.handle_mouse(&left_down(), &ctx);
        cb.handle_mouse(&left_up(), &ctx);
        assert_eq!(cb.is_checked(), i % 2 == 0);
    }
}

#[test]
fn release_outside_bounds_no_toggle() {
    let mut cb = CheckboxWidget::new("X");
    let ctx = event_ctx();

    // Press inside, then release outside — should not toggle but release capture.
    cb.handle_mouse(&left_down(), &ctx);
    let outside_up = MouseEvent {
        kind: MouseEventKind::Up(MouseButton::Left),
        pos: Point::new(300.0, 300.0),
        modifiers: Modifiers::NONE,
    };
    let r = cb.handle_mouse(&outside_up, &ctx);
    assert!(!cb.is_checked());
    assert_eq!(r.capture, CaptureRequest::Release);
    assert!(r.action.is_none());
}

#[test]
fn set_disabled_affects_focusable() {
    let mut cb = CheckboxWidget::new("X");
    assert!(cb.is_focusable());
    cb.set_disabled(true);
    assert!(!cb.is_focusable());
}

#[test]
fn sense_returns_click() {
    let cb = CheckboxWidget::new("X");
    assert_eq!(cb.sense(), crate::sense::Sense::click());
}

#[test]
fn has_three_controllers() {
    let cb = CheckboxWidget::new("X");
    assert_eq!(cb.controllers().len(), 3);
}

#[test]
fn has_visual_state_animator() {
    let cb = CheckboxWidget::new("X");
    assert!(cb.visual_states().is_some());
}
