use std::time::{Duration, Instant};

use crate::layout::BoxContent;
use crate::sense::Sense;
use crate::widgets::tests::MockMeasurer;
use crate::widgets::{LayoutCtx, Widget};

use super::{ToggleStyle, ToggleWidget};

// -- Construction and state --

#[test]
fn default_state() {
    let t = ToggleWidget::new();
    assert!(!t.is_on());
    assert!(!t.is_disabled());
    assert!(t.is_focusable());
    assert_eq!(t.toggle_progress(), 0.0);
}

#[test]
fn with_on_builder() {
    let t = ToggleWidget::new().with_on(true);
    assert!(t.is_on());
    assert_eq!(t.toggle_progress(), 1.0);
}

#[test]
fn sense_returns_click() {
    let t = ToggleWidget::new();
    assert_eq!(t.sense(), Sense::click());
}

#[test]
fn has_two_controllers() {
    let t = ToggleWidget::new();
    assert_eq!(t.controllers().len(), 2);
}

#[test]
fn has_visual_state_animator() {
    let t = ToggleWidget::new();
    assert!(t.visual_states().is_some());
}

// -- Layout --

#[test]
fn layout_fixed_size() {
    let t = ToggleWidget::new();
    let m = MockMeasurer::new();
    let ctx = LayoutCtx {
        measurer: &m,
        theme: &super::super::tests::TEST_THEME,
    };
    let layout = t.layout(&ctx);
    let s = ToggleStyle::default();

    if let BoxContent::Leaf {
        intrinsic_width,
        intrinsic_height,
    } = &layout.content
    {
        assert_eq!(*intrinsic_width, s.width);
        assert_eq!(*intrinsic_height, s.height);
    } else {
        panic!("expected leaf layout");
    }
}

// -- Programmatic state --

#[test]
fn set_on_programmatic() {
    let mut t = ToggleWidget::new();
    t.set_on(true);
    assert!(t.is_on());
    assert_eq!(t.toggle_progress(), 1.0);
    t.set_on(false);
    assert!(!t.is_on());
    assert_eq!(t.toggle_progress(), 0.0);
}

// -- Animation --

#[test]
fn set_on_is_immediate_no_animation() {
    let mut t = ToggleWidget::new();
    let now = Instant::now();
    t.set_on(true);

    assert!(!t.toggle_progress.is_animating(now));
    assert_eq!(t.toggle_progress.get(now), 1.0);
}

#[test]
fn toggle_starts_animation() {
    let mut t = ToggleWidget::new();
    t.toggle();

    let now = Instant::now();
    assert!(t.toggle_progress.is_animating(now));
    assert_eq!(t.toggle_progress.target(), 1.0);
}

#[test]
fn animation_completes_to_target() {
    let mut t = ToggleWidget::new();
    t.toggle();

    let later = Instant::now() + Duration::from_millis(200);
    assert!(!t.toggle_progress.is_animating(later));
    assert_eq!(t.toggle_progress.get(later), 1.0);
}

#[test]
fn with_on_builder_is_immediate() {
    let t = ToggleWidget::new().with_on(true);
    let now = Instant::now();
    assert!(!t.toggle_progress.is_animating(now));
    assert_eq!(t.toggle_progress.get(now), 1.0);
}

// -- Style --

#[test]
fn with_style_applies_custom_style() {
    use crate::color::Color;

    let style = ToggleStyle {
        width: 60.0,
        height: 30.0,
        off_bg: Color::BLACK,
        off_hover_bg: Color::rgb(0.2, 0.2, 0.2),
        on_bg: Color::rgb(0.0, 1.0, 0.0),
        thumb_color: Color::rgb(0.9, 0.9, 0.9),
        thumb_padding: 4.0,
        disabled_bg: Color::rgb(0.1, 0.1, 0.1),
        disabled_thumb: Color::rgb(0.3, 0.3, 0.3),
        focus_ring_color: Color::rgb(0.0, 0.0, 1.0),
    };
    let t = ToggleWidget::new().with_style(style);

    let m = MockMeasurer::new();
    let ctx = LayoutCtx {
        measurer: &m,
        theme: &super::super::tests::TEST_THEME,
    };
    let layout = t.layout(&ctx);
    if let BoxContent::Leaf {
        intrinsic_width,
        intrinsic_height,
    } = &layout.content
    {
        assert_eq!(*intrinsic_width, 60.0);
        assert_eq!(*intrinsic_height, 30.0);
    } else {
        panic!("expected leaf layout");
    }
}

// -- Paint --

#[test]
fn toggle_animation_interpolates_thumb_position() {
    let mut t = ToggleWidget::new();
    t.toggle();
    let now = Instant::now();

    let start_progress = t.toggle_progress.get(now);
    assert!(
        start_progress < 0.1,
        "at start of toggle animation, progress should be near 0, got {start_progress}"
    );

    let after = now + Duration::from_millis(200);
    let end_progress = t.toggle_progress.get(after);
    assert_eq!(
        end_progress, 1.0,
        "after toggle animation completes, progress should be 1.0"
    );
}

