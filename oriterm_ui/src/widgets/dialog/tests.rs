//! Tests for the dialog widget.

use crate::input::{EventResponse, Key, KeyEvent, Modifiers};
use crate::theme::UiTheme;
use crate::widgets::{EventCtx, LayoutCtx, Widget, WidgetAction};

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
        }
    }
}

fn key_event(key: Key) -> KeyEvent {
    KeyEvent {
        key,
        modifiers: Modifiers::NONE,
    }
}

fn event_ctx<'a>(
    measurer: &'a dyn crate::widgets::TextMeasurer,
    theme: &'a UiTheme,
) -> EventCtx<'a> {
    EventCtx {
        measurer,
        bounds: crate::geometry::Rect::new(0.0, 0.0, 400.0, 300.0),
        is_focused: true,
        focused_widget: None,
        theme,
        interaction: None,
        widget_id: None,
        frame_requests: None,
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
                crate::layout::BoxContent::Leaf { .. } => {
                    panic!("expected flex container for content zone, got leaf");
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
                crate::layout::BoxContent::Leaf { .. } => {
                    panic!("expected flex container for footer zone, got leaf");
                }
            }
        }
        crate::layout::BoxContent::Leaf { .. } => {
            panic!("expected flex container, got leaf");
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
                crate::layout::BoxContent::Leaf { .. } => {
                    panic!("expected flex container for content zone");
                }
            }
        }
        crate::layout::BoxContent::Leaf { .. } => {
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

#[test]
fn enter_emits_clicked_for_ok_default() {
    let mut dialog = DialogWidget::new("Test").with_default_button(DialogButton::Ok);
    let ok_id = dialog.ok_button_id();

    let measurer = StubMeasurer;
    let theme = UiTheme::dark();
    let ctx = event_ctx(&measurer, &theme);
    let response = dialog.handle_key(key_event(Key::Enter), &ctx);

    assert_eq!(response.action, Some(WidgetAction::Clicked(ok_id)));
}

#[test]
fn enter_emits_dismiss_for_cancel_default() {
    let mut dialog = DialogWidget::new("Test")
        .with_buttons(DialogButtons::OkCancel)
        .with_default_button(DialogButton::Cancel);
    let dialog_id = dialog.id();

    let measurer = StubMeasurer;
    let theme = UiTheme::dark();
    let ctx = event_ctx(&measurer, &theme);
    let response = dialog.handle_key(key_event(Key::Enter), &ctx);

    assert_eq!(
        response.action,
        Some(WidgetAction::DismissOverlay(dialog_id))
    );
}

#[test]
fn escape_emits_dismiss() {
    let mut dialog = DialogWidget::new("Test");
    let dialog_id = dialog.id();

    let measurer = StubMeasurer;
    let theme = UiTheme::dark();
    let ctx = event_ctx(&measurer, &theme);
    let response = dialog.handle_key(key_event(Key::Escape), &ctx);

    assert_eq!(
        response.action,
        Some(WidgetAction::DismissOverlay(dialog_id))
    );
    assert_eq!(response.response, EventResponse::RequestLayout);
}

#[test]
fn tab_toggles_focused_button() {
    let mut dialog = DialogWidget::new("Test")
        .with_buttons(DialogButtons::OkCancel)
        .with_default_button(DialogButton::Ok);

    assert_eq!(dialog.focused_button, DialogButton::Ok);

    let measurer = StubMeasurer;
    let theme = UiTheme::dark();
    let ctx = event_ctx(&measurer, &theme);
    dialog.handle_key(key_event(Key::Tab), &ctx);
    assert_eq!(dialog.focused_button, DialogButton::Cancel);

    dialog.handle_key(key_event(Key::Tab), &ctx);
    assert_eq!(dialog.focused_button, DialogButton::Ok);
}

#[test]
fn tab_is_noop_for_ok_only() {
    let mut dialog = DialogWidget::new("Test")
        .with_buttons(DialogButtons::OkOnly)
        .with_default_button(DialogButton::Ok);

    assert_eq!(dialog.focused_button, DialogButton::Ok);

    let measurer = StubMeasurer;
    let theme = UiTheme::dark();
    let ctx = event_ctx(&measurer, &theme);
    dialog.handle_key(key_event(Key::Tab), &ctx);

    // Should remain on Ok — no toggle target.
    assert_eq!(dialog.focused_button, DialogButton::Ok);
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
fn space_activates_focused_button() {
    let mut dialog = DialogWidget::new("Test").with_default_button(DialogButton::Ok);
    let ok_id = dialog.ok_button_id();

    let measurer = StubMeasurer;
    let theme = UiTheme::dark();
    let ctx = event_ctx(&measurer, &theme);
    let response = dialog.handle_key(key_event(Key::Space), &ctx);

    assert_eq!(response.action, Some(WidgetAction::Clicked(ok_id)));
}

#[test]
fn dialog_is_not_directly_focusable() {
    let dialog = DialogWidget::new("Test");
    assert!(!dialog.is_focusable());
}
