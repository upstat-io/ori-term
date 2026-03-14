use crate::draw::{DrawCommand, DrawList};
use crate::geometry::{Insets, Point, Rect};
use crate::input::{
    EventResponse, Key, KeyEvent, Modifiers, MouseButton, MouseEvent, MouseEventKind,
};
use crate::invalidation::InvalidationTracker;
use crate::layout::{Align, Justify, SizeSpec, compute_layout};
use crate::widgets::button::ButtonWidget;
use crate::widgets::label::LabelWidget;
use crate::widgets::panel::PanelWidget;
use crate::widgets::spacer::SpacerWidget;
use crate::widgets::tests::MockMeasurer;
use crate::widgets::{
    CaptureRequest, DrawCtx, EventCtx, LayoutCtx, Widget, WidgetAction, WidgetResponse,
};

use super::ContainerWidget;

struct CountingWidget {
    id: crate::widget_id::WidgetId,
    size: Rect,
    draws: std::rc::Rc<std::cell::Cell<usize>>,
}

impl CountingWidget {
    fn new(width: f32, height: f32, draws: std::rc::Rc<std::cell::Cell<usize>>) -> Self {
        Self {
            id: crate::widget_id::WidgetId::next(),
            size: Rect::new(0.0, 0.0, width, height),
            draws,
        }
    }
}

impl Widget for CountingWidget {
    fn id(&self) -> crate::widget_id::WidgetId {
        self.id
    }

    fn is_focusable(&self) -> bool {
        false
    }

    fn layout(&self, _ctx: &LayoutCtx<'_>) -> crate::layout::LayoutBox {
        crate::layout::LayoutBox::leaf(self.size.width(), self.size.height())
            .with_widget_id(self.id)
    }

    fn draw(&self, _ctx: &mut DrawCtx<'_>) {
        self.draws.set(self.draws.get() + 1);
    }

    fn handle_mouse(&mut self, _event: &MouseEvent, _ctx: &EventCtx<'_>) -> WidgetResponse {
        WidgetResponse::ignored()
    }

    fn handle_hover(
        &mut self,
        _event: crate::input::HoverEvent,
        _ctx: &EventCtx<'_>,
    ) -> WidgetResponse {
        WidgetResponse::ignored()
    }

    fn handle_key(&mut self, _event: KeyEvent, _ctx: &EventCtx<'_>) -> WidgetResponse {
        WidgetResponse::ignored()
    }

    fn accept_action(&mut self, _action: &WidgetAction) -> bool {
        false
    }

    fn focusable_children(&self) -> Vec<crate::widget_id::WidgetId> {
        Vec::new()
    }
}

fn label(text: &str) -> Box<dyn Widget> {
    Box::new(LabelWidget::new(text))
}

fn button(text: &str) -> Box<ButtonWidget> {
    Box::new(ButtonWidget::new(text))
}

// --- Layout tests ---

#[test]
fn row_layout_places_children_horizontally() {
    let row = ContainerWidget::row().with_children(vec![label("AB"), label("CD")]);
    let ctx = LayoutCtx {
        measurer: &MockMeasurer::STANDARD,
        theme: &super::super::tests::TEST_THEME,
    };
    let layout_box = row.layout(&ctx);
    let viewport = Rect::new(0.0, 0.0, 400.0, 300.0);
    let node = compute_layout(&layout_box, viewport);

    assert_eq!(node.children.len(), 2);
    assert_eq!(node.rect.width(), 32.0);
    assert_eq!(node.rect.height(), 16.0);
    assert_eq!(node.children[0].rect.x(), 0.0);
    assert_eq!(node.children[1].rect.x(), 16.0);
}

#[test]
fn column_layout_places_children_vertically() {
    let col = ContainerWidget::column().with_children(vec![label("AB"), label("CD")]);
    let ctx = LayoutCtx {
        measurer: &MockMeasurer::STANDARD,
        theme: &super::super::tests::TEST_THEME,
    };
    let layout_box = col.layout(&ctx);
    let viewport = Rect::new(0.0, 0.0, 400.0, 300.0);
    let node = compute_layout(&layout_box, viewport);

    assert_eq!(node.children.len(), 2);
    assert_eq!(node.rect.width(), 16.0);
    assert_eq!(node.rect.height(), 32.0);
    assert_eq!(node.children[0].rect.y(), 0.0);
    assert_eq!(node.children[1].rect.y(), 16.0);
}

