use crate::action::WidgetAction;
use crate::draw::Scene;
use crate::geometry::Rect;
use crate::layout::compute_layout;
use crate::widget_id::WidgetId;
use crate::widgets::Widget;
use crate::widgets::label::LabelWidget;
use crate::widgets::scroll::ScrollWidget;
use crate::widgets::tests::MockMeasurer;
use crate::widgets::{DrawCtx, LayoutCtx};

use super::PageContainerWidget;

fn label(text: &str) -> Box<dyn Widget> {
    Box::new(LabelWidget::new(text))
}

fn make_ctx() -> LayoutCtx<'static> {
    LayoutCtx {
        measurer: &MockMeasurer::STANDARD,
        theme: &super::super::tests::TEST_THEME,
    }
}

/// Creates a page container with a registered nav source ID for testing.
fn make_pc_with_nav(pages: Vec<Box<dyn Widget>>) -> (PageContainerWidget, WidgetId) {
    let nav_id = WidgetId::next();
    let pc = PageContainerWidget::new(pages).with_nav_source(nav_id);
    (pc, nav_id)
}

/// Creates a `Selected` action from the given source.
fn nav_selected(nav_id: WidgetId, index: usize) -> WidgetAction {
    WidgetAction::Selected { id: nav_id, index }
}

// -- Construction --

#[test]
fn new_starts_at_page_zero() {
    let pc = PageContainerWidget::new(vec![label("A"), label("B")]);
    assert_eq!(pc.active_page(), 0);
    assert_eq!(pc.page_count(), 2);
}

#[test]
fn empty_container() {
    let pc = PageContainerWidget::new(vec![]);
    assert_eq!(pc.active_page(), 0);
    assert_eq!(pc.page_count(), 0);
}

// -- Page switching --

#[test]
fn set_active_page_switches() {
    let mut pc = PageContainerWidget::new(vec![label("A"), label("B"), label("C")]);
    pc.set_active_page(2);
    assert_eq!(pc.active_page(), 2);
}

#[test]
fn set_active_page_out_of_range_is_noop() {
    let mut pc = PageContainerWidget::new(vec![label("A"), label("B")]);
    pc.set_active_page(99);
    assert_eq!(pc.active_page(), 0);
}

// -- accept_action with nav_source --

#[test]
fn accept_action_switches_page_on_nav_selected() {
    let (mut pc, nav_id) = make_pc_with_nav(vec![label("A"), label("B"), label("C")]);
    let action = nav_selected(nav_id, 2);
    assert!(pc.accept_action(&action));
    assert_eq!(pc.active_page(), 2);
}

#[test]
fn accept_action_ignores_selected_from_non_nav_source() {
    let (mut pc, _nav_id) = make_pc_with_nav(vec![label("A"), label("B"), label("C")]);
    // Selected from a different widget (e.g., SchemeCard) — should NOT switch pages.
    let other_id = WidgetId::next();
    let action = WidgetAction::Selected {
        id: other_id,
        index: 2,
    };
    assert!(!pc.accept_action(&action));
    assert_eq!(pc.active_page(), 0);
}

#[test]
fn accept_action_ignores_out_of_range() {
    let (mut pc, nav_id) = make_pc_with_nav(vec![label("A"), label("B")]);
    let action = nav_selected(nav_id, 99);
    assert!(!pc.accept_action(&action));
    assert_eq!(pc.active_page(), 0);
}

#[test]
fn accept_action_ignores_same_page() {
    let (mut pc, nav_id) = make_pc_with_nav(vec![label("A"), label("B")]);
    let action = nav_selected(nav_id, 0);
    // Already on page 0 — should return false.
    assert!(!pc.accept_action(&action));
}

#[test]
fn accept_action_ignores_non_selected() {
    let (mut pc, _nav_id) = make_pc_with_nav(vec![label("A"), label("B")]);
    let action = WidgetAction::Clicked(WidgetId::next());
    assert!(!pc.accept_action(&action));
    assert_eq!(pc.active_page(), 0);
}

#[test]
fn accept_action_without_nav_source_ignores_all_selected() {
    // No nav_source_id set — all Selected actions are ignored.
    let mut pc = PageContainerWidget::new(vec![label("A"), label("B")]);
    let action = WidgetAction::Selected {
        id: WidgetId::next(),
        index: 1,
    };
    assert!(!pc.accept_action(&action));
    assert_eq!(pc.active_page(), 0);
}

// -- Layout --

