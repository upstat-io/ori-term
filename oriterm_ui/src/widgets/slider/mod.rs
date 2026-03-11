//! Slider widget — a horizontal track with draggable thumb.
//!
//! Emits `WidgetAction::ValueChanged` when the value changes via drag
//! or arrow keys. Supports configurable min/max/step and keyboard
//! adjustment (arrow keys, Home/End).

use crate::color::Color;
use crate::draw::RectStyle;
use crate::geometry::{Point, Rect};
use crate::input::{HoverEvent, Key, KeyEvent, MouseButton, MouseEvent, MouseEventKind};
use crate::layout::{LayoutBox, SizeSpec};
use crate::text::TextStyle;
use crate::widget_id::WidgetId;

use crate::theme::UiTheme;

use super::{DrawCtx, EventCtx, LayoutCtx, Widget, WidgetAction, WidgetResponse};

/// Width reserved for the value label to the right of the track.
const VALUE_LABEL_WIDTH: f32 = 48.0;

/// Gap between track and value label.
const VALUE_GAP: f32 = 12.0;

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
    /// Thumb diameter.
    pub thumb_size: f32,
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
}

impl SliderStyle {
    /// Derives a slider style from the given theme.
    pub fn from_theme(theme: &UiTheme) -> Self {
        Self {
            width: 200.0,
            track_height: 4.0,
            track_bg: theme.bg_primary,
            fill_color: theme.accent,
            track_radius: 2.0,
            thumb_size: 16.0,
            thumb_color: Color::WHITE,
            thumb_hover_color: theme.bg_hover,
            thumb_border_color: theme.border,
            thumb_border_width: 1.0,
            disabled_bg: theme.bg_secondary,
            disabled_fill: theme.fg_disabled,
            focus_ring_color: theme.accent,
        }
    }
}

impl Default for SliderStyle {
    fn default() -> Self {
        Self::from_theme(&UiTheme::dark())
    }
}

/// A horizontal slider with track and draggable thumb.
///
/// Value ranges from `min` to `max` with optional `step` snapping.
/// Arrow keys adjust by `step`, Home/End jump to min/max.
#[derive(Debug, Clone)]
pub struct SliderWidget {
    id: WidgetId,
    value: f32,
    min: f32,
    max: f32,
    step: f32,
    disabled: bool,
    hovered: bool,
    dragging: bool,
    style: SliderStyle,
}

impl Default for SliderWidget {
    fn default() -> Self {
        Self::new()
    }
}