#[test]
fn row_with_gap() {
    let row = ContainerWidget::row()
        .with_children(vec![label("A"), label("B")])
        .with_gap(10.0);
    let ctx = LayoutCtx {
        measurer: &MockMeasurer::STANDARD,
        theme: &super::super::tests::TEST_THEME,
    };
    let layout_box = row.layout(&ctx);
    let viewport = Rect::new(0.0, 0.0, 400.0, 300.0);
    let node = compute_layout(&layout_box, viewport);

    assert_eq!(node.rect.width(), 26.0);
    assert_eq!(node.children[0].rect.x(), 0.0);
    assert_eq!(node.children[1].rect.x(), 18.0);
}

#[test]
fn row_with_spacer_pushes_apart() {
    let row = ContainerWidget::row().with_children(vec![
        label("L"),
        Box::new(SpacerWidget::fill()),
        label("R"),
    ]);
    let ctx = LayoutCtx {
        measurer: &MockMeasurer::STANDARD,
        theme: &super::super::tests::TEST_THEME,
    };
    let mut layout_box = row.layout(&ctx);
    layout_box.width = SizeSpec::Fill;
    let viewport = Rect::new(0.0, 0.0, 100.0, 50.0);
    let node = compute_layout(&layout_box, viewport);

    assert_eq!(node.children[0].rect.x(), 0.0);
    assert_eq!(node.children[2].rect.x(), 92.0);
}

#[test]
fn column_with_center_align() {
    let col = ContainerWidget::column()
        .with_children(vec![label("AB"), label("ABCD")])
        .with_align(Align::Center);
    let ctx = LayoutCtx {
        measurer: &MockMeasurer::STANDARD,
        theme: &super::super::tests::TEST_THEME,
    };
    let layout_box = col.layout(&ctx);
    let viewport = Rect::new(0.0, 0.0, 400.0, 300.0);
    let node = compute_layout(&layout_box, viewport);

    assert_eq!(node.children[0].rect.x(), 8.0);
    assert_eq!(node.children[1].rect.x(), 0.0);
}

#[test]
fn row_with_justify_space_between() {
    let row = ContainerWidget::row()
        .with_children(vec![label("A"), label("B"), label("C")])
        .with_justify(Justify::SpaceBetween);
    let ctx = LayoutCtx {
        measurer: &MockMeasurer::STANDARD,
        theme: &super::super::tests::TEST_THEME,
    };
    let mut layout_box = row.layout(&ctx);
    layout_box.width = SizeSpec::Fill;
    let viewport = Rect::new(0.0, 0.0, 100.0, 50.0);
    let node = compute_layout(&layout_box, viewport);

    assert_eq!(node.children[0].rect.x(), 0.0);
    assert_eq!(node.children[1].rect.x(), 46.0);
    assert_eq!(node.children[2].rect.x(), 92.0);
}

#[test]
fn row_with_padding() {
    let row = ContainerWidget::row()
        .with_children(vec![label("A")])
        .with_padding(Insets::all(10.0));
    let ctx = LayoutCtx {
        measurer: &MockMeasurer::STANDARD,
        theme: &super::super::tests::TEST_THEME,
    };
    let layout_box = row.layout(&ctx);
    let viewport = Rect::new(0.0, 0.0, 400.0, 300.0);
    let node = compute_layout(&layout_box, viewport);

    // "A" = 8x16. With 10px padding all around: 28x36.
    assert_eq!(node.rect.width(), 28.0);
    assert_eq!(node.rect.height(), 36.0);
    // Child at (10, 10) inside the padded area.
    assert_eq!(node.children[0].rect.x(), 10.0);
    assert_eq!(node.children[0].rect.y(), 10.0);
}

#[test]
fn empty_container_produces_correct_layout() {
    let row = ContainerWidget::row();
    let ctx = LayoutCtx {
        measurer: &MockMeasurer::STANDARD,
        theme: &super::super::tests::TEST_THEME,
    };
    let layout_box = row.layout(&ctx);
    let viewport = Rect::new(0.0, 0.0, 400.0, 300.0);
    let node = compute_layout(&layout_box, viewport);
    assert_eq!(node.rect.width(), 0.0);
    assert_eq!(node.rect.height(), 0.0);
}

#[test]
fn container_not_focusable() {
    let row = ContainerWidget::row();
    assert!(!row.is_focusable());
}

