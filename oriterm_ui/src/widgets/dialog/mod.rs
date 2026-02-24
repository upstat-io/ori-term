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
use crate::input::{HoverEvent, Key, KeyEvent, MouseEvent};
use crate::layout::{Direction, Justify, LayoutBox, LayoutNode, SizeSpec};
use crate::text::TextStyle;
use crate::widget_id::WidgetId;

use super::button::ButtonWidget;
use super::{DrawCtx, EventCtx, LayoutCtx, Widget, WidgetAction, WidgetResponse};

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
        let ok_button = ButtonWidget::new("OK").with_style(style.primary_button.clone());
        let cancel_button = ButtonWidget::new("Cancel").with_style(style.secondary_button.clone());

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
        self.ok_button =
            ButtonWidget::new(&self.ok_label).with_style(self.style.primary_button.clone());
        self
    }

    /// Sets the Cancel button label.
    #[must_use]
    pub fn with_cancel_label(mut self, label: impl Into<String>) -> Self {
        self.cancel_label = label.into();
        self.cancel_button =
            ButtonWidget::new(&self.cancel_label).with_style(self.style.secondary_button.clone());
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
            DialogButton::Ok => (
                self.style.primary_button.clone(),
                self.style.secondary_button.clone(),
            ),
            DialogButton::Cancel => (
                self.style.secondary_button.clone(),
                self.style.primary_button.clone(),
            ),
        };
        self.ok_button = ButtonWidget::new(&self.ok_label).with_style(ok_style);
        self.cancel_button = ButtonWidget::new(&self.cancel_label).with_style(cancel_style);
    }

    /// Build the title text style.
    fn title_style(&self) -> TextStyle {
        TextStyle::new(self.style.title_font_size, self.style.title_fg)
            .with_weight(crate::text::FontWeight::Bold)
    }

    /// Build the message text style.
    fn message_style(&self) -> TextStyle {
        TextStyle::new(self.style.message_font_size, self.style.message_fg)
    }

    /// Build the preview content text style.
    fn preview_text_style(&self) -> TextStyle {
        TextStyle::new(self.style.message_font_size, self.style.message_fg)
    }

    /// Build the two-zone layout: content zone + footer zone.
    fn build_layout(&self, ctx: &LayoutCtx<'_>) -> LayoutBox {
        let content_inner_w = DIALOG_WIDTH - self.style.padding.width();

        // Title leaf.
        let title_m = ctx
            .measurer
            .measure(&self.title, &self.title_style(), content_inner_w);
        let title_leaf = LayoutBox::leaf(content_inner_w, title_m.height);

        // Message leaf.
        let msg_m = ctx
            .measurer
            .measure(&self.message, &self.message_style(), content_inner_w);
        let msg_leaf = LayoutBox::leaf(content_inner_w, msg_m.height);

        // Content zone children: title, message, optional preview.
        let mut content_children = vec![title_leaf, msg_leaf];

        if let Some(ref content) = self.content {
            let preview_inner_w = content_inner_w - self.style.preview_padding.width();
            // Measure a single line to get line height, then multiply by line count.
            let line_m = ctx
                .measurer
                .measure("X", &self.preview_text_style(), preview_inner_w);
            let line_h = line_m.height;
            let line_count = content.text.lines().count().max(1);
            let preview_h = (line_count as f32 * line_h + self.style.preview_padding.height())
                .min(self.style.preview_max_height);
            content_children.push(LayoutBox::leaf(content_inner_w, preview_h));
        }

        let content_zone = LayoutBox::flex(Direction::Column, content_children)
            .with_padding(self.style.padding)
            .with_gap(self.style.content_spacing)
            .with_width(SizeSpec::Fill);

        // Footer zone: buttons right-aligned.
        let ok_box = self.ok_button.layout(ctx);
        let footer_zone = match self.buttons {
            DialogButtons::OkOnly => LayoutBox::flex(Direction::Row, vec![ok_box])
                .with_justify(Justify::End)
                .with_padding(self.style.footer_padding)
                .with_width(SizeSpec::Fill),
            DialogButtons::OkCancel => {
                let cancel_box = self.cancel_button.layout(ctx);
                LayoutBox::flex(Direction::Row, vec![cancel_box, ok_box])
                    .with_justify(Justify::End)
                    .with_gap(self.style.button_gap)
                    .with_padding(self.style.footer_padding)
                    .with_width(SizeSpec::Fill)
            }
        };

        // Dialog root: content zone + footer zone, no gap.
        LayoutBox::flex(Direction::Column, vec![content_zone, footer_zone])
            .with_width(SizeSpec::Fixed(DIALOG_WIDTH))
            .with_widget_id(self.id)
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

    /// Get a mutable reference to the button at the given layout index.
    ///
    /// In `OkCancel` mode: index 0 = cancel, index 1 = ok (layout order).
    /// In `OkOnly` mode: index 0 = ok.
    fn button_at_index(&mut self, index: usize) -> (&mut ButtonWidget, DialogButton) {
        match self.buttons {
            DialogButtons::OkCancel if index == 0 => {
                (&mut self.cancel_button, DialogButton::Cancel)
            }
            DialogButtons::OkOnly | DialogButtons::OkCancel => {
                (&mut self.ok_button, DialogButton::Ok)
            }
        }
    }

    /// Get an immutable reference to the button at the given layout index.
    fn button_at_index_ref(&self, index: usize) -> (&ButtonWidget, DialogButton) {
        match self.buttons {
            DialogButtons::OkCancel if index == 0 => (&self.cancel_button, DialogButton::Cancel),
            DialogButtons::OkOnly | DialogButtons::OkCancel => (&self.ok_button, DialogButton::Ok),
        }
    }

    /// Map a button click to the appropriate dialog-level response.
    fn map_button_click(&self, id: WidgetId) -> WidgetResponse {
        match self.button_for_id(id) {
            Some(DialogButton::Ok) => {
                WidgetResponse::redraw().with_action(WidgetAction::Clicked(id))
            }
            Some(DialogButton::Cancel) => {
                WidgetResponse::redraw().with_action(WidgetAction::DismissOverlay(self.id))
            }
            None => WidgetResponse::handled(),
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

    fn draw(&self, ctx: &mut DrawCtx<'_>) {
        *self.cached_layout.borrow_mut() = None;

        // Base layer: dialog bg in footer_bg color with rounded corners.
        // The footer inherits this as its background; the content zone is
        // overlaid with the lighter bg color. This avoids per-corner radius
        // issues (the GPU shader only supports uniform radius).
        ctx.draw_list.push_layer(self.style.footer_bg);

        let mut base_style =
            RectStyle::filled(self.style.footer_bg).with_radius(self.style.corner_radius);
        if self.style.border_width > 0.0 {
            base_style = base_style.with_border(self.style.border_width, self.style.border_color);
        }
        if let Some(shadow) = self.style.shadow {
            base_style = base_style.with_shadow(shadow);
        }
        ctx.draw_list.push_rect(ctx.bounds, base_style);

        let layout = self.get_or_compute_layout(ctx.measurer, ctx.theme, ctx.bounds);
        let children = &layout.children;
        if children.len() < 2 {
            ctx.draw_list.pop_layer();
            return;
        }

        // Content zone overlay: lighter bg covers everything above the
        // footer separator. Inset by border width so the base rect's
        // border remains visible around the edges.
        let bw = self.style.border_width;
        let content_rect = children[0]
            .rect
            .inset(crate::geometry::Insets::tlbr(bw, bw, 0.0, bw));
        ctx.draw_list.push_layer(self.style.bg);
        // Radius inset by border width so the content overlay sits inside
        // the base rect's rounded corners. Bottom corners also get this
        // radius, but the gap reveals footer_bg which is seamless.
        let inner_radius = (self.style.corner_radius - bw).max(0.0);
        ctx.draw_list.push_rect(
            content_rect,
            RectStyle::filled(self.style.bg).with_radius(inner_radius),
        );
        self.draw_content(ctx, &children[0]);
        ctx.draw_list.pop_layer();

        self.draw_footer(ctx, &children[1]);

        ctx.draw_list.pop_layer();
    }

    fn handle_mouse(&mut self, event: &MouseEvent, ctx: &EventCtx<'_>) -> WidgetResponse {
        let layout = self.get_or_compute_layout(ctx.measurer, ctx.theme, ctx.bounds);
        let children = &layout.children;
        if children.len() < 2 {
            return WidgetResponse::ignored();
        }

        // Footer zone is children[1]; buttons are its children.
        let focused = self.focused_button;
        for (i, btn_node) in children[1].children.iter().enumerate() {
            if !btn_node.rect.contains(event.pos) {
                continue;
            }
            let (button, btn_kind) = self.button_at_index(i);
            let btn_ctx = EventCtx {
                measurer: ctx.measurer,
                bounds: btn_node.content_rect,
                is_focused: focused == btn_kind,
                focused_widget: ctx.focused_widget,
                theme: ctx.theme,
            };
            let response = button.handle_mouse(event, &btn_ctx);
            if let Some(WidgetAction::Clicked(id)) = &response.action {
                return self.map_button_click(*id);
            }
            return response;
        }
        WidgetResponse::handled()
    }

    fn handle_hover(&mut self, event: HoverEvent, ctx: &EventCtx<'_>) -> WidgetResponse {
        let layout = self.get_or_compute_layout(ctx.measurer, ctx.theme, ctx.bounds);
        let children = &layout.children;
        if children.len() < 2 {
            return WidgetResponse::ignored();
        }

        // Footer zone is children[1]; buttons are its children.
        let focused = self.focused_button;
        for (i, btn_node) in children[1].children.iter().enumerate() {
            let (button, btn_kind) = self.button_at_index(i);
            let btn_ctx = EventCtx {
                measurer: ctx.measurer,
                bounds: btn_node.content_rect,
                is_focused: focused == btn_kind,
                focused_widget: ctx.focused_widget,
                theme: ctx.theme,
            };
            button.handle_hover(event, &btn_ctx);
        }
        WidgetResponse::handled()
    }

    fn handle_key(&mut self, event: KeyEvent, _ctx: &EventCtx<'_>) -> WidgetResponse {
        match event.key {
            Key::Enter | Key::Space => match self.focused_button {
                DialogButton::Ok => {
                    WidgetResponse::redraw().with_action(WidgetAction::Clicked(self.ok_button.id()))
                }
                DialogButton::Cancel => {
                    WidgetResponse::redraw().with_action(WidgetAction::DismissOverlay(self.id))
                }
            },
            Key::Escape => {
                WidgetResponse::redraw().with_action(WidgetAction::DismissOverlay(self.id))
            }
            Key::Tab => {
                if self.buttons == DialogButtons::OkCancel {
                    self.focus_visible = true;
                    self.focused_button = match self.focused_button {
                        DialogButton::Ok => DialogButton::Cancel,
                        DialogButton::Cancel => DialogButton::Ok,
                    };
                    WidgetResponse::redraw()
                } else {
                    WidgetResponse::handled()
                }
            }
            _ => WidgetResponse::handled(),
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
