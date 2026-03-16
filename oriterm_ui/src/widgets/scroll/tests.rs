use crate::draw::{DrawCommand, DrawList};
use crate::geometry::{Point, Rect};
use crate::input::{
    Key, KeyEvent, Modifiers, MouseButton, MouseEvent, MouseEventKind, ScrollDelta,
};
use crate::layout::compute_layout;
use crate::widgets::button::ButtonWidget;
use crate::widgets::container::ContainerWidget;
use crate::widgets::label::LabelWidget;
use crate::widgets::tests::MockMeasurer;
use crate::widgets::{DrawCtx, EventCtx, LayoutCtx, Widget};

use super::ScrollWidget;

/// Creates content that is 16px tall (single label).
fn short_content() -> Box<dyn Widget> {
    Box::new(LabelWidget::new("A".repeat(100)))
}

/// Creates a tall column of labels that overflows a small viewport.
/// 20 labels * 16px = 320px tall.
fn tall_content() -> Box<dyn Widget> {
    let labels: Vec<Box<dyn Widget>> = (0..20)
        .map(|i| Box::new(LabelWidget::new(format!("Line {i}"))) as Box<dyn Widget>)
        .collect();
    Box::new(ContainerWidget::column().with_children(labels))
}

fn make_scroll(child: Box<dyn Widget>) -> ScrollWidget {
    ScrollWidget::vertical(child)
}

#[test]
fn scroll_layout_fills_width_for_vertical() {
    let scroll = make_scroll(short_content());
    let ctx = LayoutCtx {
        measurer: &MockMeasurer::STANDARD,
        theme: &super::super::tests::TEST_THEME,
    };
    let layout_box = scroll.layout(&ctx);
    let viewport = Rect::new(0.0, 0.0, 1000.0, 1000.0);
    let node = compute_layout(&layout_box, viewport);

    // Vertical scroll uses Fill width — takes viewport width, not content width.
    assert_eq!(node.rect.width(), 1000.0);
    // Height is the child's natural height (100-char label = 16px tall).
    assert_eq!(node.rect.height(), 16.0);
}

#[test]
fn scroll_offset_starts_at_zero() {
    let scroll = make_scroll(tall_content());
    assert_eq!(scroll.scroll_offset(), 0.0);
}

#[test]
fn scroll_offset_clamps_to_range() {
    let mut scroll = make_scroll(tall_content());
    // Content 500px tall, viewport 200px → max offset = 300.
    scroll.set_scroll_offset(999.0, 500.0, 200.0);
    assert_eq!(scroll.scroll_offset(), 300.0);

    scroll.set_scroll_offset(-10.0, 500.0, 200.0);
    assert_eq!(scroll.scroll_offset(), 0.0);
}

#[test]
fn scroll_offset_zero_when_content_fits() {
    let mut scroll = make_scroll(tall_content());
    // Content 100px, viewport 200px → max offset = 0.
    scroll.set_scroll_offset(50.0, 100.0, 200.0);
    assert_eq!(scroll.scroll_offset(), 0.0);
}

#[test]
fn scroll_is_focusable() {
    let scroll = make_scroll(tall_content());
    assert!(scroll.is_focusable());
}

#[test]
fn scroll_draws_with_clip() {
    let scroll = make_scroll(tall_content());
    let measurer = MockMeasurer::STANDARD;
    let mut draw_list = DrawList::new();
    let bounds = Rect::new(0.0, 0.0, 200.0, 100.0);
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
        scene_cache: None,
        interaction: None,
        widget_id: None,
        frame_requests: None,
    };
    scroll.paint(&mut ctx);

    // Should have PushClip and PopClip commands (balanced).
    let push_count = draw_list
        .commands()
        .iter()
        .filter(|c| matches!(c, DrawCommand::PushClip { .. }))
        .count();
    let pop_count = draw_list
        .commands()
        .iter()
        .filter(|c| matches!(c, DrawCommand::PopClip))
        .count();
    assert_eq!(push_count, 1);
    assert_eq!(pop_count, 1);
}

