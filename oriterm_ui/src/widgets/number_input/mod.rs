//! Numeric input widget.
//!
//! A compact text input constrained to numeric values with min/max/step.
//! Arrow keys increment/decrement. Emits `WidgetAction::ValueChanged`
//! when the value changes.

use crate::color::Color;
use crate::controllers::{EventController, FocusController, HoverController};
use crate::draw::RectStyle;
use crate::geometry::{Point, Rect};
use crate::input::{InputEvent, Key};
use crate::layout::LayoutBox;
use crate::sense::Sense;
use crate::text::TextStyle;
use crate::theme::UiTheme;
use crate::visual_state::common_states;
use crate::visual_state::transition::VisualStateAnimator;
use crate::widget_id::WidgetId;

use super::{DrawCtx, LayoutCtx, OnInputResult, Widget, WidgetAction};

/// Default inner input width (mockup `.num-stepper input { width: 56px }`).
pub(super) const DEFAULT_INPUT_WIDTH: f32 = 56.0;

/// Widget height (mockup `.num-stepper { height: 30px }`).
pub(super) const INPUT_HEIGHT: f32 = 30.0;

/// Text font size (mockup `.num-stepper input { font-size: 12px }`).
const FONT_SIZE: f32 = 12.0;

/// Corner radius.
const CORNER_RADIUS: f32 = 0.0;

/// Outer border width (mockup `.num-stepper { border: 2px solid }`).
pub(super) const BORDER_WIDTH: f32 = 2.0;

/// Horizontal divider between stepper buttons (mockup `border-top: 1px`).
pub(super) const BUTTON_DIVIDER_WIDTH: f32 = 1.0;

/// Width of the right-side button panel (up/down arrows).
pub(super) const BUTTON_PANEL_WIDTH: f32 = 22.0;

/// A compact numeric input with min/max/step constraints.
///
/// Displays the current value centered. Arrow Up/Down increment/decrement
/// by `step`. Emits `WidgetAction::ValueChanged` when the value changes.
pub struct NumberInputWidget {
    id: WidgetId,
    value: f32,
    min: f32,
    max: f32,
    step: f32,
    /// Inner text field width (total = `input_width` + `BUTTON_PANEL_WIDTH` + 2 * `BORDER_WIDTH`).
    input_width: f32,
    /// Border color when hovered (mockup `border-color: var(--text-faint)`).
    hover_border_color: Color,
    /// Border color when focused (mockup `border-color: var(--accent)`).
    focus_border_color: Color,
    controllers: Vec<Box<dyn EventController>>,
    animator: VisualStateAnimator,
}

impl NumberInputWidget {
    /// Creates a number input with the given range and step.
    pub fn new(value: f32, min: f32, max: f32, step: f32, theme: &UiTheme) -> Self {
        Self {
            id: WidgetId::next(),
            value: value.clamp(min, max),
            min,
            max,
            step,
            input_width: DEFAULT_INPUT_WIDTH,
            hover_border_color: theme.fg_faint,
            focus_border_color: theme.accent,
            controllers: vec![
                Box::new(HoverController::new()),
                Box::new(FocusController::new()),
            ],
            animator: VisualStateAnimator::new(vec![common_states(
                theme.bg_input,
                theme.bg_card_hover,
                theme.bg_active,
                theme.bg_secondary,
            )]),
        }
    }

    /// Overrides the inner input field width (default 56px, compact 44px).
    #[must_use]
    pub fn with_input_width(mut self, px: f32) -> Self {
        self.input_width = px;
        self
    }

    /// Returns the current value.
    pub fn value(&self) -> f32 {
        self.value
    }

    /// Sets the value, clamping to [min, max].
    pub fn set_value(&mut self, value: f32) {
        self.value = value.clamp(self.min, self.max);
    }

    /// Returns the minimum.
    pub fn min(&self) -> f32 {
        self.min
    }

    /// Returns the maximum.
    pub fn max(&self) -> f32 {
        self.max
    }

    /// Formats the value for display based on step precision.
    fn format_value(&self) -> String {
        if self.step >= 1.0 {
            format!("{:.0}", self.value)
        } else if self.step >= 0.1 {
            format!("{:.1}", self.value)
        } else {
            format!("{:.2}", self.value)
        }
    }

    /// Adjusts value by the given number of steps. Returns an action if changed.
    fn adjust(&mut self, steps: i32) -> Option<WidgetAction> {
        let new_val = (self.value + steps as f32 * self.step).clamp(self.min, self.max);
        if (new_val - self.value).abs() < f32::EPSILON {
            return None;
        }
        self.value = new_val;
        Some(WidgetAction::ValueChanged {
            id: self.id,
            value: self.value,
        })
    }
}

impl Widget for NumberInputWidget {
    fn id(&self) -> WidgetId {
        self.id
    }

    fn is_focusable(&self) -> bool {
        true
    }

    fn sense(&self) -> Sense {
        Sense::click().union(Sense::focusable())
    }

