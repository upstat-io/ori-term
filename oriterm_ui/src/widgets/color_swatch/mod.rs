//! Color swatch widgets for palette editing.
//!
//! [`ColorSwatchGrid`] displays an 8-column grid of clickable color cells.
//! [`SpecialColorSwatch`] displays a large swatch with label and hex value.

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

/// Number of columns in the swatch grid.
const GRID_COLUMNS: usize = 8;

/// Swatch cell size (width and height) in logical pixels.
const CELL_SIZE: f32 = 28.0;

/// Corner radius for swatch cells.
const CELL_RADIUS: f32 = 6.0;

/// Gap between grid cells.
const CELL_GAP: f32 = 6.0;

/// Height of the index label below each cell.
const LABEL_HEIGHT: f32 = 14.0;

/// Index label font size.
const INDEX_FONT_SIZE: f32 = 9.5;

/// Total height per cell row (cell + label + gap).
const ROW_HEIGHT: f32 = CELL_SIZE + LABEL_HEIGHT + CELL_GAP;

/// A clickable grid of color swatches in 8 columns.
///
/// Each cell is a colored rounded square with an index label below.
/// Hover enlarges the cell slightly. Click emits `Selected` with the
/// cell index for future color picker integration.
pub struct ColorSwatchGrid {
    id: WidgetId,
    colors: Vec<Color>,
    controllers: Vec<Box<dyn EventController>>,
    animator: VisualStateAnimator,
}

impl ColorSwatchGrid {
    /// Creates a grid from a list of colors.
    pub fn new(colors: Vec<Color>, theme: &UiTheme) -> Self {
        Self {
            id: WidgetId::next(),
            colors,
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
        }
    }

    /// Returns the number of colors.
    pub fn color_count(&self) -> usize {
        self.colors.len()
    }

    /// Computes grid dimensions.
    fn grid_size(&self) -> (f32, f32) {
        let cols = GRID_COLUMNS.min(self.colors.len()) as f32;
        let rows = self.colors.len().div_ceil(GRID_COLUMNS) as f32;
        let w = cols * CELL_SIZE + (cols - 1.0).max(0.0) * CELL_GAP;
        let h = rows * ROW_HEIGHT;
        (w, h)
    }

    /// Hit tests a point to a cell index.
    fn hit_test_cell(&self, local: Point) -> Option<usize> {
        let col = (local.x / (CELL_SIZE + CELL_GAP)) as usize;
        let row = (local.y / ROW_HEIGHT) as usize;
        if col >= GRID_COLUMNS {
            return None;
        }
        let idx = row * GRID_COLUMNS + col;
        if idx < self.colors.len() {
            Some(idx)
        } else {
            None
        }
    }
}

impl Widget for ColorSwatchGrid {
    fn id(&self) -> WidgetId {
        self.id
    }

    fn sense(&self) -> Sense {
        Sense::click()
    }

    fn layout(&self, _ctx: &LayoutCtx<'_>) -> LayoutBox {
        let (w, h) = self.grid_size();
        LayoutBox::leaf(w, h).with_widget_id(self.id)
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

        for (i, &color) in self.colors.iter().enumerate() {
            let col = i % GRID_COLUMNS;
            let row = i / GRID_COLUMNS;
            let x = x0 + col as f32 * (CELL_SIZE + CELL_GAP);
            let y = y0 + row as f32 * ROW_HEIGHT;

            // Color cell.
            let cell_rect = Rect::new(x, y, CELL_SIZE, CELL_SIZE);
            let style = RectStyle::filled(color).with_radius(CELL_RADIUS);
            ctx.draw_list.push_rect(cell_rect, style);

            // Index label.
            let label = i.to_string();
            let label_style = TextStyle::new(INDEX_FONT_SIZE, ctx.theme.fg_faint);
            let shaped = ctx.measurer.shape(&label, &label_style, CELL_SIZE);
            let lx = x + (CELL_SIZE - shaped.width) / 2.0;
            let ly = y + CELL_SIZE + 1.0;
            ctx.draw_list
                .push_text(Point::new(lx, ly), shaped, ctx.theme.fg_faint);
        }

        if self.animator.is_animating(ctx.now) {
            ctx.request_anim_frame();
        }
    }

    fn on_input(&mut self, event: &crate::input::InputEvent, bounds: Rect) -> super::OnInputResult {
        if let crate::input::InputEvent::MouseDown { pos, .. } = event {
            let local = Point::new(pos.x - bounds.x(), pos.y - bounds.y());
            if let Some(idx) = self.hit_test_cell(local) {
                return super::OnInputResult::handled().with_action(WidgetAction::Selected {
                    id: self.id,
                    index: idx,
                });
            }
        }
        super::OnInputResult::ignored()
    }
}

