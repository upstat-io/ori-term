use crate::geometry::Rect;
use crate::layout::compute_layout;
use crate::widgets::form_layout::FormLayout;
use crate::widgets::tests::MockMeasurer;
use crate::widgets::{LayoutCtx, Widget};

use super::SettingsPanel;

fn make_panel() -> SettingsPanel {
    SettingsPanel::new(FormLayout::new())
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
fn draws_without_panic() {
    use crate::draw::DrawList;

    let panel = make_panel();
    let measurer = MockMeasurer::STANDARD;
    let mut draw_list = DrawList::new();
    let bounds = Rect::new(0.0, 0.0, 600.0, 600.0);
    let mut ctx = super::super::DrawCtx {
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
    panel.paint(&mut ctx);

    // Should produce at least the PushLayer + background rect.
    assert!(
        draw_list.commands().len() >= 2,
        "panel should produce draw commands"
    );
}
