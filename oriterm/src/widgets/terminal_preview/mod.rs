//! Terminal preview widget scaffold.
//!
//! A scaled-down live preview of a terminal tab, displayed in an overlay
//! on tab hover (Chrome/Windows-style tab preview). Full rendering via
//! offscreen texture is deferred until the Image pipeline and overlay
//! system are wired (later sections).
#![allow(dead_code, reason = "scaffold — wired in tab hover preview section")]

use oriterm_ui::draw::RectStyle;
use oriterm_ui::input::{HoverEvent, KeyEvent, MouseEvent};
use oriterm_ui::layout::LayoutBox;
use oriterm_ui::sense::Sense;
use oriterm_ui::widget_id::WidgetId;
use oriterm_ui::widgets::{DrawCtx, EventCtx, LayoutCtx, Widget, WidgetResponse};

/// Default preview width in logical pixels.
const DEFAULT_PREVIEW_WIDTH: f32 = 320.0;

/// Default preview height in logical pixels.
const DEFAULT_PREVIEW_HEIGHT: f32 = 200.0;

/// Default scale factor for thumbnail rendering.
const DEFAULT_SCALE: f32 = 0.25;

/// Corner radius for the preview frame.
const CORNER_RADIUS: f32 = 6.0;

/// Scaled-down live preview of a terminal tab.
///
/// Currently a placeholder that draws a rounded rectangle frame.
/// Full rendering (offscreen texture → `DrawCommand::Image`) is deferred
/// until the Image pipeline and overlay system are available.
pub(crate) struct TerminalPreviewWidget {
    /// Unique widget ID.
    id: WidgetId,
    /// Preview width in logical pixels.
    preview_width: f32,
    /// Preview height in logical pixels.
    preview_height: f32,
    /// Scale factor for thumbnail rendering.
    #[allow(dead_code, reason = "used when offscreen texture rendering is wired")]
    scale: f32,
}

impl TerminalPreviewWidget {
    /// Creates a preview widget with default dimensions.
    pub(crate) fn new() -> Self {
        Self {
            id: WidgetId::next(),
            preview_width: DEFAULT_PREVIEW_WIDTH,
            preview_height: DEFAULT_PREVIEW_HEIGHT,
            scale: DEFAULT_SCALE,
        }
    }

    /// Creates a preview widget with custom dimensions and scale.
    #[allow(dead_code, reason = "used when preview rendering is wired")]
    pub(crate) fn with_size(width: f32, height: f32, scale: f32) -> Self {
        Self {
            id: WidgetId::next(),
            preview_width: width,
            preview_height: height,
            scale,
        }
    }
}

impl Widget for TerminalPreviewWidget {
    fn id(&self) -> WidgetId {
        self.id
    }

    fn sense(&self) -> Sense {
        Sense::none()
    }

    fn layout(&self, _ctx: &LayoutCtx<'_>) -> LayoutBox {
        LayoutBox::leaf(self.preview_width, self.preview_height).with_widget_id(self.id)
    }

    fn paint(&self, ctx: &mut DrawCtx<'_>) {
        // Placeholder: rounded rectangle frame with theme background.
        let style = RectStyle {
            fill: Some(ctx.theme.bg_secondary),
            border: None,
            corner_radius: [CORNER_RADIUS; 4],
            shadow: None,
            gradient: None,
        };
        ctx.draw_list.push_rect(ctx.bounds, style);
    }

    fn handle_mouse(&mut self, _event: &MouseEvent, _ctx: &EventCtx<'_>) -> WidgetResponse {
        WidgetResponse::ignored()
    }

    fn handle_hover(&mut self, _event: HoverEvent, _ctx: &EventCtx<'_>) -> WidgetResponse {
        WidgetResponse::ignored()
    }

    fn handle_key(&mut self, _event: KeyEvent, _ctx: &EventCtx<'_>) -> WidgetResponse {
        WidgetResponse::ignored()
    }
}

#[cfg(test)]
mod tests;
