use crate::action::WidgetAction;
use crate::geometry::Rect;
use crate::layout::compute_layout;
use crate::widget_id::WidgetId;
use crate::widgets::form_layout::FormLayout;
use crate::widgets::tests::MockMeasurer;
use crate::widgets::{LayoutCtx, Widget};

use super::SettingsPanel;

/// Creates a panel in overlay mode with dummy footer IDs.
fn make_panel() -> (SettingsPanel, WidgetId, WidgetId, WidgetId) {
    let reset_id = WidgetId::next();
    let cancel_id = WidgetId::next();
    let save_id = WidgetId::next();
    let panel = SettingsPanel::new(Box::new(FormLayout::new()), (reset_id, cancel_id, save_id));
    (panel, reset_id, cancel_id, save_id)
}

#[test]
fn new_does_not_panic() {
    let (_panel, _, _, _) = make_panel();
}

#[test]
fn id_returns_valid_widget_id() {
    let (panel, _, _, _) = make_panel();
    assert_ne!(panel.id().raw(), 0);
}

#[test]
fn close_id_differs_from_panel_id() {
    let (panel, _, _, _) = make_panel();
    assert_ne!(panel.id(), panel.close_id());
}

#[test]
fn focusable_children_includes_close_button() {
    let (panel, _, _, _) = make_panel();
    let children = panel.focusable_children();
    assert!(
        children.contains(&panel.close_id()),
        "close button should be in focusable_children"
    );
}

#[test]
fn layout_has_fixed_width() {
    let (panel, _, _, _) = make_panel();
    let ctx = LayoutCtx {
        measurer: &MockMeasurer::STANDARD,
        theme: &super::super::tests::TEST_THEME,
    };
    let lb = panel.layout(&ctx);
    let viewport = Rect::new(0.0, 0.0, 1200.0, 800.0);
    let node = compute_layout(&lb, viewport);

    // Panel should be 860px wide (PANEL_WIDTH).
    assert_eq!(node.rect.width(), 860.0);
}

#[test]
fn layout_hugs_content_height() {
    let (panel, _, _, _) = make_panel();
    let ctx = LayoutCtx {
        measurer: &MockMeasurer::STANDARD,
        theme: &super::super::tests::TEST_THEME,
    };
    let lb = panel.layout(&ctx);
    let viewport = Rect::new(0.0, 0.0, 800.0, 600.0);
    let node = compute_layout(&lb, viewport);

    // Panel hugs its content height (header + body).
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
    let (panel, _, _, _) = make_panel();
    assert!(!panel.is_focusable());
}

#[test]
fn draws_without_panic() {
    use crate::draw::Scene;

    let (panel, _, _, _) = make_panel();
    let measurer = MockMeasurer::STANDARD;
    let mut scene = Scene::new();
    let bounds = Rect::new(0.0, 0.0, 860.0, 620.0);
    let mut ctx = super::super::DrawCtx {
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
    panel.paint(&mut ctx);

    // Should produce at least a background rect.
    assert!(scene.len() >= 2, "panel should produce draw primitives");
}

// -- Regression: on_action maps Clicked(save/cancel/close) → semantic actions --

#[test]
fn on_action_maps_save_to_save_settings() {
    let (mut panel, _, _, save_id) = make_panel();
    let bounds = Rect::new(0.0, 0.0, 600.0, 600.0);
    let result = panel.on_action(WidgetAction::Clicked(save_id), bounds);
    assert_eq!(result, Some(WidgetAction::SaveSettings));
}

#[test]
fn on_action_maps_cancel_to_cancel_settings() {
    let (mut panel, _, cancel_id, _) = make_panel();
    let bounds = Rect::new(0.0, 0.0, 600.0, 600.0);
    let result = panel.on_action(WidgetAction::Clicked(cancel_id), bounds);
    assert_eq!(result, Some(WidgetAction::CancelSettings));
}

#[test]
fn on_action_maps_close_to_cancel_settings() {
    let (mut panel, _, _, _) = make_panel();
    let close_id = panel.close_id();
    let bounds = Rect::new(0.0, 0.0, 600.0, 600.0);
    let result = panel.on_action(WidgetAction::Clicked(close_id), bounds);
    assert_eq!(result, Some(WidgetAction::CancelSettings));
}

#[test]
fn on_action_maps_reset_to_reset_defaults() {
    let (mut panel, reset_id, _, _) = make_panel();
    let bounds = Rect::new(0.0, 0.0, 600.0, 600.0);
    let result = panel.on_action(WidgetAction::Clicked(reset_id), bounds);
    assert_eq!(result, Some(WidgetAction::ResetDefaults));
}

#[test]
fn on_action_passes_through_other_actions() {
    let (mut panel, _, _, _) = make_panel();
    let other_id = WidgetId::next();
    let bounds = Rect::new(0.0, 0.0, 600.0, 600.0);
    let action = WidgetAction::Clicked(other_id);
    let result = panel.on_action(action.clone(), bounds);
    assert_eq!(result, Some(action));
}

#[test]
fn for_each_child_mut_yields_container_not_buttons() {
    let (mut panel, _, _, _) = make_panel();
    let mut child_count = 0;
    panel.for_each_child_mut(&mut |_| {
        child_count += 1;
    });
    // SettingsPanel yields exactly one child: its container.
    assert_eq!(
        child_count, 1,
        "SettingsPanel should yield exactly its container"
    );
}
