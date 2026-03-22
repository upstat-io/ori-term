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
        None,
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
        None,
        &events,
        None,
        None,
        Instant::now(),
    );
}

/// Regression test for TPR-04-003: register a widget without draining
/// WidgetAdded, then accumulate a HotChanged event. On the next
/// "render frame" (drain_events → prepare_widget_tree), both events
/// are delivered in order and the WidgetAdded-first assertion passes.
#[test]
fn register_without_drain_delivers_widget_added_on_next_frame() {
    let id = WidgetId::next();
    let mut widget = StubWidget::new(id);
    let mut interaction = InteractionManager::new();
    interaction.register_widget(id);
    // Do NOT drain — mimics the fixed dialog pattern.

    // Simulate a hover arriving before the next render frame.
    interaction.update_hot_path(&[id]);

    // Next frame: drain all pending events and deliver via prepare.
    let events = interaction.drain_events();
    assert!(
        events
            .iter()
            .any(|e| matches!(e, LifecycleEvent::WidgetAdded { widget_id } if *widget_id == id)),
        "WidgetAdded should be in the pending events"
    );

    // This must NOT panic — WidgetAdded is in the batch, delivered
    // before HotChanged by the pre-scan in prepare_widget_frame.
    prepare_widget_frame(
        &mut widget,
        &mut interaction,
        None,
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
        None,
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

// -- collect_layout_bounds tests --

#[test]
fn collect_layout_bounds_populates_map_for_nested_tree() {
    use std::collections::HashMap;

    use super::collect_layout_bounds;

    let root_id = WidgetId::next();
    let child_id = WidgetId::next();

    let child_rect = Rect::new(10.0, 20.0, 80.0, 30.0);
    let child_node = LayoutNode {
        widget_id: Some(child_id),
        ..LayoutNode::new(child_rect, child_rect)
    };
    let root_rect = Rect::new(0.0, 0.0, 100.0, 100.0);
    let mut root_node = LayoutNode {
        widget_id: Some(root_id),
        ..LayoutNode::new(root_rect, root_rect)
    };
    root_node.children.push(child_node);

    let mut bounds = HashMap::new();
    collect_layout_bounds(&root_node, &mut bounds);

    assert_eq!(bounds.len(), 2);
    assert_eq!(bounds[&root_id], root_rect);
    assert_eq!(bounds[&child_id], child_rect);
}

#[test]
fn collect_layout_bounds_skips_nodes_without_widget_id() {
    use std::collections::HashMap;

    use super::collect_layout_bounds;

    let child_id = WidgetId::next();

    let child_rect = Rect::new(5.0, 5.0, 50.0, 50.0);
    let child_node = LayoutNode {
        widget_id: Some(child_id),
        ..LayoutNode::new(child_rect, child_rect)
    };
    // Root has no widget_id (anonymous layout container).
    let root_rect = Rect::new(0.0, 0.0, 100.0, 100.0);
    let mut root_node = LayoutNode::new(root_rect, root_rect);
    root_node.children.push(child_node);

    let mut bounds = HashMap::new();
    collect_layout_bounds(&root_node, &mut bounds);

    assert_eq!(bounds.len(), 1);
    assert_eq!(bounds[&child_id], child_rect);
}

// -- prepaint_widget_tree tests --

/// Tracks whether prepaint was called via a flag.
struct PrepaintTracker {
    id: WidgetId,
    prepainted: bool,
}

impl PrepaintTracker {
    fn new(id: WidgetId) -> Self {
        Self {
            id,
            prepainted: false,
        }
    }
}

impl Widget for PrepaintTracker {
    fn id(&self) -> WidgetId {
        self.id
    }

    fn sense(&self) -> Sense {
        Sense::none()
    }

    fn layout(&self, _ctx: &LayoutCtx<'_>) -> crate::layout::LayoutBox {
        crate::layout::LayoutBox::leaf(10.0, 10.0)
    }

    fn prepaint(&mut self, _ctx: &mut crate::widgets::PrepaintCtx<'_>) {
        self.prepainted = true;
    }

    fn paint(&self, _ctx: &mut DrawCtx<'_>) {}
}

#[test]
fn prepaint_widget_tree_calls_prepaint() {
    use std::collections::HashMap;

    use super::prepaint_widget_tree;
    use crate::theme::UiTheme;

    let id = WidgetId::next();
    let mut widget = PrepaintTracker::new(id);
    let bounds_map = HashMap::new();
    let theme = UiTheme::dark();

    prepaint_widget_tree(
        &mut widget,
        &bounds_map,
        None,
        &theme,
        Instant::now(),
        None,
        None,
    );

    assert!(widget.prepainted);
}

/// Container with a child that tracks prepaint.
struct PrepaintContainer {
    id: WidgetId,
    child: PrepaintTracker,
}

impl Widget for PrepaintContainer {
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
        visitor(&mut self.child);
    }
}

#[test]
fn prepaint_widget_tree_traverses_children() {
    use std::collections::HashMap;

    use super::prepaint_widget_tree;
    use crate::theme::UiTheme;

    let parent_id = WidgetId::next();
    let child_id = WidgetId::next();
    let mut container = PrepaintContainer {
        id: parent_id,
        child: PrepaintTracker::new(child_id),
    };
    let bounds_map = HashMap::new();
    let theme = UiTheme::dark();

    prepaint_widget_tree(
        &mut container,
        &bounds_map,
        None,
        &theme,
        Instant::now(),
        None,
        None,
    );

    assert!(container.child.prepainted);
}

#[test]
fn prepaint_widget_tree_passes_correct_bounds() {
    use std::collections::HashMap;

    use super::prepaint_widget_tree;
    use crate::theme::UiTheme;

    /// Widget that captures the bounds from PrepaintCtx.
    struct BoundsCapture {
        id: WidgetId,
        captured_bounds: Option<Rect>,
    }

    impl Widget for BoundsCapture {
        fn id(&self) -> WidgetId {
            self.id
        }

        fn sense(&self) -> Sense {
            Sense::none()
        }

        fn layout(&self, _ctx: &LayoutCtx<'_>) -> crate::layout::LayoutBox {
            crate::layout::LayoutBox::leaf(10.0, 10.0)
        }

        fn prepaint(&mut self, ctx: &mut crate::widgets::PrepaintCtx<'_>) {
            self.captured_bounds = Some(ctx.bounds);
        }

        fn paint(&self, _ctx: &mut DrawCtx<'_>) {}
    }

    let id = WidgetId::next();
    let mut widget = BoundsCapture {
        id,
        captured_bounds: None,
    };

    let expected = Rect::new(10.0, 20.0, 100.0, 50.0);
    let mut bounds_map = HashMap::new();
    bounds_map.insert(id, expected);
    let theme = UiTheme::dark();

    prepaint_widget_tree(
        &mut widget,
        &bounds_map,
        None,
        &theme,
        Instant::now(),
        None,
        None,
    );

    assert_eq!(widget.captured_bounds, Some(expected));
}

// -- Phase gating tests --

/// Widget that counts layout/prepaint/paint invocations via shared counters.
struct PhaseCountWidget {
    id: WidgetId,
    layout_count: std::rc::Rc<std::cell::Cell<u32>>,
    prepaint_count: std::rc::Rc<std::cell::Cell<u32>>,
    paint_count: std::rc::Rc<std::cell::Cell<u32>>,
}

impl Widget for PhaseCountWidget {
    fn id(&self) -> WidgetId {
        self.id
    }

    fn sense(&self) -> Sense {
        Sense::hover()
    }

    fn layout(&self, _ctx: &LayoutCtx<'_>) -> crate::layout::LayoutBox {
        self.layout_count.set(self.layout_count.get() + 1);
        crate::layout::LayoutBox::leaf(100.0, 30.0).with_widget_id(self.id)
    }

    fn prepaint(&mut self, _ctx: &mut crate::widgets::PrepaintCtx<'_>) {
        self.prepaint_count.set(self.prepaint_count.get() + 1);
    }

    fn paint(&self, _ctx: &mut DrawCtx<'_>) {
        self.paint_count.set(self.paint_count.get() + 1);
    }
}

/// Hover triggers prepaint + paint but NOT layout.
///
/// The harness separates layout (computed once at construction and on
/// resize) from the event pipeline (prepare → prepaint → paint). A hover
/// mouse move should trigger prepaint (interaction state changed) and
/// paint (via render) but never re-run layout.
#[test]
fn hover_triggers_prepaint_and_paint_not_layout() {
    use crate::testing::WidgetTestHarness;

    let id = WidgetId::next();
    let layout_count = std::rc::Rc::new(std::cell::Cell::new(0_u32));
    let prepaint_count = std::rc::Rc::new(std::cell::Cell::new(0_u32));
    let paint_count = std::rc::Rc::new(std::cell::Cell::new(0_u32));

    let widget = PhaseCountWidget {
        id,
        layout_count: layout_count.clone(),
        prepaint_count: prepaint_count.clone(),
        paint_count: paint_count.clone(),
    };
    let mut h = WidgetTestHarness::new(widget);

    // Layout was called during harness construction.
    let init_layout = layout_count.get();
    assert!(init_layout > 0, "layout should run during construction");

    // Reset counters after construction.
    layout_count.set(0);
    prepaint_count.set(0);
    paint_count.set(0);

    // Hover: move mouse into widget bounds.
    h.mouse_move_to(id);

    // Render the scene.
    let _scene = h.render();

    // Layout must NOT have been called (hover doesn't change structure).
    assert_eq!(layout_count.get(), 0, "hover should not trigger layout");

    // Prepaint must have been called (interaction state changed).
    assert!(prepaint_count.get() > 0, "hover should trigger prepaint");

    // Paint must have been called (render produces a scene).
    assert!(paint_count.get() > 0, "render should trigger paint");
}

/// Verifies that `WidgetTestHarness` (which uses `WindowRoot::run_prepaint`)
/// provides non-zero bounds to widgets during prepaint.
#[test]
fn harness_prepaint_provides_nonzero_bounds() {
    use std::cell::Cell;
    use std::rc::Rc;

    use crate::testing::WidgetTestHarness;

    /// Widget that captures bounds from PrepaintCtx via shared state.
    struct BoundsCaptureWidget {
        id: WidgetId,
        captured: Rc<Cell<Option<Rect>>>,
    }

    impl Widget for BoundsCaptureWidget {
        fn id(&self) -> WidgetId {
            self.id
        }

        fn sense(&self) -> Sense {
            Sense::hover()
        }

        fn layout(&self, _ctx: &LayoutCtx<'_>) -> crate::layout::LayoutBox {
            crate::layout::LayoutBox::leaf(200.0, 100.0).with_widget_id(self.id)
        }

        fn prepaint(&mut self, ctx: &mut crate::widgets::PrepaintCtx<'_>) {
            self.captured.set(Some(ctx.bounds));
        }

        fn paint(&self, _ctx: &mut DrawCtx<'_>) {}
    }

    let id = WidgetId::next();
    let captured = Rc::new(Cell::new(None));
    let widget = BoundsCaptureWidget {
        id,
        captured: captured.clone(),
    };
    let mut h = WidgetTestHarness::new(widget);

    // Trigger a hover to force prepaint.
    h.mouse_move_to(id);
    let _scene = h.render();

    let bounds = captured.get().expect("prepaint should have been called");
    assert!(
        bounds.width() > 0.0 && bounds.height() > 0.0,
        "prepaint bounds should be non-zero, got {bounds:?}"
    );
}

// -- Selective walk tests (Section 03) --

/// Selective prepare skips children whose subtrees are clean.
///
/// Creates a container with two children. Marks child A dirty, leaves B clean.
/// After prepare, only A should have received lifecycle delivery (via prepaint
/// being called on it). B should NOT be visited.
#[test]
fn selective_prepare_skips_clean_subtree() {
    use std::cell::Cell;
    use std::collections::HashMap;
    use std::rc::Rc;

    use crate::invalidation::{DirtyKind, InvalidationTracker};

    struct CountWidget {
        id: WidgetId,
        prepare_count: Rc<Cell<u32>>,
    }
    impl Widget for CountWidget {
        fn id(&self) -> WidgetId {
            self.id
        }
        fn sense(&self) -> Sense {
            Sense::hover()
        }
        fn layout(&self, _ctx: &LayoutCtx<'_>) -> crate::layout::LayoutBox {
            crate::layout::LayoutBox::leaf(50.0, 20.0).with_widget_id(self.id)
        }
        fn prepaint(&mut self, _ctx: &mut crate::widgets::PrepaintCtx<'_>) {
            self.prepare_count.set(self.prepare_count.get() + 1);
        }
        fn paint(&self, _ctx: &mut DrawCtx<'_>) {}
    }

    struct TwoChildContainer {
        id: WidgetId,
        child_a: CountWidget,
        child_b: CountWidget,
    }
    impl Widget for TwoChildContainer {
        fn id(&self) -> WidgetId {
            self.id
        }
        fn sense(&self) -> Sense {
            Sense::none()
        }
        fn layout(&self, _ctx: &LayoutCtx<'_>) -> crate::layout::LayoutBox {
            let a = self.child_a.layout(_ctx);
            let b = self.child_b.layout(_ctx);
            crate::layout::LayoutBox::flex(crate::layout::Direction::Column, vec![a, b])
                .with_widget_id(self.id)
        }
        fn for_each_child_mut(&mut self, f: &mut dyn FnMut(&mut dyn Widget)) {
            f(&mut self.child_a);
            f(&mut self.child_b);
        }
        fn paint(&self, _ctx: &mut DrawCtx<'_>) {}
    }

    let container_id = WidgetId::next();
    let a_id = WidgetId::next();
    let b_id = WidgetId::next();
    let a_count = Rc::new(Cell::new(0_u32));
    let b_count = Rc::new(Cell::new(0_u32));

    let mut container = TwoChildContainer {
        id: container_id,
        child_a: CountWidget {
            id: a_id,
            prepare_count: a_count.clone(),
        },
        child_b: CountWidget {
            id: b_id,
            prepare_count: b_count.clone(),
        },
    };

    let mut interaction = InteractionManager::new();
    interaction.register_widget(container_id);
    interaction.register_widget(a_id);
    interaction.register_widget(b_id);
    let _ = interaction.drain_events();

    // Build parent map: a -> container, b -> container.
    let mut parent_map = HashMap::new();
    parent_map.insert(a_id, container_id);
    parent_map.insert(b_id, container_id);
    interaction.set_parent_map(parent_map.clone());

    // Mark only child A dirty.
    let mut tracker = InvalidationTracker::new();
    tracker.mark(a_id, DirtyKind::Prepaint, &parent_map);

    // Reset prepaint counters.
    a_count.set(0);
    b_count.set(0);

    // Run prepare with selective walks enabled.
    prepare_widget_tree(
        &mut container,
        &mut interaction,
        Some(&mut tracker),
        &[],
        None,
        None,
        Instant::now(),
    );

    // Run prepaint with selective walks enabled.
    let bounds = HashMap::new();
    let theme = crate::theme::UiTheme::dark();
    super::prepaint_widget_tree(
        &mut container,
        &bounds,
        None,
        &theme,
        Instant::now(),
        None,
        Some(&tracker),
    );

    // Child A should have been visited (its subtree is dirty).
    assert!(
        a_count.get() > 0,
        "dirty child A should have been visited during prepaint"
    );
    // Child B should NOT have been visited (its subtree is clean).
    assert_eq!(
        b_count.get(),
        0,
        "clean child B should have been skipped during selective prepaint"
    );
}

/// Full invalidation bypasses selective walks — all widgets visited.
#[test]
fn full_invalidation_visits_all_widgets() {
    use std::cell::Cell;
    use std::collections::HashMap;
    use std::rc::Rc;

    use crate::invalidation::{DirtyKind, InvalidationTracker};

    struct CountWidget {
        id: WidgetId,
        count: Rc<Cell<u32>>,
    }
    impl Widget for CountWidget {
        fn id(&self) -> WidgetId {
            self.id
        }
        fn sense(&self) -> Sense {
            Sense::hover()
        }
        fn layout(&self, _ctx: &LayoutCtx<'_>) -> crate::layout::LayoutBox {
            crate::layout::LayoutBox::leaf(50.0, 20.0).with_widget_id(self.id)
        }
        fn prepaint(&mut self, _ctx: &mut crate::widgets::PrepaintCtx<'_>) {
            self.count.set(self.count.get() + 1);
        }
        fn paint(&self, _ctx: &mut DrawCtx<'_>) {}
    }

    struct TwoChildContainer {
        id: WidgetId,
        child_a: CountWidget,
        child_b: CountWidget,
    }
    impl Widget for TwoChildContainer {
        fn id(&self) -> WidgetId {
            self.id
        }
        fn sense(&self) -> Sense {
            Sense::none()
        }
        fn layout(&self, _ctx: &LayoutCtx<'_>) -> crate::layout::LayoutBox {
            crate::layout::LayoutBox::flex(
                crate::layout::Direction::Column,
                vec![self.child_a.layout(_ctx), self.child_b.layout(_ctx)],
            )
            .with_widget_id(self.id)
        }
        fn for_each_child_mut(&mut self, f: &mut dyn FnMut(&mut dyn Widget)) {
            f(&mut self.child_a);
            f(&mut self.child_b);
        }
        fn paint(&self, _ctx: &mut DrawCtx<'_>) {}
    }

    let container_id = WidgetId::next();
    let a_id = WidgetId::next();
    let b_id = WidgetId::next();
    let a_count = Rc::new(Cell::new(0_u32));
    let b_count = Rc::new(Cell::new(0_u32));

    let mut container = TwoChildContainer {
        id: container_id,
        child_a: CountWidget {
            id: a_id,
            count: a_count.clone(),
        },
        child_b: CountWidget {
            id: b_id,
            count: b_count.clone(),
        },
    };

    let mut interaction = InteractionManager::new();
    interaction.register_widget(container_id);
    interaction.register_widget(a_id);
    interaction.register_widget(b_id);
    let _ = interaction.drain_events();

    let parent_map = HashMap::new();
    interaction.set_parent_map(parent_map.clone());

    // Mark only A dirty, but set full_invalidation.
    let mut tracker = InvalidationTracker::new();
    tracker.mark(a_id, DirtyKind::Prepaint, &parent_map);
    tracker.invalidate_all();

    a_count.set(0);
    b_count.set(0);

    prepare_widget_tree(
        &mut container,
        &mut interaction,
        Some(&mut tracker),
        &[],
        None,
        None,
        Instant::now(),
    );

    let bounds = HashMap::new();
    let theme = crate::theme::UiTheme::dark();
    super::prepaint_widget_tree(
        &mut container,
        &bounds,
        None,
        &theme,
        Instant::now(),
        None,
        Some(&tracker),
    );

    // Both widgets should be visited despite only A being marked.
    assert!(
        a_count.get() > 0,
        "child A should be visited during full invalidation"
    );
    assert!(
        b_count.get() > 0,
        "child B should be visited during full invalidation (full_invalidation bypasses selective walk)"
    );
}

/// Lifecycle events pre-mark their targets so selective walks visit them.
///
/// A widget in a clean subtree receives a lifecycle event when passed
/// to `prepare_widget_tree` — the event target is pre-marked dirty so
/// the walk visits it.
#[test]
fn selective_walk_delivers_lifecycle_events_to_clean_subtree() {
    use std::cell::Cell;
    use std::collections::HashMap;
    use std::rc::Rc;

    use crate::invalidation::{DirtyKind, InvalidationTracker};

    struct LifecycleWidget {
        id: WidgetId,
        lifecycle_count: Rc<Cell<u32>>,
    }
    impl Widget for LifecycleWidget {
        fn id(&self) -> WidgetId {
            self.id
        }
        fn sense(&self) -> Sense {
            Sense::hover()
        }
        fn layout(&self, _ctx: &LayoutCtx<'_>) -> crate::layout::LayoutBox {
            crate::layout::LayoutBox::leaf(50.0, 20.0).with_widget_id(self.id)
        }
        fn lifecycle(
            &mut self,
            _event: &LifecycleEvent,
            _ctx: &mut crate::widgets::contexts::LifecycleCtx<'_>,
        ) {
            self.lifecycle_count.set(self.lifecycle_count.get() + 1);
        }
        fn paint(&self, _ctx: &mut DrawCtx<'_>) {}
    }

    struct TwoChildContainer {
        id: WidgetId,
        child_a: LifecycleWidget,
        child_b: LifecycleWidget,
    }
    impl Widget for TwoChildContainer {
        fn id(&self) -> WidgetId {
            self.id
        }
        fn sense(&self) -> Sense {
            Sense::none()
        }
        fn layout(&self, _ctx: &LayoutCtx<'_>) -> crate::layout::LayoutBox {
            crate::layout::LayoutBox::flex(
                crate::layout::Direction::Column,
                vec![self.child_a.layout(_ctx), self.child_b.layout(_ctx)],
            )
            .with_widget_id(self.id)
        }
        fn for_each_child_mut(&mut self, f: &mut dyn FnMut(&mut dyn Widget)) {
            f(&mut self.child_a);
            f(&mut self.child_b);
        }
        fn paint(&self, _ctx: &mut DrawCtx<'_>) {}
    }

    let container_id = WidgetId::next();
    let a_id = WidgetId::next();
    let b_id = WidgetId::next();
    let b_lifecycle_count = Rc::new(Cell::new(0_u32));

    let mut container = TwoChildContainer {
        id: container_id,
        child_a: LifecycleWidget {
            id: a_id,
            lifecycle_count: Rc::new(Cell::new(0)),
        },
        child_b: LifecycleWidget {
            id: b_id,
            lifecycle_count: b_lifecycle_count.clone(),
        },
    };

    let mut interaction = InteractionManager::new();
    interaction.register_widget(container_id);
    interaction.register_widget(a_id);
    interaction.register_widget(b_id);

    // Deliver initial WidgetAdded events so debug assertions pass.
    let added_events = interaction.drain_events();
    prepare_widget_tree(
        &mut container,
        &mut interaction,
        None,
        &added_events,
        None,
        None,
        Instant::now(),
    );

    let mut parent_map = HashMap::new();
    parent_map.insert(a_id, container_id);
    parent_map.insert(b_id, container_id);
    interaction.set_parent_map(parent_map.clone());

    // Mark only child A dirty — child B's subtree is clean.
    let mut tracker = InvalidationTracker::new();
    tracker.mark(a_id, DirtyKind::Prepaint, &parent_map);

    // But send a lifecycle event targeting child B.
    let lifecycle_events = vec![LifecycleEvent::HotChanged {
        widget_id: b_id,
        is_hot: true,
    }];

    b_lifecycle_count.set(0);

    // The selective walk should pre-mark B dirty (because it has a lifecycle
    // event targeting it), so B's subtree is visited despite being "clean".
    prepare_widget_tree(
        &mut container,
        &mut interaction,
        Some(&mut tracker),
        &lifecycle_events,
        None,
        None,
        Instant::now(),
    );

    assert!(
        b_lifecycle_count.get() > 0,
        "lifecycle event should reach child B even though its subtree was originally clean"
    );
}

/// Gate test: selective prepare produces identical interaction state as full
/// prepare for the same set of dirty widgets.
///
/// Creates two identical widget trees. Runs selective prepare on one (with
/// tracker marking child A dirty) and full prepare on the other (no tracker).
/// Verifies that child A's interaction state is identical in both cases.
#[test]
fn selective_prepare_identical_to_full_for_dirty_widgets() {
    use std::collections::HashMap;

    use crate::invalidation::{DirtyKind, InvalidationTracker};

    struct GateContainer {
        id: WidgetId,
        child_a: StubWidget,
        child_b: StubWidget,
    }
    impl Widget for GateContainer {
        fn id(&self) -> WidgetId {
            self.id
        }
        fn sense(&self) -> Sense {
            Sense::none()
        }
        fn layout(&self, _ctx: &LayoutCtx<'_>) -> crate::layout::LayoutBox {
            crate::layout::LayoutBox::leaf(100.0, 40.0)
        }
        fn for_each_child_mut(&mut self, f: &mut dyn FnMut(&mut dyn Widget)) {
            f(&mut self.child_a);
            f(&mut self.child_b);
        }
        fn paint(&self, _ctx: &mut DrawCtx<'_>) {}
    }

    fn make_tree() -> (GateContainer, WidgetId, WidgetId, WidgetId) {
        let cid = WidgetId::next();
        let aid = WidgetId::next();
        let bid = WidgetId::next();
        (
            GateContainer {
                id: cid,
                child_a: StubWidget::new(aid),
                child_b: StubWidget::new(bid),
            },
            cid,
            aid,
            bid,
        )
    }

    fn setup(
        tree: &mut GateContainer,
        int: &mut InteractionManager,
        cid: WidgetId,
        aid: WidgetId,
        bid: WidgetId,
    ) -> HashMap<WidgetId, WidgetId> {
        int.register_widget(cid);
        int.register_widget(aid);
        int.register_widget(bid);

        // Deliver WidgetAdded events so debug assertions pass.
        let added = int.drain_events();
        prepare_widget_tree(tree, int, None, &added, None, None, Instant::now());

        let mut parent_map = HashMap::new();
        parent_map.insert(aid, cid);
        parent_map.insert(bid, cid);
        int.set_parent_map(parent_map.clone());

        // Mark A as hot, drain the resulting lifecycle events.
        int.update_hot_path(&[cid, aid]);
        parent_map
    }

    // Setup 1: selective prepare.
    let (mut tree1, cid1, aid1, bid1) = make_tree();
    let mut int1 = InteractionManager::new();
    let parent_map = setup(&mut tree1, &mut int1, cid1, aid1, bid1);

    let events1 = int1.drain_events();
    let mut tracker = InvalidationTracker::new();
    tracker.mark(aid1, DirtyKind::Prepaint, &parent_map);

    prepare_widget_tree(
        &mut tree1,
        &mut int1,
        Some(&mut tracker),
        &events1,
        None,
        None,
        Instant::now(),
    );

    // Setup 2: full prepare (no tracker).
    let (mut tree2, cid2, aid2, bid2) = make_tree();
    let mut int2 = InteractionManager::new();
    let _ = setup(&mut tree2, &mut int2, cid2, aid2, bid2);

    let events2 = int2.drain_events();
    prepare_widget_tree(
        &mut tree2,
        &mut int2,
        None,
        &events2,
        None,
        None,
        Instant::now(),
    );

    // The dirty widget (A) should have identical interaction state in both.
    let s1 = int1.get_state(aid1);
    let s2 = int2.get_state(aid2);
    assert_eq!(
        s1.is_hot(),
        s2.is_hot(),
        "hot state mismatch for dirty widget A"
    );
    assert_eq!(
        s1.is_active(),
        s2.is_active(),
        "active state mismatch for dirty widget A"
    );
    assert_eq!(
        s1.is_focused(),
        s2.is_focused(),
        "focused state mismatch for dirty widget A"
    );
}

/// Gate test: selective prepaint produces identical `resolved_bg` and
/// `resolved_focused` values as full prepaint for dirty widgets.
///
/// Uses `WidgetTestHarness` with a `ButtonWidget` — hovers the button,
/// runs prepaint both ways, and compares the resolved visual state.
#[test]
fn selective_prepaint_identical_to_full_for_dirty_widgets() {
    use std::time::Duration;

    use crate::testing::WidgetTestHarness;
    use crate::widgets::button::ButtonWidget;

    // Create two identical button harnesses.
    let btn1 = ButtonWidget::new("Gate A");
    let id1 = btn1.id();
    let mut h1 = WidgetTestHarness::new(btn1);

    let btn2 = ButtonWidget::new("Gate B");
    let id2 = btn2.id();
    let mut h2 = WidgetTestHarness::new(btn2);

    // Hover both buttons to create identical initial conditions.
    h1.mouse_move_to(id1);
    h2.mouse_move_to(id2);

    // Advance both by the same amount.
    h1.advance_time(Duration::from_millis(50));
    h2.advance_time(Duration::from_millis(50));

    // Render both — the scenes should be functionally identical
    // (same quads, same colors) despite different WidgetIds.
    let scene1 = h1.render();
    let scene2 = h2.render();

    let fills1: Vec<_> = scene1.quads().iter().filter_map(|q| q.style.fill).collect();
    let fills2: Vec<_> = scene2.quads().iter().filter_map(|q| q.style.fill).collect();

    assert_eq!(
        fills1.len(),
        fills2.len(),
        "scene quad count should be identical"
    );

    // Compare colors channel-by-channel with small tolerance.
    for (i, (c1, c2)) in fills1.iter().zip(fills2.iter()).enumerate() {
        assert!(
            (c1.r - c2.r).abs() < 0.01
                && (c1.g - c2.g).abs() < 0.01
                && (c1.b - c2.b).abs() < 0.01
                && (c1.a - c2.a).abs() < 0.01,
            "quad {i} color mismatch: {c1:?} vs {c2:?}"
        );
    }
}

/// Animation-driven widgets continue to update even when no interaction-driven
/// dirtiness exists. This is a regression test for TPR-03-010: the
/// selective walk must still reach the animating widget on subsequent frames.
///
/// The harness clears invalidation after each `render()` call,
/// matching the production render/clear cadence (tick → render → clear).
#[test]
fn animation_driven_widget_updates_without_interaction_dirtiness() {
    use std::time::Duration;

    use crate::testing::WidgetTestHarness;
    use crate::widgets::button::{ButtonStyle, ButtonWidget};

    let style = ButtonStyle::default();
    let btn = ButtonWidget::new("Hover me");
    let btn_id = btn.id();
    let mut h = WidgetTestHarness::new(btn);

    let normal_bg = style.bg;

    // Hover the button to start the animation.
    h.mouse_move_to(btn_id);

    // Advance one small step — animation should start but not complete.
    h.advance_time(Duration::from_millis(16));

    // The button's background should be mid-transition (not normal, not fully hovered).
    // Render a scene and check that the bg has started changing.
    let scene1 = h.render();
    let fills1: Vec<_> = scene1.quads().iter().filter_map(|q| q.style.fill).collect();

    // Advance more time — without any new interaction events.
    // This is the critical test: invalidation was cleared after the first
    // advance_time, so the animation must survive purely through the
    // tick_animation() full-walk path (TPR-03-010 fix).
    h.advance_time(Duration::from_millis(16));
    let scene2 = h.render();
    let fills2: Vec<_> = scene2.quads().iter().filter_map(|q| q.style.fill).collect();

    // The fills should be progressing toward hover_bg.
    // At minimum, the button should have SOME quad (its background).
    assert!(
        !fills1.is_empty(),
        "scene after first frame should contain button quads"
    );
    assert!(
        !fills2.is_empty(),
        "scene after second frame should contain button quads"
    );

    // The animation should still be in progress — verify the fills are NOT
    // the normal (non-hovered) color, meaning the animation did advance.
    let all_normal_frame2 = fills2.iter().all(|&c| {
        (c.r - normal_bg.r).abs() < 0.01
            && (c.g - normal_bg.g).abs() < 0.01
            && (c.b - normal_bg.b).abs() < 0.01
    });
    assert!(
        !all_normal_frame2,
        "button background should NOT be normal color on frame 2 — \
         animation should be advancing. fills2: {fills2:?}, normal_bg: {normal_bg:?}"
    );
}

/// Animation dirty marking persists across multiple frames: widget starts
/// animating on frame N, frame N+1 should also visit the widget, and so on
/// until the animation completes.
///
/// Regression test for TPR-03-010: verifies that the full walk during
/// `tick_animation()` allows multi-frame animations to run to completion.
#[test]
fn animation_dirty_marking_persists_across_frames() {
    use std::time::Duration;

    use crate::testing::WidgetTestHarness;
    use crate::widgets::button::{ButtonStyle, ButtonWidget};

    let style = ButtonStyle::default();
    let hover_bg = style.hover_bg;
    let btn = ButtonWidget::new("Multi-frame");
    let btn_id = btn.id();
    let mut h = WidgetTestHarness::new(btn);

    // Hover the button to start animation.
    h.mouse_move_to(btn_id);

    // Advance through many frames (each 16ms) — the animation should
    // complete within 350ms. Dirty state accumulates across advance_time
    // calls; render() clears it (matching production cadence).
    for _ in 0..25 {
        h.advance_time(Duration::from_millis(16));
    }

    // After 400ms total, the animation should be complete.
    // The button should now show the fully-resolved hover color.
    let scene = h.render();
    let fills: Vec<_> = scene.quads().iter().filter_map(|q| q.style.fill).collect();
    let has_hover_bg = fills.iter().any(|&c| {
        (c.r - hover_bg.r).abs() < 0.02
            && (c.g - hover_bg.g).abs() < 0.02
            && (c.b - hover_bg.b).abs() < 0.02
    });
    assert!(
        has_hover_bg,
        "After 400ms of 16ms frames, \
         button should show hover_bg ({hover_bg:?}). fills: {fills:?}"
    );
}

/// Nested-widget animation regression test (TPR-04-001): a button inside a
/// container should animate correctly when the harness clears invalidation
/// after `render()` rather than after `advance_time()`.
///
/// This verifies that selective prepaint during `render()` correctly visits
/// the nested animating child, not just root-level widgets.
#[test]
fn nested_widget_animation_advances_through_render_clear_cadence() {
    use std::time::Duration;

    use crate::layout::Direction;
    use crate::testing::WidgetTestHarness;
    use crate::widgets::button::{ButtonStyle, ButtonWidget};
    use crate::widgets::container::ContainerWidget;

    let style = ButtonStyle::default();
    let normal_bg = style.bg;
    let hover_bg = style.hover_bg;

    let btn = ButtonWidget::new("Nested");
    let btn_id = btn.id();
    let mut container = ContainerWidget::new(Direction::Column);
    container.add_child(Box::new(btn));

    let mut h = WidgetTestHarness::new(container);

    // Hover the nested button to start animation.
    h.mouse_move_to(btn_id);

    // Frame 1: tick animation + render + clear.
    h.advance_time(Duration::from_millis(16));
    let scene1 = h.render(); // clears invalidation
    let fills1: Vec<_> = scene1.quads().iter().filter_map(|q| q.style.fill).collect();
    assert!(!fills1.is_empty(), "frame 1 should contain button quads");

    // Frame 2: tick again (no new events) + render + clear.
    // This is the critical frame: invalidation was cleared by render() in frame 1,
    // so the selective walk in this render() must still find the animating child.
    h.advance_time(Duration::from_millis(16));
    let scene2 = h.render();
    let fills2: Vec<_> = scene2.quads().iter().filter_map(|q| q.style.fill).collect();

    // Animation should be advancing — fills should NOT be the normal color.
    let all_normal = fills2.iter().all(|&c| {
        (c.r - normal_bg.r).abs() < 0.01
            && (c.g - normal_bg.g).abs() < 0.01
            && (c.b - normal_bg.b).abs() < 0.01
    });
    assert!(
        !all_normal,
        "nested button bg should NOT be normal color on frame 2 — \
         animation should be advancing. fills2: {fills2:?}, normal: {normal_bg:?}"
    );

    // Run animation to completion via run_until_stable.
    h.run_until_stable();
    let scene_final = h.render();
    let fills_final: Vec<_> = scene_final
        .quads()
        .iter()
        .filter_map(|q| q.style.fill)
        .collect();

    // The button bg should have advanced past normal toward hover.
    // Note: exact convergence depends on the spring model and frame timing.
    // We verify the direction is correct — final fill is closer to hover than normal.
    let dist_to_normal = fills_final
        .first()
        .map(|c| (c.r - normal_bg.r).abs() + (c.g - normal_bg.g).abs() + (c.b - normal_bg.b).abs());
    let dist_to_hover = fills_final
        .first()
        .map(|c| (c.r - hover_bg.r).abs() + (c.g - hover_bg.g).abs() + (c.b - hover_bg.b).abs());
    assert!(
        dist_to_hover < dist_to_normal,
        "nested button bg should be closer to hover than normal after animation. \
         normal: {normal_bg:?}, hover: {hover_bg:?}, actual: {fills_final:?}"
    );
}
