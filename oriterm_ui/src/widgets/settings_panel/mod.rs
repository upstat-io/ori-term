//! Settings panel widget — modal overlay container for settings forms.
//!
//! Composes a header bar (title + close button), a separator, and a scrollable
//! `FormLayout` body. The close button emits `WidgetAction::DismissOverlay`
//! (translated from the button's `Clicked` action).

use std::cell::RefCell;
use std::rc::Rc;

use crate::color::Color;
use crate::draw::{RectStyle, Shadow};
use crate::geometry::{Insets, Point};
use crate::input::{HoverEvent, KeyEvent, MouseButton, MouseEvent, MouseEventKind};
use crate::layout::{Align, Direction, LayoutBox, LayoutNode, SizeSpec, compute_layout};
use crate::widget_id::WidgetId;

use crate::theme::UiTheme;

use super::button::{ButtonStyle, ButtonWidget};
use super::container::ContainerWidget;
use super::form_layout::FormLayout;
use super::label::{LabelStyle, LabelWidget};
use super::scroll::ScrollWidget;
use super::separator::SeparatorWidget;
use super::spacer::SpacerWidget;
use super::{DrawCtx, EventCtx, LayoutCtx, TextMeasurer, Widget, WidgetAction, WidgetResponse};

/// Fixed width of the settings panel in logical pixels.
const PANEL_WIDTH: f32 = 600.0;

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
    container: ContainerWidget,
    cached_layout: RefCell<Option<(crate::geometry::Rect, Rc<LayoutNode>)>>,
    /// Last cursor position during a header drag (screen-space).
    drag_origin: Option<Point>,
    /// Whether the panel draws its own chrome (header, shadow, border).
    /// `false` when embedded in a dialog window.
    show_chrome: bool,
}

impl SettingsPanel {
    /// Creates a settings panel wrapping the given form layout.
    ///
    /// Includes a header bar (title + close button), panel chrome (shadow,
    /// border, rounded corners), and drag support. Use this when the panel
    /// is shown as an overlay inside a terminal window.
    pub fn new(form: FormLayout) -> Self {
        Self::build(form, true)
    }

    /// Creates a settings panel without header or chrome.
    ///
    /// Omits the title bar, close button, shadow, border, and rounded
    /// corners — all of which are provided by the dialog window's own
    /// chrome. Use this when the panel is embedded in a dialog OS window.
    pub fn embedded(form: FormLayout) -> Self {
        Self::build(form, false)
    }

    /// Internal builder shared by `new()` and `embedded()`.
    fn build(form: FormLayout, show_chrome: bool) -> Self {
        let close_id = WidgetId::next();
        let save_id = WidgetId::next();
        let cancel_id = WidgetId::next();
        let panel_id = WidgetId::next();

        // Body: form wrapped in vertical scroll.
        let scroll = ScrollWidget::vertical(Box::new(form));

        // Footer: separator + right-aligned Cancel and Save buttons.
        let footer_sep = SeparatorWidget::horizontal();

        let cancel_btn = ButtonWidget::new("Cancel").with_style(ButtonStyle {
            padding: Insets::vh(6.0, 16.0),
            border_width: 1.0,
            ..ButtonStyle::default()
        });
        let cancel_btn = IdOverrideButton {
            inner: cancel_btn,
            id_override: cancel_id,
        };

        let save_btn = ButtonWidget::new("Save").with_style(ButtonStyle {
            padding: Insets::vh(6.0, 20.0),
            border_width: 0.0,
            bg: Color::rgb(0.25, 0.52, 0.96),
            fg: Color::WHITE,
            ..ButtonStyle::default()
        });
        let save_btn = IdOverrideButton {
            inner: save_btn,
            id_override: save_id,
        };

        let footer = ContainerWidget::row()
            .with_align(Align::Center)
            .with_width(SizeSpec::Fill)
            .with_height(SizeSpec::Fixed(FOOTER_HEIGHT))
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

        let mut container = ContainerWidget::column()
            .with_width(width)
            .with_height(SizeSpec::Hug)
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
            let close_btn = IdOverrideButton {
                inner: close_btn,
                id_override: close_id,
            };

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
            .with_child(Box::new(scroll))
            .with_child(Box::new(footer_sep))
            .with_child(Box::new(footer));

        Self {
            id: panel_id,
            close_id,
            save_id,
            cancel_id,
            container,
            cached_layout: RefCell::new(None),
            drag_origin: None,
            show_chrome,
        }
    }

