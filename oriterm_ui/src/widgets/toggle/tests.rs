use std::time::{Duration, Instant};

use crate::geometry::{Point, Rect};
use crate::input::{HoverEvent, Key, KeyEvent, Modifiers, MouseButton, MouseEvent, MouseEventKind};
use crate::layout::BoxContent;
use crate::sense::Sense;
use crate::widgets::tests::MockMeasurer;
use crate::widgets::{CaptureRequest, EventCtx, LayoutCtx, Widget, WidgetAction, WidgetResponse};

use super::{ToggleStyle, ToggleWidget};

static MEASURER: MockMeasurer = MockMeasurer::STANDARD;

fn event_ctx() -> EventCtx<'static> {
    EventCtx {
        measurer: &MEASURER,
        bounds: Rect::new(0.0, 0.0, 40.0, 22.0),
        is_focused: true,
        focused_widget: None,
        theme: &super::super::tests::TEST_THEME,
        interaction: None,
        widget_id: None,
        frame_requests: None,
    }
}

fn left_down() -> MouseEvent {
    MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        pos: Point::new(10.0, 10.0),
        modifiers: Modifiers::NONE,
    }
}

fn left_up() -> MouseEvent {
    MouseEvent {
        kind: MouseEventKind::Up(MouseButton::Left),
        pos: Point::new(10.0, 10.0),
        modifiers: Modifiers::NONE,
    }
}

fn space_key() -> KeyEvent {
    KeyEvent {
        key: Key::Space,
        modifiers: Modifiers::NONE,
    }
}

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

// -- Click and keyboard toggle (legacy compat) --

#[test]
fn click_toggles() {
    let mut t = ToggleWidget::new();
    let ctx = event_ctx();

    let r = t.handle_mouse(&left_down(), &ctx);
    assert_eq!(r.capture, CaptureRequest::Acquire);
    let r = t.handle_mouse(&left_up(), &ctx);
    assert!(t.is_on());
    assert_eq!(t.toggle_progress(), 1.0);
    assert_eq!(r.capture, CaptureRequest::Release);
    assert_eq!(
        r.action,
        Some(WidgetAction::Toggled {
            id: t.id(),
            value: true,
        })
    );

    let r = t.handle_mouse(&left_down(), &ctx);
    assert_eq!(r.capture, CaptureRequest::Acquire);
    let r = t.handle_mouse(&left_up(), &ctx);
    assert!(!t.is_on());
    assert_eq!(t.toggle_progress(), 0.0);
    assert_eq!(r.capture, CaptureRequest::Release);
    assert_eq!(
        r.action,
        Some(WidgetAction::Toggled {
            id: t.id(),
            value: false,
        })
    );
}

#[test]
fn space_toggles() {
    let mut t = ToggleWidget::new();
    let ctx = event_ctx();

    let r = t.handle_key(space_key(), &ctx);
    assert!(t.is_on());
    assert_eq!(
        r.action,
        Some(WidgetAction::Toggled {
            id: t.id(),
            value: true,
        })
    );
}

#[test]
fn disabled_ignores() {
    let mut t = ToggleWidget::new().with_disabled(true);
    let ctx = event_ctx();

    assert!(!t.is_focusable());

    let r = t.handle_mouse(&left_down(), &ctx);
    assert_eq!(r, WidgetResponse::ignored());
    let r = t.handle_mouse(&left_up(), &ctx);
    assert_eq!(r, WidgetResponse::ignored());

    let r = t.handle_key(space_key(), &ctx);
    assert_eq!(r, WidgetResponse::ignored());

    let r = t.handle_hover(HoverEvent::Enter, &ctx);
    assert_eq!(r, WidgetResponse::ignored());
}

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

#[test]
fn enter_key_does_not_toggle() {
    let mut t = ToggleWidget::new();
    let ctx = event_ctx();

    let r = t.handle_key(
        KeyEvent {
            key: Key::Enter,
            modifiers: Modifiers::NONE,
        },
        &ctx,
    );
    assert_eq!(r, WidgetResponse::ignored());
    assert!(!t.is_on());
}

#[test]
fn right_click_ignored() {
    let mut t = ToggleWidget::new();
    let ctx = event_ctx();

    let right_click = MouseEvent {
        kind: MouseEventKind::Up(MouseButton::Right),
        pos: Point::new(10.0, 10.0),
        modifiers: Modifiers::NONE,
    };
    let r = t.handle_mouse(&right_click, &ctx);
    assert_eq!(r, WidgetResponse::ignored());
    assert!(!t.is_on());
}

#[test]
fn release_outside_bounds_no_toggle() {
    let mut t = ToggleWidget::new();
    let ctx = event_ctx();

    t.handle_mouse(&left_down(), &ctx);
    let outside_up = MouseEvent {
        kind: MouseEventKind::Up(MouseButton::Left),
        pos: Point::new(300.0, 300.0),
        modifiers: Modifiers::NONE,
    };
    let r = t.handle_mouse(&outside_up, &ctx);
    assert!(!t.is_on());
    assert_eq!(r.capture, CaptureRequest::Release);
    assert!(r.action.is_none());
}

