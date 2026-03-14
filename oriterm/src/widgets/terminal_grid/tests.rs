//! Unit tests for the terminal grid widget.

use std::time::Instant;

use oriterm_ui::draw::DrawList;
use oriterm_ui::geometry::Rect;
use oriterm_ui::input::{HoverEvent, KeyEvent, Modifiers};
use oriterm_ui::layout::SizeSpec;
use oriterm_ui::text::{ShapedText, TextMetrics, TextStyle};
use oriterm_ui::theme::UiTheme;
use oriterm_ui::widgets::text_measurer::TextMeasurer;
use oriterm_ui::widgets::{DrawCtx, EventCtx, LayoutCtx, Widget};

use super::TerminalGridWidget;

/// Minimal text measurer for tests (8px per char, 16px line height).
struct TestMeasurer;

impl TextMeasurer for TestMeasurer {
    fn measure(&self, text: &str, _style: &TextStyle, _max_width: f32) -> TextMetrics {
        TextMetrics {
            width: text.len() as f32 * 8.0,
            height: 16.0,
            line_count: 1,
        }
    }

    fn shape(&self, _text: &str, _style: &TextStyle, _max_width: f32) -> ShapedText {
        ShapedText {
            glyphs: Vec::new(),
            width: 0.0,
            height: 16.0,
            baseline: 12.0,
        }
    }
}

fn make_widget() -> TerminalGridWidget {
    TerminalGridWidget::new(8.0, 16.0, 80, 24)
}

fn make_layout_ctx() -> LayoutCtx<'static> {
    static MEASURER: TestMeasurer = TestMeasurer;
    static THEME: std::sync::LazyLock<UiTheme> = std::sync::LazyLock::new(UiTheme::dark);
    LayoutCtx {
        measurer: &MEASURER,
        theme: &THEME,
    }
}

// ── Layout ──

#[test]
fn layout_returns_fill_fill() {
    let widget = make_widget();
    let ctx = make_layout_ctx();
    let layout = widget.layout(&ctx);

    assert_eq!(layout.width, SizeSpec::Fill);
    assert_eq!(layout.height, SizeSpec::Fill);
}

#[test]
fn layout_has_widget_id() {
    let widget = make_widget();
    let ctx = make_layout_ctx();
    let layout = widget.layout(&ctx);

    assert_eq!(layout.widget_id, Some(widget.id()));
}

#[test]
fn layout_intrinsic_size_matches_grid() {
    let widget = TerminalGridWidget::new(8.0, 16.0, 80, 24);
    let ctx = make_layout_ctx();
    let layout = widget.layout(&ctx);

    if let oriterm_ui::layout::BoxContent::Leaf {
        intrinsic_width,
        intrinsic_height,
    } = layout.content
    {
        assert_eq!(intrinsic_width, 80.0 * 8.0);
        assert_eq!(intrinsic_height, 24.0 * 16.0);
    } else {
        panic!("expected Leaf content");
    }
}

// ── Focusable ──

#[test]
fn is_focusable() {
    let widget = make_widget();
    assert!(widget.is_focusable());
}

// ── Bounds via set_bounds ──

#[test]
fn bounds_none_before_set() {
    let widget = make_widget();
    assert!(widget.bounds().is_none());
}

#[test]
fn bounds_some_after_set() {
    let widget = make_widget();
    let bounds = Rect::new(10.0, 20.0, 640.0, 384.0);
    widget.set_bounds(bounds);

    let stored = widget
        .bounds()
        .expect("bounds should be set after set_bounds");
    assert_eq!(stored.x(), 10.0);
    assert_eq!(stored.y(), 20.0);
    assert_eq!(stored.width(), 640.0);
    assert_eq!(stored.height(), 384.0);
}

#[test]
fn draw_emits_no_commands() {
    let widget = make_widget();
    let theme = UiTheme::dark();
    let measurer = TestMeasurer;
    let mut draw_list = DrawList::new();
    let animations_running = std::cell::Cell::new(false);

    let mut ctx = DrawCtx {
        measurer: &measurer,
        draw_list: &mut draw_list,
        bounds: Rect::new(0.0, 0.0, 640.0, 384.0),
        focused_widget: None,
        now: Instant::now(),
        animations_running: &animations_running,
        theme: &theme,
        icons: None,
    };

    widget.draw(&mut ctx);

    assert!(
        draw_list.is_empty(),
        "grid widget should not emit draw commands"
    );
}

// ── Grid size updates ──

#[test]
fn set_grid_size_updates_dimensions() {
    let mut widget = make_widget();
    assert_eq!(widget.cols(), 80);
    assert_eq!(widget.rows(), 24);

    widget.set_grid_size(120, 40);
    assert_eq!(widget.cols(), 120);
    assert_eq!(widget.rows(), 40);
}

#[test]
fn set_cell_metrics_updates_dimensions() {
    let mut widget = make_widget();
    assert_eq!(widget.cell_width(), 8.0);
    assert_eq!(widget.cell_height(), 16.0);

    widget.set_cell_metrics(9.0, 18.0);
    assert_eq!(widget.cell_width(), 9.0);
    assert_eq!(widget.cell_height(), 18.0);
}

// ── Event handling ──

#[test]
fn handle_key_returns_handled() {
    let mut widget = make_widget();
    let theme = UiTheme::dark();
    let measurer = TestMeasurer;
    let ctx = EventCtx {
        measurer: &measurer,
        bounds: Rect::new(0.0, 0.0, 640.0, 384.0),
        is_focused: true,
        focused_widget: Some(widget.id()),
        theme: &theme,
    };

    let event = KeyEvent {
        key: oriterm_ui::input::Key::Character('a'),
        modifiers: Modifiers::NONE,
    };
    let response = widget.handle_key(event, &ctx);
    assert!(response.response.is_handled());
}

#[test]
fn handle_hover_returns_ignored() {
    let mut widget = make_widget();
    let theme = UiTheme::dark();
    let measurer = TestMeasurer;
    let ctx = EventCtx {
        measurer: &measurer,
        bounds: Rect::new(0.0, 0.0, 640.0, 384.0),
        is_focused: true,
        focused_widget: Some(widget.id()),
        theme: &theme,
    };

    let response = widget.handle_hover(HoverEvent::Enter, &ctx);
    assert!(!response.response.is_handled());
}
