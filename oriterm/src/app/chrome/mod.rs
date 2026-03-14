//! Window chrome: action dispatch, platform chrome lifecycle, and layout helpers.
//!
//! Handles `WidgetAction::WindowMinimize`, `WindowMaximize`, and
//! `WindowClose` by forwarding to the appropriate winit window operations.
//! Provides unified chrome installation and refresh functions that route
//! through [`NativeChromeOps`] for cross-platform support. Provides
//! [`compute_window_layout`] — the single source of truth for top-level
//! window layout (tab bar + terminal grid positioning) via the layout engine.

mod resize;

use std::time::Instant;

#[cfg(not(target_os = "macos"))]
use winit::event_loop::ActiveEventLoop;
use winit::window::Window;

use oriterm_ui::geometry::Rect;
#[cfg(not(target_os = "macos"))]
use oriterm_ui::widgets::WidgetAction;
use oriterm_ui::widgets::window_chrome::constants::RESIZE_BORDER_WIDTH;

use super::App;
#[cfg(not(target_os = "macos"))]
use crate::font::{CachedTextMeasurer, UiFontMeasurer};
use crate::window_manager::platform::{ChromeMode, chrome_ops};

/// Install frameless window chrome via the platform trait.
///
/// Installs the OS-level subclass/hooks for hit testing (resize borders,
/// caption drag, window controls) and sets initial interactive rects.
/// On macOS/Linux, the platform impl is a no-op.
///
/// `rects` are in logical pixels, `caption_height` is in logical pixels.
/// `scale` is the display scale factor for physical pixel conversion.
pub(super) fn install_chrome(
    window: &Window,
    mode: ChromeMode,
    rects: &[Rect],
    caption_height: f32,
    scale: f32,
) {
    let ops = chrome_ops();
    let border_width = match mode {
        ChromeMode::Main => RESIZE_BORDER_WIDTH * scale,
        ChromeMode::Dialog { resizable } => {
            if resizable {
                RESIZE_BORDER_WIDTH * scale
            } else {
                0.0
            }
        }
    };
    ops.install_chrome(window, mode, border_width, caption_height * scale);
    ops.set_interactive_rects(window, rects, scale);
}

/// Refresh platform hit test rects and chrome metrics.
///
/// Updates both the interactive regions and the border/caption dimensions
/// at the OS level. Called after resize, DPI change, tab add/remove, or
/// any other event that changes chrome layout.
///
/// `rects` are in logical pixels, `caption_height` is in logical pixels.
/// `scale` is the display scale factor for physical pixel conversion.
pub(super) fn refresh_chrome(
    window: &Window,
    rects: &[Rect],
    caption_height: f32,
    scale: f32,
    resizable: bool,
) {
    let ops = chrome_ops();
    ops.set_interactive_rects(window, rects, scale);
    let border_width = if resizable {
        RESIZE_BORDER_WIDTH * scale
    } else {
        0.0
    };
    ops.set_chrome_metrics(window, border_width, caption_height * scale);
}

/// Compute the grid origin y-coordinate in physical pixels.
///
/// Rounds to an integer pixel to prevent fractional origins that cause
/// visible seams between block character rows on the GPU. Without rounding,
/// DPI scale factors like 1.25 produce half-pixel boundaries
/// (e.g. `82.0 * 1.25 = 102.5`) that mis-align cell rows.
pub(super) fn grid_origin_y(chrome_height_logical: f32, scale: f32) -> f32 {
    (chrome_height_logical * scale).round()
}

/// Logical padding around the terminal grid (in logical pixels).
///
/// Prevents text from touching the window edge. Scaled to physical pixels
/// and rounded to integer pixels during layout computation.
const GRID_PADDING: f32 = 8.0;

/// Computed top-level window layout: chrome and terminal grid positions.
pub(super) struct WindowLayout {
    /// Grid bounds in physical pixels (origin + dimensions), inset by padding.
    pub grid_rect: Rect,
    /// Number of terminal columns that fit in the grid area.
    pub cols: usize,
    /// Number of terminal rows that fit in the grid area.
    pub rows: usize,
}

