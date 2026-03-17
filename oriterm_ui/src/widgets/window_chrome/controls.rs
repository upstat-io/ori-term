//! Window control button widget (minimize, maximize/restore, close).
//!
//! Each button renders its symbol geometrically (lines for -, rect outline
//! for square, X lines for x) -- no font glyphs needed. Uses
//! [`VisualStateAnimator`] with `common_states()` for smooth 100ms color
//! transitions, matching the [`ButtonWidget`](super::super::button::ButtonWidget)
//! pattern.

use std::time::Instant;

use crate::animation::Lerp;
use crate::color::Color;
use crate::controllers::{ClickController, EventController, HoverController};
use crate::draw::RectStyle;
use crate::geometry::Rect;
use crate::layout::LayoutBox;
use crate::sense::Sense;
use crate::visual_state::common_states;
use crate::visual_state::transition::VisualStateAnimator;
use crate::widget_id::WidgetId;

use crate::icons::IconId;

use super::super::{DrawCtx, LayoutCtx, Widget};
use super::constants::{CONTROL_BUTTON_WIDTH, SYMBOL_SIZE};
use super::layout::ControlKind;

/// Colors for a window control button.
///
/// Bundled into a struct to avoid excessive constructor parameters.
#[derive(Debug, Clone, Copy)]
pub struct ControlButtonColors {
    /// Normal foreground (symbol stroke).
    pub fg: Color,
    /// Normal background (transparent when unhovered).
    pub bg: Color,
    /// Hover background for non-close buttons.
    pub hover_bg: Color,
    /// Close button hover background (platform-standard red).
    pub close_hover_bg: Color,
    /// Close button pressed background (darker red).
    pub close_pressed_bg: Color,
}

/// A window control button: minimize, maximize/restore, or close.
///
/// Renders geometric symbols (no font dependency) with animated hover
/// transitions via [`VisualStateAnimator`]. Emits `WidgetAction::WindowMinimize`,
/// `WindowMaximize`, or `WindowClose` when clicked.
pub struct WindowControlButton {
    id: WidgetId,
    kind: ControlKind,
    /// Whether the window is currently maximized (affects the maximize
    /// button symbol: square vs overlapping squares).
    is_maximized: bool,
    /// Normal foreground (symbol) color.
    fg: Color,
    /// Close button hover background (from theme).
    close_hover_bg: Color,
    /// Close button pressed background (from theme).
    close_pressed_bg: Color,
    controllers: Vec<Box<dyn EventController>>,
    animator: VisualStateAnimator,
}

impl WindowControlButton {
    /// Creates a new control button of the given kind.
    pub fn new(kind: ControlKind, colors: ControlButtonColors) -> Self {
        let (hover_bg, pressed_bg) = if kind == ControlKind::Close {
            (colors.close_hover_bg, colors.close_pressed_bg)
        } else {
            (colors.hover_bg, colors.bg)
        };

        Self {
            id: WidgetId::next(),
            kind,
            is_maximized: false,
            fg: colors.fg,
            close_hover_bg: colors.close_hover_bg,
            close_pressed_bg: colors.close_pressed_bg,
            controllers: vec![
                Box::new(HoverController::new()),
                Box::new(ClickController::new()),
            ],
            animator: VisualStateAnimator::new(vec![common_states(
                colors.bg, hover_bg, pressed_bg, colors.bg,
            )]),
        }
    }

    /// Returns this button's kind.
    pub fn kind(&self) -> ControlKind {
        self.kind
    }

    /// Updates the maximized state (affects maximize/restore symbol).
    pub fn set_maximized(&mut self, maximized: bool) {
        self.is_maximized = maximized;
    }

    /// Updates the button colors from a new theme palette.
    pub fn set_colors(&mut self, colors: ControlButtonColors) {
        self.fg = colors.fg;
        self.close_hover_bg = colors.close_hover_bg;
        self.close_pressed_bg = colors.close_pressed_bg;

        let (hover_bg, pressed_bg) = if self.kind == ControlKind::Close {
            (colors.close_hover_bg, colors.close_pressed_bg)
        } else {
            (colors.hover_bg, colors.bg)
        };

        self.animator = VisualStateAnimator::new(vec![common_states(
            colors.bg, hover_bg, pressed_bg, colors.bg,
        )]);
    }

    /// Returns the foreground (symbol) color -- white on close hover.
    fn current_fg(&self, now: Instant) -> Color {
        if self.kind == ControlKind::Close {
            // Use animator bg to derive hover progress: if bg matches
            // close_hover_bg or close_pressed_bg, use white fg.
            let bg = self.animator.get_bg_color(now);
            if bg == Color::TRANSPARENT {
                self.fg
            } else {
                // Approximate: lerp fg toward white based on bg alpha.
                let t = bg.a;
                Color::lerp(self.fg, Color::WHITE, t)
            }
        } else {
            self.fg
        }
    }
}

