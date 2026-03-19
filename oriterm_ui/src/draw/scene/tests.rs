//! Unit tests for the Scene type.

use crate::color::Color;
use crate::draw::RectStyle;
use crate::geometry::{Point, Rect};
use crate::text::ShapedText;
use crate::widget_id::WidgetId;

use super::Scene;
use super::content_mask::ContentMask;

/// Empty shaped text for tests that need a `ShapedText` value.
fn empty_shaped() -> ShapedText {
    ShapedText::new(Vec::new(), 0.0, 0.0, 0.0)
}

#[test]
fn empty_scene_has_no_primitives() {
    let scene = Scene::new();
    assert!(scene.is_empty());
    assert_eq!(scene.len(), 0);
    assert!(scene.quads().is_empty());
    assert!(scene.text_runs().is_empty());
    assert!(scene.lines().is_empty());
    assert!(scene.icons().is_empty());
    assert!(scene.images().is_empty());
}

#[test]
fn empty_scene_has_no_stacks() {
    let scene = Scene::new();
    assert!(scene.clip_stack_is_empty());
    assert!(scene.offset_stack_is_empty());
    assert!(scene.layer_bg_stack_is_empty());
}

#[test]
fn push_quad_adds_to_quads_unclipped() {
    let mut scene = Scene::new();
    let bounds = Rect::new(10.0, 20.0, 100.0, 50.0);
    let style = RectStyle::filled(Color::rgb(1.0, 0.0, 0.0));
    scene.push_quad(bounds, style.clone());

    assert_eq!(scene.quads().len(), 1);
    assert_eq!(scene.len(), 1);
    assert!(!scene.is_empty());

    let quad = &scene.quads()[0];
    assert_eq!(quad.bounds, bounds);
    assert_eq!(quad.style, style);
    assert_eq!(quad.content_mask, ContentMask::unclipped());
    assert!(quad.widget_id.is_none());
}

#[test]
fn push_quad_captures_widget_id() {
    let mut scene = Scene::new();
    let id = WidgetId::next();
    scene.set_widget_id(Some(id));
    scene.push_quad(Rect::new(0.0, 0.0, 10.0, 10.0), RectStyle::default());

    assert_eq!(scene.quads()[0].widget_id, Some(id));
}

#[test]
fn push_line_offsets_by_cumulative_translation() {
    let mut scene = Scene::new();
    scene.push_offset(10.0, 20.0);

    let from = Point::new(0.0, 0.0);
    let to = Point::new(50.0, 50.0);
    scene.push_line(from, to, 2.0, Color::WHITE);

    let line = &scene.lines()[0];
    assert_eq!(line.from, Point::new(10.0, 20.0));
    assert_eq!(line.to, Point::new(60.0, 70.0));

    scene.pop_offset();
}

#[test]
fn push_clip_intersects_nested_clips() {
    let mut scene = Scene::new();
    let outer = Rect::new(0.0, 0.0, 200.0, 200.0);
    let inner = Rect::new(50.0, 50.0, 200.0, 200.0);
    scene.push_clip(outer);
    scene.push_clip(inner);

    scene.push_quad(Rect::new(0.0, 0.0, 10.0, 10.0), RectStyle::default());

    let quad = &scene.quads()[0];
    let expected = outer.intersection(inner);
    assert_eq!(quad.content_mask.clip, expected);

    scene.pop_clip();
    scene.pop_clip();
}

#[test]
fn push_offset_applies_to_quad_bounds() {
    let mut scene = Scene::new();
    scene.push_offset(100.0, 200.0);

    let bounds = Rect::new(10.0, 20.0, 30.0, 40.0);
    scene.push_quad(bounds, RectStyle::default());

    let quad = &scene.quads()[0];
    assert_eq!(quad.bounds, Rect::new(110.0, 220.0, 30.0, 40.0));

    scene.pop_offset();
}

#[test]
fn current_clip_in_content_space_subtracts_offset() {
    let mut scene = Scene::new();
    let clip = Rect::new(100.0, 100.0, 200.0, 200.0);
    scene.push_clip(clip);
    scene.push_offset(50.0, 50.0);

    let content_clip = scene.current_clip_in_content_space().unwrap();
    assert_eq!(content_clip, clip.offset(-50.0, -50.0));

    scene.pop_offset();
    scene.pop_clip();
}

