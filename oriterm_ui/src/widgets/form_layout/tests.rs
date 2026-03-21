use crate::draw::Scene;
use crate::geometry::Rect;
use crate::layout::{SizeSpec, compute_layout};
use crate::widgets::button::ButtonWidget;
use crate::widgets::form_row::FormRow;
use crate::widgets::form_section::FormSection;
use crate::widgets::label::LabelWidget;
use crate::widgets::tests::MockMeasurer;
use crate::widgets::{DrawCtx, LayoutCtx, Widget};

use super::FormLayout;

fn test_form() -> FormLayout {
    FormLayout::new()
        .with_section(
            FormSection::new("Appearance")
                .with_row(FormRow::new(
                    "Color Scheme",
                    Box::new(LabelWidget::new("Dark")),
                ))
                .with_row(FormRow::new("Font Size", Box::new(LabelWidget::new("12")))),
        )
        .with_section(FormSection::new("Terminal").with_row(FormRow::new(
            "Scrollback",
            Box::new(LabelWidget::new("10000")),
        )))
}

#[test]
fn layout_stacks_sections_vertically() {
    let form = test_form();
    let ctx = LayoutCtx {
        measurer: &MockMeasurer::STANDARD,
        theme: &super::super::tests::TEST_THEME,
    };
    let mut layout_box = form.layout(&ctx);
    layout_box.width = SizeSpec::Fill;
    let viewport = Rect::new(0.0, 0.0, 400.0, 600.0);
    let node = compute_layout(&layout_box, viewport);

    assert_eq!(node.children.len(), 2);
    // Second section should be below the first.
    assert!(node.children[1].rect.y() > node.children[0].rect.y());
}

#[test]
fn compute_label_widths_aligns_all_rows() {
    let mut form = test_form();
    let measurer = MockMeasurer::STANDARD;
    let theme = super::super::tests::TEST_THEME;
    form.compute_label_widths(&measurer, &theme);

    // "Color Scheme" = 12 chars × 8px = 96px (widest).
    // With LABEL_PADDING (12.0): 108.0.
    let expected = 96.0 + 12.0;

    // All rows should have the same label width.
    for section in form.sections() {
        for row in section.rows() {
            // Access internal label_width via layout — verify the label box is fixed to expected.
            let ctx = LayoutCtx {
                measurer: &measurer,
                theme: &theme,
            };
            let layout_box = row.layout(&ctx);
            let viewport = Rect::new(0.0, 0.0, 400.0, 50.0);
            let node = compute_layout(&layout_box, viewport);
            let label_width = node.children[0].rect.width();
            assert_eq!(label_width, expected, "label width mismatch");
        }
    }
}

#[test]
fn not_focusable() {
    let form = test_form();
    assert!(!form.is_focusable());
}

#[test]
fn focusable_children_collects_from_sections() {
    let btn = ButtonWidget::new("Save");
    let btn_id = btn.id();
    let form = FormLayout::new()
        .with_section(FormSection::new("Actions").with_row(FormRow::new("Save", Box::new(btn))));
    let ids = form.focusable_children();
    assert_eq!(ids, vec![btn_id]);
}

#[test]
fn collapsed_section_excluded_from_focusable() {
    let btn = ButtonWidget::new("Save");
    let form = FormLayout::new().with_section(
        FormSection::new("Actions")
            .with_row(FormRow::new("Save", Box::new(btn)))
            .expanded(false),
    );
    assert!(form.focusable_children().is_empty());
}

#[test]
fn draw_produces_commands() {
    let form = test_form();
    let measurer = MockMeasurer::STANDARD;
    let mut scene = Scene::new();
    let bounds = Rect::new(0.0, 0.0, 400.0, 600.0);
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
    form.paint(&mut ctx);

    let text_cmds = scene.text_runs().len();
    // At minimum: 2 section headers (indicator + title each) + row labels/controls.
    assert!(
        text_cmds >= 4,
        "expected text commands from form, got {text_cmds}"
    );
}

#[test]
fn draw_skips_sections_outside_active_clip() {
    let form = test_form();
    let measurer = MockMeasurer::STANDARD;
    let mut scene = Scene::new();
    scene.push_clip(Rect::new(0.0, 0.0, 400.0, 110.0));
    let bounds = Rect::new(0.0, 0.0, 400.0, 600.0);
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
    form.paint(&mut ctx);

    let text_cmds = scene.text_runs().len();
    assert_eq!(text_cmds, 6, "only the first visible section should draw");
}

