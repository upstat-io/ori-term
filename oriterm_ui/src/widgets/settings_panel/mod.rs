//! Settings panel widget — modal overlay container for settings forms.
//!
//! Composes a header bar (title + close button), a separator, and a scrollable
//! `FormLayout` body. The close button emits `WidgetAction::CancelSettings`
//! (translated from the button's `Clicked` action).

mod id_override_button;

use std::cell::RefCell;
use std::rc::Rc;

use crate::color::Color;
use crate::draw::{RectStyle, Shadow};
use crate::geometry::Insets;
use crate::layout::{Align, Direction, LayoutBox, LayoutNode, SizeSpec, compute_layout};
use crate::sense::Sense;
use crate::widget_id::WidgetId;

use crate::theme::UiTheme;

use super::button::{ButtonStyle, ButtonWidget};
use super::container::ContainerWidget;
use super::label::{LabelStyle, LabelWidget};
use super::separator::SeparatorWidget;
use super::spacer::SpacerWidget;
use super::{DrawCtx, LayoutCtx, TextMeasurer, Widget, WidgetAction};

use id_override_button::IdOverrideButton;

/// Fixed width of the settings panel in logical pixels.
const PANEL_WIDTH: f32 = 860.0;

/// Height of the header bar in logical pixels.
const HEADER_HEIGHT: f32 = 48.0;

/// Font size for the header title.
const TITLE_FONT_SIZE: f32 = 16.0;

/// Padding inside the header bar.
const HEADER_PADDING: Insets = Insets::vh(0.0, 16.0);

/// Height of the footer bar in logical pixels.
const FOOTER_HEIGHT: f32 = 52.0;

/// Corner radius for the panel.
const CORNER_RADIUS: f32 = 8.0;

/// A modal settings panel with header bar, scrollable form body, and footer.
///
/// The header contains a "Settings" title label and a close button (×).
/// The body wraps a `FormLayout` in a vertical `ScrollWidget`. The footer
/// has Cancel and Save buttons. Close (×) and Cancel both emit
/// `CancelSettings`; Save emits `SaveSettings`.
///
/// When used inside a dialog window, call [`Self::embedded()`] to skip the
/// header and panel chrome (shadow, border, rounded corners) — the dialog
/// window already provides those.
pub struct SettingsPanel {
    id: WidgetId,
    close_id: WidgetId,
    save_id: WidgetId,
    cancel_id: WidgetId,
    reset_id: WidgetId,
    container: ContainerWidget,
    cached_layout: RefCell<Option<(crate::geometry::Rect, Rc<LayoutNode>)>>,
    /// Whether the panel draws its own chrome (header, shadow, border).
    /// `false` when embedded in a dialog window.
    show_chrome: bool,
}

impl SettingsPanel {
    /// Creates a settings panel wrapping the given content widget.
    ///
    /// The content widget is the pre-built body (e.g. sidebar + pages).
    /// Each page inside should handle its own scrolling. Includes a header
    /// bar (title + close button), panel chrome (shadow, border, rounded
    /// corners), and drag support. Use this when the panel is shown as an
    /// overlay inside a terminal window.
    pub fn new(content: Box<dyn Widget>) -> Self {
        Self::build(content, true)
    }

    /// Creates a settings panel without header or chrome.
    ///
    /// Omits the title bar, close button, shadow, border, and rounded
    /// corners — all of which are provided by the dialog window's own
    /// chrome. Use this when the panel is embedded in a dialog OS window.
    pub fn embedded(content: Box<dyn Widget>) -> Self {
        Self::build(content, false)
    }

