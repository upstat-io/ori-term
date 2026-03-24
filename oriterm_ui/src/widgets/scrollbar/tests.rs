use crate::color::Color;
use crate::geometry::Rect;
use crate::theme::UiTheme;

use super::{
    ScrollbarAxis, ScrollbarMetrics, ScrollbarStyle, ScrollbarVisualState, compute_rects,
    drag_delta_to_offset, draw_overlay, pointer_to_offset, should_show,
};

// Style tests

#[test]
fn style_from_theme_uses_correct_tokens() {
    let theme = UiTheme::dark();
    let s = ScrollbarStyle::from_theme(&theme);

    assert_eq!(s.thumb_color, theme.border, "rest thumb = theme.border");
    assert_eq!(
        s.thumb_hover_color, theme.fg_faint,
        "hover thumb = theme.fg_faint"
    );
    assert_eq!(
        s.thumb_drag_color, theme.fg_faint,
        "drag thumb = theme.fg_faint"
    );
    assert_eq!(s.track_color, Color::TRANSPARENT, "rest track transparent");
    assert_eq!(
        s.track_hover_color,
        Color::TRANSPARENT,
        "hover track transparent"
    );
    assert_eq!(
        s.track_drag_color,
        Color::TRANSPARENT,
        "drag track transparent"
    );
}

#[test]
fn style_default_matches_dark_theme() {
    let from_dark = ScrollbarStyle::from_theme(&UiTheme::dark());
    let default = ScrollbarStyle::default();
    assert_eq!(from_dark, default);
}

#[test]
fn style_dimensions_match_mockup() {
    let s = ScrollbarStyle::default();
    assert_eq!(s.thickness, 6.0);
    assert_eq!(s.thumb_radius, 3.0);
    assert_eq!(s.min_thumb_length, 20.0);
    assert_eq!(s.edge_inset, 2.0);
    assert!(s.hit_slop > 0.0, "hit slop should provide extra grab area");
}

#[test]
fn axis_variants_are_distinct() {
    assert_ne!(ScrollbarAxis::Vertical, ScrollbarAxis::Horizontal);
}

#[test]
fn style_from_light_theme_differs_from_dark() {
    let dark = ScrollbarStyle::from_theme(&UiTheme::dark());
    let light = ScrollbarStyle::from_theme(&UiTheme::light());
    assert_ne!(dark.thumb_color, light.thumb_color);
    assert_ne!(dark.thumb_hover_color, light.thumb_hover_color);
}

// Geometry tests

fn default_style() -> ScrollbarStyle {
    ScrollbarStyle {
        thickness: 6.0,
        hit_slop: 4.0,
        edge_inset: 2.0,
        thumb_radius: 3.0,
        min_thumb_length: 20.0,
        hover_thickness: 10.0,
        thumb_color: Color::WHITE,
        thumb_hover_color: Color::WHITE,
        thumb_drag_color: Color::WHITE,
        track_color: Color::TRANSPARENT,
        track_hover_color: Color::TRANSPARENT,
        track_drag_color: Color::TRANSPARENT,
    }
}

fn viewport() -> Rect {
    Rect::new(0.0, 0.0, 400.0, 300.0)
}

#[test]
fn should_show_false_when_content_fits() {
    let m = ScrollbarMetrics {
        axis: ScrollbarAxis::Vertical,
        content_extent: 200.0,
        view_extent: 300.0,
        scroll_offset: 0.0,
    };
    assert!(!should_show(&m));
}

#[test]
fn should_show_true_when_content_overflows() {
    let m = ScrollbarMetrics {
        axis: ScrollbarAxis::Vertical,
        content_extent: 600.0,
        view_extent: 300.0,
        scroll_offset: 0.0,
    };
    assert!(should_show(&m));
}

#[test]
fn should_show_false_when_equal() {
    let m = ScrollbarMetrics {
        axis: ScrollbarAxis::Vertical,
        content_extent: 300.0,
        view_extent: 300.0,
        scroll_offset: 0.0,
    };
    assert!(!should_show(&m));
}

#[test]
fn vertical_track_thumb_rect_computation() {
    let s = default_style();
    let vp = viewport();
    let m = ScrollbarMetrics {
        axis: ScrollbarAxis::Vertical,
        content_extent: 600.0,
        view_extent: 300.0,
        scroll_offset: 0.0,
    };
    let r = compute_rects(vp, &m, &s, 0.0);

    // Track at right edge with edge_inset.
    assert_eq!(r.track.x(), vp.right() - s.thickness - s.edge_inset);
    assert_eq!(r.track.y(), vp.y());
    assert_eq!(r.track.width(), s.thickness);
    assert_eq!(r.track.height(), vp.height());

    // Thumb: 50% ratio = 150px, at top when offset=0.
    assert_eq!(r.thumb.height(), 150.0);
    assert_eq!(r.thumb.y(), vp.y());
    assert_eq!(r.thumb.width(), s.thickness);
}

