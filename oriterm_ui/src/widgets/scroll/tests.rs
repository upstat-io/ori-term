use crate::controllers::ControllerRequests;
use crate::draw::Scene;
use crate::draw::scene::ContentMask;
use crate::geometry::{Point, Rect};
use crate::input::{InputEvent, Key, Modifiers, ScrollDelta};
use crate::interaction::LifecycleEvent;
use crate::layout::compute_layout;
use crate::widgets::container::ContainerWidget;
use crate::widgets::label::LabelWidget;
use crate::widgets::tests::MockMeasurer;
use crate::widgets::{DrawCtx, LayoutCtx, LifecycleCtx, Widget};

use super::ScrollWidget;

/// Creates content that is 16px tall (single label).
fn short_content() -> Box<dyn Widget> {
    Box::new(LabelWidget::new("A".repeat(100)))
}

/// Creates a tall column of labels that overflows a small viewport.
/// 20 labels * 16px = 320px tall.
fn tall_content() -> Box<dyn Widget> {
    let labels: Vec<Box<dyn Widget>> = (0..20)
        .map(|i| Box::new(LabelWidget::new(format!("Line {i}"))) as Box<dyn Widget>)
        .collect();
    Box::new(ContainerWidget::column().with_children(labels))
}

fn make_scroll(child: Box<dyn Widget>) -> ScrollWidget {
    ScrollWidget::vertical(child)
}

/// Standard test bounds: 200x100 viewport.
fn bounds() -> Rect {
    Rect::new(0.0, 0.0, 200.0, 100.0)
}

/// Pre-populates the scroll widget's cached child layout by computing it.
fn populate_cache(scroll: &ScrollWidget, bounds: Rect) {
    let measurer = MockMeasurer::STANDARD;
    let theme = super::super::tests::TEST_THEME;
    scroll.child_natural_size(&measurer, &theme, bounds);
}

#[test]
fn scroll_layout_fills_width_for_vertical() {
    let scroll = make_scroll(short_content());
    let ctx = LayoutCtx {
        measurer: &MockMeasurer::STANDARD,
        theme: &super::super::tests::TEST_THEME,
    };
    let layout_box = scroll.layout(&ctx);
    let viewport = Rect::new(0.0, 0.0, 1000.0, 1000.0);
    let node = compute_layout(&layout_box, viewport);

    // Vertical scroll uses Fill width — takes viewport width, not content width.
    assert_eq!(node.rect.width(), 1000.0);
    // Height is the child's natural height (100-char label = 16px tall).
    assert_eq!(node.rect.height(), 16.0);
}

#[test]
fn scroll_offset_starts_at_zero() {
    let scroll = make_scroll(tall_content());
    assert_eq!(scroll.scroll_offset(), 0.0);
}

#[test]
fn scroll_offset_clamps_to_range() {
    let mut scroll = make_scroll(tall_content());
    // Content 500px tall, viewport 200px → max offset = 300.
    scroll.set_scroll_offset(999.0, 500.0, 200.0);
    assert_eq!(scroll.scroll_offset(), 300.0);

    scroll.set_scroll_offset(-10.0, 500.0, 200.0);
    assert_eq!(scroll.scroll_offset(), 0.0);
}

#[test]
fn scroll_offset_zero_when_content_fits() {
    let mut scroll = make_scroll(tall_content());
    // Content 100px, viewport 200px → max offset = 0.
    scroll.set_scroll_offset(50.0, 100.0, 200.0);
    assert_eq!(scroll.scroll_offset(), 0.0);
}

#[test]
fn scroll_is_focusable() {
    let scroll = make_scroll(tall_content());
    assert!(scroll.is_focusable());
}

#[test]
fn scroll_draws_with_clip() {
    let scroll = make_scroll(tall_content());
    let measurer = MockMeasurer::STANDARD;
    let mut scene = Scene::new();
    let bounds = Rect::new(0.0, 0.0, 200.0, 100.0);
    let mut ctx = DrawCtx {
        measurer: &measurer,
        scene: &mut scene,
        bounds,
        now: std::time::Instant::now(),
        theme: &super::super::tests::TEST_THEME,
        icons: None,
        interaction: None,
        widget_id: None,
        frame_requests: None,
    };
    scroll.paint(&mut ctx);

    // Content primitives should have a clipped ContentMask (not unclipped).
    let clipped_texts = scene
        .text_runs()
        .iter()
        .filter(|t| t.content_mask != ContentMask::unclipped())
        .count();
    assert!(
        clipped_texts > 0,
        "text runs should be clipped to scroll viewport"
    );
}

