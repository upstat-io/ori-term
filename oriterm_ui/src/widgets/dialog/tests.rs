//! Tests for the dialog widget.

use crate::geometry::Rect;
use crate::input::{InputEvent, Key, Modifiers};
use crate::theme::UiTheme;
use crate::widgets::{LayoutCtx, Widget, WidgetAction};

use super::{DialogButton, DialogButtons, DialogWidget, PREVIEW_CHAR_LIMIT};

/// Stub text measurer for layout tests (returns fixed-size metrics).
struct StubMeasurer;

impl crate::widgets::TextMeasurer for StubMeasurer {
    fn measure(
        &self,
        _text: &str,
        _style: &crate::text::TextStyle,
        _max_width: f32,
    ) -> crate::text::TextMetrics {
        crate::text::TextMetrics {
            width: 100.0,
            height: 16.0,
            line_count: 1,
        }
    }

    fn shape(
        &self,
        _text: &str,
        _style: &crate::text::TextStyle,
        _max_width: f32,
    ) -> crate::text::ShapedText {
        crate::text::ShapedText {
            glyphs: Vec::new(),
            width: 100.0,
            height: 16.0,
            baseline: 12.0,
            size_q6: 0,
            weight: 400,
            font_source: crate::text::FontSource::Ui,
        }
    }
}

fn key_down(key: Key) -> InputEvent {
    InputEvent::KeyDown {
        key,
        modifiers: Modifiers::NONE,
    }
}

#[test]
fn builder_sets_title_and_message() {
    let dialog = DialogWidget::new("Confirm")
        .with_message("Are you sure?")
        .with_ok_label("Yes")
        .with_cancel_label("No");

    assert_eq!(dialog.title, "Confirm");
    assert_eq!(dialog.message, "Are you sure?");
    assert_eq!(dialog.ok_label, "Yes");
    assert_eq!(dialog.cancel_label, "No");
}

#[test]
fn layout_produces_two_zone_children() {
    let dialog = DialogWidget::new("Test")
        .with_message("Body text")
        .with_buttons(DialogButtons::OkCancel);

    let measurer = StubMeasurer;
    let ctx = LayoutCtx {
        measurer: &measurer,
        theme: &UiTheme::dark(),
    };
    let layout_box = dialog.layout(&ctx);

    // Root: flex column with 2 children — content zone and footer zone.
    match &layout_box.content {
        crate::layout::BoxContent::Flex { children, .. } => {
            assert_eq!(children.len(), 2);

            // Content zone should be a flex column.
            match &children[0].content {
                crate::layout::BoxContent::Flex {
                    direction,
                    children: content_children,
                    ..
                } => {
                    assert_eq!(*direction, crate::layout::Direction::Column);
                    // Title + message = 2 children (no content preview).
                    assert_eq!(content_children.len(), 2);
                }
                _ => {
                    panic!("expected flex container for content zone, got non-flex");
                }
            }

            // Footer zone should be a flex row.
            match &children[1].content {
                crate::layout::BoxContent::Flex {
                    direction,
                    children: footer_children,
                    ..
                } => {
                    assert_eq!(*direction, crate::layout::Direction::Row);
                    // Cancel + OK = 2 children.
                    assert_eq!(footer_children.len(), 2);
                }
                _ => {
                    panic!("expected flex container for footer zone, got non-flex");
                }
            }
        }
        _ => {
            panic!("expected flex container, got non-flex");
        }
    }
}

#[test]
fn layout_with_content_adds_preview_child() {
    let dialog = DialogWidget::new("Paste")
        .with_message("Paste 5 lines?")
        .with_content("echo hello\necho world");

    let measurer = StubMeasurer;
    let ctx = LayoutCtx {
        measurer: &measurer,
        theme: &UiTheme::dark(),
    };
    let layout_box = dialog.layout(&ctx);

    match &layout_box.content {
        crate::layout::BoxContent::Flex { children, .. } => {
            assert_eq!(children.len(), 2);

            // Content zone should have 3 children: title, message, preview.
            match &children[0].content {
                crate::layout::BoxContent::Flex {
                    children: content_children,
                    ..
                } => {
                    assert_eq!(content_children.len(), 3);
                }
                _ => {
                    panic!("expected flex container for content zone");
                }
            }
        }
        _ => {
            panic!("expected flex container");
        }
    }
}

