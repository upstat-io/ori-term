//! Keybinding display widgets.
//!
//! [`KbdBadge`] renders a single key as a styled badge (keycap appearance).
//! [`KeybindRow`] shows an action name on the left and key badges on the right.

use winit::window::CursorIcon;

use crate::color::Color;
use crate::controllers::{EventController, HoverController};
use crate::draw::RectStyle;
use crate::geometry::{Point, Rect};
use crate::layout::LayoutBox;
use crate::sense::Sense;
use crate::text::TextStyle;
use crate::theme::UiTheme;
use crate::visual_state::common_states;
use crate::visual_state::transition::VisualStateAnimator;
use crate::widget_id::WidgetId;

use super::{DrawCtx, LayoutCtx, Widget};

/// Badge corner radius.
const BADGE_RADIUS: f32 = 0.0;

/// Badge horizontal padding.
const BADGE_PAD_H: f32 = 6.0;

/// Badge vertical padding.
const BADGE_PAD_V: f32 = 3.0;

/// Badge text font size.
const BADGE_FONT_SIZE: f32 = 11.0;

/// Bottom border thickness for keycap depth effect.
const BADGE_BOTTOM_BORDER: f32 = 2.0;

/// Gap between badge and "+" separator.
const BADGE_GAP: f32 = 4.0;

/// Row minimum height.
const ROW_MIN_HEIGHT: f32 = 36.0;

/// Row horizontal padding.
const ROW_PAD_H: f32 = 8.0;

/// Action name font size.
const ACTION_FONT_SIZE: f32 = 13.0;

/// "+" separator font size.
const PLUS_FONT_SIZE: f32 = 10.0;

/// Row corner radius for hover background.
const ROW_RADIUS: f32 = 0.0;

/// A styled key badge with keycap appearance.
///
/// Renders a rounded rectangle with a thicker bottom border to simulate
/// a physical keycap. Display only — no interaction.
pub struct KbdBadge {
    id: WidgetId,
    key: String,
}

impl KbdBadge {
    /// Creates a key badge for the given key label.
    pub fn new(key: impl Into<String>) -> Self {
        Self {
            id: WidgetId::next(),
            key: key.into(),
        }
    }

    /// Returns the key label.
    pub fn key(&self) -> &str {
        &self.key
    }

    /// Computes the badge width based on text width.
    fn badge_width(&self, ctx: &LayoutCtx<'_>) -> f32 {
        let style = TextStyle::new(BADGE_FONT_SIZE, Color::WHITE);
        let m = ctx.measurer.measure(&self.key, &style, f32::INFINITY);
        m.width + BADGE_PAD_H * 2.0
    }

    /// Computes the badge height.
    fn badge_height(&self, ctx: &LayoutCtx<'_>) -> f32 {
        let style = TextStyle::new(BADGE_FONT_SIZE, Color::WHITE);
        let m = ctx.measurer.measure(&self.key, &style, f32::INFINITY);
        m.height + BADGE_PAD_V * 2.0 + BADGE_BOTTOM_BORDER
    }
}

impl Widget for KbdBadge {
    fn id(&self) -> WidgetId {
        self.id
    }

    fn sense(&self) -> Sense {
        Sense::none()
    }

    fn layout(&self, ctx: &LayoutCtx<'_>) -> LayoutBox {
        let w = self.badge_width(ctx);
        let h = self.badge_height(ctx);
        LayoutBox::leaf(w, h).with_widget_id(self.id)
    }

    fn paint(&self, ctx: &mut DrawCtx<'_>) {
        let bounds = ctx.bounds;

        // Main badge body.
        let body = Rect::new(
            bounds.x(),
            bounds.y(),
            bounds.width(),
            bounds.height() - BADGE_BOTTOM_BORDER,
        );
        let body_style = RectStyle::filled(ctx.theme.bg_input)
            .with_radius(BADGE_RADIUS)
            .with_border(1.0, ctx.theme.border);
        ctx.scene.push_quad(body, body_style);

        // Bottom border for keycap depth.
        let bottom = Rect::new(
            bounds.x() + 1.0,
            bounds.y() + bounds.height() - BADGE_BOTTOM_BORDER - 1.0,
            bounds.width() - 2.0,
            BADGE_BOTTOM_BORDER,
        );
        let bottom_style = RectStyle::filled(ctx.theme.border);
        ctx.scene.push_quad(bottom, bottom_style);

        // Key text centered in body.
        let text_style = TextStyle::new(BADGE_FONT_SIZE, ctx.theme.fg_primary);
        let shaped = ctx.measurer.shape(&self.key, &text_style, body.width());
        let tx = body.x() + (body.width() - shaped.width) / 2.0;
        let ty = body.y() + BADGE_PAD_V;
        ctx.scene
            .push_text(Point::new(tx, ty), shaped, ctx.theme.fg_primary);
    }
}

/// A keybinding row showing an action name and key badges.
///
/// Left side: action name label. Right side: key badges separated by "+".
/// Hover background highlights the row.
pub struct KeybindRow {
    id: WidgetId,
    action_name: String,
    keys: Vec<String>,
    controllers: Vec<Box<dyn EventController>>,
    animator: VisualStateAnimator,
}

