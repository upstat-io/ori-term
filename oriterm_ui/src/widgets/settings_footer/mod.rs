//! Settings footer widget — sticky footer for the settings right column.
//!
//! Contains an unsaved-changes indicator (hidden when clean), a fill spacer,
//! and three action buttons: Reset to Defaults, Cancel, Save. The footer
//! does NOT translate button clicks to semantic actions — that stays in
//! `SettingsPanel::on_action()`. The footer only manages its own visual
//! state (dirty indicator, Save disabled state) via `accept_action`.

use std::cell::RefCell;
use std::rc::Rc;

use crate::color::Color;
use crate::draw::RectStyle;
use crate::geometry::{Insets, Rect};
use crate::icons::IconId;
use crate::layout::{Align, Direction, LayoutBox, LayoutNode, SizeSpec, compute_layout};
use crate::sense::Sense;
use crate::text::{FontWeight, TextTransform};
use crate::widget_id::WidgetId;

use crate::theme::UiTheme;

use super::button::id_override::IdOverrideButton;
use super::button::{ButtonStyle, ButtonWidget};
use super::container::ContainerWidget;
use super::icon_widget::IconWidget;
use super::label::{LabelStyle, LabelWidget};
use super::modifiers::{VisibilityMode, VisibilityWidget};
use super::spacer::SpacerWidget;
use super::{DrawCtx, LayoutCtx, TextMeasurer, Widget, WidgetAction};

/// Height of the footer bar in logical pixels.
pub const FOOTER_HEIGHT: f32 = 52.0;

/// Settings footer with unsaved indicator + Reset/Cancel/Save buttons.
///
/// Laid out as a horizontal row with explicit margin spacers (no `LayoutBox`
/// padding — uses spacers for margins instead).
///
/// `[28px | indicator | fill | reset | 8px | cancel | 8px | save | 28px]`
///
/// The indicator group is wrapped in a `VisibilityWidget` and hidden when
/// `dirty == false`. Vertical centering is handled by `Align::Center` on
/// the 52px-tall row.
pub struct SettingsFooterWidget {
    id: WidgetId,
    margin_left: SpacerWidget,
    /// Unsaved indicator wrapped in visibility (hidden when clean).
    unsaved_visibility: VisibilityWidget,
    spacer_fill: SpacerWidget,
    reset_button: IdOverrideButton,
    spacer_1: SpacerWidget,
    cancel_button: IdOverrideButton,
    spacer_2: SpacerWidget,
    save_button: IdOverrideButton,
    margin_right: SpacerWidget,
    dirty: bool,
    cached_layout: RefCell<Option<(Rect, Rc<LayoutNode>)>>,
}