#[test]
fn scroll_wheel_changes_offset() {
    // tall_content = 20 labels * 16px = 320px tall.
    let mut scroll = ScrollWidget::vertical(tall_content());

    let measurer = MockMeasurer::STANDARD;
    // Viewport 100px tall — content (320px) overflows by 220px.
    let bounds = Rect::new(0.0, 0.0, 200.0, 100.0);
    let ctx = EventCtx {
        measurer: &measurer,
        bounds,
        is_focused: true,
        focused_widget: None,
        theme: &super::super::tests::TEST_THEME,
        interaction: None,
        widget_id: None,
        frame_requests: None,
    };

    // Scroll down (negative delta_y means scroll down in our convention).
    let event = MouseEvent {
        kind: MouseEventKind::Scroll(ScrollDelta::Lines { x: 0.0, y: -3.0 }),
        pos: Point::new(25.0, 25.0),
        modifiers: Modifiers::NONE,
    };
    let resp = scroll.handle_mouse(&event, &ctx);

    // Should have scrolled (redraw).
    assert!(resp.response.is_handled());
    // Offset should have increased (scrolled down).
    assert!(scroll.scroll_offset() > 0.0);
}

#[test]
fn key_home_resets_to_top() {
    // tall_content = 320px tall.
    let mut scroll = ScrollWidget::vertical(tall_content());
    // Manually set offset.
    scroll.set_scroll_offset(100.0, 320.0, 100.0);
    assert!(scroll.scroll_offset() > 0.0);

    let measurer = MockMeasurer::STANDARD;
    let bounds = Rect::new(0.0, 0.0, 200.0, 100.0);
    let ctx = EventCtx {
        measurer: &measurer,
        bounds,
        is_focused: true,
        focused_widget: None,
        theme: &super::super::tests::TEST_THEME,
        interaction: None,
        widget_id: None,
        frame_requests: None,
    };

    let event = KeyEvent {
        key: Key::Home,
        modifiers: Modifiers::NONE,
    };
    let resp = scroll.handle_key(event, &ctx);
    assert!(resp.response.is_handled());
    assert_eq!(scroll.scroll_offset(), 0.0);
}

#[test]
fn key_end_scrolls_to_bottom() {
    // tall_content = 320px tall, viewport 100px → max offset 220.
    let mut scroll = ScrollWidget::vertical(tall_content());

    let measurer = MockMeasurer::STANDARD;
    let bounds = Rect::new(0.0, 0.0, 200.0, 100.0);
    let ctx = EventCtx {
        measurer: &measurer,
        bounds,
        is_focused: true,
        focused_widget: None,
        theme: &super::super::tests::TEST_THEME,
        interaction: None,
        widget_id: None,
        frame_requests: None,
    };

    let event = KeyEvent {
        key: Key::End,
        modifiers: Modifiers::NONE,
    };
    let resp = scroll.handle_key(event, &ctx);
    assert!(resp.response.is_handled());
    // Content 320px, view 100px → max offset = 220.
    assert_eq!(scroll.scroll_offset(), 220.0);
}

#[test]
fn key_arrow_down_scrolls() {
    // tall_content = 320px tall, viewport 100px.
    let mut scroll = ScrollWidget::vertical(tall_content());

    let measurer = MockMeasurer::STANDARD;
    let bounds = Rect::new(0.0, 0.0, 200.0, 100.0);
    let ctx = EventCtx {
        measurer: &measurer,
        bounds,
        is_focused: true,
        focused_widget: None,
        theme: &super::super::tests::TEST_THEME,
        interaction: None,
        widget_id: None,
        frame_requests: None,
    };

    let event = KeyEvent {
        key: Key::ArrowDown,
        modifiers: Modifiers::NONE,
    };
    let resp = scroll.handle_key(event, &ctx);
    assert!(resp.response.is_handled());
    // Should have scrolled down by line_height (20px).
    assert_eq!(scroll.scroll_offset(), 20.0);
}

#[test]
fn key_page_down_scrolls_by_viewport() {
    // tall_content = 320px tall, viewport 100px.
    let mut scroll = ScrollWidget::vertical(tall_content());

    let measurer = MockMeasurer::STANDARD;
    let bounds = Rect::new(0.0, 0.0, 200.0, 100.0);
    let ctx = EventCtx {
        measurer: &measurer,
        bounds,
        is_focused: true,
        focused_widget: None,
        theme: &super::super::tests::TEST_THEME,
        interaction: None,
        widget_id: None,
        frame_requests: None,
    };

    let event = KeyEvent {
        key: Key::PageDown,
        modifiers: Modifiers::NONE,
    };
    let resp = scroll.handle_key(event, &ctx);
    assert!(resp.response.is_handled());
    // Should scroll down by one viewport height (100px).
    assert_eq!(scroll.scroll_offset(), 100.0);
}