#[test]
fn scroll_wheel_changes_offset() {
    let mut scroll = ScrollWidget::vertical(tall_content());
    populate_cache(&scroll, bounds());

    let event = InputEvent::Scroll {
        pos: Point::new(25.0, 25.0),
        delta: ScrollDelta::Lines { x: 0.0, y: -3.0 },
        modifiers: Modifiers::NONE,
    };
    let result = scroll.on_input(&event, bounds());

    assert!(result.handled, "scroll event should be handled");
    assert!(scroll.scroll_offset() > 0.0, "offset should increase");
}

#[test]
fn key_home_resets_to_top() {
    let mut scroll = ScrollWidget::vertical(tall_content());
    scroll.set_scroll_offset(100.0, 320.0, 100.0);
    populate_cache(&scroll, bounds());

    let event = InputEvent::KeyDown {
        key: Key::Home,
        modifiers: Modifiers::NONE,
    };
    let result = scroll.on_input(&event, bounds());
    assert!(result.handled);
    assert_eq!(scroll.scroll_offset(), 0.0);
}

#[test]
fn key_end_scrolls_to_bottom() {
    let mut scroll = ScrollWidget::vertical(tall_content());
    populate_cache(&scroll, bounds());

    let event = InputEvent::KeyDown {
        key: Key::End,
        modifiers: Modifiers::NONE,
    };
    let result = scroll.on_input(&event, bounds());
    assert!(result.handled);
    // Content 320px, view 100px → max offset = 220.
    assert_eq!(scroll.scroll_offset(), 220.0);
}

#[test]
fn key_arrow_down_scrolls() {
    let mut scroll = ScrollWidget::vertical(tall_content());
    populate_cache(&scroll, bounds());

    let event = InputEvent::KeyDown {
        key: Key::ArrowDown,
        modifiers: Modifiers::NONE,
    };
    let result = scroll.on_input(&event, bounds());
    assert!(result.handled);
    // Should have scrolled down by line_height (20px).
    assert_eq!(scroll.scroll_offset(), 20.0);
}

#[test]
fn key_page_down_scrolls_by_viewport() {
    let mut scroll = ScrollWidget::vertical(tall_content());
    populate_cache(&scroll, bounds());

    let event = InputEvent::KeyDown {
        key: Key::PageDown,
        modifiers: Modifiers::NONE,
    };
    let result = scroll.on_input(&event, bounds());
    assert!(result.handled);
    // Should scroll down by one viewport height (100px).
    assert_eq!(scroll.scroll_offset(), 100.0);
}

#[test]
fn key_page_up_scrolls_by_viewport() {
    let mut scroll = ScrollWidget::vertical(tall_content());
    scroll.set_scroll_offset(200.0, 320.0, 100.0);
    populate_cache(&scroll, bounds());

    let event = InputEvent::KeyDown {
        key: Key::PageUp,
        modifiers: Modifiers::NONE,
    };
    let result = scroll.on_input(&event, bounds());
    assert!(result.handled);
    // Should scroll up by one viewport height (100px): 200 - 100 = 100.
    assert_eq!(scroll.scroll_offset(), 100.0);
}

#[test]
fn key_page_down_clamps_at_bottom() {
    let mut scroll = ScrollWidget::vertical(tall_content());
    scroll.set_scroll_offset(200.0, 320.0, 100.0);
    populate_cache(&scroll, bounds());

    let event = InputEvent::KeyDown {
        key: Key::PageDown,
        modifiers: Modifiers::NONE,
    };
    scroll.on_input(&event, bounds());
    // 200 + 100 = 300, clamped to max 220.
    assert_eq!(scroll.scroll_offset(), 220.0);
}

#[test]
fn key_page_up_clamps_at_top() {
    let mut scroll = ScrollWidget::vertical(tall_content());
    scroll.set_scroll_offset(30.0, 320.0, 100.0);
    populate_cache(&scroll, bounds());

    let event = InputEvent::KeyDown {
        key: Key::PageUp,
        modifiers: Modifiers::NONE,
    };
    scroll.on_input(&event, bounds());
    // 30 - 100 = -70, clamped to 0.
    assert_eq!(scroll.scroll_offset(), 0.0);
}

// Edge cases from Chromium/Ratatui audit

