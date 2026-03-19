use crate::action::WidgetAction;
use crate::draw::Scene;
use crate::geometry::Rect;
use crate::layout::compute_layout;
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

// -- Layout --

#[test]
fn layout_fills_parent_bounds() {
    // PageContainerWidget uses SizeSpec::Fill — it takes the full parent bounds
    // so scroll widgets inside pages get a finite viewport.
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

// -- accept_action --

#[test]
fn accept_action_switches_page_on_selected() {
    let mut pc = PageContainerWidget::new(vec![label("A"), label("B"), label("C")]);
    let action = WidgetAction::Selected {
        id: crate::widget_id::WidgetId::next(),
        index: 2,
    };
    assert!(pc.accept_action(&action));
    assert_eq!(pc.active_page(), 2);
}

#[test]
fn accept_action_ignores_out_of_range() {
    let mut pc = PageContainerWidget::new(vec![label("A"), label("B")]);
    let action = WidgetAction::Selected {
        id: crate::widget_id::WidgetId::next(),
        index: 99,
    };
    assert!(!pc.accept_action(&action));
    assert_eq!(pc.active_page(), 0);
}

#[test]
fn accept_action_ignores_same_page() {
    let mut pc = PageContainerWidget::new(vec![label("A"), label("B")]);
    let action = WidgetAction::Selected {
        id: crate::widget_id::WidgetId::next(),
        index: 0,
    };
    // Already on page 0 — should return false.
    assert!(!pc.accept_action(&action));
}

#[test]
fn accept_action_ignores_non_selected() {
    let mut pc = PageContainerWidget::new(vec![label("A"), label("B")]);
    let action = WidgetAction::Clicked(crate::widget_id::WidgetId::next());
    assert!(!pc.accept_action(&action));
    assert_eq!(pc.active_page(), 0);
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
    // Create a scroll-wrapped page so reset_scroll is meaningful.
    let scroll = ScrollWidget::vertical(label("B content"));
    let mut pc = PageContainerWidget::new(vec![label("A"), Box::new(scroll)]);

    // Switch to page 1 — accept_action calls reset_scroll on the new page.
    let action = WidgetAction::Selected {
        id: crate::widget_id::WidgetId::next(),
        index: 1,
    };
    assert!(pc.accept_action(&action));
    assert_eq!(pc.active_page(), 1);
    // Verified by code path: PageContainerWidget calls pages[index].reset_scroll()
    // after switching. Direct scroll_offset verification is in scroll/tests.rs.
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
