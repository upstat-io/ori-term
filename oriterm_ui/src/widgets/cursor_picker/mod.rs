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

/// Width of each option card.
const CARD_WIDTH: f32 = 80.0;

/// Height of each option card.
const CARD_HEIGHT: f32 = 72.0;

/// Gap between cards.
const CARD_GAP: f32 = 10.0;

/// Card corner radius.
const CARD_RADIUS: f32 = 8.0;

/// Demo character to display in the cursor preview.
const DEMO_CHAR: &str = "A";

/// Font size for the cursor demo character.
const DEMO_FONT_SIZE: f32 = 18.0;

/// Font size for the label below the demo.
const LABEL_FONT_SIZE: f32 = 10.0;

/// Cursor style options.
const OPTIONS: [&str; OPTION_COUNT] = ["Block", "Bar", "Underline"];

/// Total widget width.
const TOTAL_WIDTH: f32 = OPTION_COUNT as f32 * CARD_WIDTH + (OPTION_COUNT as f32 - 1.0) * CARD_GAP;

/// Visual cursor style picker with 3 options.
///
/// Each card shows a cursor demo and label. The selected card has an accent
/// border; hover adds a subtle border.
pub struct CursorPickerWidget {
    id: WidgetId,
    selected: usize,
    controllers: Vec<Box<dyn EventController>>,
    animator: VisualStateAnimator,
}

impl CursorPickerWidget {
    /// Creates a cursor picker with the given initial selection.
    pub fn new(selected: usize, theme: &UiTheme) -> Self {
        Self {
            id: WidgetId::next(),
            selected: selected.min(OPTION_COUNT - 1),
            controllers: vec![Box::new(HoverController::new())],
            animator: VisualStateAnimator::new(vec![common_states(
                Color::TRANSPARENT,
                theme.bg_hover,
                theme.bg_active,
                Color::TRANSPARENT,
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
        let cy = card_rect.y() + 16.0;

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

            // Card background + border.
            let (border_color, border_w) = if is_sel {
                (accent, 2.0)
            } else {
                (ctx.theme.border, 1.0)
            };
            let bg = if is_sel {
                ctx.theme.accent_bg
            } else {
                Color::TRANSPARENT
            };
            let style = RectStyle::filled(bg)
                .with_radius(CARD_RADIUS)
                .with_border(border_w, border_color);
            ctx.scene.push_quad(card, style);

            // Cursor demo.
            Self::paint_cursor_demo(ctx, card, i, accent);

            // Label.
            let lbl_style = TextStyle::new(LABEL_FONT_SIZE, ctx.theme.fg_secondary);
            let shaped = ctx.measurer.shape(label, &lbl_style, CARD_WIDTH);
            let lx = x + (CARD_WIDTH - shaped.width) / 2.0;
            let ly = y0 + CARD_HEIGHT - 16.0;
            ctx.scene
                .push_text(Point::new(lx, ly), shaped, ctx.theme.fg_secondary);
        }

        if self.animator.is_animating(ctx.now) {
            ctx.request_anim_frame();
        }
    }

    fn on_input(&mut self, event: &crate::input::InputEvent, bounds: Rect) -> super::OnInputResult {
        if let crate::input::InputEvent::MouseDown { pos, .. } = event {
            let local_x = pos.x - bounds.x();
            if let Some(idx) = Self::hit_test_card(local_x) {
                if idx != self.selected {
                    self.selected = idx;
                    return super::OnInputResult::handled().with_action(WidgetAction::Selected {
                        id: self.id,
                        index: idx,
                    });
                }
            }
        }
        super::OnInputResult::ignored()
    }
}

#[cfg(test)]
mod tests;
