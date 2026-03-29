use crate::controllers::ControllerRequests;
use crate::geometry::{Point, Rect};
use crate::input::{InputEvent, Modifiers, ScrollDelta};
use crate::interaction::LifecycleEvent;
use crate::layout::BoxContent;
use crate::widgets::scrollbar::ScrollbarVisualState;
use crate::widgets::tests::MockMeasurer;
use crate::widgets::{LayoutCtx, LifecycleCtx, Widget, WidgetAction};

use super::{DragMode, MenuEntry, MenuStyle, MenuWidget};

static MEASURER: MockMeasurer = MockMeasurer::STANDARD;

fn layout_ctx() -> LayoutCtx<'static> {
    LayoutCtx {
        measurer: &MEASURER,
        theme: &super::super::tests::TEST_THEME,
    }
}

fn sample_entries() -> Vec<MenuEntry> {
    vec![
        MenuEntry::Item {
            label: "Copy".into(),
        },
        MenuEntry::Item {
            label: "Paste".into(),
        },
        MenuEntry::Separator,
        MenuEntry::Item {
            label: "Select All".into(),
        },
    ]
}

// Layout tests

#[test]
fn layout_cursor_icon_pointer() {
    let menu = MenuWidget::new(sample_entries());
    let layout = menu.layout(&layout_ctx());
    assert_eq!(
        layout.cursor_icon,
        winit::window::CursorIcon::Pointer,
        "menu items are clickable — cursor should be pointer"
    );
}

#[test]
fn layout_min_width_enforced() {
    // Short labels should still produce at least min_width.
    let menu = MenuWidget::new(vec![MenuEntry::Item { label: "X".into() }]);
    let layout = menu.layout(&layout_ctx());

    if let BoxContent::Leaf {
        intrinsic_width, ..
    } = &layout.content
    {
        assert!(
            *intrinsic_width >= MenuStyle::default().min_width,
            "width {} should be >= min_width {}",
            intrinsic_width,
            MenuStyle::default().min_width
        );
    } else {
        panic!("expected leaf layout");
    }
}

#[test]
fn layout_height_includes_all_entries() {
    let s = MenuStyle::default();
    let menu = MenuWidget::new(sample_entries());
    let layout = menu.layout(&layout_ctx());

    // 3 items × item_height + 1 separator × separator_height + 2 × padding_y
    let expected = 3.0 * s.item_height + s.separator_height + 2.0 * s.padding_y;

    if let BoxContent::Leaf {
        intrinsic_height, ..
    } = &layout.content
    {
        assert_eq!(*intrinsic_height, expected);
    } else {
        panic!("expected leaf layout");
    }
}

#[test]
fn layout_empty_menu() {
    let menu = MenuWidget::new(vec![]);
    let layout = menu.layout(&layout_ctx());

    if let BoxContent::Leaf {
        intrinsic_width,
        intrinsic_height,
    } = &layout.content
    {
        let s = MenuStyle::default();
        assert!(*intrinsic_width >= s.min_width);
        // Only vertical padding, no entries.
        assert_eq!(*intrinsic_height, s.padding_y * 2.0);
    } else {
        panic!("expected leaf layout");
    }
}

#[test]
fn layout_wide_label_exceeds_min_width() {
    // "A really long menu item label!!" = 31 chars × 8px = 248px
    let menu = MenuWidget::new(vec![MenuEntry::Item {
        label: "A really long menu item label!!".into(),
    }]);
    let layout = menu.layout(&layout_ctx());

    if let BoxContent::Leaf {
        intrinsic_width, ..
    } = &layout.content
    {
        assert!(
            *intrinsic_width > MenuStyle::default().min_width,
            "wide label should exceed min_width"
        );
    } else {
        panic!("expected leaf layout");
    }
}

// Check item tests

