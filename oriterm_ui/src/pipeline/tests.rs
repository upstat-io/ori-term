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

    prepaint_widget_tree(&mut widget, &bounds_map, None, &theme, Instant::now(), None);

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

    prepaint_widget_tree(&mut widget, &bounds_map, None, &theme, Instant::now(), None);

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