impl KeybindRow {
    /// Creates a keybind row with an action name and key sequence.
    pub fn new(action_name: impl Into<String>, keys: Vec<String>, theme: &UiTheme) -> Self {
        Self {
            id: WidgetId::next(),
            action_name: action_name.into(),
            keys,
            controllers: vec![Box::new(HoverController::new())],
            animator: VisualStateAnimator::new(vec![common_states(
                Color::TRANSPARENT,
                theme.bg_card,
                Color::TRANSPARENT,
                Color::TRANSPARENT,
            )]),
        }
    }

    /// Returns the action name.
    pub fn action_name(&self) -> &str {
        &self.action_name
    }

    /// Returns the key labels.
    pub fn keys(&self) -> &[String] {
        &self.keys
    }

    /// Computes the total width of the badge area.
    fn badges_width(&self, ctx: &LayoutCtx<'_>) -> f32 {
        let text_style = TextStyle::new(BADGE_FONT_SIZE, Color::WHITE);
        let plus_style = TextStyle::new(PLUS_FONT_SIZE, Color::WHITE);
        let plus_w = ctx.measurer.measure("+", &plus_style, f32::INFINITY).width;

        let mut w = 0.0;
        for (i, key) in self.keys.iter().enumerate() {
            if i > 0 {
                w += BADGE_GAP + plus_w + BADGE_GAP;
            }
            let m = ctx.measurer.measure(key, &text_style, f32::INFINITY);
            w += m.width + BADGE_PAD_H * 2.0;
        }
        w
    }
}

impl Widget for KeybindRow {
    fn id(&self) -> WidgetId {
        self.id
    }

    fn sense(&self) -> Sense {
        Sense::hover()
    }

    fn layout(&self, ctx: &LayoutCtx<'_>) -> LayoutBox {
        let action_style = TextStyle::new(ACTION_FONT_SIZE, Color::WHITE);
        let action_w = ctx
            .measurer
            .measure(&self.action_name, &action_style, f32::INFINITY)
            .width;
        let badges_w = self.badges_width(ctx);
        let w = action_w + 24.0 + badges_w + ROW_PAD_H * 2.0;
        LayoutBox::leaf(w, ROW_MIN_HEIGHT)
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
        let bounds = ctx.bounds;

        // Hover background.
        let bg = self.animator.get_bg_color();
        if bg.a > 0.001 {
            let style = RectStyle::filled(bg).with_radius(ROW_RADIUS);
            ctx.scene.push_quad(bounds, style);
        }

        // Action name (left-aligned, vertically centered).
        let action_style = TextStyle::new(ACTION_FONT_SIZE, ctx.theme.fg_primary);
        let shaped = ctx
            .measurer
            .shape(&self.action_name, &action_style, bounds.width());
        let ay = bounds.y() + (bounds.height() - shaped.height) / 2.0;
        ctx.scene.push_text(
            Point::new(bounds.x() + ROW_PAD_H, ay),
            shaped,
            ctx.theme.fg_primary,
        );

        // Key badges (right-aligned).
        let badge_style = TextStyle::new(BADGE_FONT_SIZE, ctx.theme.fg_primary);
        let plus_style = TextStyle::new(PLUS_FONT_SIZE, ctx.theme.fg_faint);
        let badges_w = self.badges_width(&LayoutCtx {
            measurer: ctx.measurer,
            theme: ctx.theme,
        });
        let mut bx = bounds.x() + bounds.width() - ROW_PAD_H - badges_w;
        let by = bounds.y()
            + (bounds.height() - (BADGE_FONT_SIZE + BADGE_PAD_V * 2.0 + BADGE_BOTTOM_BORDER)) / 2.0;

        for (i, key) in self.keys.iter().enumerate() {
            if i > 0 {
                bx += BADGE_GAP;
                let plus_shaped = ctx.measurer.shape("+", &plus_style, 20.0);
                let py = by + BADGE_PAD_V;
                ctx.scene
                    .push_text(Point::new(bx, py), plus_shaped, ctx.theme.fg_faint);
                bx += ctx.measurer.measure("+", &plus_style, f32::INFINITY).width + BADGE_GAP;
            }

            let m = ctx.measurer.measure(key, &badge_style, f32::INFINITY);
            let bw = m.width + BADGE_PAD_H * 2.0;
            let bh = m.height + BADGE_PAD_V * 2.0 + BADGE_BOTTOM_BORDER;

            // Badge body.
            let body = Rect::new(bx, by, bw, bh - BADGE_BOTTOM_BORDER);
            let body_s = RectStyle::filled(ctx.theme.bg_input)
                .with_radius(BADGE_RADIUS)
                .with_border(1.0, ctx.theme.border);
            ctx.scene.push_quad(body, body_s);

            // Bottom border.
            let bottom = Rect::new(
                bx + 1.0,
                by + bh - BADGE_BOTTOM_BORDER - 1.0,
                bw - 2.0,
                BADGE_BOTTOM_BORDER,
            );
            ctx.scene
                .push_quad(bottom, RectStyle::filled(ctx.theme.border));

            // Key text.
            let shaped = ctx.measurer.shape(key, &badge_style, bw);
            let tx = bx + (bw - shaped.width) / 2.0;
            let ty = by + BADGE_PAD_V;
            ctx.scene
                .push_text(Point::new(tx, ty), shaped, ctx.theme.fg_primary);

            bx += bw;
        }

        if self.animator.is_animating() {
            ctx.request_anim_frame();
        }
    }
}

#[cfg(test)]
mod tests;
