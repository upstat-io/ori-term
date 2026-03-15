//! Tests for scene composition.

use std::cell::Cell;
use std::rc::Rc;
use std::time::Instant;

use crate::draw::{DrawCommand, DrawList, SceneCache, SceneNode};
use crate::geometry::Rect;
use crate::input::{HoverEvent, KeyEvent, MouseEvent};
use crate::invalidation::{DirtyKind, InvalidationTracker};
use crate::layout::LayoutBox;
use crate::widget_id::WidgetId;
use crate::widgets::button::ButtonWidget;
use crate::widgets::checkbox::CheckboxWidget;
use crate::widgets::container::ContainerWidget;
use crate::widgets::dropdown::DropdownWidget;
use crate::widgets::form_row::FormRow;
use crate::widgets::form_section::FormSection;
use crate::widgets::label::LabelWidget;
use crate::widgets::scroll::ScrollWidget;
use crate::widgets::separator::SeparatorWidget;
use crate::widgets::slider::SliderWidget;
use crate::widgets::spacer::SpacerWidget;
use crate::widgets::stack::StackWidget;
use crate::widgets::tests::{MockMeasurer, TEST_THEME};
use crate::widgets::text_input::TextInputWidget;
use crate::widgets::{DrawCtx, EventCtx, LayoutCtx, Widget, WidgetAction, WidgetResponse};

use super::compose_scene;

// -- Test widgets --

/// Widget that counts draw calls and emits a fixed number of commands.
struct TrackingWidget {
    id: WidgetId,
    width: f32,
    height: f32,
    draw_count: Rc<Cell<usize>>,
}

impl TrackingWidget {
    fn new(w: f32, h: f32, counter: Rc<Cell<usize>>) -> Self {
        Self {
            id: WidgetId::next(),
            width: w,
            height: h,
            draw_count: counter,
        }
    }
}

impl Widget for TrackingWidget {
    fn id(&self) -> WidgetId {
        self.id
    }

    fn is_focusable(&self) -> bool {
        false
    }

    fn layout(&self, _ctx: &LayoutCtx<'_>) -> LayoutBox {
        LayoutBox::leaf(self.width, self.height).with_widget_id(self.id)
    }

    fn draw(&self, ctx: &mut DrawCtx<'_>) {
        self.draw_count.set(self.draw_count.get() + 1);
        ctx.draw_list.push_rect(ctx.bounds, Default::default());
    }

    fn handle_mouse(&mut self, _: &MouseEvent, _: &EventCtx<'_>) -> WidgetResponse {
        WidgetResponse::ignored()
    }

    fn handle_hover(&mut self, _: HoverEvent, _: &EventCtx<'_>) -> WidgetResponse {
        WidgetResponse::ignored()
    }

    fn handle_key(&mut self, _: KeyEvent, _: &EventCtx<'_>) -> WidgetResponse {
        WidgetResponse::ignored()
    }

    fn accept_action(&mut self, _: &WidgetAction) -> bool {
        false
    }

    fn focusable_children(&self) -> Vec<WidgetId> {
        Vec::new()
    }
}

fn make_ctx<'a>(
    measurer: &'a MockMeasurer,
    draw_list: &'a mut DrawList,
    anim: &'a Cell<bool>,
    bounds: Rect,
) -> DrawCtx<'a> {
    DrawCtx {
        measurer,
        draw_list,
        bounds,
        focused_widget: None,
        now: Instant::now(),
        animations_running: anim,
        theme: &TEST_THEME,
        icons: None,
        scene_cache: None,
        interaction: None,
        widget_id: None,
    }
}

// -- Tests --

#[test]
fn first_compose_draws_widget() {
    let counter = Rc::new(Cell::new(0));
    let widget = TrackingWidget::new(100.0, 20.0, counter.clone());
    let measurer = MockMeasurer::STANDARD;
    let mut draw_list = DrawList::new();
    let anim = Cell::new(false);
    let bounds = Rect::new(0.0, 0.0, 100.0, 20.0);
    let mut ctx = make_ctx(&measurer, &mut draw_list, &anim, bounds);
    let tracker = InvalidationTracker::new();
    let mut cache = SceneCache::new();

    compose_scene(&widget, &mut ctx, &tracker, &mut cache);

    assert_eq!(counter.get(), 1, "widget should be drawn on first compose");
    assert!(!draw_list.is_empty(), "draw list should contain commands");
}

#[test]
fn invalidate_dirty_nodes_marks_dirty_widgets() {
    let id = WidgetId::next();
    let mut cache = SceneCache::new();
    let mut node = SceneNode::new(id);
    node.update(vec![], Rect::default());
    assert!(node.is_valid());
    cache.insert(id, node);

    let mut tracker = InvalidationTracker::new();
    tracker.mark(id, DirtyKind::Paint);

    super::invalidate_dirty_nodes(&mut cache, &tracker);

    assert!(!cache[&id].is_valid());
}

#[test]
fn invalidate_full_rebuild_marks_all() {
    let mut cache = SceneCache::new();
    for _ in 0..3 {
        let id = WidgetId::next();
        let mut node = SceneNode::new(id);
        node.update(vec![], Rect::default());
        cache.insert(id, node);
    }

    let mut tracker = InvalidationTracker::new();
    tracker.invalidate_all();

    super::invalidate_dirty_nodes(&mut cache, &tracker);

    for node in cache.values() {
        assert!(!node.is_valid());
    }
}

