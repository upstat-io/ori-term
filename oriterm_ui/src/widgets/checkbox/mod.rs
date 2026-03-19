//! Checkbox widget — a toggleable check box with label.
//!
//! Emits `WidgetAction::Toggled` when clicked (via [`ClickController`]) or
//! activated via Space. Uses [`VisualStateAnimator`] with `common_states()`
//! for hover color transitions on the unchecked box.

use crate::color::Color;
use crate::controllers::{ClickController, EventController, HoverController};
use crate::draw::RectStyle;
use crate::geometry::{Point, Rect};
use crate::layout::LayoutBox;
use crate::sense::Sense;
use crate::text::TextStyle;
use crate::visual_state::common_states;
use crate::visual_state::transition::VisualStateAnimator;
use crate::widget_id::WidgetId;

use crate::theme::UiTheme;

use super::{DrawCtx, LayoutCtx, Widget, WidgetAction};

/// Visual style for a [`CheckboxWidget`].
#[derive(Debug, Clone, PartialEq)]
pub struct CheckboxStyle {
    /// Size of the check box square.
    pub box_size: f32,
    /// Gap between the box and label text.
    pub gap: f32,
    /// Unchecked box background.
    pub bg: Color,
    /// Unchecked box background when hovered.
    pub hover_bg: Color,
    /// Checked box background (accent fill).
    pub checked_bg: Color,
    /// Box border color.
    pub border_color: Color,
    /// Border width.
    pub border_width: f32,
    /// Corner radius.
    pub corner_radius: f32,
    /// Check mark color.
    pub check_color: Color,
    /// Label text color.
    pub label_color: Color,
    /// Font size for the label.
    pub font_size: f32,
    /// Disabled text and box color.
    pub disabled_fg: Color,
    /// Disabled background.
    pub disabled_bg: Color,
    /// Focus ring color.
    pub focus_ring_color: Color,
}

impl CheckboxStyle {
    /// Derives a checkbox style from the given theme.
    pub fn from_theme(theme: &UiTheme) -> Self {
        Self {
            box_size: 16.0,
            gap: theme.spacing,
            bg: theme.bg_primary,
            hover_bg: theme.bg_hover,
            checked_bg: theme.accent,
            border_color: theme.border,
            border_width: 1.0,
            corner_radius: 3.0,
            check_color: Color::WHITE,
            label_color: theme.fg_primary,
            font_size: theme.font_size,
            disabled_fg: theme.fg_disabled,
            disabled_bg: theme.bg_secondary,
            focus_ring_color: theme.accent,
        }
    }
}

impl Default for CheckboxStyle {
    fn default() -> Self {
        Self::from_theme(&UiTheme::dark())
    }
}

/// A checkbox with label text.
///
/// Toggles between checked and unchecked on click or Space.
/// Emits `WidgetAction::Toggled { id, value }`. Hover transitions
/// use [`VisualStateAnimator`] with `common_states()`.
pub struct CheckboxWidget {
    id: WidgetId,
    label: String,
    checked: bool,
    disabled: bool,
    style: CheckboxStyle,
    controllers: Vec<Box<dyn EventController>>,
    /// Animator for unchecked-state hover bg transition.
    animator: VisualStateAnimator,
}

impl CheckboxWidget {
    /// Creates an unchecked checkbox with the given label.
    pub fn new(label: impl Into<String>) -> Self {
        let style = CheckboxStyle::default();
        Self {
            id: WidgetId::next(),
            label: label.into(),
            checked: false,
            disabled: false,
            controllers: vec![
                Box::new(HoverController::new()),
                Box::new(ClickController::new()),
            ],
            animator: VisualStateAnimator::new(vec![common_states(
                style.bg,
                style.hover_bg,
                style.hover_bg,
                style.disabled_bg,
            )]),
            style,
        }
    }

    /// Returns whether the checkbox is checked.
    pub fn is_checked(&self) -> bool {
        self.checked
    }

    /// Sets the checked state.
    pub fn set_checked(&mut self, checked: bool) {
        self.checked = checked;
    }

    /// Returns whether the checkbox is disabled.
    pub fn is_disabled(&self) -> bool {
        self.disabled
    }

    /// Sets the disabled state.
    pub fn set_disabled(&mut self, disabled: bool) {
        self.disabled = disabled;
    }

