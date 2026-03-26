//! Tests for shared content typography helpers.

use std::time::Instant;

use oriterm_ui::draw::Scene;
use oriterm_ui::geometry::Rect;
use oriterm_ui::layout::compute_layout;
use oriterm_ui::testing::MockMeasurer;
use oriterm_ui::theme::UiTheme;
use oriterm_ui::widgets::label::LabelWidget;
use oriterm_ui::widgets::{DrawCtx, LayoutCtx, Widget};

use super::{
    build_page_header, build_section_header, build_section_header_with_description,
    build_settings_page,
};

const TEST_THEME: UiTheme = UiTheme::dark();
fn viewport() -> Rect {
    Rect::new(0.0, 0.0, 600.0, 400.0)
}

fn layout_widget(widget: &dyn Widget) -> oriterm_ui::layout::LayoutNode {
    let ctx = LayoutCtx {
        measurer: &MockMeasurer::STANDARD,
        theme: &TEST_THEME,
    };
    let lb = widget.layout(&ctx);
    compute_layout(&lb, viewport())
}

fn paint_widget(widget: &dyn Widget) -> Scene {
    let measurer = MockMeasurer::STANDARD;
    let mut scene = Scene::new();
    let mut ctx = DrawCtx {
        measurer: &measurer,
        scene: &mut scene,
        bounds: viewport(),
        now: Instant::now(),
        theme: &TEST_THEME,
        icons: None,
        interaction: None,
        widget_id: None,
        frame_requests: None,
    };
    widget.paint(&mut ctx);
    scene
}

// -- Page header tests --

#[test]
fn page_header_applies_uppercase() {
    let header = build_page_header("test", "desc", &TEST_THEME);
    let scene = paint_widget(&*header);

    // MockMeasurer applies TextTransform::Uppercase, so "test" → "TEST" = 4 glyphs.
    let title_run = &scene.text_runs()[0];
    assert_eq!(
        title_run.shaped.glyphs.len(),
        4,
        "uppercase 'TEST' should have 4 glyphs"
    );
}

#[test]
fn page_header_weight_bold() {
    let header = build_page_header("Title", "Sub", &TEST_THEME);
    let scene = paint_widget(&*header);

    let title_run = &scene.text_runs()[0];
    assert_eq!(title_run.shaped.weight, 700, "title should be bold (700)");
}

#[test]
fn page_header_title_subtitle_spacing() {
    let header = build_page_header("Title", "Sub", &TEST_THEME);
    let node = layout_widget(&*header);

    // Column with gap=4: children[0]=title, children[1]=desc.
    let title = &node.children[0];
    let desc = &node.children[1];
    let gap = desc.rect.y() - title.rect.bottom();
    assert!(
        (gap - 4.0).abs() < 0.1,
        "title-subtitle gap should be 4px, got {gap}"
    );
}

#[test]
fn page_header_padding() {
    let header = build_page_header("Test", "Sub", &TEST_THEME);
    let node = layout_widget(&*header);

    // Padding: Insets::tlbr(24, 28, 20, 28).
    let left_pad = node.content_rect.x() - node.rect.x();
    let top_pad = node.content_rect.y() - node.rect.y();
    assert!(
        (left_pad - 28.0).abs() < 0.1,
        "left padding should be 28, got {left_pad}"
    );
    assert!(
        (top_pad - 24.0).abs() < 0.1,
        "top padding should be 24, got {top_pad}"
    );
}

// -- Section header tests --

#[test]
fn section_header_two_text_runs() {
    let header = build_section_header("Test", &TEST_THEME);
    let scene = paint_widget(&*header);

    // Prefix "//" and title "TEST" are separate labels.
    assert!(
        scene.text_runs().len() >= 2,
        "should have at least 2 text runs (prefix + title), got {}",
        scene.text_runs().len()
    );
}

#[test]
fn section_header_weight_medium() {
    let header = build_section_header("Test", &TEST_THEME);
    let scene = paint_widget(&*header);

    for (i, run) in scene.text_runs().iter().enumerate() {
        assert_eq!(
            run.shaped.weight, 500,
            "text run {i} should be medium (500), got {}",
            run.shaped.weight
        );
    }
}