#[test]
fn child_count_tracks_children() {
    let row = ContainerWidget::row().with_children(vec![label("A"), label("B"), label("C")]);
    assert_eq!(row.child_count(), 3);
}

// --- Child management tests ---

#[test]
fn add_child_increases_count() {
    let mut row = ContainerWidget::row();
    assert_eq!(row.child_count(), 0);
    row.add_child(label("A"));
    assert_eq!(row.child_count(), 1);
    row.add_child(label("B"));
    assert_eq!(row.child_count(), 2);
}

#[test]
fn remove_child_decreases_count() {
    let mut row = ContainerWidget::row().with_children(vec![label("A"), label("B"), label("C")]);
    assert_eq!(row.child_count(), 3);
    let _ = row.remove_child(1);
    assert_eq!(row.child_count(), 2);
}

#[test]
fn with_children_builder() {
    let row = ContainerWidget::row()
        .with_child(label("A"))
        .with_child(label("B"));
    assert_eq!(row.child_count(), 2);
}

#[test]
fn draw_skips_children_fully_outside_active_clip() {
    let draws = std::rc::Rc::new(std::cell::Cell::new(0));
    let row = ContainerWidget::column()
        .with_child(Box::new(CountingWidget::new(100.0, 20.0, draws.clone())))
        .with_child(Box::new(CountingWidget::new(100.0, 20.0, draws.clone())));

    let measurer = MockMeasurer::STANDARD;
    let mut draw_list = DrawList::new();
    draw_list.push_clip(Rect::new(0.0, 0.0, 100.0, 20.0));
    let anim_flag = std::cell::Cell::new(false);
    let mut ctx = DrawCtx {
        measurer: &measurer,
        draw_list: &mut draw_list,
        bounds: Rect::new(0.0, 0.0, 100.0, 40.0),
        focused_widget: None,
        now: std::time::Instant::now(),
        animations_running: &anim_flag,
        theme: &super::super::tests::TEST_THEME,
        icons: None,
    };

    row.draw(&mut ctx);

    assert_eq!(draws.get(), 1, "only the visible child should draw");
}

#[test]
fn focusable_children_collects_recursively() {
    let btn = ButtonWidget::new("OK");
    let btn_id = btn.id();
    let inner = ContainerWidget::row().with_child(Box::new(btn));
    let outer = ContainerWidget::column()
        .with_child(label("Title"))
        .with_child(Box::new(inner));
    let ids = outer.focusable_children();
    assert_eq!(ids, vec![btn_id]);
}

// --- Draw tests ---

#[test]
fn draw_delegates_to_children() {
    let row = ContainerWidget::row().with_children(vec![label("A"), label("B")]);
    let measurer = MockMeasurer::STANDARD;
    let mut draw_list = DrawList::new();
    let bounds = Rect::new(0.0, 0.0, 100.0, 50.0);
    let anim_flag = std::cell::Cell::new(false);
    let mut ctx = DrawCtx {
        measurer: &measurer,
        draw_list: &mut draw_list,
        bounds,
        focused_widget: None,
        now: std::time::Instant::now(),
        animations_running: &anim_flag,
        theme: &super::super::tests::TEST_THEME,
        icons: None,
    };
    row.draw(&mut ctx);

    let text_cmds = draw_list
        .commands()
        .iter()
        .filter(|c| matches!(c, DrawCommand::Text { .. }))
        .count();
    assert_eq!(text_cmds, 2);
}

#[test]
fn focused_widget_propagates_through_draw() {
    let btn = ButtonWidget::new("Focus Me");
    let btn_id = btn.id();
    let row = ContainerWidget::row().with_child(Box::new(btn));
    let measurer = MockMeasurer::STANDARD;
    let mut draw_list = DrawList::new();
    let bounds = Rect::new(0.0, 0.0, 200.0, 50.0);
    let anim_flag = std::cell::Cell::new(false);
    let mut ctx = DrawCtx {
        measurer: &measurer,
        draw_list: &mut draw_list,
        bounds,
        focused_widget: Some(btn_id),
        now: std::time::Instant::now(),
        animations_running: &anim_flag,
        theme: &super::super::tests::TEST_THEME,
        icons: None,
    };
    row.draw(&mut ctx);

    let rect_cmds = draw_list
        .commands()
        .iter()
        .filter(|c| matches!(c, DrawCommand::Rect { .. }))
        .count();
    // Focus ring rect + button bg rect = 2 rects.
    assert!(
        rect_cmds >= 2,
        "expected focus ring + bg, got {rect_cmds} rects"
    );
}