#[test]
fn key_page_up_scrolls_by_viewport() {
    // tall_content = 320px tall, viewport 100px.
    let mut scroll = ScrollWidget::vertical(tall_content());
    scroll.set_scroll_offset(200.0, 320.0, 100.0);

    let measurer = MockMeasurer::STANDARD;
    let bounds = Rect::new(0.0, 0.0, 200.0, 100.0);
    let ctx = EventCtx {
        measurer: &measurer,
        bounds,
        is_focused: true,
        focused_widget: None,
        theme: &super::super::tests::TEST_THEME,
        interaction: None,
        widget_id: None,
        frame_requests: None,
    };

    let event = KeyEvent {
        key: Key::PageUp,
        modifiers: Modifiers::NONE,
    };
    let resp = scroll.handle_key(event, &ctx);
    assert!(resp.response.is_handled());
    // Should scroll up by one viewport height (100px): 200 - 100 = 100.
    assert_eq!(scroll.scroll_offset(), 100.0);
}

#[test]
fn key_page_down_clamps_at_bottom() {
    // tall_content = 320px, viewport 100px → max offset 220.
    let mut scroll = ScrollWidget::vertical(tall_content());
    scroll.set_scroll_offset(200.0, 320.0, 100.0);

    let measurer = MockMeasurer::STANDARD;
    let bounds = Rect::new(0.0, 0.0, 200.0, 100.0);
    let ctx = EventCtx {
        measurer: &measurer,
        bounds,
        is_focused: true,
        focused_widget: None,
        theme: &super::super::tests::TEST_THEME,
        interaction: None,
        widget_id: None,
        frame_requests: None,
    };

    let event = KeyEvent {
        key: Key::PageDown,
        modifiers: Modifiers::NONE,
    };
    scroll.handle_key(event, &ctx);
    // 200 + 100 = 300, clamped to max 220.
    assert_eq!(scroll.scroll_offset(), 220.0);
}

#[test]
fn key_page_up_clamps_at_top() {
    let mut scroll = ScrollWidget::vertical(tall_content());
    scroll.set_scroll_offset(30.0, 320.0, 100.0);

    let measurer = MockMeasurer::STANDARD;
    let bounds = Rect::new(0.0, 0.0, 200.0, 100.0);
    let ctx = EventCtx {
        measurer: &measurer,
        bounds,
        is_focused: true,
        focused_widget: None,
        theme: &super::super::tests::TEST_THEME,
        interaction: None,
        widget_id: None,
        frame_requests: None,
    };

    let event = KeyEvent {
        key: Key::PageUp,
        modifiers: Modifiers::NONE,
    };
    scroll.handle_key(event, &ctx);
    // 30 - 100 = -70, clamped to 0.
    assert_eq!(scroll.scroll_offset(), 0.0);
}

// Edge cases from Chromium/Ratatui audit

#[test]
fn scroll_clip_rect_matches_viewport() {
    let scroll = make_scroll(tall_content());
    let measurer = MockMeasurer::STANDARD;
    let mut draw_list = DrawList::new();
    let bounds = Rect::new(10.0, 20.0, 150.0, 80.0);
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
        scene_cache: None,
        interaction: None,
        widget_id: None,
        frame_requests: None,
    };
    scroll.paint(&mut ctx);

    // The PushClip should use the scroll widget's bounds exactly.
    let clip = draw_list.commands().iter().find_map(|c| match c {
        DrawCommand::PushClip { rect } => Some(*rect),
        _ => None,
    });
    assert_eq!(clip, Some(bounds), "clip rect must match scroll viewport");
}

#[test]
fn scroll_child_drawn_offset_by_scroll() {
    let mut scroll = make_scroll(tall_content());
    scroll.set_scroll_offset(40.0, 320.0, 100.0);

    let measurer = MockMeasurer::STANDARD;
    let mut draw_list = DrawList::new();
    let bounds = Rect::new(0.0, 0.0, 200.0, 100.0);
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
        scene_cache: None,
        interaction: None,
        widget_id: None,
        frame_requests: None,
    };
    scroll.paint(&mut ctx);

    // The scroll widget clips to bounds (0,0,200,100) then applies a
    // PushTranslate(0, -40) to offset the child content. Children draw at
    // their natural (unscrolled) positions — the GPU converter applies the
    // translate at render time. Content-space visibility culling skips
    // labels above the scrolled viewport, so the first drawn label is the
    // one whose layout rect intersects the visible area [40, 140].
    let translate = draw_list.commands().iter().find_map(|c| match c {
        DrawCommand::PushTranslate { dx, dy } => Some((*dx, *dy)),
        _ => None,
    });
    assert_eq!(
        translate,
        Some((0.0, -40.0)),
        "PushTranslate should offset by scroll amount"
    );

    let first_text = draw_list.commands().iter().find_map(|c| match c {
        DrawCommand::Text { position, .. } => Some(*position),
        _ => None,
    });
    assert!(first_text.is_some(), "should have text commands");
    let pos = first_text.unwrap();
    // Label 2 at y=32 (16px per label) is the first whose rect [32,48]
    // intersects the scrolled viewport [40,140].
    assert_eq!(
        pos.y, 32.0,
        "first text should be the first label intersecting the scrolled viewport"
    );
}