#[test]
fn horizontal_track_thumb_rect_computation() {
    let s = default_style();
    let vp = viewport();
    let m = ScrollbarMetrics {
        axis: ScrollbarAxis::Horizontal,
        content_extent: 800.0,
        view_extent: 400.0,
        scroll_offset: 0.0,
    };
    let r = compute_rects(vp, &m, &s, 0.0);

    // Track at bottom edge with edge_inset.
    assert_eq!(r.track.y(), vp.bottom() - s.thickness - s.edge_inset);
    assert_eq!(r.track.x(), vp.x());
    assert_eq!(r.track.height(), s.thickness);
    assert_eq!(r.track.width(), vp.width());

    // Thumb: 50% ratio = 200px, at left when offset=0.
    assert_eq!(r.thumb.width(), 200.0);
    assert_eq!(r.thumb.x(), vp.x());
    assert_eq!(r.thumb.height(), s.thickness);
}

#[test]
fn both_axis_corner_reservation() {
    let s = default_style();
    let vp = viewport();
    let reserve = s.thickness + s.edge_inset; // 8.0

    let v_metrics = ScrollbarMetrics {
        axis: ScrollbarAxis::Vertical,
        content_extent: 600.0,
        view_extent: 300.0,
        scroll_offset: 0.0,
    };
    let h_metrics = ScrollbarMetrics {
        axis: ScrollbarAxis::Horizontal,
        content_extent: 800.0,
        view_extent: 400.0,
        scroll_offset: 0.0,
    };

    let vr = compute_rects(vp, &v_metrics, &s, reserve);
    let hr = compute_rects(vp, &h_metrics, &s, reserve);

    // Vertical track shortened from bottom by reserve amount.
    assert_eq!(vr.track.height(), vp.height() - reserve);
    // Horizontal track shortened from right by reserve amount.
    assert_eq!(hr.track.width(), vp.width() - reserve);

    // Tracks should not overlap in the corner region.
    assert!(vr.track.bottom() <= hr.track.y() + reserve);
}

#[test]
fn hit_rects_expand_beyond_visible() {
    let s = default_style();
    let vp = viewport();
    let m = ScrollbarMetrics {
        axis: ScrollbarAxis::Vertical,
        content_extent: 600.0,
        view_extent: 300.0,
        scroll_offset: 0.0,
    };
    let r = compute_rects(vp, &m, &s, 0.0);

    // Hit rect extends hit_slop to the left of the track.
    assert!(r.track_hit.width() > r.track.width());
    assert_eq!(r.track_hit.width(), s.thickness + s.hit_slop);
    assert_eq!(r.track_hit.x(), r.track.x() - s.hit_slop);

    // Thumb hit rect also wider.
    assert!(r.thumb_hit.width() > r.thumb.width());
}

#[test]
fn thumb_respects_min_length() {
    let s = default_style();
    let vp = viewport();
    // Huge content → tiny ratio → but min_thumb_length enforced.
    let m = ScrollbarMetrics {
        axis: ScrollbarAxis::Vertical,
        content_extent: 100_000.0,
        view_extent: 300.0,
        scroll_offset: 0.0,
    };
    let r = compute_rects(vp, &m, &s, 0.0);
    assert_eq!(r.thumb.height(), s.min_thumb_length);
}

#[test]
fn thumb_at_max_scroll_offset() {
    let s = default_style();
    let vp = viewport();
    let m = ScrollbarMetrics {
        axis: ScrollbarAxis::Vertical,
        content_extent: 600.0,
        view_extent: 300.0,
        scroll_offset: 300.0, // max offset
    };
    let r = compute_rects(vp, &m, &s, 0.0);

    // Thumb bottom should be at track bottom.
    let diff = (r.thumb.bottom() - r.track.bottom()).abs();
    assert!(
        diff < 1.0,
        "thumb bottom {:.1} should be at track bottom {:.1}",
        r.thumb.bottom(),
        r.track.bottom()
    );
}

#[test]
fn zero_view_extent_no_panic() {
    let s = default_style();
    let vp = Rect::new(0.0, 0.0, 400.0, 0.0);
    let m = ScrollbarMetrics {
        axis: ScrollbarAxis::Vertical,
        content_extent: 100.0,
        view_extent: 0.0,
        scroll_offset: 0.0,
    };
    let _r = compute_rects(vp, &m, &s, 0.0);
    // Should not panic or produce NaN.
}