impl SettingsFooterWidget {
    /// Creates a new footer with the three buttons and unsaved indicator.
    ///
    /// All button IDs are allocated internally; retrieve them via
    /// [`button_ids()`] before boxing. Initial state: clean (indicator
    /// hidden, Save disabled).
    pub fn new(theme: &UiTheme) -> Self {
        let reset_id = WidgetId::next();
        let cancel_id = WidgetId::next();
        let save_id = WidgetId::next();

        // Unsaved indicator group: 14px alert-circle icon + "Unsaved changes" label, gap 6px.
        let icon = IconWidget::new(IconId::AlertCircle, 14, theme.warning);
        let label = LabelWidget::new("Unsaved changes").with_style(LabelStyle {
            font_size: 11.0,
            color: theme.warning,
            weight: FontWeight::NORMAL,
            text_transform: TextTransform::Uppercase,
            letter_spacing: 0.66,
            line_height: None,
            ..LabelStyle::default()
        });
        let indicator_group = ContainerWidget::row()
            .with_gap(6.0)
            .with_align(Align::Center)
            .with_child(Box::new(icon))
            .with_child(Box::new(label));
        let unsaved_visibility = VisibilityWidget::new(
            Box::new(indicator_group),
            VisibilityMode::DisplayNone, // hidden when clean
        );

        // Danger-ghost: neutral at rest, red on hover (mockup btn-danger-ghost).
        let reset_btn = ButtonWidget::new("Reset to Defaults").with_style(ButtonStyle {
            fg: theme.fg_secondary,
            hover_fg: theme.danger,
            bg: Color::TRANSPARENT,
            hover_bg: theme.danger_bg,
            pressed_bg: theme.bg_active,
            border_color: theme.border,
            hover_border_color: theme.danger,
            border_width: 2.0,
            font_size: 12.0,
            weight: FontWeight::MEDIUM,
            letter_spacing: 0.48,
            text_transform: TextTransform::Uppercase,
            padding: Insets::vh(6.0, 16.0),
            ..ButtonStyle::from_theme(theme)
        });
        let reset_button = IdOverrideButton::new(reset_btn, reset_id);

        // Ghost style: transparent bg with border (mockup btn-ghost).
        let cancel_btn = ButtonWidget::new("Cancel").with_style(ButtonStyle {
            fg: theme.fg_secondary,
            hover_fg: theme.fg_primary,
            bg: Color::TRANSPARENT,
            hover_bg: theme.bg_hover,
            pressed_bg: theme.bg_active,
            border_color: theme.border,
            hover_border_color: theme.border_strong,
            border_width: 2.0,
            font_size: 12.0,
            weight: FontWeight::MEDIUM,
            letter_spacing: 0.48,
            text_transform: TextTransform::Uppercase,
            padding: Insets::vh(6.0, 16.0),
            ..ButtonStyle::from_theme(theme)
        });
        let cancel_button = IdOverrideButton::new(cancel_btn, cancel_id);

        // Primary accent: dark text on accent bg (mockup btn-primary).
        let save_btn = ButtonWidget::new("Save").with_style(ButtonStyle {
            fg: theme.bg_secondary,
            hover_fg: theme.bg_secondary,
            bg: theme.accent,
            hover_bg: theme.accent_hover,
            pressed_bg: theme.accent,
            border_color: theme.accent,
            hover_border_color: theme.accent_hover,
            border_width: 2.0,
            font_size: 12.0,
            weight: FontWeight::BOLD,
            letter_spacing: 0.48,
            text_transform: TextTransform::Uppercase,
            padding: Insets::vh(6.0, 16.0),
            disabled_opacity: 0.4,
            ..ButtonStyle::from_theme(theme)
        });
        let mut save_button = IdOverrideButton::new(save_btn, save_id);
        save_button.set_disabled(true); // Clean state: no unsaved changes.

        Self {
            id: WidgetId::next(),
            margin_left: SpacerWidget::fixed(28.0, 0.0),
            unsaved_visibility,
            spacer_fill: SpacerWidget::fill(),
            reset_button,
            spacer_1: SpacerWidget::fixed(8.0, 0.0),
            cancel_button,
            spacer_2: SpacerWidget::fixed(8.0, 0.0),
            save_button,
            margin_right: SpacerWidget::fixed(28.0, 0.0),
            dirty: false,
            cached_layout: RefCell::new(None),
        }
    }

    /// Returns `(reset_id, cancel_id, save_id)` for external action dispatch.
    pub fn button_ids(&self) -> (WidgetId, WidgetId, WidgetId) {
        (
            self.reset_button.id(),
            self.cancel_button.id(),
            self.save_button.id(),
        )
    }

    /// Returns cached layout if bounds match, otherwise recomputes.
    fn get_or_compute_layout(
        &self,
        measurer: &dyn TextMeasurer,
        theme: &UiTheme,
        bounds: Rect,
    ) -> Rc<LayoutNode> {
        {
            let cached = self.cached_layout.borrow();
            if let Some((ref cb, ref node)) = *cached {
                if *cb == bounds {
                    return Rc::clone(node);
                }
            }
        }
        let ctx = LayoutCtx { measurer, theme };
        let layout_box = self.layout(&ctx);
        let node = Rc::new(compute_layout(&layout_box, bounds));
        *self.cached_layout.borrow_mut() = Some((bounds, Rc::clone(&node)));
        node
    }