#[test]
fn check_entries_affect_layout() {
    // Use a label long enough that both menus exceed min_width,
    // so the checkmark space difference is visible.
    let entries_no_check = vec![MenuEntry::Item {
        label: "A long enough menu item label here".into(),
    }];
    let entries_with_check = vec![MenuEntry::Check {
        label: "A long enough menu item label here".into(),
        checked: true,
    }];

    let menu_no = MenuWidget::new(entries_no_check);
    let menu_yes = MenuWidget::new(entries_with_check);

    let layout_no = menu_no.layout(&layout_ctx());
    let layout_yes = menu_yes.layout(&layout_ctx());

    if let (
        BoxContent::Leaf {
            intrinsic_width: w_no,
            ..
        },
        BoxContent::Leaf {
            intrinsic_width: w_yes,
            ..
        },
    ) = (&layout_no.content, &layout_yes.content)
    {
        // Check items add checkmark_size + checkmark_gap to the left margin.
        assert!(
            w_yes > w_no,
            "check menu should be wider: {} vs {}",
            w_yes,
            w_no
        );
    } else {
        panic!("expected leaf layouts");
    }
}

#[test]
fn menu_is_focusable() {
    let menu = MenuWidget::new(sample_entries());
    assert!(menu.is_focusable());
}

// Theme-derived style tests

#[test]
fn from_theme_light_preserves_corner_radius() {
    // Regression: popup builders must not hardcode 0.0 radius — light theme uses 4.0.
    let light = crate::theme::UiTheme::light();
    let style = MenuStyle::from_theme(&light);
    assert_eq!(style.corner_radius, 4.0);
    assert_eq!(style.hover_radius, 4.0);
}

#[test]
fn from_theme_dark_uses_zero_radius() {
    let dark = crate::theme::UiTheme::dark();
    let style = MenuStyle::from_theme(&dark);
    assert_eq!(style.corner_radius, 0.0);
    assert_eq!(style.hover_radius, 0.0);
}

#[test]
fn menu_style_owns_scrollbar_style() {
    use crate::theme::UiTheme;
    use crate::widgets::scrollbar::ScrollbarStyle;

    let theme = UiTheme::dark();
    let style = MenuStyle::from_theme(&theme);
    let expected = ScrollbarStyle::from_theme(&theme);

    // MenuStyle.scrollbar should match a fresh theme-derived scrollbar style.
    assert_eq!(style.scrollbar.thumb_color, expected.thumb_color);
    assert_eq!(style.scrollbar.thickness, expected.thickness);
    assert_eq!(
        style.scrollbar.thumb_hover_color,
        expected.thumb_hover_color
    );
}

#[test]
fn menu_scrollbar_no_hardcoded_white_alpha() {
    use crate::color::Color;

    let style = MenuStyle::default();
    // The old hardcoded Color::WHITE.with_alpha(0.25) should be gone.
    assert_ne!(
        style.scrollbar.thumb_color,
        Color::WHITE.with_alpha(0.25),
        "menu scrollbar should use theme colors, not hardcoded white-alpha"
    );
}

// Controller setup tests

#[test]
fn menu_has_controllers() {
    let menu = MenuWidget::new(sample_entries());
    assert_eq!(
        menu.controllers.len(),
        2,
        "should have HoverController + ScrubController"
    );
}

#[test]
fn menu_sense_includes_drag() {
    use crate::sense::Sense;
    let menu = MenuWidget::new(sample_entries());
    assert_eq!(menu.sense(), Sense::click_and_drag());
}

// Scrollbar interaction tests (via on_action for drag, on_input for hover)

/// Creates a scrollable menu with 20 items and max_height=200.
///
/// Total content: 20 × 32px items + 2 × 4px padding = 648px.
/// Visible: 200px. Scrollable range: 448px.
fn scrollable_menu() -> MenuWidget {
    let entries: Vec<MenuEntry> = (0..20)
        .map(|i| MenuEntry::Item {
            label: format!("Item {i}"),
        })
        .collect();
    let mut style = MenuStyle::default();
    style.max_height = Some(200.0);
    MenuWidget::new(entries).with_style(style)
}

/// Bounds for the scrollable menu.
fn menu_bounds() -> Rect {
    Rect::new(0.0, 0.0, 200.0, 200.0)
}

/// A point on the scrollbar thumb (right edge, within thumb at offset 0).
fn point_on_thumb() -> Point {
    // border=1, edge_inset=2, thickness=6 → track x = 199-6-2 = 191.
    // Thumb starts at y=1 (inner top), extends ~61px.
    Point::new(194.0, 30.0)
}

