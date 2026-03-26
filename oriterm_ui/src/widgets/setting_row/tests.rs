use crate::draw::Scene;
use crate::geometry::Rect;
use crate::layout::compute_layout;
use crate::sense::Sense;
use crate::theme::UiTheme;
use crate::widgets::label::LabelWidget;
use crate::widgets::tests::MockMeasurer;
use crate::widgets::{DrawCtx, LayoutCtx, Widget};

use super::{MIN_HEIGHT, SettingRowWidget, SettingTag, SettingTagKind};

fn make_row() -> SettingRowWidget {
    let theme = super::super::tests::TEST_THEME;
    let control = Box::new(LabelWidget::new("Value"));
    SettingRowWidget::new("Font Size", "Size of the terminal font", control, &theme)
}

fn make_row_with_tag(kind: SettingTagKind, text: &str) -> SettingRowWidget {
    let theme = super::super::tests::TEST_THEME;
    let control = Box::new(LabelWidget::new("Value"));
    SettingRowWidget::new("Font Size", "Size of the terminal font", control, &theme)
        .with_tag(SettingTag::new(kind, text))
}

fn make_ctx() -> LayoutCtx<'static> {
    LayoutCtx {
        measurer: &MockMeasurer::STANDARD,
        theme: &super::super::tests::TEST_THEME,
    }
}

fn paint_row(row: &SettingRowWidget) -> Scene {
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
    scene
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
    let scene = paint_row(&row);

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

// -- Tag storage --

#[test]
fn setting_row_stores_tags() {
    let row = make_row_with_tag(SettingTagKind::Restart, "Restart");
    assert_eq!(row.tags().len(), 1);
    assert_eq!(row.tags()[0].kind, SettingTagKind::Restart);
    assert_eq!(row.tags()[0].text, "Restart");
}

// -- Tag backward compatibility --

#[test]
fn setting_row_zero_tags_identical_output() {
    let row = make_row();
    let scene = paint_row(&row);

    // No tags: exactly 3 text runs (name + desc + control label).
    assert_eq!(
        scene.text_runs().len(),
        3,
        "zero-tag row should produce exactly 3 text runs"
    );
}

// -- Tag layout --

#[test]
fn setting_row_layout_with_tags() {
    let ctx = make_ctx();
    let plain = make_row();
    let tagged = make_row_with_tag(SettingTagKind::Restart, "Restart");

    let viewport = Rect::new(0.0, 0.0, 400.0, 300.0);
    let plain_lb = plain.layout(&ctx);
    let tagged_lb = tagged.layout(&ctx);
    let plain_node = compute_layout(&plain_lb, viewport);
    let tagged_node = compute_layout(&tagged_lb, viewport);

    // The tagged row's label column should be wider to accommodate the tag chip.
    let plain_label = &plain_node.children[0];
    let tagged_label = &tagged_node.children[0];
    assert!(
        tagged_label.rect.width() >= plain_label.rect.width(),
        "tagged label width {} should be >= plain {}",
        tagged_label.rect.width(),
        plain_label.rect.width()
    );
}

// -- Tag paint --

#[test]
fn setting_row_paint_tag_text_run() {
    let row = make_row_with_tag(SettingTagKind::Restart, "Restart");
    let scene = paint_row(&row);

    // Should have 4 text runs: name, tag text, description, control label.
    assert_eq!(
        scene.text_runs().len(),
        4,
        "tagged row should produce 4 text runs"
    );

    // Find the tag text run by weight (700 = Bold).
    let tag_run = scene
        .text_runs()
        .iter()
        .find(|r| r.shaped.weight == 700)
        .expect("should have a bold text run for the tag");

    // MockMeasurer applies TextTransform::Uppercase, producing 7 glyphs for "RESTART".
    assert_eq!(
        tag_run.shaped.glyphs.len(),
        7,
        "tag text 'RESTART' should have 7 glyphs"
    );
}

#[test]
fn setting_row_paint_tag_quad() {
    let row = make_row_with_tag(SettingTagKind::Restart, "Restart");
    let scene = paint_row(&row);
    let theme = UiTheme::dark();

    // Should have at least one quad for the tag chip background.
    let tag_quad = scene
        .quads()
        .iter()
        .find(|q| q.style.fill == Some(theme.warning_bg))
        .expect("should have a quad with warning_bg fill");

    // Border color should match text color (warning).
    let border = tag_quad
        .style
        .border
        .top
        .as_ref()
        .expect("tag quad should have a top border");
    assert_eq!(
        border.color, theme.warning,
        "border color should be warning"
    );
    assert!(
        (border.width - 1.0).abs() < f32::EPSILON,
        "border width should be 1px"
    );
}

// -- Multiple tags --

#[test]
fn setting_row_multiple_tags() {
    let theme = super::super::tests::TEST_THEME;
    let control = Box::new(LabelWidget::new("Value"));
    let row = SettingRowWidget::new("Feature", "A feature", control, &theme)
        .with_tag(SettingTag::new(SettingTagKind::New, "New"))
        .with_tag(SettingTag::new(
            SettingTagKind::Experimental,
            "Experimental",
        ));

    let scene = paint_row(&row);

    // Should have quads for both tag variants.
    let has_accent = scene
        .quads()
        .iter()
        .any(|q| q.style.fill == Some(theme.accent_bg_strong));
    let has_danger = scene
        .quads()
        .iter()
        .any(|q| q.style.fill == Some(theme.danger_bg));
    assert!(has_accent, "should have accent_bg_strong quad for New tag");
    assert!(
        has_danger,
        "should have danger_bg quad for Experimental tag"
    );
}

// -- Tag kind colors --

#[test]
fn setting_row_tag_kind_colors() {
    let theme = UiTheme::dark();

    let (text, bg) = SettingTagKind::New.colors(&theme);
    assert_eq!(text, theme.accent);
    assert_eq!(bg, theme.accent_bg_strong);

    let (text, bg) = SettingTagKind::Restart.colors(&theme);
    assert_eq!(text, theme.warning);
    assert_eq!(bg, theme.warning_bg);

    let (text, bg) = SettingTagKind::Advanced.colors(&theme);
    assert_eq!(text, theme.fg_secondary);
    assert_eq!(bg, theme.bg_secondary);

    let (text, bg) = SettingTagKind::Experimental.colors(&theme);
    assert_eq!(text, theme.danger);
    assert_eq!(bg, theme.danger_bg);
}

// -- Tag min height --

#[test]
fn setting_row_with_tags_min_height() {
    let row = make_row_with_tag(SettingTagKind::Restart, "Restart")
        .with_tag(SettingTag::new(SettingTagKind::Advanced, "Advanced"));

    let ctx = make_ctx();
    let lb = row.layout(&ctx);
    let viewport = Rect::new(0.0, 0.0, 400.0, 300.0);
    let node = compute_layout(&lb, viewport);

    assert!(
        node.rect.height() >= MIN_HEIGHT,
        "tagged row height {} should be >= {}",
        node.rect.height(),
        MIN_HEIGHT
    );
}