#[test]
fn scroll_draws_scrollbar_when_overflowing() {
    let scroll = make_scroll(tall_content());
    let measurer = MockMeasurer::STANDARD;
    let mut draw_list = DrawList::new();
    // Viewport 100px < content 320px → scrollbar should appear.
    let bounds = Rect::new(0.0, 0.0, 200.0, 100.0);
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
        scene_cache: None,
        interaction: None,
        widget_id: None,
        frame_requests: None,
    };
    scroll.paint(&mut ctx);

    // Should have a Rect command after PopClip (the scrollbar thumb).
    let after_pop = draw_list
        .commands()
        .iter()
        .skip_while(|c| !matches!(c, DrawCommand::PopClip))
        .filter(|c| matches!(c, DrawCommand::Rect { .. }))
        .count();
    assert!(
        after_pop >= 1,
        "scrollbar thumb rect should be drawn after clip"
    );
}

#[test]
fn scroll_no_scrollbar_when_content_fits() {
    let scroll = make_scroll(short_content());
    let measurer = MockMeasurer::STANDARD;
    let mut draw_list = DrawList::new();
    // Viewport 100px > content 16px → no scrollbar.
    let bounds = Rect::new(0.0, 0.0, 1000.0, 100.0);
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
        scene_cache: None,
        interaction: None,
        widget_id: None,
        frame_requests: None,
    };
    scroll.paint(&mut ctx);

    // No Rect commands after PopClip (no scrollbar).
    let after_pop = draw_list
        .commands()
        .iter()
        .skip_while(|c| !matches!(c, DrawCommand::PopClip))
        .filter(|c| matches!(c, DrawCommand::Rect { .. }))
        .count();
    assert_eq!(after_pop, 0, "no scrollbar when content fits");
}

#[test]
fn scroll_multiple_wheel_events_accumulate() {
    let mut scroll = ScrollWidget::vertical(tall_content());
    let measurer = MockMeasurer::STANDARD;
    let bounds = Rect::new(0.0, 0.0, 200.0, 100.0);
    let ctx = EventCtx {
        measurer: &measurer,
        bounds,
        is_focused: true,
        focused_widget: None,
        theme: &super::super::tests::TEST_THEME,
        interaction: None,
        widget_id: None,
        frame_requests: None,
    };

    // Scroll down 3 times.
    for _ in 0..3 {
        let event = MouseEvent {
            kind: MouseEventKind::Scroll(ScrollDelta::Lines { x: 0.0, y: -1.0 }),
            pos: Point::new(25.0, 25.0),
            modifiers: Modifiers::NONE,
        };
        scroll.handle_mouse(&event, &ctx);
    }

    // 3 lines * 20px line_height = 60px offset.
    assert_eq!(scroll.scroll_offset(), 60.0);
}

#[test]
fn scroll_wheel_clamps_at_bottom() {
    let mut scroll = ScrollWidget::vertical(tall_content());
    let measurer = MockMeasurer::STANDARD;
    let bounds = Rect::new(0.0, 0.0, 200.0, 100.0);
    let ctx = EventCtx {
        measurer: &measurer,
        bounds,
        is_focused: true,
        focused_widget: None,
        theme: &super::super::tests::TEST_THEME,
        interaction: None,
        widget_id: None,
        frame_requests: None,
    };

    // Scroll way past the bottom.
    let event = MouseEvent {
        kind: MouseEventKind::Scroll(ScrollDelta::Lines { x: 0.0, y: -999.0 }),
        pos: Point::new(25.0, 25.0),
        modifiers: Modifiers::NONE,
    };
    scroll.handle_mouse(&event, &ctx);

    // Content 320px, viewport 100px → max offset 220.
    assert_eq!(scroll.scroll_offset(), 220.0);
}