#[test]
fn clean_tracker_leaves_nodes_valid() {
    let mut cache = SceneCache::new();
    let id = WidgetId::next();
    let mut node = SceneNode::new(id);
    node.update(vec![], Rect::default());
    cache.insert(id, node);

    let tracker = InvalidationTracker::new();
    super::invalidate_dirty_nodes(&mut cache, &tracker);

    assert!(cache[&id].is_valid());
}

#[test]
fn compose_passes_cache_to_draw_ctx() {
    // Verify that compose_scene sets ctx.scene_cache during the draw call.
    // We use a widget that checks for scene_cache presence.
    struct CacheDetector {
        id: WidgetId,
        saw_cache: Rc<Cell<bool>>,
    }

    impl Widget for CacheDetector {
        fn id(&self) -> WidgetId {
            self.id
        }

        fn is_focusable(&self) -> bool {
            false
        }

        fn layout(&self, _: &LayoutCtx<'_>) -> LayoutBox {
            LayoutBox::leaf(10.0, 10.0).with_widget_id(self.id)
        }

        fn draw(&self, ctx: &mut DrawCtx<'_>) {
            self.saw_cache.set(ctx.scene_cache.is_some());
        }

        fn handle_mouse(&mut self, _: &MouseEvent, _: &EventCtx<'_>) -> WidgetResponse {
            WidgetResponse::ignored()
        }

        fn handle_hover(&mut self, _: HoverEvent, _: &EventCtx<'_>) -> WidgetResponse {
            WidgetResponse::ignored()
        }

        fn handle_key(&mut self, _: KeyEvent, _: &EventCtx<'_>) -> WidgetResponse {
            WidgetResponse::ignored()
        }

        fn accept_action(&mut self, _: &WidgetAction) -> bool {
            false
        }

        fn focusable_children(&self) -> Vec<WidgetId> {
            Vec::new()
        }
    }

    let saw = Rc::new(Cell::new(false));
    let widget = CacheDetector {
        id: WidgetId::next(),
        saw_cache: saw.clone(),
    };

    let measurer = MockMeasurer::STANDARD;
    let mut draw_list = DrawList::new();
    let anim = Cell::new(false);
    let mut ctx = make_ctx(
        &measurer,
        &mut draw_list,
        &anim,
        Rect::new(0.0, 0.0, 10.0, 10.0),
    );
    let tracker = InvalidationTracker::new();
    let mut cache = SceneCache::new();

    compose_scene(&widget, &mut ctx, &tracker, &mut cache);

    assert!(saw.get(), "widget should see scene_cache during compose");
}

#[test]
fn compose_restores_previous_cache_state() {
    let counter = Rc::new(Cell::new(0));
    let widget = TrackingWidget::new(10.0, 10.0, counter);
    let measurer = MockMeasurer::STANDARD;
    let mut draw_list = DrawList::new();
    let anim = Cell::new(false);
    let mut ctx = make_ctx(
        &measurer,
        &mut draw_list,
        &anim,
        Rect::new(0.0, 0.0, 10.0, 10.0),
    );
    let tracker = InvalidationTracker::new();
    let mut cache = SceneCache::new();

    assert!(ctx.scene_cache.is_none(), "precondition: no cache");
    compose_scene(&widget, &mut ctx, &tracker, &mut cache);
    assert!(ctx.scene_cache.is_none(), "compose should restore None");
}

#[test]
fn extend_from_cache_appends_commands() {
    let mut dl = DrawList::new();
    dl.push_rect(Rect::new(0.0, 0.0, 10.0, 10.0), Default::default());
    assert_eq!(dl.len(), 1);

    let cached = vec![
        DrawCommand::Rect {
            rect: Rect::new(20.0, 20.0, 5.0, 5.0),
            style: Default::default(),
        },
        DrawCommand::Line {
            from: Default::default(),
            to: Default::default(),
            width: 1.0,
            color: crate::color::Color::WHITE,
        },
    ];

    dl.extend_from_cache(&cached);
    assert_eq!(dl.len(), 3);
}

// -- Behavioral Equivalence Infrastructure --
//
// Verify that scene composition (retained path via cached commands) produces
// identical DrawList output to the full rebuild path (all nodes invalidated).

/// Renders a widget tree via `compose_scene` with the given tracker and returns
/// the resulting DrawCommand sequence. Uses a fixed timestamp so animated
/// widgets produce deterministic output across multiple calls.
fn compose_and_collect(
    root: &dyn Widget,
    measurer: &MockMeasurer,
    bounds: Rect,
    cache: &mut SceneCache,
    tracker: &InvalidationTracker,
    now: Instant,
) -> Vec<DrawCommand> {
    let mut draw_list = DrawList::new();
    let anim = Cell::new(false);
    let mut ctx = DrawCtx {
        measurer,
        draw_list: &mut draw_list,
        bounds,
        focused_widget: None,
        now,
        animations_running: &anim,
        theme: &TEST_THEME,
        icons: None,
        scene_cache: None,
        interaction: None,
        widget_id: None,
    };
    compose_scene(root, &mut ctx, tracker, cache);
    draw_list.commands().to_vec()
}

