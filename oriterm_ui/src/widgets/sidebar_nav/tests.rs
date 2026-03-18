//! Tests for the sidebar navigation widget.

use crate::action::WidgetAction;
use crate::geometry::Rect;
use crate::input::{InputEvent, Key, Modifiers};
use crate::layout::BoxContent;
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
    assert_eq!(w.item_states.len(), 3);
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
fn layout_height_accounts_for_all_items() {
    let theme = crate::theme::UiTheme::dark();
    let w = SidebarNavWidget::new(test_sections(), &theme);
    let m = MockMeasurer::new();
    let ctx = LayoutCtx {
        measurer: &m,
        theme: &theme,
    };
    let layout = w.layout(&ctx);

    if let BoxContent::Leaf {
        intrinsic_height, ..
    } = &layout.content
    {
        // 2 sections * SECTION_TITLE_HEIGHT + 3 items * ITEM_HEIGHT + padding + version
        let expected = 16.0 * 2.0 // SIDEBAR_PADDING_Y * 2
            + SECTION_TITLE_HEIGHT * 2.0
            + ITEM_HEIGHT * 3.0
            + 24.0; // version space
        assert_eq!(*intrinsic_height, expected);
    } else {
        panic!("expected leaf layout");
    }
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
    let action = WidgetAction::Selected {
        id: WidgetId::next(),
        index: 2,
    };
    assert!(w.accept_action(&action));
    assert_eq!(w.active_page(), 2);
}

#[test]
fn accept_action_ignores_same_page() {
    let theme = crate::theme::UiTheme::dark();
    let mut w = SidebarNavWidget::new(test_sections(), &theme);
    let action = WidgetAction::Selected {
        id: WidgetId::next(),
        index: 0,
    };
    assert!(!w.accept_action(&action));
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
