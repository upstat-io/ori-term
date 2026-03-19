use crate::geometry::{Insets, Rect};
use crate::layout::BoxContent;
use crate::sense::Sense;
use crate::widgets::tests::MockMeasurer;
use crate::widgets::{LayoutCtx, Widget};

use super::{ButtonStyle, ButtonWidget};

// -- Construction and state --

#[test]
fn default_state() {
    let btn = ButtonWidget::new("OK");
    assert_eq!(btn.label(), "OK");
    assert!(!btn.is_disabled());
    assert!(btn.is_focusable());
}

#[test]
fn disabled_not_focusable() {
    let btn = ButtonWidget::new("OK").with_disabled(true);
    assert!(!btn.is_focusable());
}

#[test]
fn set_disabled_toggles() {
    let mut btn = ButtonWidget::new("OK");
    assert!(!btn.is_disabled());
    btn.set_disabled(true);
    assert!(btn.is_disabled());
    assert!(!btn.is_focusable());
    btn.set_disabled(false);
    assert!(!btn.is_disabled());
    assert!(btn.is_focusable());
}

#[test]
fn set_label_updates() {
    let mut btn = ButtonWidget::new("OK");
    btn.label = "Cancel".into();
    assert_eq!(btn.label(), "Cancel");
}

// -- Sense and controllers --

#[test]
fn sense_returns_click() {
    let btn = ButtonWidget::new("OK");
    assert_eq!(btn.sense(), Sense::click());
}

#[test]
fn has_four_controllers() {
    let btn = ButtonWidget::new("OK");
    assert_eq!(btn.controllers().len(), 4);
}

#[test]
fn has_visual_state_animator() {
    let btn = ButtonWidget::new("OK");
    assert!(btn.visual_states().is_some());
}

// -- Layout --

#[test]
fn layout_includes_padding() {
    let btn = ButtonWidget::new("OK");
    let m = MockMeasurer::new();
    let ctx = LayoutCtx {
        measurer: &m,
        theme: &super::super::tests::TEST_THEME,
    };
    let layout = btn.layout(&ctx);
    let style = ButtonStyle::default();

    if let BoxContent::Leaf {
        intrinsic_width,
        intrinsic_height,
    } = &layout.content
    {
        // "OK" = 2 chars * 8px = 16px + padding (12 + 12 = 24) = 40px.
        assert_eq!(*intrinsic_width, 16.0 + style.padding.width());
        // 16px line + padding (6 + 6 = 12) = 28px.
        assert_eq!(*intrinsic_height, 16.0 + style.padding.height());
    } else {
        panic!("expected leaf layout");
    }
}

#[test]
fn empty_label_layout() {
    let btn = ButtonWidget::new("");
    let m = MockMeasurer::new();
    let ctx = LayoutCtx {
        measurer: &m,
        theme: &super::super::tests::TEST_THEME,
    };
    let layout = btn.layout(&ctx);
    let style = ButtonStyle::default();

    if let BoxContent::Leaf {
        intrinsic_width, ..
    } = &layout.content
    {
        // Empty text = 0px + padding.
        assert_eq!(*intrinsic_width, style.padding.width());
    } else {
        panic!("expected leaf layout");
    }
}

// -- Style --

#[test]
fn with_style_applies_custom_style() {
    use crate::color::Color;

    let style = ButtonStyle {
        fg: Color::BLACK,
        bg: Color::WHITE,
        hover_bg: Color::rgb(0.9, 0.9, 0.9),
        pressed_bg: Color::rgb(0.7, 0.7, 0.7),
        border_color: Color::BLACK,
        border_width: 2.0,
        corner_radius: 12.0,
        padding: Insets::all(20.0),
        font_size: 18.0,
        disabled_fg: Color::rgb(0.5, 0.5, 0.5),
        disabled_bg: Color::rgb(0.3, 0.3, 0.3),
        focus_ring_color: Color::rgb(0.0, 0.0, 1.0),
    };
    let btn = ButtonWidget::new("Styled").with_style(style);

    let m = MockMeasurer::new();
    let ctx = LayoutCtx {
        measurer: &m,
        theme: &super::super::tests::TEST_THEME,
    };
    let layout = btn.layout(&ctx);
    if let BoxContent::Leaf {
        intrinsic_width,
        intrinsic_height,
    } = &layout.content
    {
        // "Styled" = 6 chars * 8px = 48px + padding (20 + 20) = 88.
        assert_eq!(*intrinsic_width, 88.0);
        // 16px line + padding (20 + 20) = 56.
        assert_eq!(*intrinsic_height, 56.0);
    } else {
        panic!("expected leaf layout");
    }
}

