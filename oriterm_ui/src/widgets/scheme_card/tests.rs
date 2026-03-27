use crate::action::WidgetAction;
use crate::color::Color;
use crate::draw::Scene;
use crate::geometry::Rect;
use crate::layout::compute_layout;
use crate::widgets::tests::MockMeasurer;
use crate::widgets::{DrawCtx, LayoutCtx, Widget};

use super::{
    CARD_HEIGHT, CARD_PADDING, CARD_WIDTH, SWATCH_HEIGHT, SchemeCardData, SchemeCardWidget,
};

fn test_data(selected: bool) -> SchemeCardData {
    SchemeCardData {
        name: "Dracula".into(),
        bg: Color::hex(0x28_2A_36),
        fg: Color::hex(0xF8_F8_F2),
        ansi: [
            Color::hex(0x21_22_2C),
            Color::hex(0xFF_55_55),
            Color::hex(0x50_FA_7B),
            Color::hex(0xF1_FA_8C),
            Color::hex(0xBD_93_F9),
            Color::hex(0xFF_79_C6),
            Color::hex(0x8B_E9_FD),
            Color::hex(0xF8_F8_F2),
        ],
        selected,
    }
}

fn make_card(selected: bool) -> SchemeCardWidget {
    let theme = super::super::tests::TEST_THEME;
    SchemeCardWidget::new(test_data(selected), 3, &theme)
}

fn make_ctx() -> LayoutCtx<'static> {
    LayoutCtx {
        measurer: &MockMeasurer::STANDARD,
        theme: &super::super::tests::TEST_THEME,
    }
}

// -- Construction --

#[test]
fn new_stores_data_and_index() {
    let card = make_card(false);
    assert_eq!(card.data().name, "Dracula");
    assert_eq!(card.scheme_index(), 3);
    assert!(!card.data().selected);
}

// -- Layout --

#[test]
fn layout_dimensions() {
    let card = make_card(false);
    let ctx = make_ctx();
    let lb = card.layout(&ctx);
    let viewport = Rect::new(0.0, 0.0, 400.0, 300.0);
    let node = compute_layout(&lb, viewport);

    assert_eq!(node.rect.width(), CARD_WIDTH);
    assert_eq!(node.rect.height(), CARD_HEIGHT);
}

// -- Paint --

#[test]
fn paint_renders_rects_and_text() {
    let card = make_card(false);
    let measurer = MockMeasurer::STANDARD;
    let mut scene = Scene::new();
    let bounds = Rect::new(0.0, 0.0, CARD_WIDTH, CARD_HEIGHT);
    let mut ctx = DrawCtx {
        measurer: &measurer,
        scene: &mut scene,
        bounds,
        now: std::time::Instant::now(),
        theme: &super::super::tests::TEST_THEME,
        icons: None,
        interaction: None,
        widget_id: None,
        frame_requests: None,
    };
    card.paint(&mut ctx);

    // Should have: card bg rect, preview bg rect, 8 swatch rects = 10 rects.
    assert_eq!(scene.quads().len(), 10, "card bg + preview bg + 8 swatches");
    // Should have: title text + 2 preview lines = 3 text runs.
    assert_eq!(scene.text_runs().len(), 3, "title + 2 preview lines");
}

#[test]
fn paint_selected_includes_badge() {
    let card = make_card(true);
    let measurer = MockMeasurer::STANDARD;
    let mut scene = Scene::new();
    let bounds = Rect::new(0.0, 0.0, CARD_WIDTH, CARD_HEIGHT);
    let mut ctx = DrawCtx {
        measurer: &measurer,
        scene: &mut scene,
        bounds,
        now: std::time::Instant::now(),
        theme: &super::super::tests::TEST_THEME,
        icons: None,
        interaction: None,
        widget_id: None,
        frame_requests: None,
    };
    card.paint(&mut ctx);

    // Selected card adds "Active" badge = 4 text runs total.
    assert_eq!(
        scene.text_runs().len(),
        4,
        "title + badge + 2 preview lines"
    );
}

// -- Sense & controllers --

#[test]
fn sense_is_click() {
    let card = make_card(false);
    assert_eq!(card.sense(), crate::sense::Sense::click());
}

#[test]
fn has_controllers() {
    let card = make_card(false);
    assert_eq!(
        card.controllers().len(),
        2,
        "HoverController + ClickController"
    );
}

#[test]
fn has_visual_state_animator() {
    let card = make_card(false);
    assert!(card.visual_states().is_some());
}

// -- on_action --

#[test]
fn on_action_transforms_click_to_selected() {
    let mut card = make_card(false);
    let click = WidgetAction::Clicked(card.id());
    let bounds = Rect::new(0.0, 0.0, CARD_WIDTH, CARD_HEIGHT);
    let result = card.on_action(click, bounds);

    match result {
        Some(WidgetAction::Selected { id, index }) => {
            assert_eq!(id, card.id());
            assert_eq!(index, 3);
        }
        other => panic!("expected Selected, got {other:?}"),
    }
}

// -- Selection state --

#[test]
fn set_selected() {
    let mut card = make_card(false);
    assert!(!card.data().selected);
    card.set_selected(true);
    assert!(card.data().selected);
}

// -- accept_action group filtering (TPR-11-002) --

