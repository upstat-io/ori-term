//! Searchable dropdown — trigger + filterable popup.
//!
//! Two-widget split:
//! - [`SearchableDropdownWidget`] — the closed trigger button. Owns the
//!   canonical `items` and `selected` state. Emits
//!   `WidgetAction::OpenSearchableDropdown` on click / Enter / Space /
//!   `ArrowDown`.
//! - [`SearchableDropdownPopupWidget`] (in `popup` submodule) — the overlay
//!   widget mounted by the App layer in response to the open action. Owns
//!   the ephemeral filter state (`query`, `filtered_indices`, `highlighted`)
//!   and emits `WidgetAction::Selected` routed back via the trigger's id.
//!
//! State-ownership rule: the trigger does NOT intercept `Selected` to mutate
//! its own `selected` field. The form-builder rebuilds the trigger from the
//! mutated config on the next frame; backward mutation is a
//! `LEAK:backward-reference` to avoid.

mod popup;

pub use popup::{SearchableDropdownPopupWidget, SearchableDropdownStyle};

use winit::window::CursorIcon;

use crate::action::KeymapAction;
use crate::controllers::{ClickController, EventController, HoverController};
use crate::draw::RectStyle;
use crate::geometry::{Point, Rect};
use crate::icons::IconId;
use crate::layout::LayoutBox;
use crate::sense::Sense;
use crate::text::TextStyle;
use crate::visual_state::common_states;
use crate::visual_state::transition::VisualStateAnimator;
use crate::widget_id::WidgetId;
use crate::widgets::dropdown::DropdownStyle;

use super::{DrawCtx, LayoutCtx, Widget, WidgetAction};

/// Closed trigger for a searchable dropdown.
///
/// Renders almost identically to [`crate::widgets::dropdown::DropdownWidget`]
/// — the visible difference is none, but the emitted action is
/// [`WidgetAction::OpenSearchableDropdown`] (carrying the full canonical
/// item list) so the App layer can mount a [`SearchableDropdownPopupWidget`]
/// instead of a plain `MenuWidget`.
pub struct SearchableDropdownWidget {
    id: WidgetId,
    items: Vec<String>,
    selected: Option<usize>,
    style: DropdownStyle,
    controllers: Vec<Box<dyn EventController>>,
    animator: VisualStateAnimator,
}

impl SearchableDropdownWidget {
    /// Creates a searchable dropdown with the given item list and no initial
    /// selection. Use [`Self::with_selected`] to set the initial highlight.
    pub fn new(items: Vec<String>) -> Self {
        let style = DropdownStyle::default();
        Self {
            id: WidgetId::next(),
            items,
            selected: None,
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

    /// Returns this widget's id (the routing target for popup-emitted
    /// `Selected` actions).
    pub fn id(&self) -> WidgetId {
        self.id
    }

    /// Returns the canonical item list.
    pub fn items(&self) -> &[String] {
        &self.items
    }

    /// Returns the currently selected canonical index, if any.
    pub fn selected(&self) -> Option<usize> {
        self.selected
    }

    /// Sets the selected index via builder (clamps if out of range).
    #[must_use]
    pub fn with_selected(mut self, index: usize) -> Self {
        self.selected = (index < self.items.len()).then_some(index);
        self
    }

    /// Overrides the minimum width of the trigger.
    #[must_use]
    pub fn with_min_width(mut self, px: f32) -> Self {
        self.style.min_width = px;
        self
    }

    /// Overrides the visual style.
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

    fn current_text(&self) -> &str {
        self.selected
            .and_then(|i| self.items.get(i))
            .map_or("", String::as_str)
    }

    fn text_style(&self) -> TextStyle {
        TextStyle::new(self.style.font_size, self.style.fg)
    }

    fn open_action(&self, bounds: Rect, initial_highlight: Option<usize>) -> WidgetAction {
        WidgetAction::OpenSearchableDropdown {
            id: self.id,
            items: self.items.clone(),
            selected: self.selected,
            anchor: bounds,
            initial_highlight,
        }
    }
}

impl std::fmt::Debug for SearchableDropdownWidget {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SearchableDropdownWidget")
            .field("id", &self.id)
            .field("items_len", &self.items.len())
            .field("selected", &self.selected)
            .field("style", &self.style)
            .field("controllers_len", &self.controllers.len())
            .field("animator", &self.animator)
            .finish()
    }
}

impl Widget for SearchableDropdownWidget {
    fn id(&self) -> WidgetId {
        self.id
    }

