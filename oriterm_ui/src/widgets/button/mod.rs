//! Button widget with hover, pressed, and disabled visual states.
//!
//! Emits `WidgetAction::Clicked` on mouse click (via [`ClickController`]) or
//! keyboard activation (Enter/Space when focused). Uses [`VisualStateAnimator`]
//! with `common_states()` for smooth state color transitions.

use crate::color::Color;
use crate::controllers::{
    ClickController, EventController, FocusController, HoverController, KeyActivationController,
};
use crate::draw::RectStyle;
use crate::geometry::{Insets, Point};
use crate::layout::LayoutBox;
use crate::sense::Sense;
use crate::text::TextStyle;
use crate::visual_state::common_states;
use crate::visual_state::transition::VisualStateAnimator;
use crate::widget_id::WidgetId;

use crate::theme::UiTheme;

use super::{DrawCtx, LayoutCtx, Widget};

/// Visual style for a [`ButtonWidget`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ButtonStyle {
    /// Text color.
    pub fg: Color,
    /// Background color (normal state).
    pub bg: Color,
    /// Background when hovered.
    pub hover_bg: Color,
    /// Background when pressed.
    pub pressed_bg: Color,
    /// Border color.
    pub border_color: Color,
    /// Border width.
    pub border_width: f32,
    /// Corner radius.
    pub corner_radius: f32,
    /// Inner padding.
    pub padding: Insets,
    /// Font size in points.
    pub font_size: f32,
    /// Disabled text color.
    pub disabled_fg: Color,
    /// Disabled background color.
    pub disabled_bg: Color,
    /// Focus ring color.
    pub focus_ring_color: Color,
}

impl ButtonStyle {
    /// Derives a button style from the given theme.
    pub fn from_theme(theme: &UiTheme) -> Self {
        Self {
            fg: theme.fg_primary,
            bg: theme.bg_primary,
            hover_bg: theme.bg_hover,
            pressed_bg: theme.bg_active,
            border_color: theme.border,
            border_width: 1.0,
            corner_radius: theme.corner_radius,
            padding: Insets::vh(6.0, 12.0),
            font_size: theme.font_size,
            disabled_fg: theme.fg_disabled,
            disabled_bg: theme.bg_secondary,
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
}

impl ButtonWidget {
    /// Creates a button with the given label text.
    pub fn new(label: impl Into<String>) -> Self {
        let style = ButtonStyle::default();
        Self {
            id: WidgetId::next(),
            label: label.into(),
            disabled: false,
            controllers: vec![
                Box::new(HoverController::new()),
                Box::new(ClickController::new()),
                Box::new(KeyActivationController::new()),
                Box::new(FocusController::new()),
            ],
            animator: VisualStateAnimator::new(vec![common_states(
                style.bg,
                style.hover_bg,
                style.pressed_bg,
                style.disabled_bg,
            )]),
            style,
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

    /// Sets the disabled state.
    pub fn set_disabled(&mut self, disabled: bool) {
        self.disabled = disabled;
    }

    /// Sets the button style.
    #[must_use]
    pub fn with_style(mut self, style: ButtonStyle) -> Self {
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
            self.style.disabled_fg
        } else {
            self.style.fg
        }
    }

    /// Builds the `TextStyle` for measurement and shaping.
    fn text_style(&self) -> TextStyle {
        TextStyle::new(self.style.font_size, self.current_fg())
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

    fn paint(&self, ctx: &mut DrawCtx<'_>) {
        // Focus ring: use InteractionManager when available, fall back to
        // legacy `focused_widget` field during transition (§08.6 removes it).
        let focused = ctx.is_interaction_focused();
        if focused {
            let ring_rect = ctx.bounds.inset(Insets::all(-2.0));
            let ring_style = RectStyle::filled(Color::TRANSPARENT)
                .with_border(2.0, self.style.focus_ring_color)
                .with_radius(self.style.corner_radius + 2.0);
            ctx.draw_list.push_rect(ring_rect, ring_style);
        }

        // Background from visual state animator (transitions between Normal,
        // Hovered, Pressed, Disabled states automatically).
        let bg = self.animator.get_bg_color(ctx.now);
        ctx.draw_list.push_layer(bg);

        let bg_style = RectStyle::filled(bg)
            .with_border(self.style.border_width, self.style.border_color)
            .with_radius(self.style.corner_radius);
        ctx.draw_list.push_rect(ctx.bounds, bg_style);

        // Label text, centered in the padded area.
        if !self.label.is_empty() {
            let style = self.text_style();
            let inner = ctx.bounds.inset(self.style.padding);
            let shaped = ctx.measurer.shape(&self.label, &style, inner.width());
            let x = inner.x() + (inner.width() - shaped.width) / 2.0;
            let y = inner.y() + (inner.height() - shaped.height) / 2.0;
            ctx.draw_list
                .push_text(Point::new(x, y), shaped, self.current_fg());
        }

        ctx.draw_list.pop_layer();

        // Signal continued redraws while the animator is transitioning.
        if self.animator.is_animating(ctx.now) {
            ctx.request_anim_frame();
        }
    }
}

#[cfg(test)]
mod tests;
