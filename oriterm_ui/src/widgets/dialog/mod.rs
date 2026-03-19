//! Confirmation dialog widget with title, message, and OK/Cancel buttons.
//!
//! Two-zone layout inspired by Windows Terminal's `ContentDialog`: a content
//! area (title + message + optional preview) separated from a darker button
//! footer by a 1px line. Composes [`ButtonWidget`] instances for interactive
//! buttons and manages keyboard focus cycling between them.

mod rendering;
mod style;

use std::cell::RefCell;
use std::rc::Rc;

use crate::draw::RectStyle;
use crate::geometry::Rect;
use crate::input::{InputEvent, Key};
use crate::layout::{LayoutBox, LayoutNode};
use crate::sense::Sense;
use crate::widget_id::WidgetId;

use super::button::ButtonWidget;
use super::{DrawCtx, LayoutCtx, OnInputResult, Widget, WidgetAction};

pub use style::DialogStyle;

/// Dialog width in logical pixels.
const DIALOG_WIDTH: f32 = 400.0;

/// Maximum character length for preview text before truncation.
const PREVIEW_CHAR_LIMIT: usize = 512;

/// Which buttons to show (Chrome `kOk | kCancel` pattern).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DialogButtons {
    /// Single OK button only.
    OkOnly,
    /// OK and Cancel buttons.
    OkCancel,
}

/// Identifies which button is default (Enter activates it).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DialogButton {
    /// The OK/confirm button.
    Ok,
    /// The Cancel/dismiss button.
    Cancel,
}

/// Optional rich content block displayed between message and buttons.
///
/// Generic enough for any content — the paste dialog uses it for a
/// clipboard preview, but future dialogs could show other content.
pub struct DialogContent {
    /// Text to display (may be truncated).
    pub text: String,
    /// Whether to use monospace font (hint for future shaper support).
    pub monospace: bool,
}

/// Confirmation dialog widget.
///
/// Displays a title, message body, optional content preview, and one or
/// two buttons. The content area and button footer are visually separated
/// by a 1px line, with the footer having a slightly darker background.
pub struct DialogWidget {
    id: WidgetId,
    title: String,
    message: String,
    content: Option<DialogContent>,
    buttons: DialogButtons,
    ok_label: String,
    cancel_label: String,
    default_button: DialogButton,
    ok_button: ButtonWidget,
    cancel_button: ButtonWidget,
    style: DialogStyle,
    focused_button: DialogButton,
    /// Whether to show the focus ring on the focused button.
    ///
    /// Starts `false` — the focus ring only appears after the user presses
    /// Tab (`:focus-visible` behavior). This avoids a subtle focus-ring
    /// artifact on the default button when the dialog first opens.
    focus_visible: bool,
    /// Cached layout result, keyed by bounds.
    cached_layout: RefCell<Option<(Rect, Rc<LayoutNode>)>>,
}

impl DialogWidget {
    /// Creates a dialog with the given title.
    ///
    /// Defaults: `OkCancel` buttons, "OK"/"Cancel" labels, `Ok` as default.
    pub fn new(title: impl Into<String>) -> Self {
        let style = DialogStyle::default();
        let ok_button = ButtonWidget::new("OK").with_style(style.primary_button);
        let cancel_button = ButtonWidget::new("Cancel").with_style(style.secondary_button);

        Self {
            id: WidgetId::next(),
            title: title.into(),
            message: String::new(),
            content: None,
            buttons: DialogButtons::OkCancel,
            ok_label: "OK".into(),
            cancel_label: "Cancel".into(),
            default_button: DialogButton::Ok,
            ok_button,
            cancel_button,
            style,
            focused_button: DialogButton::Ok,
            focus_visible: false,
            cached_layout: RefCell::new(None),
        }
    }

    /// Sets the message body text.
    #[must_use]
    pub fn with_message(mut self, message: impl Into<String>) -> Self {
        self.message = message.into();
        self
    }

    /// Sets optional content displayed between message and buttons.
    ///
    /// Text is truncated at 512 characters with an ellipsis, matching the
    /// Windows Terminal `ContentDialog` pattern for clipboard previews.
    #[must_use]
    pub fn with_content(mut self, text: impl Into<String>) -> Self {
        let mut t = text.into();
        if t.len() > PREVIEW_CHAR_LIMIT {
            let end = t.floor_char_boundary(PREVIEW_CHAR_LIMIT);
            t.truncate(end);
            t.push('\u{2026}');
        }
        self.content = Some(DialogContent {
            text: t,
            monospace: true,
        });
        self
    }

    /// Sets which buttons to show.
    #[must_use]
    pub fn with_buttons(mut self, buttons: DialogButtons) -> Self {
        self.buttons = buttons;
        self
    }

    /// Sets the OK button label.
    #[must_use]
    pub fn with_ok_label(mut self, label: impl Into<String>) -> Self {
        self.ok_label = label.into();
        self.ok_button = ButtonWidget::new(&self.ok_label).with_style(self.style.primary_button);
        self
    }

    /// Sets the Cancel button label.
    #[must_use]
    pub fn with_cancel_label(mut self, label: impl Into<String>) -> Self {
        self.cancel_label = label.into();
        self.cancel_button =
            ButtonWidget::new(&self.cancel_label).with_style(self.style.secondary_button);
        self
    }

    /// Sets the default button (activated by Enter).
    #[must_use]
    pub fn with_default_button(mut self, button: DialogButton) -> Self {
        self.default_button = button;
        self.focused_button = button;
        self.rebuild_button_styles();
        self
    }

    /// Sets the dialog visual style.
    #[must_use]
    pub fn with_style(mut self, style: DialogStyle) -> Self {
        self.style = style;
        self.rebuild_button_styles();
        self
    }