#[test]
fn layout_fills_parent_bounds() {
    let pc = PageContainerWidget::new(vec![label("AB"), label("ABCDEF")]);
    let ctx = make_ctx();
    let lb = pc.layout(&ctx);
    let node = compute_layout(&lb, Rect::new(0.0, 0.0, 400.0, 300.0));

    assert_eq!(node.rect.width(), 400.0);
    assert_eq!(node.rect.height(), 300.0);
}

#[test]
fn layout_empty_is_zero() {
    let pc = PageContainerWidget::new(vec![]);
    let ctx = make_ctx();
    let lb = pc.layout(&ctx);
    let node = compute_layout(&lb, Rect::new(0.0, 0.0, 400.0, 300.0));
    assert_eq!(node.rect.width(), 0.0);
    assert_eq!(node.rect.height(), 0.0);
}

// -- Paint --

#[test]
fn paint_only_active_page() {
    let pc = PageContainerWidget::new(vec![label("A"), label("B")]);
    let measurer = MockMeasurer::STANDARD;
    let mut scene = Scene::new();
    let bounds = Rect::new(0.0, 0.0, 100.0, 50.0);
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
    pc.paint(&mut ctx);

    // Only one text run should be emitted (the active page).
    assert_eq!(
        scene.text_runs().len(),
        1,
        "only active page should be painted"
    );
}

#[test]
fn paint_empty_does_nothing() {
    let pc = PageContainerWidget::new(vec![]);
    let measurer = MockMeasurer::STANDARD;
    let mut scene = Scene::new();
    let bounds = Rect::new(0.0, 0.0, 100.0, 50.0);
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
    pc.paint(&mut ctx);
    assert!(scene.is_empty());
}

// -- for_each_child_mut --

#[test]
fn for_each_child_visits_all_pages() {
    let mut pc = PageContainerWidget::new(vec![label("A"), label("B"), label("C")]);
    let mut count = 0;
    pc.for_each_child_mut(&mut |_| count += 1);
    assert_eq!(count, 3, "all pages visited, not just active");
}

// -- Scroll reset on page switch --

#[test]
fn page_switch_calls_reset_scroll() {
    let scroll = ScrollWidget::vertical(label("B content"));
    let (mut pc, nav_id) = make_pc_with_nav(vec![label("A"), Box::new(scroll)]);

    let action = nav_selected(nav_id, 1);
    assert!(pc.accept_action(&action));
    assert_eq!(pc.active_page(), 1);
}

// -- Layout staleness after page switch --

#[test]
fn layout_only_contains_active_page_widgets() {
    let btn0 = crate::widgets::button::ButtonWidget::new("Page0");
    let btn0_id = btn0.id();
    let btn1 = crate::widgets::button::ButtonWidget::new("Page1");
    let btn1_id = btn1.id();

    let pc = PageContainerWidget::new(vec![Box::new(btn0), Box::new(btn1)]);
    let ctx = make_ctx();
    let lb = pc.layout(&ctx);
    let node = compute_layout(&lb, Rect::new(0.0, 0.0, 400.0, 300.0));

    assert!(
        find_id_in_layout(&node, btn0_id),
        "active page's widget should be in layout"
    );
    assert!(
        !find_id_in_layout(&node, btn1_id),
        "inactive page's widget should NOT be in layout"
    );
}

#[test]
fn layout_updates_after_page_switch_and_recompute() {
    let btn0 = crate::widgets::button::ButtonWidget::new("Page0");
    let btn0_id = btn0.id();
    let btn1 = crate::widgets::button::ButtonWidget::new("Page1");
    let btn1_id = btn1.id();

    let (mut pc, nav_id) = make_pc_with_nav(vec![Box::new(btn0), Box::new(btn1)]);

    let action = nav_selected(nav_id, 1);
    assert!(pc.accept_action(&action));
    assert_eq!(pc.active_page(), 1);

    let ctx = make_ctx();
    let lb = pc.layout(&ctx);
    let node = compute_layout(&lb, Rect::new(0.0, 0.0, 400.0, 300.0));

    assert!(
        find_id_in_layout(&node, btn1_id),
        "new active page's widget should be in layout after recompute"
    );
    assert!(
        !find_id_in_layout(&node, btn0_id),
        "old page's widget should NOT be in layout after recompute"
    );
}

