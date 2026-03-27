//! Button widget with hover, pressed, and disabled visual states.
//!
//! Emits `WidgetAction::Clicked` on mouse click (via [`ClickController`]) or
//! keyboard activation (Enter/Space via keymap `Activate` action when focused).
//! Uses [`VisualStateAnimator`] with `common_states()` for smooth state color
//! transitions.

use crate::action::WidgetAction;
use crate::color::Color;
use crate::controllers::{ClickController, EventController, FocusController, HoverController};
use crate::draw::RectStyle;
use crate::geometry::{Insets, Point, Rect};
use crate::layout::LayoutBox;
use crate::sense::Sense;
use crate::text::{FontWeight, TextStyle, TextTransform};
use crate::visual_state::common_states;
use crate::visual_state::transition::VisualStateAnimator;
use crate::widget_id::WidgetId;

use crate::theme::UiTheme;

use super::{DrawCtx, LayoutCtx, PrepaintCtx, Widget};

/// Visual style for a [`ButtonWidget`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ButtonStyle {
    /// Text color.
    pub fg: Color,
    /// Text color on hover (defaults to same as `fg`).
    pub hover_fg: Color,
    /// Background color (normal state).
    pub bg: Color,
    /// Background when hovered.
    pub hover_bg: Color,
    /// Background when pressed.
    pub pressed_bg: Color,
    /// Border color.
    pub border_color: Color,
    /// Border color on hover (defaults to same as `border_color`).
    pub hover_border_color: Color,
    /// Border width.
    pub border_width: f32,
    /// Corner radius.
    pub corner_radius: f32,
    /// Inner padding.
    pub padding: Insets,
    /// Font size in logical pixels.
    pub font_size: f32,
    /// Font weight (CSS 100–900).
    pub weight: FontWeight,
    /// Inter-glyph letter spacing in logical pixels.
    pub letter_spacing: f32,
    /// Text transform (uppercase, lowercase, capitalize, none).
    pub text_transform: TextTransform,
    /// Disabled text color (used when `disabled_opacity == 1.0`).
    pub disabled_fg: Color,
    /// Disabled background color (used when `disabled_opacity == 1.0`).
    pub disabled_bg: Color,
    /// Opacity applied to the entire button when disabled.
    ///
    /// When `< 1.0`, modulates bg/border/fg alpha uniformly (CSS `opacity`
    /// semantics). When `1.0` (default), uses `disabled_fg`/`disabled_bg`
    /// color swap instead, for backward compatibility.
    pub disabled_opacity: f32,
    /// Focus ring color.
    pub focus_ring_color: Color,
}

impl ButtonStyle {
    /// Derives a button style from the given theme.
    pub fn from_theme(theme: &UiTheme) -> Self {
        Self {
            fg: theme.fg_primary,
            hover_fg: theme.fg_primary,
            bg: theme.bg_primary,
            hover_bg: theme.bg_hover,
            pressed_bg: theme.bg_active,
            border_color: theme.border,
            hover_border_color: theme.border,
            border_width: 1.0,
            corner_radius: theme.corner_radius,
            padding: Insets::vh(6.0, 12.0),
            font_size: theme.font_size,
            weight: FontWeight::NORMAL,
            letter_spacing: 0.0,
            text_transform: TextTransform::None,
            disabled_fg: theme.fg_disabled,
            disabled_bg: theme.bg_secondary,
            disabled_opacity: 1.0,
            focus_ring_color: theme.accent,
        }
    }
}

impl Default for ButtonStyle {
    fn default() -> Self {
        Self::from_theme(&UiTheme::dark())
    }
}

/// Interactive button widget.
///
/// Emits `WidgetAction::Clicked(id)` when clicked (via [`ClickController`])
/// or keyboard-activated (Enter/Space). Hover, pressed, and disabled visual
/// state transitions are handled by [`VisualStateAnimator`] with `common_states()`.
pub struct ButtonWidget {
    id: WidgetId,
    label: String,
    disabled: bool,
    style: ButtonStyle,
    controllers: Vec<Box<dyn EventController>>,
    animator: VisualStateAnimator,
    /// Pre-resolved background color from prepaint (animator interpolation).
    resolved_bg: Color,
    /// Pre-resolved focus state from prepaint (interaction query).
    resolved_focused: bool,
    /// Pre-resolved hover state from prepaint.
    resolved_hovered: bool,
}

impl ButtonWidget {
    /// Creates a button with the given label text.
    pub fn new(label: impl Into<String>) -> Self {
        let style = ButtonStyle::default();
        let bg = style.bg;
        Self {
            id: WidgetId::next(),
            label: label.into(),
            disabled: false,
            controllers: vec![
                Box::new(HoverController::new()),
                Box::new(ClickController::new()),
                Box::new(FocusController::new()),
            ],
            animator: VisualStateAnimator::new(vec![common_states(
                style.bg,
                style.hover_bg,
                style.pressed_bg,
                style.disabled_bg,
            )]),
            style,
            resolved_bg: bg,
            resolved_focused: false,
            resolved_hovered: false,
        }
    }

    /// Returns the button label.
    pub fn label(&self) -> &str {
        &self.label
    }