/// Compute the top-level window layout via the layout engine.
///
/// Builds a `Column { TabBar(fixed), Grid(fill) }` descriptor and runs the
/// two-pass flexbox solver to determine positions. The tab bar gets a fixed
/// height (logical `TAB_BAR_HEIGHT` scaled to physical pixels, rounded to
/// prevent subpixel seams). The terminal grid fills the remaining space.
///
/// All coordinates are in physical pixels — consistent with cell metrics,
/// GPU renderer, and the winit viewport.
pub(super) fn compute_window_layout(
    viewport_w: u32,
    viewport_h: u32,
    cell: &crate::font::CellMetrics,
    scale: f32,
) -> WindowLayout {
    use oriterm_ui::layout::{Direction, LayoutBox, SizeSpec, compute_layout};

    let tab_bar_h_px = grid_origin_y(
        oriterm_ui::widgets::tab_bar::constants::TAB_BAR_HEIGHT,
        scale,
    );

    let root = LayoutBox::flex(
        Direction::Column,
        vec![
            // Tab bar: fixed height in physical pixels, fills width.
            LayoutBox::leaf(viewport_w as f32, tab_bar_h_px)
                .with_width(SizeSpec::Fill)
                .with_height(SizeSpec::Fixed(tab_bar_h_px)),
            // Terminal grid: fills remaining space.
            LayoutBox::leaf(0.0, 0.0)
                .with_width(SizeSpec::Fill)
                .with_height(SizeSpec::Fill),
        ],
    )
    .with_width(SizeSpec::Fill)
    .with_height(SizeSpec::Fill);

    let viewport = Rect::new(0.0, 0.0, viewport_w as f32, viewport_h as f32);
    let layout = compute_layout(&root, viewport);
    let raw_grid = layout.children[1].rect;

    // Padding in physical pixels, rounded to integer to prevent subpixel
    // seams. Applied as a left/top origin shift on the grid rect.
    let pad = (GRID_PADDING * scale).round();

    // Compute grid dimensions from the visible area after padding.
    // The grid origin is shifted right by `pad`, so the renderable width
    // for cells is `raw_width - pad`. This must match the WM_SIZING snap
    // formula (`snapped_w = cols * cell_w + pad`) so the column count is
    // stable during interactive resize.
    let grid_pixel_w = (raw_grid.width() - pad).max(0.0) as u32;
    let grid_pixel_h = (raw_grid.height() - pad).max(0.0) as u32;
    let cols = cell.columns(grid_pixel_w).max(1);
    let rows = cell.rows(grid_pixel_h).max(1);
    let grid_rect = Rect::new(
        raw_grid.x() + pad,
        raw_grid.y() + pad,
        raw_grid.width(),
        raw_grid.height(),
    );

    WindowLayout {
        grid_rect,
        cols,
        rows,
    }
}

impl App {
    /// Dispatch a window chrome action to the corresponding window operation.
    ///
    /// Returns `true` if the action was handled (recognized as a chrome action).
    /// On macOS, native traffic lights handle these actions directly.
    #[cfg(not(target_os = "macos"))]
    pub(super) fn handle_chrome_action(
        &mut self,
        action: &WidgetAction,
        event_loop: &ActiveEventLoop,
    ) -> bool {
        match action {
            WidgetAction::WindowMinimize => {
                if let Some(ctx) = self.focused_ctx() {
                    ctx.window.window().set_minimized(true);
                }
                true
            }
            WidgetAction::WindowMaximize => {
                self.toggle_maximize();
                true
            }
            WidgetAction::WindowClose => {
                if let Some(wid) = self.focused_window_id {
                    self.close_window(wid, event_loop);
                }
                true
            }
            _ => false,
        }
    }

    /// Toggle the window between maximized and restored state.
    ///
    /// Updates the winit window, the `TermWindow` state, and the chrome
    /// widget's maximized flag.
    pub(super) fn toggle_maximize(&mut self) {
        if let Some(ctx) = self.focused_ctx_mut() {
            let maximized = !ctx.window.is_maximized();
            ctx.window.window().set_maximized(maximized);
            ctx.window.set_maximized(maximized);
            #[cfg(not(target_os = "macos"))]
            ctx.tab_bar.set_maximized(maximized);
            ctx.dirty = true;
        }
    }

    /// Returns `true` if the cursor position is within the tab bar zone.
    ///
    /// The tab bar spans from y=0 to `TAB_BAR_HEIGHT` (logical pixels).
    pub(super) fn cursor_in_tab_bar(&self, position: winit::dpi::PhysicalPosition<f64>) -> bool {
        let Some(ctx) = self.focused_ctx() else {
            return false;
        };
        let scale = ctx.window.scale_factor().factor() as f32;
        let logical_y = position.y as f32 / scale;
        logical_y < oriterm_ui::widgets::tab_bar::constants::TAB_BAR_HEIGHT
    }