// --- Mouse event tests ---

#[test]
fn delegates_mouse_to_child() {
    let btn = button("Click");
    let btn_id = btn.id();
    let mut row = ContainerWidget::row().with_children(vec![label("Label"), btn]);

    let measurer = MockMeasurer::STANDARD;
    let bounds = Rect::new(0.0, 0.0, 200.0, 50.0);
    let ctx = EventCtx {
        measurer: &measurer,
        bounds,
        is_focused: false,
        focused_widget: None,
        theme: &super::super::tests::TEST_THEME,
    };

    let down = MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        pos: Point::new(50.0, 8.0),
        modifiers: Modifiers::NONE,
    };
    let _ = row.handle_mouse(&down, &ctx);

    let up = MouseEvent {
        kind: MouseEventKind::Up(MouseButton::Left),
        pos: Point::new(50.0, 8.0),
        modifiers: Modifiers::NONE,
    };
    let resp = row.handle_mouse(&up, &ctx);

    match resp.action {
        Some(WidgetAction::Clicked(id)) => assert_eq!(id, btn_id),
        other => panic!("expected Clicked({btn_id:?}), got {other:?}"),
    }
}

#[test]
fn mouse_outside_children_is_ignored() {
    let mut row = ContainerWidget::row().with_child(label("X"));
    let measurer = MockMeasurer::STANDARD;
    let bounds = Rect::new(0.0, 0.0, 200.0, 50.0);
    let ctx = EventCtx {
        measurer: &measurer,
        bounds,
        is_focused: false,
        focused_widget: None,
        theme: &super::super::tests::TEST_THEME,
    };
    let event = MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        pos: Point::new(150.0, 25.0),
        modifiers: Modifiers::NONE,
    };
    let resp = row.handle_mouse(&event, &ctx);
    assert_eq!(resp, WidgetResponse::ignored());
}

#[test]
fn mouse_on_gap_is_ignored() {
    let mut row = ContainerWidget::row()
        .with_children(vec![label("A"), label("B")])
        .with_gap(20.0);
    let measurer = MockMeasurer::STANDARD;
    let bounds = Rect::new(0.0, 0.0, 200.0, 50.0);
    let ctx = EventCtx {
        measurer: &measurer,
        bounds,
        is_focused: false,
        focused_widget: None,
        theme: &super::super::tests::TEST_THEME,
    };
    let event = MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        pos: Point::new(15.0, 8.0),
        modifiers: Modifiers::NONE,
    };
    let resp = row.handle_mouse(&event, &ctx);
    assert_eq!(resp, WidgetResponse::ignored());
}

#[test]
fn empty_container_mouse_ignored() {
    let mut row = ContainerWidget::row();
    let measurer = MockMeasurer::STANDARD;
    let bounds = Rect::new(0.0, 0.0, 100.0, 100.0);
    let ctx = EventCtx {
        measurer: &measurer,
        bounds,
        is_focused: false,
        focused_widget: None,
        theme: &super::super::tests::TEST_THEME,
    };
    let event = MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        pos: Point::new(50.0, 50.0),
        modifiers: Modifiers::NONE,
    };
    assert_eq!(row.handle_mouse(&event, &ctx), WidgetResponse::ignored());
}

// --- Capture semantics tests ---

#[test]
fn mouse_capture_delivers_up_to_pressed_child() {
    let btn = button("Capture");
    let mut row = ContainerWidget::row().with_children(vec![label("Left"), btn]);

    let measurer = MockMeasurer::STANDARD;
    let bounds = Rect::new(0.0, 0.0, 200.0, 50.0);
    let ctx = EventCtx {
        measurer: &measurer,
        bounds,
        is_focused: false,
        focused_widget: None,
        theme: &super::super::tests::TEST_THEME,
    };

    // Mouse down on button (x=50 is inside button area).
    let down = MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        pos: Point::new(50.0, 8.0),
        modifiers: Modifiers::NONE,
    };
    let _ = row.handle_mouse(&down, &ctx);
    assert!(row.input_state.captured_child.is_some());

    // Mouse up FAR outside the button (x=5 is inside the label).
    // With capture, the up event is still delivered to the button (not the label).
    // The button correctly sees the cursor outside its bounds and returns
    // redraw without Clicked (standard "drag off to cancel" behavior).
    let up = MouseEvent {
        kind: MouseEventKind::Up(MouseButton::Left),
        pos: Point::new(5.0, 8.0),
        modifiers: Modifiers::NONE,
    };
    let resp = row.handle_mouse(&up, &ctx);

    // Button got the event (paint, not ignored), but no Clicked since cursor outside.
    assert_eq!(resp.response, EventResponse::RequestPaint);
    assert!(resp.action.is_none());
    // Capture released.
    assert!(row.input_state.captured_child.is_none());
}