#[test]
fn scroll_clip_rect_matches_viewport() {
    let scroll = make_scroll(tall_content());
    let measurer = MockMeasurer::STANDARD;
    let mut scene = Scene::new();
    let bounds = Rect::new(10.0, 20.0, 150.0, 80.0);
    let mut ctx = DrawCtx {
        measurer: &measurer,
        scene: &mut scene,
        bounds,
        now: std::time::Instant::now(),
        theme: &super::super::tests::TEST_THEME,
        icons: None,
        interaction: None,
        widget_id: None,
        frame_requests: None,
    };
    scroll.paint(&mut ctx);

    // Content primitives should be clipped to the scroll viewport bounds.
    let first_clip = scene.text_runs().first().map(|t| t.content_mask.clip);
    assert_eq!(
        first_clip,
        Some(bounds),
        "clip rect must match scroll viewport"
    );
}

#[test]
fn scroll_child_drawn_offset_by_scroll() {
    let mut scroll = make_scroll(tall_content());
    scroll.set_scroll_offset(40.0, 320.0, 100.0);

    let measurer = MockMeasurer::STANDARD;
    let mut scene = Scene::new();
    let bounds = Rect::new(0.0, 0.0, 200.0, 100.0);
    let mut ctx = DrawCtx {
        measurer: &measurer,
        scene: &mut scene,
        bounds,
        now: std::time::Instant::now(),
        theme: &super::super::tests::TEST_THEME,
        icons: None,
        interaction: None,
        widget_id: None,
        frame_requests: None,
    };
    scroll.paint(&mut ctx);

    // Scene bakes the translate offset into positions directly.
    // With 40px scroll offset, the first visible text (y=32 content space)
    // should have position y = 32 - 40 = -8, but labels outside the clip
    // are culled. The first drawn text should be the one intersecting the viewport.
    let first_text = scene.text_runs().first();
    assert!(first_text.is_some(), "should have text runs");
    let pos = first_text.unwrap().position;
    assert_eq!(
        pos.y, -8.0,
        "first text y should be offset by scroll amount (32 - 40 = -8)"
    );
}

#[test]
fn scroll_draws_scrollbar_when_overflowing() {
    let scroll = make_scroll(tall_content());
    let measurer = MockMeasurer::STANDARD;
    let mut scene = Scene::new();
    let bounds = Rect::new(0.0, 0.0, 200.0, 100.0);
    let mut ctx = DrawCtx {
        measurer: &measurer,
        scene: &mut scene,
        bounds,
        now: std::time::Instant::now(),
        theme: &super::super::tests::TEST_THEME,
        icons: None,
        interaction: None,
        widget_id: None,
        frame_requests: None,
    };
    scroll.paint(&mut ctx);

    // Scrollbar rects are drawn outside the clip scope (unclipped).
    let unclipped_rects = scene
        .quads()
        .iter()
        .filter(|q| q.content_mask == ContentMask::unclipped())
        .count();
    assert!(
        unclipped_rects >= 1,
        "scrollbar thumb rect should be unclipped"
    );
}

#[test]
fn scroll_no_scrollbar_when_content_fits() {
    let scroll = make_scroll(short_content());
    let measurer = MockMeasurer::STANDARD;
    let mut scene = Scene::new();
    let bounds = Rect::new(0.0, 0.0, 1000.0, 100.0);
    let mut ctx = DrawCtx {
        measurer: &measurer,
        scene: &mut scene,
        bounds,
        now: std::time::Instant::now(),
        theme: &super::super::tests::TEST_THEME,
        icons: None,
        interaction: None,
        widget_id: None,
        frame_requests: None,
    };
    scroll.paint(&mut ctx);

    // No unclipped rects when content fits (no scrollbar).
    let unclipped_rects = scene
        .quads()
        .iter()
        .filter(|q| q.content_mask == ContentMask::unclipped())
        .count();
    assert_eq!(unclipped_rects, 0, "no scrollbar when content fits");
}

#[test]
fn scroll_multiple_wheel_events_accumulate() {
    let mut scroll = ScrollWidget::vertical(tall_content());
    populate_cache(&scroll, bounds());

    for _ in 0..3 {
        let event = InputEvent::Scroll {
            pos: Point::new(25.0, 25.0),
            delta: ScrollDelta::Lines { x: 0.0, y: -1.0 },
            modifiers: Modifiers::NONE,
        };
        scroll.on_input(&event, bounds());
    }

    // 3 lines * 20px line_height = 60px offset.
    assert_eq!(scroll.scroll_offset(), 60.0);
}

