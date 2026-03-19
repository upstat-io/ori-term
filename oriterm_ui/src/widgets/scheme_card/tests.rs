use crate::action::WidgetAction;
use crate::color::Color;
use crate::draw::Scene;
use crate::geometry::Rect;
use crate::layout::compute_layout;
use crate::widgets::tests::MockMeasurer;
use crate::widgets::{DrawCtx, LayoutCtx, Widget};

use super::{CARD_HEIGHT, CARD_WIDTH, SchemeCardData, SchemeCardWidget};

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
