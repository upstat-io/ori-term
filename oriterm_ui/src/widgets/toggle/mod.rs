//! Toggle switch widget — a pill-shaped on/off switch.
//!
//! Emits `WidgetAction::Toggled` when clicked (via [`ClickController`]) or
//! activated via Space. Uses [`AnimProperty`] for smooth thumb sliding
//! (150ms, `EaseInOut`) and [`VisualStateAnimator`] for hover color transitions.

use std::time::{Duration, Instant};

use crate::animation::{AnimBehavior, AnimProperty};
use crate::color::Color;
use crate::controllers::{ClickController, EventController, HoverController};
use crate::draw::RectStyle;
use crate::geometry::Rect;
use crate::layout::LayoutBox;
use crate::sense::Sense;
use crate::visual_state::common_states;
use crate::visual_state::transition::VisualStateAnimator;
use crate::widget_id::WidgetId;

use crate::theme::UiTheme;

use super::{DrawCtx, LayoutCtx, Widget, WidgetAction};

/// Duration of the toggle slide animation.
const TOGGLE_DURATION: Duration = Duration::from_millis(150);

/// Visual style for a [`ToggleWidget`].
#[derive(Debug, Clone, PartialEq)]
pub struct ToggleStyle {
    /// Width of the pill track.
    pub width: f32,
    /// Height of the pill track.
    pub height: f32,
    /// Off-state track background.
    pub off_bg: Color,
    /// Off-state hovered track background.
    pub off_hover_bg: Color,
    /// On-state track background.
    pub on_bg: Color,
    /// Thumb color.
    pub thumb_color: Color,
    /// Padding between track edge and thumb.
    pub thumb_padding: f32,
    /// Disabled track background.
    pub disabled_bg: Color,
    /// Disabled thumb color.
    pub disabled_thumb: Color,
    /// Focus ring color.
    pub focus_ring_color: Color,
}

impl ToggleStyle {
    /// Derives a toggle style from the given theme.
    pub fn from_theme(theme: &UiTheme) -> Self {
        Self {
            width: 40.0,
            height: 22.0,
            off_bg: theme.bg_primary,
            off_hover_bg: theme.bg_hover,
            on_bg: theme.accent,
            thumb_color: Color::WHITE,
            thumb_padding: 2.0,
            disabled_bg: theme.bg_secondary,
            disabled_thumb: theme.fg_disabled,
            focus_ring_color: theme.accent,
        }
    }
}

impl Default for ToggleStyle {
    fn default() -> Self {
        Self::from_theme(&UiTheme::dark())
    }
}

/// A pill-shaped toggle switch.
///
/// The thumb slides smoothly between on (1.0) and off (0.0) positions
/// using an [`AnimProperty`] with `EaseInOut` easing over 150ms. Track
/// hover transitions use [`VisualStateAnimator`] with `common_states()`.
pub struct ToggleWidget {
    id: WidgetId,
    on: bool,
    disabled: bool,
    /// Animated thumb position: 0.0 = off, 1.0 = on.
    toggle_progress: AnimProperty<f32>,
    style: ToggleStyle,
    controllers: Vec<Box<dyn EventController>>,
    /// Animator for off-state hover bg transition.
    animator: VisualStateAnimator,
}

impl Default for ToggleWidget {
    fn default() -> Self {
        Self::new()
    }
}

impl ToggleWidget {
    /// Creates a toggle in the off state.
    pub fn new() -> Self {
        let style = ToggleStyle::default();
        Self {
            id: WidgetId::next(),
            on: false,
            disabled: false,
            toggle_progress: AnimProperty::with_behavior(
                0.0,
                AnimBehavior::ease_in_out(TOGGLE_DURATION.as_millis() as u64),
            ),
            controllers: vec![
                Box::new(HoverController::new()),
                Box::new(ClickController::new()),
            ],
            animator: VisualStateAnimator::new(vec![common_states(
                style.off_bg,
                style.off_hover_bg,
                style.off_hover_bg,
                style.disabled_bg,
            )]),
            style,
        }
    }

    /// Returns whether the toggle is on.
    pub fn is_on(&self) -> bool {
        self.on
    }

    /// Sets the on/off state programmatically (no animation).
    pub fn set_on(&mut self, on: bool) {
        self.on = on;
        self.toggle_progress
            .set_immediate(if on { 1.0 } else { 0.0 });
    }

    /// Returns the target animation progress (0.0 or 1.0).
    pub fn toggle_progress(&self) -> f32 {
        self.toggle_progress.target()
    }

    /// Returns whether the toggle is disabled.
    pub fn is_disabled(&self) -> bool {
        self.disabled
    }

    /// Sets the disabled state.
    pub fn set_disabled(&mut self, disabled: bool) {
        self.disabled = disabled;
    }