#[test]
fn scroll_wheel_clamps_at_bottom() {
    let mut scroll = ScrollWidget::vertical(tall_content());
    populate_cache(&scroll, bounds());

    let event = InputEvent::Scroll {
        pos: Point::new(25.0, 25.0),
        delta: ScrollDelta::Lines { x: 0.0, y: -999.0 },
        modifiers: Modifiers::NONE,
    };
    scroll.on_input(&event, bounds());

    // Content 320px, viewport 100px → max offset 220.
    assert_eq!(scroll.scroll_offset(), 220.0);
}

#[test]
fn scroll_wheel_clamps_at_top() {
    let mut scroll = ScrollWidget::vertical(tall_content());
    populate_cache(&scroll, bounds());

    let event = InputEvent::Scroll {
        pos: Point::new(25.0, 25.0),
        delta: ScrollDelta::Lines { x: 0.0, y: 5.0 },
        modifiers: Modifiers::NONE,
    };
    scroll.on_input(&event, bounds());
    assert_eq!(scroll.scroll_offset(), 0.0);
}

#[test]
fn scroll_pixel_delta_works() {
    let mut scroll = ScrollWidget::vertical(tall_content());
    populate_cache(&scroll, bounds());

    let event = InputEvent::Scroll {
        pos: Point::new(25.0, 25.0),
        delta: ScrollDelta::Pixels { x: 0.0, y: -35.0 },
        modifiers: Modifiers::NONE,
    };
    scroll.on_input(&event, bounds());
    assert_eq!(scroll.scroll_offset(), 35.0);
}

#[test]
fn arrow_up_scrolls_upward() {
    let mut scroll = ScrollWidget::vertical(tall_content());
    scroll.set_scroll_offset(100.0, 320.0, 100.0);
    populate_cache(&scroll, bounds());

    let event = InputEvent::KeyDown {
        key: Key::ArrowUp,
        modifiers: Modifiers::NONE,
    };
    scroll.on_input(&event, bounds());
    assert_eq!(scroll.scroll_offset(), 80.0); // 100 - 20
}

// Horizontal and both-direction tests (Chromium scroll view patterns)

/// Creates a wide row of labels that overflows a narrow viewport.
/// 20 labels * 8px * 10 chars = 1600px wide.
fn wide_content() -> Box<dyn Widget> {
    let labels: Vec<Box<dyn Widget>> = (0..20)
        .map(|i| Box::new(LabelWidget::new(format!("HorizLbl{i}"))) as Box<dyn Widget>)
        .collect();
    Box::new(ContainerWidget::row().with_children(labels))
}

#[test]
fn horizontal_scroll_new_constructor() {
    let scroll = ScrollWidget::new(wide_content(), super::ScrollDirection::Horizontal);
    assert_eq!(scroll.scroll_offset(), 0.0);
    assert!(scroll.is_focusable());
}

#[test]
fn both_direction_new_constructor() {
    let scroll = ScrollWidget::new(tall_content(), super::ScrollDirection::Both);
    assert_eq!(scroll.scroll_offset(), 0.0);
    assert!(scroll.is_focusable());
}

#[test]
fn horizontal_scroll_draws_with_clip() {
    let scroll = ScrollWidget::new(wide_content(), super::ScrollDirection::Horizontal);
    let measurer = MockMeasurer::STANDARD;
    let mut scene = Scene::new();
    let bounds = Rect::new(0.0, 0.0, 100.0, 50.0);
    let mut ctx = DrawCtx {
        measurer: &measurer,
        scene: &mut scene,
        bounds,
        now: std::time::Instant::now(),
        theme: &super::super::tests::TEST_THEME,
        icons: None,
        interaction: None,
        widget_id: None,
        frame_requests: None,
    };
    scroll.paint(&mut ctx);

    // Content primitives should be clipped to the scroll viewport.
    let clipped_texts = scene
        .text_runs()
        .iter()
        .filter(|t| t.content_mask != ContentMask::unclipped())
        .count();
    assert!(
        clipped_texts > 0,
        "text runs should be clipped to scroll viewport"
    );
}

#[test]
fn scroll_content_exactly_fits_viewport() {
    let mut scroll = ScrollWidget::vertical(tall_content());
    scroll.set_scroll_offset(50.0, 320.0, 320.0);
    assert_eq!(scroll.scroll_offset(), 0.0, "no scroll when content fits");
}

