//! Unit tests for the terminal preview widget scaffold.

use oriterm_ui::layout::SizeSpec;
use oriterm_ui::text::{ShapedText, TextMetrics, TextStyle};
use oriterm_ui::theme::UiTheme;
use oriterm_ui::widgets::text_measurer::TextMeasurer;
use oriterm_ui::widgets::{LayoutCtx, Widget};

use super::TerminalPreviewWidget;

/// Minimal text measurer for tests.
struct TestMeasurer;

impl TextMeasurer for TestMeasurer {
    fn measure(&self, text: &str, _style: &TextStyle, _max_width: f32) -> TextMetrics {
        TextMetrics {
            width: text.len() as f32 * 8.0,
            height: 16.0,
            line_count: 1,
        }
    }

    fn shape(&self, _text: &str, _style: &TextStyle, _max_width: f32) -> ShapedText {
        ShapedText {
            glyphs: Vec::new(),
            width: 0.0,
            height: 16.0,
            baseline: 12.0,
            size_q6: 0,
            weight: 400,
        }
    }
}

fn make_layout_ctx() -> LayoutCtx<'static> {
    static MEASURER: TestMeasurer = TestMeasurer;
    static THEME: std::sync::LazyLock<UiTheme> = std::sync::LazyLock::new(UiTheme::dark);
    LayoutCtx {
        measurer: &MEASURER,
        theme: &THEME,
    }
}

#[test]
fn layout_returns_fixed_size() {
    let widget = TerminalPreviewWidget::new();
    let ctx = make_layout_ctx();
    let layout = widget.layout(&ctx);

    // Default is Hug (not Fill) — fixed intrinsic dimensions.
    assert_eq!(layout.width, SizeSpec::Hug);
    assert_eq!(layout.height, SizeSpec::Hug);
}

#[test]
fn layout_has_default_dimensions() {
    let widget = TerminalPreviewWidget::new();
    let ctx = make_layout_ctx();
    let layout = widget.layout(&ctx);

    if let oriterm_ui::layout::BoxContent::Leaf {
        intrinsic_width,
        intrinsic_height,
    } = layout.content
    {
        assert_eq!(intrinsic_width, 320.0);
        assert_eq!(intrinsic_height, 200.0);
    } else {
        panic!("expected Leaf content");
    }
}

#[test]
fn is_not_focusable() {
    let widget = TerminalPreviewWidget::new();
    assert!(!widget.is_focusable());
}

#[test]
fn layout_has_widget_id() {
    let widget = TerminalPreviewWidget::new();
    let ctx = make_layout_ctx();
    let layout = widget.layout(&ctx);

    assert_eq!(layout.widget_id, Some(widget.id()));
}

#[test]
fn custom_size_widget() {
    let widget = TerminalPreviewWidget::with_size(400.0, 250.0, 0.5);
    let ctx = make_layout_ctx();
    let layout = widget.layout(&ctx);

    if let oriterm_ui::layout::BoxContent::Leaf {
        intrinsic_width,
        intrinsic_height,
    } = layout.content
    {
        assert_eq!(intrinsic_width, 400.0);
        assert_eq!(intrinsic_height, 250.0);
    } else {
        panic!("expected Leaf content");
    }
}