#[test]
fn scroll_wheel_clamps_at_top() {
    let mut scroll = ScrollWidget::vertical(tall_content());
    let measurer = MockMeasurer::STANDARD;
    let bounds = Rect::new(0.0, 0.0, 200.0, 100.0);
    let ctx = EventCtx {
        measurer: &measurer,
        bounds,
        is_focused: true,
        focused_widget: None,
        theme: &super::super::tests::TEST_THEME,
        interaction: None,
        widget_id: None,
        frame_requests: None,
    };

    // Scroll up from top (should stay at 0).
    let event = MouseEvent {
        kind: MouseEventKind::Scroll(ScrollDelta::Lines { x: 0.0, y: 5.0 }),
        pos: Point::new(25.0, 25.0),
        modifiers: Modifiers::NONE,
    };
    scroll.handle_mouse(&event, &ctx);
    assert_eq!(scroll.scroll_offset(), 0.0);
}

#[test]
fn scroll_pixel_delta_works() {
    let mut scroll = ScrollWidget::vertical(tall_content());
    let measurer = MockMeasurer::STANDARD;
    let bounds = Rect::new(0.0, 0.0, 200.0, 100.0);
    let ctx = EventCtx {
        measurer: &measurer,
        bounds,
        is_focused: true,
        focused_widget: None,
        theme: &super::super::tests::TEST_THEME,
        interaction: None,
        widget_id: None,
        frame_requests: None,
    };

    // Trackpad-style pixel delta.
    let event = MouseEvent {
        kind: MouseEventKind::Scroll(ScrollDelta::Pixels { x: 0.0, y: -35.0 }),
        pos: Point::new(25.0, 25.0),
        modifiers: Modifiers::NONE,
    };
    scroll.handle_mouse(&event, &ctx);
    assert_eq!(scroll.scroll_offset(), 35.0);
}

#[test]
fn scroll_delegates_non_scroll_mouse_to_child() {
    // Button inside scroll container — click should reach it.
    let btn = ButtonWidget::new("Click");
    let btn_id = btn.id();
    let mut scroll = ScrollWidget::vertical(Box::new(btn));

    let measurer = MockMeasurer::STANDARD;
    let bounds = Rect::new(0.0, 0.0, 200.0, 100.0);
    let ctx = EventCtx {
        measurer: &measurer,
        bounds,
        is_focused: false,
        focused_widget: None,
        theme: &super::super::tests::TEST_THEME,
        interaction: None,
        widget_id: None,
        frame_requests: None,
    };

    let down = MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        pos: Point::new(10.0, 10.0),
        modifiers: Modifiers::NONE,
    };
    let _ = scroll.handle_mouse(&down, &ctx);

    let up = MouseEvent {
        kind: MouseEventKind::Up(MouseButton::Left),
        pos: Point::new(10.0, 10.0),
        modifiers: Modifiers::NONE,
    };
    let resp = scroll.handle_mouse(&up, &ctx);

    match resp.action {
        Some(crate::widgets::WidgetAction::Clicked(id)) => assert_eq!(id, btn_id),
        other => panic!("expected Clicked through scroll, got {other:?}"),
    }
}

#[test]
fn scroll_delegates_click_with_nonzero_origin() {
    // Button inside scroll container at non-zero origin —
    // verifies coordinate system is correct when scroll widget isn't at (0,0).
    let btn = ButtonWidget::new("Click");
    let btn_id = btn.id();
    let mut scroll = ScrollWidget::vertical(Box::new(btn));

    let measurer = MockMeasurer::STANDARD;
    let bounds = Rect::new(100.0, 200.0, 300.0, 150.0);
    let ctx = EventCtx {
        measurer: &measurer,
        bounds,
        is_focused: false,
        focused_widget: None,
        theme: &super::super::tests::TEST_THEME,
        interaction: None,
        widget_id: None,
        frame_requests: None,
    };

    // Click inside the button area (offset from bounds origin).
    let down = MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        pos: Point::new(110.0, 210.0),
        modifiers: Modifiers::NONE,
    };
    let _ = scroll.handle_mouse(&down, &ctx);

    let up = MouseEvent {
        kind: MouseEventKind::Up(MouseButton::Left),
        pos: Point::new(110.0, 210.0),
        modifiers: Modifiers::NONE,
    };
    let resp = scroll.handle_mouse(&up, &ctx);

    match resp.action {
        Some(crate::widgets::WidgetAction::Clicked(id)) => assert_eq!(id, btn_id),
        other => panic!("expected Clicked through scroll at non-zero origin, got {other:?}"),
    }
}