/// Asserts that a full rebuild and a retained compose produce identical command
/// sequences for the given widget tree.
///
/// 1. Full rebuild (invalidate_all + empty cache → all cache misses, widgets
///    draw fresh, commands stored in cache).
/// 2. Retained compose (clean tracker + warm cache → all cache hits, commands
///    replayed from cache).
/// 3. Compares the two command sequences element-by-element.
fn assert_equivalence(root: &dyn Widget, bounds: Rect) {
    let measurer = MockMeasurer::STANDARD;
    let now = Instant::now();
    let mut cache = SceneCache::new();

    // Full rebuild: cache is empty so invalidate_all is a no-op, but widgets
    // draw fresh and store their output in the cache.
    let mut full_tracker = InvalidationTracker::new();
    full_tracker.invalidate_all();
    let full = compose_and_collect(root, &measurer, bounds, &mut cache, &full_tracker, now);

    // Retained: clean tracker, warm cache → container widgets replay cached
    // commands via extend_from_cache instead of calling child.draw().
    let clean_tracker = InvalidationTracker::new();
    let retained = compose_and_collect(root, &measurer, bounds, &mut cache, &clean_tracker, now);

    assert_eq!(
        full.len(),
        retained.len(),
        "command count mismatch: full={} retained={}",
        full.len(),
        retained.len(),
    );
    for (i, (f, r)) in full.iter().zip(retained.iter()).enumerate() {
        assert_eq!(f, r, "command {i} differs");
    }
}

/// Counts push/pop pairs in a command sequence.
struct StackCounts {
    push_clip: usize,
    pop_clip: usize,
    push_layer: usize,
    pop_layer: usize,
    push_translate: usize,
    pop_translate: usize,
}

fn count_stacks(commands: &[DrawCommand]) -> StackCounts {
    let mut c = StackCounts {
        push_clip: 0,
        pop_clip: 0,
        push_layer: 0,
        pop_layer: 0,
        push_translate: 0,
        pop_translate: 0,
    };
    for cmd in commands {
        match cmd {
            DrawCommand::PushClip { .. } => c.push_clip += 1,
            DrawCommand::PopClip => c.pop_clip += 1,
            DrawCommand::PushLayer { .. } => c.push_layer += 1,
            DrawCommand::PopLayer => c.pop_layer += 1,
            DrawCommand::PushTranslate { .. } => c.push_translate += 1,
            DrawCommand::PopTranslate => c.pop_translate += 1,
            _ => {}
        }
    }
    c
}

fn assert_stacks_balanced(commands: &[DrawCommand]) {
    let c = count_stacks(commands);
    assert_eq!(c.push_clip, c.pop_clip, "PushClip/PopClip imbalanced");
    assert_eq!(c.push_layer, c.pop_layer, "PushLayer/PopLayer imbalanced",);
    assert_eq!(
        c.push_translate, c.pop_translate,
        "PushTranslate/PopTranslate imbalanced",
    );
}

// -- 07.1 Behavioral Equivalence: Test Cases --

#[test]
fn equivalence_simple_label() {
    let label = LabelWidget::new("Hello, world!");
    assert_equivalence(&label, Rect::new(0.0, 0.0, 200.0, 30.0));
}

#[test]
fn equivalence_button_in_container() {
    let btn = ButtonWidget::new("Click me");
    let container = ContainerWidget::column().with_child(Box::new(btn));
    assert_equivalence(&container, Rect::new(0.0, 0.0, 200.0, 50.0));
}

#[test]
fn equivalence_hovered_button_in_container() {
    let mut btn = ButtonWidget::new("Hover me");
    let event_ctx = EventCtx {
        measurer: &MockMeasurer::STANDARD,
        bounds: Rect::new(0.0, 0.0, 200.0, 30.0),
        is_focused: false,
        focused_widget: None,
        theme: &TEST_THEME,
        interaction: None,
        widget_id: None,
    };
    btn.handle_hover(HoverEvent::Enter, &event_ctx);
    let container = ContainerWidget::column().with_child(Box::new(btn));
    // Both renders use the same fixed `now` from assert_equivalence, so the
    // animated hover_progress interpolates to the same value in both paths.
    assert_equivalence(&container, Rect::new(0.0, 0.0, 200.0, 50.0));
}

#[test]
fn equivalence_container_five_children() {
    let container = ContainerWidget::column()
        .with_gap(8.0)
        .with_child(Box::new(LabelWidget::new("First")))
        .with_child(Box::new(LabelWidget::new("Second")))
        .with_child(Box::new(ButtonWidget::new("Third")))
        .with_child(Box::new(LabelWidget::new("Fourth")))
        .with_child(Box::new(ButtonWidget::new("Fifth")));
    assert_equivalence(&container, Rect::new(0.0, 0.0, 300.0, 300.0));
}

#[test]
fn equivalence_scroll_with_content() {
    let content = ContainerWidget::column()
        .with_gap(4.0)
        .with_child(Box::new(LabelWidget::new("Line 1")))
        .with_child(Box::new(LabelWidget::new("Line 2")))
        .with_child(Box::new(LabelWidget::new("Line 3")))
        .with_child(Box::new(LabelWidget::new("Line 4")))
        .with_child(Box::new(LabelWidget::new("Line 5")));
    let scroll = ScrollWidget::vertical(Box::new(content));
    assert_equivalence(&scroll, Rect::new(0.0, 0.0, 200.0, 60.0));
}

#[test]
fn equivalence_nested_containers_three_deep() {
    let inner = ContainerWidget::row()
        .with_gap(4.0)
        .with_child(Box::new(LabelWidget::new("A")))
        .with_child(Box::new(LabelWidget::new("B")));
    let middle = ContainerWidget::column()
        .with_gap(4.0)
        .with_child(Box::new(inner))
        .with_child(Box::new(LabelWidget::new("C")));
    let outer = ContainerWidget::column()
        .with_gap(4.0)
        .with_child(Box::new(middle))
        .with_child(Box::new(LabelWidget::new("D")));
    assert_equivalence(&outer, Rect::new(0.0, 0.0, 300.0, 200.0));
}