    fn is_focusable(&self) -> bool {
        true
    }

    fn sense(&self) -> Sense {
        Sense::click()
    }

    fn layout(&self, ctx: &LayoutCtx<'_>) -> LayoutBox {
        let style = self.text_style();
        let max_text_w = self
            .items
            .iter()
            .map(|item| ctx.measurer.measure(item, &style, f32::INFINITY).width)
            .fold(0.0_f32, f32::max);
        let content_w = max_text_w + self.style.padding.width() + self.style.indicator_width;
        let w = content_w.max(self.style.min_width);
        let metrics = ctx.measurer.measure("Mg", &style, f32::INFINITY);
        let h = metrics.height + self.style.padding.height();
        LayoutBox::leaf(w, h)
            .with_widget_id(self.id)
            .with_cursor_icon(CursorIcon::Pointer)
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
        let hovered = ctx.is_hot();
        let bounds = ctx.bounds;
        let s = &self.style;

        let border_color = if focused {
            s.focus_border_color
        } else if hovered {
            s.hover_border_color
        } else {
            s.border_color
        };

        let bg = self.animator.get_bg_color();
        ctx.scene.push_layer_bg(bg);

        let bg_style = RectStyle::filled(bg)
            .with_border(s.border_width, border_color)
            .with_radius(s.corner_radius);
        ctx.scene.push_quad(bounds, bg_style);

        let inner = bounds.inset(s.padding);
        let text_w = inner.width() - s.indicator_width;
        let text_style = self.text_style();
        let shaped = ctx.measurer.shape(self.current_text(), &text_style, text_w);
        let y = inner.y() + (inner.height() - shaped.height) / 2.0;
        ctx.scene.push_text(Point::new(inner.x(), y), shaped, s.fg);

        let icon_size: u32 = 10;
        if let Some(resolved) = ctx
            .icons
            .and_then(|ic| ic.get(IconId::DropdownArrow, icon_size))
        {
            let icon_f = icon_size as f32;
            let ix = bounds.right() - s.padding.right + (s.padding.right - icon_f) / 2.0;
            let iy = bounds.y() + (bounds.height() - icon_f) / 2.0;
            let icon_rect = Rect::new(ix, iy, icon_f, icon_f);
            ctx.scene.push_icon(
                icon_rect,
                resolved.atlas_page,
                resolved.uv,
                s.indicator_color,
            );
        }

        ctx.scene.pop_layer_bg();

        if self.animator.is_animating() {
            ctx.request_anim_frame();
        }
    }

    fn on_action(&mut self, action: WidgetAction, bounds: Rect) -> Option<WidgetAction> {
        match action {
            WidgetAction::Clicked(_) => Some(self.open_action(bounds, None)),
            other => Some(other),
        }
    }

    fn key_context(&self) -> Option<&'static str> {
        // Reuse the Dropdown context so default keybindings (Confirm /
        // NavigateDown / NavigateUp) fire on this widget too.
        Some("Dropdown")
    }

    fn handle_keymap_action(
        &mut self,
        action: &dyn KeymapAction,
        bounds: Rect,
    ) -> Option<WidgetAction> {
        match action.name() {
            "widget::Confirm" => Some(self.open_action(bounds, None)),
            // ArrowDown / ArrowUp on a closed trigger both open the popup with
            // the first item highlighted — mirrors `DropdownWidget` semantics
            // where either arrow key deepens the interaction.
            "widget::NavigateDown" | "widget::NavigateUp" => {
                Some(self.open_action(bounds, Some(0)))
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests;
