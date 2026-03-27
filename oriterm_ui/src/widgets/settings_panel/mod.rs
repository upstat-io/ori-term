//! Settings panel widget — modal overlay container for settings forms.
//!
//! Composes an optional header bar (title + close button) and the content
//! widget (sidebar + right column with pages and footer). The close button
//! and footer button clicks are translated to semantic settings actions.
//!
//! The footer itself lives in `SettingsFooterWidget` — the panel receives
//! its button IDs via the constructor and maps `Clicked` actions to
//! `SaveSettings`, `CancelSettings`, or `ResetDefaults`.

use std::cell::RefCell;
use std::rc::Rc;

use crate::color::Color;
use crate::draw::{RectStyle, Shadow};
use crate::geometry::Insets;
use crate::layout::{Align, Direction, LayoutBox, LayoutNode, SizeSpec, compute_layout};
use crate::sense::Sense;
use crate::widget_id::WidgetId;

use crate::theme::UiTheme;

use crate::text::FontWeight;

use super::button::id_override::IdOverrideButton;
use super::button::{ButtonStyle, ButtonWidget};
use super::container::ContainerWidget;
use super::label::{LabelStyle, LabelWidget};
use super::spacer::SpacerWidget;
use super::{DrawCtx, LayoutCtx, TextMeasurer, Widget, WidgetAction};

/// Fixed width of the settings panel in logical pixels.
const PANEL_WIDTH: f32 = 860.0;

/// Height of the header bar in logical pixels.
const HEADER_HEIGHT: f32 = 48.0;

/// Font size for the header title.
const TITLE_FONT_SIZE: f32 = 16.0;

/// Padding inside the header bar.
const HEADER_PADDING: Insets = Insets::vh(0.0, 16.0);

/// Corner radius for the panel.
const CORNER_RADIUS: f32 = 0.0;

/// A modal settings panel with optional header bar and content area.
///
/// In overlay mode, the header contains a "Settings" title label and a close
/// button (×). In embedded (dialog) mode, the header is omitted since the
/// dialog window provides its own chrome.
///
/// The footer buttons (Reset, Cancel, Save) live in `SettingsFooterWidget`,
/// which is part of the content tree. This panel receives the button IDs via
/// its constructor and translates `Clicked` actions to semantic actions.
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
    /// Includes a header bar (title + close button), panel chrome (shadow,
    /// border, rounded corners). Use this when the panel is shown as an
    /// overlay inside a terminal window. `footer_ids` are
    /// `(reset_id, cancel_id, save_id)` from the footer widget.
    pub fn new(content: Box<dyn Widget>, footer_ids: (WidgetId, WidgetId, WidgetId)) -> Self {
        Self::build(content, true, footer_ids)
    }

    /// Creates a settings panel without header or chrome.
    ///
    /// Omits the title bar, close button, shadow, border, and rounded
    /// corners — all of which are provided by the dialog window's own
    /// chrome. Use this when the panel is embedded in a dialog OS window.
    pub fn embedded(content: Box<dyn Widget>, footer_ids: (WidgetId, WidgetId, WidgetId)) -> Self {
        Self::build(content, false, footer_ids)
    }

    /// Internal builder shared by `new()` and `embedded()`.
    fn build(
        content: Box<dyn Widget>,
        show_chrome: bool,
        footer_ids: (WidgetId, WidgetId, WidgetId),
    ) -> Self {
        let close_id = WidgetId::next();
        let (reset_id, cancel_id, save_id) = footer_ids;
        let panel_id = WidgetId::next();

        // Body: the content widget fills remaining height. Clip so scroll
        // content cannot overflow into the header area.
        let body = ContainerWidget::row()
            .with_width(SizeSpec::Fill)
            .with_height(SizeSpec::Fill)
            .with_clip(true)
            .with_child(content);

        // Build the column layout. Overlay mode gets a header; embedded skips it.
        let width = if show_chrome {
            SizeSpec::Fixed(PANEL_WIDTH)
        } else {
            SizeSpec::Fill
        };

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
                weight: FontWeight::BOLD,
                line_height: None,
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

            let header_sep = super::separator::SeparatorWidget::horizontal();

            container = container
                .with_child(Box::new(header))
                .with_child(Box::new(header_sep));
        }

        container = container.with_child(Box::new(body));

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
            ctx.scene.push_layer_bg(bg);
            let bg_style = RectStyle::filled(bg)
                .with_border(2.0, ctx.theme.border_strong)
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

        let layout = self.get_or_compute_layout(ctx.measurer, ctx.theme, ctx.bounds);
        if let Some(child_node) = layout.children.first() {
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
            // Structural changes (page switch, dirty state, widget add/remove)
            // invalidate the cached layout so the next paint recomputes it.
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