#[test]
fn clear_empties_all_arrays_and_resets_stacks() {
    let mut scene = Scene::new();
    scene.push_quad(Rect::new(0.0, 0.0, 10.0, 10.0), RectStyle::default());
    scene.push_line(
        Point::new(0.0, 0.0),
        Point::new(10.0, 10.0),
        1.0,
        Color::WHITE,
    );

    assert!(!scene.is_empty());

    scene.clear();
    assert!(scene.is_empty());
    assert!(scene.clip_stack_is_empty());
    assert!(scene.offset_stack_is_empty());
    assert!(scene.layer_bg_stack_is_empty());
    assert!(scene.widget_id().is_none());
}

#[test]
fn clear_retains_vec_capacity() {
    let mut scene = Scene::new();
    for _ in 0..100 {
        scene.push_quad(Rect::new(0.0, 0.0, 10.0, 10.0), RectStyle::default());
    }
    let cap = scene.quads.capacity();
    assert!(cap >= 100);

    scene.clear();
    assert_eq!(scene.quads.capacity(), cap);
}

#[test]
fn layer_bg_captured_in_text_bg_hint() {
    let mut scene = Scene::new();
    let bg = Color::rgba(0.1, 0.2, 0.3, 1.0);
    scene.push_layer_bg(bg);

    scene.push_text(Point::new(10.0, 20.0), empty_shaped(), Color::WHITE);

    let text = &scene.text_runs()[0];
    assert_eq!(text.bg_hint, Some(bg));

    scene.pop_layer_bg();
}

#[test]
fn text_without_layer_bg_has_no_bg_hint() {
    let mut scene = Scene::new();
    scene.push_text(Point::new(10.0, 20.0), empty_shaped(), Color::WHITE);

    let text = &scene.text_runs()[0];
    assert_eq!(text.bg_hint, None);
}

#[test]
fn push_icon_with_offset_and_clip() {
    let mut scene = Scene::new();
    let clip = Rect::new(0.0, 0.0, 500.0, 500.0);
    scene.push_clip(clip);
    scene.push_offset(10.0, 20.0);

    scene.push_icon(
        Rect::new(5.0, 5.0, 16.0, 16.0),
        1,
        [0.0, 0.0, 1.0, 1.0],
        Color::WHITE,
    );

    let icon = &scene.icons()[0];
    assert_eq!(icon.rect, Rect::new(15.0, 25.0, 16.0, 16.0));
    assert_eq!(icon.content_mask.clip, clip);
    assert_eq!(icon.atlas_page, 1);

    scene.pop_offset();
    scene.pop_clip();
}

#[test]
fn push_image_with_offset() {
    let mut scene = Scene::new();
    scene.push_offset(30.0, 40.0);

    scene.push_image(Rect::new(0.0, 0.0, 64.0, 64.0), 42, [0.0, 0.0, 1.0, 1.0]);

    let img = &scene.images()[0];
    assert_eq!(img.rect, Rect::new(30.0, 40.0, 64.0, 64.0));
    assert_eq!(img.texture_id, 42);
    assert_eq!(img.content_mask, ContentMask::unclipped());

    scene.pop_offset();
}

#[test]
#[should_panic(expected = "pop_clip without matching push_clip")]
fn unbalanced_pop_clip_panics_in_debug() {
    let mut scene = Scene::new();
    scene.pop_clip();
}

#[test]
#[should_panic(expected = "pop_offset without matching push_offset")]
fn unbalanced_pop_offset_panics_in_debug() {
    let mut scene = Scene::new();
    scene.pop_offset();
}

#[test]
#[should_panic(expected = "pop_layer_bg without matching push_layer_bg")]
fn unbalanced_pop_layer_bg_panics_in_debug() {
    let mut scene = Scene::new();
    scene.pop_layer_bg();
}