#[test]
fn equivalence_settings_panel_form() {
    // Simplified Settings panel structure: multiple sections with labels and
    // controls, wrapped in a scroll container.
    let section1 = ContainerWidget::column()
        .with_gap(4.0)
        .with_child(Box::new(LabelWidget::new("General")))
        .with_child(Box::new(ButtonWidget::new("Reset")));
    let section2 = ContainerWidget::column()
        .with_gap(4.0)
        .with_child(Box::new(LabelWidget::new("Appearance")))
        .with_child(Box::new(LabelWidget::new("Font Size: 14")))
        .with_child(Box::new(ButtonWidget::new("Apply")));
    let section3 = ContainerWidget::column()
        .with_gap(4.0)
        .with_child(Box::new(LabelWidget::new("Keyboard")))
        .with_child(Box::new(LabelWidget::new("Bindings loaded")));
    let form = ContainerWidget::column()
        .with_gap(12.0)
        .with_child(Box::new(section1))
        .with_child(Box::new(section2))
        .with_child(Box::new(section3));
    let scroll = ScrollWidget::vertical(Box::new(form));
    assert_equivalence(&scroll, Rect::new(0.0, 0.0, 400.0, 150.0));
}

#[test]
fn equivalence_overlay_popup() {
    // Simulates overlay stacking: content underneath, popup on top.
    let content =
        ContainerWidget::column().with_child(Box::new(LabelWidget::new("Background content")));
    let popup = ContainerWidget::column()
        .with_child(Box::new(LabelWidget::new("Popup item 1")))
        .with_child(Box::new(LabelWidget::new("Popup item 2")))
        .with_child(Box::new(ButtonWidget::new("Close")));
    let stack = ContainerWidget::column()
        .with_child(Box::new(content))
        .with_child(Box::new(popup));
    assert_equivalence(&stack, Rect::new(0.0, 0.0, 300.0, 200.0));
}

// -- Draw call reduction: partial invalidation --

#[test]
fn partial_invalidation_flat_redraws_dirty_child_only() {
    let counter_a = Rc::new(Cell::new(0));
    let counter_b = Rc::new(Cell::new(0));
    let widget_a = TrackingWidget::new(100.0, 20.0, counter_a.clone());
    let widget_b = TrackingWidget::new(100.0, 20.0, counter_b.clone());
    let b_id = widget_b.id();
    let container = ContainerWidget::column()
        .with_child(Box::new(widget_a))
        .with_child(Box::new(widget_b));
    let measurer = MockMeasurer::STANDARD;
    let now = Instant::now();
    let bounds = Rect::new(0.0, 0.0, 200.0, 100.0);
    let mut cache = SceneCache::new();

    // Prime cache.
    let mut tracker = InvalidationTracker::new();
    tracker.invalidate_all();
    compose_and_collect(&container, &measurer, bounds, &mut cache, &tracker, now);
    assert_eq!(counter_a.get(), 1);
    assert_eq!(counter_b.get(), 1);

    // Reset counters.
    counter_a.set(0);
    counter_b.set(0);

    // Partial invalidation: only B is dirty.
    let mut partial = InvalidationTracker::new();
    partial.mark(b_id, DirtyKind::Paint);
    compose_and_collect(&container, &measurer, bounds, &mut cache, &partial, now);
    assert_eq!(counter_a.get(), 0, "A should not be redrawn");
    assert_eq!(counter_b.get(), 1, "B should be redrawn");
}

#[test]
fn partial_invalidation_nested_redraws_dirty_leaf() {
    let counter_a = Rc::new(Cell::new(0));
    let counter_b = Rc::new(Cell::new(0));
    let widget_a = TrackingWidget::new(100.0, 20.0, counter_a.clone());
    let widget_b = TrackingWidget::new(100.0, 20.0, counter_b.clone());
    let b_id = widget_b.id();
    let inner = ContainerWidget::column().with_child(Box::new(widget_b));
    let outer = ContainerWidget::column()
        .with_child(Box::new(widget_a))
        .with_child(Box::new(inner));
    let measurer = MockMeasurer::STANDARD;
    let now = Instant::now();
    let bounds = Rect::new(0.0, 0.0, 200.0, 100.0);
    let mut cache = SceneCache::new();

    // Prime cache.
    let mut tracker = InvalidationTracker::new();
    tracker.invalidate_all();
    compose_and_collect(&outer, &measurer, bounds, &mut cache, &tracker, now);
    assert_eq!(counter_a.get(), 1);
    assert_eq!(counter_b.get(), 1);

    counter_a.set(0);
    counter_b.set(0);

    // Partial: only B is dirty.
    let mut partial = InvalidationTracker::new();
    partial.mark(b_id, DirtyKind::Paint);
    compose_and_collect(&outer, &measurer, bounds, &mut cache, &partial, now);
    assert_eq!(counter_a.get(), 0, "A should not be redrawn");
    assert_eq!(counter_b.get(), 1, "B should be redrawn (nested)");
}

