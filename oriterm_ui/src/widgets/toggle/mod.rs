//! Toggle switch widget — a pill-shaped on/off switch.
//!
//! Emits `WidgetAction::Toggled` when clicked or dragged (via
//! [`ScrubController`]) or activated via Space. Uses [`AnimProperty`]
//! for smooth thumb sliding (150ms, `EaseInOut`) and
//! [`VisualStateAnimator`] for hover color transitions.

use std::time::Duration;

use crate::animation::{AnimBehavior, AnimProperty};
use crate::color::Color;
use crate::controllers::{EventController, HoverController, ScrubController};
use crate::draw::RectStyle;
use crate::geometry::{Point, Rect};
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
    /// Width of the track.
    pub width: f32,
    /// Height of the track.
    pub height: f32,
    /// Off-state track background.
    pub off_bg: Color,
    /// Off-state hovered track background.
    pub off_hover_bg: Color,
    /// On-state track background.
    pub on_bg: Color,
    /// Off-state thumb color.
    pub off_thumb_color: Color,
    /// On-state thumb color.
    pub on_thumb_color: Color,
    /// Padding between track edge and thumb.
    pub thumb_padding: f32,
    /// Thumb width and height (square). Decoupled from track height.
    pub thumb_size: f32,
    /// Track border width.
    pub border_width: f32,
    /// Off-state track border color.
    pub off_border_color: Color,
    /// On-state track border color.
    pub on_border_color: Color,
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
            width: 38.0,
            height: 20.0,
            off_bg: theme.bg_active,
            off_hover_bg: theme.bg_hover,
            on_bg: theme.accent_bg_strong,
            off_thumb_color: theme.fg_faint,
            on_thumb_color: theme.accent,
            thumb_padding: 3.0,
            thumb_size: 12.0,
            border_width: 2.0,
            off_border_color: theme.border,
            on_border_color: theme.accent,
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
///
/// Supports both click (tap to toggle) and drag (scrub the thumb)
/// interactions via [`ScrubController`].
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
    /// Press position at drag start, for click-vs-drag discrimination.
    drag_origin: Option<Point>,
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
                Box::new(ScrubController::new()),
            ],
            animator: VisualStateAnimator::new(vec![common_states(
                style.off_bg,
                style.off_hover_bg,
                style.off_hover_bg,
                style.disabled_bg,
            )]),
            style,
            drag_origin: None,
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
        self.toggle_progress.set(target);
        WidgetAction::Toggled {
            id: self.id,
            value: self.on,
        }
    }

    /// Returns the thumb color based on on/off and disabled state.
    fn thumb_color(&self) -> Color {
        if self.disabled {
            self.style.disabled_thumb
        } else if self.on {
            self.style.on_thumb_color
        } else {
            self.style.off_thumb_color
        }
    }

    /// Converts a pixel X position to a progress value (0.0–1.0).
    fn progress_from_x(&self, x: f32, bounds: Rect) -> f32 {
        let s = &self.style;
        let travel = s.width - s.thumb_padding * 2.0 - s.thumb_size;
        let left = bounds.x() + s.thumb_padding + s.thumb_size / 2.0;
        if travel <= 0.0 {
            return 0.0;
        }
        ((x - left) / travel).clamp(0.0, 1.0)
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
            .field("drag_origin", &self.drag_origin)
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
        Sense::click_and_drag()
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

    fn prepaint(&mut self, _ctx: &mut crate::widgets::PrepaintCtx<'_>) {
        self.toggle_progress.tick();
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

        // Focus ring — rectangular, no radius.
        if focused {
            let ring = ctx.bounds.inset(crate::geometry::Insets::all(-2.0));
            let ring_style =
                RectStyle::filled(Color::TRANSPARENT).with_border(2.0, s.focus_ring_color);
            ctx.scene.push_quad(ring, ring_style);
        }

        // Track bg + border: on/off state dependent.
        let track_bg = if self.disabled {
            s.disabled_bg
        } else if self.on {
            s.on_bg
        } else {
            self.animator.get_bg_color()
        };
        let border_color = if self.on {
            s.on_border_color
        } else {
            s.off_border_color
        };
        let track_style = RectStyle::filled(track_bg).with_border(s.border_width, border_color);
        ctx.scene.push_quad(ctx.bounds, track_style);

        // Thumb — square, position driven by animation.
        let progress = self.toggle_progress.get();
        let travel = s.width - s.thumb_padding * 2.0 - s.thumb_size;
        let thumb_x = ctx.bounds.x() + s.thumb_padding + travel * progress;
        let thumb_y = ctx.bounds.y() + s.thumb_padding;
        let thumb_rect = Rect::new(thumb_x, thumb_y, s.thumb_size, s.thumb_size);
        let thumb_style = RectStyle::filled(self.thumb_color());
        ctx.scene.push_quad(thumb_rect, thumb_style);

        // Signal continued redraws while animating.
        let animating = self.toggle_progress.is_animating() || self.animator.is_animating();
        if animating {
            ctx.request_anim_frame();
        }
    }

    fn on_action(&mut self, action: WidgetAction, bounds: Rect) -> Option<WidgetAction> {
        match action {
            WidgetAction::DragStart { pos, .. } => {
                self.drag_origin = Some(pos);
                None
            }
            WidgetAction::DragUpdate { total_delta, .. } => {
                if let Some(origin) = self.drag_origin {
                    let x = origin.x + total_delta.x;
                    let progress = self.progress_from_x(x, bounds);
                    self.toggle_progress.set_immediate(progress);
                }
                None
            }
            WidgetAction::DragEnd { pos, .. } => {
                if let Some(origin) = self.drag_origin.take() {
                    // Half the thumb travel distance: anything below is a
                    // click/tap, anything above is a positional drag commit.
                    let s = &self.style;
                    let travel = s.width - s.thumb_padding * 2.0 - s.thumb_size;
                    let total_move = (pos.x - origin.x).abs();

                    if total_move < travel / 2.0 {
                        // Click or insignificant drag — toggle.
                        Some(self.toggle())
                    } else {
                        // Intentional drag — commit based on final position.
                        let progress = self.progress_from_x(pos.x, bounds);
                        let new_on = progress > 0.5;
                        let target = if new_on { 1.0 } else { 0.0 };
                        self.toggle_progress.set(target);
                        if new_on == self.on {
                            // Dragged but ended in same state — snap back.
                            None
                        } else {
                            self.on = new_on;
                            Some(WidgetAction::Toggled {
                                id: self.id,
                                value: self.on,
                            })
                        }
                    }
                } else {
                    None
                }
            }
            // Keyboard activation (Space) returns Toggled directly.
            WidgetAction::Toggled { .. } => Some(action),
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
            Some(self.toggle())
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests;