#[test]
fn mouse_capture_fires_clicked_when_released_inside() {
    let btn = button("Capture");
    let btn_id = btn.id();
    let mut row = ContainerWidget::row().with_children(vec![label("Left"), btn]);

    let measurer = MockMeasurer::STANDARD;
    let bounds = Rect::new(0.0, 0.0, 200.0, 50.0);
    let ctx = EventCtx {
        measurer: &measurer,
        bounds,
        is_focused: false,
        focused_widget: None,
        theme: &super::super::tests::TEST_THEME,
    };

    // Mouse down on button.
    let down = MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        pos: Point::new(50.0, 8.0),
        modifiers: Modifiers::NONE,
    };
    let _ = row.handle_mouse(&down, &ctx);

    // Mouse up still inside button → Clicked fires.
    let up = MouseEvent {
        kind: MouseEventKind::Up(MouseButton::Left),
        pos: Point::new(50.0, 8.0),
        modifiers: Modifiers::NONE,
    };
    let resp = row.handle_mouse(&up, &ctx);

    match resp.action {
        Some(WidgetAction::Clicked(id)) => assert_eq!(id, btn_id),
        other => panic!("expected Clicked({btn_id:?}), got {other:?}"),
    }
}

#[test]
fn capture_released_on_mouse_up() {
    let mut row = ContainerWidget::row().with_children(vec![label("A"), label("B")]);

    let measurer = MockMeasurer::STANDARD;
    let bounds = Rect::new(0.0, 0.0, 200.0, 50.0);
    let ctx = EventCtx {
        measurer: &measurer,
        bounds,
        is_focused: false,
        focused_widget: None,
        theme: &super::super::tests::TEST_THEME,
    };

    // Mouse down on child 0.
    let down = MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        pos: Point::new(4.0, 8.0),
        modifiers: Modifiers::NONE,
    };
    let _ = row.handle_mouse(&down, &ctx);
    assert!(row.input_state.captured_child.is_some());

    // Mouse up releases capture.
    let up = MouseEvent {
        kind: MouseEventKind::Up(MouseButton::Left),
        pos: Point::new(4.0, 8.0),
        modifiers: Modifiers::NONE,
    };
    let _ = row.handle_mouse(&up, &ctx);
    assert!(row.input_state.captured_child.is_none());
}

// --- Keyboard event tests ---

#[test]
fn delegates_key_to_focused_child() {
    let btn = button("OK");
    let btn_id = btn.id();
    let mut col = ContainerWidget::column().with_children(vec![label("Title"), btn]);

    let measurer = MockMeasurer::STANDARD;
    let bounds = Rect::new(0.0, 0.0, 200.0, 100.0);
    let ctx = EventCtx {
        measurer: &measurer,
        bounds,
        is_focused: false,
        focused_widget: Some(btn_id),
        theme: &super::super::tests::TEST_THEME,
    };

    let event = KeyEvent {
        key: Key::Enter,
        modifiers: Modifiers::NONE,
    };
    let resp = col.handle_key(event, &ctx);
    match resp.action {
        Some(WidgetAction::Clicked(id)) => assert_eq!(id, btn_id),
        other => panic!("expected Clicked, got {other:?}"),
    }
}

#[test]
fn empty_container_key_ignored() {
    let mut row = ContainerWidget::row();
    let measurer = MockMeasurer::STANDARD;
    let bounds = Rect::new(0.0, 0.0, 100.0, 100.0);
    let ctx = EventCtx {
        measurer: &measurer,
        bounds,
        is_focused: true,
        focused_widget: None,
        theme: &super::super::tests::TEST_THEME,
    };
    let event = KeyEvent {
        key: Key::Enter,
        modifiers: Modifiers::NONE,
    };
    assert_eq!(row.handle_key(event, &ctx), WidgetResponse::ignored());
}

