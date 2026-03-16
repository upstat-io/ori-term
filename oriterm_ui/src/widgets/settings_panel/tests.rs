use crate::geometry::{Point, Rect};
use crate::input::{Modifiers, MouseButton, MouseEvent, MouseEventKind};
use crate::layout::compute_layout;
use crate::widgets::form_layout::FormLayout;
use crate::widgets::tests::MockMeasurer;
use crate::widgets::{EventCtx, LayoutCtx, Widget, WidgetAction};

use super::SettingsPanel;

fn make_panel() -> SettingsPanel {
    SettingsPanel::new(FormLayout::new())
}

/// Creates a panel with a checkbox for click testing. Returns (panel, checkbox_id).
fn make_panel_with_checkbox() -> (SettingsPanel, crate::widget_id::WidgetId) {
    use crate::widgets::checkbox::CheckboxWidget;
    use crate::widgets::form_row::FormRow;
    use crate::widgets::form_section::FormSection;

    let checkbox = CheckboxWidget::new("Test toggle");
    let checkbox_id = checkbox.id();

    let mut form = FormLayout::new().with_section(
        FormSection::new("General").with_row(FormRow::new("My option", Box::new(checkbox))),
    );
    form.compute_label_widths(&MockMeasurer::STANDARD, &super::super::tests::TEST_THEME);

    (SettingsPanel::new(form), checkbox_id)
}

#[test]
fn new_does_not_panic() {
    let _panel = make_panel();
}

#[test]
fn id_returns_valid_widget_id() {
    let panel = make_panel();
    // IDs are monotonically increasing — just verify it's nonzero.
    assert_ne!(panel.id().raw(), 0);
}

#[test]
fn close_id_differs_from_panel_id() {
    let panel = make_panel();
    assert_ne!(panel.id(), panel.close_id());
}

#[test]
fn focusable_children_includes_close_button() {
    let panel = make_panel();
    let children = panel.focusable_children();
    assert!(
        children.contains(&panel.close_id()),
        "close button should be in focusable_children"
    );
}

#[test]
fn layout_has_fixed_width() {
    let panel = make_panel();
    let ctx = LayoutCtx {
        measurer: &MockMeasurer::STANDARD,
        theme: &super::super::tests::TEST_THEME,
    };
    let lb = panel.layout(&ctx);
    let viewport = Rect::new(0.0, 0.0, 800.0, 600.0);
    let node = compute_layout(&lb, viewport);

    // Panel should be 600px wide (PANEL_WIDTH).
    assert_eq!(node.rect.width(), 600.0);
}

#[test]
fn layout_hugs_content_height() {
    let panel = make_panel();
    let ctx = LayoutCtx {
        measurer: &MockMeasurer::STANDARD,
        theme: &super::super::tests::TEST_THEME,
    };
    let lb = panel.layout(&ctx);
    let viewport = Rect::new(0.0, 0.0, 800.0, 600.0);
    let node = compute_layout(&lb, viewport);

    // Panel hugs its content height (header + separator + form body).
    // With an empty form, height is at least the header bar (48px).
    assert!(
        node.rect.height() >= 48.0,
        "panel should include header height"
    );
    assert!(
        node.rect.height() <= 600.0,
        "panel should not exceed viewport"
    );
}

#[test]
fn not_focusable() {
    let panel = make_panel();
    assert!(!panel.is_focusable());
}

#[test]
fn close_button_click_emits_dismiss() {
    let mut panel = make_panel();
    let measurer = MockMeasurer::STANDARD;
    let bounds = Rect::new(0.0, 0.0, 600.0, 600.0);
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

    // The close button is in the top-right of the header. Click at ~(575, 24)
    // which is well within the header row area near the right side.
    let click_pos = Point::new(575.0, 24.0);

    let down = MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        pos: click_pos,
        modifiers: Modifiers::NONE,
    };
    let _ = panel.handle_mouse(&down, &ctx);

    let up = MouseEvent {
        kind: MouseEventKind::Up(MouseButton::Left),
        pos: click_pos,
        modifiers: Modifiers::NONE,
    };
    let resp = panel.handle_mouse(&up, &ctx);

    assert_eq!(
        resp.action,
        Some(WidgetAction::CancelSettings),
        "close button should emit CancelSettings"
    );
}

