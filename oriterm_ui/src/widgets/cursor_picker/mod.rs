//! Cursor style picker widget.
//!
//! Displays three side-by-side cards (Block, Bar, Underline) with cursor
//! demos. Click to select. Emits `WidgetAction::Selected` with the cursor
//! style index.

use crate::color::Color;
use crate::controllers::{EventController, HoverController};
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

/// Number of cursor style options.
const OPTION_COUNT: usize = 3;

/// Width of each option card (mockup `min-width: 80px`, box-sizing total).
const CARD_WIDTH: f32 = 80.0;

/// Gap between cards (mockup `.cursor-preview { gap: 24px }`).
pub(super) const CARD_GAP: f32 = 24.0;

/// Card corner radius.
const CARD_RADIUS: f32 = 0.0;

/// Card vertical padding (mockup `padding: 12px 20px`).
const CARD_PAD_V: f32 = 12.0;

/// Card border width.
const CARD_BORDER: f32 = 2.0;

/// Demo character to display in the cursor preview.
const DEMO_CHAR: &str = "A";

/// Font size for the cursor demo character (mockup `font-size: 16px`).
pub(super) const DEMO_FONT_SIZE: f32 = 16.0;

/// Font size for the label below the demo (mockup `font-size: 11px`).
pub(super) const LABEL_FONT_SIZE: f32 = 11.0;

/// Gap between demo and label (mockup `gap: 6px`).
const DEMO_LABEL_GAP: f32 = 6.0;

/// Cursor style options.
const OPTIONS: [&str; OPTION_COUNT] = ["Block", "Bar", "Underline"];

/// Total widget width.
const TOTAL_WIDTH: f32 = OPTION_COUNT as f32 * CARD_WIDTH + (OPTION_COUNT as f32 - 1.0) * CARD_GAP;

/// Card height: top pad + demo height (16px) + gap + label height (11px) + bottom pad.
/// Using approximate line heights: demo ~20px, label ~14px.
pub(super) const CARD_HEIGHT: f32 = CARD_PAD_V + 20.0 + DEMO_LABEL_GAP + 14.0 + CARD_PAD_V;

/// Visual cursor style picker with 3 options.
///
/// Each card shows a cursor demo and label. The selected card has an accent
/// border; hover adds a subtle border.
pub struct CursorPickerWidget {
    id: WidgetId,
    selected: usize,
    /// Per-card hover tracking (manual hit testing).
    hovered_card: Option<usize>,
    controllers: Vec<Box<dyn EventController>>,
    animator: VisualStateAnimator,
}

impl CursorPickerWidget {
    /// Creates a cursor picker with the given initial selection.
    pub fn new(selected: usize, theme: &UiTheme) -> Self {
        Self {
            id: WidgetId::next(),
            selected: selected.min(OPTION_COUNT - 1),
            hovered_card: None,
            controllers: vec![Box::new(HoverController::new())],
            animator: VisualStateAnimator::new(vec![common_states(
                theme.bg_card,
                theme.bg_hover,
                theme.bg_active,
                theme.bg_card,
            )]),
        }
    }

    /// Returns the selected cursor style index.
    pub fn selected(&self) -> usize {
        self.selected
    }

    /// Sets the selected cursor style index.
    pub fn set_selected(&mut self, index: usize) {
        if index < OPTION_COUNT {
            self.selected = index;
        }
    }

    /// Hit tests a local X coordinate to a card index.
    fn hit_test_card(local_x: f32) -> Option<usize> {
        for i in 0..OPTION_COUNT {
            let x = i as f32 * (CARD_WIDTH + CARD_GAP);
            if local_x >= x && local_x < x + CARD_WIDTH {
                return Some(i);
            }
        }
        None
    }

