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
    // 31 fixed control IDs (30 controls + sidebar) + N scheme card IDs.
    let expected = 31 + ids.scheme_card_ids.len();
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

/// Regression test for sidebar_id must be captured so
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

/// Regression test for update info wiring through the builder.
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
    set.insert(ids.cursor_blink_fade_toggle.raw());
    set.insert(ids.text_blink_fade_toggle.raw());
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

/// Configured-but-uninstalled font family — when `Config.font.family` names a
/// family the host system does not have installed, the dropdown still includes
/// the configured name (prepended at index 1, after "Default (System)"), so the
/// open dialog reflects the saved configuration faithfully rather than silently
/// snapping to "Default (System)" (closes opencode F2 GAP per
/// `bug-tracker/plans/BUG-02-012/section-06-tpr-findings.md` Round 1).
#[test]
fn font_family_dropdown_prepends_configured_uninstalled_family() {
    let mut config = Config::default();
    let bogus = "ZZZ_NotInstalled_Family_For_Testing";
    config.font.family = Some(bogus.to_owned());
    let theme = UiTheme::default();
    let (_content, ids, _footer) = build_settings_dialog(&config, &theme, 0, 1.0, 1.0, None);

    assert!(
        ids.font_family_items.len() >= 2,
        "items must always have Default + at least one more (the configured family) — got {}",
        ids.font_family_items.len()
    );
    assert_eq!(
        ids.font_family_items[0], "Default (System)",
        "index 0 is always the Default sentinel"
    );
    assert_eq!(
        ids.font_family_items[1], bogus,
        "configured-but-uninstalled family must be prepended at index 1"
    );
    let count = ids
        .font_family_items
        .iter()
        .filter(|s| s.eq_ignore_ascii_case(bogus))
        .count();
    assert_eq!(
        count, 1,
        "configured family must appear exactly once (no double-insert if also enumerated)"
    );
}

/// When the configured family IS installed (or matches an enumerated family),
/// the dropdown does NOT prepend a duplicate entry.
#[test]
fn font_family_dropdown_skips_prepend_when_family_is_enumerated() {
    let catalog = crate::font::discovery::enumerate_mono_families();
    let Some(installed) = catalog.first().map(|fe| fe.display_name.clone()) else {
        eprintln!(
            "SKIP: no enumerated families on this host — prepend skip path is host-dependent"
        );
        return;
    };
    let mut config = Config::default();
    config.font.family = Some(installed.clone());
    let theme = UiTheme::default();
    let (_content, ids, _footer) = build_settings_dialog(&config, &theme, 0, 1.0, 1.0, None);

    let count = ids
        .font_family_items
        .iter()
        .filter(|s| s.eq_ignore_ascii_case(&installed))
        .count();
    assert_eq!(
        count, 1,
        "installed family appears exactly once (the enumeration entry); no prepend duplicate"
    );
}
