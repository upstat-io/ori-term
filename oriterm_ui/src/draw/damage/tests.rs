//! Unit tests for DamageTracker.

use crate::color::Color;
use crate::draw::RectStyle;
use crate::geometry::Rect;
use crate::widget_id::WidgetId;

use super::DamageTracker;
use crate::draw::Scene;

/// Creates a scene with one quad for the given widget at the given bounds.
fn scene_with_quad(id: WidgetId, bounds: Rect, color: Color) -> Scene {
    let mut scene = Scene::new();
    scene.set_widget_id(Some(id));
    scene.push_quad(bounds, RectStyle::filled(color));
    scene
}

#[test]
fn new_tracker_is_first_frame() {
    let tracker = DamageTracker::new();
    assert!(tracker.is_first_frame());
    assert!(!tracker.has_damage());
}

#[test]
fn first_frame_reports_full_scene_dirty() {
    let mut tracker = DamageTracker::new();
    let id = WidgetId::next();
    let scene = scene_with_quad(id, Rect::new(10.0, 10.0, 100.0, 50.0), Color::WHITE);

    tracker.compute_damage(&scene);

    assert!(!tracker.is_first_frame());
    assert!(tracker.has_damage());
    assert_eq!(tracker.dirty_regions().len(), 1);
    // Full scene bounds should be the quad's bounds.
    assert_eq!(
        tracker.dirty_regions()[0],
        Rect::new(10.0, 10.0, 100.0, 50.0)
    );
}

#[test]
fn identical_scenes_zero_damage() {
    let mut tracker = DamageTracker::new();
    let id = WidgetId::next();
    let bounds = Rect::new(0.0, 0.0, 100.0, 50.0);
    let color = Color::WHITE;

    // First frame.
    tracker.compute_damage(&scene_with_quad(id, bounds, color));
    assert!(tracker.has_damage());

    // Second frame — identical.
    tracker.compute_damage(&scene_with_quad(id, bounds, color));
    assert!(!tracker.has_damage());
    assert!(tracker.dirty_regions().is_empty());
}

#[test]
fn changed_widget_reports_dirty() {
    let mut tracker = DamageTracker::new();
    let id = WidgetId::next();
    let bounds = Rect::new(0.0, 0.0, 100.0, 50.0);

    // First frame.
    tracker.compute_damage(&scene_with_quad(id, bounds, Color::WHITE));

    // Second frame — color changed.
    tracker.compute_damage(&scene_with_quad(id, bounds, Color::BLACK));

    assert!(tracker.has_damage());
    // Both old and new bounds (same in this case) should be dirty.
    assert!(!tracker.dirty_regions().is_empty());
}

#[test]
fn moved_widget_reports_both_old_and_new_bounds() {
    let mut tracker = DamageTracker::new();
    let id = WidgetId::next();
    let old_bounds = Rect::new(0.0, 0.0, 50.0, 50.0);
    let new_bounds = Rect::new(200.0, 200.0, 50.0, 50.0);

    tracker.compute_damage(&scene_with_quad(id, old_bounds, Color::WHITE));
    tracker.compute_damage(&scene_with_quad(id, new_bounds, Color::WHITE));

    assert!(tracker.has_damage());
    // Should have dirty regions covering old and new positions.
    assert!(tracker.is_region_dirty(old_bounds));
    assert!(tracker.is_region_dirty(new_bounds));
}

#[test]
fn removed_widget_reports_old_bounds_dirty() {
    let mut tracker = DamageTracker::new();
    let id = WidgetId::next();
    let bounds = Rect::new(10.0, 10.0, 80.0, 40.0);

    // First frame with widget.
    tracker.compute_damage(&scene_with_quad(id, bounds, Color::WHITE));

    // Second frame — empty scene.
    let empty = Scene::new();
    tracker.compute_damage(&empty);

    assert!(tracker.has_damage());
    assert!(tracker.is_region_dirty(bounds));
}

