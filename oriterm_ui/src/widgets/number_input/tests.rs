use crate::action::WidgetAction;
use crate::draw::Scene;
use crate::geometry::Rect;
use crate::input::{InputEvent, Key, Modifiers};
use crate::layout::compute_layout;
use crate::widgets::tests::MockMeasurer;
use crate::widgets::{DrawCtx, LayoutCtx, Widget};

use super::{INPUT_HEIGHT, INPUT_WIDTH, NumberInputWidget};

fn theme() -> &'static crate::theme::UiTheme {
    &super::super::tests::TEST_THEME
}

fn make_input(value: f32, min: f32, max: f32, step: f32) -> NumberInputWidget {
    NumberInputWidget::new(value, min, max, step, theme())
}

// -- Construction --

#[test]
fn new_clamps_value() {
    let w = make_input(150.0, 0.0, 100.0, 1.0);
    assert_eq!(w.value(), 100.0);
}

#[test]
fn new_stores_range() {
    let w = make_input(50.0, 10.0, 90.0, 5.0);
    assert_eq!(w.min(), 10.0);
    assert_eq!(w.max(), 90.0);
    assert_eq!(w.value(), 50.0);
}

// -- Value adjustment --

#[test]
fn arrow_up_increments() {
    let mut w = make_input(5.0, 0.0, 10.0, 1.0);
    let event = InputEvent::KeyDown {
        key: Key::ArrowUp,
        modifiers: Modifiers::NONE,
    };
    let result = w.on_input(&event, Rect::new(0.0, 0.0, 0.0, 0.0));
    assert!(result.handled);
    assert_eq!(w.value(), 6.0);
    match result.action {
        Some(WidgetAction::ValueChanged { value, .. }) => assert_eq!(value, 6.0),
        other => panic!("expected ValueChanged, got {other:?}"),
    }
}

#[test]
fn arrow_down_decrements() {
    let mut w = make_input(5.0, 0.0, 10.0, 1.0);
    let event = InputEvent::KeyDown {
        key: Key::ArrowDown,
        modifiers: Modifiers::NONE,
    };
    let result = w.on_input(&event, Rect::new(0.0, 0.0, 0.0, 0.0));
    assert!(result.handled);
    assert_eq!(w.value(), 4.0);
}

#[test]
fn clamps_at_max() {
    let mut w = make_input(10.0, 0.0, 10.0, 1.0);
    let event = InputEvent::KeyDown {
        key: Key::ArrowUp,
        modifiers: Modifiers::NONE,
    };
    let result = w.on_input(&event, Rect::new(0.0, 0.0, 0.0, 0.0));
    assert!(result.handled);
    assert_eq!(w.value(), 10.0);
    assert!(result.action.is_none(), "no change = no action");
}

#[test]
fn clamps_at_min() {
    let mut w = make_input(0.0, 0.0, 10.0, 1.0);
    let event = InputEvent::KeyDown {
        key: Key::ArrowDown,
        modifiers: Modifiers::NONE,
    };
    let result = w.on_input(&event, Rect::new(0.0, 0.0, 0.0, 0.0));
    assert!(result.handled);
    assert_eq!(w.value(), 0.0);
    assert!(result.action.is_none());
}

#[test]
fn set_value_clamps() {
    let mut w = make_input(5.0, 0.0, 10.0, 1.0);
    w.set_value(15.0);
    assert_eq!(w.value(), 10.0);
    w.set_value(-5.0);
    assert_eq!(w.value(), 0.0);
}

// -- Layout --

#[test]
fn layout_dimensions() {
    let w = make_input(0.0, 0.0, 100.0, 1.0);
    let ctx = LayoutCtx {
        measurer: &MockMeasurer::STANDARD,
        theme: theme(),
    };
    let lb = w.layout(&ctx);
    let node = compute_layout(&lb, Rect::new(0.0, 0.0, 400.0, 300.0));
    assert_eq!(node.rect.width(), INPUT_WIDTH);
    assert_eq!(node.rect.height(), INPUT_HEIGHT);
}

// -- Paint --

#[test]
fn paint_produces_rect_and_text() {
    let w = make_input(42.0, 0.0, 100.0, 1.0);
    let measurer = MockMeasurer::STANDARD;
    let mut scene = Scene::new();
    let bounds = Rect::new(0.0, 0.0, INPUT_WIDTH, INPUT_HEIGHT);
    let mut ctx = DrawCtx {
        measurer: &measurer,
        scene: &mut scene,
        bounds,
        now: std::time::Instant::now(),
        theme: theme(),
        icons: None,
        interaction: None,
        widget_id: None,
        frame_requests: None,
    };
    w.paint(&mut ctx);

    // 1 background + 2 dividers = 3 quads.
    assert_eq!(
        scene.quads().len(),
        3,
        "background + vertical divider + horizontal divider"
    );
    // 1 value text + 2 arrow labels = 3 text runs.
    assert_eq!(
        scene.text_runs().len(),
        3,
        "value text + up arrow + down arrow"
    );
}

// -- Sense & focusability --

#[test]
fn sense_includes_click_and_focus() {
    let w = make_input(0.0, 0.0, 100.0, 1.0);
    let s = w.sense();
    assert!(s.has_click());
    assert!(s.has_focus());
}

#[test]
fn is_focusable() {
    let w = make_input(0.0, 0.0, 100.0, 1.0);
    assert!(w.is_focusable());
}

// -- Format --

#[test]
fn format_integer_step() {
    let w = make_input(42.0, 0.0, 100.0, 1.0);
    assert_eq!(w.format_value(), "42");
}

#[test]
fn format_decimal_step() {
    let w = make_input(3.5, 0.0, 10.0, 0.5);
    assert_eq!(w.format_value(), "3.5");
}

#[test]
fn format_fine_step() {
    let w = make_input(1.25, 0.0, 5.0, 0.05);
    assert_eq!(w.format_value(), "1.25");
}
