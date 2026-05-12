//! Three-phase rendering pipeline: Extract → Prepare → Render.

mod chrome;
mod debug_overlay;
mod draw_helpers;
mod multi_pane;
mod post_render;
mod predicates;
pub(in crate::app) mod preedit;
mod search_bar;

use predicates::{RedrawPredicateInputs, compute_redraw_predicates};

use std::time::Instant;

use super::App;
use super::mouse_selection::{self, GridCtx};
use super::perf_stats::FramePhases;
use super::window_context::WindowContext;
use crate::gpu::GpuState;
use crate::gpu::recovery::gate_outcome;
use crate::gpu::{
    FrameSearch, FrameSelection, MarkCursorOverride, ViewportSize, extract_frame_from_snapshot,
    extract_frame_from_snapshot_into, snapshot_palette,
};
use oriterm_core::{Column, CursorShape, TermMode};
use oriterm_mux::PaneId;

impl App {
    /// Mark all windows as needing a redraw.
    ///
    /// Used when mux notifications (PTY output, layout changes) may affect
    /// any window — not just the focused one. In multi-window setups, pane
    /// output in the unfocused window must still trigger a render.
    pub(super) fn mark_all_windows_dirty(&mut self) {
        for ctx in self.windows.values_mut() {
            ctx.root.mark_dirty();
        }
    }

    /// Mark only the window containing `pane_id` as dirty.
    ///
    /// Falls back to [`mark_all_windows_dirty`] if the pane's window cannot
    /// be resolved (orphan pane during close, or session out of sync).
    pub(super) fn mark_pane_window_dirty(&mut self, pane_id: PaneId) {
        if let Some(session_wid) = self.session.window_for_pane(pane_id) {
            for ctx in self.windows.values_mut() {
                if ctx.window.session_window_id() == session_wid {
                    ctx.root.mark_dirty();
                    return;
                }
            }
        }
        // Fallback: pane not found in session → mark all dirty.
        self.mark_all_windows_dirty();
    }

    /// Register (or clear) an animation-frame deadline on the window
    /// containing `pane_id`.
    ///
    /// Routes `MuxNotification::AnimationDeadlineChanged { deadline }` into
    /// the owning window's `RenderScheduler` via `set_animation_deadline`.
    /// `Some(t)` upserts — the scheduler wakes the winit event loop at `t`
    /// via `ControlFlow::WaitUntil`. `None` clears the prior deadline so a
    /// queued wake from an animation that has since stopped does NOT fire
    /// spuriously — critical for honoring the "zero idle CPU beyond cursor
    /// blink" invariant across animation start/stop cycles.
    pub(super) fn request_pane_animation_frame_at(
        &mut self,
        pane_id: PaneId,
        deadline: Option<Instant>,
    ) {
        let Some(session_wid) = self.session.window_for_pane(pane_id) else {
            return;
        };
        let key = pane_id.raw();
        for ctx in self.windows.values_mut() {
            if ctx.window.session_window_id() == session_wid {
                ctx.root
                    .scheduler_mut()
                    .set_animation_deadline(key, deadline);
                return;
            }
        }
    }
    /// Execute the three-phase rendering pipeline: Extract → Prepare → Render.
    #[expect(
        clippy::too_many_lines,
        reason = "linear three-phase pipeline: Extract → Prepare → Render"
    )]
    pub(super) fn handle_redraw(&mut self) -> FramePhases {
        log::trace!("RedrawRequested");
        let mut phases = FramePhases::default();

        // I1 render gate (5.16.2). Direct callers (`WindowEvent::RedrawRequested`
        // in `event_loop.rs`, `tab_drag::tear_off`, the Windows modal-loop
        // path) bypass `render_dirty_windows`'s gate, so this is the second
        // canonical call site of `gate_outcome`. Both sites read from the
        // same `GpuHealth` SSOT — no parallel state.
        if let Some(gated) = gate_outcome(&self.gpu_health) {
            log::trace!("handle_redraw gate: {gated:?}");
            return phases;
        }

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
            let preds = compute_redraw_predicates(RedrawPredicateInputs {
                snap_is_none,
                snap_dirty,
                pane_changed,
                preedit_revision: self.ime.preedit_revision,
                prev_preedit_revision: ctx.prev_preedit_revision,
            });
            let snapshot_changed = preds.snapshot_changed;
            let content_changed = preds.content_changed;
            if snapshot_changed {
                mux.refresh_pane_snapshot(pane_id);
            }

            // Fast path (embedded): swap RenderableContent directly from
            // the terminal, bypassing the WireCell round-trip. Only attempt
            // when SNAPSHOT was refreshed — `swap_renderable_content` does a
            // blind `std::mem::swap` on the cache and would surface a stale
            // buffer carrying the prior frame's destructive preedit overlay
            // if gated on the broader `content_changed`. See the helper's
            // type-level doc for the regression anchor.
            let swapped = snapshot_changed
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
                frame.clear_transient_fields();
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
            let focused = ctx.window.window().has_focus();
            frame.palette.opacity = draw_helpers::resolve_palette_opacity(
                ctx.window.surface_has_alpha(),
                focused,
                &self.config,
            );
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
                draw_helpers::blink_opacity(
                    self.cursor_blink.intensity(),
                    self.config.terminal.cursor_blink_fade,
                )
            } else {
                1.0_f32
            };

            // Text blink opacity: always active (any cell could have BLINK).
            let text_blink_opacity = draw_helpers::blink_opacity(
                self.text_blink.intensity(),
                self.config.terminal.text_blink_fade,
            );
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

            // Track text-blink opacity for multi-pane pane-cache invalidation
            // (read by `redraw/multi_pane/mod.rs` `blink_opacity_changed`).
            // The single-pane "blink-changed" predicate now flows through
            // `WindowRenderer::has_dispatch_change` (the dispatch fingerprint
            // hashes `text_blink_opacity`), which chrome queries via
            // `cache_invalidated_this_frame()`.
            ctx.prev_text_blink_opacity = text_blink_opacity;
            // Sync prev_preedit_revision so the next frame's
            // `compute_redraw_predicates` only flags `content_changed` on
            // an ACTUAL revision delta.
            ctx.prev_preedit_revision = self.ime.preedit_revision;

            // Chrome: tab bar, overlays, search bar, status bar, window border.
            let widgets_start = Instant::now();
            // Take search out of ctx.frame so ChromeParams carries an owned
            // local borrow (avoids &mut self vs &FrameSearch borrow conflict).
            let chrome_search = ctx.frame.as_mut().and_then(|f| f.search.take());
            let (cols, rows) = ctx
                .frame
                .as_ref()
                .map_or((0, 0), |f| (f.content_cols, f.content_rows));
            let chrome::ChromeRenderResult {
                needs_full_render,
                tab_bar_animating,
            } = chrome::render_chrome(
                ctx,
                &self.config,
                &self.ui_theme,
                gpu,
                &chrome::ChromeParams {
                    pane_count: 1,
                    content_cols: cols,
                    content_rows: rows,
                    search: chrome_search.as_ref(),
                },
            );
            if let (Some(frame), Some(s)) = (ctx.frame.as_mut(), chrome_search) {
                frame.search = Some(s);
            }
            phases.widgets = widgets_start.elapsed();

            // Debug performance overlay (Ctrl+Shift+F12).
            let mut overlay = DebugOverlayCtx {
                enabled: self.debug_overlay_enabled,
                debug_fps: &mut self.debug_fps,
                last_render: &self.last_render,
                scale,
            };
            draw_debug_overlay_if_enabled(&mut overlay, ctx, gpu);

            // Re-borrow renderer for GPU submission (prior borrow ended
            // when render_chrome returned via NLL).
            let renderer = ctx.renderer.as_mut().expect("renderer checked");

            // Apply deferred DXGI ResizeBuffers before surface acquisition
            // (minimizes the DWM stale-content window during resize).
            ctx.window.apply_pending_surface_resize(gpu);

            let gpu_start = Instant::now();
            let result =
                renderer.render_to_surface(gpu, pipelines, ctx.window.surface(), needs_full_render);
            phases.gpu_render = gpu_start.elapsed();

            draw_helpers::apply_post_render_ui_stale(ctx, &result, tab_bar_animating);
            (result, blinking_now, cursor_pos)
        };

        self.finish_render(render_result, blinking_now, Some(cursor_pos));

        phases
    }
}