#[test]
fn paint_signals_animation_while_toggling() {
    use crate::animation::FrameRequestFlags;
    use crate::draw::Scene;
    use crate::geometry::Rect;

    let mut t = ToggleWidget::new();
    t.toggle();

    let measurer = MockMeasurer::STANDARD;
    let mut scene = Scene::new();
    let bounds = Rect::new(0.0, 0.0, 40.0, 22.0);
    let flags = FrameRequestFlags::new();
    let now = Instant::now();
    let mut draw_ctx = super::super::DrawCtx {
        measurer: &measurer,
        scene: &mut scene,
        bounds,
        now,
        theme: &super::super::tests::TEST_THEME,
        icons: None,
        interaction: None,
        widget_id: None,
        frame_requests: Some(&flags),
    };
    t.paint(&mut draw_ctx);

    assert!(
        flags.anim_frame_requested(),
        "paint() should request anim frame while toggle animates"
    );
}

#[test]
fn paint_no_animation_signal_when_idle() {
    use crate::animation::FrameRequestFlags;
    use crate::draw::Scene;
    use crate::geometry::Rect;

    let t = ToggleWidget::new();

    let measurer = MockMeasurer::STANDARD;
    let mut scene = Scene::new();
    let bounds = Rect::new(0.0, 0.0, 40.0, 22.0);
    let flags = FrameRequestFlags::new();
    let now = Instant::now();
    let mut draw_ctx = super::super::DrawCtx {
        measurer: &measurer,
        scene: &mut scene,
        bounds,
        now,
        theme: &super::super::tests::TEST_THEME,
        icons: None,
        interaction: None,
        widget_id: None,
        frame_requests: Some(&flags),
    };
    t.paint(&mut draw_ctx);

    assert!(
        !flags.anim_frame_requested(),
        "paint() should not request anim frame when idle"
    );
}

#[test]
fn paint_thumb_at_on_position() {
    use crate::draw::Scene;
    use crate::geometry::Rect;

    let t = ToggleWidget::new().with_on(true);
    let style = ToggleStyle::default();

    let measurer = MockMeasurer::STANDARD;
    let mut scene = Scene::new();
    let bounds = Rect::new(0.0, 0.0, style.width, style.height);
    let now = Instant::now();
    let mut draw_ctx = super::super::DrawCtx {
        measurer: &measurer,
        scene: &mut scene,
        bounds,
        now,
        theme: &super::super::tests::TEST_THEME,
        icons: None,
        interaction: None,
        widget_id: None,
        frame_requests: None,
    };
    t.paint(&mut draw_ctx);

    let rects: Vec<_> = scene.quads().iter().map(|q| q.bounds).collect();
    assert!(rects.len() >= 2, "should have track + thumb rects");

    let thumb_rect = rects.last().unwrap();
    let thumb_diameter = style.height - style.thumb_padding * 2.0;
    let travel = style.width - style.thumb_padding * 2.0 - thumb_diameter;
    let expected_x = bounds.x() + style.thumb_padding + travel;
    assert!(
        (thumb_rect.x() - expected_x).abs() < 0.1,
        "ON state thumb x: expected {expected_x}, got {}",
        thumb_rect.x()
    );
}

#[test]
fn paint_thumb_at_off_position() {
    use crate::draw::Scene;
    use crate::geometry::Rect;

    let t = ToggleWidget::new();
    let style = ToggleStyle::default();

    let measurer = MockMeasurer::STANDARD;
    let mut scene = Scene::new();
    let bounds = Rect::new(0.0, 0.0, style.width, style.height);
    let now = Instant::now();
    let mut draw_ctx = super::super::DrawCtx {
        measurer: &measurer,
        scene: &mut scene,
        bounds,
        now,
        theme: &super::super::tests::TEST_THEME,
        icons: None,
        interaction: None,
        widget_id: None,
        frame_requests: None,
    };
    t.paint(&mut draw_ctx);

    let rects: Vec<_> = scene.quads().iter().map(|q| q.bounds).collect();
    assert!(rects.len() >= 2, "should have track + thumb rects");

    let thumb_rect = rects.last().unwrap();
    let expected_x = bounds.x() + style.thumb_padding;
    assert!(
        (thumb_rect.x() - expected_x).abs() < 0.1,
        "OFF state thumb x: expected {expected_x}, got {}",
        thumb_rect.x()
    );
}

// -- Harness integration test --

#[test]
fn harness_toggle_click_flips_value() {
    use std::time::Duration;

    use crate::action::WidgetAction;
    use crate::testing::WidgetTestHarness;

    let toggle = ToggleWidget::new();
    let toggle_id = toggle.id();
    let mut h = WidgetTestHarness::new(toggle);

    // First click -> toggled ON.
    let actions = h.click(toggle_id);
    assert!(
        actions
            .iter()
            .any(|a| matches!(a, WidgetAction::Toggled { id, value: true } if *id == toggle_id)),
        "first click should produce Toggled(true), got: {actions:?}"
    );

    // Advance clock past double-click timeout before second click.
    h.advance_time(Duration::from_millis(600));

    // Second click -> toggled OFF.
    let actions = h.click(toggle_id);
    assert!(
        actions
            .iter()
            .any(|a| matches!(a, WidgetAction::Toggled { id, value: false } if *id == toggle_id)),
        "second click should produce Toggled(false), got: {actions:?}"
    );
}