/// A point on the scrollbar track below the thumb (at offset 0).
fn point_on_track_below_thumb() -> Point {
    Point::new(194.0, 150.0)
}

/// A point in the menu content area (not over the scrollbar).
fn point_on_content() -> Point {
    Point::new(50.0, 40.0)
}

#[test]
fn scrollbar_hover_on_mouse_move() {
    let mut menu = scrollable_menu();
    let bounds = menu_bounds();

    let event = InputEvent::MouseMove {
        pos: point_on_thumb(),
        modifiers: Modifiers::NONE,
    };
    menu.on_input(&event, bounds);

    assert!(
        menu.scrollbar_state.track_hovered || menu.scrollbar_state.thumb_hovered,
        "scrollbar should be hovered when cursor is over it"
    );
    assert_eq!(
        menu.scrollbar_state.visual_state(),
        ScrollbarVisualState::Hovered
    );
}

#[test]
fn scrollbar_hover_clears_entry_hover() {
    let mut menu = scrollable_menu();
    let bounds = menu_bounds();

    // First hover an entry.
    let move_to_content = InputEvent::MouseMove {
        pos: point_on_content(),
        modifiers: Modifiers::NONE,
    };
    menu.on_input(&move_to_content, bounds);
    assert!(menu.hovered.is_some(), "should hover an entry first");

    // Move to scrollbar — entry hover should clear.
    let move_to_scrollbar = InputEvent::MouseMove {
        pos: point_on_thumb(),
        modifiers: Modifiers::NONE,
    };
    menu.on_input(&move_to_scrollbar, bounds);
    assert!(
        menu.hovered.is_none(),
        "entry hover should clear when cursor is on scrollbar"
    );
}

/// Wheel scrolling with cursor over the scrollbar must NOT highlight a
/// menu row behind the bar (regression: TPR-07-015).
#[test]
fn scroll_wheel_over_scrollbar_keeps_hover_clear() {
    let mut menu = scrollable_menu();
    let bounds = menu_bounds();

    // Move cursor to the scrollbar first.
    let move_ev = InputEvent::MouseMove {
        pos: point_on_thumb(),
        modifiers: Modifiers::NONE,
    };
    menu.on_input(&move_ev, bounds);
    assert!(
        menu.scrollbar_state.track_hovered || menu.scrollbar_state.thumb_hovered,
        "scrollbar should be hovered"
    );
    assert!(menu.hovered.is_none(), "entry hover should be cleared");

    // Scroll with wheel while cursor is over the scrollbar.
    let scroll_ev = InputEvent::Scroll {
        delta: ScrollDelta::Lines { x: 0.0, y: -3.0 },
        pos: point_on_thumb(),
        modifiers: Modifiers::NONE,
    };
    menu.on_input(&scroll_ev, bounds);

    // After scrolling, hovered must still be None because the cursor is
    // on the scrollbar, not on a menu item.
    assert!(
        menu.hovered.is_none(),
        "wheel scroll over scrollbar must not highlight a row behind the bar"
    );
}

#[test]
fn scrollbar_thumb_drag_updates_offset() {
    let mut menu = scrollable_menu();
    let bounds = menu_bounds();
    assert_eq!(menu.scroll_offset, 0.0);

    // Simulate DragStart on thumb via on_action.
    let pos = point_on_thumb();
    menu.on_action(WidgetAction::DragStart { id: menu.id, pos }, bounds);
    assert!(menu.scrollbar_state.dragging, "should start dragging");
    assert_eq!(menu.drag_mode, Some(DragMode::ScrollbarThumb));

    // Drag down by 50px via DragUpdate.
    let total_delta = Point::new(0.0, 50.0);
    menu.on_action(
        WidgetAction::DragUpdate {
            id: menu.id,
            delta: total_delta,
            total_delta,
        },
        bounds,
    );
    assert!(
        menu.scroll_offset > 0.0,
        "scroll offset should increase after dragging down"
    );

    // Release via DragEnd.
    let end_pos = Point::new(pos.x, pos.y + 50.0);
    menu.on_action(
        WidgetAction::DragEnd {
            id: menu.id,
            pos: end_pos,
        },
        bounds,
    );
    assert!(!menu.scrollbar_state.dragging, "drag should end on release");
    assert_eq!(menu.drag_mode, None);
}