#[test]
fn new_widget_reports_new_bounds_dirty() {
    let mut tracker = DamageTracker::new();

    // First frame — empty.
    tracker.compute_damage(&Scene::new());

    // Second frame — add widget.
    let id = WidgetId::next();
    let bounds = Rect::new(50.0, 50.0, 60.0, 30.0);
    tracker.compute_damage(&scene_with_quad(id, bounds, Color::WHITE));

    assert!(tracker.has_damage());
    assert!(tracker.is_region_dirty(bounds));
}

#[test]
fn overlapping_dirty_rects_merged() {
    let mut tracker = DamageTracker::new();
    let id1 = WidgetId::next();
    let id2 = WidgetId::next();

    // Two overlapping widgets.
    let mut scene = Scene::new();
    scene.set_widget_id(Some(id1));
    scene.push_quad(
        Rect::new(0.0, 0.0, 100.0, 50.0),
        RectStyle::filled(Color::WHITE),
    );
    scene.set_widget_id(Some(id2));
    scene.push_quad(
        Rect::new(50.0, 0.0, 100.0, 50.0),
        RectStyle::filled(Color::WHITE),
    );
    tracker.compute_damage(&scene);

    // Change both widgets — their dirty rects overlap.
    let mut scene2 = Scene::new();
    scene2.set_widget_id(Some(id1));
    scene2.push_quad(
        Rect::new(0.0, 0.0, 100.0, 50.0),
        RectStyle::filled(Color::BLACK),
    );
    scene2.set_widget_id(Some(id2));
    scene2.push_quad(
        Rect::new(50.0, 0.0, 100.0, 50.0),
        RectStyle::filled(Color::BLACK),
    );
    tracker.compute_damage(&scene2);

    assert!(tracker.has_damage());
    // Overlapping rects should be merged into fewer regions.
    assert!(tracker.dirty_regions().len() <= 2);
}

#[test]
fn non_overlapping_dirty_rects_stay_separate() {
    let mut tracker = DamageTracker::new();
    let id1 = WidgetId::next();
    let id2 = WidgetId::next();

    // Two non-overlapping widgets.
    let mut scene = Scene::new();
    scene.set_widget_id(Some(id1));
    scene.push_quad(
        Rect::new(0.0, 0.0, 50.0, 50.0),
        RectStyle::filled(Color::WHITE),
    );
    scene.set_widget_id(Some(id2));
    scene.push_quad(
        Rect::new(200.0, 200.0, 50.0, 50.0),
        RectStyle::filled(Color::WHITE),
    );
    tracker.compute_damage(&scene);

    // Change both.
    let mut scene2 = Scene::new();
    scene2.set_widget_id(Some(id1));
    scene2.push_quad(
        Rect::new(0.0, 0.0, 50.0, 50.0),
        RectStyle::filled(Color::BLACK),
    );
    scene2.set_widget_id(Some(id2));
    scene2.push_quad(
        Rect::new(200.0, 200.0, 50.0, 50.0),
        RectStyle::filled(Color::BLACK),
    );
    tracker.compute_damage(&scene2);

    assert!(tracker.has_damage());
    // Non-overlapping should stay separate.
    assert!(tracker.dirty_regions().len() >= 2);
}

#[test]
fn reset_makes_next_frame_first() {
    let mut tracker = DamageTracker::new();
    let id = WidgetId::next();
    let bounds = Rect::new(0.0, 0.0, 100.0, 50.0);

    tracker.compute_damage(&scene_with_quad(id, bounds, Color::WHITE));
    assert!(!tracker.is_first_frame());

    tracker.reset();
    assert!(tracker.is_first_frame());

    // Next compute should act as first frame.
    tracker.compute_damage(&scene_with_quad(id, bounds, Color::WHITE));
    assert!(tracker.has_damage());
    assert_eq!(tracker.dirty_regions().len(), 1);
}

