//! Tests for scene composition.

use std::cell::Cell;
use std::rc::Rc;

use crate::draw::{DrawCommand, DrawList, SceneCache, SceneNode};
use crate::geometry::Rect;
use crate::input::{HoverEvent, KeyEvent, MouseEvent};
use crate::invalidation::{DirtyKind, InvalidationTracker};
use crate::layout::LayoutBox;
use crate::widget_id::WidgetId;
use crate::widgets::tests::{MockMeasurer, TEST_THEME};
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
        now: std::time::Instant::now(),
        animations_running: anim,
        theme: &TEST_THEME,
        icons: None,
        scene_cache: None,
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