#[test]
fn with_style_rebuilds_animator() {
    use crate::color::Color;

    let style = ButtonStyle {
        bg: Color::WHITE,
        hover_bg: Color::rgb(0.9, 0.9, 0.9),
        pressed_bg: Color::rgb(0.7, 0.7, 0.7),
        disabled_bg: Color::rgb(0.3, 0.3, 0.3),
        ..ButtonStyle::default()
    };
    let btn = ButtonWidget::new("OK").with_style(style);

    // The animator's initial bg should be the style's normal bg.
    let now = std::time::Instant::now();
    let animator = btn.visual_states().unwrap();
    assert_eq!(animator.get_bg_color(now), Color::WHITE);
}

// -- Paint --

#[test]
fn paint_produces_draw_commands() {
    use crate::draw::Scene;

    let btn = ButtonWidget::new("OK");
    let measurer = MockMeasurer::STANDARD;
    let mut scene = Scene::new();
    let bounds = Rect::new(0.0, 0.0, 100.0, 30.0);
    let now = std::time::Instant::now();
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
    btn.paint(&mut draw_ctx);

    // Should have produced draw commands: layer + rect + text + pop_layer.
    assert!(!scene.is_empty());
}

// -- Harness integration tests --

#[test]
fn harness_full_click_cycle() {
    use crate::action::WidgetAction;
    use crate::input::MouseButton;
    use crate::testing::WidgetTestHarness;

    let btn = ButtonWidget::new("Click me");
    let btn_id = btn.id();
    let mut h = WidgetTestHarness::new(btn);

    // Layout produces non-zero bounds.
    let bounds = h.widget_bounds(btn_id);
    assert!(bounds.width() > 0.0);
    assert!(bounds.height() > 0.0);

    // Hover.
    h.mouse_move_to(btn_id);
    assert!(h.is_hot(btn_id));

    // Press.
    h.mouse_down(MouseButton::Left);
    assert!(h.is_active(btn_id));

    // Release -> Clicked action.
    h.mouse_up(MouseButton::Left);
    assert!(!h.is_active(btn_id));
    let actions = h.take_actions();
    assert!(
        actions
            .iter()
            .any(|a| matches!(a, WidgetAction::Clicked(id) if *id == btn_id))
    );

    // Move away -> not hot.
    h.mouse_move(crate::geometry::Point::new(9999.0, 9999.0));
    assert!(!h.is_hot(btn_id));
}

#[test]
fn harness_keyboard_activation() {
    use crate::action::WidgetAction;
    use crate::input::{Key, Modifiers};
    use crate::testing::WidgetTestHarness;

    let btn = ButtonWidget::new("KB");
    let btn_id = btn.id();
    let mut h = WidgetTestHarness::new(btn);

    // Focus the button via click.
    h.click(btn_id);
    assert!(h.is_focused(btn_id));

    // Enter key -> Clicked.
    let actions = h.key_press(Key::Enter, Modifiers::NONE);
    assert!(
        actions
            .iter()
            .any(|a| matches!(a, WidgetAction::Clicked(id) if *id == btn_id)),
        "Enter should produce Clicked action, got: {actions:?}"
    );

    // Space key -> Clicked.
    let actions = h.key_press(Key::Space, Modifiers::NONE);
    assert!(
        actions
            .iter()
            .any(|a| matches!(a, WidgetAction::Clicked(id) if *id == btn_id)),
        "Space should produce Clicked action, got: {actions:?}"
    );
}