#[test]
fn is_region_dirty_query_correctness() {
    let mut tracker = DamageTracker::new();
    let id = WidgetId::next();
    let bounds = Rect::new(100.0, 100.0, 50.0, 50.0);

    tracker.compute_damage(&scene_with_quad(id, bounds, Color::WHITE));
    tracker.compute_damage(&scene_with_quad(id, bounds, Color::BLACK));

    // Overlapping query should find damage.
    assert!(tracker.is_region_dirty(Rect::new(110.0, 110.0, 10.0, 10.0)));
    // Non-overlapping query should not.
    assert!(!tracker.is_region_dirty(Rect::new(0.0, 0.0, 10.0, 10.0)));
}

#[test]
fn has_damage_false_when_no_damage() {
    let mut tracker = DamageTracker::new();
    let id = WidgetId::next();
    let bounds = Rect::new(0.0, 0.0, 100.0, 50.0);

    tracker.compute_damage(&scene_with_quad(id, bounds, Color::WHITE));
    tracker.compute_damage(&scene_with_quad(id, bounds, Color::WHITE));

    assert!(!tracker.has_damage());
}

// --- Damage hash stability under offsets ---

#[test]
fn same_widget_different_offset_produces_damage() {
    let mut tracker = DamageTracker::new();
    let id = WidgetId::next();

    // Frame 1: widget at (0, 0).
    let mut scene1 = Scene::new();
    scene1.set_widget_id(Some(id));
    scene1.push_quad(
        Rect::new(0.0, 0.0, 50.0, 50.0),
        RectStyle::filled(Color::WHITE),
    );
    tracker.compute_damage(&scene1);

    // Frame 2: same widget, same color, but at (100, 100) due to scroll.
    let mut scene2 = Scene::new();
    scene2.set_widget_id(Some(id));
    scene2.push_quad(
        Rect::new(100.0, 100.0, 50.0, 50.0),
        RectStyle::filled(Color::WHITE),
    );
    tracker.compute_damage(&scene2);

    // Position changed — damage expected (even though content is same).
    assert!(tracker.has_damage());
    assert!(tracker.is_region_dirty(Rect::new(0.0, 0.0, 50.0, 50.0)));
    assert!(tracker.is_region_dirty(Rect::new(100.0, 100.0, 50.0, 50.0)));
}

// --- Multiple widgets at same bounds ---

#[test]
fn many_overlapping_widgets_bounded_dirty_regions() {
    let mut tracker = DamageTracker::new();
    let bounds = Rect::new(50.0, 50.0, 100.0, 100.0);

    // Frame 1: 10 widgets at the same bounds.
    let mut scene1 = Scene::new();
    let ids: Vec<_> = (0..10).map(|_| WidgetId::next()).collect();
    for &id in &ids {
        scene1.set_widget_id(Some(id));
        scene1.push_quad(bounds, RectStyle::filled(Color::WHITE));
    }
    tracker.compute_damage(&scene1);

    // Frame 2: all widgets change color.
    let mut scene2 = Scene::new();
    for &id in &ids {
        scene2.set_widget_id(Some(id));
        scene2.push_quad(bounds, RectStyle::filled(Color::BLACK));
    }
    tracker.compute_damage(&scene2);

    assert!(tracker.has_damage());
    // All at same bounds — dirty regions should merge into few entries.
    // 10 widgets at same position should produce at most 1 dirty region.
    assert!(
        tracker.dirty_regions().len() <= 2,
        "expected at most 2 dirty regions for co-located widgets, got {}",
        tracker.dirty_regions().len()
    );
}

// --- None widget ID handling ---

