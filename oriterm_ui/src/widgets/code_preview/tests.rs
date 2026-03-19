use crate::draw::Scene;
use crate::geometry::Rect;
use crate::layout::compute_layout;
use crate::sense::Sense;
use crate::widgets::tests::MockMeasurer;
use crate::widgets::{DrawCtx, LayoutCtx, Widget};

use super::CodePreviewWidget;

#[test]
fn sense_is_none() {
    let w = CodePreviewWidget::new();
    assert_eq!(w.sense(), Sense::none());
}

#[test]
fn layout_has_positive_dimensions() {
    let w = CodePreviewWidget::new();
    let ctx = LayoutCtx {
        measurer: &MockMeasurer::STANDARD,
        theme: &super::super::tests::TEST_THEME,
    };
    let lb = w.layout(&ctx);
    let node = compute_layout(&lb, Rect::new(0.0, 0.0, 400.0, 300.0));
    assert!(node.rect.width() > 0.0);
    assert!(node.rect.height() > 0.0);
}

#[test]
fn paint_produces_rect_and_text() {
    let w = CodePreviewWidget::new();
    let measurer = MockMeasurer::STANDARD;
    let mut scene = Scene::new();
    let bounds = Rect::new(0.0, 0.0, 280.0, 120.0);
    let mut ctx = DrawCtx {
        measurer: &measurer,
        scene: &mut scene,
        bounds,
        now: std::time::Instant::now(),
        theme: &super::super::tests::TEST_THEME,
        icons: None,
        interaction: None,
        widget_id: None,
        frame_requests: None,
    };
    w.paint(&mut ctx);

    // 1 background rect.
    assert_eq!(scene.quads().len(), 1);
    // "PREVIEW" label + multiple code spans.
    let texts = scene.text_runs().len();
    assert!(texts > 5, "should have label + code spans, got {texts}");
}

#[test]
fn not_focusable() {
    let w = CodePreviewWidget::new();
    assert!(!w.is_focusable());
}

#[test]
fn default_impl() {
    let w = CodePreviewWidget::default();
    assert_eq!(w.sense(), Sense::none());
}