#[test]
fn rapid_toggle_maintains_consistency() {
    let mut t = ToggleWidget::new();
    let ctx = event_ctx();

    for i in 0..6 {
        t.handle_key(space_key(), &ctx);
        assert_eq!(t.is_on(), i % 2 == 0);
        let expected_progress = if t.is_on() { 1.0 } else { 0.0 };
        assert_eq!(t.toggle_progress(), expected_progress);
    }
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
    let ctx = event_ctx();
    t.handle_key(space_key(), &ctx);

    let now = Instant::now();
    assert!(t.toggle_progress.is_animating(now));
    assert_eq!(t.toggle_progress.target(), 1.0);
}

#[test]
fn animation_completes_to_target() {
    let mut t = ToggleWidget::new();
    let ctx = event_ctx();
    t.handle_key(space_key(), &ctx);

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
    let ctx = event_ctx();

    t.handle_key(space_key(), &ctx);
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
    use crate::draw::DrawList;

    let mut t = ToggleWidget::new();
    let ctx = event_ctx();
    t.handle_key(space_key(), &ctx);

    let measurer = MockMeasurer::STANDARD;
    let mut draw_list = DrawList::new();
    let bounds = Rect::new(0.0, 0.0, 40.0, 22.0);
    let anim_flag = std::cell::Cell::new(false);
    let now = Instant::now();
    let mut draw_ctx = super::super::DrawCtx {
        measurer: &measurer,
        draw_list: &mut draw_list,
        bounds,
        focused_widget: None,
        now,
        animations_running: &anim_flag,
        theme: &super::super::tests::TEST_THEME,
        icons: None,
        scene_cache: None,
        interaction: None,
        widget_id: None,
        frame_requests: None,
    };
    t.paint(&mut draw_ctx);

    assert!(
        anim_flag.get(),
        "paint() should signal animations_running while toggle animates"
    );
}

#[test]
fn paint_no_animation_signal_when_idle() {
    use crate::draw::DrawList;

    let t = ToggleWidget::new();

    let measurer = MockMeasurer::STANDARD;
    let mut draw_list = DrawList::new();
    let bounds = Rect::new(0.0, 0.0, 40.0, 22.0);
    let anim_flag = std::cell::Cell::new(false);
    let now = Instant::now();
    let mut draw_ctx = super::super::DrawCtx {
        measurer: &measurer,
        draw_list: &mut draw_list,
        bounds,
        focused_widget: None,
        now,
        animations_running: &anim_flag,
        theme: &super::super::tests::TEST_THEME,
        icons: None,
        scene_cache: None,
        interaction: None,
        widget_id: None,
        frame_requests: None,
    };
    t.paint(&mut draw_ctx);

    assert!(
        !anim_flag.get(),
        "paint() should not signal animations_running when idle"
    );
}

#[test]
fn paint_thumb_at_on_position() {
    use crate::draw::{DrawCommand, DrawList};

    let t = ToggleWidget::new().with_on(true);
    let style = ToggleStyle::default();

    let measurer = MockMeasurer::STANDARD;
    let mut draw_list = DrawList::new();
    let bounds = Rect::new(0.0, 0.0, style.width, style.height);
    let anim_flag = std::cell::Cell::new(false);
    let now = Instant::now();
    let mut draw_ctx = super::super::DrawCtx {
        measurer: &measurer,
        draw_list: &mut draw_list,
        bounds,
        focused_widget: None,
        now,
        animations_running: &anim_flag,
        theme: &super::super::tests::TEST_THEME,
        icons: None,
        scene_cache: None,
        interaction: None,
        widget_id: None,
        frame_requests: None,
    };
    t.paint(&mut draw_ctx);

    let rects: Vec<_> = draw_list
        .commands()
        .iter()
        .filter_map(|c| match c {
            DrawCommand::Rect { rect, .. } => Some(*rect),
            _ => None,
        })
        .collect();
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
    use crate::draw::{DrawCommand, DrawList};

    let t = ToggleWidget::new();
    let style = ToggleStyle::default();

    let measurer = MockMeasurer::STANDARD;
    let mut draw_list = DrawList::new();
    let bounds = Rect::new(0.0, 0.0, style.width, style.height);
    let anim_flag = std::cell::Cell::new(false);
    let now = Instant::now();
    let mut draw_ctx = super::super::DrawCtx {
        measurer: &measurer,
        draw_list: &mut draw_list,
        bounds,
        focused_widget: None,
        now,
        animations_running: &anim_flag,
        theme: &super::super::tests::TEST_THEME,
        icons: None,
        scene_cache: None,
        interaction: None,
        widget_id: None,
        frame_requests: None,
    };
    t.paint(&mut draw_ctx);

    let rects: Vec<_> = draw_list
        .commands()
        .iter()
        .filter_map(|c| match c {
            DrawCommand::Rect { rect, .. } => Some(*rect),
            _ => None,
        })
        .collect();
    assert!(rects.len() >= 2, "should have track + thumb rects");

    let thumb_rect = rects.last().unwrap();
    let expected_x = bounds.x() + style.thumb_padding;
    assert!(
        (thumb_rect.x() - expected_x).abs() < 0.1,
        "OFF state thumb x: expected {expected_x}, got {}",
        thumb_rect.x()
    );
}