    /// Internal builder shared by `new()` and `embedded()`.
    fn build(content: Box<dyn Widget>, show_chrome: bool) -> Self {
        let close_id = WidgetId::next();
        let save_id = WidgetId::next();
        let cancel_id = WidgetId::next();
        let reset_id = WidgetId::next();
        let panel_id = WidgetId::next();

        // Body: the content widget fills remaining height so the footer
        // stays pinned at the bottom (sticky footer pattern). Clip so
        // scroll content cannot overflow into the separator/footer area.
        let body = ContainerWidget::row()
            .with_width(SizeSpec::Fill)
            .with_height(SizeSpec::Fill)
            .with_clip(true)
            .with_child(content);

        // Footer: separator + Reset to Defaults (left) | Cancel + Save (right).
        let footer_sep = SeparatorWidget::horizontal();

        let ghost_style = ButtonStyle {
            padding: Insets::vh(6.0, 16.0),
            border_width: 1.0,
            ..ButtonStyle::default()
        };

        let reset_btn = ButtonWidget::new("Reset to Defaults").with_style(ghost_style);
        let reset_btn = IdOverrideButton::new(reset_btn, reset_id);

        let cancel_btn = ButtonWidget::new("Cancel").with_style(ghost_style);
        let cancel_btn = IdOverrideButton::new(cancel_btn, cancel_id);

        let save_btn = ButtonWidget::new("Save").with_style(ButtonStyle {
            padding: Insets::vh(6.0, 20.0),
            border_width: 0.0,
            bg: Color::rgb(0.25, 0.52, 0.96),
            fg: Color::WHITE,
            ..ButtonStyle::default()
        });
        let save_btn = IdOverrideButton::new(save_btn, save_id);

        let footer = ContainerWidget::row()
            .with_align(Align::Center)
            .with_width(SizeSpec::Fill)
            .with_height(SizeSpec::Fixed(FOOTER_HEIGHT))
            .with_child(Box::new(SpacerWidget::fixed(HEADER_PADDING.left, 0.0)))
            .with_child(Box::new(reset_btn))
            .with_child(Box::new(SpacerWidget::fill()))
            .with_child(Box::new(cancel_btn))
            .with_child(Box::new(SpacerWidget::fixed(8.0, 0.0)))
            .with_child(Box::new(save_btn))
            .with_child(Box::new(SpacerWidget::fixed(HEADER_PADDING.right, 0.0)));

        // Build the column layout. Overlay mode gets a header; embedded skips it.
        let width = if show_chrome {
            SizeSpec::Fixed(PANEL_WIDTH)
        } else {
            SizeSpec::Fill
        };

        // The container fills the available height so the footer stays
        // pinned at the bottom. The content widget inside uses Fill to take
        // remaining space after the fixed-height footer.
        let height = if show_chrome {
            SizeSpec::Hug
        } else {
            SizeSpec::Fill
        };

        let mut container = ContainerWidget::column()
            .with_width(width)
            .with_height(height)
            .with_clip(true);

        if show_chrome {
            // Header: left margin + title + fill spacer + close button + right margin.
            let title = LabelWidget::new("Settings").with_style(LabelStyle {
                font_size: TITLE_FONT_SIZE,
                ..LabelStyle::default()
            });

            let close_btn = ButtonWidget::new("\u{00d7}") // × character
                .with_style(ButtonStyle {
                    padding: Insets::vh(4.0, 10.0),
                    border_width: 0.0,
                    bg: Color::TRANSPARENT,
                    ..ButtonStyle::default()
                });
            let close_btn = IdOverrideButton::new(close_btn, close_id);

            let header = ContainerWidget::row()
                .with_align(Align::Center)
                .with_width(SizeSpec::Fill)
                .with_height(SizeSpec::Fixed(HEADER_HEIGHT))
                .with_child(Box::new(SpacerWidget::fixed(HEADER_PADDING.left, 0.0)))
                .with_child(Box::new(title))
                .with_child(Box::new(SpacerWidget::fill()))
                .with_child(Box::new(close_btn))
                .with_child(Box::new(SpacerWidget::fixed(HEADER_PADDING.right, 0.0)));

            let header_sep = SeparatorWidget::horizontal();

            container = container
                .with_child(Box::new(header))
                .with_child(Box::new(header_sep));
        }

        container = container
            .with_child(Box::new(body))
            .with_child(Box::new(footer_sep))
            .with_child(Box::new(footer));

        Self {
            id: panel_id,
            close_id,
            save_id,
            cancel_id,
            reset_id,
            container,
            cached_layout: RefCell::new(None),
            show_chrome,
        }
    }

    /// Returns the close button's `WidgetId`.
    ///
    /// Only meaningful when `show_chrome` is `true`. When embedded, the
    /// returned ID points to an unused widget.
    #[allow(
        dead_code,
        reason = "settings panel API — used when overlay settings is wired"
    )]
    pub(crate) fn close_id(&self) -> WidgetId {
        self.close_id
    }

    /// Returns the save button's `WidgetId`.
    #[cfg(test)]
    pub(crate) fn save_id(&self) -> WidgetId {
        self.save_id
    }

    /// Returns the cancel button's `WidgetId`.
    #[cfg(test)]
    pub(crate) fn cancel_id(&self) -> WidgetId {
        self.cancel_id
    }

    /// Returns the width spec for the panel's root layout box.
    fn width_spec(&self) -> SizeSpec {
        if self.show_chrome {
            SizeSpec::Fixed(PANEL_WIDTH)
        } else {
            SizeSpec::Fill
        }
    }

    /// Clears the cached layout so it is recomputed on the next draw.
    ///
    /// Call this when external state that affects layout changes (e.g.
    /// scale factor / DPI), since the cache is keyed on bounds only.
    pub fn invalidate_cache(&self) {
        *self.cached_layout.borrow_mut() = None;
    }

    /// Returns cached layout if bounds match, otherwise recomputes.
    fn get_or_compute_layout(
        &self,
        measurer: &dyn TextMeasurer,
        theme: &UiTheme,
        bounds: crate::geometry::Rect,
    ) -> Rc<LayoutNode> {
        {
            let cached = self.cached_layout.borrow();
            if let Some((ref cb, ref node)) = *cached {
                if *cb == bounds {
                    return Rc::clone(node);
                }
            }
        }
        let ctx = LayoutCtx { measurer, theme };
        let wrapper = self.layout(&ctx);
        let node = Rc::new(compute_layout(&wrapper, bounds));
        *self.cached_layout.borrow_mut() = Some((bounds, Rc::clone(&node)));
        node
    }
}