    /// Sets the style.
    #[must_use]
    pub fn with_style(mut self, style: CheckboxStyle) -> Self {
        self.animator = VisualStateAnimator::new(vec![common_states(
            style.bg,
            style.hover_bg,
            style.hover_bg,
            style.disabled_bg,
        )]);
        self.style = style;
        self
    }

    /// Sets the initial checked state via builder.
    #[must_use]
    pub fn with_checked(mut self, checked: bool) -> Self {
        self.checked = checked;
        self
    }

    /// Sets the disabled state via builder.
    #[must_use]
    pub fn with_disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Toggles the check state and returns the resulting action.
    fn toggle(&mut self) -> WidgetAction {
        self.checked = !self.checked;
        WidgetAction::Toggled {
            id: self.id,
            value: self.checked,
        }
    }

    /// Returns the label text color based on state.
    fn label_fg(&self) -> Color {
        if self.disabled {
            self.style.disabled_fg
        } else {
            self.style.label_color
        }
    }

    /// Builds the label `TextStyle`.
    fn text_style(&self) -> TextStyle {
        TextStyle::new(self.style.font_size, self.label_fg())
    }
}

impl std::fmt::Debug for CheckboxWidget {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CheckboxWidget")
            .field("id", &self.id)
            .field("label", &self.label)
            .field("checked", &self.checked)
            .field("disabled", &self.disabled)
            .field("style", &self.style)
            .field("controller_count", &self.controllers.len())
            .field("animator", &self.animator)
            .finish()
    }
}

impl Widget for CheckboxWidget {
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
        let w = self.style.box_size + self.style.gap + metrics.width;
        let h = self.style.box_size.max(metrics.height);
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
        let focused = ctx.is_interaction_focused();
        let bounds = ctx.bounds;
        let s = &self.style;

        // Check box rect — vertically centered.
        let box_y = bounds.y() + (bounds.height() - s.box_size) / 2.0;
        let box_rect = Rect::new(bounds.x(), box_y, s.box_size, s.box_size);

        // Focus ring around the box.
        if focused {
            let ring = box_rect.inset(crate::geometry::Insets::all(-2.0));
            let ring_style = RectStyle::filled(Color::TRANSPARENT)
                .with_border(2.0, s.focus_ring_color)
                .with_radius(s.corner_radius + 2.0);
            ctx.scene.push_quad(ring, ring_style);
        }

        // Box bg: checked_bg when checked, animator-driven hover when unchecked.
        let box_bg = if self.disabled {
            s.disabled_bg
        } else if self.checked {
            s.checked_bg
        } else {
            self.animator.get_bg_color(ctx.now)
        };
        let box_style = RectStyle::filled(box_bg)
            .with_border(s.border_width, s.border_color)
            .with_radius(s.corner_radius);
        ctx.scene.push_quad(box_rect, box_style);

        // Check mark — simple diagonal lines forming a check.
        if self.checked {
            let color = if self.disabled {
                s.disabled_fg
            } else {
                s.check_color
            };
            let inset = s.box_size * 0.25;
            let x0 = box_rect.x() + inset;
            let y0 = box_rect.y() + s.box_size * 0.5;
            let x1 = box_rect.x() + s.box_size * 0.4;
            let y1 = box_rect.bottom() - inset;
            let x2 = box_rect.right() - inset;
            let y2 = box_rect.y() + inset;

            ctx.scene
                .push_line(Point::new(x0, y0), Point::new(x1, y1), 2.0, color);
            ctx.scene
                .push_line(Point::new(x1, y1), Point::new(x2, y2), 2.0, color);
        }

        // Label text.
        if !self.label.is_empty() {
            let style = self.text_style();
            let text_x = bounds.x() + s.box_size + s.gap;
            let text_w = bounds.width() - s.box_size - s.gap;
            let shaped = ctx.measurer.shape(&self.label, &style, text_w);
            let text_y = bounds.y() + (bounds.height() - shaped.height) / 2.0;
            ctx.scene
                .push_text(Point::new(text_x, text_y), shaped, self.label_fg());
        }

        // Signal continued redraws while the animator is transitioning.
        if self.animator.is_animating(ctx.now) {
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
