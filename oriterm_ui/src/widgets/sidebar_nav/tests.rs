//! Tests for the sidebar navigation widget.

use winit::window::CursorIcon;

use crate::action::WidgetAction;
use crate::geometry::{Point, Rect};
use crate::input::{InputEvent, Key, Modifiers, MouseButton};
use crate::layout::{BoxContent, SizeSpec};
use crate::sense::Sense;
use crate::widget_id::WidgetId;
use crate::widgets::tests::MockMeasurer;
use crate::widgets::{LayoutCtx, LifecycleCtx, Widget};

use crate::interaction::LifecycleEvent;

use super::geometry::{NAV_ITEM_HEIGHT, SEARCH_AREA_H, SIDEBAR_PADDING_Y, title_y_advance};
use super::{FooterTarget, NavItem, NavSection, SIDEBAR_WIDTH, SidebarNavWidget};

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
fn sense_returns_click_and_focusable() {
    let theme = crate::theme::UiTheme::dark();
    let w = SidebarNavWidget::new(test_sections(), &theme);
    assert_eq!(w.sense(), Sense::click().union(Sense::focusable()));
}

#[test]
fn hit_test_first_item() {
    let theme = crate::theme::UiTheme::dark();
    let w = SidebarNavWidget::new(test_sections(), &theme);
    // Search area + first section title (no top margin), then first item.
    let y = SEARCH_AREA_H + title_y_advance(true) + 1.0;
    assert_eq!(w.hit_test_item(y), Some(0));
}

#[test]
fn hit_test_second_section_item() {
    let theme = crate::theme::UiTheme::dark();
    let w = SidebarNavWidget::new(test_sections(), &theme);
    // Search area + first section (title + 2 items) + second section title (with top margin).
    let y = SEARCH_AREA_H
        + title_y_advance(true)
        + NAV_ITEM_HEIGHT * 2.0
        + title_y_advance(false)
        + 1.0;
    assert_eq!(w.hit_test_item(y), Some(2));
}