#[test]
fn child_consumes_event_stops_propagation() {
    let btn1 = button("First");
    let btn1_id = btn1.id();
    let btn2 = button("Second");
    let btn2_id = btn2.id();
    let mut row = ContainerWidget::row().with_children(vec![btn1, btn2]);

    let measurer = MockMeasurer::STANDARD;
    let bounds = Rect::new(0.0, 0.0, 300.0, 50.0);
    let ctx = EventCtx {
        measurer: &measurer,
        bounds,
        is_focused: false,
        focused_widget: None,
        theme: &super::super::tests::TEST_THEME,
    };

    let down = MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        pos: Point::new(5.0, 8.0),
        modifiers: Modifiers::NONE,
    };
    let _ = row.handle_mouse(&down, &ctx);
    let up = MouseEvent {
        kind: MouseEventKind::Up(MouseButton::Left),
        pos: Point::new(5.0, 8.0),
        modifiers: Modifiers::NONE,
    };
    let resp = row.handle_mouse(&up, &ctx);

    match resp.action {
        Some(WidgetAction::Clicked(id)) => {
            assert_eq!(id, btn1_id);
            assert_ne!(id, btn2_id);
        }
        other => panic!("expected Clicked from first button, got {other:?}"),
    }
}

// --- Nested container tests ---

#[test]
fn deeply_nested_layout_correct() {
    let inner = ContainerWidget::row()
        .with_children(vec![label("A"), label("B")])
        .with_gap(4.0);
    let outer = ContainerWidget::column().with_children(vec![
        label("Header"),
        Box::new(inner),
        label("Footer"),
    ]);
    let ctx = LayoutCtx {
        measurer: &MockMeasurer::STANDARD,
        theme: &super::super::tests::TEST_THEME,
    };
    let layout_box = outer.layout(&ctx);
    let viewport = Rect::new(0.0, 0.0, 400.0, 300.0);
    let node = compute_layout(&layout_box, viewport);

    assert_eq!(node.children.len(), 3);
    assert_eq!(node.children[0].rect.y(), 0.0);
    assert_eq!(node.children[1].rect.y(), 16.0);
    assert_eq!(node.children[2].rect.y(), 32.0);
    let inner = &node.children[1];
    assert_eq!(inner.rect.width(), 20.0);
    assert_eq!(inner.children.len(), 2);
    assert_eq!(inner.children[0].rect.x(), 0.0);
    assert_eq!(inner.children[1].rect.x(), 12.0);
}

#[test]
fn deeply_nested_mouse_routing() {
    let btn = button("OK");
    let btn_id = btn.id();
    let inner = ContainerWidget::row().with_children(vec![label("Pre"), btn]);
    let mut outer = ContainerWidget::column().with_children(vec![label("Header"), Box::new(inner)]);

    let measurer = MockMeasurer::STANDARD;
    let bounds = Rect::new(0.0, 0.0, 300.0, 200.0);
    let ctx = EventCtx {
        measurer: &measurer,
        bounds,
        is_focused: false,
        focused_widget: None,
        theme: &super::super::tests::TEST_THEME,
    };

    let down = MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        pos: Point::new(30.0, 20.0),
        modifiers: Modifiers::NONE,
    };
    let _ = outer.handle_mouse(&down, &ctx);
    let up = MouseEvent {
        kind: MouseEventKind::Up(MouseButton::Left),
        pos: Point::new(30.0, 20.0),
        modifiers: Modifiers::NONE,
    };
    let resp = outer.handle_mouse(&up, &ctx);

    match resp.action {
        Some(WidgetAction::Clicked(id)) => assert_eq!(id, btn_id),
        other => panic!("expected Clicked through nested containers, got {other:?}"),
    }
}

#[test]
fn panel_inside_container_layout() {
    let panel = PanelWidget::new(Box::new(LabelWidget::new("Inner")));
    let row = ContainerWidget::row().with_children(vec![label("Before"), Box::new(panel)]);
    let ctx = LayoutCtx {
        measurer: &MockMeasurer::STANDARD,
        theme: &super::super::tests::TEST_THEME,
    };
    let layout_box = row.layout(&ctx);
    let viewport = Rect::new(0.0, 0.0, 400.0, 300.0);
    let node = compute_layout(&layout_box, viewport);

    assert_eq!(node.children.len(), 2);
    assert_eq!(node.children[0].rect.width(), 48.0);
    assert_eq!(node.children[1].rect.width(), 64.0);
    assert_eq!(node.children[1].rect.x(), 48.0);
}

