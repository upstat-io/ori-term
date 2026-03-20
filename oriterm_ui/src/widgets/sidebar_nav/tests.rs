//! Tests for the sidebar navigation widget.

use crate::action::WidgetAction;
use crate::geometry::Rect;
use crate::input::{InputEvent, Key, Modifiers};
use crate::layout::{BoxContent, SizeSpec};
use crate::sense::Sense;
use crate::widget_id::WidgetId;
use crate::widgets::tests::MockMeasurer;
use crate::widgets::{LayoutCtx, Widget};

use super::{
    ITEM_HEIGHT, NavItem, NavSection, SECTION_TITLE_HEIGHT, SIDEBAR_WIDTH, SidebarNavWidget,
};

fn test_sections() -> Vec<NavSection> {
    vec![
        NavSection {
            title: "General".into(),
            items: vec![
                NavItem {
                    label: "Appearance".into(),
                    icon: None,
                    page_index: 0,
                },
                NavItem {
                    label: "Colors".into(),
                    icon: None,
                    page_index: 1,
                },
            ],
        },
        NavSection {
            title: "Advanced".into(),
            items: vec![NavItem {
                label: "Rendering".into(),
                icon: None,
                page_index: 2,
            }],
        },
    ]
}

#[test]
fn construction_default_state() {
    let theme = crate::theme::UiTheme::dark();
    let w = SidebarNavWidget::new(test_sections(), &theme);
    assert_eq!(w.active_page(), 0);
    assert_eq!(w.sections.len(), 2);
    assert!(w.hovered_item.is_none());
}

#[test]
fn set_active_page() {
    let theme = crate::theme::UiTheme::dark();
    let mut w = SidebarNavWidget::new(test_sections(), &theme);
    w.set_active_page(2);
    assert_eq!(w.active_page(), 2);
}

#[test]
fn layout_width_is_sidebar_width() {
    let theme = crate::theme::UiTheme::dark();
    let w = SidebarNavWidget::new(test_sections(), &theme);
    let m = MockMeasurer::new();
    let ctx = LayoutCtx {
        measurer: &m,
        theme: &theme,
    };
    let layout = w.layout(&ctx);

    if let BoxContent::Leaf {
        intrinsic_width, ..
    } = &layout.content
    {
        assert_eq!(*intrinsic_width, SIDEBAR_WIDTH);
    } else {
        panic!("expected leaf layout");
    }
}

#[test]
fn layout_height_fills_parent() {
    let theme = crate::theme::UiTheme::dark();
    let w = SidebarNavWidget::new(test_sections(), &theme);
    let m = MockMeasurer::new();
    let ctx = LayoutCtx {
        measurer: &m,
        theme: &theme,
    };
    let layout = w.layout(&ctx);

    // Sidebar uses SizeSpec::Fill for height — it stretches to fill parent.
    assert_eq!(layout.height, SizeSpec::Fill);
}

#[test]
fn sense_returns_click() {
    let theme = crate::theme::UiTheme::dark();
    let w = SidebarNavWidget::new(test_sections(), &theme);
    assert_eq!(w.sense(), Sense::click());
}

#[test]
fn hit_test_first_item() {
    let theme = crate::theme::UiTheme::dark();
    let w = SidebarNavWidget::new(test_sections(), &theme);
    // First section title takes SECTION_TITLE_HEIGHT, then first item starts.
    let y = SECTION_TITLE_HEIGHT + 1.0;
    assert_eq!(w.hit_test_item(y), Some(0));
}

#[test]
fn hit_test_second_section_item() {
    let theme = crate::theme::UiTheme::dark();
    let w = SidebarNavWidget::new(test_sections(), &theme);
    // After first section (title + 2 items) + second section title.
    let y = SECTION_TITLE_HEIGHT + ITEM_HEIGHT * 2.0 + SECTION_TITLE_HEIGHT + 1.0;
    assert_eq!(w.hit_test_item(y), Some(2));
}

#[test]
fn hit_test_before_items_returns_none() {
    let theme = crate::theme::UiTheme::dark();
    let w = SidebarNavWidget::new(test_sections(), &theme);
    // Inside section title area.
    assert_eq!(w.hit_test_item(5.0), None);
}