impl std::fmt::Debug for WindowControlButton {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WindowControlButton")
            .field("id", &self.id)
            .field("kind", &self.kind)
            .field("is_maximized", &self.is_maximized)
            .field("fg", &self.fg)
            .field("close_hover_bg", &self.close_hover_bg)
            .field("close_pressed_bg", &self.close_pressed_bg)
            .field("controller_count", &self.controllers.len())
            .field("animator", &self.animator)
            .finish()
    }
}

/// Draw the minimize symbol: a horizontal dash centered in the button.
fn draw_minimize(ctx: &mut DrawCtx<'_>, bounds: Rect, fg: Color) {
    let cx = bounds.x() + bounds.width() / 2.0;
    let cy = bounds.y() + bounds.height() / 2.0;
    let half = SYMBOL_SIZE / 2.0;
    let icon_size = SYMBOL_SIZE.round() as u32;
    if let Some(resolved) = ctx.icons.and_then(|ic| ic.get(IconId::Minimize, icon_size)) {
        let icon_rect = Rect::new(cx - half, cy - half, SYMBOL_SIZE, SYMBOL_SIZE);
        ctx.draw_list
            .push_icon(icon_rect, resolved.atlas_page, resolved.uv, fg);
    }
}

/// Draw the maximize symbol: a square outline centered in the button.
fn draw_maximize(ctx: &mut DrawCtx<'_>, bounds: Rect, fg: Color) {
    let cx = bounds.x() + bounds.width() / 2.0;
    let cy = bounds.y() + bounds.height() / 2.0;
    let half = SYMBOL_SIZE / 2.0;
    let icon_size = SYMBOL_SIZE.round() as u32;
    if let Some(resolved) = ctx.icons.and_then(|ic| ic.get(IconId::Maximize, icon_size)) {
        let icon_rect = Rect::new(cx - half, cy - half, SYMBOL_SIZE, SYMBOL_SIZE);
        ctx.draw_list
            .push_icon(icon_rect, resolved.atlas_page, resolved.uv, fg);
    }
}

/// Draw the restore symbol: two overlapping square outlines.
fn draw_restore(ctx: &mut DrawCtx<'_>, bounds: Rect, fg: Color) {
    let cx = bounds.x() + bounds.width() / 2.0;
    let cy = bounds.y() + bounds.height() / 2.0;
    let half = SYMBOL_SIZE / 2.0;
    let icon_size = SYMBOL_SIZE.round() as u32;
    if let Some(resolved) = ctx.icons.and_then(|ic| ic.get(IconId::Restore, icon_size)) {
        let icon_rect = Rect::new(cx - half, cy - half, SYMBOL_SIZE, SYMBOL_SIZE);
        ctx.draw_list
            .push_icon(icon_rect, resolved.atlas_page, resolved.uv, fg);
    }
}

/// Draw the close symbol: an X centered in the button.
fn draw_close(ctx: &mut DrawCtx<'_>, bounds: Rect, fg: Color) {
    let cx = bounds.x() + bounds.width() / 2.0;
    let cy = bounds.y() + bounds.height() / 2.0;
    let half = SYMBOL_SIZE / 2.0;
    let icon_size = SYMBOL_SIZE.round() as u32;
    if let Some(resolved) = ctx
        .icons
        .and_then(|ic| ic.get(IconId::WindowClose, icon_size))
    {
        let icon_rect = Rect::new(cx - half, cy - half, SYMBOL_SIZE, SYMBOL_SIZE);
        ctx.draw_list
            .push_icon(icon_rect, resolved.atlas_page, resolved.uv, fg);
    }
}

impl Widget for WindowControlButton {
    fn id(&self) -> WidgetId {
        self.id
    }

    fn is_focusable(&self) -> bool {
        false
    }

    fn sense(&self) -> Sense {
        Sense::click()
    }

    fn layout(&self, _ctx: &LayoutCtx<'_>) -> LayoutBox {
        LayoutBox::leaf(CONTROL_BUTTON_WIDTH, 0.0).with_widget_id(self.id)
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
        let bg = self.animator.get_bg_color(ctx.now);
        let fg = self.current_fg(ctx.now);

        // Button background (only visible on hover/press).
        if bg != Color::TRANSPARENT {
            let style = RectStyle::filled(bg);
            ctx.draw_list.push_rect(ctx.bounds, style);
        }

        // Symbol glyph.
        match self.kind {
            ControlKind::Minimize => draw_minimize(ctx, ctx.bounds, fg),
            ControlKind::MaximizeRestore => {
                if self.is_maximized {
                    draw_restore(ctx, ctx.bounds, fg);
                } else {
                    draw_maximize(ctx, ctx.bounds, fg);
                }
            }
            ControlKind::Close => draw_close(ctx, ctx.bounds, fg),
        }

        // Request continued redraws during animation.
        if self.animator.is_animating(ctx.now) {
            ctx.request_anim_frame();
        }
    }
}
