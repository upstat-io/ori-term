use winit::window::CursorIcon;

use crate::layout::BoxContent;
use crate::sense::Sense;
use crate::widgets::tests::MockMeasurer;
use crate::widgets::{LayoutCtx, Widget};

use super::{SliderStyle, SliderWidget, ValueDisplay};

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

    let expected_w = style.width + super::VALUE_GAP + super::VALUE_LABEL_WIDTH;
    if let BoxContent::Leaf {
        intrinsic_width,
        intrinsic_height,
    } = &layout.content
    {
        assert_eq!(*intrinsic_width, expected_w);
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

// -- Constants --

#[test]
fn slider_value_gap_is_10px() {
    assert_eq!(super::VALUE_GAP, 10.0);
}

#[test]
fn slider_value_label_width_is_32px() {
    assert_eq!(super::VALUE_LABEL_WIDTH, 32.0);
}

// -- Value display formatting --

#[test]
fn slider_percent_display_mode() {
    let s = SliderWidget::new()
        .with_range(0.0, 100.0)
        .with_step(1.0)
        .with_value(100.0)
        .with_display(ValueDisplay::Percent);
    assert_eq!(s.format_value(), "100%");
}

#[test]
fn slider_value_at_min_shows_correct_format() {
    let s = SliderWidget::new()
        .with_range(30.0, 100.0)
        .with_step(1.0)
        .with_value(30.0)
        .with_display(ValueDisplay::Percent);
    assert_eq!(s.format_value(), "30%");
}

#[test]
fn slider_suffix_display_mode() {
    let s = SliderWidget::new()
        .with_range(0.0, 100.0)
        .with_step(1.0)
        .with_value(14.0)
        .with_display(ValueDisplay::Suffix("px"));
    assert_eq!(s.format_value(), "14px");
}

#[test]
fn slider_format_value_numeric() {
    let s = SliderWidget::new()
        .with_range(0.0, 100.0)
        .with_step(1.0)
        .with_value(42.0)
        .with_display(ValueDisplay::Numeric);
    assert_eq!(s.format_value(), "42");
}

#[test]
fn slider_hidden_display_mode() {
    let s = SliderWidget::new()
        .with_range(0.0, 100.0)
        .with_step(1.0)
        .with_value(50.0)
        .with_display(ValueDisplay::Hidden);
    assert_eq!(s.format_value(), "");
}

// -- Cursor icon --

#[test]
fn layout_cursor_icon_pointer() {
    let s = SliderWidget::new();
    let m = MockMeasurer::new();
    let ctx = LayoutCtx {
        measurer: &m,
        theme: &super::super::tests::TEST_THEME,
    };
    let layout = s.layout(&ctx);
    assert_eq!(
        layout.cursor_icon,
        CursorIcon::Pointer,
        "slider should declare Pointer cursor"
    );
}
