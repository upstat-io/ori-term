//! Tests for the settings dialog builder.

use std::collections::HashSet;

use oriterm_ui::theme::UiTheme;

use super::{SettingsIds, build_settings_dialog};
use crate::config::Config;

#[test]
fn dialog_builds_without_panic() {
    let config = Config::default();
    let theme = UiTheme::default();
    let (_content, _ids, _footer_ids) = build_settings_dialog(&config, &theme, 0, 1.0, 1.0, None);
}

#[test]
fn settings_ids_all_distinct() {
    let config = Config::default();
    let theme = UiTheme::default();
    let (_content, ids, _footer_ids) = build_settings_dialog(&config, &theme, 0, 1.0, 1.0, None);
    let all = collect_ids(&ids);
    // 29 fixed control IDs (28 controls + sidebar) + N scheme card IDs.
    let expected = 29 + ids.scheme_card_ids.len();
    assert_eq!(all.len(), expected, "all widget IDs must be distinct");
}

#[test]
fn content_widget_has_valid_id() {
    let config = Config::default();
    let theme = UiTheme::default();
    let (content, _ids, _footer_ids) = build_settings_dialog(&config, &theme, 0, 1.0, 1.0, None);
    assert_ne!(content.id().raw(), 0);
}

#[test]
fn all_page_ids_are_set() {
    let config = Config::default();
    let theme = UiTheme::default();
    let (_content, ids, _footer_ids) = build_settings_dialog(&config, &theme, 0, 1.0, 1.0, None);
    let all = collect_ids(&ids);
    // Every ID must be non-placeholder.
    assert!(
        all.iter().all(|id| *id != 0),
        "no placeholder IDs should remain"
    );
}

#[test]
fn scheme_card_ids_captured() {
    let config = Config::default();
    let theme = UiTheme::default();
    let (_content, ids, _footer_ids) = build_settings_dialog(&config, &theme, 0, 1.0, 1.0, None);
    // Scheme cards are captured during colors page building.
    assert!(
        !ids.scheme_card_ids.is_empty(),
        "scheme card IDs must be captured"
    );
}

/// Regression test for TPR-11-001: sidebar_id must be captured so
/// `dispatch_dialog_settings_action` can gate `active_page` updates.
#[test]
fn sidebar_id_captured() {
    let config = Config::default();
    let theme = UiTheme::default();
    let (_content, ids, _footer_ids) = build_settings_dialog(&config, &theme, 0, 1.0, 1.0, None);
    assert_ne!(
        ids.sidebar_id,
        oriterm_ui::widget_id::WidgetId::placeholder(),
        "sidebar_id must be non-placeholder"
    );
    // Must be distinct from any scheme card ID.
    assert!(
        !ids.scheme_card_ids.contains(&ids.sidebar_id),
        "sidebar_id must not collide with scheme card IDs"
    );
}

/// Regression test for TPR-10-016: update info wiring through the builder.
#[test]
fn dialog_builds_with_update_info() {
    let config = Config::default();
    let theme = UiTheme::default();
    let info = Some((
        "Update Available",
        "v2.0.0 ready",
        "https://example.com/update",
    ));
    let (content, ids, _footer_ids) = build_settings_dialog(&config, &theme, 0, 1.0, 1.0, info);
    // Sidebar must still be captured.
    assert_ne!(
        ids.sidebar_id,
        oriterm_ui::widget_id::WidgetId::placeholder(),
        "sidebar_id must be non-placeholder when update info is provided"
    );
    assert_ne!(content.id().raw(), 0);
}

// -- Composition tests --

#[test]
fn footer_buttons_reachable_through_widget_tree() {
    use oriterm_ui::widgets::Widget;
    use oriterm_ui::widgets::settings_panel::SettingsPanel;

    let config = Config::default();
    let theme = UiTheme::default();
    let (content, _ids, footer_ids) = build_settings_dialog(&config, &theme, 0, 1.0, 1.0, None);
    let panel = SettingsPanel::embedded(content, footer_ids);
    let focusable = panel.focusable_children();

    let (reset_id, cancel_id, _save_id) = footer_ids;
    assert!(
        focusable.contains(&reset_id),
        "reset button should be reachable through focusable_children"
    );
    assert!(
        focusable.contains(&cancel_id),
        "cancel button should be reachable through focusable_children"
    );
    // Save is initially disabled, so not focusable — that's correct behavior.
}

