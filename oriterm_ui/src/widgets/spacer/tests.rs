use crate::geometry::Rect;
use crate::layout::{SizeSpec, compute_layout};
use crate::widgets::tests::MockMeasurer;
use crate::widgets::{LayoutCtx, Widget};

use super::SpacerWidget;

#[test]
fn fixed_spacer_layout() {
    let spacer = SpacerWidget::fixed(20.0, 10.0);
    let ctx = LayoutCtx {
        measurer: &MockMeasurer::STANDARD,
        theme: &super::super::tests::TEST_THEME,
    };
    let layout_box = spacer.layout(&ctx);
    let viewport = Rect::new(0.0, 0.0, 400.0, 300.0);
    let node = compute_layout(&layout_box, viewport);
    assert_eq!(node.rect.width(), 20.0);
    assert_eq!(node.rect.height(), 10.0);
}

#[test]
fn fill_spacer_fills_viewport() {
    let spacer = SpacerWidget::fill();
    let ctx = LayoutCtx {
        measurer: &MockMeasurer::STANDARD,
        theme: &super::super::tests::TEST_THEME,
    };
    let layout_box = spacer.layout(&ctx);
    let viewport = Rect::new(0.0, 0.0, 400.0, 300.0);
    let node = compute_layout(&layout_box, viewport);
    assert_eq!(node.rect.width(), 400.0);
    assert_eq!(node.rect.height(), 300.0);
}

#[test]
fn spacer_not_focusable() {
    let s = SpacerWidget::fixed(10.0, 10.0);
    assert!(!s.is_focusable());
}

#[test]
fn spacer_ids_unique() {
    let a = SpacerWidget::fixed(10.0, 10.0);
    let b = SpacerWidget::fill();
    assert_ne!(a.id(), b.id());
}

#[test]
fn fixed_spacer_size_spec() {
    let spacer = SpacerWidget::fixed(50.0, 25.0);
    let ctx = LayoutCtx {
        measurer: &MockMeasurer::STANDARD,
        theme: &super::super::tests::TEST_THEME,
    };
    let layout_box = spacer.layout(&ctx);
    assert_eq!(layout_box.width, SizeSpec::Fixed(50.0));
    assert_eq!(layout_box.height, SizeSpec::Fixed(25.0));
}

#[test]
fn fill_spacer_size_spec() {
    let spacer = SpacerWidget::fill();
    let ctx = LayoutCtx {
        measurer: &MockMeasurer::STANDARD,
        theme: &super::super::tests::TEST_THEME,
    };
    let layout_box = spacer.layout(&ctx);
    assert_eq!(layout_box.width, SizeSpec::Fill);
    assert_eq!(layout_box.height, SizeSpec::Fill);
}