#[test]
fn scroll_delegates_checkbox_toggle_through_form_hierarchy() {
    // Full form hierarchy: ScrollWidget → FormLayout → FormSection → FormRow → Checkbox
    // This tests the complete settings panel event chain.
    use crate::widgets::checkbox::CheckboxWidget;
    use crate::widgets::form_layout::FormLayout;
    use crate::widgets::form_row::FormRow;
    use crate::widgets::form_section::FormSection;

    let checkbox = CheckboxWidget::new("Enable feature");
    let checkbox_id = checkbox.id();

    let mut form = FormLayout::new().with_section(
        FormSection::new("Test Section").with_row(FormRow::new("My Setting", Box::new(checkbox))),
    );
    form.compute_label_widths(&MockMeasurer::STANDARD, &super::super::tests::TEST_THEME);

    let mut scroll = ScrollWidget::vertical(Box::new(form));

    let measurer = MockMeasurer::STANDARD;
    // Non-zero origin simulating a centered overlay panel.
    let bounds = Rect::new(150.0, 100.0, 500.0, 400.0);

    // First, draw to populate layout caches (matching real app behavior).
    let mut draw_list = DrawList::new();
    let anim_flag = std::cell::Cell::new(false);
    let mut draw_ctx = DrawCtx {
        measurer: &measurer,
        draw_list: &mut draw_list,
        bounds,
        focused_widget: None,
        now: std::time::Instant::now(),
        animations_running: &anim_flag,
        theme: &super::super::tests::TEST_THEME,
        icons: None,
        scene_cache: None,
        interaction: None,
        widget_id: None,
        frame_requests: None,
    };
    scroll.paint(&mut draw_ctx);

    // Now send mouse events. The click target needs to be within the
    // control column of the form row. The label column is on the left,
    // the control is on the right. We click far enough right to be in
    // the control zone.
    let event_ctx = EventCtx {
        measurer: &measurer,
        bounds,
        is_focused: false,
        focused_widget: None,
        theme: &super::super::tests::TEST_THEME,
        interaction: None,
        widget_id: None,
        frame_requests: None,
    };

    // Click in the right half of the form (control column area).
    // Form padding top (16) + section header (28) + row gap (12) = 56,
    // so +60 is in the first row. Label column is ~100px, control is right.
    let click_x = bounds.x() + 150.0; // In the control column (checkbox natural width)
    let click_y = bounds.y() + 60.0; // Below form padding + section header + gap

    let down = MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        pos: Point::new(click_x, click_y),
        modifiers: Modifiers::NONE,
    };
    let _ = scroll.handle_mouse(&down, &event_ctx);

    let up = MouseEvent {
        kind: MouseEventKind::Up(MouseButton::Left),
        pos: Point::new(click_x, click_y),
        modifiers: Modifiers::NONE,
    };
    let resp = scroll.handle_mouse(&up, &event_ctx);

    match resp.action {
        Some(crate::widgets::WidgetAction::Toggled { id, value }) => {
            assert_eq!(id, checkbox_id, "toggled wrong checkbox");
            assert!(value, "checkbox should be checked after toggle");
        }
        other => panic!(
            "expected Toggled action from checkbox click at ({click_x}, {click_y}), got {other:?}"
        ),
    }
}

