//! Tests for `SettingsFooterWidget`.

use crate::action::WidgetAction;
use crate::geometry::Rect;
use crate::layout::compute_layout;
use crate::theme::UiTheme;
use crate::widgets::tests::MockMeasurer;
use crate::widgets::{LayoutCtx, Widget};

use super::{FOOTER_HEIGHT, SettingsFooterWidget};

fn make_footer() -> SettingsFooterWidget {
    let theme = UiTheme::dark();
    SettingsFooterWidget::new(&theme)
}

#[test]
fn new_does_not_panic() {
    let _footer = make_footer();
}

#[test]
fn initial_dirty_is_false() {
    let footer = make_footer();
    assert!(!footer.dirty);
}

#[test]
fn focusable_children_clean_excludes_save() {
    let footer = make_footer();
    let (reset_id, cancel_id, save_id) = footer.button_ids();
    let focusable = footer.focusable_children();

    assert!(focusable.contains(&reset_id), "reset should be focusable");
    assert!(focusable.contains(&cancel_id), "cancel should be focusable");
    assert!(
        !focusable.contains(&save_id),
        "save should NOT be focusable when clean"
    );
    assert_eq!(focusable.len(), 2, "2 focusable buttons when clean");

    // IDs should be distinct.
    assert_ne!(reset_id, cancel_id);
    assert_ne!(cancel_id, save_id);
    assert_ne!(reset_id, save_id);
}

#[test]
fn focusable_children_dirty_includes_save() {
    let mut footer = make_footer();
    footer.accept_action(&WidgetAction::SettingsUnsaved(true));
    let (reset_id, cancel_id, save_id) = footer.button_ids();
    let focusable = footer.focusable_children();

    assert!(focusable.contains(&reset_id), "reset should be focusable");
    assert!(focusable.contains(&cancel_id), "cancel should be focusable");
    assert!(
        focusable.contains(&save_id),
        "save should be focusable when dirty"
    );
    assert_eq!(focusable.len(), 3, "3 focusable buttons when dirty");
}

#[test]
fn button_ids_are_distinct() {
    let footer = make_footer();
    let (r, c, s) = footer.button_ids();
    assert_ne!(r, c);
    assert_ne!(c, s);
    assert_ne!(r, s);
}

#[test]
fn on_action_passes_through() {
    let mut footer = make_footer();
    let other_id = crate::widget_id::WidgetId::next();
    let action = WidgetAction::Clicked(other_id);
    let bounds = Rect::new(0.0, 0.0, 600.0, 52.0);
    let result = footer.on_action(action.clone(), bounds);
    assert_eq!(result, Some(action));
}

#[test]
fn accept_unsaved_true_sets_dirty() {
    let mut footer = make_footer();
    let handled = footer.accept_action(&WidgetAction::SettingsUnsaved(true));
    assert!(handled);
    assert!(footer.dirty);
}

#[test]
fn accept_unsaved_false_clears_dirty() {
    let mut footer = make_footer();
    footer.accept_action(&WidgetAction::SettingsUnsaved(true));
    footer.accept_action(&WidgetAction::SettingsUnsaved(false));
    assert!(!footer.dirty);
}

// -- Layout tests --

fn layout_footer(footer: &SettingsFooterWidget) -> crate::layout::LayoutNode {
    let m = MockMeasurer::STANDARD;
    let ctx = LayoutCtx {
        measurer: &m,
        theme: &crate::testing::TEST_THEME,
    };
    let lb = footer.layout(&ctx);
    compute_layout(&lb, Rect::new(0.0, 0.0, 600.0, FOOTER_HEIGHT))
}

#[test]
fn footer_fixed_height() {
    let footer = make_footer();
    let node = layout_footer(&footer);
    assert!(
        (node.rect.height() - FOOTER_HEIGHT).abs() < 0.01,
        "footer height should be {FOOTER_HEIGHT}, got {}",
        node.rect.height()
    );
}

#[test]
fn unsaved_hidden_when_clean() {
    let footer = make_footer();
    let node = layout_footer(&footer);
    // children: [margin_left, indicator, fill, reset, gap, cancel, gap, save, margin_right]
    let indicator = &node.children[1];
    assert!(
        indicator.content_rect.width() < 0.01,
        "indicator should be collapsed when clean, got width {}",
        indicator.content_rect.width()
    );
}

#[test]
fn unsaved_visible_when_dirty() {
    let mut footer = make_footer();
    footer.accept_action(&WidgetAction::SettingsUnsaved(true));
    let node = layout_footer(&footer);
    let indicator = &node.children[1];
    assert!(
        indicator.content_rect.width() > 1.0,
        "indicator should be visible when dirty, got width {}",
        indicator.content_rect.width()
    );
}

