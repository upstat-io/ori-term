//! Tests for the sidebar navigation widget.

use crate::layout::BoxContent;
use crate::sense::Sense;
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