#[test]
fn container_with_scroll_form_click_reaches_checkbox() {
    // Simulates the SettingsPanel structure: Container(column) with
    // header + scroll(FormLayout). Verifies click events route through
    // the Container's capture semantics to the checkbox.
    use crate::layout::SizeSpec;
    use crate::widgets::checkbox::CheckboxWidget;
    use crate::widgets::form_layout::FormLayout;
    use crate::widgets::form_row::FormRow;
    use crate::widgets::form_section::FormSection;
    use crate::widgets::separator::SeparatorWidget;

    let checkbox = CheckboxWidget::new("Test toggle");
    let checkbox_id = checkbox.id();

    let mut form = FormLayout::new().with_section(
        FormSection::new("General").with_row(FormRow::new("My option", Box::new(checkbox))),
    );
    form.compute_label_widths(&MockMeasurer::STANDARD, &super::super::tests::TEST_THEME);

    let scroll = ScrollWidget::vertical(Box::new(form));

    // Simulate SettingsPanel's container structure.
    let header = LabelWidget::new("Settings");
    let separator = SeparatorWidget::horizontal();

    let mut container = ContainerWidget::column()
        .with_width(SizeSpec::Fixed(500.0))
        .with_height(SizeSpec::Hug)
        .with_child(Box::new(header))
        .with_child(Box::new(separator))
        .with_child(Box::new(scroll));

    let measurer = MockMeasurer::STANDARD;
    // Simulate centered overlay at a non-zero position.
    let bounds = Rect::new(140.0, 80.0, 500.0, 400.0);

    // Draw first to populate caches.
    let mut draw_list = DrawList::new();
    let anim_flag = std::cell::Cell::new(false);
    let mut draw_ctx = DrawCtx {
        measurer: &measurer,
        draw_list: &mut draw_list,
        bounds,
        focused_widget: None,
        now: std::time::Instant::now(),
        animations_running: &anim_flag,
        theme: &super::super::tests::TEST_THEME,
        icons: None,
        scene_cache: None,
        interaction: None,
        widget_id: None,
        frame_requests: None,
    };
    container.paint(&mut draw_ctx);

    // Compute layout to find exact scroll widget position for click targeting.
    let layout_ctx = LayoutCtx {
        measurer: &measurer,
        theme: &super::super::tests::TEST_THEME,
    };
    let layout_box = container.layout(&layout_ctx);
    let layout = compute_layout(&layout_box, bounds);

    // Click in the control column area, below the header + separator.
    // header=child[0], separator=child[1], scroll=child[2].
    let scroll_node = &layout.children[2];
    // Offset 60 = past form padding(16) + section header(28) + gap(12) into row.
    let click_x = scroll_node.rect.x() + 150.0;
    let click_y = scroll_node.rect.y() + 60.0;

    let event_ctx = EventCtx {
        measurer: &measurer,
        bounds,
        is_focused: false,
        focused_widget: None,
        theme: &super::super::tests::TEST_THEME,
        interaction: None,
        widget_id: None,
        frame_requests: None,
    };

    let down = MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        pos: Point::new(click_x, click_y),
        modifiers: Modifiers::NONE,
    };
    let down_resp = container.handle_mouse(&down, &event_ctx);

    let up = MouseEvent {
        kind: MouseEventKind::Up(MouseButton::Left),
        pos: Point::new(click_x, click_y),
        modifiers: Modifiers::NONE,
    };
    let up_resp = container.handle_mouse(&up, &event_ctx);

    match up_resp.action {
        Some(crate::widgets::WidgetAction::Toggled { id, value }) => {
            assert_eq!(id, checkbox_id, "toggled wrong checkbox");
            assert!(value, "checkbox should be checked after toggle");
        }
        other => panic!(
            "expected Toggled action from container→scroll→form→checkbox click at ({click_x}, {click_y}), \
             down_resp={down_resp:?}, got {other:?}"
        ),
    }
}

#[test]
fn arrow_up_scrolls_upward() {
    let mut scroll = ScrollWidget::vertical(tall_content());
    // Start scrolled down.
    scroll.set_scroll_offset(100.0, 320.0, 100.0);

    let measurer = MockMeasurer::STANDARD;
    let bounds = Rect::new(0.0, 0.0, 200.0, 100.0);
    let ctx = EventCtx {
        measurer: &measurer,
        bounds,
        is_focused: true,
        focused_widget: None,
        theme: &super::super::tests::TEST_THEME,
        interaction: None,
        widget_id: None,
        frame_requests: None,
    };

    let event = KeyEvent {
        key: Key::ArrowUp,
        modifiers: Modifiers::NONE,
    };
    scroll.handle_key(event, &ctx);
    assert_eq!(scroll.scroll_offset(), 80.0); // 100 - 20
}

// Horizontal and both-direction tests (Chromium scroll view patterns)

/// Creates a wide row of labels that overflows a narrow viewport.
/// 20 labels * 8px * 10 chars = 1600px wide.
fn wide_content() -> Box<dyn Widget> {
    let labels: Vec<Box<dyn Widget>> = (0..20)
        .map(|i| Box::new(LabelWidget::new(format!("HorizLbl{i}"))) as Box<dyn Widget>)
        .collect();
    Box::new(ContainerWidget::row().with_children(labels))
}

#[test]
fn horizontal_scroll_new_constructor() {
    let scroll = ScrollWidget::new(wide_content(), super::ScrollDirection::Horizontal);
    assert_eq!(scroll.scroll_offset(), 0.0);
    assert!(scroll.is_focusable());
}

#[test]
fn both_direction_new_constructor() {
    let scroll = ScrollWidget::new(tall_content(), super::ScrollDirection::Both);
    assert_eq!(scroll.scroll_offset(), 0.0);
    assert!(scroll.is_focusable());
}