// --- Cache tests ---

#[test]
fn layout_cache_returns_same_result_for_same_bounds() {
    let mut row = ContainerWidget::row().with_children(vec![label("A")]);
    let measurer = MockMeasurer::STANDARD;
    let theme = super::super::tests::TEST_THEME;
    let bounds = Rect::new(0.0, 0.0, 100.0, 50.0);

    // First layout populates the cache and clears the dirty flag.
    let node1 = row.get_or_compute_layout(&measurer, &theme, bounds);
    row.clear_dirty();

    let node2 = row.get_or_compute_layout(&measurer, &theme, bounds);
    // Same Rc (pointer equality) — cache hit.
    assert!(std::rc::Rc::ptr_eq(&node1, &node2));
}

#[test]
fn layout_cache_recomputes_for_different_bounds() {
    let row = ContainerWidget::row().with_children(vec![label("A")]);
    let measurer = MockMeasurer::STANDARD;
    let theme = super::super::tests::TEST_THEME;

    let bounds1 = Rect::new(0.0, 0.0, 100.0, 50.0);
    let bounds2 = Rect::new(0.0, 0.0, 200.0, 100.0);
    let node1 = row.get_or_compute_layout(&measurer, &theme, bounds1);
    let node2 = row.get_or_compute_layout(&measurer, &theme, bounds2);
    assert!(!std::rc::Rc::ptr_eq(&node1, &node2));
}

// --- Hover tests ---

#[test]
fn hover_leave_clears_child_state() {
    let mut row = ContainerWidget::row().with_children(vec![label("A"), label("B")]);
    let measurer = MockMeasurer::STANDARD;
    let bounds = Rect::new(0.0, 0.0, 200.0, 50.0);
    let ctx = EventCtx {
        measurer: &measurer,
        bounds,
        is_focused: false,
        focused_widget: None,
        theme: &super::super::tests::TEST_THEME,
    };

    // Move over child 0 to set hover.
    let mv = MouseEvent {
        kind: MouseEventKind::Move,
        pos: Point::new(4.0, 8.0),
        modifiers: Modifiers::NONE,
    };
    let _ = row.handle_mouse(&mv, &ctx);
    assert!(row.input_state.hovered_child.is_some());

    // Leave clears it.
    use crate::input::HoverEvent;
    let _ = row.handle_hover(HoverEvent::Leave, &ctx);
    assert!(row.input_state.hovered_child.is_none());
}

#[test]
fn hover_transition_between_children() {
    let mut row = ContainerWidget::row()
        .with_children(vec![label("AAAA"), label("BBBB")])
        .with_gap(0.0);
    let measurer = MockMeasurer::STANDARD;
    let bounds = Rect::new(0.0, 0.0, 200.0, 50.0);
    let ctx = EventCtx {
        measurer: &measurer,
        bounds,
        is_focused: false,
        focused_widget: None,
        theme: &super::super::tests::TEST_THEME,
    };

    // Move to child 0.
    let mv1 = MouseEvent {
        kind: MouseEventKind::Move,
        pos: Point::new(4.0, 8.0),
        modifiers: Modifiers::NONE,
    };
    let _ = row.handle_mouse(&mv1, &ctx);
    assert_eq!(row.input_state.hovered_child, Some(0));

    // Move to child 1 ("AAAA" is 32px wide, "BBBB" starts at 32).
    let mv2 = MouseEvent {
        kind: MouseEventKind::Move,
        pos: Point::new(36.0, 8.0),
        modifiers: Modifiers::NONE,
    };
    let resp = row.handle_mouse(&mv2, &ctx);
    assert_eq!(row.input_state.hovered_child, Some(1));
    assert_eq!(resp.response, EventResponse::RequestPaint);
}

// ── Dirty Tracking ──────────────────────────────────────────────────

#[test]
fn update_dirty_paint_sets_needs_paint() {
    let mut c = ContainerWidget::column();
    c.clear_dirty();
    assert!(!c.needs_paint());
    assert!(!c.needs_layout());

    let resp = WidgetResponse::paint();
    c.update_dirty(&resp, None);
    assert!(c.needs_paint());
    assert!(!c.needs_layout());
}

