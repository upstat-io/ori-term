use winit::window::CursorIcon;

use crate::geometry::Rect;
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
fn has_three_controllers() {
    // Hover + Click + Focus — the focus controller was added per Round 1
    // codex F1 to ensure click-to-focus so subsequent keymap actions
    // dispatch to this widget (was 2 before, hence the rename).
    let dd = DropdownWidget::new(items());
    assert_eq!(dd.controllers().len(), 3);
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
    // Regression: keyboard Confirm must open the popup, not
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
    // Regression: Escape on a closed dropdown trigger must NOT
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

// -- Cursor icon --

#[test]
fn layout_cursor_icon_pointer() {
    let dd = DropdownWidget::new(items());
    let m = MockMeasurer::new();
    let ctx = LayoutCtx {
        measurer: &m,
        theme: &super::super::tests::TEST_THEME,
    };
    let layout = dd.layout(&ctx);
    assert_eq!(
        layout.cursor_icon,
        CursorIcon::Pointer,
        "dropdown should declare Pointer cursor"
    );
}

/// Repeated open/close cycles must not grow the scene primitive count.
///
/// Regression guard: dropdown trigger click → dismiss → repeat should not
/// leak quads or text runs across cycles.
#[test]
fn dropdown_open_close_cycle_stable_scene_size() {
    use crate::input::MouseButton;
    use crate::testing::WidgetTestHarness;

    let dd = DropdownWidget::new(items());
    let dd_id = dd.id();
    let mut h = WidgetTestHarness::with_size(dd, 300.0, 200.0);
    let bounds = h.widget_bounds(dd_id);
    let center = crate::geometry::Point::new(
        bounds.x() + bounds.width() / 2.0,
        bounds.y() + bounds.height() / 2.0,
    );

    // Warmup: first render establishes baseline.
    h.render();

    let mut counts: Vec<usize> = Vec::new();
    for _ in 0..10 {
        // Click to trigger OpenDropdown action (actual overlay push
        // requires parent container wiring, but the click + dismiss
        // exercises the widget's internal state).
        h.mouse_move(center);
        h.mouse_down(MouseButton::Left);
        h.mouse_up(MouseButton::Left);
        if h.has_overlays() {
            h.dismiss_overlays();
        }
        let scene = h.render();
        counts.push(scene.quads().len() + scene.text_runs().len());
    }

    // Verify no monotonic growth.
    let first = counts[0];
    let last = *counts.last().unwrap();
    assert!(
        last <= first + 5,
        "scene primitive count grew from {first} to {last} over 10 cycles — possible leak"
    );
}

// -- Searchable mode --

/// Plain dropdown emits OpenDropdown on Clicked.
#[test]
fn click_emits_open_dropdown_when_not_searchable() {
    let mut dd = DropdownWidget::new(items());
    let bounds = Rect::new(0.0, 0.0, 100.0, 30.0);
    let action = dd
        .on_action(WidgetAction::Clicked(dd.id()), bounds)
        .expect("click must emit an action");
    assert!(
        matches!(action, WidgetAction::OpenDropdown { .. }),
        "non-searchable dropdown emits OpenDropdown, got {action:?}"
    );
}

/// Searchable dropdown emits OpenSearchableDropdown on Clicked.
#[test]
fn click_emits_open_searchable_when_searchable() {
    let mut dd = DropdownWidget::new(items()).with_searchable(true);
    let bounds = Rect::new(0.0, 0.0, 100.0, 30.0);
    let action = dd
        .on_action(WidgetAction::Clicked(dd.id()), bounds)
        .expect("click must emit an action");
    match action {
        WidgetAction::OpenSearchableDropdown {
            id,
            items: emitted_items,
            selected,
            anchor,
            initial_highlight,
        } => {
            assert_eq!(id, dd.id());
            assert_eq!(emitted_items, items());
            assert_eq!(selected, Some(0));
            assert_eq!(anchor, bounds);
            assert_eq!(initial_highlight, None);
        }
        other => panic!("searchable dropdown must emit OpenSearchableDropdown, got {other:?}"),
    }
}

/// Searchable mode: NavigateDown opens popup with initial_highlight=Some(0)
/// instead of cycling inline (which would emit Selected and update self.selected).
#[test]
fn searchable_navigate_down_opens_popup_does_not_cycle() {
    use crate::action::keymap_action::NavigateDown;
    let mut dd = DropdownWidget::new(items())
        .with_searchable(true)
        .with_selected(0);
    let bounds = Rect::new(0.0, 0.0, 100.0, 30.0);
    let action = dd
        .handle_keymap_action(&NavigateDown, bounds)
        .expect("NavigateDown must emit an action");
    match action {
        WidgetAction::OpenSearchableDropdown {
            initial_highlight, ..
        } => {
            assert_eq!(initial_highlight, Some(0));
        }
        other => panic!("searchable NavigateDown must open popup, got {other:?}"),
    }
    assert_eq!(
        dd.selected(),
        0,
        "searchable trigger must NOT cycle selected on NavigateDown — popup owns navigation"
    );
}

/// Plain mode: NavigateDown still cycles inline as before.
#[test]
fn plain_navigate_down_cycles_inline() {
    use crate::action::keymap_action::NavigateDown;
    let mut dd = DropdownWidget::new(items()).with_selected(0);
    let bounds = Rect::new(0.0, 0.0, 100.0, 30.0);
    let action = dd
        .handle_keymap_action(&NavigateDown, bounds)
        .expect("NavigateDown must emit an action");
    assert!(
        matches!(action, WidgetAction::Selected { .. }),
        "plain dropdown cycles inline (emits Selected), got {action:?}"
    );
    assert_eq!(dd.selected(), 1, "plain dropdown advances selected by one");
}

/// is_searchable() reflects the builder.
#[test]
fn is_searchable_reflects_builder() {
    assert!(!DropdownWidget::new(items()).is_searchable());
    assert!(
        DropdownWidget::new(items())
            .with_searchable(true)
            .is_searchable()
    );
    assert!(
        !DropdownWidget::new(items())
            .with_searchable(true)
            .with_searchable(false)
            .is_searchable()
    );
}