    fn layout(&self, _ctx: &LayoutCtx<'_>) -> LayoutBox {
        let total_w = self.input_width + BUTTON_PANEL_WIDTH + 2.0 * BORDER_WIDTH;
        LayoutBox::leaf(total_w, INPUT_HEIGHT).with_widget_id(self.id)
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
        let focused = ctx.is_interaction_focused();
        let hovered = ctx.is_hot();

        // Border color: focus > hover > default (matching dropdown's pattern).
        let border_color = if focused {
            self.focus_border_color
        } else if hovered {
            self.hover_border_color
        } else {
            ctx.theme.border
        };

        // Background.
        let bg = self.animator.get_bg_color();
        let style = RectStyle::filled(bg)
            .with_radius(CORNER_RADIUS)
            .with_border(BORDER_WIDTH, border_color);
        ctx.scene.push_quad(bounds, style);

        // Value text, centered in the text area (left of button panel).
        let text_area_w = bounds.width() - BUTTON_PANEL_WIDTH;
        let text = self.format_value();
        let text_style = TextStyle::new(FONT_SIZE, ctx.theme.fg_primary);
        let shaped = ctx.measurer.shape(&text, &text_style, text_area_w);
        let tx = bounds.x() + (text_area_w - shaped.width) / 2.0;
        let ty = bounds.y() + (bounds.height() - shaped.height) / 2.0;
        ctx.scene
            .push_text(Point::new(tx, ty), shaped, ctx.theme.fg_primary);

        // Button panel: vertical divider + up/down arrows.
        let panel_x = bounds.x() + text_area_w;
        let half_h = bounds.height() / 2.0;

        // Vertical divider (mockup `border-left: 2px solid`).
        let divider = Rect::new(panel_x, bounds.y(), BORDER_WIDTH, bounds.height());
        ctx.scene
            .push_quad(divider, RectStyle::filled(border_color));

        // Horizontal divider between stepper buttons (mockup `border-top: 1px`).
        let h_divider = Rect::new(
            panel_x + BORDER_WIDTH,
            bounds.y() + half_h - BUTTON_DIVIDER_WIDTH / 2.0,
            BUTTON_PANEL_WIDTH - BORDER_WIDTH,
            BUTTON_DIVIDER_WIDTH,
        );
        ctx.scene
            .push_quad(h_divider, RectStyle::filled(ctx.theme.border));

        // Stepper arrows via icon pipeline (avoids missing Unicode glyphs).
        let arrow_size: f32 = 8.0;
        let arrow_color = ctx.theme.fg_faint;
        let arrow_area_x = panel_x + BORDER_WIDTH;
        let arrow_area_w = BUTTON_PANEL_WIDTH - BORDER_WIDTH;
        let icon_px = arrow_size.round() as u32;

        // Up arrow — centered in top half of button panel.
        if let Some(up) = ctx
            .icons
            .and_then(|ic| ic.get(crate::icons::IconId::StepperUp, icon_px))
        {
            let up_x = arrow_area_x + (arrow_area_w - arrow_size) / 2.0;
            let up_y = bounds.y() + (half_h - arrow_size) / 2.0;
            let up_rect = Rect::new(up_x, up_y, arrow_size, arrow_size);
            ctx.scene
                .push_icon(up_rect, up.atlas_page, up.uv, arrow_color);
        }

        // Down arrow — centered in bottom half of button panel.
        if let Some(dn) = ctx
            .icons
            .and_then(|ic| ic.get(crate::icons::IconId::StepperDown, icon_px))
        {
            let dn_x = arrow_area_x + (arrow_area_w - arrow_size) / 2.0;
            let dn_y = bounds.y() + half_h + (half_h - arrow_size) / 2.0;
            let dn_rect = Rect::new(dn_x, dn_y, arrow_size, arrow_size);
            ctx.scene
                .push_icon(dn_rect, dn.atlas_page, dn.uv, arrow_color);
        }

        if self.animator.is_animating() {
            ctx.request_anim_frame();
        }
    }

    fn on_input(&mut self, event: &InputEvent, bounds: Rect) -> OnInputResult {
        match event {
            InputEvent::KeyDown {
                key: Key::ArrowUp, ..
            } => {
                if let Some(action) = self.adjust(1) {
                    return OnInputResult::handled().with_action(action);
                }
                OnInputResult::handled()
            }
            InputEvent::KeyDown {
                key: Key::ArrowDown,
                ..
            } => {
                if let Some(action) = self.adjust(-1) {
                    return OnInputResult::handled().with_action(action);
                }
                OnInputResult::handled()
            }
            InputEvent::MouseDown { pos, .. } => {
                // Check if click is in the button panel area.
                let panel_x = bounds.x() + bounds.width() - BUTTON_PANEL_WIDTH;
                if pos.x >= panel_x {
                    let mid_y = bounds.y() + bounds.height() / 2.0;
                    let steps = if pos.y < mid_y { 1 } else { -1 };
                    if let Some(action) = self.adjust(steps) {
                        return OnInputResult::handled().with_action(action);
                    }
                    return OnInputResult::handled();
                }
                OnInputResult::ignored()
            }
            _ => OnInputResult::ignored(),
        }
    }
}

#[cfg(test)]
mod tests;
