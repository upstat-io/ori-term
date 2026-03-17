//! Dropdown widget — a trigger button showing the selected item.
//!
//! Displays the currently selected item and a dropdown indicator.
//! On click or Enter/Space, emits `WidgetAction::OpenDropdown` for
//! the app layer to open a popup overlay. Arrow keys cycle through
//! items directly, emitting `WidgetAction::Selected`. Uses
//! [`VisualStateAnimator`] with `common_states()` for smooth hover
//! color transitions.

use crate::color::Color;
use crate::controllers::{ClickController, EventController, HoverController};
use crate::draw::RectStyle;
use crate::geometry::{Insets, Point, Rect};
use crate::icons::IconId;
use crate::layout::LayoutBox;
use crate::sense::Sense;
use crate::text::TextStyle;
use crate::visual_state::common_states;
use crate::visual_state::transition::VisualStateAnimator;
use crate::widget_id::WidgetId;

use crate::theme::UiTheme;

use super::{DrawCtx, LayoutCtx, Widget, WidgetAction};

/// Visual style for a [`DropdownWidget`].
#[derive(Debug, Clone, PartialEq)]
pub struct DropdownStyle {
    /// Text color.
    pub fg: Color,
    /// Background color.
    pub bg: Color,
    /// Hovered background.
    pub hover_bg: Color,
    /// Pressed background.
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
    /// Width reserved for the dropdown indicator arrow.
    pub indicator_width: f32,
    /// Indicator color.
    pub indicator_color: Color,
    /// Disabled text color.
    pub disabled_fg: Color,
    /// Disabled background.
    pub disabled_bg: Color,
    /// Focus ring color.
    pub focus_ring_color: Color,
}

impl DropdownStyle {
    /// Derives a dropdown style from the given theme.
    pub fn from_theme(theme: &UiTheme) -> Self {
        Self {
            fg: theme.fg_primary,
            bg: theme.bg_primary,
            hover_bg: theme.bg_hover,
            pressed_bg: theme.bg_active,
            border_color: theme.border,
            border_width: 1.0,
            corner_radius: theme.corner_radius,
            padding: Insets::vh(6.0, 10.0),
            font_size: theme.font_size,
            indicator_width: 20.0,
            indicator_color: theme.fg_primary,
            disabled_fg: theme.fg_disabled,
            disabled_bg: theme.bg_secondary,
            focus_ring_color: theme.accent,
        }
    }
}

impl Default for DropdownStyle {
    fn default() -> Self {
        Self::from_theme(&UiTheme::dark())
    }
}

/// A dropdown trigger button showing the selected item.
///
/// Arrow Up/Down keys cycle through items directly. Enter/Space and
/// mouse click emit `WidgetAction::OpenDropdown` for the app layer
/// to open a popup overlay with the options list. Hover transitions
/// use [`VisualStateAnimator`] with `common_states()`.
pub struct DropdownWidget {
    id: WidgetId,
    items: Vec<String>,
    selected: usize,
    disabled: bool,
    style: DropdownStyle,
    controllers: Vec<Box<dyn EventController>>,
    /// Animator for bg state transitions (Normal/Hovered/Pressed/Disabled).
    animator: VisualStateAnimator,
}