#[test]
fn scrollbar_track_click_jumps_offset() {
    let mut menu = scrollable_menu();
    let bounds = menu_bounds();
    assert_eq!(menu.scroll_offset, 0.0);

    // DragStart on track below the thumb.
    let pos = point_on_track_below_thumb();
    menu.on_action(WidgetAction::DragStart { id: menu.id, pos }, bounds);
    assert_eq!(menu.drag_mode, Some(DragMode::ScrollbarTrack));
    assert!(
        !menu.scrollbar_state.dragging,
        "track click should not start drag"
    );
    assert!(
        menu.scroll_offset > 0.0,
        "scroll offset should jump on track click"
    );
}

#[test]
fn scrollbar_visual_state_rest_by_default() {
    let menu = scrollable_menu();
    assert_eq!(
        menu.scrollbar_state.visual_state(),
        ScrollbarVisualState::Rest
    );
}

#[test]
fn scrollbar_visual_state_dragging() {
    let mut menu = scrollable_menu();
    let bounds = menu_bounds();

    // Start drag via on_action.
    menu.on_action(
        WidgetAction::DragStart {
            id: menu.id,
            pos: point_on_thumb(),
        },
        bounds,
    );
    assert_eq!(
        menu.scrollbar_state.visual_state(),
        ScrollbarVisualState::Dragging
    );
}

#[test]
fn scrollbar_hot_loss_clears_hover() {
    let mut menu = scrollable_menu();
    let bounds = menu_bounds();

    // Hover the scrollbar.
    let move_event = InputEvent::MouseMove {
        pos: point_on_thumb(),
        modifiers: Modifiers::NONE,
    };
    menu.on_input(&move_event, bounds);
    assert_eq!(
        menu.scrollbar_state.visual_state(),
        ScrollbarVisualState::Hovered
    );

    // Simulate hot loss.
    let event = LifecycleEvent::HotChanged {
        widget_id: menu.id,
        is_hot: false,
    };
    let mut lctx = LifecycleCtx {
        widget_id: menu.id,
        interaction: &crate::interaction::InteractionState::default(),
        requests: ControllerRequests::NONE,
    };
    menu.lifecycle(&event, &mut lctx);

    assert_eq!(
        menu.scrollbar_state.visual_state(),
        ScrollbarVisualState::Rest,
        "hover state should reset on hot loss"
    );
}

#[test]
fn scrollbar_drag_persists_through_hot_loss() {
    let mut menu = scrollable_menu();
    let bounds = menu_bounds();

    // Start drag via on_action.
    menu.on_action(
        WidgetAction::DragStart {
            id: menu.id,
            pos: point_on_thumb(),
        },
        bounds,
    );
    assert!(menu.scrollbar_state.dragging);

    // Simulate hot loss — drag must persist.
    let event = LifecycleEvent::HotChanged {
        widget_id: menu.id,
        is_hot: false,
    };
    let mut lctx = LifecycleCtx {
        widget_id: menu.id,
        interaction: &crate::interaction::InteractionState::default(),
        requests: ControllerRequests::NONE,
    };
    menu.lifecycle(&event, &mut lctx);

    assert!(
        menu.scrollbar_state.dragging,
        "drag must persist through hot loss"
    );
}

#[test]
fn non_scrollable_menu_item_press_via_on_action() {
    let mut menu = MenuWidget::new(sample_entries());
    let bounds = Rect::new(0.0, 0.0, 200.0, 200.0);

    // DragStart on content area.
    let pos = Point::new(50.0, 20.0);
    menu.on_action(WidgetAction::DragStart { id: menu.id, pos }, bounds);
    assert!(
        !menu.scrollbar_state.dragging,
        "non-scrollable menu should not start scrollbar drag"
    );
    assert_eq!(
        menu.drag_mode,
        Some(DragMode::ItemPress),
        "should record item press"
    );
}

// Item click tests (DragStart + DragEnd → Selected action)