#[test]
fn accept_unsaved_reaches_footer() {
    use oriterm_ui::action::WidgetAction;
    use oriterm_ui::widgets::Widget;
    use oriterm_ui::widgets::settings_panel::SettingsPanel;

    let config = Config::default();
    let theme = UiTheme::default();
    let (content, _ids, footer_ids) = build_settings_dialog(&config, &theme, 0, 1.0, 1.0, None);
    let mut panel = SettingsPanel::embedded(content, footer_ids);

    let handled = panel.accept_action(&WidgetAction::SettingsUnsaved(true));
    assert!(
        handled,
        "SettingsUnsaved should be handled by the footer through the panel"
    );
}

#[test]
fn footer_buttons_have_correct_height() {
    use oriterm_ui::geometry::Rect;
    use oriterm_ui::layout::compute_layout;
    use oriterm_ui::widgets::Widget;
    use oriterm_ui::widgets::settings_panel::SettingsPanel;

    let config = Config::default();
    let theme = UiTheme::default();
    let (content, _ids, footer_ids) = build_settings_dialog(&config, &theme, 0, 1.0, 1.0, None);
    let panel = SettingsPanel::embedded(content, footer_ids);

    // Simulate dialog dimensions (860×620 at logical pixels).
    let measurer = oriterm_ui::testing::MockMeasurer::STANDARD;
    let ctx = oriterm_ui::widgets::LayoutCtx {
        measurer: &measurer,
        theme: &theme,
    };
    let lb = panel.layout(&ctx);
    let viewport = Rect::new(0.0, 0.0, 860.0, 620.0);
    let root = compute_layout(&lb, viewport);

    // Walk the tree to find nodes with the footer button IDs.
    let (reset_id, cancel_id, save_id) = footer_ids;
    let ids = [reset_id, cancel_id, save_id];

    fn find_by_id(
        node: &oriterm_ui::layout::LayoutNode,
        id: oriterm_ui::widget_id::WidgetId,
    ) -> Option<Rect> {
        if node.widget_id == Some(id) {
            return Some(node.rect);
        }
        for child in &node.children {
            if let Some(r) = find_by_id(child, id) {
                return Some(r);
            }
        }
        None
    }

    for &id in &ids {
        let rect = find_by_id(&root, id);
        assert!(rect.is_some(), "button {id:?} not found in layout tree");
        let rect = rect.unwrap();
        assert!(
            rect.height() >= 20.0,
            "button {id:?} height is {}, expected >= 20px (rect: {rect:?})",
            rect.height()
        );
    }
}

fn collect_ids(ids: &SettingsIds) -> HashSet<u64> {
    let mut set = HashSet::new();
    // Navigation.
    set.insert(ids.sidebar_id.raw());
    // Appearance.
    set.insert(ids.theme_dropdown.raw());
    set.insert(ids.opacity_slider.raw());
    set.insert(ids.blur_toggle.raw());
    set.insert(ids.unfocused_opacity_slider.raw());
    set.insert(ids.decorations_dropdown.raw());
    set.insert(ids.tab_bar_style_dropdown.raw());
    // Colors — per-card IDs.
    for card_id in &ids.scheme_card_ids {
        set.insert(card_id.raw());
    }
    // Font.
    set.insert(ids.font_family_dropdown.raw());
    set.insert(ids.font_size_input.raw());
    set.insert(ids.font_weight_dropdown.raw());
    set.insert(ids.ligatures_toggle.raw());
    set.insert(ids.line_height_input.raw());
    // Font — Advanced.
    set.insert(ids.hinting_dropdown.raw());
    set.insert(ids.subpixel_aa_dropdown.raw());
    set.insert(ids.subpixel_positioning_dropdown.raw());
    set.insert(ids.atlas_filtering_dropdown.raw());
    // Terminal.
    set.insert(ids.cursor_picker.raw());
    set.insert(ids.cursor_blink_toggle.raw());
    set.insert(ids.scrollback_input.raw());
    set.insert(ids.shell_input.raw());
    set.insert(ids.paste_warning_dropdown.raw());
    // Window.
    set.insert(ids.tab_bar_position_dropdown.raw());
    set.insert(ids.grid_padding_input.raw());
    set.insert(ids.restore_session_toggle.raw());
    set.insert(ids.initial_columns_input.raw());
    set.insert(ids.initial_rows_input.raw());
    // Bell.
    set.insert(ids.bell_animation_dropdown.raw());
    set.insert(ids.bell_duration_dropdown.raw());
    // Rendering.
    set.insert(ids.gpu_backend_dropdown.raw());
    set
}