#[test]
fn offset_plus_clip_interaction() {
    let mut scene = Scene::new();
    // Offset first, then clip — push_clip resolves the clip rect to
    // viewport space by applying the current offset.
    scene.push_offset(50.0, 50.0);
    let clip = Rect::new(100.0, 100.0, 200.0, 200.0);
    scene.push_clip(clip);

    scene.push_quad(Rect::new(0.0, 0.0, 10.0, 10.0), RectStyle::default());

    let quad = &scene.quads()[0];
    // Bounds offset by (50, 50).
    assert_eq!(quad.bounds, Rect::new(50.0, 50.0, 10.0, 10.0));
    // Clip was (100,100,200,200) in content space, resolved to (150,150,200,200)
    // in viewport space by push_clip applying the cumulative offset.
    assert_eq!(
        quad.content_mask.clip,
        Rect::new(150.0, 150.0, 200.0, 200.0)
    );

    scene.pop_clip();
    scene.pop_offset();
}

// --- Integration tests ---

/// Minimal widget for build_scene testing.
struct TestWidget {
    id: WidgetId,
    bounds: Rect,
}

impl crate::widgets::Widget for TestWidget {
    fn id(&self) -> WidgetId {
        self.id
    }

    fn layout(&self, _ctx: &crate::widgets::contexts::LayoutCtx<'_>) -> crate::layout::LayoutBox {
        crate::layout::LayoutBox::leaf(0.0, 0.0)
    }

    fn paint(&self, ctx: &mut crate::widgets::contexts::DrawCtx<'_>) {
        ctx.scene
            .push_quad(self.bounds, RectStyle::filled(Color::WHITE));
    }
}

#[test]
fn build_scene_produces_correct_scene() {
    use crate::draw::build_scene;
    use crate::testing::MockMeasurer;
    use crate::theme::UiTheme;

    let measurer = MockMeasurer {
        char_width: 8.0,
        line_height: 16.0,
    };
    let theme = UiTheme::default();
    let widget = TestWidget {
        id: WidgetId::next(),
        bounds: Rect::new(10.0, 20.0, 100.0, 50.0),
    };
    let mut scene = Scene::new();
    let now = std::time::Instant::now();
    let mut ctx = crate::widgets::contexts::DrawCtx {
        measurer: &measurer,
        scene: &mut scene,
        bounds: Rect::new(0.0, 0.0, 800.0, 600.0),
        now,
        theme: &theme,
        icons: None,
        interaction: None,
        widget_id: None,
        frame_requests: None,
    };

    build_scene(&widget, &mut ctx);

    assert_eq!(scene.quads().len(), 1);
    assert_eq!(scene.quads()[0].bounds, Rect::new(10.0, 20.0, 100.0, 50.0));
}

#[test]
fn build_scene_damage_cycle() {
    use crate::draw::{DamageTracker, build_scene};
    use crate::testing::MockMeasurer;
    use crate::theme::UiTheme;

    let measurer = MockMeasurer {
        char_width: 8.0,
        line_height: 16.0,
    };
    let theme = UiTheme::default();
    let mut tracker = DamageTracker::new();
    let now = std::time::Instant::now();

    // Frame 1: widget draws white quad.
    let widget = TestWidget {
        id: WidgetId::next(),
        bounds: Rect::new(0.0, 0.0, 100.0, 50.0),
    };
    let mut scene = Scene::new();
    let mut ctx = crate::widgets::contexts::DrawCtx {
        measurer: &measurer,
        scene: &mut scene,
        bounds: Rect::new(0.0, 0.0, 800.0, 600.0),
        now,
        theme: &theme,
        icons: None,
        interaction: None,
        widget_id: None,
        frame_requests: None,
    };
    build_scene(&widget, &mut ctx);
    tracker.compute_damage(&scene);
    assert!(tracker.has_damage()); // First frame always dirty.

    // Frame 2: identical — no damage.
    let mut scene2 = Scene::new();
    let mut ctx2 = crate::widgets::contexts::DrawCtx {
        measurer: &measurer,
        scene: &mut scene2,
        bounds: Rect::new(0.0, 0.0, 800.0, 600.0),
        now,
        theme: &theme,
        icons: None,
        interaction: None,
        widget_id: None,
        frame_requests: None,
    };
    build_scene(&widget, &mut ctx2);
    tracker.compute_damage(&scene2);
    assert!(!tracker.has_damage());
}