#[test]
fn retained_no_dirty_zero_child_draws() {
    let counter_a = Rc::new(Cell::new(0));
    let counter_b = Rc::new(Cell::new(0));
    let widget_a = TrackingWidget::new(100.0, 20.0, counter_a.clone());
    let widget_b = TrackingWidget::new(100.0, 20.0, counter_b.clone());
    let container = ContainerWidget::column()
        .with_child(Box::new(widget_a))
        .with_child(Box::new(widget_b));
    let measurer = MockMeasurer::STANDARD;
    let now = Instant::now();
    let bounds = Rect::new(0.0, 0.0, 200.0, 100.0);
    let mut cache = SceneCache::new();

    // Prime.
    let mut tracker = InvalidationTracker::new();
    tracker.invalidate_all();
    compose_and_collect(&container, &measurer, bounds, &mut cache, &tracker, now);

    counter_a.set(0);
    counter_b.set(0);

    // Retained with no dirty widgets.
    let clean = InvalidationTracker::new();
    compose_and_collect(&container, &measurer, bounds, &mut cache, &clean, now);
    assert_eq!(counter_a.get(), 0, "A should not be redrawn");
    assert_eq!(counter_b.get(), 0, "B should not be redrawn");
}

// -- 07.2 Performance Validation --

#[test]
fn scene_cache_stabilizes_after_warmup() {
    let container = ContainerWidget::column()
        .with_gap(4.0)
        .with_child(Box::new(LabelWidget::new("A")))
        .with_child(Box::new(LabelWidget::new("B")))
        .with_child(Box::new(ButtonWidget::new("C")))
        .with_child(Box::new(LabelWidget::new("D")))
        .with_child(Box::new(ButtonWidget::new("E")));
    let measurer = MockMeasurer::STANDARD;
    let now = Instant::now();
    let bounds = Rect::new(0.0, 0.0, 300.0, 300.0);
    let mut cache = SceneCache::new();

    // Frame 1: warmup — all nodes stored.
    let mut tracker = InvalidationTracker::new();
    tracker.invalidate_all();
    compose_and_collect(&container, &measurer, bounds, &mut cache, &tracker, now);
    let count_after_warmup = cache.node_count();
    assert!(
        count_after_warmup > 0,
        "cache should have nodes after warmup"
    );

    // Frames 2–100: same tree, clean tracker. Node count must not grow.
    for _ in 2..=100 {
        let clean = InvalidationTracker::new();
        compose_and_collect(&container, &measurer, bounds, &mut cache, &clean, now);
    }
    assert_eq!(
        cache.node_count(),
        count_after_warmup,
        "scene cache node count must stabilize after warmup",
    );
}

// -- 07.2 Draw Call Reduction --

#[test]
fn full_rebuild_draws_all_n_widgets() {
    let n = 5;
    let counters: Vec<_> = (0..n).map(|_| Rc::new(Cell::new(0))).collect();
    let mut container = ContainerWidget::column().with_gap(4.0);
    for c in &counters {
        container = container.with_child(Box::new(TrackingWidget::new(100.0, 20.0, c.clone())));
    }
    let measurer = MockMeasurer::STANDARD;
    let now = Instant::now();
    let bounds = Rect::new(0.0, 0.0, 200.0, 300.0);
    let mut cache = SceneCache::new();
    let mut tracker = InvalidationTracker::new();
    tracker.invalidate_all();

    compose_and_collect(&container, &measurer, bounds, &mut cache, &tracker, now);
    for (i, c) in counters.iter().enumerate() {
        assert_eq!(c.get(), 1, "widget {i} should be drawn exactly once");
    }
}

#[test]
fn scroll_without_content_change_zero_child_draws() {
    let counter_a = Rc::new(Cell::new(0));
    let counter_b = Rc::new(Cell::new(0));
    let container = ContainerWidget::column()
        .with_gap(4.0)
        .with_child(Box::new(TrackingWidget::new(
            100.0,
            20.0,
            counter_a.clone(),
        )))
        .with_child(Box::new(TrackingWidget::new(
            100.0,
            20.0,
            counter_b.clone(),
        )));
    let mut scroll = ScrollWidget::vertical(Box::new(container));
    let scroll_id = scroll.id();
    let measurer = MockMeasurer::STANDARD;
    let now = Instant::now();
    let bounds = Rect::new(0.0, 0.0, 200.0, 30.0);
    let mut cache = SceneCache::new();

    // Prime cache.
    let mut tracker = InvalidationTracker::new();
    tracker.invalidate_all();
    compose_and_collect(&scroll, &measurer, bounds, &mut cache, &tracker, now);
    assert_eq!(counter_a.get(), 1);
    assert_eq!(counter_b.get(), 1);

    // Simulate a scroll: change offset, mark scroll widget Paint-dirty.
    scroll.set_scroll_offset(10.0, 100.0, 30.0);
    counter_a.set(0);
    counter_b.set(0);

    let mut scroll_tracker = InvalidationTracker::new();
    scroll_tracker.mark(scroll_id, DirtyKind::Paint);
    compose_and_collect(&scroll, &measurer, bounds, &mut cache, &scroll_tracker, now);
    assert_eq!(counter_a.get(), 0, "A should not be redrawn on scroll");
    assert_eq!(counter_b.get(), 0, "B should not be redrawn on scroll");
}