impl Widget for SettingsPanel {
    fn id(&self) -> WidgetId {
        self.id
    }

    fn is_focusable(&self) -> bool {
        false
    }

    fn layout(&self, ctx: &LayoutCtx<'_>) -> LayoutBox {
        let child_box = self.container.layout(ctx);
        let width = self.width_spec();
        // Embedded mode fills the dialog height so the footer stays pinned.
        let height = if self.show_chrome {
            SizeSpec::Hug
        } else {
            SizeSpec::Fill
        };
        LayoutBox::flex(Direction::Column, vec![child_box])
            .with_width(width)
            .with_height(height)
            .with_widget_id(self.id)
    }

    fn sense(&self) -> Sense {
        Sense::none()
    }

    fn paint(&self, ctx: &mut DrawCtx<'_>) {
        let bg = ctx.theme.bg_primary;

        if self.show_chrome {
            // Overlay mode: panel background with rounded corners and shadow.
            ctx.scene.push_layer_bg(bg);
            let bg_style = RectStyle::filled(bg)
                .with_border(1.0, ctx.theme.border)
                .with_radius(CORNER_RADIUS)
                .with_shadow(Shadow {
                    offset_x: 0.0,
                    offset_y: 4.0,
                    blur_radius: 16.0,
                    spread: 0.0,
                    color: ctx.theme.shadow,
                });
            ctx.scene.push_quad(ctx.bounds, bg_style);
        }

        // Draw the inner container.
        let layout = self.get_or_compute_layout(ctx.measurer, ctx.theme, ctx.bounds);
        if let Some(child_node) = layout.children.first() {
            // Sticky footer background: draw an opaque bar covering the
            // separator + footer area BEFORE the container draws, so the
            // container's separator and buttons render on top of it.
            // Child layout: [header?, scroll_content, separator, footer].
            let children = &child_node.children;
            debug_assert!(
                children.len() >= 2,
                "settings panel expects at least separator + footer"
            );
            if let Some(sep_idx) = children.len().checked_sub(2) {
                if let Some(sep_node) = children.get(sep_idx) {
                    let footer_bg = crate::geometry::Rect::new(
                        ctx.bounds.x(),
                        sep_node.rect.y(),
                        ctx.bounds.width(),
                        ctx.bounds.bottom() - sep_node.rect.y(),
                    );
                    ctx.scene.push_quad(footer_bg, RectStyle::filled(bg));
                }
            }

            let mut child_ctx = DrawCtx {
                measurer: ctx.measurer,
                scene: ctx.scene,
                bounds: child_node.content_rect,
                now: ctx.now,
                theme: ctx.theme,
                icons: ctx.icons,
                interaction: None,
                widget_id: None,
                frame_requests: ctx.frame_requests,
            };
            self.container.paint(&mut child_ctx);
        }

        if self.show_chrome {
            ctx.scene.pop_layer_bg();
        }
    }

    fn for_each_child_mut(&mut self, visitor: &mut dyn FnMut(&mut dyn Widget)) {
        visitor(&mut self.container);
    }

    fn on_action(
        &mut self,
        action: WidgetAction,
        _bounds: crate::geometry::Rect,
    ) -> Option<WidgetAction> {
        // Translate button clicks to semantic settings actions.
        match action {
            WidgetAction::Clicked(id) if id == self.close_id || id == self.cancel_id => {
                Some(WidgetAction::CancelSettings)
            }
            WidgetAction::Clicked(id) if id == self.save_id => Some(WidgetAction::SaveSettings),
            WidgetAction::Clicked(id) if id == self.reset_id => Some(WidgetAction::ResetDefaults),
            _ => Some(action),
        }
    }

    fn accept_action(&mut self, action: &WidgetAction) -> bool {
        let handled = self.container.accept_action(action);
        if handled {
            // Structural changes (page switch, widget add/remove) invalidate
            // the cached layout so the next paint recomputes it with the new
            // active page's widgets.
            self.invalidate_cache();
        }
        handled
    }

    fn focusable_children(&self) -> Vec<WidgetId> {
        self.container.focusable_children()
    }

    fn key_context(&self) -> Option<&'static str> {
        Some("Settings")
    }
}

#[cfg(test)]
mod tests;
