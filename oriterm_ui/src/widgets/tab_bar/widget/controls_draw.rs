//! Window control button drawing for the tab bar.
//!
//! Renders minimize, maximize/restore, and close buttons inside the tab
//! bar's reserved controls zone. Extracted from `draw.rs` to keep that
//! file under the 500-line limit.
//!
//! Gated to non-macOS platforms: macOS uses native traffic light buttons
//! provided by `fullsize_content_view(true)`.

#[cfg(not(target_os = "macos"))]
use crate::geometry::Rect;
#[cfg(not(target_os = "macos"))]
use crate::widgets::{DrawCtx, Widget};

#[cfg(not(target_os = "macos"))]
use super::super::constants::{CONTROLS_ZONE_WIDTH, TAB_BAR_HEIGHT};
#[cfg(not(target_os = "macos"))]
use super::TabBarWidget;

/// Width of each control button (zone width divided equally among 3 buttons).
///
/// On Windows this equals `CONTROL_BUTTON_WIDTH` (46px); on Linux
/// the zone is smaller (100px) so buttons are ~33px each.
#[cfg(not(target_os = "macos"))]
const BUTTON_WIDTH: f32 = CONTROLS_ZONE_WIDTH / 3.0;

#[cfg(not(target_os = "macos"))]
impl TabBarWidget {
    /// Draws the window control buttons (minimize, maximize/restore, close).
    ///
    /// Called from [`Widget::draw`] after the dropdown button and before the
    /// dragged tab overlay. Each button is drawn by delegating to its
    /// [`WindowControlButton::draw`](crate::widgets::window_chrome::controls::WindowControlButton::draw)
    /// with bounds computed from the tab bar's controls zone.
    pub(super) fn draw_window_controls(&self, ctx: &mut DrawCtx<'_>) {
        let controls_x = self.layout.controls_x();
        let y0 = ctx.bounds.y();

        for (i, ctrl) in self.controls.iter().enumerate() {
            let btn_rect = Rect::new(
                controls_x + i as f32 * BUTTON_WIDTH,
                y0,
                BUTTON_WIDTH,
                TAB_BAR_HEIGHT,
            );
            let mut child_ctx = DrawCtx {
                measurer: ctx.measurer,
                draw_list: ctx.draw_list,
                bounds: btn_rect,
                now: ctx.now,
                theme: ctx.theme,
                icons: ctx.icons,
                scene_cache: ctx.scene_cache.as_deref_mut(),
                interaction: None,
                widget_id: None,
                frame_requests: None,
            };
            ctrl.paint(&mut child_ctx);
        }
    }

    /// Returns the bounding rectangle for the control button at `index`.
    ///
    /// Used by [`dispatch_control_input`](Self::dispatch_control_input) and
    /// [`interactive_rects`](Self::interactive_rects) to determine control
    /// button positions without duplicating geometry logic.
    pub(super) fn control_rect(&self, index: usize) -> Rect {
        let controls_x = self.layout.controls_x();
        Rect::new(
            controls_x + index as f32 * BUTTON_WIDTH,
            0.0,
            BUTTON_WIDTH,
            TAB_BAR_HEIGHT,
        )
    }
}
