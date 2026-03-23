//! Dragged tab overlay drawing.
//!
//! Renders the floating dragged tab with an opaque backing rect, rounded
//! active-style background, tab label, and close icon. Extracted from
//! `draw.rs` to keep that file under the 500-line limit.

use crate::draw::{RectStyle, Shadow};
use crate::geometry::Rect;
use crate::icons::IconId;

use super::TabBarWidget;
use super::draw::{ACTIVE_TAB_RADIUS, CLOSE_ICON_INSET, TabStrip};

use super::super::constants::{CLOSE_BUTTON_RIGHT_PAD, CLOSE_BUTTON_WIDTH};
use crate::widgets::DrawCtx;

impl TabBarWidget {
    /// Draws the dragged tab as a floating overlay.
    ///
    /// Called separately from the main tab bar pass. The dragged tab is
    /// excluded from normal rendering and drawn here with an opaque backing
    /// rect so it floats above everything.
    pub(super) fn draw_dragged_tab_overlay(&self, ctx: &mut DrawCtx<'_>, strip: &TabStrip) {
        let Some((index, visual_x)) = self.drag_visual else {
            return;
        };
        if index >= self.tabs.len() {
            return;
        }

        let tab = &self.tabs[index];
        let w = self.layout.tab_width_at(index);

        // Rounded tab shape with active background and drop shadow.
        let tab_rect = Rect::new(visual_x, strip.y, w, strip.h);
        let shadow = Shadow {
            offset_x: 0.0,
            offset_y: 2.0,
            blur_radius: 8.0,
            spread: 0.0,
            color: ctx.theme.shadow,
        };
        let style = RectStyle::filled(self.colors.active_bg)
            .with_per_corner_radius(ACTIVE_TAB_RADIUS, ACTIVE_TAB_RADIUS, 0.0, 0.0)
            .with_shadow(shadow);
        ctx.scene.push_layer_bg(self.colors.active_bg);
        ctx.scene.push_quad(tab_rect, style);

        self.draw_tab_label(ctx, tab, visual_x, strip);

        // Close button (always visible on dragged tab).
        self.draw_close_icon(ctx, index, visual_x, strip);

        ctx.scene.pop_layer_bg();
    }

    /// Draws the × icon at a given tab X position (no hover — for drag overlay).
    fn draw_close_icon(&self, ctx: &mut DrawCtx<'_>, index: usize, tab_x: f32, strip: &TabStrip) {
        let cx =
            tab_x + self.layout.tab_width_at(index) - CLOSE_BUTTON_RIGHT_PAD - CLOSE_BUTTON_WIDTH;
        let cy = strip.y + (strip.h - CLOSE_BUTTON_WIDTH) / 2.0;
        let fg = self.colors.close_fg;

        let icon_size = (CLOSE_BUTTON_WIDTH - 2.0 * CLOSE_ICON_INSET).round() as u32;
        if let Some(resolved) = ctx.icons.and_then(|ic| ic.get(IconId::Close, icon_size)) {
            let icon_rect = Rect::new(
                cx + CLOSE_ICON_INSET,
                cy + CLOSE_ICON_INSET,
                CLOSE_BUTTON_WIDTH - 2.0 * CLOSE_ICON_INSET,
                CLOSE_BUTTON_WIDTH - 2.0 * CLOSE_ICON_INSET,
            );
            ctx.scene
                .push_icon(icon_rect, resolved.atlas_page, resolved.uv, fg);
        }
    }
}
