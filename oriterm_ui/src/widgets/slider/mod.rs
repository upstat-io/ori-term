//! Slider widget — a horizontal track with draggable thumb.
//!
//! Emits `WidgetAction::ValueChanged` when the value changes via drag
//! or arrow keys. Supports configurable min/max/step and keyboard
//! adjustment (arrow keys, Home/End). Uses [`VisualStateAnimator`]
//! with `common_states()` for smooth thumb color transitions.

mod widget_impl;

use crate::color::Color;
use crate::controllers::{EventController, HoverController, ScrubController};
use crate::geometry::{Point, Rect};
use crate::visual_state::common_states;
use crate::visual_state::transition::VisualStateAnimator;
use crate::widget_id::WidgetId;

use crate::theme::UiTheme;

use super::WidgetAction;

/// Width reserved for the value label to the right of the track.
const VALUE_LABEL_WIDTH: f32 = 32.0;

/// Gap between track and value label.
const VALUE_GAP: f32 = 10.0;

/// Visual style for a [`SliderWidget`].
#[derive(Debug, Clone, PartialEq)]
pub struct SliderStyle {
    /// Total width of the slider (track + thumb).
    pub width: f32,
    /// Track height.
    pub track_height: f32,
    /// Track background color.
    pub track_bg: Color,
    /// Filled portion color (left of thumb).
    pub fill_color: Color,
    /// Track corner radius.
    pub track_radius: f32,
    /// Thumb width.
    pub thumb_width: f32,
    /// Thumb height.
    pub thumb_height: f32,
    /// Thumb color.
    pub thumb_color: Color,
    /// Thumb color when hovered.
    pub thumb_hover_color: Color,
    /// Thumb border color.
    pub thumb_border_color: Color,
    /// Thumb border width.
    pub thumb_border_width: f32,
    /// Disabled track/thumb color.
    pub disabled_bg: Color,
    /// Disabled fill color.
    pub disabled_fill: Color,
    /// Focus ring color.
    pub focus_ring_color: Color,
    /// Font size for the value label.
    pub value_font_size: f32,
}

impl SliderStyle {
    /// Derives a slider style from the given theme.
    pub fn from_theme(theme: &UiTheme) -> Self {
        Self {
            width: 120.0,
            track_height: 4.0,
            track_bg: theme.border,
            fill_color: theme.border,
            track_radius: theme.corner_radius,
            thumb_width: 12.0,
            thumb_height: 14.0,
            thumb_color: theme.accent,
            thumb_hover_color: theme.accent_hover,
            thumb_border_color: theme.bg_primary,
            thumb_border_width: 2.0,
            disabled_bg: theme.bg_secondary,
            disabled_fill: theme.fg_disabled,
            focus_ring_color: theme.accent,
            value_font_size: 12.0,
        }
    }
}

impl Default for SliderStyle {
    fn default() -> Self {
        Self::from_theme(&UiTheme::dark())
    }
}

/// How the slider value is formatted in the label area.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValueDisplay {
    /// No value label shown.
    Hidden,
    /// Raw numeric value (e.g., "14", "0.5").
    Numeric,
    /// Value followed by "%" (e.g., "100%").
    Percent,
    /// Value followed by a custom suffix (e.g., "14px").
    Suffix(&'static str),
}

/// A horizontal slider with track and draggable thumb.
///
/// Value ranges from `min` to `max` with optional `step` snapping.
/// Arrow keys adjust by `step`, Home/End jump to min/max. Thumb
/// hover transitions use [`VisualStateAnimator`] with `common_states()`.
pub struct SliderWidget {
    pub(super) id: WidgetId,
    pub(super) value: f32,
    pub(super) min: f32,
    pub(super) max: f32,
    pub(super) step: f32,
    pub(super) disabled: bool,
    pub(super) display: ValueDisplay,
    pub(super) style: SliderStyle,
    pub(super) controllers: Vec<Box<dyn EventController>>,
    /// Animator for thumb color transition.
    pub(super) animator: VisualStateAnimator,
    /// Position at scrub start, for computing value from `total_delta`.
    pub(super) drag_origin: Option<Point>,
    /// Bounds snapshot at drag start. During capture, the dispatch system
    /// may pass stale or fallback bounds from a different widget if the
    /// mouse leaves the slider. Caching at drag start avoids jumps.
    pub(super) drag_bounds: Option<Rect>,
}

impl Default for SliderWidget {
    fn default() -> Self {
        Self::new()
    }
}