/// Overlay-specific inputs for [`draw_debug_overlay_if_enabled`].
///
/// Groups the overlay's own config (toggle + EWMA accumulator + last-frame
/// timestamp + DPI scale) into one parameter so the function signature stays
/// compact. `&mut WindowContext` and `&GpuState` remain separate arguments —
/// they are shared with the rest of the redraw pipeline and not
/// overlay-specific.
struct DebugOverlayCtx<'a> {
    enabled: bool,
    debug_fps: &'a mut f32,
    last_render: &'a Instant,
    scale: f32,
}

/// Draw the debug performance overlay if enabled (Ctrl+Shift+F12).
///
/// Extracted from [`App::handle_redraw`] to reduce function length.
/// Updates EWMA FPS and renders the overlay scene.
fn draw_debug_overlay_if_enabled(
    overlay: &mut DebugOverlayCtx<'_>,
    ctx: &mut WindowContext,
    gpu: &GpuState,
) {
    if !overlay.enabled {
        return;
    }
    let scale = overlay.scale;
    // Update EWMA FPS from last render interval.
    let elapsed = overlay.last_render.elapsed().as_secs_f32();
    if elapsed > 0.0 {
        let instant_fps = 1.0 / elapsed;
        // EWMA with alpha=0.1 for smooth display.
        *overlay.debug_fps = if *overlay.debug_fps < 1.0 {
            instant_fps
        } else {
            0.1 * instant_fps + 0.9 * *overlay.debug_fps
        };
    }

    let renderer = ctx.renderer.as_mut().expect("renderer checked");
    let stats = debug_overlay::DebugStats {
        fps: *overlay.debug_fps,
        dirty_rows: renderer
            .prepared
            .scratch_dirty
            .iter()
            .filter(|&&d| d)
            .count(),
        total_rows: renderer.prepared.scratch_dirty.len(),
        instances: renderer.prepared.total_instances(),
        draw_calls: renderer.prepared.count_draw_calls(),
        mono_atlas: (renderer.atlas().len(), renderer.atlas().page_count()),
        subpixel_atlas: (
            renderer.subpixel_atlas().len(),
            renderer.subpixel_atlas().page_count(),
        ),
        color_atlas: (
            renderer.color_atlas().len(),
            renderer.color_atlas().page_count(),
        ),
    };
    let (pw, ph) = ctx.window.size_px();
    let lw = pw as f32 / scale;
    let lh = ph as f32 / scale;
    App::draw_debug_overlay(
        &stats,
        renderer,
        &mut ctx.chrome_scene,
        &mut ctx.debug_overlay_buf,
        lw,
        lh,
        scale,
        gpu,
        &ctx.text_cache,
    );
}