    /// Returns whether the button is disabled.
    pub fn is_disabled(&self) -> bool {
        self.disabled
    }

    /// Returns the pre-resolved background color from the last prepaint.
    pub fn resolved_bg(&self) -> Color {
        self.resolved_bg
    }

    /// Returns the pre-resolved focus state from the last prepaint.
    pub fn resolved_focused(&self) -> bool {
        self.resolved_focused
    }

    /// Sets the disabled state.
    pub fn set_disabled(&mut self, disabled: bool) {
        self.disabled = disabled;
    }

    /// Sets the button style.
    #[must_use]
    pub fn with_style(mut self, style: ButtonStyle) -> Self {
        self.resolved_bg = style.bg;
        self.animator = VisualStateAnimator::new(vec![common_states(
            style.bg,
            style.hover_bg,
            style.pressed_bg,
            style.disabled_bg,
        )]);
        self.style = style;
        self
    }

    /// Sets the disabled state via builder.
    #[must_use]
    pub fn with_disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Returns the current text color based on state.
    fn current_fg(&self) -> Color {
        if self.disabled {
            if self.style.disabled_opacity < 1.0 {
                self.style
                    .fg
                    .with_alpha(self.style.fg.a * self.style.disabled_opacity)
            } else {
                self.style.disabled_fg
            }
        } else if self.resolved_hovered {
            self.style.hover_fg
        } else {
            self.style.fg
        }
    }

    /// Builds the `TextStyle` for measurement and shaping.
    fn text_style(&self) -> TextStyle {
        TextStyle {
            size: self.style.font_size,
            color: self.current_fg(),
            weight: self.style.weight,
            letter_spacing: self.style.letter_spacing,
            text_transform: self.style.text_transform,
            ..TextStyle::default()
        }
    }
}

impl std::fmt::Debug for ButtonWidget {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ButtonWidget")
            .field("id", &self.id)
            .field("label", &self.label)
            .field("disabled", &self.disabled)
            .field("style", &self.style)
            .field("controller_count", &self.controllers.len())
            .field("animator", &self.animator)
            .field("resolved_bg", &self.resolved_bg)
            .field("resolved_focused", &self.resolved_focused)
            .field("resolved_hovered", &self.resolved_hovered)
            .finish()
    }
}

impl Widget for ButtonWidget {
    fn id(&self) -> WidgetId {
        self.id
    }

    fn is_focusable(&self) -> bool {
        !self.disabled
    }

    fn sense(&self) -> Sense {
        Sense::click()
    }

    fn layout(&self, ctx: &LayoutCtx<'_>) -> LayoutBox {
        let style = self.text_style();
        let metrics = ctx.measurer.measure(&self.label, &style, f32::INFINITY);
        let w = metrics.width + self.style.padding.width();
        let h = metrics.height + self.style.padding.height();
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

    fn prepaint(&mut self, ctx: &mut PrepaintCtx<'_>) {
        self.resolved_bg = self.animator.get_bg_color();
        self.resolved_focused = ctx.is_interaction_focused();
        self.resolved_hovered = ctx.is_hot();
    }

    fn paint(&self, ctx: &mut DrawCtx<'_>) {
        // Focus ring from pre-resolved interaction state.
        if self.resolved_focused {
            let ring_rect = ctx.bounds.inset(Insets::all(-2.0));
            let ring_style = RectStyle::filled(Color::TRANSPARENT)
                .with_border(2.0, self.style.focus_ring_color)
                .with_radius(self.style.corner_radius + 2.0);
            ctx.scene.push_quad(ring_rect, ring_style);
        }

        // Background from pre-resolved animator state.
        let opacity_fade = self.disabled && self.style.disabled_opacity < 1.0;
        let bg = if opacity_fade {
            self.resolved_bg
                .with_alpha(self.resolved_bg.a * self.style.disabled_opacity)
        } else {
            self.resolved_bg
        };
        ctx.scene.push_layer_bg(bg);

        let border_color = if self.resolved_hovered {
            self.style.hover_border_color
        } else {
            self.style.border_color
        };
        let border_color = if opacity_fade {
            border_color.with_alpha(border_color.a * self.style.disabled_opacity)
        } else {
            border_color
        };
        let bg_style = RectStyle::filled(bg)
            .with_border(self.style.border_width, border_color)
            .with_radius(self.style.corner_radius);
        ctx.scene.push_quad(ctx.bounds, bg_style);

        // Label text, centered in the padded area.
        if !self.label.is_empty() {
            let style = self.text_style();
            let inner = ctx.bounds.inset(self.style.padding);
            let shaped = ctx.measurer.shape(&self.label, &style, inner.width());
            let x = inner.x() + (inner.width() - shaped.width) / 2.0;
            let y = inner.y() + (inner.height() - shaped.height) / 2.0;
            ctx.scene
                .push_text(Point::new(x, y), shaped, self.current_fg());
        }

        ctx.scene.pop_layer_bg();

        // Signal continued redraws while the animator is transitioning.
        if self.animator.is_animating() {
            ctx.request_anim_frame();
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

pub(crate) mod id_override;

#[cfg(test)]
mod tests;