impl SliderWidget {
    /// Creates a slider with value 0.0, range 0.0..1.0, step 0.01.
    pub fn new() -> Self {
        let style = SliderStyle::default();
        Self {
            id: WidgetId::next(),
            value: 0.0,
            min: 0.0,
            max: 1.0,
            step: 0.01,
            disabled: false,
            display: ValueDisplay::Numeric,
            controllers: vec![
                Box::new(HoverController::new()),
                Box::new(ScrubController::new()),
            ],
            animator: VisualStateAnimator::new(vec![common_states(
                style.thumb_color,
                style.thumb_hover_color,
                style.thumb_hover_color,
                style.disabled_bg,
            )]),
            style,
            drag_origin: None,
            drag_bounds: None,
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

    /// Returns the minimum value.
    pub fn min(&self) -> f32 {
        self.min
    }

    /// Returns the maximum value.
    pub fn max(&self) -> f32 {
        self.max
    }

    /// Returns whether the slider is disabled.
    pub fn is_disabled(&self) -> bool {
        self.disabled
    }

    /// Sets the range.
    #[must_use]
    pub fn with_range(mut self, min: f32, max: f32) -> Self {
        self.min = min;
        self.max = max;
        self.value = self.value.clamp(min, max);
        self
    }

    /// Sets the step increment.
    #[must_use]
    pub fn with_step(mut self, step: f32) -> Self {
        self.step = step;
        self
    }

    /// Sets the initial value.
    #[must_use]
    pub fn with_value(mut self, value: f32) -> Self {
        self.value = value.clamp(self.min, self.max);
        self
    }

    /// Sets the disabled state.
    #[must_use]
    pub fn with_disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Sets how the value label is formatted.
    #[must_use]
    pub fn with_display(mut self, display: ValueDisplay) -> Self {
        self.display = display;
        self
    }

    /// Sets the style.
    #[must_use]
    pub fn with_style(mut self, style: SliderStyle) -> Self {
        self.animator = VisualStateAnimator::new(vec![common_states(
            style.thumb_color,
            style.thumb_hover_color,
            style.thumb_hover_color,
            style.disabled_bg,
        )]);
        self.style = style;
        self
    }

    /// Returns the normalized position (0.0..1.0) of the current value.
    pub(super) fn normalized(&self) -> f32 {
        if (self.max - self.min).abs() < f32::EPSILON {
            return 0.0;
        }
        (self.value - self.min) / (self.max - self.min)
    }

    /// Converts a pixel X position within bounds to a value.
    pub(super) fn value_from_x(&self, x: f32, bounds: Rect) -> f32 {
        let half_thumb = self.style.thumb_width / 2.0;
        let track_left = bounds.x() + half_thumb;
        let track_width = bounds.width() - self.style.thumb_width;
        if track_width <= 0.0 {
            return self.min;
        }
        let t = ((x - track_left) / track_width).clamp(0.0, 1.0);
        let raw = self.min + t * (self.max - self.min);
        self.snap_to_step(raw)
    }

    /// Snaps a raw value to the nearest step.
    pub(super) fn snap_to_step(&self, raw: f32) -> f32 {
        if self.step <= 0.0 {
            return raw.clamp(self.min, self.max);
        }
        let steps = ((raw - self.min) / self.step).round();
        (self.min + steps * self.step).clamp(self.min, self.max)
    }

    /// Returns the track area (excluding value label space) within the given bounds.
    pub(super) fn track_bounds(&self, bounds: Rect) -> Rect {
        let label_space = VALUE_LABEL_WIDTH + VALUE_GAP;
        let w = (bounds.width() - label_space).max(self.style.thumb_width);
        Rect::new(bounds.x(), bounds.y(), w, bounds.height())
    }

    /// Formats the current value for display based on step precision.
    pub(super) fn format_value(&self) -> String {
        let num = if self.step >= 1.0 {
            format!("{:.0}", self.value)
        } else if self.step >= 0.1 {
            format!("{:.1}", self.value)
        } else {
            format!("{:.2}", self.value)
        };
        match &self.display {
            ValueDisplay::Hidden => String::new(),
            ValueDisplay::Numeric => num,
            ValueDisplay::Percent => format!("{num}%"),
            ValueDisplay::Suffix(s) => format!("{num}{s}"),
        }
    }

    /// Sets value and returns action if it changed.
    pub(super) fn set_value_action(&mut self, new_value: f32) -> Option<WidgetAction> {
        let clamped = new_value.clamp(self.min, self.max);
        if (clamped - self.value).abs() < f32::EPSILON {
            return None;
        }
        self.value = clamped;
        Some(WidgetAction::ValueChanged {
            id: self.id,
            value: self.value,
        })
    }
}

impl std::fmt::Debug for SliderWidget {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SliderWidget")
            .field("id", &self.id)
            .field("value", &self.value)
            .field("min", &self.min)
            .field("max", &self.max)
            .field("step", &self.step)
            .field("disabled", &self.disabled)
            .field("display", &self.display)
            .field("style", &self.style)
            .field("controller_count", &self.controllers.len())
            .field("animator", &self.animator)
            .field("drag_origin", &self.drag_origin)
            .field("drag_bounds", &self.drag_bounds)
            .finish()
    }
}

#[cfg(test)]
mod tests;
