//! Three-phase rendering pipeline: Extract → Prepare → Render.

mod draw_helpers;
mod multi_pane;
mod post_render;
pub(in crate::app) mod preedit;
mod search_bar;

use std::time::Instant;

use super::App;
use super::mouse_selection::{self, GridCtx};
use super::perf_stats::FramePhases;
use crate::gpu::{
    FrameSearch, FrameSelection, MarkCursorOverride, ViewportSize, extract_frame_from_snapshot,
    extract_frame_from_snapshot_into, snapshot_palette,
};
use oriterm_core::{Column, CursorShape, TermMode};

impl App {
    /// Execute the three-phase rendering pipeline: Extract → Prepare → Render.
    #[expect(
        clippy::too_many_lines,
        reason = "linear three-phase pipeline: Extract → Prepare → Render"
    )]
    pub(super) fn handle_redraw(&mut self) -> FramePhases {
        log::trace!("RedrawRequested");
        let mut phases = FramePhases::default();

        if let Some(ctx) = self.focused_ctx_mut() {
            ctx.root.set_urgent_redraw(false);
        }

        // Compute URL hover segments before the render block (which borrows
        // ctx.renderer mutably). Take the Vec from the previous frame to
        // reuse its capacity, avoiding a per-frame allocation.
        let mut url_segments = self
            .focused_ctx_mut()
            .and_then(|ctx| ctx.frame.as_mut())
            .map_or_else(Vec::new, |f| std::mem::take(&mut f.hovered_url_segments));
        self.fill_hovered_url_viewport_segments(&mut url_segments);

        // Multi-pane check: if the active tab has splits, dispatch to the
        // multi-pane renderer which iterates all panes in one GPU frame.
        if let Some((layouts, dividers)) = self.compute_pane_layouts() {
            self.handle_redraw_multi_pane(&layouts, &dividers, url_segments);
            return phases;
        }

        // Resolve pane ID before the render block: `active_pane_id()` borrows
        // `&self`, which conflicts with the `&mut ctx.renderer` inside the
        // block. `PaneId` is `Copy`, so the borrow ends here.
        let Some(pane_id) = self.active_pane_id() else {
            log::warn!("redraw: no active pane");
            return phases;
        };

        // Copy selection before the render block (where ctx.renderer is
        // mutably borrowed, preventing immutable self borrows).
        let pane_sel = self.pane_selection(pane_id).copied();
        let pane_mc = self.pane_mark_cursor(pane_id);

        let (render_result, blinking_now, cursor_pos) = {
            let Some(gpu) = self.gpu.as_ref() else {
                log::warn!("redraw: no gpu");
                return phases;
            };
            let Some(pipelines) = self.pipelines.as_ref() else {
                log::warn!("redraw: no pipelines");
                return phases;
            };
            let Some(ctx) = self
                .focused_window_id
                .and_then(|id| self.windows.get_mut(&id))
            else {
                log::warn!("redraw: no window");
                return phases;
            };
            let Some(renderer) = ctx.renderer.as_mut() else {
                log::warn!("redraw: no renderer");
                return phases;
            };
            if !ctx.window.has_surface_area() {
                log::warn!("redraw: no surface area");
                return phases;
            }

            let (w, h) = ctx.window.size_px();
            let viewport = ViewportSize::new(w, h);
            let cell = renderer.cell_metrics();
            let Some(mux) = self.mux.as_mut() else {
                return phases;
            };

            // Extract phase: refresh snapshot if needed.
            // Detect tab switch / tear-off: when the rendered pane changes,
            // force a refresh to flush stale `renderable_cache` entries left
            // by the previous `swap_renderable_content` cycle.
            let extract_start = Instant::now();
            let pane_changed = ctx.last_rendered_pane != Some(pane_id);
            ctx.last_rendered_pane = Some(pane_id);
            let snap_is_none = mux.pane_snapshot(pane_id).is_none();
            let snap_dirty = mux.is_pane_snapshot_dirty(pane_id);
            let content_changed = snap_is_none || snap_dirty || pane_changed;
            if content_changed {
                mux.refresh_pane_snapshot(pane_id);
            }

            // Fast path (embedded): swap RenderableContent directly from
            // the terminal, bypassing the WireCell round-trip. Only attempt
            // when content was refreshed — stale cache entries from prior
            // tab switches would contaminate the frame otherwise.
            let swapped = content_changed
                && ctx
                    .frame
                    .as_mut()
                    .is_some_and(|f| mux.swap_renderable_content(pane_id, &mut f.content));

            let Some(snapshot) = mux.pane_snapshot(pane_id) else {
                log::warn!("redraw: no snapshot for pane {pane_id:?}");
                ctx.root.mark_dirty();
                return phases;
            };
            if swapped {
                let frame = ctx.frame.as_mut().expect("frame exists when swapped");
                frame.viewport = viewport;
                frame.cell_size = cell;
                frame.content_cols = snapshot.cols as usize;
                frame.content_rows = snapshot.cells.len();
                frame.palette = snapshot_palette(snapshot);
                frame.selection = None;
                frame.search = None;
                frame.hovered_cell = None;
                frame.hovered_url_segments.clear();
                frame.mark_cursor = None;
                frame.fg_dim = 1.0;
                frame.prompt_marker_rows.clear();
            } else if content_changed || ctx.frame.is_none() {
                // Only re-extract when content actually changed. On cursor-
                // blink-only redraws the existing frame is still valid — skip
                // the O(rows*cols) snapshot-to-renderable copy.
                match &mut ctx.frame {
                    Some(existing) => {
                        extract_frame_from_snapshot_into(snapshot, existing, viewport, cell);
                    }
                    slot @ None => {
                        *slot = Some(extract_frame_from_snapshot(snapshot, viewport, cell));
                    }
                }
            } else {
                // Cursor-blink-only: reuse existing frame as-is.
            }
            mux.clear_pane_snapshot_dirty(pane_id);
            phases.extract = extract_start.elapsed();

            let frame = ctx.frame.as_mut().expect("frame just assigned");

            // Set window opacity from config, accounting for focus state.
            // Unfocused windows use the dimmer unfocused_opacity value.
            // Force 1.0 when the surface doesn't support alpha (Vulkan Opaque
            // on Windows) — sub-1.0 opacity produces an invisible window.
            let focused = ctx.window.window().has_focus();
            frame.palette.opacity = if !ctx.window.surface_has_alpha() {
                1.0
            } else if focused {
                self.config.window.effective_opacity()
            } else {
                self.config.window.effective_unfocused_opacity()
            };
            frame.window_focused = focused;
            frame.subpixel_positioning = renderer.subpixel_positioning();

            // IME preedit: overlay composition text at the cursor position
            // (underlined) so it flows through the normal shaping pipeline.
            if !self.ime.preedit.is_empty() {
                let cols = frame.columns();
                preedit::overlay_preedit_cells(&self.ime.preedit, &mut frame.content, cols);
            }

            // Annotate frame with pane-level state (mark cursor, search)
            // and client-side selection from App state.
            let base = frame.content.stable_row_base;
            // Mark cursor from App state (copied before render block).
            frame.mark_cursor = pane_mc.and_then(|mc| {
                let (line, col) = mc.to_viewport(frame.content.stable_row_base, frame.rows())?;
                Some(MarkCursorOverride {
                    line,
                    column: Column(col),
                    shape: CursorShape::HollowBlock,
                })
            });
            // Search from snapshot.
            {
                let mux = self.mux.as_ref().expect("mux checked");
                frame.search = mux
                    .pane_snapshot(pane_id)
                    .and_then(FrameSearch::from_snapshot);
            }
            // Selection lives on App, not Pane (copied before render block).
            frame.selection = pane_sel.map(|sel| FrameSelection::new(&sel, base));

            // Detect selection changes: compare the current selection state
            // with what was last rendered. When the selection changed (even
            // without terminal output), the prepare phase must rebuild
            // instance buffers and the render phase must re-render the
            // content cache texture.
            let num_rows = frame.rows();
            let new_sel_snap = frame
                .selection
                .as_ref()
                .and_then(|s| s.damage_snapshot(num_rows));
            let selection_changed = new_sel_snap != renderer.prepared.prev_selection_snapshot;

            // Compute hovered cell for hyperlink underline rendering.
            let cell_metrics = renderer.cell_metrics();
            let hovered_cell = {
                let grid_ctx = GridCtx {
                    widget: &ctx.terminal_grid,
                    cell: cell_metrics,
                    word_delimiters: &self.config.behavior.word_delimiters,
                };
                mouse_selection::pixel_to_cell(self.mouse.cursor_pos(), &grid_ctx)
                    .map(|(col, line)| (line, col))
            };
            frame.hovered_cell = hovered_cell;

            // Implicit URL hover: viewport-relative segments computed above.
            // The Vec was taken from the previous frame to reuse capacity.
            frame.hovered_url_segments = url_segments;

            // Visual prompt markers: clear extracted rows if the feature is disabled.
            if !self.config.behavior.prompt_markers {
                frame.prompt_marker_rows.clear();
            }

            // Capture blinking mode and cursor position for post-render
            // updates. State mutation is deferred to after GPU submission
            // so the render block stays free of side effects.
            let blinking_now = frame.content.mode.contains(TermMode::CURSOR_BLINKING);
            let cursor_pos = (frame.content.cursor.line, frame.content.cursor.column.0);

            // On false→true transition, force cursor visible this frame (the
            // timer reset hasn't happened yet, so is_visible() may be stale).
            let cursor_opacity = if blinking_now && self.blinking_active {
                let raw = self.cursor_blink.intensity();
                if self.config.terminal.cursor_blink_fade {
                    raw
                } else if raw > 0.5 {
                    1.0
                } else {
                    0.0
                }
            } else {
                1.0_f32
            };

            // Text blink opacity: always active (any cell could have BLINK).
            let text_blink_opacity = {
                let raw = self.text_blink.intensity();
                if self.config.terminal.text_blink_fade {
                    raw
                } else if raw > 0.5 {
                    1.0
                } else {
                    0.0
                }
            };
            frame.text_blink_opacity = text_blink_opacity;

            // Grid origin from layout bounds. When the layout engine
            // positions the grid (e.g. below a tab bar), this shifts all
            // cell rendering. Both bounds and cell metrics are in physical
            // pixels; the viewport (screen_size uniform) is also physical,
            // so the shader maps physical positions to NDC correctly.
            let origin = ctx
                .terminal_grid
                .bounds()
                .map_or((0.0, 0.0), |b| (b.x(), b.y()));

            let prepare_start = Instant::now();
            renderer.prepare(
                frame,
                gpu,
                pipelines,
                origin,
                cursor_opacity,
                content_changed,
            );

            // Scale factor for logical→physical coordinate conversion.
            let scale = ctx.window.scale_factor().factor() as f32;

            // Resolve icon atlas entries at physical pixel sizes.
            renderer.resolve_icons(gpu, scale);
            phases.prepare = prepare_start.elapsed();

            // Phase gating: prepare + prepaint widget trees if dirty.
            let widgets_start = Instant::now();
            draw_helpers::phase_gate_widgets(
                &mut ctx.root,
                &mut ctx.tab_bar,
                ctx.tab_bar_phys_rect,
                renderer,
                &ctx.text_cache,
                &self.ui_theme,
                scale,
                ctx.ui_stale,
            );

            // Draw tab bar (unified chrome bar). Tab bar contains text
            // (tab titles), so uses the text-aware draw list conversion.
            // Skipped when the tab bar is hidden.
            let tab_bar_hidden =
                self.config.window.tab_bar_position == crate::config::TabBarPosition::Hidden;
            let logical_w = (w as f32 / scale).round() as u32;
            let (interaction, flags, damage) = ctx.root.interaction_frame_requests_and_damage_mut();
            let tab_bar_ref = if tab_bar_hidden {
                None
            } else {
                Some(&ctx.tab_bar)
            };
            let tb_phys = ctx.tab_bar_phys_rect;
            let tab_bar_bounds = oriterm_ui::geometry::Rect::new(
                tb_phys.x() / scale,
                tb_phys.y() / scale,
                tb_phys.width() / scale,
                tb_phys.height() / scale,
            );
            let tab_bar_animating = Self::draw_tab_bar(
                tab_bar_ref,
                renderer,
                &mut ctx.chrome_scene,
                tab_bar_bounds,
                scale,
                gpu,
                &self.ui_theme,
                &ctx.text_cache,
                interaction,
                flags,
                damage,
            );
            if tab_bar_animating {
                ctx.root.mark_dirty();
            }

            // Draw overlays with per-overlay compositor opacity.
            let logical_size = (logical_w as f32, h as f32 / scale);
            let (overlays, layer_tree, interaction, flags) = ctx
                .root
                .overlays_layer_tree_interaction_and_frame_requests();
            let overlays_animating = Self::draw_overlays(
                overlays,
                renderer,
                &mut ctx.chrome_scene,
                logical_size,
                scale,
                gpu,
                layer_tree,
                &self.ui_theme,
                &ctx.text_cache,
                interaction,
                flags,
            );
            if overlays_animating {
                ctx.root.mark_dirty();
            }

            // Draw search bar overlay when search is active.
            if let Some(search) = frame.search.as_ref() {
                // Position below all chrome (caption + tab bar).
                // When the tab bar is hidden, chrome height is zero so
                // the search badge sits at the top of the grid area.
                let chrome_h = if tab_bar_hidden {
                    0.0
                } else {
                    ctx.tab_bar.metrics().height
                };
                Self::draw_search_bar(
                    search,
                    renderer,
                    &mut ctx.chrome_scene,
                    &mut ctx.search_bar_buf,
                    logical_w as f32,
                    chrome_h,
                    scale,
                    gpu,
                    &ctx.text_cache,
                );
            }

            // Update and draw status bar at the bottom of the window.
            if self.config.window.show_status_bar
                && self.config.window.tab_bar_position != crate::config::TabBarPosition::Bottom
            {
                ctx.status_bar.set_data(draw_helpers::status_bar_data(
                    1,
                    frame.content_cols,
                    frame.content_rows,
                ));
                let phys = ctx.status_bar_phys_rect;
                let sb_bounds = oriterm_ui::geometry::Rect::new(
                    phys.x() / scale,
                    phys.y() / scale,
                    phys.width() / scale,
                    phys.height() / scale,
                );
                Self::draw_status_bar(
                    &ctx.status_bar,
                    renderer,
                    &mut ctx.chrome_scene,
                    sb_bounds,
                    scale,
                    gpu,
                    &self.ui_theme,
                    &ctx.text_cache,
                );
            }

            // Full content render when terminal content changed, selection
            // changed, or chrome/overlay visuals are stale. Only cursor-
            // blink-only frames may reuse the cached texture.
            let needs_full_render = content_changed || selection_changed || ctx.ui_stale;

            // Overlay tiers render above the cached content every frame, so
            // only chrome animations keep the content cache stale.
            ctx.ui_stale = tab_bar_animating;

            // Window border: 2px border-strong frame, skipped when maximized/fullscreen.
            // macOS: the compositor provides a native window shadow — no border needed.
            #[cfg(not(target_os = "macos"))]
            if !ctx.window.is_maximized() && !ctx.window.is_fullscreen() {
                let border_color =
                    crate::gpu::scene_convert::color_to_rgb(self.ui_theme.border_strong);
                renderer.append_window_border(w, h, border_color, (2.0 * scale).round());
            }

            phases.widgets = widgets_start.elapsed();

            // Apply deferred DXGI ResizeBuffers just before acquiring the
            // surface texture. This minimizes the gap between swap chain
            // invalidation and frame presentation, preventing the DWM from
            // stretching stale content during interactive resize.
            ctx.window.apply_pending_surface_resize(gpu);

            let gpu_start = Instant::now();
            let result =
                renderer.render_to_surface(gpu, pipelines, ctx.window.surface(), needs_full_render);
            phases.gpu_render = gpu_start.elapsed();
            (result, blinking_now, cursor_pos)
        };

        self.finish_render(render_result, blinking_now, Some(cursor_pos));

        phases
    }
}