impl SliderWidget {
    /// Creates a slider with value 0.0, range 0.0..1.0, step 0.01.
    pub fn new() -> Self {
        Self {
            id: WidgetId::next(),
            value: 0.0,
            min: 0.0,
            max: 1.0,
            step: 0.01,
            disabled: false,
            hovered: false,
            dragging: false,
            style: SliderStyle::default(),
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

    /// Returns whether the slider is hovered.
    pub fn is_hovered(&self) -> bool {
        self.hovered
    }

    /// Returns whether the thumb is being dragged.
    pub fn is_dragging(&self) -> bool {
        self.dragging
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

    /// Sets the style.
    #[must_use]
    pub fn with_style(mut self, style: SliderStyle) -> Self {
        self.style = style;
        self
    }

    /// Returns the normalized position (0.0..1.0) of the current value.
    fn normalized(&self) -> f32 {
        if (self.max - self.min).abs() < f32::EPSILON {
            return 0.0;
        }
        (self.value - self.min) / (self.max - self.min)
    }

    /// Converts a pixel X position within bounds to a value.
    fn value_from_x(&self, x: f32, bounds: Rect) -> f32 {
        let half_thumb = self.style.thumb_size / 2.0;
        let track_left = bounds.x() + half_thumb;
        let track_width = bounds.width() - self.style.thumb_size;
        if track_width <= 0.0 {
            return self.min;
        }
        let t = ((x - track_left) / track_width).clamp(0.0, 1.0);
        let raw = self.min + t * (self.max - self.min);
        self.snap_to_step(raw)
    }

    /// Snaps a raw value to the nearest step.
    fn snap_to_step(&self, raw: f32) -> f32 {
        if self.step <= 0.0 {
            return raw.clamp(self.min, self.max);
        }
        let steps = ((raw - self.min) / self.step).round();
        (self.min + steps * self.step).clamp(self.min, self.max)
    }

    /// Returns the track area (excluding value label space) within the given bounds.
    fn track_bounds(&self, bounds: Rect) -> Rect {
        let label_space = VALUE_LABEL_WIDTH + VALUE_GAP;
        let w = (bounds.width() - label_space).max(self.style.thumb_size);
        Rect::new(bounds.x(), bounds.y(), w, bounds.height())
    }

    /// Formats the current value for display based on step precision.
    fn format_value(&self) -> String {
        if self.step >= 1.0 {
            format!("{:.0}", self.value)
        } else if self.step >= 0.1 {
            format!("{:.1}", self.value)
        } else {
            format!("{:.2}", self.value)
        }
    }

    /// Sets value and returns action if it changed.
    fn set_value_action(&mut self, new_value: f32) -> Option<WidgetAction> {
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

impl Widget for SliderWidget {
    fn id(&self) -> WidgetId {
        self.id
    }

    fn is_focusable(&self) -> bool {
        !self.disabled
    }

    fn layout(&self, _ctx: &LayoutCtx<'_>) -> LayoutBox {
        let height = self.style.thumb_size.max(self.style.track_height);
        LayoutBox::leaf(self.style.width, height)
            .with_width(SizeSpec::Fill)
            .with_widget_id(self.id)
    }

    fn draw(&self, ctx: &mut DrawCtx<'_>) {
        let focused = ctx.focused_widget == Some(self.id);
        let s = &self.style;
        let tb = self.track_bounds(ctx.bounds);

        // Focus ring around track area.
        if focused {
            let ring = tb.inset(crate::geometry::Insets::all(-2.0));
            let ring_style = RectStyle::filled(Color::TRANSPARENT)
                .with_border(2.0, s.focus_ring_color)
                .with_radius(s.track_radius + 2.0);
            ctx.draw_list.push_rect(ring, ring_style);
        }

        // Track background.
        let track_y = tb.y() + (tb.height() - s.track_height) / 2.0;
        let track_rect = Rect::new(tb.x(), track_y, tb.width(), s.track_height);
        let bg_color = if self.disabled {
            s.disabled_bg
        } else {
            s.track_bg
        };
        let track_style = RectStyle::filled(bg_color).with_radius(s.track_radius);
        ctx.draw_list.push_rect(track_rect, track_style);

        // Filled portion (left of thumb).
        let norm = self.normalized();
        let fill_width = norm * tb.width();
        if fill_width > 0.0 {
            let fill_rect = Rect::new(tb.x(), track_y, fill_width, s.track_height);
            let fill_color = if self.disabled {
                s.disabled_fill
            } else {
                s.fill_color
            };
            let fill_style = RectStyle::filled(fill_color).with_radius(s.track_radius);
            ctx.draw_list.push_rect(fill_rect, fill_style);
        }

        // Thumb.
        let half_thumb = s.thumb_size / 2.0;
        let travel = tb.width() - s.thumb_size;
        let thumb_x = tb.x() + travel * norm;
        let thumb_y = tb.y() + (tb.height() - s.thumb_size) / 2.0;
        let thumb_rect = Rect::new(thumb_x, thumb_y, s.thumb_size, s.thumb_size);
        let thumb_bg = if self.disabled {
            s.disabled_bg
        } else if self.hovered || self.dragging {
            s.thumb_hover_color
        } else {
            s.thumb_color
        };
        let thumb_style = RectStyle::filled(thumb_bg)
            .with_border(s.thumb_border_width, s.thumb_border_color)
            .with_radius(half_thumb);
        ctx.draw_list.push_rect(thumb_rect, thumb_style);

        // Value label to the right of the track.
        let value_text = self.format_value();
        let text_style = TextStyle::new(ctx.theme.font_size, ctx.theme.fg_secondary);
        let shaped = ctx
            .measurer
            .shape(&value_text, &text_style, VALUE_LABEL_WIDTH);
        let label_x = tb.right() + VALUE_GAP;
        // Right-align within the label area.
        let text_x = label_x + VALUE_LABEL_WIDTH - shaped.width;
        let text_y = ctx.bounds.y() + (ctx.bounds.height() - shaped.height) / 2.0;
        ctx.draw_list
            .push_text(Point::new(text_x, text_y), shaped, ctx.theme.fg_secondary);
    }

    fn handle_mouse(&mut self, event: &MouseEvent, ctx: &EventCtx<'_>) -> WidgetResponse {
        if self.disabled {
            return WidgetResponse::ignored();
        }
        let tb = self.track_bounds(ctx.bounds);
        match event.kind {
            MouseEventKind::Down(MouseButton::Left) => {
                self.dragging = true;
                let new_val = self.value_from_x(event.pos.x, tb);
                let action = self.set_value_action(new_val);
                let mut r = WidgetResponse::focus().with_capture();
                if let Some(a) = action {
                    r = r.with_action(a);
                }
                r
            }
            MouseEventKind::Up(MouseButton::Left) => {
                self.dragging = false;
                WidgetResponse::paint().with_release_capture()
            }
            MouseEventKind::Move if self.dragging => {
                let new_val = self.value_from_x(event.pos.x, tb);
                let action = self.set_value_action(new_val);
                let mut r = WidgetResponse::paint();
                if let Some(a) = action {
                    r = r.with_action(a);
                }
                r
            }
            _ => WidgetResponse::ignored(),
        }
    }

    fn handle_hover(&mut self, event: HoverEvent, _ctx: &EventCtx<'_>) -> WidgetResponse {
        if self.disabled {
            return WidgetResponse::ignored();
        }
        match event {
            HoverEvent::Enter => {
                self.hovered = true;
                WidgetResponse::paint()
            }
            HoverEvent::Leave => {
                self.hovered = false;
                WidgetResponse::paint()
            }
        }
    }

    fn handle_key(&mut self, event: KeyEvent, ctx: &EventCtx<'_>) -> WidgetResponse {
        if self.disabled || !ctx.is_focused {
            return WidgetResponse::ignored();
        }
        let delta = match event.key {
            Key::ArrowRight | Key::ArrowUp => self.step,
            Key::ArrowLeft | Key::ArrowDown => -self.step,
            Key::Home => {
                let action = self.set_value_action(self.min);
                let mut r = WidgetResponse::paint();
                if let Some(a) = action {
                    r = r.with_action(a);
                }
                return r;
            }
            Key::End => {
                let action = self.set_value_action(self.max);
                let mut r = WidgetResponse::paint();
                if let Some(a) = action {
                    r = r.with_action(a);
                }
                return r;
            }
            _ => return WidgetResponse::ignored(),
        };
        let new_val = self.snap_to_step(self.value + delta);
        let action = self.set_value_action(new_val);
        let mut r = WidgetResponse::paint();
        if let Some(a) = action {
            r = r.with_action(a);
        }
        r
    }
}

#[cfg(test)]
mod tests;
