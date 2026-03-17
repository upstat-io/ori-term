use crate::draw::{DrawCommand, DrawList};
use crate::geometry::Rect;
use crate::layout::compute_layout;
use crate::widgets::container::ContainerWidget;
use crate::widgets::label::LabelWidget;
use crate::widgets::tests::MockMeasurer;
use crate::widgets::{DrawCtx, LayoutCtx, Widget};

use super::StackWidget;

fn label(text: &str) -> Box<dyn Widget> {
    Box::new(LabelWidget::new(text))
}

#[test]
fn stack_sizes_to_largest_child() {
    // "AB" = 16px, "ABCD" = 32px. Stack should be 32x16.
    let stack = StackWidget::new(vec![label("AB"), label("ABCD")]);
    let ctx = LayoutCtx {
        measurer: &MockMeasurer::STANDARD,
        theme: &super::super::tests::TEST_THEME,
    };
    let layout_box = stack.layout(&ctx);
    let viewport = Rect::new(0.0, 0.0, 400.0, 300.0);
    let node = compute_layout(&layout_box, viewport);

    assert_eq!(node.rect.width(), 32.0);
    assert_eq!(node.rect.height(), 16.0);
}

#[test]
fn stack_draws_all_children() {
    let stack = StackWidget::new(vec![label("A"), label("B")]);
    let measurer = MockMeasurer::STANDARD;
    let mut draw_list = DrawList::new();
    let bounds = Rect::new(0.0, 0.0, 100.0, 50.0);
    let mut ctx = DrawCtx {
        measurer: &measurer,
        draw_list: &mut draw_list,
        bounds,
        now: std::time::Instant::now(),
        theme: &super::super::tests::TEST_THEME,
        icons: None,
        scene_cache: None,
        interaction: None,
        widget_id: None,
        frame_requests: None,
    };
    stack.paint(&mut ctx);

    let text_cmds = draw_list
        .commands()
        .iter()
        .filter(|c| matches!(c, DrawCommand::Text { .. }))
        .count();
    assert_eq!(text_cmds, 2, "both children should be drawn");
}

#[test]
fn stack_not_focusable() {
    let stack = StackWidget::new(vec![]);
    assert!(!stack.is_focusable());
}

#[test]
fn stack_child_count() {
    let stack = StackWidget::new(vec![label("A"), label("B"), label("C")]);
    assert_eq!(stack.child_count(), 3);
}

#[test]
fn stack_empty() {
    let stack = StackWidget::new(vec![]);
    let ctx = LayoutCtx {
        measurer: &MockMeasurer::STANDARD,
        theme: &super::super::tests::TEST_THEME,
    };
    let layout_box = stack.layout(&ctx);
    let viewport = Rect::new(0.0, 0.0, 400.0, 300.0);
    let node = compute_layout(&layout_box, viewport);
    assert_eq!(node.rect.width(), 0.0);
    assert_eq!(node.rect.height(), 0.0);
}

// Edge cases from Chromium/Ratatui audit

#[test]
fn stack_draws_in_painter_order() {
    // Verify the first child is drawn before the last (painter's order).
    let stack = StackWidget::new(vec![label("Back"), label("Front")]);
    let measurer = MockMeasurer::STANDARD;
    let mut draw_list = DrawList::new();
    let bounds = Rect::new(0.0, 0.0, 100.0, 50.0);
    let mut ctx = DrawCtx {
        measurer: &measurer,
        draw_list: &mut draw_list,
        bounds,
        now: std::time::Instant::now(),
        theme: &super::super::tests::TEST_THEME,
        icons: None,
        scene_cache: None,
        interaction: None,
        widget_id: None,
        frame_requests: None,
    };
    stack.paint(&mut ctx);

    // Both are Text commands — first drawn is "Back", second is "Front".
    let texts: Vec<&str> = draw_list
        .commands()
        .iter()
        .filter_map(|c| match c {
            DrawCommand::Text { shaped, .. } => {
                // Use glyph count as proxy — "Back" has 4 glyphs, "Front" has 5.
                Some(if shaped.glyph_count() == 4 {
                    "Back"
                } else {
                    "Front"
                })
            }
            _ => None,
        })
        .collect();
    assert_eq!(texts, vec!["Back", "Front"]);
}

#[test]
fn stack_sizes_to_flex_child() {
    // A Flex (Column) child with two labels should contribute its natural size.
    // 2 labels * 16px = 32px tall, "Hello" = 5*8 = 40px wide.
    let col: Box<dyn Widget> =
        Box::new(ContainerWidget::column().with_children(vec![label("Hello"), label("World")]));
    let stack = StackWidget::new(vec![col]);
    let ctx = LayoutCtx {
        measurer: &MockMeasurer::STANDARD,
        theme: &super::super::tests::TEST_THEME,
    };
    let layout_box = stack.layout(&ctx);
    let viewport = Rect::new(0.0, 0.0, 400.0, 300.0);
    let node = compute_layout(&layout_box, viewport);

    assert_eq!(node.rect.width(), 40.0);
    assert_eq!(node.rect.height(), 32.0);
}

#[test]
fn stack_sizes_to_largest_including_flex() {
    // Mix of Leaf (label) and Flex (column) children. Stack sizes to the largest.
    // Label "Wide label!!" = 12*8 = 96px wide, 16px tall.
    // Column of 3 labels = 3*16 = 48px tall, "AB" = 16px wide.
    let wide_label = label("Wide label!!");
    let tall_col: Box<dyn Widget> = Box::new(ContainerWidget::column().with_children(vec![
        label("AB"),
        label("AB"),
        label("AB"),
    ]));
    let stack = StackWidget::new(vec![wide_label, tall_col]);
    let ctx = LayoutCtx {
        measurer: &MockMeasurer::STANDARD,
        theme: &super::super::tests::TEST_THEME,
    };
    let layout_box = stack.layout(&ctx);
    let viewport = Rect::new(0.0, 0.0, 400.0, 300.0);
    let node = compute_layout(&layout_box, viewport);

    // Width from label (96px), height from column (48px).
    assert_eq!(node.rect.width(), 96.0);
    assert_eq!(node.rect.height(), 48.0);
}