#[test]
fn page_for_flat_index() {
    let theme = crate::theme::UiTheme::dark();
    let w = SidebarNavWidget::new(test_sections(), &theme);
    assert_eq!(w.page_for_flat_index(0), Some(0));
    assert_eq!(w.page_for_flat_index(1), Some(1));
    assert_eq!(w.page_for_flat_index(2), Some(2));
    assert_eq!(w.page_for_flat_index(3), None);
}

#[test]
fn with_version_builder() {
    let theme = crate::theme::UiTheme::dark();
    let w = SidebarNavWidget::new(test_sections(), &theme).with_version("v0.2.0");
    assert_eq!(w.version, "v0.2.0");
}

#[test]
fn debug_impl() {
    let theme = crate::theme::UiTheme::dark();
    let w = SidebarNavWidget::new(test_sections(), &theme);
    let dbg = format!("{w:?}");
    assert!(dbg.contains("SidebarNavWidget"));
    assert!(dbg.contains("active_page"));
}

// -- Focusability --

#[test]
fn is_focusable() {
    let theme = crate::theme::UiTheme::dark();
    let w = SidebarNavWidget::new(test_sections(), &theme);
    assert!(w.is_focusable());
}

// -- accept_action for page sync --

#[test]
fn accept_action_updates_active_page() {
    let theme = crate::theme::UiTheme::dark();
    let mut w = SidebarNavWidget::new(test_sections(), &theme);
    let own_id = w.id();
    let action = WidgetAction::Selected {
        id: own_id,
        index: 2,
    };
    assert!(w.accept_action(&action));
    assert_eq!(w.active_page(), 2);
}

#[test]
fn accept_action_ignores_same_page() {
    let theme = crate::theme::UiTheme::dark();
    let mut w = SidebarNavWidget::new(test_sections(), &theme);
    let own_id = w.id();
    let action = WidgetAction::Selected {
        id: own_id,
        index: 0,
    };
    assert!(!w.accept_action(&action));
}

#[test]
fn accept_action_ignores_external_selected() {
    let theme = crate::theme::UiTheme::dark();
    let mut w = SidebarNavWidget::new(test_sections(), &theme);
    // Selected from a different widget (e.g., SchemeCard) should be ignored.
    let action = WidgetAction::Selected {
        id: WidgetId::next(),
        index: 2,
    };
    assert!(!w.accept_action(&action));
    assert_eq!(w.active_page(), 0);
}

// -- Arrow key navigation --

fn arrow_down() -> InputEvent {
    InputEvent::KeyDown {
        key: Key::ArrowDown,
        modifiers: Modifiers::NONE,
    }
}

fn arrow_up() -> InputEvent {
    InputEvent::KeyDown {
        key: Key::ArrowUp,
        modifiers: Modifiers::NONE,
    }
}

#[test]
fn arrow_down_emits_selected() {
    let theme = crate::theme::UiTheme::dark();
    let mut w = SidebarNavWidget::new(test_sections(), &theme);
    let bounds = Rect::new(0.0, 0.0, SIDEBAR_WIDTH, 400.0);
    let result = w.on_input(&arrow_down(), bounds);
    assert!(result.handled);
    match result.action {
        Some(WidgetAction::Selected { index, .. }) => assert_eq!(index, 1),
        other => panic!("expected Selected(1), got {other:?}"),
    }
}

#[test]
fn arrow_up_at_top_is_ignored() {
    let theme = crate::theme::UiTheme::dark();
    let mut w = SidebarNavWidget::new(test_sections(), &theme);
    let bounds = Rect::new(0.0, 0.0, SIDEBAR_WIDTH, 400.0);
    let result = w.on_input(&arrow_up(), bounds);
    assert!(!result.handled);
}

#[test]
fn arrow_down_at_bottom_is_ignored() {
    let theme = crate::theme::UiTheme::dark();
    let mut w = SidebarNavWidget::new(test_sections(), &theme);
    w.set_active_page(2); // Last item.
    let bounds = Rect::new(0.0, 0.0, SIDEBAR_WIDTH, 400.0);
    let result = w.on_input(&arrow_down(), bounds);
    assert!(!result.handled);
}

