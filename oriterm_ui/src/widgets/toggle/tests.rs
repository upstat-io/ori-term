use winit::window::CursorIcon;

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
fn sense_returns_click_and_drag() {
    let t = ToggleWidget::new();
    assert_eq!(t.sense(), Sense::click_and_drag());
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
    t.set_on(true);

    assert!(!t.toggle_progress.is_animating());
    assert_eq!(t.toggle_progress.get(), 1.0);
}

#[test]
fn toggle_starts_animation() {
    let mut t = ToggleWidget::new();
    t.toggle();

    assert!(t.toggle_progress.is_animating());
    assert_eq!(t.toggle_progress.target(), 1.0);
}

#[test]
fn animation_completes_to_target() {
    let mut t = ToggleWidget::new();
    t.toggle();

    // 150ms at 60fps = 9 frames. Tick past completion.
    for _ in 0..10 {
        t.toggle_progress.tick();
    }
    assert!(!t.toggle_progress.is_animating());
    assert_eq!(t.toggle_progress.get(), 1.0);
}

#[test]
fn with_on_builder_is_immediate() {
    let t = ToggleWidget::new().with_on(true);
    assert!(!t.toggle_progress.is_animating());
    assert_eq!(t.toggle_progress.get(), 1.0);
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
        off_thumb_color: Color::rgb(0.9, 0.9, 0.9),
        on_thumb_color: Color::rgb(0.0, 0.8, 0.0),
        thumb_padding: 4.0,
        thumb_size: 22.0,
        border_width: 2.0,
        off_border_color: Color::rgb(0.3, 0.3, 0.3),
        on_border_color: Color::rgb(0.0, 0.8, 0.0),
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

    let start_progress = t.toggle_progress.get();
    assert!(
        start_progress < 0.1,
        "at start of toggle animation, progress should be near 0, got {start_progress}"
    );

    // 150ms at 60fps = 9 frames. Tick past completion.
    for _ in 0..10 {
        t.toggle_progress.tick();
    }
    let end_progress = t.toggle_progress.get();
    assert_eq!(
        end_progress, 1.0,
        "after toggle animation completes, progress should be 1.0"
    );
}

#[test]
fn paint_signals_animation_while_toggling() {
    use std::time::Instant;

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
    use std::time::Instant;

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
    use std::time::Instant;

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
    let travel = style.width - style.thumb_padding * 2.0 - style.thumb_size;
    let expected_x = bounds.x() + style.thumb_padding + travel;
    assert!(
        (thumb_rect.x() - expected_x).abs() < 0.1,
        "ON state thumb x: expected {expected_x}, got {}",
        thumb_rect.x()
    );
}