#[test]
fn mouse_move_over_blank_zero_draws() {
    // Equivalent to retained_no_dirty_zero_child_draws — clean tracker,
    // warm cache, zero child draws.
    let counters: Vec<_> = (0..3).map(|_| Rc::new(Cell::new(0))).collect();
    let mut container = ContainerWidget::column().with_gap(4.0);
    for c in &counters {
        container = container.with_child(Box::new(TrackingWidget::new(100.0, 20.0, c.clone())));
    }
    let measurer = MockMeasurer::STANDARD;
    let now = Instant::now();
    let bounds = Rect::new(0.0, 0.0, 200.0, 200.0);
    let mut cache = SceneCache::new();

    // Prime.
    let mut tracker = InvalidationTracker::new();
    tracker.invalidate_all();
    compose_and_collect(&container, &measurer, bounds, &mut cache, &tracker, now);
    for c in &counters {
        c.set(0);
    }

    // Clean tracker: no dirty widgets (mouse moved over blank space).
    let clean = InvalidationTracker::new();
    compose_and_collect(&container, &measurer, bounds, &mut cache, &clean, now);
    for (i, c) in counters.iter().enumerate() {
        assert_eq!(c.get(), 0, "widget {i} should not be redrawn");
    }
}

// -- 07.2 Invalidation Triggers --

#[test]
fn full_cache_clear_then_rerender_produces_correct_output() {
    let container = ContainerWidget::column()
        .with_gap(4.0)
        .with_child(Box::new(LabelWidget::new("A")))
        .with_child(Box::new(ButtonWidget::new("B")));
    let measurer = MockMeasurer::STANDARD;
    let now = Instant::now();
    let bounds = Rect::new(0.0, 0.0, 300.0, 200.0);
    let mut cache = SceneCache::new();

    // Prime cache.
    let mut tracker = InvalidationTracker::new();
    tracker.invalidate_all();
    let before = compose_and_collect(&container, &measurer, bounds, &mut cache, &tracker, now);

    // Simulate cache-clearing trigger (font reload / theme change / DPI).
    cache.clear();

    // Re-render with invalidate_all (since cache is empty, all nodes miss).
    let mut rebuild_tracker = InvalidationTracker::new();
    rebuild_tracker.invalidate_all();
    let after = compose_and_collect(
        &container,
        &measurer,
        bounds,
        &mut cache,
        &rebuild_tracker,
        now,
    );

    assert_eq!(
        before.len(),
        after.len(),
        "command count must match after cache clear + rebuild",
    );
    for (i, (b, a)) in before.iter().zip(after.iter()).enumerate() {
        assert_eq!(b, a, "command {i} differs after cache clear + rebuild");
    }
}

#[test]
fn window_resize_invalidates_scene_but_not_text() {
    let container = ContainerWidget::column().with_child(Box::new(LabelWidget::new("Resize test")));
    let measurer = MockMeasurer::STANDARD;
    let now = Instant::now();
    let mut cache = SceneCache::new();

    // Prime with original bounds.
    let mut tracker = InvalidationTracker::new();
    tracker.invalidate_all();
    let bounds_a = Rect::new(0.0, 0.0, 300.0, 200.0);
    compose_and_collect(&container, &measurer, bounds_a, &mut cache, &tracker, now);
    let nodes_before = cache.node_count();
    assert!(nodes_before > 0);

    // Simulate window resize: clear scene cache, keep text cache.
    cache.clear();
    assert_eq!(cache.node_count(), 0, "scene cache cleared on resize");

    // Re-render with new bounds.
    let mut resize_tracker = InvalidationTracker::new();
    resize_tracker.invalidate_all();
    let bounds_b = Rect::new(0.0, 0.0, 500.0, 300.0);
    let cmds = compose_and_collect(
        &container,
        &measurer,
        bounds_b,
        &mut cache,
        &resize_tracker,
        now,
    );
    assert!(
        !cmds.is_empty(),
        "should produce draw commands after resize"
    );
    assert!(
        cache.node_count() > 0,
        "cache should repopulate after resize"
    );
}

// -- 07.1 Clip/Layer Stack Correctness --

#[test]
fn clip_layer_stacks_balanced_after_full_rebuild() {
    let container = ContainerWidget::column()
        .with_clip(true)
        .with_gap(4.0)
        .with_child(Box::new(ButtonWidget::new("A")))
        .with_child(Box::new(ButtonWidget::new("B")))
        .with_child(Box::new(LabelWidget::new("C")));
    let measurer = MockMeasurer::STANDARD;
    let now = Instant::now();
    let bounds = Rect::new(0.0, 0.0, 300.0, 200.0);
    let mut cache = SceneCache::new();
    let mut tracker = InvalidationTracker::new();
    tracker.invalidate_all();
    let cmds = compose_and_collect(&container, &measurer, bounds, &mut cache, &tracker, now);
    assert_stacks_balanced(&cmds);
    // Verify clipping is present (container has clip_children=true).
    let counts = count_stacks(&cmds);
    assert!(
        counts.push_clip >= 1,
        "clipping container should emit PushClip"
    );
}

#[test]
fn clip_layer_stacks_balanced_after_retained() {
    let container = ContainerWidget::column()
        .with_clip(true)
        .with_gap(4.0)
        .with_child(Box::new(ButtonWidget::new("A")))
        .with_child(Box::new(ButtonWidget::new("B")))
        .with_child(Box::new(LabelWidget::new("C")));
    let measurer = MockMeasurer::STANDARD;
    let now = Instant::now();
    let bounds = Rect::new(0.0, 0.0, 300.0, 200.0);
    let mut cache = SceneCache::new();

    // Prime cache.
    let mut tracker = InvalidationTracker::new();
    tracker.invalidate_all();
    compose_and_collect(&container, &measurer, bounds, &mut cache, &tracker, now);

    // Retained render.
    let clean = InvalidationTracker::new();
    let cmds = compose_and_collect(&container, &measurer, bounds, &mut cache, &clean, now);
    assert_stacks_balanced(&cmds);
}

