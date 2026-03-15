use crate::draw::DrawList;
use crate::geometry::{Point, Rect};
use crate::input::{Modifiers, MouseButton, MouseEvent, MouseEventKind};
use crate::layout::{SizeSpec, compute_layout};
use crate::widgets::button::ButtonWidget;
use crate::widgets::form_row::FormRow;
use crate::widgets::label::LabelWidget;
use crate::widgets::tests::MockMeasurer;
use crate::widgets::{DrawCtx, EventCtx, LayoutCtx, Widget, WidgetResponse};

use super::FormSection;

fn test_section() -> FormSection {
    FormSection::new("Appearance")
        .with_row(FormRow::new("Color", Box::new(LabelWidget::new("Blue"))))
        .with_row(FormRow::new("Size", Box::new(LabelWidget::new("12"))))
}

#[test]
fn expanded_layout_includes_all_rows() {
    let section = test_section();
    let ctx = LayoutCtx {
        measurer: &MockMeasurer::STANDARD,
        theme: &super::super::tests::TEST_THEME,
    };
    let mut layout_box = section.layout(&ctx);
    layout_box.width = SizeSpec::Fill;
    let viewport = Rect::new(0.0, 0.0, 400.0, 300.0);
    let node = compute_layout(&layout_box, viewport);

    // Header + 2 rows = 3 children.
    assert_eq!(node.children.len(), 3);
}

#[test]
fn collapsed_layout_only_has_header() {
    let section = test_section().expanded(false);
    let ctx = LayoutCtx {
        measurer: &MockMeasurer::STANDARD,
        theme: &super::super::tests::TEST_THEME,
    };
    let mut layout_box = section.layout(&ctx);
    layout_box.width = SizeSpec::Fill;
    let viewport = Rect::new(0.0, 0.0, 400.0, 300.0);
    let node = compute_layout(&layout_box, viewport);

    // Only header.
    assert_eq!(node.children.len(), 1);
}

#[test]
fn collapsed_height_less_than_expanded() {
    let ctx = LayoutCtx {
        measurer: &MockMeasurer::STANDARD,
        theme: &super::super::tests::TEST_THEME,
    };
    let viewport = Rect::new(0.0, 0.0, 400.0, 300.0);

    let expanded = test_section();
    let mut eb = expanded.layout(&ctx);
    eb.width = SizeSpec::Fill;
    let expanded_node = compute_layout(&eb, viewport);

    let collapsed = test_section().expanded(false);
    let mut cb = collapsed.layout(&ctx);
    cb.width = SizeSpec::Fill;
    let collapsed_node = compute_layout(&cb, viewport);

    assert!(
        collapsed_node.rect.height() < expanded_node.rect.height(),
        "collapsed {} should be < expanded {}",
        collapsed_node.rect.height(),
        expanded_node.rect.height(),
    );
}

#[test]
fn click_on_header_toggles_expanded() {
    let mut section = test_section();
    assert!(section.is_expanded());

    let measurer = MockMeasurer::STANDARD;
    let bounds = Rect::new(0.0, 0.0, 400.0, 300.0);
    let ctx = EventCtx {
        measurer: &measurer,
        bounds,
        is_focused: false,
        focused_widget: None,
        theme: &super::super::tests::TEST_THEME,
        interaction: None,
        widget_id: None,
    };

    // Click on header (y < HEADER_HEIGHT=28).
    let event = MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        pos: Point::new(50.0, 10.0),
        modifiers: Modifiers::NONE,
    };
    let resp = section.handle_mouse(&event, &ctx);
    assert!(!section.is_expanded());
    assert_eq!(resp, WidgetResponse::layout());

    // Click again to re-expand.
    let _ = section.handle_mouse(&event, &ctx);
    assert!(section.is_expanded());
}

#[test]
fn focusable_children_empty_when_collapsed() {
    let btn = ButtonWidget::new("Click");
    let btn_id = btn.id();
    let section = FormSection::new("Test").with_row(FormRow::new("Action", Box::new(btn)));

    // When expanded, button is focusable.
    assert_eq!(section.focusable_children(), vec![btn_id]);

    // When collapsed, no focusable children.
    let collapsed = FormSection::new("Test")
        .with_row(FormRow::new("Action", Box::new(ButtonWidget::new("Click"))))
        .expanded(false);
    assert!(collapsed.focusable_children().is_empty());
}

#[test]
fn not_focusable() {
    let section = test_section();
    assert!(!section.is_focusable());
}

#[test]
fn draw_produces_header_text() {
    let section = test_section();
    let measurer = MockMeasurer::STANDARD;
    let mut draw_list = DrawList::new();
    let bounds = Rect::new(0.0, 0.0, 400.0, 300.0);
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
    };
    section.draw(&mut ctx);

    let text_cmds = draw_list
        .commands()
        .iter()
        .filter(|c| matches!(c, crate::draw::DrawCommand::Text { .. }))
        .count();
    // Indicator + title + 2 rows × (label + control) = 2 + 4 = 6.
    assert!(
        text_cmds >= 2,
        "expected at least header text commands, got {text_cmds}"
    );
}

#[test]
fn draw_skips_rows_outside_active_clip() {
    let section = test_section();
    let measurer = MockMeasurer::STANDARD;
    let mut draw_list = DrawList::new();
    draw_list.push_clip(Rect::new(0.0, 0.0, 400.0, 60.0));
    let bounds = Rect::new(0.0, 0.0, 400.0, 300.0);
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
    };
    section.draw(&mut ctx);

    let text_cmds = draw_list
        .commands()
        .iter()
        .filter(|c| matches!(c, crate::draw::DrawCommand::Text { .. }))
        .count();
    assert_eq!(text_cmds, 4, "header and first visible row should draw");
}