impl DropdownWidget {
    /// Creates a dropdown with the given items, selecting the first.
    ///
    /// Panics if `items` is empty.
    pub fn new(items: Vec<String>) -> Self {
        assert!(!items.is_empty(), "dropdown requires at least one item");
        let style = DropdownStyle::default();
        Self {
            id: WidgetId::next(),
            items,
            selected: 0,
            disabled: false,
            controllers: vec![
                Box::new(HoverController::new()),
                Box::new(ClickController::new()),
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

    /// Returns the currently selected index.
    pub fn selected(&self) -> usize {
        self.selected
    }

    /// Returns the currently selected item text.
    pub fn selected_text(&self) -> &str {
        &self.items[self.selected]
    }

    /// Returns the items list.
    pub fn items(&self) -> &[String] {
        &self.items
    }

    /// Sets the selected index, clamping to valid range.
    pub fn set_selected(&mut self, index: usize) {
        self.selected = index.min(self.items.len() - 1);
    }

    /// Returns whether the dropdown is disabled.
    pub fn is_disabled(&self) -> bool {
        self.disabled
    }

    /// Sets the disabled state.
    pub fn set_disabled(&mut self, disabled: bool) {
        self.disabled = disabled;
    }

    /// Sets the selected index via builder.
    #[must_use]
    pub fn with_selected(mut self, index: usize) -> Self {
        self.selected = index.min(self.items.len() - 1);
        self
    }

    /// Sets the disabled state via builder.
    #[must_use]
    pub fn with_disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Sets the style, rebuilding the animator.
    #[must_use]
    pub fn with_style(mut self, style: DropdownStyle) -> Self {
        self.animator = VisualStateAnimator::new(vec![common_states(
            style.bg,
            style.hover_bg,
            style.pressed_bg,
            style.disabled_bg,
        )]);
        self.style = style;
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

    /// Builds the `TextStyle` for measurement.
    fn text_style(&self) -> TextStyle {
        TextStyle::new(self.style.font_size, self.current_fg())
    }
}

impl std::fmt::Debug for DropdownWidget {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DropdownWidget")
            .field("id", &self.id)
            .field("items", &self.items)
            .field("selected", &self.selected)
            .field("disabled", &self.disabled)
            .field("style", &self.style)
            .field("controller_count", &self.controllers.len())
            .field("animator", &self.animator)
            .finish()
    }
}

impl Widget for DropdownWidget {
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
        // Natural width accommodates the widest item + padding + indicator.
        // Fill width lets the dropdown stretch in form rows.
        let style = self.text_style();
        let max_text_w = self
            .items
            .iter()
            .map(|item| ctx.measurer.measure(item, &style, f32::INFINITY).width)
            .fold(0.0_f32, f32::max);
        let w = max_text_w + self.style.padding.width() + self.style.indicator_width;
        let metrics = ctx.measurer.measure(&self.items[0], &style, f32::INFINITY);
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
        let focused = ctx.is_interaction_focused();
        let bounds = ctx.bounds;
        let s = &self.style;

        // Focus ring (outside the layer).
        if focused {
            let ring = bounds.inset(Insets::all(-2.0));
            let ring_style = RectStyle::filled(Color::TRANSPARENT)
                .with_border(2.0, s.focus_ring_color)
                .with_radius(s.corner_radius + 2.0);
            ctx.draw_list.push_rect(ring, ring_style);
        }

        // Background from visual state animator (transitions between Normal,
        // Hovered, Pressed, Disabled states automatically).
        let bg = self.animator.get_bg_color(ctx.now);
        ctx.draw_list.push_layer(bg);

        // Background rect.
        let bg_style = RectStyle::filled(bg)
            .with_border(s.border_width, s.border_color)
            .with_radius(s.corner_radius);
        ctx.draw_list.push_rect(bounds, bg_style);

        // Selected item text.
        let inner = bounds.inset(s.padding);
        let text_w = inner.width() - s.indicator_width;
        let style = self.text_style();
        let shaped = ctx.measurer.shape(self.selected_text(), &style, text_w);
        let y = inner.y() + (inner.height() - shaped.height) / 2.0;
        ctx.draw_list
            .push_text(Point::new(inner.x(), y), shaped, self.current_fg());

        // Dropdown indicator (downward-pointing chevron).
        let ind_x = bounds.right() - s.indicator_width;
        let ind_center_x = ind_x + s.indicator_width / 2.0;
        let ind_center_y = bounds.y() + bounds.height() / 2.0;
        let arrow_half = 4.0;
        let ind_color = if self.disabled {
            s.disabled_fg
        } else {
            s.indicator_color
        };

        let icon_size = (arrow_half * 2.0_f32).round() as u32;
        if let Some(resolved) = ctx
            .icons
            .and_then(|ic| ic.get(IconId::ChevronDown, icon_size))
        {
            let icon_rect = Rect::new(
                ind_center_x - arrow_half,
                ind_center_y - arrow_half / 2.0,
                arrow_half * 2.0,
                arrow_half,
            );
            ctx.draw_list
                .push_icon(icon_rect, resolved.atlas_page, resolved.uv, ind_color);
        } else {
            // Text fallback when icon atlas is not available.
            let chevron_style = TextStyle::new(s.font_size, ind_color);
            let shaped = ctx
                .measurer
                .shape("\u{25BE}", &chevron_style, s.indicator_width);
            let chevron_y = ind_center_y - shaped.height / 2.0;
            let chevron_x = ind_center_x - shaped.width / 2.0;
            ctx.draw_list
                .push_text(Point::new(chevron_x, chevron_y), shaped, ind_color);
        }

        ctx.draw_list.pop_layer();

        // Signal continued redraws while the animator is transitioning.
        if self.animator.is_animating(ctx.now) {
            ctx.request_anim_frame();
        }
    }

    fn on_action(&mut self, action: WidgetAction, bounds: Rect) -> Option<WidgetAction> {
        match action {
            WidgetAction::Clicked(_) => {
                // Transform generic click into dropdown open with widget state.
                Some(WidgetAction::OpenDropdown {
                    id: self.id,
                    options: self.items.clone(),
                    selected: self.selected,
                    anchor: bounds,
                })
            }
            other => Some(other),
        }
    }

    fn accept_action(&mut self, action: &WidgetAction) -> bool {
        if let WidgetAction::Selected { id, index } = action {
            if *id == self.id {
                self.set_selected(*index);
                return true;
            }
        }
        false
    }
}

#[cfg(test)]
mod tests;