#[test]
fn item_click_without_prior_mouse_move() {
    // Regression: TPR-07-012 — menu opens under stationary cursor, first
    // click must select the item under the press position, not no-op.
    let mut menu = MenuWidget::new(sample_entries());
    let bounds = Rect::new(0.0, 0.0, 200.0, 200.0);
    assert_eq!(menu.hovered, None, "no hover before any input");

    // DragStart on first item — no prior MouseMove.
    let pos = Point::new(50.0, 20.0);
    menu.on_action(WidgetAction::DragStart { id: menu.id, pos }, bounds);
    assert_eq!(menu.drag_mode, Some(DragMode::ItemPress));
    assert_eq!(
        menu.hovered,
        Some(0),
        "handle_drag_start should set hover from press position"
    );

    // DragEnd (no DragUpdate — cursor didn't move).
    let result = menu.on_action(WidgetAction::DragEnd { id: menu.id, pos }, bounds);
    match result {
        Some(WidgetAction::Selected { index, .. }) => {
            assert_eq!(index, 0, "should select first item");
        }
        other => panic!("expected Selected action, got {:?}", other),
    }
}

#[test]
fn item_click_emits_selected() {
    let mut menu = MenuWidget::new(sample_entries());
    let bounds = Rect::new(0.0, 0.0, 200.0, 200.0);

    // Move to first item to set hover.
    let move_event = InputEvent::MouseMove {
        pos: Point::new(50.0, 20.0),
        modifiers: Modifiers::NONE,
    };
    menu.on_input(&move_event, bounds);
    assert_eq!(menu.hovered, Some(0));

    // DragStart on item (ScrubController fires this on MouseDown).
    let pos = Point::new(50.0, 20.0);
    menu.on_action(WidgetAction::DragStart { id: menu.id, pos }, bounds);
    assert_eq!(menu.drag_mode, Some(DragMode::ItemPress));

    // DragEnd (ScrubController fires this on MouseUp).
    let result = menu.on_action(WidgetAction::DragEnd { id: menu.id, pos }, bounds);
    match result {
        Some(WidgetAction::Selected { index, .. }) => {
            assert_eq!(index, 0, "should select first item");
        }
        other => panic!("expected Selected action, got {:?}", other),
    }
}

#[test]
fn click_on_separator_does_not_select() {
    let mut menu = MenuWidget::new(sample_entries());
    let bounds = Rect::new(0.0, 0.0, 200.0, 200.0);
    let s = &MenuStyle::default();

    // Move to separator area.
    let sep_y = s.padding_y + s.item_height * 2.0 + s.separator_height * 0.5;
    let move_event = InputEvent::MouseMove {
        pos: Point::new(50.0, sep_y),
        modifiers: Modifiers::NONE,
    };
    menu.on_input(&move_event, bounds);
    assert_eq!(menu.hovered, None, "separator should not be hoverable");

    // DragStart + DragEnd on separator.
    let pos = Point::new(50.0, sep_y);
    menu.on_action(WidgetAction::DragStart { id: menu.id, pos }, bounds);
    let result = menu.on_action(WidgetAction::DragEnd { id: menu.id, pos }, bounds);
    assert!(result.is_none(), "separator click should not select");
}

#[test]
fn release_outside_menu_does_not_select() {
    let mut menu = MenuWidget::new(sample_entries());
    let bounds = Rect::new(0.0, 0.0, 200.0, 200.0);

    // Hover item.
    let move_event = InputEvent::MouseMove {
        pos: Point::new(50.0, 20.0),
        modifiers: Modifiers::NONE,
    };
    menu.on_input(&move_event, bounds);
    assert_eq!(menu.hovered, Some(0));

    // DragStart on item.
    let pos = Point::new(50.0, 20.0);
    menu.on_action(WidgetAction::DragStart { id: menu.id, pos }, bounds);

    // DragUpdate moving outside — hover should clear via entry_at_y.
    let total_delta = Point::new(0.0, 280.0);
    menu.on_action(
        WidgetAction::DragUpdate {
            id: menu.id,
            delta: total_delta,
            total_delta,
        },
        bounds,
    );
    assert_eq!(menu.hovered, None, "hover should clear when outside");

    // DragEnd outside.
    let result = menu.on_action(
        WidgetAction::DragEnd {
            id: menu.id,
            pos: Point::new(50.0, 300.0),
        },
        bounds,
    );
    assert!(result.is_none(), "releasing outside should not select");
}