#[test]
fn scroll_content_exactly_fits_no_scrollbar() {
    let label = LabelWidget::new("A".repeat(10));
    let scroll = ScrollWidget::vertical(Box::new(label));
    let measurer = MockMeasurer::STANDARD;
    let mut scene = Scene::new();
    let bounds = Rect::new(0.0, 0.0, 200.0, 16.0);
    let mut ctx = DrawCtx {
        measurer: &measurer,
        scene: &mut scene,
        bounds,
        now: std::time::Instant::now(),
        theme: &super::super::tests::TEST_THEME,
        icons: None,
        interaction: None,
        widget_id: None,
        frame_requests: None,
    };
    scroll.paint(&mut ctx);

    // No unclipped rects when content exactly fits (no scrollbar).
    let unclipped_rects = scene
        .quads()
        .iter()
        .filter(|q| q.content_mask == ContentMask::unclipped())
        .count();
    assert_eq!(unclipped_rects, 0, "no scrollbar when content exactly fits");
}

#[test]
fn scroll_hover_resets_on_leave_but_drag_persists() {
    let mut scroll = ScrollWidget::vertical(tall_content());

    // Simulate active drag + hover.
    scroll.v_bar.track_hovered = true;
    scroll.v_bar.thumb_hovered = true;
    scroll.v_bar.dragging = true;

    // HotChanged(false) should clear hover but NOT drag — the drag
    // owns the pointer via capture until MouseUp, matching native
    // scrollbar behavior where you can drag outside the widget.
    let event = LifecycleEvent::HotChanged {
        widget_id: scroll.id(),
        is_hot: false,
    };
    let mut lctx = LifecycleCtx {
        widget_id: scroll.id(),
        interaction: &crate::interaction::InteractionState::default(),
        requests: ControllerRequests::NONE,
    };
    scroll.lifecycle(&event, &mut lctx);
    assert!(!scroll.v_bar.track_hovered, "track_hovered should reset");
    assert!(!scroll.v_bar.thumb_hovered, "thumb_hovered should reset");
    assert!(
        scroll.v_bar.dragging,
        "dragging must persist through hot loss"
    );
}

#[test]
fn scroll_with_scrollbar_style() {
    use super::ScrollbarStyle;
    use crate::color::Color;

    let mut custom_style = ScrollbarStyle::default();
    custom_style.thickness = 10.0;
    custom_style.thumb_color = Color::WHITE;
    custom_style.track_color = Color::BLACK;
    custom_style.thumb_radius = 5.0;
    custom_style.min_thumb_length = 30.0;

    let scroll = ScrollWidget::vertical(tall_content()).with_scrollbar_style(custom_style);
    let measurer = MockMeasurer::STANDARD;
    let mut scene = Scene::new();
    let bounds = Rect::new(0.0, 0.0, 200.0, 100.0);
    let mut ctx = DrawCtx {
        measurer: &measurer,
        scene: &mut scene,
        bounds,
        now: std::time::Instant::now(),
        theme: &super::super::tests::TEST_THEME,
        icons: None,
        interaction: None,
        widget_id: None,
        frame_requests: None,
    };
    scroll.paint(&mut ctx);
    assert!(!scene.is_empty());
}

// -- reset_scroll --

#[test]
fn reset_scroll_clears_offset() {
    let mut scroll = make_scroll(tall_content());
    scroll.set_scroll_offset(100.0, 320.0, 100.0);
    assert!((scroll.scroll_offset() - 100.0).abs() < f32::EPSILON);

    scroll.reset_scroll();
    assert!((scroll.scroll_offset()).abs() < f32::EPSILON);
}

#[test]
fn reset_scroll_invalidates_cached_child_layout() {
    let mut scroll = make_scroll(tall_content());

    // Populate the layout cache by computing child natural size.
    let measurer = MockMeasurer::STANDARD;
    let theme = crate::widgets::tests::TEST_THEME;
    let viewport = Rect::new(0.0, 0.0, 200.0, 100.0);
    let _ = scroll.child_natural_size(&measurer, &theme, viewport);
    assert!(
        scroll.cached_child_layout.borrow().is_some(),
        "cache should be populated after child_natural_size"
    );

    scroll.reset_scroll();
    assert!(
        scroll.cached_child_layout.borrow().is_none(),
        "reset_scroll must invalidate cached_child_layout"
    );
}