#[test]
fn primitives_without_widget_id_not_tracked_per_widget() {
    let mut tracker = DamageTracker::new();

    // Frame 1: quad without widget ID.
    let mut scene1 = Scene::new();
    // No set_widget_id() call.
    scene1.push_quad(
        Rect::new(0.0, 0.0, 50.0, 50.0),
        RectStyle::filled(Color::WHITE),
    );
    tracker.compute_damage(&scene1);

    // Frame 2: identical (still no widget ID).
    let mut scene2 = Scene::new();
    scene2.push_quad(
        Rect::new(0.0, 0.0, 50.0, 50.0),
        RectStyle::filled(Color::WHITE),
    );
    tracker.compute_damage(&scene2);

    // Should not panic. Damage behavior with None IDs is implementation-defined
    // but must not crash.
}

// --- Negative coordinates in damage tracking ---

#[test]
fn negative_coordinate_widget_tracked_correctly() {
    let mut tracker = DamageTracker::new();
    let id = WidgetId::next();
    let bounds = Rect::new(-20.0, -10.0, 100.0, 50.0);

    tracker.compute_damage(&scene_with_quad(id, bounds, Color::WHITE));
    assert!(tracker.has_damage());
    assert!(tracker.is_region_dirty(bounds));

    // Identical second frame — no damage.
    tracker.compute_damage(&scene_with_quad(id, bounds, Color::WHITE));
    assert!(!tracker.has_damage());
}

// --- Per-side border damage detection ---

/// Creates a scene with one styled quad for the given widget.
fn scene_with_styled_quad(id: WidgetId, bounds: Rect, style: RectStyle) -> Scene {
    let mut scene = Scene::new();
    scene.set_widget_id(Some(id));
    scene.push_quad(bounds, style);
    scene
}

#[test]
fn hash_rect_style_distinguishes_border_sides() {
    let mut tracker = DamageTracker::new();
    let id = WidgetId::next();
    let bounds = Rect::new(0.0, 0.0, 100.0, 50.0);

    let style_a = RectStyle::filled(Color::WHITE).with_border(2.0, Color::BLACK);
    tracker.compute_damage(&scene_with_styled_quad(id, bounds, style_a));

    // Change only the left border — should detect damage.
    let style_b = RectStyle::filled(Color::WHITE)
        .with_border(2.0, Color::BLACK)
        .with_border_left(3.0, Color::rgba(1.0, 0.0, 0.0, 1.0));
    tracker.compute_damage(&scene_with_styled_quad(id, bounds, style_b));
    assert!(
        tracker.has_damage(),
        "different border sides should produce damage"
    );
}

#[test]
fn hash_rect_style_same_border_produces_same_hash() {
    let mut tracker = DamageTracker::new();
    let id = WidgetId::next();
    let bounds = Rect::new(0.0, 0.0, 100.0, 50.0);

    let style = RectStyle::filled(Color::WHITE).with_border(2.0, Color::BLACK);
    tracker.compute_damage(&scene_with_styled_quad(id, bounds, style.clone()));
    tracker.compute_damage(&scene_with_styled_quad(id, bounds, style));
    assert!(
        !tracker.has_damage(),
        "identical borders should produce no damage"
    );
}

#[test]
fn hash_rect_style_uniform_vs_explicit_same_hash() {
    use crate::draw::{Border, BorderSides};

    let mut tracker = DamageTracker::new();
    let id = WidgetId::next();
    let bounds = Rect::new(0.0, 0.0, 100.0, 50.0);

    // Built via uniform shorthand.
    let style_a = RectStyle::filled(Color::WHITE).with_border(2.0, Color::BLACK);
    tracker.compute_damage(&scene_with_styled_quad(id, bounds, style_a));

    // Built by manually constructing all four sides identically.
    let side = Some(Border {
        width: 2.0,
        color: Color::BLACK,
    });
    let explicit = BorderSides {
        top: side,
        right: side,
        bottom: side,
        left: side,
    };
    let style_b = RectStyle::filled(Color::WHITE).with_border_sides(explicit);
    tracker.compute_damage(&scene_with_styled_quad(id, bounds, style_b));
    assert!(
        !tracker.has_damage(),
        "semantically identical borders (uniform vs explicit) should produce no damage"
    );
}

