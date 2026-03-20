//! Color scheme preview card.
//!
//! Displays a scheme name, mini terminal preview with syntax coloring,
//! and a swatch bar of the 8 standard ANSI colors. Emits
//! `WidgetAction::Selected` on click.

use crate::color::Color;
use crate::controllers::{ClickController, EventController, HoverController};
use crate::draw::RectStyle;
use crate::geometry::{Point, Rect};
use crate::layout::LayoutBox;
use crate::sense::Sense;
use crate::text::TextStyle;
use crate::theme::UiTheme;
use crate::visual_state::common_states;
use crate::visual_state::transition::VisualStateAnimator;
use crate::widget_id::WidgetId;

use super::{DrawCtx, LayoutCtx, Widget, WidgetAction};

/// Card corner radius.
const CORNER_RADIUS: f32 = 8.0;

/// Vertical padding inside the card.
const CARD_PADDING: f32 = 8.0;

/// Horizontal padding inside the card.
const CARD_PADDING_H: f32 = 10.0;

/// Height of the title bar area.
const TITLE_HEIGHT: f32 = 24.0;

/// Height of the terminal preview area.
const PREVIEW_HEIGHT: f32 = 56.0;

/// Height of the swatch bar area (swatch + padding).
const SWATCH_BAR_HEIGHT: f32 = 20.0;

/// Swatch rectangle height.
const SWATCH_HEIGHT: f32 = 12.0;

/// Gap between swatch rectangles.
const SWATCH_GAP: f32 = 3.0;

/// Swatch corner radius.
const SWATCH_RADIUS: f32 = 2.0;

/// Total card height.
const CARD_HEIGHT: f32 =
    CARD_PADDING + TITLE_HEIGHT + PREVIEW_HEIGHT + SWATCH_BAR_HEIGHT + CARD_PADDING;

/// Card width (natural size; parent grid determines actual).
const CARD_WIDTH: f32 = 200.0;

/// Preview text font size.
const PREVIEW_FONT_SIZE: f32 = 11.0;

/// Title font size.
const TITLE_FONT_SIZE: f32 = 12.0;

/// Badge font size.
const BADGE_FONT_SIZE: f32 = 9.0;

/// Data describing a color scheme for rendering in a [`SchemeCardWidget`].
///
/// Passed via constructor — the widget does not own scheme definitions.
#[derive(Debug, Clone)]
pub struct SchemeCardData {
    /// Scheme display name.
    pub name: String,
    /// Background color.
    pub bg: Color,
    /// Foreground (default text) color.
    pub fg: Color,
    /// Standard ANSI colors 0-7.
    pub ansi: [Color; 8],
    /// Whether this scheme is currently selected.
    pub selected: bool,
}

/// A color scheme preview card.
///
/// Renders a title bar, mini terminal preview, and swatch bar. Click to
/// select. Selected cards show an accent border and background tint.
pub struct SchemeCardWidget {
    id: WidgetId,
    data: SchemeCardData,
    /// Index emitted in `Selected` action.
    scheme_index: usize,

    // Interaction.
    controllers: Vec<Box<dyn EventController>>,
    animator: VisualStateAnimator,
}

impl SchemeCardWidget {
    /// Creates a scheme card from data and its position index.
    pub fn new(data: SchemeCardData, scheme_index: usize, theme: &UiTheme) -> Self {
        Self {
            id: WidgetId::next(),
            controllers: vec![
                Box::new(HoverController::new()),
                Box::new(ClickController::new()),
            ],
            animator: VisualStateAnimator::new(vec![common_states(
                Color::TRANSPARENT,
                theme.bg_hover,
                theme.bg_active,
                Color::TRANSPARENT,
            )]),
            data,
            scheme_index,
        }
    }

    /// Returns the scheme data.
    pub fn data(&self) -> &SchemeCardData {
        &self.data
    }

    /// Returns the scheme index.
    pub fn scheme_index(&self) -> usize {
        self.scheme_index
    }

    /// Sets the selected state.
    pub fn set_selected(&mut self, selected: bool) {
        self.data.selected = selected;
    }

    /// Paints the title bar: scheme name + optional "Active" badge.
    fn paint_title(&self, ctx: &mut DrawCtx<'_>, x: f32, y: f32, w: f32) {
        let style = TextStyle::new(TITLE_FONT_SIZE, ctx.theme.fg_primary);
        let shaped = ctx.measurer.shape(&self.data.name, &style, w);
        ctx.scene
            .push_text(Point::new(x, y), shaped, ctx.theme.fg_primary);

        // "Active" badge when selected.
        if self.data.selected {
            let badge_text = "Active";
            let badge_style = TextStyle::new(BADGE_FONT_SIZE, ctx.theme.accent);
            let badge_shaped = ctx.measurer.shape(badge_text, &badge_style, w);
            let badge_x = x + w - badge_shaped.width - 4.0;
            let badge_y = y + 2.0;
            ctx.scene
                .push_text(Point::new(badge_x, badge_y), badge_shaped, ctx.theme.accent);
        }
    }