// -- per-axis scrollbar state --

#[test]
fn scroll_per_axis_bar_state_defaults_to_rest() {
    let scroll = ScrollWidget::vertical(tall_content());
    assert!(!scroll.v_bar.dragging);
    assert!(!scroll.v_bar.track_hovered);
    assert!(!scroll.v_bar.thumb_hovered);
    assert!(!scroll.h_bar.dragging);
    assert!(!scroll.h_bar.track_hovered);
    assert!(!scroll.h_bar.thumb_hovered);
}

#[test]
fn scroll_horizontal_bar_hover_resets_on_leave() {
    let mut scroll = ScrollWidget::new(tall_content(), super::ScrollDirection::Both);
    scroll.h_bar.track_hovered = true;
    scroll.h_bar.thumb_hovered = true;
    scroll.h_bar.dragging = true;

    let event = LifecycleEvent::HotChanged {
        widget_id: scroll.id(),
        is_hot: false,
    };
    let mut lctx = LifecycleCtx {
        widget_id: scroll.id(),
        interaction: &crate::interaction::InteractionState::default(),
        requests: ControllerRequests::NONE,
    };
    scroll.lifecycle(&event, &mut lctx);
    assert!(!scroll.h_bar.track_hovered);
    assert!(!scroll.h_bar.thumb_hovered);
    assert!(
        scroll.h_bar.dragging,
        "dragging must persist through hot loss"
    );
}

#[test]
fn scroll_visual_state_reflects_interaction() {
    use crate::widgets::scrollbar::ScrollbarVisualState;

    let mut scroll = ScrollWidget::vertical(tall_content());
    assert_eq!(scroll.v_bar.visual_state(), ScrollbarVisualState::Rest);

    scroll.v_bar.track_hovered = true;
    assert_eq!(scroll.v_bar.visual_state(), ScrollbarVisualState::Hovered);

    scroll.v_bar.dragging = true;
    assert_eq!(scroll.v_bar.visual_state(), ScrollbarVisualState::Dragging);
}

#[test]
fn scroll_default_style_is_theme_backed() {
    use super::ScrollbarStyle;
    use crate::theme::UiTheme;

    let scroll = ScrollWidget::vertical(tall_content());
    let expected = ScrollbarStyle::from_theme(&UiTheme::dark());
    assert_eq!(scroll.scrollbar_style.thumb_color, expected.thumb_color);
    assert_eq!(scroll.scrollbar_style.thickness, expected.thickness);
}

#[test]
fn scroll_vertical_emits_scrollbar_quad() {
    let scroll = make_scroll(tall_content());
    let measurer = MockMeasurer::STANDARD;
    let mut scene = Scene::new();
    let b = bounds();
    // Populate cache first so draw_scrollbars sees overflow.
    let theme = super::super::tests::TEST_THEME;
    scroll.child_natural_size(&measurer, &theme, b);

    let mut ctx = DrawCtx {
        measurer: &measurer,
        scene: &mut scene,
        bounds: b,
        now: std::time::Instant::now(),
        theme: &theme,
        icons: None,
        interaction: None,
        widget_id: None,
        frame_requests: None,
    };
    scroll.paint(&mut ctx);

    // Should have at least one quad for the thumb.
    assert!(
        scene.quads().iter().any(|q| q.bounds.width() <= 8.0),
        "expected a thin scrollbar quad"
    );
}

#[test]
fn scroll_horizontal_mode_scroll_by_x() {
    let mut scroll = ScrollWidget::new(tall_content(), super::ScrollDirection::Horizontal);
    // Scroll_by_x should work.
    let changed = scroll.scroll_by_x(50.0, 600.0, 200.0);
    assert!(changed);
    assert!((scroll.scroll_offset_x - 50.0).abs() < f32::EPSILON);
}

#[test]
fn scroll_both_wheel_applies_both_axes() {
    let mut scroll = ScrollWidget::new(tall_content(), super::ScrollDirection::Both);
    populate_cache(&scroll, bounds());

    let event = InputEvent::Scroll {
        pos: Point::new(100.0, 50.0),
        delta: ScrollDelta::Pixels { x: -30.0, y: -40.0 },
        modifiers: Modifiers::NONE,
    };
    let result = scroll.on_input(&event, bounds());
    // Wheel input in Both mode should be handled.
    assert!(result.handled);
}