#[test]
fn unsaved_group_does_not_overlap_reset() {
    let mut footer = make_footer();
    footer.accept_action(&WidgetAction::SettingsUnsaved(true));
    let node = layout_footer(&footer);
    // children: [margin_left, indicator, fill, reset, gap, cancel, gap, save, margin_right]
    let indicator_right = node.children[1].rect.x() + node.children[1].rect.width();
    let reset_left = node.children[3].rect.x();
    assert!(
        indicator_right <= reset_left + 0.01,
        "indicator right edge ({indicator_right}) should not overlap reset left ({reset_left})"
    );
}

#[test]
fn accept_unsaved_updates_indicator_visibility() {
    let mut footer = make_footer();

    // Start clean: indicator collapsed.
    let node = layout_footer(&footer);
    assert!(node.children[1].content_rect.width() < 0.01);

    // Make dirty: indicator visible.
    footer.accept_action(&WidgetAction::SettingsUnsaved(true));
    let node = layout_footer(&footer);
    assert!(node.children[1].content_rect.width() > 1.0);

    // Make clean again: indicator collapsed.
    footer.accept_action(&WidgetAction::SettingsUnsaved(false));
    let node = layout_footer(&footer);
    assert!(node.children[1].content_rect.width() < 0.01);
}

// -- Save button disabled opacity (TPR-12-012) --

#[test]
fn save_button_uses_opacity_fade_when_disabled() {
    let footer = make_footer();
    // Save button starts disabled (clean state). Paint and check that the
    // Save button's background uses alpha modulation, not the legacy color swap.
    let scene = paint_footer(&footer);
    let quads = scene.quads();
    // The Save button is the last painted button. Its bg quad should have
    // alpha < 1.0 because disabled_opacity=0.4 modulates the accent color.
    let save_bg = quads
        .iter()
        .filter(|q| {
            if let Some(fill) = q.style.fill {
                // Accent color with alpha modulation (0.4) will have a < 1.0.
                fill.a < 0.99 && fill.a > 0.01
            } else {
                false
            }
        })
        .last();
    assert!(
        save_bg.is_some(),
        "disabled Save button should have an alpha-modulated bg quad (opacity fade), \
         but no quads with 0.01 < alpha < 0.99 were found"
    );
}

// -- Paint tests --

fn paint_footer(footer: &SettingsFooterWidget) -> crate::draw::Scene {
    let m = MockMeasurer::STANDARD;
    let mut scene = crate::draw::Scene::new();
    let bounds = Rect::new(0.0, 0.0, 600.0, FOOTER_HEIGHT);
    let theme = crate::testing::TEST_THEME;
    let mut ctx = crate::widgets::DrawCtx {
        measurer: &m,
        scene: &mut scene,
        bounds,
        now: std::time::Instant::now(),
        theme: &theme,
        icons: None,
        interaction: None,
        widget_id: None,
        frame_requests: None,
    };
    footer.paint(&mut ctx);
    scene
}

#[test]
fn paint_produces_separator_quad() {
    let footer = make_footer();
    let scene = paint_footer(&footer);
    let theme = UiTheme::dark();
    // First quad should be a 2px-tall separator at the top.
    let quads = scene.quads();
    let sep = quads
        .iter()
        .find(|q| q.style.fill == Some(theme.border) && (q.bounds.height() - 2.0).abs() < 0.01);
    assert!(
        sep.is_some(),
        "should have a 2px separator quad with border color"
    );
}

#[test]
fn paint_dirty_renders_warning_text() {
    let mut footer = make_footer();
    footer.accept_action(&WidgetAction::SettingsUnsaved(true));
    let scene = paint_footer(&footer);
    // Should have text runs (button labels + indicator label).
    let text_runs = scene.text_runs();
    assert!(!text_runs.is_empty(), "dirty footer should have text runs");
}

#[test]
fn paint_clean_no_warning_text_from_indicator() {
    let footer = make_footer();
    let scene = paint_footer(&footer);
    // When clean, the indicator is hidden (DisplayNone), so its text should
    // not appear. Button labels still appear (Reset, Cancel, Save).
    let text_runs = scene.text_runs();
    // Button labels: 3 buttons each with a text run.
    // No indicator text should be present.
    assert!(
        text_runs.len() <= 3,
        "clean footer should have at most 3 text runs (buttons only), got {}",
        text_runs.len()
    );
}