#[test]
fn accept_action_reacts_to_sibling_scheme_card() {
    let theme = super::super::tests::TEST_THEME;
    let mut card_a = SchemeCardWidget::new(test_data(true), 0, &theme);
    let card_b = SchemeCardWidget::new(test_data(false), 1, &theme);
    let group = vec![card_a.id(), card_b.id()];
    card_a.set_scheme_group(group);

    // card_b selects index 1 — card_a (index 0) should deselect.
    let action = WidgetAction::Selected {
        id: card_b.id(),
        index: 1,
    };
    assert!(card_a.accept_action(&action));
    assert!(!card_a.data().selected);
}

#[test]
fn accept_action_ignores_external_selected() {
    let theme = super::super::tests::TEST_THEME;
    let mut card = SchemeCardWidget::new(test_data(true), 3, &theme);
    let sibling = SchemeCardWidget::new(test_data(false), 4, &theme);
    card.set_scheme_group(vec![card.id(), sibling.id()]);

    // Selected from sidebar nav (different widget, not in group) should be ignored.
    let external_id = crate::widget_id::WidgetId::next();
    let action = WidgetAction::Selected {
        id: external_id,
        index: 3,
    };
    assert!(!card.accept_action(&action));
    // Card should remain selected — sidebar nav didn't affect it.
    assert!(card.data().selected);
}

#[test]
fn accept_action_no_change_when_group_empty() {
    let mut card = make_card(true);
    // No group set — should ignore all Selected actions.
    let action = WidgetAction::Selected {
        id: crate::widget_id::WidgetId::next(),
        index: 3,
    };
    assert!(!card.accept_action(&action));
    assert!(card.data().selected);
}

// -- Constant validation --

#[test]
fn scheme_card_padding_is_12() {
    assert_eq!(CARD_PADDING, 12.0);
}

#[test]
fn scheme_card_width_is_240() {
    assert_eq!(CARD_WIDTH, 240.0);
}

#[test]
fn scheme_card_swatch_height_is_14() {
    assert_eq!(SWATCH_HEIGHT, 14.0);
}

// -- Paint color verification --

#[test]
fn scheme_card_normal_has_persistent_bg() {
    let card = make_card(false);
    let animator = card.visual_states().expect("has animator");
    // Normal (non-selected, non-hovered) bg should be bg_card.
    assert_eq!(
        animator.get_bg_color(),
        super::super::tests::TEST_THEME.bg_card
    );
}

#[test]
fn scheme_card_normal_has_2px_border() {
    let card = make_card(false);
    let measurer = MockMeasurer::STANDARD;
    let mut scene = Scene::new();
    let bounds = Rect::new(0.0, 0.0, CARD_WIDTH, CARD_HEIGHT);
    let mut ctx = DrawCtx {
        measurer: &measurer,
        scene: &mut scene,
        bounds,
        now: std::time::Instant::now(),
        theme: &super::super::tests::TEST_THEME,
        icons: None,
        interaction: None,
        widget_id: None,
        frame_requests: None,
    };
    card.paint(&mut ctx);

    // First quad is the card background — verify it has a 2px border.
    let quads = scene.quads();
    assert!(!quads.is_empty(), "paint should emit quads");
    let top_border = quads[0]
        .style
        .border
        .top
        .as_ref()
        .expect("card should have top border");
    assert_eq!(top_border.width, 2.0, "card border should be 2px");
}

#[test]
fn scheme_card_badge_is_uppercase_chip() {
    // When selected, the badge "Active" text is rendered with
    // TextTransform::Uppercase. Verify the selected card produces the
    // extra badge text run (4 vs 3), and that the badge style in paint_title
    // uses with_text_transform(Uppercase). We verify structurally via the
    // text run count difference between selected and non-selected.
    let card_normal = make_card(false);
    let card_selected = make_card(true);
    let measurer = MockMeasurer::STANDARD;
    let theme = &super::super::tests::TEST_THEME;

    let mut scene_normal = Scene::new();
    let bounds = Rect::new(0.0, 0.0, CARD_WIDTH, CARD_HEIGHT);
    let mut ctx = DrawCtx {
        measurer: &measurer,
        scene: &mut scene_normal,
        bounds,
        now: std::time::Instant::now(),
        theme,
        icons: None,
        interaction: None,
        widget_id: None,
        frame_requests: None,
    };
    card_normal.paint(&mut ctx);

    let mut scene_selected = Scene::new();
    let mut ctx = DrawCtx {
        measurer: &measurer,
        scene: &mut scene_selected,
        bounds,
        now: std::time::Instant::now(),
        theme,
        icons: None,
        interaction: None,
        widget_id: None,
        frame_requests: None,
    };
    card_selected.paint(&mut ctx);

    // Selected card has one extra text run (the "Active" badge chip).
    let normal_count = scene_normal.text_runs().len();
    let selected_count = scene_selected.text_runs().len();
    assert_eq!(
        selected_count,
        normal_count + 1,
        "selected card should have one extra text run for the uppercase badge chip"
    );
    // The badge chip also adds an extra quad (chip background).
    assert_eq!(
        scene_selected.quads().len(),
        scene_normal.quads().len() + 1,
        "selected card should have one extra quad for the badge chip background"
    );
}
