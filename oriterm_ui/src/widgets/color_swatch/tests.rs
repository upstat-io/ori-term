use crate::action::WidgetAction;
use crate::color::Color;
use crate::draw::{DrawCommand, DrawList};
use crate::geometry::{Point, Rect};
use crate::input::InputEvent;
use crate::layout::compute_layout;
use crate::sense::Sense;
use crate::widgets::tests::MockMeasurer;
use crate::widgets::{DrawCtx, LayoutCtx, Widget};

use super::{CELL_GAP, CELL_SIZE, ColorSwatchGrid, GRID_COLUMNS, ROW_HEIGHT, SpecialColorSwatch};

fn ansi_colors() -> Vec<Color> {
    vec![
        Color::hex(0x00_00_00),
        Color::hex(0xCC_00_00),
        Color::hex(0x00_CC_00),
        Color::hex(0xCC_CC_00),
        Color::hex(0x00_00_CC),
        Color::hex(0xCC_00_CC),
        Color::hex(0x00_CC_CC),
        Color::hex(0xCC_CC_CC),
    ]
}

fn theme() -> &'static crate::theme::UiTheme {
    &super::super::tests::TEST_THEME
}

// -- ColorSwatchGrid --

#[test]
fn grid_color_count() {
    let grid = ColorSwatchGrid::new(ansi_colors(), theme());
    assert_eq!(grid.color_count(), 8);
}

#[test]
fn grid_layout_one_row() {
    let grid = ColorSwatchGrid::new(ansi_colors(), theme());
    let ctx = LayoutCtx {
        measurer: &MockMeasurer::STANDARD,
        theme: theme(),
    };
    let lb = grid.layout(&ctx);
    let viewport = Rect::new(0.0, 0.0, 400.0, 300.0);
    let node = compute_layout(&lb, viewport);

    // 8 columns: 8 * 28 + 7 * 6 = 224 + 42 = 266.
    let expected_w = GRID_COLUMNS as f32 * CELL_SIZE + (GRID_COLUMNS as f32 - 1.0) * CELL_GAP;
    assert_eq!(node.rect.width(), expected_w);
    // 1 row.
    assert_eq!(node.rect.height(), ROW_HEIGHT);
}

#[test]
fn grid_layout_two_rows() {
    // 16 colors = 2 rows.
    let mut colors = ansi_colors();
    colors.extend(ansi_colors());
    let grid = ColorSwatchGrid::new(colors, theme());
    let ctx = LayoutCtx {
        measurer: &MockMeasurer::STANDARD,
        theme: theme(),
    };
    let lb = grid.layout(&ctx);
    let viewport = Rect::new(0.0, 0.0, 400.0, 300.0);
    let node = compute_layout(&lb, viewport);

    assert_eq!(node.rect.height(), ROW_HEIGHT * 2.0);
}

#[test]
fn grid_paint_produces_rects_and_labels() {
    let grid = ColorSwatchGrid::new(ansi_colors(), theme());
    let measurer = MockMeasurer::STANDARD;
    let mut draw_list = DrawList::new();
    let bounds = Rect::new(0.0, 0.0, 300.0, 100.0);
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
    grid.paint(&mut ctx);

    // 8 colored rects + 8 index labels.
    let rects = draw_list
        .commands()
        .iter()
        .filter(|c| matches!(c, DrawCommand::Rect { .. }))
        .count();
    assert_eq!(rects, 8, "one rect per color cell");

    let texts = draw_list
        .commands()
        .iter()
        .filter(|c| matches!(c, DrawCommand::Text { .. }))
        .count();
    assert_eq!(texts, 8, "one index label per cell");
}

#[test]
fn grid_click_emits_selected() {
    let mut grid = ColorSwatchGrid::new(ansi_colors(), theme());
    // Click on cell 3 (col=3, row=0).
    let cx = 3.0 * (CELL_SIZE + CELL_GAP) + CELL_SIZE / 2.0;
    let cy = CELL_SIZE / 2.0;
    let bounds = Rect::new(0.0, 0.0, 300.0, 100.0);
    let event = InputEvent::MouseDown {
        pos: Point::new(cx, cy),
        button: crate::input::MouseButton::Left,
        modifiers: crate::input::Modifiers::NONE,
    };
    let result = grid.on_input(&event, bounds);
    assert!(result.handled);
    match result.action {
        Some(WidgetAction::Selected { index, .. }) => assert_eq!(index, 3),
        other => panic!("expected Selected(3), got {other:?}"),
    }
}

#[test]
fn grid_sense_is_click() {
    let grid = ColorSwatchGrid::new(ansi_colors(), theme());
    assert_eq!(grid.sense(), Sense::click());
}

// -- SpecialColorSwatch --

#[test]
fn special_swatch_stores_label_and_color() {
    let swatch = SpecialColorSwatch::new("Foreground", Color::WHITE, theme());
    assert_eq!(swatch.label(), "Foreground");
    assert_eq!(swatch.color(), Color::WHITE);
}

#[test]
fn special_swatch_set_color() {
    let mut swatch = SpecialColorSwatch::new("BG", Color::BLACK, theme());
    swatch.set_color(Color::hex(0xFF_00_00));
    assert_eq!(swatch.color(), Color::hex(0xFF_00_00));
}

#[test]
fn special_swatch_paint_produces_swatch_and_labels() {
    let swatch = SpecialColorSwatch::new("Cursor", Color::hex(0x00_FF_00), theme());
    let measurer = MockMeasurer::STANDARD;
    let mut draw_list = DrawList::new();
    let bounds = Rect::new(0.0, 0.0, 80.0, 56.0);
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
    swatch.paint(&mut ctx);

    // 1 swatch rect.
    let rects = draw_list
        .commands()
        .iter()
        .filter(|c| matches!(c, DrawCommand::Rect { .. }))
        .count();
    assert_eq!(rects, 1, "one color swatch rect");

    // 2 text commands: label + hex value.
    let texts = draw_list
        .commands()
        .iter()
        .filter(|c| matches!(c, DrawCommand::Text { .. }))
        .count();
    assert_eq!(texts, 2, "label + hex value");
}

#[test]
fn special_swatch_sense_is_hover() {
    let swatch = SpecialColorSwatch::new("BG", Color::BLACK, theme());
    assert_eq!(swatch.sense(), Sense::hover());
}

#[test]
fn special_swatch_hex_format() {
    let swatch = SpecialColorSwatch::new("Test", Color::hex(0xFF_80_00), theme());
    assert_eq!(swatch.hex_string(), "#FF8000");
}