    /// Update tab bar hover state and width lock from cursor position.
    ///
    /// Called from `CursorMoved`. Computes which tab bar element the cursor
    /// targets via [`hit_test`](oriterm_ui::widgets::tab_bar::hit_test),
    /// updates the widget's hover hit (marking dirty on change), and manages
    /// the tab width lock (acquire on enter, release on leave).
    pub(super) fn update_tab_bar_hover(&mut self, position: winit::dpi::PhysicalPosition<f64>) {
        let in_tab_bar = self.cursor_in_tab_bar(position);
        let locked = self.tab_width_lock().is_some();

        // Manage tab width lock. Skip when a tab drag is active — the drag
        // owns the lock lifecycle and cursor movement outside the bar (toward
        // tear-off) must not release it prematurely.
        if !self.has_tab_drag() {
            match (in_tab_bar, locked) {
                (true, false) => {
                    let tab_width = self
                        .focused_ctx()
                        .map_or(0.0, |ctx| ctx.tab_bar.layout().base_tab_width());
                    self.acquire_tab_width_lock(tab_width);
                }
                (false, true) => self.release_tab_width_lock(),
                (true, true) | (false, false) => {}
            }
        }

        // Compute hit test result.
        let hit = if in_tab_bar {
            let ctx_data = self.focused_ctx().map(|ctx| {
                (
                    ctx.window.scale_factor().factor() as f32,
                    ctx.tab_bar.layout().clone(),
                )
            });

            match ctx_data {
                Some((scale, layout)) => {
                    let x = position.x as f32 / scale;
                    let y = position.y as f32 / scale;
                    oriterm_ui::widgets::tab_bar::hit_test(x, y, &layout)
                }
                _ => oriterm_ui::widgets::tab_bar::TabBarHit::None,
            }
        } else {
            oriterm_ui::widgets::tab_bar::TabBarHit::None
        };

        // Drive control button hover animation when cursor targets controls.
        #[cfg(not(target_os = "macos"))]
        self.update_control_hover_animation(position, &hit);

        // Apply hover hit, redraw on change.
        if let Some(ctx) = self.focused_ctx_mut() {
            if ctx.tab_bar.hover_hit() != hit {
                ctx.tab_bar.set_hover_hit(hit, Instant::now());
                ctx.dirty = true;
                ctx.ui_stale = true;
            }
        }
    }

    /// Drive control button hover animation for the focused window.
    ///
    /// Forwards the cursor position and hit result to the tab bar's
    /// control hover handler, which manages fade-in/fade-out animations
    /// on minimize, maximize, and close buttons.
    #[cfg(not(target_os = "macos"))]
    fn update_control_hover_animation(
        &mut self,
        position: winit::dpi::PhysicalPosition<f64>,
        hit: &oriterm_ui::widgets::tab_bar::TabBarHit,
    ) {
        use oriterm_ui::widgets::tab_bar::TabBarHit;
        let is_control_hit = matches!(
            hit,
            TabBarHit::Minimize | TabBarHit::Maximize | TabBarHit::CloseWindow
        );

        let Some(ctx) = self
            .focused_window_id
            .and_then(|id| self.windows.get_mut(&id))
        else {
            return;
        };
        if !is_control_hit && ctx.tab_bar.hover_hit() == *hit {
            return;
        }
        let scale = ctx.window.scale_factor().factor() as f32;
        let pos =
            oriterm_ui::geometry::Point::new(position.x as f32 / scale, position.y as f32 / scale);
        let Some(renderer) = ctx.renderer.as_ref() else {
            return;
        };
        let measurer = CachedTextMeasurer::new(
            UiFontMeasurer::new(renderer.active_ui_collection(), scale),
            &ctx.text_cache,
            scale,
        );
        let event_ctx = oriterm_ui::widgets::EventCtx {
            measurer: &measurer,
            bounds: Rect::default(),
            is_focused: false,
            focused_widget: None,
            theme: &self.ui_theme,
        };
        let resp = ctx.tab_bar.update_control_hover(pos, &event_ctx);
        if matches!(
            resp.response,
            oriterm_ui::input::EventResponse::RequestPaint
                | oriterm_ui::input::EventResponse::RequestLayout
        ) {
            resp.mark_tracker(&mut ctx.invalidation);
            ctx.dirty = true;
            ctx.ui_stale = true;
        }
    }

    /// Clear tab bar hover state (including control button hover).
    ///
    /// Called when the cursor leaves the window to reset hover highlighting.
    pub(super) fn clear_tab_bar_hover(&mut self) {
        let Some(ctx) = self
            .focused_window_id
            .and_then(|id| self.windows.get_mut(&id))
        else {
            return;
        };
        let had_hover = ctx.tab_bar.hover_hit() != oriterm_ui::widgets::tab_bar::TabBarHit::None;
        if had_hover {
            ctx.tab_bar.set_hover_hit(
                oriterm_ui::widgets::tab_bar::TabBarHit::None,
                Instant::now(),
            );
        }
        // Clear control button hover animation (not on macOS — native traffic lights).
        #[cfg(not(target_os = "macos"))]
        if let Some(renderer) = ctx.renderer.as_ref() {
            let scale = ctx.window.scale_factor().factor() as f32;
            let measurer = CachedTextMeasurer::new(
                UiFontMeasurer::new(renderer.active_ui_collection(), scale),
                &ctx.text_cache,
                scale,
            );
            let event_ctx = oriterm_ui::widgets::EventCtx {
                measurer: &measurer,
                bounds: Rect::default(),
                is_focused: false,
                focused_widget: None,
                theme: &self.ui_theme,
            };
            ctx.tab_bar.clear_control_hover(&event_ctx);
        }
        if had_hover {
            ctx.dirty = true;
            ctx.ui_stale = true;
        }
    }
}

#[cfg(test)]
mod tests;
