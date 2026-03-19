//! Tests for the per-frame widget orchestration pipeline.
//!
//! Safety rail tests validate debug assertions added in Section 02:
//! double-visit detection, lifecycle ordering, registered-widget checks,
//! cross-phase consistency, and `register_widget` idempotency.

use std::collections::HashSet;
use std::time::Instant;

use crate::geometry::Rect;
use crate::interaction::{InteractionManager, LifecycleEvent};
use crate::layout::LayoutNode;
use crate::sense::Sense;
use crate::widget_id::WidgetId;
use crate::widgets::Widget;
use crate::widgets::contexts::{DrawCtx, LayoutCtx};

use super::{
    check_cross_phase_consistency, collect_layout_widget_ids, prepare_widget_frame,
    prepare_widget_tree,
};

// -- Test helpers --

/// Minimal widget for safety rail tests.
struct StubWidget {
    id: WidgetId,
}

impl StubWidget {
    fn new(id: WidgetId) -> Self {
        Self { id }
    }
}

impl Widget for StubWidget {
    fn id(&self) -> WidgetId {
        self.id
    }

    fn sense(&self) -> Sense {
        Sense::none()
    }

    fn layout(&self, _ctx: &LayoutCtx<'_>) -> crate::layout::LayoutBox {
        crate::layout::LayoutBox::leaf(10.0, 10.0)
    }

    fn paint(&self, _ctx: &mut DrawCtx<'_>) {}
}

/// Container that yields two separate children, but visits one of them
/// twice by holding two copies with the same WidgetId.
struct DoubleVisitContainer {
    id: WidgetId,
    child_a: StubWidget,
    child_b: StubWidget,
}

impl Widget for DoubleVisitContainer {
    fn id(&self) -> WidgetId {
        self.id
    }

    fn sense(&self) -> Sense {
        Sense::none()
    }

    fn layout(&self, _ctx: &LayoutCtx<'_>) -> crate::layout::LayoutBox {
        crate::layout::LayoutBox::leaf(100.0, 100.0)
    }

    fn paint(&self, _ctx: &mut DrawCtx<'_>) {}

    fn for_each_child_mut(&mut self, visitor: &mut dyn FnMut(&mut dyn Widget)) {
        visitor(&mut self.child_a);
        // Visit child_b which has the same WidgetId as child_a — double visit.
        visitor(&mut self.child_b);
    }
}

// -- Double-visit detection in pre-paint --

#[test]
#[should_panic(expected = "visited child")]
fn double_visit_in_prepare_widget_tree_panics() {
    let parent_id = WidgetId::next();
    let child_id = WidgetId::next();
    let mut container = DoubleVisitContainer {
        id: parent_id,
        child_a: StubWidget::new(child_id),
        child_b: StubWidget::new(child_id), // same ID — triggers double-visit
    };
    let mut interaction = InteractionManager::new();
    interaction.register_widget(parent_id);
    interaction.register_widget(child_id);
    let events = interaction.drain_events();

    prepare_widget_tree(
        &mut container,
        &mut interaction,
        &events,
        None,
        None,
        Instant::now(),
    );
}

// -- Lifecycle ordering: WidgetAdded-first --

#[test]
#[should_panic(expected = "before WidgetAdded")]
fn lifecycle_before_widget_added_panics() {
    let id = WidgetId::next();
    let mut widget = StubWidget::new(id);
    let mut interaction = InteractionManager::new();
    interaction.register_widget(id);
    // Drain WidgetAdded but do NOT deliver it via prepare_widget_frame.
    let _ = interaction.drain_events();

    // Now send HotChanged without ever delivering WidgetAdded.
    let events = vec![LifecycleEvent::HotChanged {
        widget_id: id,
        is_hot: true,
    }];
    prepare_widget_frame(
        &mut widget,
        &mut interaction,
        &events,
        None,
        None,
        Instant::now(),
    );
}

// -- Unregistered widget assertion --

#[test]
#[should_panic(expected = "unregistered widget")]
fn lifecycle_to_unregistered_widget_panics() {
    let id = WidgetId::next();
    let mut widget = StubWidget::new(id);
    let mut interaction = InteractionManager::new();
    // Do NOT register the widget.

    let events = vec![LifecycleEvent::HotChanged {
        widget_id: id,
        is_hot: true,
    }];
    prepare_widget_frame(
        &mut widget,
        &mut interaction,
        &events,
        None,
        None,
        Instant::now(),
    );
}

// -- Cross-phase consistency --

#[test]
fn cross_phase_superset_does_not_panic() {
    // Dispatch visiting extra children beyond layout is valid.
    let id_a = WidgetId::next();
    let id_b = WidgetId::next();
    let id_c = WidgetId::next();

    let mut layout_ids = HashSet::new();
    layout_ids.insert(id_a);
    layout_ids.insert(id_b);

    let mut dispatch_ids = HashSet::new();
    dispatch_ids.insert(id_a);
    dispatch_ids.insert(id_b);
    dispatch_ids.insert(id_c); // extra child in dispatch — valid

    check_cross_phase_consistency(&layout_ids, &dispatch_ids);
}

#[test]
#[should_panic(expected = "Cross-phase mismatch")]
fn cross_phase_missing_dispatch_child_panics() {
    let id_a = WidgetId::next();
    let id_b = WidgetId::next();

    let mut layout_ids = HashSet::new();
    layout_ids.insert(id_a);
    layout_ids.insert(id_b);

    let mut dispatch_ids = HashSet::new();
    dispatch_ids.insert(id_a);
    // id_b is in layout but NOT in dispatch — bug!

    check_cross_phase_consistency(&layout_ids, &dispatch_ids);
}

#[test]
fn collect_layout_widget_ids_walks_tree() {
    let root_id = WidgetId::next();
    let child_id = WidgetId::next();

    let child_node = LayoutNode {
        widget_id: Some(child_id),
        ..LayoutNode::new(
            Rect::new(0.0, 0.0, 10.0, 10.0),
            Rect::new(0.0, 0.0, 10.0, 10.0),
        )
    };
    let mut root_node = LayoutNode {
        widget_id: Some(root_id),
        ..LayoutNode::new(
            Rect::new(0.0, 0.0, 100.0, 100.0),
            Rect::new(0.0, 0.0, 100.0, 100.0),
        )
    };
    root_node.children.push(child_node);

    let mut ids = HashSet::new();
    collect_layout_widget_ids(&root_node, &mut ids);

    assert!(ids.contains(&root_id));
    assert!(ids.contains(&child_id));
    assert_eq!(ids.len(), 2);
}

// -- register_widget idempotency --

#[test]
fn register_widget_only_emits_widget_added_once() {
    let mut mgr = InteractionManager::new();
    let id = WidgetId::next();

    mgr.register_widget(id);
    let events = mgr.drain_events();
    assert_eq!(events.len(), 1);
    assert!(matches!(
        events[0],
        LifecycleEvent::WidgetAdded { widget_id } if widget_id == id
    ));

    // Second registration — no event.
    mgr.register_widget(id);
    let events = mgr.drain_events();
    assert!(events.is_empty());
}
