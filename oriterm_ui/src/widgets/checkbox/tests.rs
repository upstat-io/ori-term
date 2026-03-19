use crate::layout::BoxContent;
use crate::widgets::tests::MockMeasurer;
use crate::widgets::{LayoutCtx, Widget};

use super::{CheckboxStyle, CheckboxWidget};

#[test]
fn default_state() {
    let cb = CheckboxWidget::new("Accept");
    assert!(!cb.is_checked());
    assert!(!cb.is_disabled());
    assert!(cb.is_focusable());
}

#[test]
fn with_checked_builder() {
    let cb = CheckboxWidget::new("X").with_checked(true);
    assert!(cb.is_checked());
}

#[test]
fn layout_dimensions() {
    let cb = CheckboxWidget::new("Check");
    let m = MockMeasurer::new();
    let ctx = LayoutCtx {
        measurer: &m,
        theme: &super::super::tests::TEST_THEME,
    };
    let layout = cb.layout(&ctx);
    let s = CheckboxStyle::default();

    if let BoxContent::Leaf {
        intrinsic_width,
        intrinsic_height,
    } = &layout.content
    {
        // "Check" = 5 * 8 = 40, box = 16, gap = 8 → 64.
        assert_eq!(*intrinsic_width, s.box_size + s.gap + 40.0);
        // max(box_size=16, line_height=16) = 16.
        assert_eq!(*intrinsic_height, 16.0);
    } else {
        panic!("expected leaf layout");
    }
}

#[test]
fn set_checked_programmatic() {
    let mut cb = CheckboxWidget::new("X");
    cb.set_checked(true);
    assert!(cb.is_checked());
    cb.set_checked(false);
    assert!(!cb.is_checked());
}

#[test]
fn rapid_toggle_sequence() {
    let mut cb = CheckboxWidget::new("X");

    // Toggle 4 times directly — verifies state consistency.
    for i in 0..4 {
        cb.toggle();
        assert_eq!(cb.is_checked(), i % 2 == 0);
    }
}

#[test]
fn set_disabled_affects_focusable() {
    let mut cb = CheckboxWidget::new("X");
    assert!(cb.is_focusable());
    cb.set_disabled(true);
    assert!(!cb.is_focusable());
}

#[test]
fn sense_returns_click() {
    let cb = CheckboxWidget::new("X");
    assert_eq!(cb.sense(), crate::sense::Sense::click());
}

#[test]
fn has_two_controllers() {
    let cb = CheckboxWidget::new("X");
    assert_eq!(cb.controllers().len(), 2);
}

#[test]
fn has_visual_state_animator() {
    let cb = CheckboxWidget::new("X");
    assert!(cb.visual_states().is_some());
}
