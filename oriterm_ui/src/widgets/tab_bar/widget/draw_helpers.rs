//! Tab bar drawing helpers.
//!
//! Separators, action buttons (new-tab, dropdown), icon rendering,
//! bell animation phase, and button-position functions. Extracted from
//! `draw.rs` to keep that file under the 500-line limit.

use std::time::Instant;

use crate::color::Color;
use crate::draw::RectStyle;
use crate::geometry::{Point, Rect};
use crate::icons::IconId;

use super::super::constants::{DROPDOWN_BUTTON_WIDTH, NEW_TAB_BUTTON_WIDTH};
use super::super::hit::TabBarHit;
use super::draw::TabStrip;
use super::{TabBarWidget, TabEntry};

use crate::widgets::DrawCtx;

// Drawing constants (logical pixels).

/// Half-arm length of the + icon in the new-tab button.
const PLUS_ARM: f32 = 5.0;

/// Half-extent of the ▾ chevron in the dropdown button.
const CHEVRON_HALF: f32 = 5.0;

/// Duration of the bell pulse animation in seconds.
const BELL_DURATION_SECS: f32 = 3.0;

/// Frequency of the bell pulse sine wave in Hz.
const BELL_FREQUENCY_HZ: f32 = 2.0;

impl TabBarWidget {
    /// Draws right-edge separators on each tab with suppression rules.
    ///
    /// Each tab gets a 1px right border (full height, no inset) matching
    /// the mockup's `.tab { border-right: 1px solid var(--border) }`.
    pub(super) fn draw_separators(&self, ctx: &mut DrawCtx<'_>, strip: &TabStrip) {
        for i in 0..self.tabs.len() {
            // Suppress the active tab's right separator.
            if i == self.active_index {
                continue;
            }
            // Suppress adjacent to hovered tab.
            if let TabBarHit::Tab(h) | TabBarHit::CloseTab(h) = self.hover_hit {
                if i == h {
                    continue;
                }
            }
            // Suppress adjacent to dragged tab.
            if let Some((d, _)) = self.drag_visual {
                if i == d {
                    continue;
                }
            }

            let right_x = self.layout.tab_x(i) + self.layout.tab_width_at(i);
            ctx.scene.push_line(
                Point::new(right_x, strip.y),
                Point::new(right_x, strip.y + strip.h),
                1.0,
                self.colors.separator,
            );
        }
    }

    /// Draws the new-tab (+) button.
    pub(super) fn draw_new_tab_button(&self, ctx: &mut DrawCtx<'_>, strip: &TabStrip) {
        let bx = new_tab_button_x(self);
        let btn = Rect::new(bx, strip.y, NEW_TAB_BUTTON_WIDTH, strip.h);

        // 1px left border.
        ctx.scene.push_line(
            Point::new(bx, strip.y),
            Point::new(bx, strip.y + strip.h),
            1.0,
            self.colors.bar_border,
        );

        if self.hover_hit == TabBarHit::NewTab {
            ctx.scene
                .push_quad(btn, RectStyle::filled(self.colors.button_hover_bg));
        }

        // + icon.
        let cx = bx + NEW_TAB_BUTTON_WIDTH / 2.0;
        let cy = strip.y + strip.h / 2.0;
        let fg = self.colors.close_fg;
        let icon_size = (PLUS_ARM * 2.0).round() as u32;
        let icon_rect = Rect::new(cx - PLUS_ARM, cy - PLUS_ARM, PLUS_ARM * 2.0, PLUS_ARM * 2.0);
        draw_icon(ctx, IconId::Plus, icon_rect, icon_size, fg);
    }

    /// Draws the dropdown (▾) button.
    pub(super) fn draw_dropdown_button(&self, ctx: &mut DrawCtx<'_>, strip: &TabStrip) {
        let bx = dropdown_button_x(self);
        let btn = Rect::new(bx, strip.y, DROPDOWN_BUTTON_WIDTH, strip.h);

        // 1px left border.
        ctx.scene.push_line(
            Point::new(bx, strip.y),
            Point::new(bx, strip.y + strip.h),
            1.0,
            self.colors.bar_border,
        );

        if self.hover_hit == TabBarHit::Dropdown {
            ctx.scene
                .push_quad(btn, RectStyle::filled(self.colors.button_hover_bg));
        }

        // ▾ chevron.
        let cx = bx + DROPDOWN_BUTTON_WIDTH / 2.0;
        let cy = strip.y + strip.h / 2.0;
        let fg = self.colors.close_fg;
        let icon_size = (CHEVRON_HALF * 2.0).round() as u32;
        let icon_rect = Rect::new(
            cx - CHEVRON_HALF,
            cy - CHEVRON_HALF,
            CHEVRON_HALF * 2.0,
            CHEVRON_HALF * 2.0,
        );
        draw_icon(ctx, IconId::ChevronDown, icon_rect, icon_size, fg);
    }
}

/// Emit a vector icon from the pre-resolved icon atlas.
///
/// No-ops when `ctx.icons` is `None` (tests) or the icon wasn't resolved.
pub(super) fn draw_icon(ctx: &mut DrawCtx<'_>, id: IconId, rect: Rect, size_px: u32, color: Color) {
    if let Some(resolved) = ctx.icons.and_then(|ic| ic.get(id, size_px)) {
        ctx.scene
            .push_icon(rect, resolved.atlas_page, resolved.uv, color);
    }
}

/// Computes the bell animation phase for a tab.
///
/// Returns 0.0–1.0 for an active bell animation, 0.0 otherwise.
/// The phase follows a decaying sine wave that pulses for
/// [`BELL_DURATION_SECS`] seconds after the bell fires.
pub(super) fn bell_phase(tab: &TabEntry, now: Instant) -> f32 {
    let Some(start) = tab.bell_start else {
        return 0.0;
    };
    let elapsed = now.duration_since(start).as_secs_f32();
    if elapsed >= BELL_DURATION_SECS {
        return 0.0;
    }
    let fade = 1.0 - (elapsed / BELL_DURATION_SECS);
    let wave = (elapsed * BELL_FREQUENCY_HZ * std::f32::consts::TAU)
        .sin()
        .abs();
    wave * fade
}

/// X position of the new-tab button, adjusted for drag.
///
/// When dragging a tab past the end of the strip, the button moves
/// right to stay visible: `max(default_x, drag_x + tab_width)`.
pub(super) fn new_tab_button_x(widget: &TabBarWidget) -> f32 {
    let default_x = widget.layout.new_tab_x();
    if let Some((idx, drag_x)) = widget.drag_visual {
        default_x.max(drag_x + widget.layout.tab_width_at(idx))
    } else {
        default_x
    }
}

/// X position of the dropdown button, adjusted for drag.
pub(super) fn dropdown_button_x(widget: &TabBarWidget) -> f32 {
    let default_x = widget.layout.dropdown_x();
    if let Some((idx, drag_x)) = widget.drag_visual {
        default_x.max(drag_x + widget.layout.tab_width_at(idx) + NEW_TAB_BUTTON_WIDTH)
    } else {
        default_x
    }
}
