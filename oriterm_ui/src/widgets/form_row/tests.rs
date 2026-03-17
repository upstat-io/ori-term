use crate::draw::DrawList;
use crate::geometry::Rect;
use crate::layout::compute_layout;
use crate::widgets::button::ButtonWidget;
use crate::widgets::label::LabelWidget;
use crate::widgets::tests::MockMeasurer;
use crate::widgets::{DrawCtx, LayoutCtx, Widget};

use super::FormRow;

fn label_control(label_text: &str, control_text: &str) -> FormRow {
    FormRow::new(label_text, Box::new(LabelWidget::new(control_text)))
}

#[test]
fn layout_produces_two_column_structure() {
    let row = label_control("Name", "value");
    let ctx = LayoutCtx {
        measurer: &MockMeasurer::STANDARD,
        theme: &super::super::tests::TEST_THEME,
    };
    let layout_box = row.layout(&ctx);
    let viewport = Rect::new(0.0, 0.0, 400.0, 50.0);
    let node = compute_layout(&layout_box, viewport);

    // Row should have 2 children: label and control.
    assert_eq!(node.children.len(), 2);
    // Label column has fixed width (default 100).
    assert_eq!(node.children[0].rect.width(), 100.0);
    // Control takes its natural width (not forced to fill).
    assert!(node.children[1].rect.width() > 0.0);
}

#[test]
fn label_width_changes_column_proportions() {
    let mut row = label_control("Name", "value");
    row.set_label_width(150.0);
    let ctx = LayoutCtx {
        measurer: &MockMeasurer::STANDARD,
        theme: &super::super::tests::TEST_THEME,
    };
    let layout_box = row.layout(&ctx);
    let viewport = Rect::new(0.0, 0.0, 400.0, 50.0);
    let node = compute_layout(&layout_box, viewport);

    assert_eq!(node.children[0].rect.width(), 150.0);
    // Control takes its natural width (not forced to fill).
    assert!(node.children[1].rect.width() > 0.0);
}

#[test]
fn not_focusable() {
    let row = label_control("Test", "val");
    assert!(!row.is_focusable());
}

#[test]
fn focusable_children_returns_control_ids() {
    let btn = ButtonWidget::new("Click");
    let btn_id = btn.id();
    let row = FormRow::new("Action", Box::new(btn));
    let ids = row.focusable_children();
    assert_eq!(ids, vec![btn_id]);
}

#[test]
fn draw_produces_text_commands() {
    let row = label_control("Name", "value");
    let measurer = MockMeasurer::STANDARD;
    let mut draw_list = DrawList::new();
    let bounds = Rect::new(0.0, 0.0, 400.0, 50.0);
    let mut ctx = DrawCtx {
        measurer: &measurer,
        draw_list: &mut draw_list,
        bounds,
        focused_widget: None,
        now: std::time::Instant::now(),
        theme: &super::super::tests::TEST_THEME,
        icons: None,
        scene_cache: None,
        interaction: None,
        widget_id: None,
        frame_requests: None,
    };
    row.paint(&mut ctx);

    let text_cmds = draw_list
        .commands()
        .iter()
        .filter(|c| matches!(c, crate::draw::DrawCommand::Text { .. }))
        .count();
    // Label text + control label text = 2 text commands.
    assert_eq!(text_cmds, 2);
}

#[test]
fn measure_label_width_returns_text_width() {
    let row = label_control("ABCD", "value");
    let measurer = MockMeasurer::STANDARD;
    let theme = super::super::tests::TEST_THEME;
    let width = row.measure_label_width(&measurer, &theme);
    // MockMeasurer: 8px per char, "ABCD" = 32px.
    assert_eq!(width, 32.0);
}
