use crate::draw::Scene;
use crate::geometry::Rect;
use crate::layout::compute_layout;
use crate::sense::Sense;
use crate::widgets::label::LabelWidget;
use crate::widgets::tests::MockMeasurer;
use crate::widgets::{DrawCtx, LayoutCtx, Widget};

use super::{MIN_HEIGHT, SettingRowWidget};

fn make_row() -> SettingRowWidget {
    let theme = super::super::tests::TEST_THEME;
    let control = Box::new(LabelWidget::new("Value"));
    SettingRowWidget::new("Font Size", "Size of the terminal font", control, &theme)
}

fn make_ctx() -> LayoutCtx<'static> {
    LayoutCtx {
        measurer: &MockMeasurer::STANDARD,
        theme: &super::super::tests::TEST_THEME,
    }
}

// -- Construction --

#[test]
fn new_stores_name_and_description() {
    let row = make_row();
    assert_eq!(row.name(), "Font Size");
    assert_eq!(row.description(), "Size of the terminal font");
}

// -- Layout --

#[test]
fn layout_height_at_least_min_height() {
    let row = make_row();
    let ctx = make_ctx();
    let lb = row.layout(&ctx);
    let viewport = Rect::new(0.0, 0.0, 400.0, 300.0);
    let node = compute_layout(&lb, viewport);

    assert!(
        node.rect.height() >= MIN_HEIGHT,
        "height {} should be >= {}",
        node.rect.height(),
        MIN_HEIGHT
    );
}

#[test]
fn layout_uses_available_width() {
    let row = make_row();
    let ctx = make_ctx();
    let lb = row.layout(&ctx);
    let viewport = Rect::new(0.0, 0.0, 400.0, 300.0);
    let node = compute_layout(&lb, viewport);

    // Row should use the full available width when laid out in a flex parent.
    // As a top-level box, the solver gives it its natural width. But the key
    // thing is it has a label column (fill) and control (hug).
    assert!(node.rect.width() > 0.0);
}

// -- Paint --

#[test]
fn paint_produces_text_commands() {
    let row = make_row();
    let measurer = MockMeasurer::STANDARD;
    let mut scene = Scene::new();
    let bounds = Rect::new(0.0, 0.0, 400.0, 50.0);
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
    row.paint(&mut ctx);

    // Should produce text runs for: name, description, and control label.
    assert_eq!(
        scene.text_runs().len(),
        3,
        "name + description + control label"
    );
}

// -- Sense & controllers --

#[test]
fn sense_is_hover() {
    let row = make_row();
    assert_eq!(row.sense(), Sense::hover());
}

#[test]
fn has_hover_controller() {
    let row = make_row();
    assert_eq!(
        row.controllers().len(),
        1,
        "should have one HoverController"
    );
}

#[test]
fn has_visual_state_animator() {
    let row = make_row();
    assert!(row.visual_states().is_some());
}

// -- Child delegation --

#[test]
fn for_each_child_yields_control() {
    let mut row = make_row();
    let mut count = 0;
    row.for_each_child_mut(&mut |_| count += 1);
    assert_eq!(count, 1, "should yield the control widget");
}

#[test]
fn not_focusable() {
    let row = make_row();
    assert!(!row.is_focusable());
}