    /// Collects all child widgets in layout order for iteration.
    fn children(&self) -> [&dyn Widget; 9] {
        [
            &self.margin_left,
            &self.unsaved_visibility,
            &self.spacer_fill,
            &self.reset_button,
            &self.spacer_1,
            &self.cancel_button,
            &self.spacer_2,
            &self.save_button,
            &self.margin_right,
        ]
    }
}

impl Widget for SettingsFooterWidget {
    fn id(&self) -> WidgetId {
        self.id
    }

    fn is_focusable(&self) -> bool {
        false
    }

    fn sense(&self) -> Sense {
        Sense::none()
    }

    fn layout(&self, ctx: &LayoutCtx<'_>) -> LayoutBox {
        let children = vec![
            self.margin_left.layout(ctx),
            self.unsaved_visibility.layout(ctx),
            self.spacer_fill.layout(ctx),
            self.reset_button.layout(ctx),
            self.spacer_1.layout(ctx),
            self.cancel_button.layout(ctx),
            self.spacer_2.layout(ctx),
            self.save_button.layout(ctx),
            self.margin_right.layout(ctx),
        ];
        // No LayoutBox padding — uses spacer children for horizontal margins.
        // Horizontal margins use explicit SpacerWidget children (28px each).
        // Vertical centering is handled by Align::Center — buttons (~28px)
        // centered in the 52px row yield ~12px top/bottom margins.
        LayoutBox::flex(Direction::Row, children)
            .with_align(Align::Center)
            .with_width(SizeSpec::Fill)
            .with_height(SizeSpec::Fixed(FOOTER_HEIGHT))
            .with_widget_id(self.id)
    }

    fn paint(&self, ctx: &mut DrawCtx<'_>) {
        // 2px separator at the top of the footer bounds.
        let sep_rect = Rect::new(ctx.bounds.x(), ctx.bounds.y(), ctx.bounds.width(), 2.0);
        ctx.scene
            .push_quad(sep_rect, RectStyle::filled(ctx.theme.border));

        let layout = self.get_or_compute_layout(ctx.measurer, ctx.theme, ctx.bounds);
        let children = self.children();

        for (idx, child) in children.iter().enumerate() {
            if let Some(child_node) = layout.children.get(idx) {
                let child_id = child.id();
                let bounds = child_node.rect;
                let mut child_ctx = ctx.for_child(child_id, bounds);
                child.paint(&mut child_ctx);
            }
        }
    }

    fn for_each_child_mut(&mut self, visitor: &mut dyn FnMut(&mut dyn Widget)) {
        visitor(&mut self.margin_left);
        visitor(&mut self.unsaved_visibility);
        visitor(&mut self.spacer_fill);
        visitor(&mut self.reset_button);
        visitor(&mut self.spacer_1);
        visitor(&mut self.cancel_button);
        visitor(&mut self.spacer_2);
        visitor(&mut self.save_button);
        visitor(&mut self.margin_right);
    }

    fn on_action(&mut self, action: WidgetAction, _bounds: Rect) -> Option<WidgetAction> {
        // Footer does NOT translate button clicks — SettingsPanel handles that.
        Some(action)
    }

    fn accept_action(&mut self, action: &WidgetAction) -> bool {
        if let WidgetAction::SettingsUnsaved(dirty) = action {
            let mode = if *dirty {
                VisibilityMode::Visible
            } else {
                VisibilityMode::DisplayNone
            };
            self.unsaved_visibility.set_mode(mode);
            self.save_button.set_disabled(!*dirty);
            self.dirty = *dirty;
            *self.cached_layout.borrow_mut() = None;
            return true;
        }

        // Propagate to children (buttons don't override accept_action, but
        // VisibilityWidget might need it for future actions).
        let mut handled = self.unsaved_visibility.accept_action(action);
        handled |= self.reset_button.accept_action(action);
        handled |= self.cancel_button.accept_action(action);
        handled |= self.save_button.accept_action(action);
        handled
    }

    fn focusable_children(&self) -> Vec<WidgetId> {
        let mut ids = Vec::new();
        ids.extend(self.reset_button.focusable_children());
        ids.extend(self.cancel_button.focusable_children());
        ids.extend(self.save_button.focusable_children());
        ids
    }
}

#[cfg(test)]
mod tests;