/// Special color swatch — large swatch with label and hex value.
///
/// Used for foreground, background, cursor, and selection color display.
/// Displays a 28x28 swatch, a label, and the hex color value.
pub struct SpecialColorSwatch {
    id: WidgetId,
    label: String,
    color: Color,
    controllers: Vec<Box<dyn EventController>>,
    animator: VisualStateAnimator,
}

/// Swatch size for special color display.
const SPECIAL_SWATCH_SIZE: f32 = 28.0;

/// Special swatch corner radius.
const SPECIAL_SWATCH_RADIUS: f32 = 6.0;

/// Label font size for special swatch.
const SPECIAL_LABEL_SIZE: f32 = 11.0;

/// Hex value font size.
const HEX_FONT_SIZE: f32 = 10.0;

/// Total width of a special swatch cell.
const SPECIAL_CELL_WIDTH: f32 = 80.0;

/// Total height of a special swatch cell.
const SPECIAL_CELL_HEIGHT: f32 = 56.0;

impl SpecialColorSwatch {
    /// Creates a special swatch with label and color.
    pub fn new(label: impl Into<String>, color: Color, theme: &UiTheme) -> Self {
        Self {
            id: WidgetId::next(),
            label: label.into(),
            color,
            controllers: vec![Box::new(HoverController::new())],
            animator: VisualStateAnimator::new(vec![common_states(
                Color::TRANSPARENT,
                theme.bg_card_hover,
                Color::TRANSPARENT,
                Color::TRANSPARENT,
            )]),
        }
    }

    /// Returns the label.
    pub fn label(&self) -> &str {
        &self.label
    }

    /// Returns the displayed color.
    pub fn color(&self) -> Color {
        self.color
    }

    /// Sets the displayed color.
    pub fn set_color(&mut self, color: Color) {
        self.color = color;
    }

    /// Formats color as hex string.
    fn hex_string(&self) -> String {
        let r = (self.color.r * 255.0) as u8;
        let g = (self.color.g * 255.0) as u8;
        let b = (self.color.b * 255.0) as u8;
        format!("#{r:02X}{g:02X}{b:02X}")
    }
}

impl Widget for SpecialColorSwatch {
    fn id(&self) -> WidgetId {
        self.id
    }

    fn sense(&self) -> Sense {
        Sense::hover()
    }

    fn layout(&self, _ctx: &LayoutCtx<'_>) -> LayoutBox {
        LayoutBox::leaf(SPECIAL_CELL_WIDTH, SPECIAL_CELL_HEIGHT).with_widget_id(self.id)
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
        let bounds = ctx.bounds;

        // Hover background.
        let bg = self.animator.get_bg_color(ctx.now);
        if bg.a > 0.001 {
            let rect_style = RectStyle::filled(bg).with_radius(4.0);
            ctx.draw_list.push_rect(bounds, rect_style);
        }

        // Color swatch (centered horizontally).
        let sx = bounds.x() + (bounds.width() - SPECIAL_SWATCH_SIZE) / 2.0;
        let sy = bounds.y() + 4.0;
        let swatch_rect = Rect::new(sx, sy, SPECIAL_SWATCH_SIZE, SPECIAL_SWATCH_SIZE);
        let swatch_style = RectStyle::filled(self.color).with_radius(SPECIAL_SWATCH_RADIUS);
        ctx.draw_list.push_rect(swatch_rect, swatch_style);

        // Label.
        let label_style = TextStyle::new(SPECIAL_LABEL_SIZE, ctx.theme.fg_primary);
        let shaped = ctx
            .measurer
            .shape(&self.label, &label_style, bounds.width());
        let lx = bounds.x() + (bounds.width() - shaped.width) / 2.0;
        let ly = sy + SPECIAL_SWATCH_SIZE + 2.0;
        ctx.draw_list
            .push_text(Point::new(lx, ly), shaped, ctx.theme.fg_primary);

        // Hex value.
        let hex = self.hex_string();
        let hex_style = TextStyle::new(HEX_FONT_SIZE, ctx.theme.fg_faint);
        let hex_shaped = ctx.measurer.shape(&hex, &hex_style, bounds.width());
        let hx = bounds.x() + (bounds.width() - hex_shaped.width) / 2.0;
        let hy = ly + SPECIAL_LABEL_SIZE + 1.0;
        ctx.draw_list
            .push_text(Point::new(hx, hy), hex_shaped, ctx.theme.fg_faint);

        if self.animator.is_animating(ctx.now) {
            ctx.request_anim_frame();
        }
    }
}

#[cfg(test)]
mod tests;