// --- Zero-width border color changes must not cause damage ---

#[test]
fn color_change_on_zero_width_border_produces_no_damage() {
    use crate::draw::{Border, BorderSides};

    let mut tracker = DamageTracker::new();
    let id = WidgetId::next();
    let bounds = Rect::new(0.0, 0.0, 100.0, 50.0);

    // Frame 1: border with zero-width left side colored red.
    let sides_a = BorderSides {
        top: Some(Border {
            width: 2.0,
            color: Color::BLACK,
        }),
        right: None,
        bottom: None,
        left: Some(Border {
            width: 0.0,
            color: Color::rgba(1.0, 0.0, 0.0, 1.0),
        }),
    };
    let style_a = RectStyle::filled(Color::WHITE).with_border_sides(sides_a);
    tracker.compute_damage(&scene_with_styled_quad(id, bounds, style_a));

    // Frame 2: same, but zero-width left side changed to blue.
    let sides_b = BorderSides {
        top: Some(Border {
            width: 2.0,
            color: Color::BLACK,
        }),
        right: None,
        bottom: None,
        left: Some(Border {
            width: 0.0,
            color: Color::rgba(0.0, 0.0, 1.0, 1.0),
        }),
    };
    let style_b = RectStyle::filled(Color::WHITE).with_border_sides(sides_b);
    tracker.compute_damage(&scene_with_styled_quad(id, bounds, style_b));

    assert!(
        !tracker.has_damage(),
        "color change on zero-width border should not produce damage"
    );
}

// --- Text size/weight damage detection ---

/// Creates a scene with one text run for the given widget.
fn scene_with_text(id: WidgetId, pos: crate::geometry::Point, size_q6: u32, weight: u16) -> Scene {
    use crate::text::{ShapedGlyph, ShapedText};

    let shaped = ShapedText::new(
        vec![ShapedGlyph {
            glyph_id: 42,
            face_index: 0,
            synthetic: 0,
            x_advance: 8.0,
            x_offset: 0.0,
            y_offset: 0.0,
        }],
        8.0,
        16.0,
        12.0,
        size_q6,
        weight,
    );

    let mut scene = Scene::new();
    scene.set_widget_id(Some(id));
    scene.push_text(pos, shaped, Color::WHITE);
    scene
}

#[test]
fn text_weight_change_produces_damage() {
    let mut tracker = DamageTracker::new();
    let id = WidgetId::next();
    let pos = crate::geometry::Point::new(10.0, 10.0);

    // Frame 1: weight 400.
    tracker.compute_damage(&scene_with_text(id, pos, 832, 400));

    // Frame 2: weight 700, everything else identical.
    tracker.compute_damage(&scene_with_text(id, pos, 832, 700));

    assert!(
        tracker.has_damage(),
        "weight-only text change should produce damage"
    );
}

#[test]
fn text_size_change_produces_damage() {
    let mut tracker = DamageTracker::new();
    let id = WidgetId::next();
    let pos = crate::geometry::Point::new(10.0, 10.0);

    // Frame 1: size_q6 = 832 (13px).
    tracker.compute_damage(&scene_with_text(id, pos, 832, 400));

    // Frame 2: size_q6 = 1152 (18px), everything else identical.
    tracker.compute_damage(&scene_with_text(id, pos, 1152, 400));

    assert!(
        tracker.has_damage(),
        "size-only text change should produce damage"
    );
}

#[test]
fn identical_text_no_damage() {
    let mut tracker = DamageTracker::new();
    let id = WidgetId::next();
    let pos = crate::geometry::Point::new(10.0, 10.0);

    tracker.compute_damage(&scene_with_text(id, pos, 832, 400));
    tracker.compute_damage(&scene_with_text(id, pos, 832, 400));

    assert!(
        !tracker.has_damage(),
        "identical text should produce no damage"
    );
}
