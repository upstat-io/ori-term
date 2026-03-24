use crate::layout::BoxContent;
use crate::widgets::tests::MockMeasurer;
use crate::widgets::{LayoutCtx, Widget};

use super::{MenuEntry, MenuStyle, MenuWidget};

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