    /// Sets the style.
    #[must_use]
    pub fn with_style(mut self, style: ToggleStyle) -> Self {
        self.animator = VisualStateAnimator::new(vec![common_states(
            style.off_bg,
            style.off_hover_bg,
            style.off_hover_bg,
            style.disabled_bg,
        )]);
        self.style = style;
        self
    }

    /// Sets initial on state via builder (no animation).
    #[must_use]
    pub fn with_on(mut self, on: bool) -> Self {
        self.on = on;
        self.toggle_progress
            .set_immediate(if on { 1.0 } else { 0.0 });
        self
    }

    /// Sets disabled state via builder.
    #[must_use]
    pub fn with_disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Toggles state with animation and returns the action.
    fn toggle(&mut self) -> WidgetAction {
        self.on = !self.on;
        let target = if self.on { 1.0 } else { 0.0 };
        self.toggle_progress.set(target, Instant::now());
        WidgetAction::Toggled {
            id: self.id,
            value: self.on,
        }
    }

    /// Returns the thumb color based on state.
    fn thumb_color(&self) -> Color {
        if self.disabled {
            self.style.disabled_thumb
        } else {
            self.style.thumb_color
        }
    }
}

impl std::fmt::Debug for ToggleWidget {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ToggleWidget")
            .field("id", &self.id)
            .field("on", &self.on)
            .field("disabled", &self.disabled)
            .field("toggle_progress", &self.toggle_progress)
            .field("style", &self.style)
            .field("controller_count", &self.controllers.len())
            .field("animator", &self.animator)
            .finish()
    }
}

impl Widget for ToggleWidget {
    fn id(&self) -> WidgetId {
        self.id
    }

    fn is_focusable(&self) -> bool {
        !self.disabled
    }

    fn sense(&self) -> Sense {
        Sense::click()
    }

    fn layout(&self, _ctx: &LayoutCtx<'_>) -> LayoutBox {
        // Expand hit area so users don't need pixel-perfect aim on the
        // small 40x22 toggle. The radius extends the clickable zone by
        // 8px in each direction without affecting visual bounds.
        LayoutBox::leaf(self.style.width, self.style.height)
            .with_widget_id(self.id)
            .with_interact_radius(8.0)
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
        let focused = ctx.is_interaction_focused();
        let s = &self.style;
        let radius = s.height / 2.0;

        // Focus ring.
        if focused {
            let ring = ctx.bounds.inset(crate::geometry::Insets::all(-2.0));
            let ring_style = RectStyle::filled(Color::TRANSPARENT)
                .with_border(2.0, s.focus_ring_color)
                .with_radius(radius + 2.0);
            ctx.scene.push_quad(ring, ring_style);
        }

        // Track bg: on_bg when on, animator-driven hover transition when off.
        let track_bg = if self.disabled {
            s.disabled_bg
        } else if self.on {
            s.on_bg
        } else {
            self.animator.get_bg_color(ctx.now)
        };
        let track_style = RectStyle::filled(track_bg).with_radius(radius);
        ctx.scene.push_quad(ctx.bounds, track_style);

        // Thumb — a circle within the track, position driven by animation.
        let progress = self.toggle_progress.get(ctx.now);
        let thumb_diameter = s.height - s.thumb_padding * 2.0;
        let thumb_radius = thumb_diameter / 2.0;
        let travel = s.width - s.thumb_padding * 2.0 - thumb_diameter;
        let thumb_x = ctx.bounds.x() + s.thumb_padding + travel * progress;
        let thumb_y = ctx.bounds.y() + s.thumb_padding;
        let thumb_rect = Rect::new(thumb_x, thumb_y, thumb_diameter, thumb_diameter);
        let thumb_style = RectStyle::filled(self.thumb_color()).with_radius(thumb_radius);
        ctx.scene.push_quad(thumb_rect, thumb_style);

        // Signal continued redraws while animating.
        let animating =
            self.toggle_progress.is_animating(ctx.now) || self.animator.is_animating(ctx.now);
        if animating {
            ctx.request_anim_frame();
        }
    }

    fn on_action(&mut self, action: WidgetAction, _bounds: Rect) -> Option<WidgetAction> {
        match action {
            WidgetAction::Clicked(_) => Some(self.toggle()),
            other => Some(other),
        }
    }

    fn key_context(&self) -> Option<&'static str> {
        Some("Button")
    }

    fn handle_keymap_action(
        &mut self,
        action: &dyn crate::action::KeymapAction,
        _bounds: Rect,
    ) -> Option<WidgetAction> {
        if action.name() == "widget::Activate" {
            Some(WidgetAction::Clicked(self.id))
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests;