    /// Returns the close button's `WidgetId`.
    pub fn close_id(&self) -> WidgetId {
        self.close_id
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
        let child_box = self.container.layout(&ctx);
        let width = if self.show_chrome {
            SizeSpec::Fixed(PANEL_WIDTH)
        } else {
            SizeSpec::Fill
        };
        let wrapper = LayoutBox::flex(Direction::Column, vec![child_box])
            .with_width(width)
            .with_height(SizeSpec::Hug)
            .with_widget_id(self.id);
        let node = Rc::new(compute_layout(&wrapper, bounds));
        *self.cached_layout.borrow_mut() = Some((bounds, Rc::clone(&node)));
        node
    }

    /// Returns `true` if the position is within the header drag zone.
    ///
    /// The drag zone covers the full header except the close button area
    /// (rightmost ~50px). This prevents drag from intercepting close clicks.
    fn is_header_drag_zone(pos: Point, bounds: crate::geometry::Rect) -> bool {
        let header_bottom = bounds.y() + HEADER_HEIGHT;
        let close_btn_left = bounds.x() + bounds.width() - 50.0;
        pos.y >= bounds.y() && pos.y < header_bottom && pos.x < close_btn_left
    }

    /// Intercepts button `Clicked` actions and translates to semantic actions.
    ///
    /// Close (×) and Cancel → `CancelSettings`. Save → `SaveSettings`.
    fn translate_action(&self, response: WidgetResponse) -> WidgetResponse {
        match response.action {
            Some(WidgetAction::Clicked(id)) if id == self.close_id || id == self.cancel_id => {
                WidgetResponse {
                    response: response.response,
                    action: Some(WidgetAction::CancelSettings),
                    capture: response.capture,
                }
            }
            Some(WidgetAction::Clicked(id)) if id == self.save_id => WidgetResponse {
                response: response.response,
                action: Some(WidgetAction::SaveSettings),
                capture: response.capture,
            },
            _ => response,
        }
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
        let width = if self.show_chrome {
            SizeSpec::Fixed(PANEL_WIDTH)
        } else {
            SizeSpec::Fill
        };
        LayoutBox::flex(Direction::Column, vec![child_box])
            .with_width(width)
            .with_height(SizeSpec::Hug)
            .with_widget_id(self.id)
    }

    fn draw(&self, ctx: &mut DrawCtx<'_>) {
        let bg = ctx.theme.bg_primary;

        if self.show_chrome {
            // Overlay mode: panel background with rounded corners and shadow.
            ctx.draw_list.push_layer(bg);
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
            ctx.draw_list.push_rect(ctx.bounds, bg_style);
        }

        // Draw the inner container.
        let layout = self.get_or_compute_layout(ctx.measurer, ctx.theme, ctx.bounds);
        if let Some(child_node) = layout.children.first() {
            let mut child_ctx = DrawCtx {
                measurer: ctx.measurer,
                draw_list: ctx.draw_list,
                bounds: child_node.content_rect,
                focused_widget: ctx.focused_widget,
                now: ctx.now,
                animations_running: ctx.animations_running,
                theme: ctx.theme,
                icons: ctx.icons,
            };
            self.container.draw(&mut child_ctx);
        }

        if self.show_chrome {
            ctx.draw_list.pop_layer();
        }
    }

    fn handle_mouse(&mut self, event: &MouseEvent, ctx: &EventCtx<'_>) -> WidgetResponse {
        // Overlay mode: header drag support.
        if self.show_chrome {
            // Active drag: track movement and emit MoveOverlay deltas.
            if self.drag_origin.is_some() {
                return match event.kind {
                    MouseEventKind::Move => {
                        let origin = self.drag_origin.unwrap();
                        let dx = event.pos.x - origin.x;
                        let dy = event.pos.y - origin.y;
                        self.drag_origin = Some(event.pos);
                        WidgetResponse::paint().with_action(WidgetAction::MoveOverlay {
                            delta_x: dx,
                            delta_y: dy,
                        })
                    }
                    MouseEventKind::Up(MouseButton::Left) => {
                        self.drag_origin = None;
                        WidgetResponse::paint().with_release_capture()
                    }
                    _ => WidgetResponse::handled(),
                };
            }

            // Start drag on mouse-down in the header drag zone.
            if matches!(event.kind, MouseEventKind::Down(MouseButton::Left))
                && Self::is_header_drag_zone(event.pos, ctx.bounds)
            {
                self.drag_origin = Some(event.pos);
                return WidgetResponse::handled().with_capture();
            }
        }

        // Delegate non-drag events to children.
        let layout = self.get_or_compute_layout(ctx.measurer, ctx.theme, ctx.bounds);
        if let Some(child_node) = layout.children.first() {
            let child_ctx = EventCtx {
                measurer: ctx.measurer,
                bounds: child_node.content_rect,
                is_focused: false,
                focused_widget: ctx.focused_widget,
                theme: ctx.theme,
            };
            let resp = self.container.handle_mouse(event, &child_ctx);
            if resp.response.needs_layout() {
                *self.cached_layout.borrow_mut() = None;
            }
            return self.translate_action(resp);
        }
        WidgetResponse::handled()
    }

