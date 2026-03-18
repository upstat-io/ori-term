//! Numeric input widget.
//!
//! A compact text input constrained to numeric values with min/max/step.
//! Arrow keys increment/decrement. Emits `WidgetAction::ValueChanged`
//! when the value changes.

use crate::controllers::{ClickController, EventController, FocusController, HoverController};
use crate::draw::RectStyle;
use crate::geometry::Point;
use crate::input::{InputEvent, Key};
use crate::layout::LayoutBox;
use crate::sense::Sense;
use crate::text::TextStyle;
use crate::theme::UiTheme;
use crate::visual_state::common_states;
use crate::visual_state::transition::VisualStateAnimator;
use crate::widget_id::WidgetId;

use super::{DrawCtx, LayoutCtx, OnInputResult, Widget, WidgetAction};

/// Default widget width.
const INPUT_WIDTH: f32 = 80.0;

/// Widget height.
const INPUT_HEIGHT: f32 = 32.0;

/// Text font size.
const FONT_SIZE: f32 = 13.0;

/// Corner radius.
const CORNER_RADIUS: f32 = 4.0;

/// Border width.
const BORDER_WIDTH: f32 = 1.0;

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
            controllers: vec![
                Box::new(HoverController::new()),
                Box::new(ClickController::new()),
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
        LayoutBox::leaf(INPUT_WIDTH, INPUT_HEIGHT).with_widget_id(self.id)
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

        // Background.
        let bg = self.animator.get_bg_color(ctx.now);
        let border_color = if focused {
            ctx.theme.accent
        } else {
            ctx.theme.border
        };
        let style = RectStyle::filled(bg)
            .with_radius(CORNER_RADIUS)
            .with_border(BORDER_WIDTH, border_color);
        ctx.draw_list.push_rect(bounds, style);

        // Value text, centered.
        let text = self.format_value();
        let text_style = TextStyle::new(FONT_SIZE, ctx.theme.fg_primary);
        let shaped = ctx.measurer.shape(&text, &text_style, bounds.width());
        let tx = bounds.x() + (bounds.width() - shaped.width) / 2.0;
        let ty = bounds.y() + (bounds.height() - shaped.height) / 2.0;
        ctx.draw_list
            .push_text(Point::new(tx, ty), shaped, ctx.theme.fg_primary);

        if self.animator.is_animating(ctx.now) {
            ctx.request_anim_frame();
        }
    }

    fn on_input(&mut self, event: &InputEvent, _bounds: crate::geometry::Rect) -> OnInputResult {
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
            _ => OnInputResult::ignored(),
        }
    }
}

#[cfg(test)]
mod tests;