#[test]
fn section_header_separator_present() {
    let header = build_section_header("Test", &TEST_THEME);
    let scene = paint_widget(&*header);

    assert!(
        !scene.lines().is_empty(),
        "should have at least one line (separator)"
    );
}

#[test]
fn section_header_bottom_spacing_12() {
    let header = build_section_header("Test", &TEST_THEME);
    let node = layout_widget(&*header);

    // Column: children[0] = title row, children[1] = 12px spacer.
    assert_eq!(node.children.len(), 2, "should have title row + spacer");
    let spacer = &node.children[1];
    assert!(
        (spacer.rect.height() - 12.0).abs() < 0.1,
        "spacer should be 12px, got {}",
        spacer.rect.height()
    );
}

// -- Description tests --

#[test]
fn section_description_gap_4_then_12() {
    let header = build_section_header_with_description("Test", "A description", &TEST_THEME);
    let node = layout_widget(&*header);

    // Column: title_row, spacer(4), desc_label, spacer(12).
    assert!(
        node.children.len() >= 4,
        "should have 4 children, got {}",
        node.children.len()
    );
    let spacer_4 = &node.children[1];
    let spacer_12 = &node.children[3];
    assert!(
        (spacer_4.rect.height() - 4.0).abs() < 0.1,
        "title-desc spacer should be 4px, got {}",
        spacer_4.rect.height()
    );
    assert!(
        (spacer_12.rect.height() - 12.0).abs() < 0.1,
        "desc-rows spacer should be 12px, got {}",
        spacer_12.rect.height()
    );
}

#[test]
fn section_description_weight() {
    let header = build_section_header_with_description("Test", "Desc text", &TEST_THEME);
    let scene = paint_widget(&*header);

    // Text runs: prefix "//" (500), title "TEST" (500), description "Desc text" (400).
    assert!(
        scene.text_runs().len() >= 3,
        "should have at least 3 text runs, got {}",
        scene.text_runs().len()
    );
    let desc_run = &scene.text_runs()[2];
    assert_eq!(
        desc_run.shaped.weight, 400,
        "description should be normal weight (400)"
    );
}

// -- Body spacing tests --

#[test]
fn body_spacing_section_gap_28() {
    let s1 = Box::new(LabelWidget::new("section one")) as Box<dyn Widget>;
    let s2 = Box::new(LabelWidget::new("section two")) as Box<dyn Widget>;
    let page = build_settings_page("T", "D", vec![s1, s2], &TEST_THEME);
    let node = layout_widget(&*page);

    // Root column: children[0] = header, children[1] = scroll.
    // Scroll child[0] = body column, body column children = sections.
    let scroll = &node.children[1];
    assert!(!scroll.children.is_empty(), "scroll should have body child");
    let body = &scroll.children[0];
    assert!(
        body.children.len() >= 2,
        "body should have 2 section children, got {}",
        body.children.len()
    );
    let sec1 = &body.children[0];
    let sec2 = &body.children[1];
    let gap = sec2.rect.y() - sec1.rect.bottom();
    assert!(
        (gap - 28.0).abs() < 0.5,
        "inter-section gap should be 28px, got {gap}"
    );
}

#[test]
fn body_spacing_bottom_padding_28() {
    let s1 = Box::new(LabelWidget::new("section one")) as Box<dyn Widget>;
    let page = build_settings_page("T", "D", vec![s1], &TEST_THEME);
    let node = layout_widget(&*page);

    // Root column → scroll → body column.
    let scroll = &node.children[1];
    let body = &scroll.children[0];
    let bottom_pad = body.rect.bottom() - body.content_rect.bottom();
    assert!(
        (bottom_pad - 28.0).abs() < 0.5,
        "body bottom padding should be 28px, got {bottom_pad}"
    );
}

#[test]
fn settings_page_still_builds() {
    let page = build_settings_page("Title", "Desc", vec![], &TEST_THEME);
    assert_ne!(page.id().raw(), 0, "settings page should have non-zero ID");
}