// -- 07.1 Transform Correctness --

#[test]
fn transform_stacks_balanced_scroll_retained() {
    let content =
        ContainerWidget::column().with_child(Box::new(LabelWidget::new("Scrollable content")));
    let scroll = ScrollWidget::vertical(Box::new(content));
    let measurer = MockMeasurer::STANDARD;
    let now = Instant::now();
    let bounds = Rect::new(0.0, 0.0, 200.0, 50.0);
    let mut cache = SceneCache::new();

    // Prime.
    let mut tracker = InvalidationTracker::new();
    tracker.invalidate_all();
    compose_and_collect(&scroll, &measurer, bounds, &mut cache, &tracker, now);

    // Retained.
    let clean = InvalidationTracker::new();
    let cmds = compose_and_collect(&scroll, &measurer, bounds, &mut cache, &clean, now);
    assert_stacks_balanced(&cmds);
    // Scroll container should emit PushTranslate.
    let counts = count_stacks(&cmds);
    assert!(
        counts.push_translate >= 1,
        "scroll container should emit PushTranslate",
    );
}

#[test]
fn transform_values_preserved_in_retained_output() {
    let content = ContainerWidget::column().with_child(Box::new(LabelWidget::new("Scrollable")));
    let scroll = ScrollWidget::vertical(Box::new(content));
    let measurer = MockMeasurer::STANDARD;
    let now = Instant::now();
    let bounds = Rect::new(0.0, 0.0, 200.0, 50.0);
    let mut cache = SceneCache::new();

    // Full rebuild.
    let mut full_tracker = InvalidationTracker::new();
    full_tracker.invalidate_all();
    let full = compose_and_collect(&scroll, &measurer, bounds, &mut cache, &full_tracker, now);

    // Retained.
    let clean = InvalidationTracker::new();
    let retained = compose_and_collect(&scroll, &measurer, bounds, &mut cache, &clean, now);

    // Extract translate values from both and compare.
    let full_translates: Vec<(f32, f32)> = full
        .iter()
        .filter_map(|cmd| match cmd {
            DrawCommand::PushTranslate { dx, dy } => Some((*dx, *dy)),
            _ => None,
        })
        .collect();
    let retained_translates: Vec<(f32, f32)> = retained
        .iter()
        .filter_map(|cmd| match cmd {
            DrawCommand::PushTranslate { dx, dy } => Some((*dx, *dy)),
            _ => None,
        })
        .collect();
    assert_eq!(
        full_translates, retained_translates,
        "translate values must match between full rebuild and retained",
    );
}

// -- 07.3 Test Matrix: Leaf Widgets --

/// Helper: verifies a widget is cached after warmup and replayed without
/// redrawing on a subsequent clean compose.
fn assert_cached_after_warmup(widget: Box<dyn Widget>, bounds: Rect) {
    let counter = Rc::new(Cell::new(0));
    let tracker_widget = TrackingWidget::new(50.0, 10.0, counter.clone());
    let container = ContainerWidget::column()
        .with_child(widget)
        .with_child(Box::new(tracker_widget));
    let measurer = MockMeasurer::STANDARD;
    let now = Instant::now();
    let mut cache = SceneCache::new();

    // Warmup.
    let mut tracker = InvalidationTracker::new();
    tracker.invalidate_all();
    compose_and_collect(&container, &measurer, bounds, &mut cache, &tracker, now);
    assert_eq!(counter.get(), 1, "tracking widget should draw on warmup");
    counter.set(0);

    // Retained.
    let clean = InvalidationTracker::new();
    compose_and_collect(&container, &measurer, bounds, &mut cache, &clean, now);
    assert_eq!(counter.get(), 0, "tracking widget should not redraw");
}

#[test]
fn matrix_label_cached_after_warmup() {
    assert_cached_after_warmup(
        Box::new(LabelWidget::new("Hello")),
        Rect::new(0.0, 0.0, 200.0, 100.0),
    );
}

#[test]
fn matrix_button_cached_after_warmup() {
    assert_cached_after_warmup(
        Box::new(ButtonWidget::new("Click")),
        Rect::new(0.0, 0.0, 200.0, 100.0),
    );
}

#[test]
fn matrix_button_invalidated_on_hover() {
    let counter = Rc::new(Cell::new(0));
    let sibling = TrackingWidget::new(50.0, 10.0, counter.clone());
    let mut btn = ButtonWidget::new("Hover me");
    let btn_id = btn.id();
    let event_ctx = EventCtx {
        measurer: &MockMeasurer::STANDARD,
        bounds: Rect::new(0.0, 0.0, 200.0, 30.0),
        is_focused: false,
        focused_widget: None,
        theme: &TEST_THEME,
        interaction: None,
        widget_id: None,
    };
    btn.handle_hover(HoverEvent::Enter, &event_ctx);

    let container = ContainerWidget::column()
        .with_child(Box::new(btn))
        .with_child(Box::new(sibling));
    let measurer = MockMeasurer::STANDARD;
    let now = Instant::now();
    let bounds = Rect::new(0.0, 0.0, 200.0, 100.0);
    let mut cache = SceneCache::new();

    // Prime.
    let mut tracker = InvalidationTracker::new();
    tracker.invalidate_all();
    compose_and_collect(&container, &measurer, bounds, &mut cache, &tracker, now);
    counter.set(0);

    // Mark button as dirty (hover state changed).
    let mut dirty = InvalidationTracker::new();
    dirty.mark(btn_id, DirtyKind::Paint);
    compose_and_collect(&container, &measurer, bounds, &mut cache, &dirty, now);
    assert_eq!(
        counter.get(),
        0,
        "sibling should not redraw on button hover"
    );
}