#[test]
fn horizontal_scroll_draws_with_clip() {
    let scroll = ScrollWidget::new(wide_content(), super::ScrollDirection::Horizontal);
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
        scene_cache: None,
        interaction: None,
        widget_id: None,
        frame_requests: None,
    };
    scroll.paint(&mut ctx);

    // Clip should be balanced.
    let push_count = draw_list
        .commands()
        .iter()
        .filter(|c| matches!(c, DrawCommand::PushClip { .. }))
        .count();
    let pop_count = draw_list
        .commands()
        .iter()
        .filter(|c| matches!(c, DrawCommand::PopClip))
        .count();
    assert_eq!(push_count, 1);
    assert_eq!(pop_count, 1);
}

#[test]
fn scroll_content_exactly_fits_viewport() {
    // When content height == viewport height, max offset should be 0.
    let mut scroll = ScrollWidget::vertical(tall_content());
    // tall_content = 320px. Set viewport to 320px.
    scroll.set_scroll_offset(50.0, 320.0, 320.0);
    assert_eq!(scroll.scroll_offset(), 0.0, "no scroll when content fits");
}

#[test]
fn scroll_content_exactly_fits_no_scrollbar() {
    // Content exactly fitting the viewport should not draw a scrollbar.
    let label = LabelWidget::new("A".repeat(10)); // 80px wide, 16px tall
    let scroll = ScrollWidget::vertical(Box::new(label));
    let measurer = MockMeasurer::STANDARD;
    let mut draw_list = DrawList::new();
    // Viewport exactly matches content height (16px).
    let bounds = Rect::new(0.0, 0.0, 200.0, 16.0);
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
        scene_cache: None,
        interaction: None,
        widget_id: None,
        frame_requests: None,
    };
    scroll.paint(&mut ctx);

    // No scrollbar rects after PopClip.
    let after_pop = draw_list
        .commands()
        .iter()
        .skip_while(|c| !matches!(c, DrawCommand::PopClip))
        .filter(|c| matches!(c, DrawCommand::Rect { .. }))
        .count();
    assert_eq!(after_pop, 0, "no scrollbar when content exactly fits");
}

#[test]
fn scroll_hover_delegates_to_child() {
    use crate::input::HoverEvent;

    let btn = ButtonWidget::new("HoverMe");
    let mut scroll = ScrollWidget::vertical(Box::new(btn));
    let measurer = MockMeasurer::STANDARD;
    let bounds = Rect::new(0.0, 0.0, 200.0, 100.0);
    let ctx = EventCtx {
        measurer: &measurer,
        bounds,
        is_focused: false,
        focused_widget: None,
        theme: &super::super::tests::TEST_THEME,
        interaction: None,
        widget_id: None,
        frame_requests: None,
    };

    // Hover should delegate to the child.
    let resp = scroll.handle_hover(HoverEvent::Enter, &ctx);
    // ButtonWidget returns redraw on hover enter.
    assert!(resp.response.is_handled());
}

#[test]
fn scroll_track_hovered_resets_on_leave() {
    use crate::input::HoverEvent;

    let mut scroll = ScrollWidget::vertical(tall_content());
    let measurer = MockMeasurer::STANDARD;
    let bounds = Rect::new(0.0, 0.0, 200.0, 100.0);
    let ctx = EventCtx {
        measurer: &measurer,
        bounds,
        is_focused: false,
        focused_widget: None,
        theme: &super::super::tests::TEST_THEME,
        interaction: None,
        widget_id: None,
        frame_requests: None,
    };

    // Simulate scrollbar hover by setting track_hovered manually.
    scroll.scrollbar.track_hovered = true;

    // Leave event should reset track_hovered.
    scroll.handle_hover(HoverEvent::Leave, &ctx);
    assert!(
        !scroll.scrollbar.track_hovered,
        "track_hovered should be false after Leave event"
    );
}

#[test]
fn scroll_with_scrollbar_style() {
    use super::ScrollbarStyle;
    use crate::color::Color;

    let custom_style = ScrollbarStyle {
        width: 10.0,
        thumb_color: Color::WHITE,
        track_color: Color::BLACK,
        thumb_radius: 5.0,
        min_thumb_height: 30.0,
    };
    let scroll = ScrollWidget::vertical(tall_content()).with_scrollbar_style(custom_style);
    // Just verify it doesn't panic and produces valid output.
    let measurer = MockMeasurer::STANDARD;
    let mut draw_list = DrawList::new();
    let bounds = Rect::new(0.0, 0.0, 200.0, 100.0);
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
        scene_cache: None,
        interaction: None,
        widget_id: None,
        frame_requests: None,
    };
    scroll.paint(&mut ctx);
    assert!(!draw_list.is_empty());
}
