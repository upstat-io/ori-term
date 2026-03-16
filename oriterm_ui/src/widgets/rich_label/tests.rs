use crate::color::Color;
use crate::text::TextStyle;
use crate::widgets::tests::MockMeasurer;
use crate::widgets::{LayoutCtx, Widget};

use super::{RichLabel, TextSpan};

fn test_theme() -> &'static crate::theme::UiTheme {
    &super::super::tests::TEST_THEME
}

#[test]
fn two_spans_correct_layout_width() {
    let label = RichLabel::new(vec![
        TextSpan {
            text: "Hello".into(),
            style: TextStyle::new(12.0, Color::WHITE),
        },
        TextSpan {
            text: " World".into(),
            style: TextStyle::new(12.0, Color::hex(0xFF_00_00)),
        },
    ]);

    let m = MockMeasurer::new();
    let ctx = LayoutCtx {
        measurer: &m,
        theme: test_theme(),
    };
    let layout = label.layout(&ctx);

    // "Hello" = 5 chars * 8px = 40, " World" = 6 chars * 8px = 48.
    // Total = 88px.
    if let crate::layout::BoxContent::Leaf {
        intrinsic_width,
        intrinsic_height,
    } = &layout.content
    {
        assert_eq!(*intrinsic_width, 88.0);
        assert_eq!(*intrinsic_height, 16.0);
    } else {
        panic!("expected leaf layout");
    }
}

#[test]
fn empty_spans_zero_size() {
    let label = RichLabel::new(vec![]);

    let m = MockMeasurer::new();
    let ctx = LayoutCtx {
        measurer: &m,
        theme: test_theme(),
    };
    let layout = label.layout(&ctx);

    if let crate::layout::BoxContent::Leaf {
        intrinsic_width,
        intrinsic_height,
    } = &layout.content
    {
        assert_eq!(*intrinsic_width, 0.0);
        assert_eq!(*intrinsic_height, 0.0);
    } else {
        panic!("expected leaf layout");
    }
}

#[test]
fn single_span_matches_label_dimensions() {
    let text = "test string";
    let style = TextStyle::new(13.0, Color::WHITE);

    // RichLabel with one span.
    let rich = RichLabel::new(vec![TextSpan {
        text: text.into(),
        style: style.clone(),
    }]);

    // Plain Label for comparison.
    let label = crate::widgets::label::LabelWidget::new(text).with_style(
        crate::widgets::label::LabelStyle {
            color: style.color,
            font_size: style.size,
            ..crate::widgets::label::LabelStyle::default()
        },
    );

    let m = MockMeasurer::new();
    let ctx = LayoutCtx {
        measurer: &m,
        theme: test_theme(),
    };

    let rich_layout = rich.layout(&ctx);
    let label_layout = label.layout(&ctx);

    if let (
        crate::layout::BoxContent::Leaf {
            intrinsic_width: rw,
            intrinsic_height: rh,
        },
        crate::layout::BoxContent::Leaf {
            intrinsic_width: lw,
            intrinsic_height: lh,
        },
    ) = (&rich_layout.content, &label_layout.content)
    {
        assert_eq!(*rw, *lw);
        assert_eq!(*rh, *lh);
    } else {
        panic!("expected leaf layouts");
    }
}

#[test]
fn sense_is_none() {
    let label = RichLabel::new(vec![]);
    assert_eq!(label.sense(), crate::sense::Sense::none());
}

#[test]
fn not_focusable() {
    let label = RichLabel::new(vec![]);
    assert!(!label.is_focusable());
}

#[test]
fn has_widget_id() {
    let label = RichLabel::new(vec![]);
    let m = MockMeasurer::new();
    let ctx = LayoutCtx {
        measurer: &m,
        theme: test_theme(),
    };
    let layout = label.layout(&ctx);
    assert_eq!(layout.widget_id, Some(label.id()));
}