    /// Paints the cursor demo for one card.
    fn paint_cursor_demo(
        ctx: &mut DrawCtx<'_>,
        card_rect: Rect,
        style_index: usize,
        accent: Color,
    ) {
        let cx = card_rect.x() + card_rect.width() / 2.0;
        let cy = card_rect.y() + CARD_PAD_V;

        let text_style = TextStyle::new(DEMO_FONT_SIZE, ctx.theme.fg_primary);
        let shaped = ctx.measurer.shape(DEMO_CHAR, &text_style, CARD_WIDTH);
        let char_w = shaped.width;
        let char_h = shaped.height;
        let tx = cx - char_w / 2.0;

        match style_index {
            0 => {
                // Block: accent background behind character.
                let block = Rect::new(tx - 2.0, cy, char_w + 4.0, char_h);
                ctx.scene.push_quad(block, RectStyle::filled(accent));
                ctx.scene
                    .push_text(Point::new(tx, cy), shaped, ctx.theme.bg_primary);
            }
            1 => {
                // Bar: 2px accent bar on left of character.
                ctx.scene
                    .push_text(Point::new(tx, cy), shaped, ctx.theme.fg_primary);
                let bar = Rect::new(tx - 3.0, cy, 2.0, char_h);
                ctx.scene.push_quad(bar, RectStyle::filled(accent));
            }
            _ => {
                // Underline: 2px accent line on bottom of character.
                ctx.scene
                    .push_text(Point::new(tx, cy), shaped, ctx.theme.fg_primary);
                let underline = Rect::new(tx - 2.0, cy + char_h - 2.0, char_w + 4.0, 2.0);
                ctx.scene.push_quad(underline, RectStyle::filled(accent));
            }
        }
    }
}

impl Widget for CursorPickerWidget {
    fn id(&self) -> WidgetId {
        self.id
    }

    fn sense(&self) -> Sense {
        Sense::click()
    }

    fn layout(&self, _ctx: &LayoutCtx<'_>) -> LayoutBox {
        LayoutBox::leaf(TOTAL_WIDTH, CARD_HEIGHT).with_widget_id(self.id)
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

    fn paint(&self, ctx: &mut DrawCtx<'_>) {
        let x0 = ctx.bounds.x();
        let y0 = ctx.bounds.y();
        let accent = ctx.theme.accent;

        for (i, label) in OPTIONS.iter().enumerate() {
            let x = x0 + i as f32 * (CARD_WIDTH + CARD_GAP);
            let card = Rect::new(x, y0, CARD_WIDTH, CARD_HEIGHT);
            let is_sel = i == self.selected;
            let is_hov = self.hovered_card == Some(i);

            // Card background + border (mockup: bg_card rest, bg_hover hover, accent_bg selected).
            let (bg, border_color) = if is_sel {
                (ctx.theme.accent_bg, accent)
            } else if is_hov {
                (ctx.theme.bg_hover, ctx.theme.border_strong)
            } else {
                (ctx.theme.bg_card, ctx.theme.border)
            };
            let style = RectStyle::filled(bg)
                .with_radius(CARD_RADIUS)
                .with_border(CARD_BORDER, border_color);
            ctx.scene.push_quad(card, style);

            // Cursor demo (centered in card with padding).
            Self::paint_cursor_demo(ctx, card, i, accent);

            // Label (mockup `font-size: 11px`, color `--text-muted`).
            let lbl_style = TextStyle::new(LABEL_FONT_SIZE, ctx.theme.fg_secondary);
            let shaped = ctx.measurer.shape(label, &lbl_style, CARD_WIDTH);
            let lx = x + (CARD_WIDTH - shaped.width) / 2.0;
            let ly = card.y() + CARD_HEIGHT - CARD_PAD_V - shaped.height;
            ctx.scene
                .push_text(Point::new(lx, ly), shaped, ctx.theme.fg_secondary);
        }

        if self.animator.is_animating() {
            ctx.request_anim_frame();
        }
    }

    fn on_input(&mut self, event: &crate::input::InputEvent, bounds: Rect) -> super::OnInputResult {
        use crate::input::InputEvent;
        match event {
            InputEvent::MouseMove { pos, .. } => {
                let local_x = pos.x - bounds.x();
                let prev = self.hovered_card;
                self.hovered_card = if bounds.contains(*pos) {
                    Self::hit_test_card(local_x)
                } else {
                    None
                };
                if self.hovered_card != prev {
                    return super::OnInputResult::handled();
                }
                super::OnInputResult::ignored()
            }
            InputEvent::MouseDown { pos, .. } => {
                let local_x = pos.x - bounds.x();
                if let Some(idx) = Self::hit_test_card(local_x) {
                    if idx != self.selected {
                        self.selected = idx;
                        return super::OnInputResult::handled().with_action(
                            WidgetAction::Selected {
                                id: self.id,
                                index: idx,
                            },
                        );
                    }
                }
                super::OnInputResult::ignored()
            }
            _ => super::OnInputResult::ignored(),
        }
    }
}

#[cfg(test)]
mod tests;