    /// Paints the mini terminal preview.
    fn paint_preview(&self, ctx: &mut DrawCtx<'_>, area: Rect) {
        // Preview background.
        let bg_style = RectStyle::filled(self.data.bg).with_radius(4.0);
        ctx.scene.push_quad(area, bg_style);

        let x = area.x() + 6.0;
        let line_h = PREVIEW_FONT_SIZE + 4.0;
        let style = TextStyle::new(PREVIEW_FONT_SIZE, self.data.fg);

        // Line 1: `$ cargo build --release`
        let y1 = area.y() + 6.0;
        let line1 = "$ cargo build --release";
        let shaped1 = ctx.measurer.shape(line1, &style, area.width() - 12.0);
        ctx.scene
            .push_text(Point::new(x, y1), shaped1, self.data.fg);

        // Line 2: `Compiling ori_term v0.1.0` in green (ansi[2]).
        let y2 = y1 + line_h;
        let green = self.data.ansi[2];
        let green_style = TextStyle::new(PREVIEW_FONT_SIZE, green);
        let line2 = "Compiling ori_term v0.1.0";
        let shaped2 = ctx.measurer.shape(line2, &green_style, area.width() - 12.0);
        ctx.scene.push_text(Point::new(x, y2), shaped2, green);
    }

    /// Paints the 8-color swatch bar.
    fn paint_swatch_bar(&self, ctx: &mut DrawCtx<'_>, x: f32, y: f32, w: f32) {
        let count = self.data.ansi.len() as f32;
        let total_gaps = (count - 1.0) * SWATCH_GAP;
        let swatch_w = (w - total_gaps) / count;

        for (i, &color) in self.data.ansi.iter().enumerate() {
            let sx = x + i as f32 * (swatch_w + SWATCH_GAP);
            let rect = Rect::new(sx, y, swatch_w, SWATCH_HEIGHT);
            let style = RectStyle::filled(color).with_radius(SWATCH_RADIUS);
            ctx.scene.push_quad(rect, style);
        }
    }
}

impl Widget for SchemeCardWidget {
    fn id(&self) -> WidgetId {
        self.id
    }

    fn sense(&self) -> Sense {
        Sense::click()
    }

    fn layout(&self, _ctx: &LayoutCtx<'_>) -> LayoutBox {
        LayoutBox::leaf(CARD_WIDTH, CARD_HEIGHT).with_widget_id(self.id)
    }

    fn controllers(&self) -> &[Box<dyn EventController>] {
        &self.controllers
    }

    fn controllers_mut(&mut self) -> &mut [Box<dyn EventController>] {
        &mut self.controllers
    }

    fn visual_states(&self) -> Option<&VisualStateAnimator> {
        Some(&self.animator)
    }

    fn visual_states_mut(&mut self) -> Option<&mut VisualStateAnimator> {
        Some(&mut self.animator)
    }

    fn on_action(&mut self, action: WidgetAction, _bounds: Rect) -> Option<WidgetAction> {
        match action {
            WidgetAction::Clicked(_) => Some(WidgetAction::Selected {
                id: self.id,
                index: self.scheme_index,
            }),
            other => Some(other),
        }
    }

    fn accept_action(&mut self, action: &WidgetAction) -> bool {
        // When any scheme card emits Selected, update this card's visual state.
        if let WidgetAction::Selected { index, .. } = action {
            let should_be_selected = *index == self.scheme_index;
            if self.data.selected != should_be_selected {
                self.data.selected = should_be_selected;
                return true;
            }
        }
        false
    }

    fn paint(&self, ctx: &mut DrawCtx<'_>) {
        let bounds = ctx.bounds;

        // Card background + border.
        let (border_color, border_width) = if self.data.selected {
            (ctx.theme.accent, 2.0)
        } else {
            let hover_bg = self.animator.get_bg_color(ctx.now);
            if hover_bg.a > 0.01 {
                (ctx.theme.border, 1.0)
            } else {
                (Color::TRANSPARENT, 0.0)
            }
        };

        let card_bg = if self.data.selected {
            ctx.theme.accent_bg
        } else {
            self.animator.get_bg_color(ctx.now)
        };

        let card_style = RectStyle::filled(card_bg)
            .with_radius(CORNER_RADIUS)
            .with_border(border_width, border_color);
        ctx.scene.push_quad(bounds, card_style);

        let x = bounds.x() + CARD_PADDING_H;
        let w = bounds.width() - CARD_PADDING_H * 2.0;
        let mut y = bounds.y() + CARD_PADDING;

        // Title bar.
        self.paint_title(ctx, x, y, w);
        y += TITLE_HEIGHT;

        // Terminal preview.
        let preview_rect = Rect::new(x, y, w, PREVIEW_HEIGHT);
        self.paint_preview(ctx, preview_rect);
        y += PREVIEW_HEIGHT + 4.0;

        // Swatch bar.
        self.paint_swatch_bar(ctx, x, y, w);

        // Keep animating during hover transitions.
        if self.animator.is_animating(ctx.now) {
            ctx.request_anim_frame();
        }
    }
}

#[cfg(test)]
mod tests;