#[test]
fn with_content_truncates_long_text() {
    let long = "x".repeat(1000);
    let dialog = DialogWidget::new("Test").with_content(long);

    let content = dialog.content.as_ref().expect("content should be set");
    // Truncated at PREVIEW_CHAR_LIMIT + ellipsis char.
    assert!(content.text.len() <= PREVIEW_CHAR_LIMIT + 3); // +3 for U+2026 (3 bytes)
    assert!(content.text.ends_with('\u{2026}'));
    assert!(content.monospace);
}

#[test]
fn with_content_preserves_short_text() {
    let short = "echo hello";
    let dialog = DialogWidget::new("Test").with_content(short);

    let content = dialog.content.as_ref().expect("content should be set");
    assert_eq!(content.text, "echo hello");
}

// -- on_input: Tab focus cycling --

#[test]
fn tab_toggles_focused_button() {
    let mut dialog = DialogWidget::new("Test")
        .with_buttons(DialogButtons::OkCancel)
        .with_default_button(DialogButton::Ok);
    let bounds = Rect::new(0.0, 0.0, 400.0, 300.0);

    assert_eq!(dialog.focused_button, DialogButton::Ok);

    dialog.on_input(&key_down(Key::Tab), bounds);
    assert_eq!(dialog.focused_button, DialogButton::Cancel);

    dialog.on_input(&key_down(Key::Tab), bounds);
    assert_eq!(dialog.focused_button, DialogButton::Ok);
}

#[test]
fn tab_is_noop_for_ok_only() {
    let mut dialog = DialogWidget::new("Test")
        .with_buttons(DialogButtons::OkOnly)
        .with_default_button(DialogButton::Ok);
    let bounds = Rect::new(0.0, 0.0, 400.0, 300.0);

    assert_eq!(dialog.focused_button, DialogButton::Ok);

    let result = dialog.on_input(&key_down(Key::Tab), bounds);
    assert!(!result.handled);
    assert_eq!(dialog.focused_button, DialogButton::Ok);
}

#[test]
fn escape_handled_by_on_input() {
    let mut dialog = DialogWidget::new("Test");
    let bounds = Rect::new(0.0, 0.0, 400.0, 300.0);

    let result = dialog.on_input(&key_down(Key::Escape), bounds);
    assert!(result.handled);
}

// -- on_action: button click mapping --

#[test]
fn on_action_maps_ok_button_click() {
    let mut dialog = DialogWidget::new("Test").with_buttons(DialogButtons::OkCancel);
    let ok_id = dialog.ok_button_id();
    let bounds = Rect::new(0.0, 0.0, 400.0, 300.0);

    let result = dialog.on_action(WidgetAction::Clicked(ok_id), bounds);
    assert_eq!(result, Some(WidgetAction::Clicked(ok_id)));
}

#[test]
fn on_action_maps_cancel_button_click_to_dismiss() {
    let mut dialog = DialogWidget::new("Test").with_buttons(DialogButtons::OkCancel);
    let dialog_id = dialog.id();
    let cancel_id = dialog.cancel_button_id();
    let bounds = Rect::new(0.0, 0.0, 400.0, 300.0);

    let result = dialog.on_action(WidgetAction::Clicked(cancel_id), bounds);
    assert_eq!(result, Some(WidgetAction::DismissOverlay(dialog_id)));
}

#[test]
fn ok_only_has_single_focusable_child() {
    let dialog = DialogWidget::new("Test").with_buttons(DialogButtons::OkOnly);

    let ids = dialog.focusable_children();
    assert_eq!(ids.len(), 1);
    assert_eq!(ids[0], dialog.ok_button_id());
}

#[test]
fn ok_cancel_has_two_focusable_children() {
    let dialog = DialogWidget::new("Test").with_buttons(DialogButtons::OkCancel);

    let ids = dialog.focusable_children();
    assert_eq!(ids.len(), 2);
    assert_eq!(ids[0], dialog.cancel_button_id());
    assert_eq!(ids[1], dialog.ok_button_id());
}

#[test]
fn dialog_is_not_directly_focusable() {
    let dialog = DialogWidget::new("Test");
    assert!(!dialog.is_focusable());
}
