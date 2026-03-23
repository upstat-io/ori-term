use crate::layout::BoxContent;
use crate::sense::Sense;
use crate::widgets::tests::MockMeasurer;
use crate::widgets::{LayoutCtx, Widget};

use super::{SliderStyle, SliderWidget};

#[test]
fn default_state() {
    let s = SliderWidget::new();
    assert_eq!(s.value(), 0.0);
    assert_eq!(s.min(), 0.0);
    assert_eq!(s.max(), 1.0);
    assert!(!s.is_disabled());
    assert!(s.is_focusable());
}

#[test]
fn with_range_and_value() {
    let s = SliderWidget::new().with_range(10.0, 100.0).with_value(50.0);
    assert_eq!(s.value(), 50.0);
    assert_eq!(s.min(), 10.0);
    assert_eq!(s.max(), 100.0);
}

#[test]
fn value_clamped_to_range() {
    let s = SliderWidget::new().with_range(0.0, 10.0).with_value(20.0);
    assert_eq!(s.value(), 10.0);
}

#[test]
fn layout_dimensions() {
    let s = SliderWidget::new();
    let m = MockMeasurer::new();
    let ctx = LayoutCtx {
        measurer: &m,
        theme: &super::super::tests::TEST_THEME,
    };
    let layout = s.layout(&ctx);
    let style = SliderStyle::default();

    if let BoxContent::Leaf {
        intrinsic_width,
        intrinsic_height,
    } = &layout.content
    {
        assert_eq!(*intrinsic_width, style.width);
        assert_eq!(*intrinsic_height, style.thumb_height);
    } else {
        panic!("expected leaf layout");
    }
}

#[test]
fn min_equals_max_returns_min() {
    let s = SliderWidget::new().with_range(5.0, 5.0).with_value(5.0);
    assert_eq!(s.value(), 5.0);
}

#[test]
fn set_value_clamps() {
    let mut s = SliderWidget::new().with_range(0.0, 10.0);
    s.set_value(20.0);
    assert_eq!(s.value(), 10.0);
    s.set_value(-5.0);
    assert_eq!(s.value(), 0.0);
}

// -- Sense and controllers --

#[test]
fn sense_returns_click_and_drag() {
    let s = SliderWidget::new();
    assert_eq!(s.sense(), Sense::click_and_drag());
}

#[test]
fn has_two_controllers() {
    let s = SliderWidget::new();
    assert_eq!(s.controllers().len(), 2);
}

#[test]
fn has_visual_state_animator() {
    let s = SliderWidget::new();
    assert!(s.visual_states().is_some());
}
