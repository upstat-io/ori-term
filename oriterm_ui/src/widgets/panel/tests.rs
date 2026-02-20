use crate::draw::DrawList;
use crate::geometry::{Insets, Rect};
use crate::layout::compute_layout;
use crate::widgets::button::ButtonWidget;
use crate::widgets::label::LabelWidget;
use crate::widgets::tests::MockMeasurer;
use crate::widgets::{DrawCtx, LayoutCtx, Widget};

use super::{PanelStyle, PanelWidget};

fn make_panel_with_label(label: &str) -> PanelWidget {
    let child = Box::new(LabelWidget::new(label));
    PanelWidget::new(child)
}

#[test]
fn panel_layout_includes_padding() {
    let panel = make_panel_with_label("Hello");
    let ctx = LayoutCtx {
        measurer: &MockMeasurer::STANDARD,
    };
    let layout_box = panel.layout(&ctx);
    let viewport = Rect::new(0.0, 0.0, 400.0, 300.0);
    let node = compute_layout(&layout_box, viewport);

    // Label: 5 chars * 8px = 40px wide, 16px tall.
    // Default padding: 12px all sides.
    // Panel size: 40 + 24 = 64 wide, 16 + 24 = 40 tall (Hug mode).
    assert_eq!(node.rect.width(), 64.0);
    assert_eq!(node.rect.height(), 40.0);
}

#[test]
fn panel_child_gets_content_rect() {
    let panel = make_panel_with_label("Hi");
    let ctx = LayoutCtx {
        measurer: &MockMeasurer::STANDARD,
    };
    let layout_box = panel.layout(&ctx);
    let viewport = Rect::new(0.0, 0.0, 400.0, 300.0);
    let node = compute_layout(&layout_box, viewport);

    // Child is the first child of the panel layout node.
    assert_eq!(node.children.len(), 1);
    let child = &node.children[0];
    // Child rect should be inset by padding (12px each side).
    assert_eq!(child.rect.x(), 12.0);
    assert_eq!(child.rect.y(), 12.0);
}

#[test]
fn panel_draws_background_rect() {
    let panel = make_panel_with_label("Test");
    let measurer = MockMeasurer::STANDARD;
    let mut draw_list = DrawList::new();
    let bounds = Rect::new(10.0, 20.0, 100.0, 50.0);
    let mut ctx = DrawCtx {
        measurer: &measurer,
        draw_list: &mut draw_list,
        bounds,
        focused_widget: None,
    };
    panel.draw(&mut ctx);

    // First command should be the background rect at the panel's bounds.
    let cmds = draw_list.commands();
    assert!(!cmds.is_empty(), "panel should produce draw commands");
    match &cmds[0] {
        crate::draw::DrawCommand::Rect { rect, .. } => {
            assert_eq!(*rect, bounds);
        }
        other => panic!("expected Rect command, got {other:?}"),
    }
}

#[test]
fn panel_not_focusable() {
    let panel = make_panel_with_label("X");
    assert!(!panel.is_focusable());
}

#[test]
fn panel_custom_style() {
    use crate::color::Color;
    let style = PanelStyle {
        bg: Color::WHITE,
        border_width: 2.0,
        corner_radius: 16.0,
        padding: Insets::all(20.0),
        ..PanelStyle::default()
    };
    let panel = make_panel_with_label("Styled").with_style(style.clone());
    let ctx = LayoutCtx {
        measurer: &MockMeasurer::STANDARD,
    };
    let layout_box = panel.layout(&ctx);
    let viewport = Rect::new(0.0, 0.0, 400.0, 300.0);
    let node = compute_layout(&layout_box, viewport);

    // "Styled" = 6 chars * 8 = 48px, + 40px padding = 88px wide.
    assert_eq!(node.rect.width(), 88.0);
    assert_eq!(node.rect.height(), 56.0); // 16 + 40
}

#[test]
fn panel_delegates_key_to_child() {
    use crate::input::{Key, KeyEvent, Modifiers};

    let child = Box::new(ButtonWidget::new("Click me"));
    let child_id = child.id();
    let mut panel = PanelWidget::new(child);

    let measurer = MockMeasurer::STANDARD;
    let bounds = Rect::new(0.0, 0.0, 200.0, 100.0);
    let ctx = super::super::EventCtx {
        measurer: &measurer,
        bounds,
        is_focused: true,
    };
    let event = KeyEvent {
        key: Key::Enter,
        modifiers: Modifiers::NONE,
    };
    let response = panel.handle_key(event, &ctx);

    // Button should have emitted Clicked action.
    assert!(response.action.is_some());
    match response.action {
        Some(super::super::WidgetAction::Clicked(id)) => assert_eq!(id, child_id),
        other => panic!("expected Clicked, got {other:?}"),
    }
}