#[test]
fn paint_thumb_at_off_position() {
    use std::time::Instant;

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

// -- on_action unit tests --

#[test]
fn on_action_click_toggles() {
    use crate::action::WidgetAction;
    use crate::geometry::{Point, Rect};

    let mut t = ToggleWidget::new();
    let bounds = Rect::new(0.0, 0.0, 40.0, 22.0);
    let center = Point::new(20.0, 11.0);

    // Simulate DragStart + DragEnd at same position (click).
    let r = t.on_action(
        WidgetAction::DragStart {
            id: t.id(),
            pos: center,
        },
        bounds,
    );
    assert!(r.is_none(), "DragStart should not emit action");
    let r = t.on_action(
        WidgetAction::DragEnd {
            id: t.id(),
            pos: center,
        },
        bounds,
    );
    assert!(
        matches!(r, Some(WidgetAction::Toggled { value: true, .. })),
        "click should toggle ON, got: {r:?}"
    );
    assert!(t.is_on());
}

#[test]
fn on_action_drag_right_turns_on() {
    use crate::action::WidgetAction;
    use crate::geometry::{Point, Rect};

    let mut t = ToggleWidget::new();
    let bounds = Rect::new(0.0, 0.0, 40.0, 22.0);

    // Start at left edge, drag to right.
    let start = Point::new(5.0, 11.0);
    let end = Point::new(35.0, 11.0);
    let delta = Point::new(end.x - start.x, 0.0);

    t.on_action(
        WidgetAction::DragStart {
            id: t.id(),
            pos: start,
        },
        bounds,
    );
    t.on_action(
        WidgetAction::DragUpdate {
            id: t.id(),
            delta,
            total_delta: delta,
        },
        bounds,
    );
    let r = t.on_action(
        WidgetAction::DragEnd {
            id: t.id(),
            pos: end,
        },
        bounds,
    );
    assert!(
        matches!(r, Some(WidgetAction::Toggled { value: true, .. })),
        "drag right should toggle ON, got: {r:?}"
    );
    assert!(t.is_on());
}

#[test]
fn on_action_drag_left_turns_off() {
    use crate::action::WidgetAction;
    use crate::geometry::{Point, Rect};

    let mut t = ToggleWidget::new().with_on(true);
    let bounds = Rect::new(0.0, 0.0, 40.0, 22.0);

    // Start at right edge, drag to left.
    let start = Point::new(35.0, 11.0);
    let end = Point::new(5.0, 11.0);
    let delta = Point::new(end.x - start.x, 0.0);

    t.on_action(
        WidgetAction::DragStart {
            id: t.id(),
            pos: start,
        },
        bounds,
    );
    t.on_action(
        WidgetAction::DragUpdate {
            id: t.id(),
            delta,
            total_delta: delta,
        },
        bounds,
    );
    let r = t.on_action(
        WidgetAction::DragEnd {
            id: t.id(),
            pos: end,
        },
        bounds,
    );
    assert!(
        matches!(r, Some(WidgetAction::Toggled { value: false, .. })),
        "drag left should toggle OFF, got: {r:?}"
    );
    assert!(!t.is_on());
}

#[test]
fn on_action_small_drag_treated_as_click() {
    use crate::action::WidgetAction;
    use crate::geometry::{Point, Rect};

    let mut t = ToggleWidget::new();
    let bounds = Rect::new(0.0, 0.0, 40.0, 22.0);

    // Small drag (< half travel) in the OFF zone — should toggle like a click.
    let start = Point::new(15.0, 11.0);
    let end = Point::new(10.0, 11.0);
    let delta = Point::new(end.x - start.x, 0.0);

    t.on_action(
        WidgetAction::DragStart {
            id: t.id(),
            pos: start,
        },
        bounds,
    );
    t.on_action(
        WidgetAction::DragUpdate {
            id: t.id(),
            delta,
            total_delta: delta,
        },
        bounds,
    );
    let r = t.on_action(
        WidgetAction::DragEnd {
            id: t.id(),
            pos: end,
        },
        bounds,
    );
    assert!(
        matches!(r, Some(WidgetAction::Toggled { value: true, .. })),
        "small drag should toggle like a click, got: {r:?}"
    );
    assert!(t.is_on());
}

#[test]
fn on_action_long_drag_snap_back_when_same_state() {
    use crate::action::WidgetAction;
    use crate::geometry::{Point, Rect};

    // Toggle is ON, drag from right past midpoint then release in ON zone.
    // Travel = 18px, so movement must be >= 9px to be a positional drag.
    let mut t = ToggleWidget::new().with_on(true);
    let bounds = Rect::new(0.0, 0.0, 40.0, 22.0);

    // Start at left edge (unusual), drag 10px right — ends in ON zone.
    let start = Point::new(5.0, 11.0);
    let end = Point::new(35.0, 11.0);
    let delta = Point::new(end.x - start.x, 0.0);

    t.on_action(
        WidgetAction::DragStart {
            id: t.id(),
            pos: start,
        },
        bounds,
    );
    t.on_action(
        WidgetAction::DragUpdate {
            id: t.id(),
            delta,
            total_delta: delta,
        },
        bounds,
    );
    let r = t.on_action(
        WidgetAction::DragEnd {
            id: t.id(),
            pos: end,
        },
        bounds,
    );
    // Already ON, drag ends in ON zone → snap back, no action.
    assert!(
        r.is_none(),
        "long drag ending in same state should snap back, got: {r:?}"
    );
    assert!(t.is_on());
}

// -- Harness integration tests --

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

#[test]
fn harness_toggle_drag_right_turns_on() {
    use crate::action::WidgetAction;
    use crate::geometry::Point;
    use crate::testing::WidgetTestHarness;

    let toggle = ToggleWidget::new();
    let toggle_id = toggle.id();
    let mut h = WidgetTestHarness::new(toggle);

    // Drag from left to right across the toggle.
    let start = Point::new(5.0, 11.0);
    let end = Point::new(35.0, 11.0);
    let actions = h.drag(start, end, 5);
    assert!(
        actions
            .iter()
            .any(|a| matches!(a, WidgetAction::Toggled { id, value: true } if *id == toggle_id)),
        "drag right should produce Toggled(true), got: {actions:?}"
    );
}

// -- Geometry invariants (section 13.6) --

#[test]
fn toggle_thumb_size_is_12px() {
    let style = ToggleStyle::from_theme(&crate::theme::UiTheme::dark());
    assert_eq!(style.thumb_size, 12.0);
}

#[test]
fn toggle_travel_is_20px() {
    let style = ToggleStyle::from_theme(&crate::theme::UiTheme::dark());
    // travel = width - 2*thumb_padding - thumb_size = 38 - 6 - 12 = 20.
    let travel = style.width - 2.0 * style.thumb_padding - style.thumb_size;
    assert_eq!(travel, 20.0);
}

#[test]
fn toggle_off_thumb_position() {
    let style = ToggleStyle::from_theme(&crate::theme::UiTheme::dark());
    // When off, thumb sits at x = thumb_padding.
    let expected_x = style.thumb_padding;
    assert_eq!(expected_x, 3.0);
}

#[test]
fn toggle_on_thumb_position() {
    let style = ToggleStyle::from_theme(&crate::theme::UiTheme::dark());
    let travel = style.width - 2.0 * style.thumb_padding - style.thumb_size;
    // When on, thumb sits at x = thumb_padding + travel = 3 + 20 = 23.
    let expected_x = style.thumb_padding + travel;
    assert_eq!(expected_x, 23.0);
}

#[test]
fn toggle_outer_size_38x20() {
    let style = ToggleStyle::from_theme(&crate::theme::UiTheme::dark());
    assert_eq!(style.width, 38.0);
    assert_eq!(style.height, 20.0);
}

// -- Cursor icon --

#[test]
fn layout_cursor_icon_pointer() {
    let t = ToggleWidget::new();
    let m = MockMeasurer::new();
    let ctx = LayoutCtx {
        measurer: &m,
        theme: &super::super::tests::TEST_THEME,
    };
    let layout = t.layout(&ctx);
    assert_eq!(
        layout.cursor_icon,
        CursorIcon::Pointer,
        "toggle should declare Pointer cursor"
    );
}