// Harness integration tests — full propagation pipeline

#[test]
fn harness_item_click_produces_selected() {
    use crate::input::MouseButton;
    use crate::testing::WidgetTestHarness;

    let entries = sample_entries();
    let menu = MenuWidget::new(entries);
    let menu_id = menu.id();
    let mut h = WidgetTestHarness::new(menu);

    let bounds = h.widget_bounds(menu_id);
    let s = MenuStyle::default();

    // Click center of first item row.
    let item_y = bounds.y() + s.padding_y + s.item_height * 0.5;
    let pos = Point::new(bounds.x() + 50.0, item_y);

    h.mouse_move(pos);
    h.mouse_down(MouseButton::Left);
    assert!(h.is_active(menu_id), "menu should capture on item press");
    h.mouse_up(MouseButton::Left);
    assert!(!h.is_active(menu_id), "capture should release on mouse up");

    let actions = h.take_actions();
    assert!(
        actions
            .iter()
            .any(|a| matches!(a, WidgetAction::Selected { index: 0, .. })),
        "expected Selected {{index: 0}}, got: {:?}",
        actions,
    );
}

#[test]
fn harness_scrollbar_drag_captures_and_releases() {
    use crate::input::MouseButton;
    use crate::testing::WidgetTestHarness;

    let entries: Vec<MenuEntry> = (0..20)
        .map(|i| MenuEntry::Item {
            label: format!("Item {i}"),
        })
        .collect();
    let mut style = MenuStyle::default();
    style.max_height = Some(200.0);
    let menu = MenuWidget::new(entries).with_style(style.clone());
    let menu_id = menu.id();
    let mut h = WidgetTestHarness::with_size(menu, 300.0, 200.0);

    let bounds = h.widget_bounds(menu_id);

    // Scrollbar thumb position: right edge minus border, edge_inset, half-thickness.
    let thumb_x = bounds.right()
        - style.border_width
        - style.scrollbar.edge_inset
        - style.scrollbar.thickness * 0.5;
    let thumb_y = bounds.y() + 30.0;

    // Move to scrollbar, then press.
    h.mouse_move(Point::new(thumb_x, thumb_y));
    h.mouse_down(MouseButton::Left);
    assert!(
        h.is_active(menu_id),
        "menu should capture on scrollbar thumb press"
    );

    // Drag down.
    h.mouse_move(Point::new(thumb_x, thumb_y + 40.0));

    // Release.
    h.mouse_up(MouseButton::Left);
    assert!(
        !h.is_active(menu_id),
        "capture should release after scrollbar drag"
    );
}

#[test]
fn harness_scrollbar_track_click_captures_via_scrub() {
    use crate::input::MouseButton;
    use crate::testing::WidgetTestHarness;

    let entries: Vec<MenuEntry> = (0..20)
        .map(|i| MenuEntry::Item {
            label: format!("Item {i}"),
        })
        .collect();
    let mut style = MenuStyle::default();
    style.max_height = Some(200.0);
    let menu = MenuWidget::new(entries).with_style(style.clone());
    let menu_id = menu.id();
    let mut h = WidgetTestHarness::with_size(menu, 300.0, 200.0);

    let bounds = h.widget_bounds(menu_id);

    // Track position below the thumb (at scroll_offset=0, thumb is near top).
    let track_x = bounds.right()
        - style.border_width
        - style.scrollbar.edge_inset
        - style.scrollbar.thickness * 0.5;
    let track_y = bounds.y() + 150.0;

    // Click on track — ScrubController captures on any MouseDown.
    h.mouse_move(Point::new(track_x, track_y));
    h.mouse_down(MouseButton::Left);
    assert!(
        h.is_active(menu_id),
        "ScrubController captures on all MouseDown events"
    );

    // Release clears capture.
    h.mouse_up(MouseButton::Left);
    assert!(
        !h.is_active(menu_id),
        "capture should release after track click"
    );
}