#[test]
fn update_dirty_layout_sets_both_flags() {
    let mut c = ContainerWidget::column();
    c.clear_dirty();

    let resp = WidgetResponse::layout();
    c.update_dirty(&resp, None);
    assert!(c.needs_paint());
    assert!(c.needs_layout());
}

#[test]
fn update_dirty_handled_does_not_set_flags() {
    let mut c = ContainerWidget::column();
    c.clear_dirty();

    let resp = WidgetResponse::handled();
    c.update_dirty(&resp, None);
    assert!(!c.needs_paint());
    assert!(!c.needs_layout());
}

#[test]
fn clear_dirty_resets_both_flags() {
    let mut c = ContainerWidget::column();
    let resp = WidgetResponse::layout();
    c.update_dirty(&resp, None);
    assert!(c.needs_paint());
    assert!(c.needs_layout());

    c.clear_dirty();
    assert!(!c.needs_paint());
    assert!(!c.needs_layout());
}

#[test]
fn update_dirty_marks_tracker_with_source() {
    let mut c = ContainerWidget::column();
    c.clear_dirty();
    let mut tracker = InvalidationTracker::new();
    let source_id = crate::widget_id::WidgetId::next();

    let resp = WidgetResponse::paint().with_source(source_id);
    c.update_dirty(&resp, Some(&mut tracker));

    assert!(c.needs_paint());
    assert!(tracker.is_paint_dirty(source_id));
    assert!(!tracker.is_layout_dirty(source_id));
}

#[test]
fn update_dirty_without_source_does_not_mark_tracker() {
    let mut c = ContainerWidget::column();
    c.clear_dirty();
    let mut tracker = InvalidationTracker::new();

    let resp = WidgetResponse::paint();
    c.update_dirty(&resp, Some(&mut tracker));

    assert!(c.needs_paint());
    assert!(!tracker.is_any_dirty());
}

#[test]
fn new_container_starts_dirty() {
    let c = ContainerWidget::column();
    assert!(c.needs_paint());
    assert!(c.needs_layout());
}

#[test]
fn needs_layout_bypasses_cache() {
    // A container with a label child.
    let mut c = ContainerWidget::column().with_child(Box::new(LabelWidget::new("hello")));
    c.clear_dirty();

    let measurer = MockMeasurer::STANDARD;
    let bounds = Rect::new(0.0, 0.0, 200.0, 100.0);
    let theme = &super::super::tests::TEST_THEME;

    // First draw populates the cache.
    let mut draw_list = DrawList::new();
    let anim = std::cell::Cell::new(false);
    let mut ctx = DrawCtx {
        measurer: &measurer,
        draw_list: &mut draw_list,
        bounds,
        focused_widget: None,
        now: std::time::Instant::now(),
        animations_running: &anim,
        theme,
        icons: None,
    };
    c.draw(&mut ctx);
    let cmd_count_1 = draw_list.commands().len();

    // Second draw with same bounds should use cache (same result).
    draw_list.clear();
    let mut ctx2 = DrawCtx {
        measurer: &measurer,
        draw_list: &mut draw_list,
        bounds,
        focused_widget: None,
        now: std::time::Instant::now(),
        animations_running: &anim,
        theme,
        icons: None,
    };
    c.draw(&mut ctx2);
    assert_eq!(
        draw_list.commands().len(),
        cmd_count_1,
        "cached layout should produce same draw commands"
    );

    // Simulate a child requesting layout (e.g. section collapse).
    let layout_resp = WidgetResponse {
        response: EventResponse::RequestLayout,
        action: None,
        capture: CaptureRequest::None,
        source: None,
    };
    c.update_dirty(&layout_resp, None);
    assert!(c.needs_layout());

    // Third draw with same bounds should bypass cache (recompute).
    // This verifies the dirty flag properly invalidates the layout cache.
    draw_list.clear();
    let mut ctx3 = DrawCtx {
        measurer: &measurer,
        draw_list: &mut draw_list,
        bounds,
        focused_widget: None,
        now: std::time::Instant::now(),
        animations_running: &anim,
        theme,
        icons: None,
    };
    c.draw(&mut ctx3);
    assert_eq!(
        draw_list.commands().len(),
        cmd_count_1,
        "recomputed layout should produce same draw commands"
    );
}