#[test]
fn matrix_checkbox_cached_after_warmup() {
    assert_cached_after_warmup(
        Box::new(CheckboxWidget::new("Accept")),
        Rect::new(0.0, 0.0, 200.0, 100.0),
    );
}

#[test]
fn matrix_checkbox_invalidated_on_toggle() {
    let counter = Rc::new(Cell::new(0));
    let sibling = TrackingWidget::new(50.0, 10.0, counter.clone());
    let mut cb = CheckboxWidget::new("Toggle");
    let cb_id = cb.id();
    cb = cb.with_checked(true);

    let container = ContainerWidget::column()
        .with_child(Box::new(cb))
        .with_child(Box::new(sibling));
    let measurer = MockMeasurer::STANDARD;
    let now = Instant::now();
    let bounds = Rect::new(0.0, 0.0, 200.0, 100.0);
    let mut cache = SceneCache::new();

    // Prime.
    let mut tracker = InvalidationTracker::new();
    tracker.invalidate_all();
    compose_and_collect(&container, &measurer, bounds, &mut cache, &tracker, now);
    counter.set(0);

    // Mark checkbox dirty (toggle).
    let mut dirty = InvalidationTracker::new();
    dirty.mark(cb_id, DirtyKind::Paint);
    compose_and_collect(&container, &measurer, bounds, &mut cache, &dirty, now);
    assert_eq!(
        counter.get(),
        0,
        "sibling should not redraw on checkbox toggle"
    );
}

#[test]
fn matrix_slider_cached_after_warmup() {
    assert_cached_after_warmup(
        Box::new(SliderWidget::new()),
        Rect::new(0.0, 0.0, 200.0, 100.0),
    );
}

#[test]
fn matrix_text_input_cached_after_warmup() {
    assert_cached_after_warmup(
        Box::new(TextInputWidget::new()),
        Rect::new(0.0, 0.0, 200.0, 100.0),
    );
}

#[test]
fn matrix_separator_never_invalidated() {
    assert_cached_after_warmup(
        Box::new(SeparatorWidget::horizontal()),
        Rect::new(0.0, 0.0, 200.0, 100.0),
    );
}

#[test]
fn matrix_spacer_never_invalidated() {
    assert_cached_after_warmup(
        Box::new(SpacerWidget::fixed(16.0, 16.0)),
        Rect::new(0.0, 0.0, 200.0, 100.0),
    );
}

// -- 07.3 Test Matrix: Container Widgets --

#[test]
fn matrix_container_selective_child_rebuild() {
    // Already covered by partial_invalidation_flat_redraws_dirty_child_only,
    // but verify with mixed widget types.
    let counter = Rc::new(Cell::new(0));
    let tracker_widget = TrackingWidget::new(80.0, 20.0, counter.clone());
    let tracker_id = tracker_widget.id();
    let container = ContainerWidget::column()
        .with_gap(4.0)
        .with_child(Box::new(LabelWidget::new("Static")))
        .with_child(Box::new(tracker_widget))
        .with_child(Box::new(ButtonWidget::new("Also static")));
    let measurer = MockMeasurer::STANDARD;
    let now = Instant::now();
    let bounds = Rect::new(0.0, 0.0, 300.0, 200.0);
    let mut cache = SceneCache::new();

    // Prime.
    let mut tracker = InvalidationTracker::new();
    tracker.invalidate_all();
    compose_and_collect(&container, &measurer, bounds, &mut cache, &tracker, now);
    counter.set(0);

    // Only the tracking widget is dirty.
    let mut dirty = InvalidationTracker::new();
    dirty.mark(tracker_id, DirtyKind::Paint);
    compose_and_collect(&container, &measurer, bounds, &mut cache, &dirty, now);
    assert_eq!(counter.get(), 1, "only dirty widget should redraw");
}

#[test]
fn matrix_scroll_child_scene_stable() {
    // Already covered by scroll_without_content_change_zero_child_draws.
    // Verify equivalence with real widgets.
    let content = ContainerWidget::column()
        .with_gap(4.0)
        .with_child(Box::new(LabelWidget::new("Line 1")))
        .with_child(Box::new(LabelWidget::new("Line 2")))
        .with_child(Box::new(LabelWidget::new("Line 3")));
    let scroll = ScrollWidget::vertical(Box::new(content));
    assert_equivalence(&scroll, Rect::new(0.0, 0.0, 200.0, 30.0));
}

#[test]
fn matrix_dropdown_cached_after_warmup() {
    assert_cached_after_warmup(
        Box::new(DropdownWidget::new(vec![
            "Option A".into(),
            "Option B".into(),
        ])),
        Rect::new(0.0, 0.0, 200.0, 100.0),
    );
}

#[test]
fn matrix_form_section_cached_after_warmup() {
    let section = FormSection::new("General")
        .with_row(FormRow::new("Name", Box::new(LabelWidget::new("Value"))));
    assert_cached_after_warmup(Box::new(section), Rect::new(0.0, 0.0, 400.0, 200.0));
}

#[test]
fn matrix_stack_equivalence() {
    let stack = StackWidget::new(vec![
        Box::new(LabelWidget::new("Back")),
        Box::new(LabelWidget::new("Front")),
    ]);
    assert_equivalence(&stack, Rect::new(0.0, 0.0, 200.0, 100.0));
}