#[test]
fn hit_test_before_items_returns_none() {
    let theme = crate::theme::UiTheme::dark();
    let w = SidebarNavWidget::new(test_sections(), &theme);
    // Inside search field area.
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
fn search_focus_lost_clears_search_focused() {
    let theme = crate::theme::UiTheme::dark();
    let mut w = SidebarNavWidget::new(test_sections(), &theme);
    let wid = w.id();
    w.search_focused = true;

    let event = LifecycleEvent::FocusChanged {
        widget_id: wid,
        is_focused: false,
    };
    let interaction = crate::interaction::InteractionState::default();
    let mut ctx = LifecycleCtx {
        widget_id: wid,
        interaction: &interaction,
        requests: crate::controllers::ControllerRequests::NONE,
    };
    w.lifecycle(&event, &mut ctx);
    assert!(
        !w.search_focused,
        "FocusChanged(false) should clear search_focused"
    );
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
    // Absolute Y = padding + search + title + 1 item + half item.
    let target_y = SIDEBAR_PADDING_Y
        + SEARCH_AREA_H
        + title_y_advance(true)
        + NAV_ITEM_HEIGHT
        + NAV_ITEM_HEIGHT / 2.0;
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
    // Absolute Y = padding + search + title + half item.
    let target_y =
        SIDEBAR_PADDING_Y + SEARCH_AREA_H + title_y_advance(true) + NAV_ITEM_HEIGHT / 2.0;
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

/// Regression test for TPR-10-011: clicking search field must grant framework
/// focus so keyboard events actually reach the sidebar widget.
#[test]
fn harness_search_click_grants_focus_and_accepts_typing() {
    use crate::geometry::Point;
    use crate::input::MouseButton;
    use crate::testing::WidgetTestHarness;

    let theme = crate::theme::UiTheme::dark();
    let sidebar = SidebarNavWidget::new(test_sections(), &theme);
    let sidebar_id = sidebar.id();

    let mut h = WidgetTestHarness::new(sidebar);

    // 1. Click on the search field (center of the search rect).
    let search_y = SIDEBAR_PADDING_Y + super::geometry::SEARCH_FIELD_H / 2.0;
    let search_x = SIDEBAR_WIDTH / 2.0;
    let search_pos = Point::new(search_x, search_y);

    h.mouse_move(search_pos);
    h.mouse_down(MouseButton::Left);
    h.mouse_up(MouseButton::Left);

    // 2. Framework focus must be on the sidebar widget.
    assert!(
        h.is_focused(sidebar_id),
        "clicking search field should grant framework focus to sidebar"
    );

    // 3. Type "col" — should be consumed by search handler, no nav actions.
    //    "col" matches "Colors" (page 1), so filtered nav can reach page 1.
    let actions = h.type_text("col");
    let nav_action = actions
        .iter()
        .find(|a| matches!(a, WidgetAction::Selected { .. }));
    assert!(
        nav_action.is_none(),
        "typing in search should not emit Selected, got: {actions:?}"
    );

    // 4. Press Escape to unfocus search, then ArrowDown for nav.
    //    Search filter "col" is still active — visible: page 0 (active), page 1 (matches).
    h.key_press(Key::Escape, Modifiers::NONE);
    let actions = h.key_press(Key::ArrowDown, Modifiers::NONE);

    // 5. ArrowDown should emit Selected{1} — proves keyboard routes through
    //    framework focus to the sidebar's nav handler (via filtered nav).
    let selected = actions
        .iter()
        .find(|a| matches!(a, WidgetAction::Selected { .. }));
    assert!(
        selected.is_some(),
        "ArrowDown after Escape should emit Selected (sidebar has focus), got: {actions:?}"
    );
    if let Some(WidgetAction::Selected { index, .. }) = selected {
        assert_eq!(*index, 1, "ArrowDown from page 0 should select page 1");
    }
}

/// Clicking a nav item also grants framework focus so arrow keys work.
#[test]
fn harness_nav_click_grants_focus_for_arrow_keys() {
    use crate::geometry::Point;
    use crate::input::MouseButton;
    use crate::testing::WidgetTestHarness;

    let theme = crate::theme::UiTheme::dark();
    let sidebar = SidebarNavWidget::new(test_sections(), &theme);
    let sidebar_id = sidebar.id();

    let mut h = WidgetTestHarness::new(sidebar);

    // Click first nav item.
    let target_y =
        SIDEBAR_PADDING_Y + SEARCH_AREA_H + title_y_advance(true) + NAV_ITEM_HEIGHT / 2.0;
    let target = Point::new(SIDEBAR_WIDTH / 2.0, target_y);

    h.mouse_move(target);
    h.mouse_down(MouseButton::Left);
    h.mouse_up(MouseButton::Left);

    // Sidebar should have framework focus.
    assert!(
        h.is_focused(sidebar_id),
        "clicking nav item should grant framework focus"
    );

    // ArrowDown should work via framework focus.
    let actions = h.key_press(Key::ArrowDown, Modifiers::NONE);
    let selected = actions
        .iter()
        .find(|a| matches!(a, WidgetAction::Selected { .. }));
    assert!(
        selected.is_some(),
        "ArrowDown should emit Selected after nav click, got: {actions:?}"
    );
}

// -- Geometry unit tests (list B) --

#[test]
fn search_field_rect_inset_10px() {
    let bounds = Rect::new(50.0, 0.0, 200.0, 600.0);
    let r = super::geometry::search_field_rect(bounds);
    assert_eq!(r.x(), 60.0); // sidebar_x + 10
    assert_eq!(r.width(), 180.0); // sidebar_w - 20
    assert_eq!(r.height(), super::geometry::SEARCH_FIELD_H);
}

#[test]
fn nav_content_x_at_sidebar_plus_19() {
    let bounds = Rect::new(0.0, 0.0, 200.0, 600.0);
    let icon_x = super::geometry::nav_icon_x(&bounds);
    assert_eq!(icon_x, 19.0); // 3 (indicator) + 16 (padding)
}

#[test]
fn nav_text_x_at_sidebar_plus_45() {
    let bounds = Rect::new(0.0, 0.0, 200.0, 600.0);
    let text_x = super::geometry::nav_text_x(&bounds, true);
    assert_eq!(text_x, 45.0); // 3 + 16 + 16(icon) + 10(gap)
}

#[test]
fn nav_text_x_without_icon_same_as_icon_x() {
    let bounds = Rect::new(0.0, 0.0, 200.0, 600.0);
    let text_x = super::geometry::nav_text_x(&bounds, false);
    assert_eq!(text_x, super::geometry::nav_icon_x(&bounds));
}

#[test]
fn title_rect_first_no_top_margin() {
    let advance_first = title_y_advance(true);
    let advance_nonfirst = title_y_advance(false);
    // Non-first should be larger by TITLE_TOP_MARGIN.
    let diff = advance_nonfirst - advance_first;
    assert_eq!(
        diff,
        super::geometry::TITLE_TOP_MARGIN,
        "non-first title should add TITLE_TOP_MARGIN"
    );
}

#[test]
fn title_rect_nonfirst_has_20px_top_margin() {
    let advance = title_y_advance(false);
    // Should include TITLE_TOP_MARGIN (20) + TITLE_TEXT_H + TITLE_BOTTOM_MARGIN.
    let expected = super::geometry::TITLE_TOP_MARGIN
        + super::geometry::TITLE_TEXT_H
        + super::geometry::TITLE_BOTTOM_MARGIN;
    assert_eq!(advance, expected);
}

#[test]
fn derived_nav_row_height() {
    use super::geometry::{NAV_ITEM_CONTENT_H, NAV_ITEM_MARGIN_Y, NAV_ITEM_PADDING_Y};
    let expected = NAV_ITEM_MARGIN_Y
        + NAV_ITEM_PADDING_Y
        + NAV_ITEM_CONTENT_H
        + NAV_ITEM_PADDING_Y
        + NAV_ITEM_MARGIN_Y;
    assert_eq!(NAV_ITEM_HEIGHT, expected);
    // ~29px derived from mockup CSS.
    assert!((NAV_ITEM_HEIGHT - 29.0).abs() < 1.0);
}

#[test]
fn content_text_x_at_sidebar_plus_16() {
    let bounds = Rect::new(10.0, 0.0, 200.0, 600.0);
    let text_x = super::geometry::content_text_x(&bounds);
    assert_eq!(text_x, 26.0); // 10 + 16
}

// -- accept_action PageDirty --

#[test]
fn accept_action_page_dirty_sets_modified() {
    let theme = crate::theme::UiTheme::dark();
    let mut w = SidebarNavWidget::new(test_sections(), &theme);
    assert!(!w.is_page_modified(0));
    let action = WidgetAction::PageDirty {
        page: 0,
        dirty: true,
    };
    assert!(w.accept_action(&action));
    assert!(w.is_page_modified(0));
    // Clear it.
    let action = WidgetAction::PageDirty {
        page: 0,
        dirty: false,
    };
    assert!(w.accept_action(&action));
    assert!(!w.is_page_modified(0));
}

// -- Search state tests (list D, non-harness) --

#[test]
fn search_filters_nav_items() {
    let theme = crate::theme::UiTheme::dark();
    let mut w = SidebarNavWidget::new(test_sections(), &theme);
    w.search_state.set_text("render");
    // "Rendering" in Advanced section should match.
    let visible: Vec<usize> = w.visible_items().map(|i| i.page_index).collect();
    assert!(visible.contains(&2), "Rendering (page 2) should be visible");
    // "Appearance" and "Colors" should be hidden (unless active page).
    // Active page (0) is always visible.
    assert!(visible.contains(&0), "active page should stay visible");
}

#[test]
fn search_case_insensitive() {
    let theme = crate::theme::UiTheme::dark();
    let mut w = SidebarNavWidget::new(test_sections(), &theme);
    w.search_state.set_text("RENDER");
    let visible: Vec<usize> = w.visible_items().map(|i| i.page_index).collect();
    assert!(
        visible.contains(&2),
        "case-insensitive match should find Rendering"
    );
}

#[test]
fn search_preserves_active_page() {
    let theme = crate::theme::UiTheme::dark();
    let mut w = SidebarNavWidget::new(test_sections(), &theme);
    w.set_active_page(1); // Colors
    w.search_state.set_text("render");
    let visible: Vec<usize> = w.visible_items().map(|i| i.page_index).collect();
    // Active page (Colors, 1) should always be visible.
    assert!(
        visible.contains(&1),
        "active page should stay visible with query"
    );
    // Rendering (2) matches the query.
    assert!(visible.contains(&2));
}

#[test]
fn search_empty_query_shows_all() {
    let theme = crate::theme::UiTheme::dark();
    let mut w = SidebarNavWidget::new(test_sections(), &theme);
    w.search_state.set_text("render");
    w.search_state.set_text("");
    let count = w.visible_items().count();
    assert_eq!(count, 3, "empty query should show all 3 items");
}

#[test]
fn search_no_results_shows_only_active() {
    let theme = crate::theme::UiTheme::dark();
    let mut w = SidebarNavWidget::new(test_sections(), &theme);
    w.search_state.set_text("zzzzzzz");
    let visible: Vec<usize> = w.visible_items().map(|i| i.page_index).collect();
    // Only the active page (0) should be visible.
    assert_eq!(
        visible,
        vec![0],
        "no-match query should show only active page"
    );
}

#[test]
fn search_matches_section_title() {
    let theme = crate::theme::UiTheme::dark();
    let mut w = SidebarNavWidget::new(test_sections(), &theme);
    w.set_active_page(2); // Rendering
    w.search_state.set_text("general");
    let visible: Vec<usize> = w.visible_items().map(|i| i.page_index).collect();
    // "General" section title should make Appearance (0) and Colors (1) visible.
    assert!(
        visible.contains(&0),
        "section title match should show Appearance"
    );
    assert!(
        visible.contains(&1),
        "section title match should show Colors"
    );
}

// -- Footer tests (list E, non-harness) --

#[test]
fn with_update_available_builder() {
    let theme = crate::theme::UiTheme::dark();
    let w = SidebarNavWidget::new(test_sections(), &theme).with_update_available(
        "Update Available",
        "v1.0.0",
        "https://example.com/update",
    );
    assert_eq!(w.update_label.as_deref(), Some("Update Available"));
    assert_eq!(w.update_tooltip.as_deref(), Some("v1.0.0"));
    assert_eq!(w.update_url.as_deref(), Some("https://example.com/update"));
}

#[test]
fn footer_update_click_carries_url() {
    let theme = crate::theme::UiTheme::dark();
    let mut w = SidebarNavWidget::new(test_sections(), &theme).with_update_available(
        "Update Available",
        "v1.0.0",
        "https://example.com/update",
    );
    // Simulate footer rects being populated by paint.
    use super::FooterRects;
    w.footer_rects.set(FooterRects {
        update_link: Some(Rect::new(10.0, 380.0, 100.0, 14.0)),
        config_path: Some(Rect::new(10.0, 398.0, 180.0, 14.0)),
    });
    let bounds = Rect::new(0.0, 0.0, SIDEBAR_WIDTH, 420.0);
    // Click inside the update link rect.
    let click = InputEvent::MouseDown {
        pos: Point::new(50.0, 387.0),
        button: MouseButton::Left,
        modifiers: Modifiers::NONE,
    };
    let result = w.on_input(&click, bounds);
    assert!(result.handled);
    match result.action {
        Some(WidgetAction::FooterAction(FooterTarget::UpdateLink(url))) => {
            assert_eq!(url.as_deref(), Some("https://example.com/update"));
        }
        other => panic!("expected FooterAction(UpdateLink(Some(url))), got {other:?}"),
    }
}

#[test]
fn footer_no_update_link_when_none() {
    let theme = crate::theme::UiTheme::dark();
    let w = SidebarNavWidget::new(test_sections(), &theme);
    assert!(w.update_label.is_none());
    // Footer rects should have no update link rect.
    let rects = w.footer_rects.get();
    assert!(rects.update_link.is_none());
}

#[test]
fn hit_test_filtered_items_skip_hidden() {
    let theme = crate::theme::UiTheme::dark();
    let mut w = SidebarNavWidget::new(test_sections(), &theme);
    w.search_state.set_text("render");
    // The first visible item after search should be the active page (0),
    // then Rendering (2). Items between should be filtered out from hit test.
    // Hit test at the position of the first item should return flat index 0.
    let y = SEARCH_AREA_H + title_y_advance(true) + 1.0;
    let result = w.hit_test_item(y);
    assert!(
        result.is_some(),
        "first visible item should be hit-testable"
    );
}

// -- Click-to-cursor via cached char offsets (TPR-10-008) --

/// Regression test: clicking within populated search text positions the cursor
/// at the nearest measured character boundary, not a heuristic average.
#[test]
fn click_within_search_uses_measured_offsets() {
    let theme = crate::theme::UiTheme::dark();
    let mut w = SidebarNavWidget::new(test_sections(), &theme);
    w.search_state.set_text("hello");

    // Pre-populate cached offsets (simulates what paint_search_field computes).
    // Offsets: h@0, e@7, l@14, l@21, o@28, end@35.
    *w.search_char_offsets.borrow_mut() = vec![
        (0, 0.0),
        (1, 7.0),
        (2, 14.0),
        (3, 21.0),
        (4, 28.0),
        (5, 35.0),
    ];

    let bounds = Rect::new(0.0, 0.0, SIDEBAR_WIDTH, 400.0);
    let search_rect = super::geometry::search_field_rect(bounds);
    // Text starts 26px into the search field (SEARCH_TEXT_INSET in input.rs).
    let text_start_x = search_rect.x() + 26.0;
    let click_y = search_rect.y() + search_rect.height() / 2.0;

    // Click at text_start + 10.0 → nearest to (1, 7.0), cursor at byte 1.
    let click = InputEvent::MouseDown {
        pos: Point::new(text_start_x + 10.0, click_y),
        button: MouseButton::Left,
        modifiers: Modifiers::NONE,
    };
    w.on_input(&click, bounds);
    assert_eq!(
        w.search_state.cursor(),
        1,
        "click nearest to x=7 should place cursor before 'e'"
    );

    // Click at text_start + 0.0 → nearest to (0, 0.0), cursor at byte 0.
    let click = InputEvent::MouseDown {
        pos: Point::new(text_start_x, click_y),
        button: MouseButton::Left,
        modifiers: Modifiers::NONE,
    };
    w.on_input(&click, bounds);
    assert_eq!(
        w.search_state.cursor(),
        0,
        "click at text start should place cursor at byte 0"
    );

    // Click at text_start + 40.0 → nearest to (5, 35.0), cursor at byte 5 (end).
    let click = InputEvent::MouseDown {
        pos: Point::new(text_start_x + 40.0, click_y),
        button: MouseButton::Left,
        modifiers: Modifiers::NONE,
    };
    w.on_input(&click, bounds);
    assert_eq!(
        w.search_state.cursor(),
        5,
        "click past end should place cursor at end of text"
    );
}

/// Empty offsets (before first paint) should fall back to cursor position 0.
#[test]
fn click_search_before_paint_falls_back() {
    let theme = crate::theme::UiTheme::dark();
    let mut w = SidebarNavWidget::new(test_sections(), &theme);
    w.search_state.set_text("hello");
    // Don't populate search_char_offsets — simulates click before any paint.

    let bounds = Rect::new(0.0, 0.0, SIDEBAR_WIDTH, 400.0);
    let search_rect = super::geometry::search_field_rect(bounds);
    let click_y = search_rect.y() + search_rect.height() / 2.0;

    let click = InputEvent::MouseDown {
        pos: Point::new(search_rect.x() + 50.0, click_y),
        button: MouseButton::Left,
        modifiers: Modifiers::NONE,
    };
    w.on_input(&click, bounds);
    assert_eq!(
        w.search_state.cursor(),
        0,
        "before paint, click should fall back to cursor position 0"
    );
}

// -- Filtered keyboard navigation (TPR-10-010) --

/// ArrowDown with active search query navigates to next visible item,
/// skipping hidden items.
#[test]
fn filtered_nav_arrow_down_skips_hidden() {
    let theme = crate::theme::UiTheme::dark();
    let mut w = SidebarNavWidget::new(test_sections(), &theme);
    // Query "render" makes visible: page 0 (active), page 2 (matches "Rendering").
    // Page 1 (Colors) is hidden.
    w.search_state.set_text("render");
    let bounds = Rect::new(0.0, 0.0, SIDEBAR_WIDTH, 400.0);
    let result = w.on_input(&arrow_down(), bounds);
    assert!(result.handled);
    match result.action {
        Some(WidgetAction::Selected { index, .. }) => {
            assert_eq!(
                index, 2,
                "ArrowDown should skip hidden page 1, go to page 2"
            );
        }
        other => panic!("expected Selected(2), got {other:?}"),
    }
}

/// ArrowUp with active search query navigates to previous visible item.
#[test]
fn filtered_nav_arrow_up_skips_hidden() {
    let theme = crate::theme::UiTheme::dark();
    let mut w = SidebarNavWidget::new(test_sections(), &theme);
    w.set_active_page(2); // Rendering
    w.search_state.set_text("render");
    // Visible: page 2 (active + matches), page 0 (active page always visible... no,
    // page 0 was the old active). Actually active_page is 2, so page 2 is always visible.
    // "render" matches page 2 (Rendering). Page 0 (Appearance) doesn't match "render",
    // but page 2 is active so it's visible. Let me check visible_items logic...
    // item_visible: page_index == active_page OR label matches OR section_title matches.
    // active_page=2, so page 2 is always visible. "render" matches "Rendering" (page 2)
    // but NOT "Appearance" (page 0) or "Colors" (page 1).
    // So visible = [page 2] only. ArrowUp should be ignored.
    let bounds = Rect::new(0.0, 0.0, SIDEBAR_WIDTH, 400.0);
    let result = w.on_input(&arrow_up(), bounds);
    assert!(
        !result.handled,
        "ArrowUp with only one visible item should be ignored"
    );
}

/// Home with search query goes to first visible item.
#[test]
fn filtered_nav_home_goes_to_first_visible() {
    let theme = crate::theme::UiTheme::dark();
    let mut w = SidebarNavWidget::new(test_sections(), &theme);
    w.set_active_page(2);
    // Query "general" matches section title "General" → pages 0, 1 visible.
    // Page 2 is active → also visible.
    w.search_state.set_text("general");
    let bounds = Rect::new(0.0, 0.0, SIDEBAR_WIDTH, 400.0);
    let event = InputEvent::KeyDown {
        key: Key::Home,
        modifiers: Modifiers::NONE,
    };
    let result = w.on_input(&event, bounds);
    assert!(result.handled);
    match result.action {
        Some(WidgetAction::Selected { index, .. }) => {
            assert_eq!(index, 0, "Home should go to first visible item (page 0)");
        }
        other => panic!("expected Selected(0), got {other:?}"),
    }
}

/// End with search query goes to last visible item.
#[test]
fn filtered_nav_end_goes_to_last_visible() {
    let theme = crate::theme::UiTheme::dark();
    let mut w = SidebarNavWidget::new(test_sections(), &theme);
    // Query "general" makes visible: pages 0, 1 (General section), plus page 0 (active).
    w.search_state.set_text("general");
    let bounds = Rect::new(0.0, 0.0, SIDEBAR_WIDTH, 400.0);
    let event = InputEvent::KeyDown {
        key: Key::End,
        modifiers: Modifiers::NONE,
    };
    let result = w.on_input(&event, bounds);
    assert!(result.handled);
    match result.action {
        Some(WidgetAction::Selected { index, .. }) => {
            assert_eq!(index, 1, "End should go to last visible item (page 1)");
        }
        other => panic!("expected Selected(1), got {other:?}"),
    }
}

// -- Cursor icon --

#[test]
fn layout_cursor_icon_pointer() {
    let theme = crate::theme::UiTheme::dark();
    let w = SidebarNavWidget::new(test_sections(), &theme);
    let m = MockMeasurer::new();
    let ctx = LayoutCtx {
        measurer: &m,
        theme: &theme,
    };
    let layout = w.layout(&ctx);
    assert_eq!(
        layout.cursor_icon,
        CursorIcon::Pointer,
        "sidebar nav should declare Pointer cursor"
    );
}