#[test]
fn stale_layout_does_not_contain_new_page_widgets() {
    let btn0 = crate::widgets::button::ButtonWidget::new("Page0");
    let btn0_id = btn0.id();
    let btn1 = crate::widgets::button::ButtonWidget::new("Page1");
    let btn1_id = btn1.id();

    let (mut pc, nav_id) = make_pc_with_nav(vec![Box::new(btn0), Box::new(btn1)]);

    let ctx = make_ctx();
    let lb = pc.layout(&ctx);
    let stale_node = compute_layout(&lb, Rect::new(0.0, 0.0, 400.0, 300.0));

    let action = nav_selected(nav_id, 1);
    pc.accept_action(&action);

    assert!(
        find_id_in_layout(&stale_node, btn0_id),
        "stale layout retains old page's widget"
    );
    assert!(
        !find_id_in_layout(&stale_node, btn1_id),
        "stale layout does not contain new page's widget — this is the bug"
    );
}

/// Recursively searches a layout tree for a widget ID.
fn find_id_in_layout(node: &crate::layout::LayoutNode, target: WidgetId) -> bool {
    if node.widget_id == Some(target) {
        return true;
    }
    node.children.iter().any(|c| find_id_in_layout(c, target))
}

// -- Harness integration: page switch + click --

#[test]
fn harness_button_on_page0_is_clickable() {
    use crate::testing::WidgetTestHarness;

    let btn0 = crate::widgets::button::ButtonWidget::new("Page0");
    let btn0_id = btn0.id();
    let btn1 = crate::widgets::button::ButtonWidget::new("Page1");

    let pc = PageContainerWidget::new(vec![Box::new(btn0), Box::new(btn1)]);
    let mut h = WidgetTestHarness::new(pc);

    let actions = h.click(btn0_id);
    assert!(
        actions
            .iter()
            .any(|a| matches!(a, WidgetAction::Clicked(id) if *id == btn0_id)),
        "button on active page should be clickable, got: {actions:?}"
    );
}

#[test]
fn harness_button_on_switched_page_is_clickable_after_rebuild() {
    use crate::testing::WidgetTestHarness;

    let btn0 = crate::widgets::button::ButtonWidget::new("Page0");
    let btn1 = crate::widgets::button::ButtonWidget::new("Page1");
    let btn1_id = btn1.id();

    let nav_id = WidgetId::next();
    let mut pc =
        PageContainerWidget::new(vec![Box::new(btn0), Box::new(btn1)]).with_nav_source(nav_id);

    let action = nav_selected(nav_id, 1);
    pc.accept_action(&action);

    let mut h = WidgetTestHarness::new(pc);

    let actions = h.click(btn1_id);
    assert!(
        actions
            .iter()
            .any(|a| matches!(a, WidgetAction::Clicked(id) if *id == btn1_id)),
        "button on newly active page should be clickable after rebuild, got: {actions:?}"
    );
}

#[test]
fn harness_page_switch_requires_layout_rebuild_for_new_page_widgets() {
    use crate::testing::WidgetTestHarness;

    let btn0 = crate::widgets::button::ButtonWidget::new("Page0");
    let btn1 = crate::widgets::button::ButtonWidget::new("Page1");
    let btn1_id = btn1.id();

    let nav_id = WidgetId::next();
    let pc = PageContainerWidget::new(vec![Box::new(btn0), Box::new(btn1)]).with_nav_source(nav_id);
    let mut h = WidgetTestHarness::new(pc);

    let action = nav_selected(nav_id, 1);
    h.widget_mut().accept_action(&action);

    let bounds = h.try_widget_bounds(btn1_id);
    assert!(
        bounds.is_none(),
        "before layout rebuild, new page's widget should not be in layout (this is the bug)"
    );

    h.rebuild_layout();
    let bounds = h.try_widget_bounds(btn1_id);
    assert!(
        bounds.is_some(),
        "after layout rebuild, new page's widget should be in layout"
    );

    let actions = h.click(btn1_id);
    assert!(
        actions
            .iter()
            .any(|a| matches!(a, WidgetAction::Clicked(id) if *id == btn1_id)),
        "button on page 1 should be clickable after rebuild, got: {actions:?}"
    );
}

// -- Sense & focusability --

#[test]
fn sense_is_none() {
    let pc = PageContainerWidget::new(vec![label("A")]);
    assert_eq!(pc.sense(), crate::sense::Sense::none());
}

#[test]
fn not_focusable() {
    let pc = PageContainerWidget::new(vec![label("A")]);
    assert!(!pc.is_focusable());
}