    /// Returns the OK button's widget ID.
    pub fn ok_button_id(&self) -> WidgetId {
        self.ok_button.id()
    }

    /// Returns the Cancel button's widget ID.
    pub fn cancel_button_id(&self) -> WidgetId {
        self.cancel_button.id()
    }

    /// Rebuild button styles after `default_button` or style changes.
    fn rebuild_button_styles(&mut self) {
        let (ok_style, cancel_style) = match self.default_button {
            DialogButton::Ok => (self.style.primary_button, self.style.secondary_button),
            DialogButton::Cancel => (self.style.secondary_button, self.style.primary_button),
        };
        self.ok_button = ButtonWidget::new(&self.ok_label).with_style(ok_style);
        self.cancel_button = ButtonWidget::new(&self.cancel_label).with_style(cancel_style);
    }

    /// Determine which button corresponds to a given widget ID.
    fn button_for_id(&self, id: WidgetId) -> Option<DialogButton> {
        if id == self.ok_button.id() {
            Some(DialogButton::Ok)
        } else if id == self.cancel_button.id() {
            Some(DialogButton::Cancel)
        } else {
            None
        }
    }

    /// Get an immutable reference to the button at the given layout index.
    fn button_at_index_ref(&self, index: usize) -> (&ButtonWidget, DialogButton) {
        match self.buttons {
            DialogButtons::OkCancel if index == 0 => (&self.cancel_button, DialogButton::Cancel),
            DialogButtons::OkOnly | DialogButtons::OkCancel => (&self.ok_button, DialogButton::Ok),
        }
    }
}

impl Widget for DialogWidget {
    fn id(&self) -> WidgetId {
        self.id
    }

    fn is_focusable(&self) -> bool {
        false
    }

    fn layout(&self, ctx: &LayoutCtx<'_>) -> LayoutBox {
        self.build_layout(ctx)
    }

    fn sense(&self) -> Sense {
        Sense::none()
    }

    fn paint(&self, ctx: &mut DrawCtx<'_>) {
        *self.cached_layout.borrow_mut() = None;

        // Base layer: dialog bg in footer_bg color with rounded corners.
        // The footer inherits this as its background; the content zone is
        // overlaid with the lighter bg color. This avoids per-corner radius
        // issues (the GPU shader only supports uniform radius).
        ctx.scene.push_layer_bg(self.style.footer_bg);

        let mut base_style =
            RectStyle::filled(self.style.footer_bg).with_radius(self.style.corner_radius);
        if self.style.border_width > 0.0 {
            base_style = base_style.with_border(self.style.border_width, self.style.border_color);
        }
        if let Some(shadow) = self.style.shadow {
            base_style = base_style.with_shadow(shadow);
        }
        ctx.scene.push_quad(ctx.bounds, base_style);

        let layout = self.get_or_compute_layout(ctx.measurer, ctx.theme, ctx.bounds);
        let children = &layout.children;
        if children.len() < 2 {
            ctx.scene.pop_layer_bg();
            return;
        }

        // Content zone overlay: lighter bg covers everything above the
        // footer separator. Inset by border width so the base rect's
        // border remains visible around the edges.
        let bw = self.style.border_width;
        let content_rect = children[0]
            .rect
            .inset(crate::geometry::Insets::tlbr(bw, bw, 0.0, bw));
        ctx.scene.push_layer_bg(self.style.bg);
        // Radius inset by border width so the content overlay sits inside
        // the base rect's rounded corners. Bottom corners also get this
        // radius, but the gap reveals footer_bg which is seamless.
        let inner_radius = (self.style.corner_radius - bw).max(0.0);
        ctx.scene.push_quad(
            content_rect,
            RectStyle::filled(self.style.bg).with_radius(inner_radius),
        );
        self.draw_content(ctx, &children[0]);
        ctx.scene.pop_layer_bg();

        self.draw_footer(ctx, &children[1]);

        ctx.scene.pop_layer_bg();
    }

    fn for_each_child_mut(&mut self, visitor: &mut dyn FnMut(&mut dyn Widget)) {
        visitor(&mut self.ok_button);
        visitor(&mut self.cancel_button);
    }

    fn on_input(&mut self, event: &InputEvent, _bounds: Rect) -> OnInputResult {
        if let InputEvent::KeyDown { key, .. } = event {
            match key {
                Key::Enter | Key::Space => {
                    // Activate the focused button — handled via on_action.
                    return OnInputResult::handled();
                }
                Key::Escape => return OnInputResult::handled(),
                Key::Tab => {
                    if self.buttons == DialogButtons::OkCancel {
                        self.focus_visible = true;
                        self.focused_button = match self.focused_button {
                            DialogButton::Ok => DialogButton::Cancel,
                            DialogButton::Cancel => DialogButton::Ok,
                        };
                        return OnInputResult::handled();
                    }
                }
                _ => {}
            }
        }
        OnInputResult::ignored()
    }

    fn on_action(&mut self, action: WidgetAction, _bounds: Rect) -> Option<WidgetAction> {
        match action {
            WidgetAction::Clicked(id) => {
                // Map button clicks to dialog-level actions.
                match self.button_for_id(id) {
                    Some(DialogButton::Cancel) => Some(WidgetAction::DismissOverlay(self.id)),
                    Some(DialogButton::Ok) | None => Some(WidgetAction::Clicked(id)),
                }
            }
            other => Some(other),
        }
    }

    fn focusable_children(&self) -> Vec<WidgetId> {
        match self.buttons {
            DialogButtons::OkOnly => vec![self.ok_button.id()],
            DialogButtons::OkCancel => {
                vec![self.cancel_button.id(), self.ok_button.id()]
            }
        }
    }
}

#[cfg(test)]
mod tests;
