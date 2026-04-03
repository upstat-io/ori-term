//! Multi-pane rendering: compute pane layouts and render all panes.
//!
//! When a tab has more than one pane (split or floating), this module takes
//! over from the single-pane fast path. Each pane is extracted, prepared at
//! its layout-computed pixel offset, and instances accumulate into one shared
//! `PreparedFrame` for a single GPU submission.

mod helpers;
mod pane_layouts;

use crate::session::{DividerLayout, PaneLayout};
use oriterm_core::{Column, CursorShape, TermMode};

use super::App;
use super::mouse_selection::{self, GridCtx};
use crate::gpu::{
    FrameSearch, FrameSelection, MarkCursorOverride, ViewportSize, extract_frame_from_snapshot,
    extract_frame_from_snapshot_into, snapshot_palette,
};

impl App {
    /// Execute the multi-pane rendering pipeline.
    ///
    /// Iterates all pane layouts, extracts and prepares each pane at its
    /// pixel offset, then appends dividers and a focus border. Chrome, tab
    /// bar, overlays, and search bar are drawn after all panes. Instances
    /// accumulate in a single `PreparedFrame` for one GPU submission.
    #[expect(
        clippy::too_many_lines,
        reason = "linear multi-pane pipeline: begin → per-pane extract+prepare → dividers → border → chrome → render"
    )]
    pub(super) fn handle_redraw_multi_pane(
        &mut self,
        layouts: &[PaneLayout],
        dividers: &[DividerLayout],
        mut url_segments: Vec<crate::url_detect::UrlSegment>,
    ) {
        self.populate_multi_pane_scratch(layouts);

        let (render_result, blinking_now) = {
            let Some(gpu) = self.gpu.as_ref() else {
                log::warn!("redraw multi: no gpu");
                return;
            };
            let Some(pipelines) = self.pipelines.as_ref() else {
                log::warn!("redraw multi: no pipelines");
                return;
            };
            let Some(ctx) = self
                .focused_window_id
                .and_then(|id| self.windows.get_mut(&id))
            else {
                log::warn!("redraw multi: no window");
                return;
            };
            let Some(renderer) = ctx.renderer.as_mut() else {
                log::warn!("redraw multi: no renderer");
                return;
            };

            if !ctx.window.has_surface_area() {
                return;
            }

            // Multi-pane: clear single-pane tracking so switching back to
            // a single-pane tab forces a content refresh (prevents stale
            // renderable_cache contamination from the swap path).
            ctx.last_rendered_pane = None;

            let (w, h) = ctx.window.size_px();
            let viewport = ViewportSize::new(w, h);
            let cell = renderer.cell_metrics();
            let bg = ctx
                .frame
                .as_ref()
                .map_or(oriterm_core::Rgb { r: 0, g: 0, b: 0 }, |f| {
                    f.palette.background
                });
            let win_focused = ctx.window.window().has_focus();
            let opacity = f64::from(super::draw_helpers::resolve_palette_opacity(
                ctx.window.surface_has_alpha(),
                win_focused,
                &self.config,
            ));

            renderer.begin_multi_pane_frame(viewport, bg, opacity);

            let dim_inactive = self.config.pane.dim_inactive;
            let inactive_opacity = self.config.pane.effective_inactive_opacity();

            let mut focused_rect = None;
            let mut blinking_now = self.blinking_active;
            let mut any_content_changed = false;
            let mut scratch_frame_pane = None;

            // Compute text blink opacity once (same for all panes) and detect
            // changes. When blink opacity changes, all cached panes are stale
            // because the old alpha is baked into glyph instances.
            let text_blink_opacity = super::draw_helpers::blink_opacity(
                self.text_blink.intensity(),
                self.config.terminal.text_blink_fade,
            );
            let blink_opacity_changed = (text_blink_opacity - ctx.prev_text_blink_opacity).abs()
                > super::draw_helpers::BLINK_OPACITY_EPSILON;
            ctx.prev_text_blink_opacity = text_blink_opacity;

            for layout in layouts {
                let pane_id = layout.pane_id;

                // Dirty check: unified snapshot-based dirty tracking.
                // Blink opacity changes invalidate all cached panes because
                // the old alpha is baked into glyph instances.
                let is_cached = ctx.pane_cache.is_cached(pane_id, layout);
                let snap_dirty = self
                    .mux
                    .as_ref()
                    .is_some_and(|m| m.is_pane_snapshot_dirty(pane_id));
                let no_snapshot = self
                    .mux
                    .as_ref()
                    .is_some_and(|m| m.pane_snapshot(pane_id).is_none());
                let dirty = layout.is_focused
                    || snap_dirty
                    || no_snapshot
                    || !is_cached
                    || blink_opacity_changed;
                any_content_changed |= dirty;

                if dirty {
                    let pane_viewport = ViewportSize::new(
                        layout.pixel_rect.width as u32,
                        layout.pixel_rect.height as u32,
                    );

                    // Extract phase: refresh snapshot if needed.
                    let mux = self.mux.as_mut().expect("mux checked");
                    let content_refreshed =
                        mux.pane_snapshot(pane_id).is_none() || mux.is_pane_snapshot_dirty(pane_id);
                    if content_refreshed {
                        mux.refresh_pane_snapshot(pane_id);
                    }

                    // Fast path (embedded): swap RenderableContent directly,
                    // bypassing WireCell round-trip. Only attempt when content
                    // was refreshed — stale cache entries from prior iterations
                    // would contaminate the frame otherwise.
                    let swapped = content_refreshed
                        && ctx
                            .frame
                            .as_mut()
                            .is_some_and(|f| mux.swap_renderable_content(pane_id, &mut f.content));

                    let Some(snapshot) = mux.pane_snapshot(pane_id) else {
                        log::warn!("multi-pane: no snapshot for pane {pane_id:?}");
                        ctx.root.mark_dirty();
                        continue;
                    };
                    if swapped {
                        let frame = ctx.frame.as_mut().expect("frame exists when swapped");
                        frame.viewport = pane_viewport;
                        frame.cell_size = cell;
                        frame.content_cols = snapshot.cols as usize;
                        frame.content_rows = snapshot.cells.len();
                        frame.palette = snapshot_palette(snapshot);
                        frame.clear_transient_fields();
                        frame.window_focused = true;
                        scratch_frame_pane = Some(pane_id);
                    } else if helpers::should_reextract_scratch_frame(
                        content_refreshed,
                        ctx.frame.is_none(),
                        scratch_frame_pane == Some(pane_id),
                    ) {
                        // `ctx.frame` is a shared scratch buffer across all
                        // panes in the loop. Even when a pane's snapshot isn't
                        // dirty, the scratch buffer may currently hold another
                        // pane's extracted content, so re-extract unless the
                        // scratch frame is known to already belong to this
                        // pane.
                        match &mut ctx.frame {
                            Some(existing) => {
                                extract_frame_from_snapshot_into(
                                    snapshot,
                                    existing,
                                    pane_viewport,
                                    cell,
                                );
                            }
                            slot @ None => {
                                *slot = Some(extract_frame_from_snapshot(
                                    snapshot,
                                    pane_viewport,
                                    cell,
                                ));
                            }
                        }
                        scratch_frame_pane = Some(pane_id);
                    } else {
                        // Cursor-blink-only: reuse existing frame as-is.
                    }
                    mux.clear_pane_snapshot_dirty(pane_id);

                    let frame = ctx.frame.as_mut().expect("frame just assigned");

                    let pane_focused = ctx.window.window().has_focus();
                    frame.palette.opacity = super::draw_helpers::resolve_palette_opacity(
                        ctx.window.surface_has_alpha(),
                        pane_focused,
                        &self.config,
                    );
                    frame.window_focused = pane_focused;
                    frame.subpixel_positioning = renderer.subpixel_positioning();

                    if layout.is_focused && !self.ime.preedit.is_empty() {
                        let cols = frame.columns();
                        super::preedit::overlay_preedit_cells(
                            &self.ime.preedit,
                            &mut frame.content,
                            cols,
                        );
                    }

                    // Pane-level annotations (mark cursor, search) and
                    // client-side selection from App state.
                    let base = frame.content.stable_row_base;
                    // Mark cursor from App state (copied before render block).
                    frame.mark_cursor = if layout.is_focused {
                        self.scratch_pane_mcs.get(&pane_id).and_then(|mc| {
                            let (line, col) =
                                mc.to_viewport(frame.content.stable_row_base, frame.rows())?;
                            Some(MarkCursorOverride {
                                line,
                                column: Column(col),
                                shape: CursorShape::HollowBlock,
                            })
                        })
                    } else {
                        None
                    };
                    // Search from snapshot.
                    frame.search = self
                        .mux
                        .as_ref()
                        .and_then(|m| m.pane_snapshot(pane_id))
                        .and_then(FrameSearch::from_snapshot);
                    // Selection lives on App, not Pane (copied before render block).
                    frame.selection = self
                        .scratch_pane_sels
                        .get(&pane_id)
                        .map(|sel| FrameSelection::new(sel, base));

                    if layout.is_focused {
                        let cell_metrics = renderer.cell_metrics();
                        let grid_ctx = GridCtx {
                            widget: &ctx.terminal_grid,
                            cell: cell_metrics,
                            word_delimiters: &self.config.behavior.word_delimiters,
                        };
                        frame.hovered_cell =
                            mouse_selection::pixel_to_cell(self.mouse.cursor_pos(), &grid_ctx)
                                .map(|(col, line)| (line, col));
                        frame.hovered_url_segments = std::mem::take(&mut url_segments);
                    } else {
                        frame.hovered_cell = None;
                        frame.hovered_url_segments.clear();
                    }

                    // Visual prompt markers: clear if disabled.
                    if !self.config.behavior.prompt_markers {
                        frame.prompt_marker_rows.clear();
                    }

                    if layout.is_focused {
                        blinking_now = frame.content.mode.contains(TermMode::CURSOR_BLINKING);
                        let pos = (frame.content.cursor.line, frame.content.cursor.column.0);
                        if pos != self.last_cursor_pos {
                            self.last_cursor_pos = pos;
                            // Inline reset_cursor_blink: split borrow
                            // prevents &mut self while self.windows is borrowed.
                            self.cursor_blink.reset();
                            self.blink_wakeup_gen
                                .store(0, std::sync::atomic::Ordering::Release);
                        }
                    }

                    frame.fg_dim = if layout.is_focused || !dim_inactive {
                        1.0
                    } else {
                        inactive_opacity
                    };

                    // Text blink: same opacity for all panes (pre-computed above).
                    frame.text_blink_opacity = text_blink_opacity;

                    let origin = (layout.pixel_rect.x, layout.pixel_rect.y);
                    // Compute cursor opacity per-pane using current frame's
                    // blinking_now (not stale self.blinking_active alone).
                    let pane_cursor_opacity = if layout.is_focused {
                        if blinking_now && self.blinking_active {
                            super::draw_helpers::blink_opacity(
                                self.cursor_blink.intensity(),
                                self.config.terminal.cursor_blink_fade,
                            )
                        } else {
                            1.0
                        }
                    } else {
                        0.0
                    };

                    let cached = ctx
                        .pane_cache
                        .get_or_prepare(pane_id, layout, true, |target| {
                            renderer.prepare_pane_into(
                                frame,
                                gpu,
                                origin,
                                pane_cursor_opacity,
                                target,
                            );
                        });
                    renderer.prepared.extend_from(cached);
                } else {
                    // Cache hit — merge cached instances without extraction.
                    let cached = ctx
                        .pane_cache
                        .get_cached(pane_id)
                        .expect("is_cached verified");
                    renderer.prepared.extend_from(cached);
                }

                if layout.is_focused {
                    focused_rect = Some(layout.pixel_rect);
                }
            }

            // Restore focused pane's search for the search bar.
            if let Some(focused) = layouts.iter().find(|l| l.is_focused) {
                if let Some(frame) = ctx.frame.as_mut() {
                    frame.search = self
                        .mux
                        .as_ref()
                        .and_then(|m| m.pane_snapshot(focused.pane_id))
                        .and_then(FrameSearch::from_snapshot);
                }
            }

            // Dividers between split panes.
            let divider_color = self.config.pane.effective_divider_color();
            let accent_color = self.config.pane.effective_focus_border_color();
            let hovered = ctx.hovering_divider;
            renderer.append_dividers(dividers, divider_color, accent_color, hovered);

            // Floating pane decorations (shadow + border).
            for layout in layouts.iter().filter(|l| l.is_floating) {
                renderer.append_floating_decoration(&layout.pixel_rect, accent_color);
            }

            // Focus border on active pane (only when multiple panes visible).
            let scale = ctx.window.scale_factor().factor() as f32;
            if layouts.len() > 1 {
                if let Some(rect) = &focused_rect {
                    renderer.append_focus_border(rect, accent_color, (2.0 * scale).round());
                }
            }

            // Chrome: tab bar, overlays, search bar, status bar, window border.
            let needs_full_render = super::chrome::render_chrome(
                ctx,
                &self.config,
                &self.ui_theme,
                gpu,
                &super::chrome::ChromeParams {
                    pane_count: layouts.len(),
                    content_dirty: any_content_changed,
                    selection_changed: false,
                    blink_changed: false,
                },
            );

            // Re-borrow renderer for GPU submission (prior borrow ended
            // when render_chrome returned via NLL).
            let renderer = ctx.renderer.as_mut().expect("renderer checked");

            ctx.window.apply_pending_surface_resize(gpu);

            let result =
                renderer.render_to_surface(gpu, pipelines, ctx.window.surface(), needs_full_render);
            (result, blinking_now)
        };

        self.finish_render(render_result, blinking_now, None);
    }
}

#[cfg(test)]
mod tests;