#[test]
fn zero_content_extent_no_panic() {
    let s = default_style();
    let vp = viewport();
    let m = ScrollbarMetrics {
        axis: ScrollbarAxis::Vertical,
        content_extent: 0.0,
        view_extent: 300.0,
        scroll_offset: 0.0,
    };
    let r = compute_rects(vp, &m, &s, 0.0);
    // Thumb length should be full track (ratio = 1.0).
    assert_eq!(r.thumb.height(), r.track.height());
}

// Pointer and drag tests

#[test]
fn pointer_to_offset_maps_track_position() {
    let s = default_style();
    let vp = viewport();
    let m = ScrollbarMetrics {
        axis: ScrollbarAxis::Vertical,
        content_extent: 600.0,
        view_extent: 300.0,
        scroll_offset: 0.0,
    };
    let r = compute_rects(vp, &m, &s, 0.0);

    // Top of track → offset 0.
    let top = pointer_to_offset(r.track.y(), &r, &m);
    assert!(top.abs() < 1.0);

    // Bottom of track → max offset.
    let bottom = pointer_to_offset(r.track.bottom(), &r, &m);
    let max = m.content_extent - m.view_extent;
    assert!((bottom - max).abs() < 1.0);

    // Middle of track → half offset.
    let mid_y = r.track.y() + r.track.height() / 2.0;
    let mid = pointer_to_offset(mid_y, &r, &m);
    assert!((mid - max / 2.0).abs() < 1.0);
}

#[test]
fn drag_delta_maps_pixel_to_content() {
    let s = default_style();
    let vp = viewport();
    let m = ScrollbarMetrics {
        axis: ScrollbarAxis::Vertical,
        content_extent: 600.0,
        view_extent: 300.0,
        scroll_offset: 0.0,
    };
    let r = compute_rects(vp, &m, &s, 0.0);

    // Moving by the usable track space (track - thumb) should yield max offset.
    let max = m.content_extent - m.view_extent;
    let usable = r.track.height() - r.thumb.height();
    let delta = drag_delta_to_offset(usable, &r, &m);
    assert!(
        (delta - max).abs() < 1.0,
        "dragging full usable space ({usable}px) should yield max offset ({max}), got {delta}"
    );
}

#[test]
fn drag_delta_thumb_tracks_pointer_1_to_1() {
    let s = default_style();
    let vp = viewport();
    let m = ScrollbarMetrics {
        axis: ScrollbarAxis::Vertical,
        content_extent: 600.0,
        view_extent: 300.0,
        scroll_offset: 0.0,
    };
    let r = compute_rects(vp, &m, &s, 0.0);

    // Moving 1px on the track should move the thumb ~1px (no drift).
    // thumb_position = scroll_ratio * (track_h - thumb_h)
    // After dragging 1px: new_offset = drag_delta_to_offset(1.0, ...)
    // new_thumb_pos = (new_offset / max) * (track_h - thumb_h)
    // Should be ~1.0
    let max = m.content_extent - m.view_extent;
    let offset_delta = drag_delta_to_offset(1.0, &r, &m);
    let usable = r.track.height() - r.thumb.height();
    let thumb_move = (offset_delta / max) * usable;
    assert!(
        (thumb_move - 1.0).abs() < 0.01,
        "1px pointer movement should move thumb ~1px, got {thumb_move}"
    );
}

// Draw overlay tests

#[test]
fn draw_overlay_emits_thumb_quad() {
    use crate::draw::Scene;

    let s = default_style();
    let vp = viewport();
    let m = ScrollbarMetrics {
        axis: ScrollbarAxis::Vertical,
        content_extent: 600.0,
        view_extent: 300.0,
        scroll_offset: 0.0,
    };
    let r = compute_rects(vp, &m, &s, 0.0);

    let mut scene = Scene::default();
    draw_overlay(&mut scene, &r, &s, ScrollbarVisualState::Rest);

    // With transparent track, only 1 quad (thumb) should be emitted.
    assert_eq!(scene.quads().len(), 1);
}

#[test]
fn draw_overlay_emits_track_when_visible() {
    use crate::draw::Scene;

    let mut s = default_style();
    s.track_hover_color = Color::WHITE.with_alpha(0.1);

    let vp = viewport();
    let m = ScrollbarMetrics {
        axis: ScrollbarAxis::Vertical,
        content_extent: 600.0,
        view_extent: 300.0,
        scroll_offset: 0.0,
    };
    let r = compute_rects(vp, &m, &s, 0.0);

    let mut scene = Scene::default();
    draw_overlay(&mut scene, &r, &s, ScrollbarVisualState::Hovered);

    // Track + thumb = 2 quads.
    assert_eq!(scene.quads().len(), 2);
}

#[test]
fn visual_state_default_is_rest() {
    assert_eq!(ScrollbarVisualState::default(), ScrollbarVisualState::Rest);
}
