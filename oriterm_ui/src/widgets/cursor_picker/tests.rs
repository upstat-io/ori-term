use winit::window::CursorIcon;

use crate::action::WidgetAction;
use crate::draw::Scene;
use crate::geometry::{Point, Rect};
use crate::input::{InputEvent, Modifiers, MouseButton};
use crate::layout::compute_layout;
use crate::sense::Sense;
use crate::widgets::tests::MockMeasurer;
use crate::widgets::{DrawCtx, LayoutCtx, Widget};

use super::{
    CARD_GAP, CARD_HEIGHT, CARD_WIDTH, CursorPickerWidget, DEMO_FONT_SIZE, LABEL_FONT_SIZE,
    TOTAL_WIDTH,
};

fn theme() -> &'static crate::theme::UiTheme {
    &super::super::tests::TEST_THEME
}

fn make_picker(selected: usize) -> CursorPickerWidget {
    CursorPickerWidget::new(selected, theme())
}

#[test]
fn new_stores_selection() {
    let p = make_picker(1);
    assert_eq!(p.selected(), 1);
}

#[test]
fn new_clamps_selection() {
    let p = make_picker(99);
    assert_eq!(p.selected(), 2);
}

#[test]
fn layout_dimensions() {
    let p = make_picker(0);
    let ctx = LayoutCtx {
        measurer: &MockMeasurer::STANDARD,
        theme: theme(),
    };
    let lb = p.layout(&ctx);
    let node = compute_layout(&lb, Rect::new(0.0, 0.0, 400.0, 300.0));
    assert_eq!(node.rect.width(), TOTAL_WIDTH);
    assert_eq!(node.rect.height(), CARD_HEIGHT);
}

#[test]
fn paint_renders_three_cards() {
    let p = make_picker(0);
    let measurer = MockMeasurer::STANDARD;
    let mut scene = Scene::new();
    let bounds = Rect::new(0.0, 0.0, TOTAL_WIDTH, CARD_HEIGHT);
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
    p.paint(&mut ctx);

    // 3 card backgrounds + 3 cursor demo elements = 6+ rects.
    let rects = scene.quads().len();
    assert!(
        rects >= 6,
        "3 cards + 3 cursor demos = 6+ rects, got {rects}"
    );
    // 3 labels + 3 demo characters = 6 text runs.
    assert_eq!(scene.text_runs().len(), 6, "3 labels + 3 demo chars");
}

#[test]
fn click_selects_card() {
    let mut p = make_picker(0);
    // Click on card 2 (last card).
    let cx = 2.0 * (CARD_WIDTH + CARD_GAP) + CARD_WIDTH / 2.0;
    let cy = CARD_HEIGHT / 2.0;
    let bounds = Rect::new(0.0, 0.0, TOTAL_WIDTH, CARD_HEIGHT);
    let event = InputEvent::MouseDown {
        pos: Point::new(cx, cy),
        button: MouseButton::Left,
        modifiers: Modifiers::NONE,
    };
    let result = p.on_input(&event, bounds);
    assert!(result.handled);
    assert_eq!(p.selected(), 2);
    match result.action {
        Some(WidgetAction::Selected { index, .. }) => assert_eq!(index, 2),
        other => panic!("expected Selected(2), got {other:?}"),
    }
}

#[test]
fn click_same_card_is_noop() {
    let mut p = make_picker(1);
    let cx = 1.0 * (CARD_WIDTH + CARD_GAP) + CARD_WIDTH / 2.0;
    let bounds = Rect::new(0.0, 0.0, TOTAL_WIDTH, CARD_HEIGHT);
    let event = InputEvent::MouseDown {
        pos: Point::new(cx, CARD_HEIGHT / 2.0),
        button: MouseButton::Left,
        modifiers: Modifiers::NONE,
    };
    let result = p.on_input(&event, bounds);
    assert!(!result.handled, "clicking already-selected card is a noop");
}

#[test]
fn sense_is_click() {
    let p = make_picker(0);
    assert_eq!(p.sense(), Sense::click());
}

#[test]
fn set_selected() {
    let mut p = make_picker(0);
    p.set_selected(2);
    assert_eq!(p.selected(), 2);
}

#[test]
fn set_selected_out_of_range_is_noop() {
    let mut p = make_picker(1);
    p.set_selected(99);
    assert_eq!(p.selected(), 1);
}

// -- Constant validation --

#[test]
fn cursor_picker_card_gap_is_24() {
    assert_eq!(CARD_GAP, 24.0);
}

#[test]
fn cursor_picker_demo_font_size_is_16() {
    assert_eq!(DEMO_FONT_SIZE, 16.0);
}

#[test]
fn cursor_picker_label_font_size_is_11() {
    assert_eq!(LABEL_FONT_SIZE, 11.0);
}

// -- Animator base colors --

#[test]
fn cursor_picker_paint_normal_bg_is_raised() {
    let p = make_picker(0);
    let animator = p.visual_states().expect("has animator");
    // Normal state bg should be bg_card (the raised surface color).
    assert_eq!(animator.get_bg_color(), theme().bg_card);
}

#[test]
fn layout_cursor_icon_pointer() {
    let p = make_picker(0);
    let ctx = LayoutCtx {
        measurer: &MockMeasurer::STANDARD,
        theme: theme(),
    };
    let layout = p.layout(&ctx);
    assert_eq!(
        layout.cursor_icon,
        CursorIcon::Pointer,
        "cursor picker should declare Pointer cursor"
    );
}

#[test]
fn cursor_picker_paint_active_bg_is_accent() {
    let p = make_picker(0);
    let measurer = MockMeasurer::STANDARD;
    let mut scene = Scene::new();
    let bounds = Rect::new(0.0, 0.0, TOTAL_WIDTH, CARD_HEIGHT);
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
    p.paint(&mut ctx);

    // Selected card (index 0) gets accent_bg background. The first quad
    // pushed is for card 0 — verify its fill color is accent_bg.
    let quads = scene.quads();
    assert!(!quads.is_empty(), "paint should emit quads");
    assert_eq!(
        quads[0].style.fill,
        Some(theme().accent_bg),
        "selected card bg should be accent_bg"
    );
}
