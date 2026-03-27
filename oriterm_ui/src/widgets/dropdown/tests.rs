use crate::layout::BoxContent;
use crate::sense::Sense;
use crate::widgets::tests::MockMeasurer;
use crate::widgets::{LayoutCtx, Widget, WidgetAction};

use super::{DropdownStyle, DropdownWidget};

fn items() -> Vec<String> {
    vec!["Alpha".into(), "Beta".into(), "Gamma".into()]
}

// -- Construction and state --

#[test]
fn default_state() {
    let dd = DropdownWidget::new(items());
    assert_eq!(dd.selected(), 0);
    assert_eq!(dd.selected_text(), "Alpha");
    assert_eq!(dd.items().len(), 3);
    assert!(!dd.is_disabled());
    assert!(dd.is_focusable());
}

#[test]
fn with_selected_builder() {
    let dd = DropdownWidget::new(items()).with_selected(2);
    assert_eq!(dd.selected(), 2);
    assert_eq!(dd.selected_text(), "Gamma");
}

#[test]
fn selected_clamped() {
    let dd = DropdownWidget::new(items()).with_selected(100);
    assert_eq!(dd.selected(), 2); // Clamped to last index.
}

// -- Sense and controllers --

#[test]
fn sense_returns_click() {
    let dd = DropdownWidget::new(items());
    assert_eq!(dd.sense(), Sense::click());
}

#[test]
fn has_two_controllers() {
    let dd = DropdownWidget::new(items());
    assert_eq!(dd.controllers().len(), 2);
}

#[test]
fn has_visual_state_animator() {
    let dd = DropdownWidget::new(items());
    assert!(dd.visual_states().is_some());
}

// -- Layout --

#[test]
fn layout_accommodates_widest_item() {
    let dd = DropdownWidget::new(items());
    let m = MockMeasurer::new();
    let ctx = LayoutCtx {
        measurer: &m,
        theme: &super::super::tests::TEST_THEME,
    };
    let layout = dd.layout(&ctx);
    let s = DropdownStyle::default();

    if let BoxContent::Leaf {
        intrinsic_width, ..
    } = &layout.content
    {
        // "Gamma" = 5 chars * 8 = 40 (widest) + padding + indicator, clamped to min_width.
        let content_w = 40.0 + s.padding.width() + s.indicator_width;
        let expected = content_w.max(s.min_width);
        assert_eq!(*intrinsic_width, expected);
    } else {
        panic!("expected leaf layout");
    }
}

// -- Programmatic selection --

#[test]
fn set_selected_programmatic() {
    let mut dd = DropdownWidget::new(items());
    dd.set_selected(1);
    assert_eq!(dd.selected(), 1);
    assert_eq!(dd.selected_text(), "Beta");
}

#[test]
fn set_selected_clamped() {
    let mut dd = DropdownWidget::new(items());
    dd.set_selected(99);
    assert_eq!(dd.selected(), 2);
}

#[test]
fn set_disabled_prevents_interaction() {
    let mut dd = DropdownWidget::new(items());

    dd.set_disabled(true);
    assert!(dd.is_disabled());
    assert!(!dd.is_focusable());
}

// -- accept_action --

#[test]
fn accept_action_updates_selection() {
    let mut dd = DropdownWidget::new(items());
    let id = dd.id();

    let action = WidgetAction::Selected { id, index: 2 };
    assert!(dd.accept_action(&action));
    assert_eq!(dd.selected(), 2);
    assert_eq!(dd.selected_text(), "Gamma");
}

#[test]
fn accept_action_ignores_wrong_id() {
    let mut dd = DropdownWidget::new(items());
    let other_id = crate::widget_id::WidgetId::next();

    let action = WidgetAction::Selected {
        id: other_id,
        index: 1,
    };
    assert!(!dd.accept_action(&action));
    assert_eq!(dd.selected(), 0);
}

// -- Keymap actions --

#[test]
fn confirm_emits_open_dropdown_not_selected() {
    // Regression: TPR-13-009 — keyboard Confirm must open the popup, not
    // silently cycle the selection.
    use crate::action::keymap_action::Confirm;
    use crate::geometry::Rect;
    let mut dd = DropdownWidget::new(items());
    let bounds = Rect::new(10.0, 20.0, 140.0, 30.0);

    let result = dd.handle_keymap_action(&Confirm, bounds);
    assert!(
        matches!(result, Some(WidgetAction::OpenDropdown { .. })),
        "Confirm should emit OpenDropdown, got: {result:?}"
    );
}

#[test]
fn dismiss_does_not_emit_overlay_action() {
    // Regression: TPR-13-008 — Escape on a closed dropdown trigger must NOT
    // emit DismissOverlay (which would close the entire settings dialog).
    use crate::action::keymap_action::Dismiss;
    use crate::geometry::Rect;
    let mut dd = DropdownWidget::new(items());
    let bounds = Rect::new(10.0, 20.0, 140.0, 30.0);

    let result = dd.handle_keymap_action(&Dismiss, bounds);
    assert!(
        result.is_none(),
        "Dismiss on closed trigger should be no-op, got: {result:?}"
    );
}

// -- Style --

#[test]
fn with_style_rebuilds_animator() {
    use crate::color::Color;

    let style = DropdownStyle {
        bg: Color::WHITE,
        hover_bg: Color::rgb(0.9, 0.9, 0.9),
        pressed_bg: Color::rgb(0.7, 0.7, 0.7),
        disabled_bg: Color::rgb(0.3, 0.3, 0.3),
        ..DropdownStyle::default()
    };
    let dd = DropdownWidget::new(items()).with_style(style);

    // The animator's initial bg should be the style's normal bg.
    let animator = dd.visual_states().unwrap();
    assert_eq!(animator.get_bg_color(), Color::WHITE);
}
