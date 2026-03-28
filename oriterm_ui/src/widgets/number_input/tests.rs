use winit::window::CursorIcon;

use crate::action::WidgetAction;
use crate::draw::Scene;
use crate::geometry::Rect;
use crate::input::{InputEvent, Key, Modifiers};
use crate::layout::compute_layout;
use crate::widgets::tests::MockMeasurer;
use crate::widgets::{DrawCtx, LayoutCtx, Widget};

use super::{
    BORDER_WIDTH, BUTTON_DIVIDER_WIDTH, BUTTON_PANEL_WIDTH, DEFAULT_INPUT_WIDTH, INPUT_HEIGHT,
    NumberInputWidget,
};

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
    let expected_w = DEFAULT_INPUT_WIDTH + BUTTON_PANEL_WIDTH + 2.0 * BORDER_WIDTH;
    assert_eq!(node.rect.width(), expected_w);
    assert_eq!(node.rect.height(), INPUT_HEIGHT);
}

// -- Paint --

#[test]
fn paint_produces_rect_and_text() {
    let w = make_input(42.0, 0.0, 100.0, 1.0);
    let measurer = MockMeasurer::STANDARD;
    let mut scene = Scene::new();
    let total_w = DEFAULT_INPUT_WIDTH + BUTTON_PANEL_WIDTH + 2.0 * BORDER_WIDTH;
    let bounds = Rect::new(0.0, 0.0, total_w, INPUT_HEIGHT);
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
    // 1 value text (arrows are now icons, not text runs).
    assert_eq!(scene.text_runs().len(), 1, "value text only");
}

/// TPR-13-010 regression: stepper arrows must not depend on Unicode glyph
/// coverage. When icons are available, paint produces icon entries instead
/// of text runs for the arrows.
#[test]
fn paint_stepper_arrows_use_icons_not_text() {
    use crate::icons::{IconId, ResolvedIcon, ResolvedIcons};

    let w = make_input(42.0, 0.0, 100.0, 1.0);
    let measurer = MockMeasurer::STANDARD;
    let mut scene = Scene::new();
    let total_w = DEFAULT_INPUT_WIDTH + BUTTON_PANEL_WIDTH + 2.0 * BORDER_WIDTH;
    let bounds = Rect::new(0.0, 0.0, total_w, INPUT_HEIGHT);

    // Provide resolved icons so the paint path can find them.
    let mut icons = ResolvedIcons::new();
    let dummy = ResolvedIcon {
        atlas_page: 0,
        uv: [0.0, 0.0, 0.1, 0.1],
    };
    icons.insert(IconId::StepperUp, 8, dummy);
    icons.insert(IconId::StepperDown, 8, dummy);

    let mut ctx = DrawCtx {
        measurer: &measurer,
        scene: &mut scene,
        bounds,
        now: std::time::Instant::now(),
        theme: theme(),
        icons: Some(&icons),
        interaction: None,
        widget_id: None,
        frame_requests: None,
    };
    w.paint(&mut ctx);

    // With icons resolved, we should get icon entries for the arrows.
    assert_eq!(
        scene.icons().len(),
        2,
        "should have 2 icon entries (up + down stepper arrows)"
    );
    // Still only 1 text run (the value label).
    assert_eq!(scene.text_runs().len(), 1, "value text only, no arrow text");
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

// -- Dimensional constants --

#[test]
fn number_input_default_width() {
    let expected = DEFAULT_INPUT_WIDTH + BUTTON_PANEL_WIDTH + 2.0 * BORDER_WIDTH;
    assert_eq!(expected, 82.0, "56 + 22 + 4 = 82");

    // Verify layout agrees.
    let w = make_input(0.0, 0.0, 100.0, 1.0);
    let ctx = LayoutCtx {
        measurer: &MockMeasurer::STANDARD,
        theme: theme(),
    };
    let lb = w.layout(&ctx);
    let node = compute_layout(&lb, Rect::new(0.0, 0.0, 400.0, 300.0));
    assert_eq!(node.rect.width(), 82.0);
}

#[test]
fn number_input_compact_width() {
    let w = make_input(0.0, 0.0, 100.0, 1.0).with_input_width(44.0);
    let ctx = LayoutCtx {
        measurer: &MockMeasurer::STANDARD,
        theme: theme(),
    };
    let lb = w.layout(&ctx);
    let node = compute_layout(&lb, Rect::new(0.0, 0.0, 400.0, 300.0));
    let expected = 44.0 + BUTTON_PANEL_WIDTH + 2.0 * BORDER_WIDTH;
    assert_eq!(expected, 70.0, "44 + 22 + 4 = 70");
    assert_eq!(node.rect.width(), 70.0);
}

#[test]
fn number_input_height_is_30() {
    assert_eq!(INPUT_HEIGHT, 30.0);

    let w = make_input(0.0, 0.0, 100.0, 1.0);
    let ctx = LayoutCtx {
        measurer: &MockMeasurer::STANDARD,
        theme: theme(),
    };
    let lb = w.layout(&ctx);
    let node = compute_layout(&lb, Rect::new(0.0, 0.0, 400.0, 300.0));
    assert_eq!(node.rect.height(), 30.0);
}

#[test]
fn number_input_border_width_is_2() {
    assert_eq!(BORDER_WIDTH, 2.0);
}

#[test]
fn number_input_stepper_panel_width() {
    assert_eq!(BUTTON_PANEL_WIDTH, 22.0);
}

#[test]
fn number_input_horizontal_divider_is_1px() {
    assert_eq!(BUTTON_DIVIDER_WIDTH, 1.0);
}

#[test]
fn number_input_arrow_keys_adjust_value() {
    let mut w = make_input(5.0, 0.0, 10.0, 1.0);

    // Arrow up increments.
    let up = InputEvent::KeyDown {
        key: Key::ArrowUp,
        modifiers: Modifiers::NONE,
    };
    let r = w.on_input(&up, Rect::new(0.0, 0.0, 82.0, 30.0));
    assert!(r.handled);
    assert_eq!(w.value(), 6.0);

    // Arrow down decrements.
    let down = InputEvent::KeyDown {
        key: Key::ArrowDown,
        modifiers: Modifiers::NONE,
    };
    let r = w.on_input(&down, Rect::new(0.0, 0.0, 82.0, 30.0));
    assert!(r.handled);
    assert_eq!(w.value(), 5.0);
}

// -- Cursor icon --

#[test]
fn layout_cursor_icon_pointer() {
    let w = make_input(0.0, 0.0, 100.0, 1.0);
    let ctx = LayoutCtx {
        measurer: &MockMeasurer::STANDARD,
        theme: theme(),
    };
    let layout = w.layout(&ctx);
    assert_eq!(
        layout.cursor_icon,
        CursorIcon::Pointer,
        "number input should declare Pointer cursor for stepper buttons"
    );
}