#[test]
fn close_button_key_emits_dismiss() {
    use crate::input::{Key, KeyEvent};

    let mut panel = make_panel();
    let close_id = panel.close_id();
    let measurer = MockMeasurer::STANDARD;
    let bounds = Rect::new(0.0, 0.0, 600.0, 600.0);
    let ctx = EventCtx {
        measurer: &measurer,
        bounds,
        is_focused: false,
        focused_widget: Some(close_id),
        theme: &super::super::tests::TEST_THEME,
        interaction: None,
        widget_id: None,
        frame_requests: None,
    };

    let event = KeyEvent {
        key: Key::Enter,
        modifiers: Modifiers::NONE,
    };
    let resp = panel.handle_key(event, &ctx);

    assert_eq!(
        resp.action,
        Some(WidgetAction::CancelSettings),
        "close button Enter key should emit CancelSettings"
    );
}

#[test]
fn draws_without_panic() {
    use crate::draw::DrawList;

    let panel = make_panel();
    let measurer = MockMeasurer::STANDARD;
    let mut draw_list = DrawList::new();
    let bounds = Rect::new(0.0, 0.0, 600.0, 600.0);
    let anim_flag = std::cell::Cell::new(false);
    let mut ctx = super::super::DrawCtx {
        measurer: &measurer,
        draw_list: &mut draw_list,
        bounds,
        focused_widget: None,
        now: std::time::Instant::now(),
        animations_running: &anim_flag,
        theme: &super::super::tests::TEST_THEME,
        icons: None,
        scene_cache: None,
        interaction: None,
        widget_id: None,
        frame_requests: None,
    };
    panel.paint(&mut ctx);

    // Should produce at least the PushLayer + background rect.
    assert!(
        draw_list.commands().len() >= 2,
        "panel should produce draw commands"
    );
}

#[test]
fn checkbox_click_emits_toggled() {
    use crate::draw::DrawList;

    let (mut panel, checkbox_id) = make_panel_with_checkbox();
    let measurer = MockMeasurer::STANDARD;
    let bounds = Rect::new(0.0, 0.0, 600.0, 600.0);

    // Draw first to populate layout caches throughout the widget tree.
    let mut draw_list = DrawList::new();
    let anim_flag = std::cell::Cell::new(false);
    let mut draw_ctx = super::super::DrawCtx {
        measurer: &measurer,
        draw_list: &mut draw_list,
        bounds,
        focused_widget: None,
        now: std::time::Instant::now(),
        animations_running: &anim_flag,
        theme: &super::super::tests::TEST_THEME,
        icons: None,
        scene_cache: None,
        interaction: None,
        widget_id: None,
        frame_requests: None,
    };
    panel.paint(&mut draw_ctx);

    // Click in the checkbox area: past header (48) + separator (~2) +
    // form padding top (16) + section header (28) + row gap (12) = 106,
    // so y=112 is in the first row. x=150 is within the checkbox control
    // (label column ~84px + padding 24px = 108, checkbox extends ~112px).
    let click_pos = Point::new(150.0, 112.0);

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

    let down = MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        pos: click_pos,
        modifiers: Modifiers::NONE,
    };
    let _ = panel.handle_mouse(&down, &ctx);

    let up = MouseEvent {
        kind: MouseEventKind::Up(MouseButton::Left),
        pos: click_pos,
        modifiers: Modifiers::NONE,
    };
    let resp = panel.handle_mouse(&up, &ctx);

    match resp.action {
        Some(WidgetAction::Toggled { id, value }) => {
            assert_eq!(id, checkbox_id, "toggled ID should match checkbox");
            assert!(value, "checkbox should toggle to true");
        }
        other => panic!("expected Toggled action, got {other:?}"),
    }
}