/// Creates a three-section form for viewport culling tests. Each section
/// has two rows, so the form is tall enough that sections 2 and 3 can be
/// scrolled off-screen when the clip rect is narrow.
fn tall_form() -> FormLayout {
    FormLayout::new()
        .with_section(
            FormSection::new("Section One")
                .with_row(FormRow::new("A1", Box::new(LabelWidget::new("val"))))
                .with_row(FormRow::new("A2", Box::new(LabelWidget::new("val")))),
        )
        .with_section(
            FormSection::new("Section Two")
                .with_row(FormRow::new("B1", Box::new(LabelWidget::new("val"))))
                .with_row(FormRow::new("B2", Box::new(LabelWidget::new("val")))),
        )
        .with_section(
            FormSection::new("Section Three")
                .with_row(FormRow::new("C1", Box::new(LabelWidget::new("val"))))
                .with_row(FormRow::new("C2", Box::new(LabelWidget::new("val")))),
        )
}

#[test]
fn partially_visible_section_still_paints() {
    // Clip rect covers the first section fully and the second section
    // partially. The second section should still paint (partial visibility),
    // while the third section should be fully off-screen and not paint.
    let form = tall_form();
    let measurer = MockMeasurer::STANDARD;

    // First, compute layout to find section heights.
    let bounds = Rect::new(0.0, 0.0, 400.0, 600.0);
    let ctx = LayoutCtx {
        measurer: &measurer,
        theme: &super::super::tests::TEST_THEME,
    };
    let layout_box = form.layout(&ctx);
    let layout = compute_layout(&layout_box, bounds);
    let s1_bottom = layout.children[0].rect.bottom();
    let s2_top = layout.children[1].rect.y();

    // Clip covers first section fully + just 1px into the second section.
    let clip_h = s2_top + 1.0;
    let mut scene = Scene::new();
    scene.push_clip(Rect::new(0.0, 0.0, 400.0, clip_h));

    // Paint with full-height bounds (simulating scroll content area).
    let mut draw_ctx = DrawCtx {
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
    form.paint(&mut draw_ctx);

    // Count text runs. Section 1 paints fully (6 runs). Section 2 is
    // partially visible — its header paints but FormSection's row-level
    // culling may skip rows below the clip. Section 3 is fully off-screen.
    let text_cmds = scene.text_runs().len();
    // Section 1: 6 (header indicator + title + 2×(label + value))
    // Section 2: at least 2 (header indicator + title); rows may be culled
    // Section 3: 0 (fully off-screen)
    assert!(
        text_cmds > 6 && text_cmds <= 12,
        "section 2 should partially paint (got {text_cmds})"
    );
    // Verify the clip height is between the first and second section boundaries.
    assert!(
        clip_h > s1_bottom && clip_h < layout.children[2].rect.y(),
        "clip should partially cover section 2 but not reach section 3"
    );
}

#[test]
fn scroll_offset_culls_top_sections() {
    // Simulate a scroll offset that pushes the first section above the
    // viewport. The clip is in viewport space; the offset shifts content
    // upward. current_clip_in_content_space() should convert correctly.
    let form = tall_form();
    let measurer = MockMeasurer::STANDARD;

    let bounds = Rect::new(0.0, 0.0, 400.0, 600.0);
    let ctx = LayoutCtx {
        measurer: &measurer,
        theme: &super::super::tests::TEST_THEME,
    };
    let layout_box = form.layout(&ctx);
    let layout = compute_layout(&layout_box, bounds);
    let s2_top = layout.children[1].rect.y();

    // Scroll offset that hides the first section: scroll past section 1.
    let scroll_offset = s2_top + 1.0;

    // Viewport clip at (0,0) with height = total form height - scroll_offset.
    let clip_h = 200.0;
    let mut scene = Scene::new();
    scene.push_clip(Rect::new(0.0, 0.0, 400.0, clip_h));
    scene.push_offset(0.0, -scroll_offset);

    let mut draw_ctx = DrawCtx {
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
    form.paint(&mut draw_ctx);

    scene.pop_offset();
    scene.pop_clip();

    // Section 1 is above the scroll offset — should be culled.
    // Section 2 and 3 should be visible.
    let text_cmds = scene.text_runs().len();
    assert_eq!(
        text_cmds, 12,
        "sections 2 and 3 should paint, section 1 culled by scroll"
    );
}

#[test]
fn default_creates_empty_form() {
    let form = FormLayout::default();
    assert_eq!(form.sections().len(), 0);
}
