use crate::text::{TextOverflow, TextTransform};
use crate::widgets::tests::MockMeasurer;
use crate::widgets::{LayoutCtx, Widget};

use super::{LabelStyle, LabelWidget};

#[test]
fn default_style() {
    let label = LabelWidget::new("hello");
    assert_eq!(label.text(), "hello");
    assert!(!label.is_focusable());
}

#[test]
fn layout_uses_measurer() {
    let label = LabelWidget::new("test");
    let m = MockMeasurer::new();
    let ctx = LayoutCtx {
        measurer: &m,
        theme: &super::super::tests::TEST_THEME,
    };
    let layout = label.layout(&ctx);

    // "test" = 4 chars * 8px = 32px wide, 16px tall.
    if let crate::layout::BoxContent::Leaf {
        intrinsic_width,
        intrinsic_height,
    } = &layout.content
    {
        assert_eq!(*intrinsic_width, 32.0);
        assert_eq!(*intrinsic_height, 16.0);
    } else {
        panic!("expected leaf layout");
    }
}

#[test]
fn layout_has_widget_id() {
    let label = LabelWidget::new("x");
    let m = MockMeasurer::new();
    let ctx = LayoutCtx {
        measurer: &m,
        theme: &super::super::tests::TEST_THEME,
    };
    let layout = label.layout(&ctx);
    assert_eq!(layout.widget_id, Some(label.id()));
}

#[test]
fn set_text_updates() {
    let mut label = LabelWidget::new("before");
    label.set_text("after");
    assert_eq!(label.text(), "after");
}

#[test]
fn with_style_applies() {
    let style = LabelStyle {
        font_size: 20.0,
        overflow: TextOverflow::Ellipsis,
        line_height: None,
        ..LabelStyle::default()
    };
    let label = LabelWidget::new("styled").with_style(style.clone());
    assert_eq!(label.style.font_size, 20.0);
    assert_eq!(label.style.overflow, TextOverflow::Ellipsis);
}

#[test]
fn label_style_text_transform_forwarded_to_layout() {
    let style = LabelStyle {
        text_transform: TextTransform::Uppercase,
        line_height: None,
        ..LabelStyle::default()
    };
    let label = LabelWidget::new("hello").with_style(style);
    let m = MockMeasurer::new();
    let ctx = LayoutCtx {
        measurer: &m,
        theme: &super::super::tests::TEST_THEME,
    };
    let layout = label.layout(&ctx);
    // MockMeasurer applies text_transform: "hello" -> "HELLO" = 5 chars * 8px = 40px.
    if let crate::layout::BoxContent::Leaf {
        intrinsic_width, ..
    } = &layout.content
    {
        assert_eq!(
            *intrinsic_width, 40.0,
            "uppercase transform should be applied"
        );
    } else {
        panic!("expected leaf layout");
    }
}

#[test]
fn label_style_line_height_forwarded_to_layout() {
    let style = LabelStyle {
        font_size: 12.0,
        line_height: Some(1.5),
        ..LabelStyle::default()
    };
    let label = LabelWidget::new("Hello").with_style(style);
    let m = MockMeasurer::new();
    let ctx = LayoutCtx {
        measurer: &m,
        theme: &super::super::tests::TEST_THEME,
    };
    let layout = label.layout(&ctx);
    // MockMeasurer with line_height 1.5 and size 12.0: height = 12.0 * 1.5 = 18.0.
    if let crate::layout::BoxContent::Leaf {
        intrinsic_height, ..
    } = &layout.content
    {
        assert_eq!(
            *intrinsic_height, 18.0,
            "line_height should override: 12.0 * 1.5 = 18.0, not 16.0",
        );
    } else {
        panic!("expected leaf layout");
    }
}

#[test]
fn empty_text_layout() {
    let label = LabelWidget::new("");
    let m = MockMeasurer::new();
    let ctx = LayoutCtx {
        measurer: &m,
        theme: &super::super::tests::TEST_THEME,
    };
    let layout = label.layout(&ctx);
    if let crate::layout::BoxContent::Leaf {
        intrinsic_width, ..
    } = &layout.content
    {
        assert_eq!(*intrinsic_width, 0.0);
    } else {
        panic!("expected leaf layout");
    }
}