    fn handle_hover(&mut self, event: HoverEvent, ctx: &EventCtx<'_>) -> WidgetResponse {
        let layout = self.get_or_compute_layout(ctx.measurer, ctx.theme, ctx.bounds);
        if let Some(child_node) = layout.children.first() {
            let child_ctx = EventCtx {
                measurer: ctx.measurer,
                bounds: child_node.content_rect,
                is_focused: false,
                focused_widget: ctx.focused_widget,
                theme: ctx.theme,
            };
            return self.container.handle_hover(event, &child_ctx);
        }
        WidgetResponse::handled()
    }

    fn handle_key(&mut self, event: KeyEvent, ctx: &EventCtx<'_>) -> WidgetResponse {
        let layout = self.get_or_compute_layout(ctx.measurer, ctx.theme, ctx.bounds);
        if let Some(child_node) = layout.children.first() {
            let child_ctx = EventCtx {
                measurer: ctx.measurer,
                bounds: child_node.content_rect,
                is_focused: false,
                focused_widget: ctx.focused_widget,
                theme: ctx.theme,
            };
            let resp = self.container.handle_key(event, &child_ctx);
            if resp.response.needs_layout() {
                *self.cached_layout.borrow_mut() = None;
            }
            return self.translate_action(resp);
        }
        WidgetResponse::handled()
    }

    fn accept_action(&mut self, action: &WidgetAction) -> bool {
        self.container.accept_action(action)
    }

    fn focusable_children(&self) -> Vec<WidgetId> {
        self.container.focusable_children()
    }
}

/// Wrapper around `ButtonWidget` that overrides its `WidgetId`.
///
/// Needed because `ButtonWidget::new()` generates its own ID internally,
/// but we need a known ID to intercept the `Clicked` action.
struct IdOverrideButton {
    inner: ButtonWidget,
    id_override: WidgetId,
}

impl Widget for IdOverrideButton {
    fn id(&self) -> WidgetId {
        self.id_override
    }

    fn is_focusable(&self) -> bool {
        self.inner.is_focusable()
    }

    fn layout(&self, ctx: &LayoutCtx<'_>) -> LayoutBox {
        // Rewrite the widget id on the layout box.
        let mut lb = self.inner.layout(ctx);
        lb = lb.with_widget_id(self.id_override);
        lb
    }

    fn draw(&self, ctx: &mut DrawCtx<'_>) {
        self.inner.draw(ctx);
    }

    fn handle_mouse(&mut self, event: &MouseEvent, ctx: &EventCtx<'_>) -> WidgetResponse {
        let resp = self.inner.handle_mouse(event, ctx);
        // Rewrite the clicked id to our override.
        match resp.action {
            Some(WidgetAction::Clicked(_)) => WidgetResponse {
                response: resp.response,
                action: Some(WidgetAction::Clicked(self.id_override)),
                capture: resp.capture,
            },
            _ => resp,
        }
    }

    fn handle_hover(&mut self, event: HoverEvent, ctx: &EventCtx<'_>) -> WidgetResponse {
        self.inner.handle_hover(event, ctx)
    }

    fn handle_key(&mut self, event: KeyEvent, ctx: &EventCtx<'_>) -> WidgetResponse {
        let resp = self.inner.handle_key(event, ctx);
        match resp.action {
            Some(WidgetAction::Clicked(_)) => WidgetResponse {
                response: resp.response,
                action: Some(WidgetAction::Clicked(self.id_override)),
                capture: resp.capture,
            },
            _ => resp,
        }
    }

    fn focusable_children(&self) -> Vec<WidgetId> {
        if self.is_focusable() {
            vec![self.id_override]
        } else {
            Vec::new()
        }
    }
}

#[cfg(test)]
mod tests;
