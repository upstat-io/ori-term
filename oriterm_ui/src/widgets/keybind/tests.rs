use crate::draw::{DrawCommand, DrawList};
use crate::geometry::Rect;
use crate::layout::compute_layout;
use crate::sense::Sense;
use crate::widgets::tests::MockMeasurer;
use crate::widgets::{DrawCtx, LayoutCtx, Widget};

use super::{KbdBadge, KeybindRow};

fn theme() -> &'static crate::theme::UiTheme {
    &super::super::tests::TEST_THEME
}

// -- KbdBadge --

#[test]
fn badge_stores_key() {
    let b = KbdBadge::new("Ctrl");
    assert_eq!(b.key(), "Ctrl");
}

#[test]
fn badge_sense_is_none() {
    let b = KbdBadge::new("A");
    assert_eq!(b.sense(), Sense::none());
}

#[test]
fn badge_layout_has_positive_size() {
    let b = KbdBadge::new("Shift");
    let ctx = LayoutCtx {
        measurer: &MockMeasurer::STANDARD,
        theme: theme(),
    };
    let lb = b.layout(&ctx);
    let node = compute_layout(&lb, Rect::new(0.0, 0.0, 200.0, 100.0));
    assert!(node.rect.width() > 0.0);
    assert!(node.rect.height() > 0.0);
}

#[test]
fn badge_paint_produces_rects_and_text() {
    let b = KbdBadge::new("Tab");
    let measurer = MockMeasurer::STANDARD;
    let mut draw_list = DrawList::new();
    let bounds = Rect::new(0.0, 0.0, 40.0, 28.0);
    let mut ctx = DrawCtx {
        measurer: &measurer,
        draw_list: &mut draw_list,
        bounds,
        now: std::time::Instant::now(),
        theme: theme(),
        icons: None,
        scene_cache: None,
        interaction: None,
        widget_id: None,
        frame_requests: None,
    };
    b.paint(&mut ctx);

    // 2 rects: body + bottom border.
    let rects = draw_list
        .commands()
        .iter()
        .filter(|c| matches!(c, DrawCommand::Rect { .. }))
        .count();
    assert_eq!(rects, 2, "body + bottom border");

    // 1 text: key label.
    let texts = draw_list
        .commands()
        .iter()
        .filter(|c| matches!(c, DrawCommand::Text { .. }))
        .count();
    assert_eq!(texts, 1);
}

// -- KeybindRow --

#[test]
fn row_stores_action_and_keys() {
    let row = KeybindRow::new("Copy", vec!["Ctrl".into(), "C".into()], theme());
    assert_eq!(row.action_name(), "Copy");
    assert_eq!(row.keys(), &["Ctrl", "C"]);
}

#[test]
fn row_sense_is_hover() {
    let row = KeybindRow::new("Paste", vec!["Ctrl".into(), "V".into()], theme());
    assert_eq!(row.sense(), Sense::hover());
}

#[test]
fn row_layout_has_positive_size() {
    let row = KeybindRow::new("Paste", vec!["Ctrl".into(), "V".into()], theme());
    let ctx = LayoutCtx {
        measurer: &MockMeasurer::STANDARD,
        theme: theme(),
    };
    let lb = row.layout(&ctx);
    let node = compute_layout(&lb, Rect::new(0.0, 0.0, 400.0, 100.0));
    assert!(node.rect.width() > 0.0);
    assert!(node.rect.height() > 0.0);
}

#[test]
fn row_paint_produces_action_label_and_badges() {
    let row = KeybindRow::new("Copy", vec!["Ctrl".into(), "C".into()], theme());
    let measurer = MockMeasurer::STANDARD;
    let mut draw_list = DrawList::new();
    let bounds = Rect::new(0.0, 0.0, 300.0, 36.0);
    let mut ctx = DrawCtx {
        measurer: &measurer,
        draw_list: &mut draw_list,
        bounds,
        now: std::time::Instant::now(),
        theme: theme(),
        icons: None,
        scene_cache: None,
        interaction: None,
        widget_id: None,
        frame_requests: None,
    };
    row.paint(&mut ctx);

    // Rects: 2 badge bodies + 2 bottom borders = 4.
    let rects = draw_list
        .commands()
        .iter()
        .filter(|c| matches!(c, DrawCommand::Rect { .. }))
        .count();
    assert_eq!(rects, 4, "2 badge bodies + 2 bottom borders");

    // Texts: 1 action name + 1 "+" separator + 2 key labels = 4.
    let texts = draw_list
        .commands()
        .iter()
        .filter(|c| matches!(c, DrawCommand::Text { .. }))
        .count();
    assert_eq!(texts, 4, "action + plus + 2 keys");
}

#[test]
fn row_has_hover_controller() {
    let row = KeybindRow::new("X", vec!["A".into()], theme());
    assert_eq!(row.controllers().len(), 1);
}

#[test]
fn row_has_visual_state_animator() {
    let row = KeybindRow::new("X", vec!["A".into()], theme());
    assert!(row.visual_states().is_some());
}
