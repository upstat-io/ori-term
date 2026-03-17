use crate::draw::DrawList;
use crate::geometry::{Point, Rect};
use crate::input::{Modifiers, MouseButton, MouseEvent, MouseEventKind};
use crate::layout::{SizeSpec, compute_layout};
use crate::widgets::button::ButtonWidget;
use crate::widgets::form_row::FormRow;
use crate::widgets::form_section::FormSection;
use crate::widgets::label::LabelWidget;
use crate::widgets::tests::MockMeasurer;
use crate::widgets::{DrawCtx, EventCtx, LayoutCtx, Widget, WidgetResponse};

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
fn mouse_delegates_to_section() {
    let mut form = test_form();
    let measurer = MockMeasurer::STANDARD;
    let bounds = Rect::new(0.0, 0.0, 400.0, 600.0);
    let ctx = EventCtx {
        measurer: &measurer,
        bounds,
        is_focused: false,
        focused_widget: None,
        theme: &super::super::tests::TEST_THEME,
        interaction: None,
        widget_id: None,
        frame_requests: None,
    };

    // Click on the first section header (past FORM_PADDING top=16,
    // within HEADER_HEIGHT=28, so y=26 is in the header area).
    let event = MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        pos: Point::new(50.0, 26.0),
        modifiers: Modifiers::NONE,
    };
    let resp = form.handle_mouse(&event, &ctx);
    // Should toggle the first section (layout response).
    assert_eq!(resp, WidgetResponse::layout());
    assert!(!form.sections()[0].is_expanded());
}

#[test]
fn draw_produces_commands() {
    let form = test_form();
    let measurer = MockMeasurer::STANDARD;
    let mut draw_list = DrawList::new();
    let bounds = Rect::new(0.0, 0.0, 400.0, 600.0);
    let mut ctx = DrawCtx {
        measurer: &measurer,
        draw_list: &mut draw_list,
        bounds,
        focused_widget: None,
        now: std::time::Instant::now(),
        theme: &super::super::tests::TEST_THEME,
        icons: None,
        scene_cache: None,
        interaction: None,
        widget_id: None,
        frame_requests: None,
    };
    form.paint(&mut ctx);

    let text_cmds = draw_list
        .commands()
        .iter()
        .filter(|c| matches!(c, crate::draw::DrawCommand::Text { .. }))
        .count();
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
    let mut draw_list = DrawList::new();
    draw_list.push_clip(Rect::new(0.0, 0.0, 400.0, 110.0));
    let bounds = Rect::new(0.0, 0.0, 400.0, 600.0);
    let mut ctx = DrawCtx {
        measurer: &measurer,
        draw_list: &mut draw_list,
        bounds,
        focused_widget: None,
        now: std::time::Instant::now(),
        theme: &super::super::tests::TEST_THEME,
        icons: None,
        scene_cache: None,
        interaction: None,
        widget_id: None,
        frame_requests: None,
    };
    form.paint(&mut ctx);

    let text_cmds = draw_list
        .commands()
        .iter()
        .filter(|c| matches!(c, crate::draw::DrawCommand::Text { .. }))
        .count();
    assert_eq!(text_cmds, 6, "only the first visible section should draw");
}

#[test]
fn default_creates_empty_form() {
    let form = FormLayout::default();
    assert_eq!(form.sections().len(), 0);
}