#[test]
fn home_key_goes_to_first() {
    let theme = crate::theme::UiTheme::dark();
    let mut w = SidebarNavWidget::new(test_sections(), &theme);
    w.set_active_page(2);
    let bounds = Rect::new(0.0, 0.0, SIDEBAR_WIDTH, 400.0);
    let event = InputEvent::KeyDown {
        key: Key::Home,
        modifiers: Modifiers::NONE,
    };
    let result = w.on_input(&event, bounds);
    assert!(result.handled);
    match result.action {
        Some(WidgetAction::Selected { index, .. }) => assert_eq!(index, 0),
        other => panic!("expected Selected(0), got {other:?}"),
    }
}

#[test]
fn end_key_goes_to_last() {
    let theme = crate::theme::UiTheme::dark();
    let mut w = SidebarNavWidget::new(test_sections(), &theme);
    let bounds = Rect::new(0.0, 0.0, SIDEBAR_WIDTH, 400.0);
    let event = InputEvent::KeyDown {
        key: Key::End,
        modifiers: Modifiers::NONE,
    };
    let result = w.on_input(&event, bounds);
    assert!(result.handled);
    match result.action {
        Some(WidgetAction::Selected { index, .. }) => assert_eq!(index, 2),
        other => panic!("expected Selected(2), got {other:?}"),
    }
}

// -- Harness integration: single click emits Selected --

#[test]
fn harness_single_click_emits_selected() {
    use crate::geometry::Point;
    use crate::input::MouseButton;
    use crate::testing::WidgetTestHarness;

    let theme = crate::theme::UiTheme::dark();
    let sidebar = SidebarNavWidget::new(test_sections(), &theme);
    let _sidebar_id = sidebar.id();

    let mut h = WidgetTestHarness::new(sidebar);

    // Compute click target: second item (Colors, page_index=1).
    // Y = SECTION_TITLE_HEIGHT (General) + ITEM_HEIGHT (Appearance) + half ITEM_HEIGHT.
    let target_y = SECTION_TITLE_HEIGHT + ITEM_HEIGHT + ITEM_HEIGHT / 2.0;
    let target_x = SIDEBAR_WIDTH / 2.0;
    let target = Point::new(target_x, target_y);

    h.root_mut().clear_actions();
    // Single click: move → down → up.
    h.mouse_move(target);
    h.mouse_down(MouseButton::Left);
    h.mouse_up(MouseButton::Left);
    let actions = h.take_actions();

    // Should emit Selected { index: 1 } for the Colors item.
    let selected = actions
        .iter()
        .find(|a| matches!(a, WidgetAction::Selected { .. }));
    assert!(
        selected.is_some(),
        "single click on sidebar item should emit Selected, got: {actions:?}"
    );
    if let Some(WidgetAction::Selected { index, .. }) = selected {
        assert_eq!(*index, 1, "should select Colors (page_index=1)");
    }
}

#[test]
fn harness_single_click_on_first_item_emits_selected() {
    use crate::geometry::Point;
    use crate::input::MouseButton;
    use crate::testing::WidgetTestHarness;

    let theme = crate::theme::UiTheme::dark();
    let mut sidebar = SidebarNavWidget::new(test_sections(), &theme);
    // Start on page 2 so clicking page 0 is a real switch.
    sidebar.set_active_page(2);
    let _sidebar_id = sidebar.id();

    let mut h = WidgetTestHarness::new(sidebar);

    // Click first item (Appearance, page_index=0).
    let target_y = SECTION_TITLE_HEIGHT + ITEM_HEIGHT / 2.0;
    let target_x = SIDEBAR_WIDTH / 2.0;
    let target = Point::new(target_x, target_y);

    h.root_mut().clear_actions();
    h.mouse_move(target);
    h.mouse_down(MouseButton::Left);
    h.mouse_up(MouseButton::Left);
    let actions = h.take_actions();

    let selected = actions
        .iter()
        .find(|a| matches!(a, WidgetAction::Selected { .. }));
    assert!(
        selected.is_some(),
        "single click on first sidebar item should emit Selected, got: {actions:?}"
    );
    if let Some(WidgetAction::Selected { index, .. }) = selected {
        assert_eq!(*index, 0, "should select Appearance (page_index=0)");
    }
}
