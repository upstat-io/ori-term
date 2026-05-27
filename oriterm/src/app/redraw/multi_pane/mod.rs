//! Multi-pane rendering: compute pane layouts and render all panes.
//!
//! When a tab has more than one pane (split or floating), this module takes
//! over from the single-pane fast path. Each pane is extracted, prepared at
//! its layout-computed pixel offset, and instances accumulate into one shared
//! `PreparedFrame` for a single GPU submission.

mod helpers;
mod pane_layouts;

use crate::session::{DividerLayout, PaneLayout};
use oriterm_core::{Column, CursorShape, RenderableCursor, TermMode};

use super::App;
use super::mouse_selection::{self, GridCtx};
use crate::gpu::frame_input::FramePalette;
use crate::gpu::prepare::{DispatchFingerprintInputs, PaneRowState, compute_pane_damage_key};
use crate::gpu::{FrameSearch, FrameSelection, MarkCursorOverride, ViewportSize, snapshot_palette};

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
            // Focused-pane snapshot is the SSOT for chrome-relevant
            // values: clear color, status-bar cols/rows, search-bar
            // state. Look it up ONCE here (rather than re-querying per
            // consumer downstream) and project owned values into outer
            // locals so the &PaneSnapshot borrow on self.mux ends before
            // the per-pane mut-self loop body runs.
            let (bg, focused_cols, focused_rows, focused_search) = {
                let focused_snap = layouts
                    .iter()
                    .find(|l| l.is_focused)
                    .and_then(|l| self.mux.as_ref().and_then(|m| m.pane_snapshot(l.pane_id)));
                let bg = focused_snap
                    .map(snapshot_palette)
                    .map_or(oriterm_core::Rgb { r: 0, g: 0, b: 0 }, |p| p.background);
                let (cols, rows, search) = helpers::focused_pane_chrome_state(focused_snap);
                (bg, cols, rows, search)
            };
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

            let _ = blink_opacity_changed; // now covered via damage_key (text_blink_opacity field)

            for layout in layouts {
                let pane_id = layout.pane_id;

                // Snapshot freshness — must refresh BEFORE damage_key
                // computation so the key derives from post-refresh state.
                let snap_dirty = self
                    .mux
                    .as_ref()
                    .is_some_and(|m| m.is_pane_snapshot_dirty(pane_id));
                let no_snapshot = self
                    .mux
                    .as_ref()
                    .is_some_and(|m| m.pane_snapshot(pane_id).is_none());
                let needs_refresh = snap_dirty || no_snapshot;
                if needs_refresh {
                    if let Some(mux) = self.mux.as_mut() {
                        mux.refresh_pane_snapshot(pane_id);
                    }
                }
                let dirty_content = needs_refresh;

                // Compute prospective damage_key from snapshot + layout + state.
                // Layered SSOT: compute_dispatch_fingerprint (frame inputs) +
                // PaneRowState (row inputs single-pane handles via per-row dirty).
                let damage_key = {
                    let Some(snap) = self.mux.as_ref().and_then(|m| m.pane_snapshot(pane_id))
                    else {
                        // No snapshot after refresh attempt — log + skip.
                        log::warn!("multi-pane: no snapshot for pane {pane_id:?} after refresh");
                        ctx.root.mark_dirty();
                        continue;
                    };
                    let pane_focused = ctx.window.window().has_focus();
                    let snap_palette = snapshot_palette(snap);
                    let dim_inactive_cfg = self.config.pane.dim_inactive;
                    let inactive_opacity_cfg = self.config.pane.effective_inactive_opacity();
                    let pane_search = FrameSearch::from_snapshot(snap);
                    let palette_for_pane = FramePalette {
                        background: snap_palette.background,
                        foreground: snap_palette.foreground,
                        cursor_color: snap_palette.cursor_color,
                        selection_fg: snap_palette.selection_fg,
                        selection_bg: snap_palette.selection_bg,
                        opacity: super::draw_helpers::resolve_palette_opacity(
                            ctx.window.surface_has_alpha(),
                            pane_focused,
                            &self.config,
                        ),
                    };
                    let pane_viewport = ViewportSize::new(
                        layout.pixel_rect.width as u32,
                        layout.pixel_rect.height as u32,
                    );
                    let dispatch_inputs = DispatchFingerprintInputs {
                        viewport: pane_viewport,
                        cell_size: cell,
                        content_cols: snap.cols as usize,
                        content_rows: snap.cells.len(),
                        origin: (layout.pixel_rect.x, layout.pixel_rect.y),
                        text_blink_opacity,
                        palette: palette_for_pane.damage_fingerprint(),
                        fg_dim: if layout.is_focused || !dim_inactive_cfg {
                            1.0
                        } else {
                            inactive_opacity_cfg
                        },
                        subpixel_positioning: renderer.subpixel_positioning(),
                        search: pane_search.as_ref().map(FrameSearch::damage_fingerprint),
                    };
                    // Row-state inputs (selection, hovered, mark, cursor opacity,
                    // preedit, focus). Focused-pane only fields zeroed otherwise.
                    let selection_damage = self
                        .scratch_pane_sels
                        .get(&pane_id)
                        .map(|sel| FrameSelection::new(sel, snap.stable_row_base))
                        .and_then(|fsel| fsel.damage_snapshot(snap.cells.len()));
                    let mark_cursor_key = if layout.is_focused {
                        self.scratch_pane_mcs.get(&pane_id).and_then(|mc| {
                            let (line, col) =
                                mc.to_viewport(snap.stable_row_base, snap.cells.len())?;
                            Some(MarkCursorOverride {
                                line,
                                column: Column(col),
                                shape: CursorShape::HollowBlock,
                            })
                        })
                    } else {
                        None
                    };
                    let hovered_cell_key = if layout.is_focused {
                        let cell_metrics = renderer.cell_metrics();
                        let grid_ctx = GridCtx {
                            widget: &ctx.terminal_grid,
                            cell: cell_metrics,
                            word_delimiters: &self.config.behavior.word_delimiters,
                        };
                        mouse_selection::pixel_to_cell(self.mouse.cursor_pos(), &grid_ctx)
                            .map(|(col, line)| (line, col))
                    } else {
                        None
                    };
                    // Read CURSOR_BLINKING directly from snapshot — `blinking_now`
                    // (the loop-local that mirrors focused-pane mode) is only
                    // updated INSIDE the cache-miss block, so it's unreliable
                    // for damage_key computation that gates cache-hit/miss.
                    let snap_blinking = TermMode::from_bits_truncate(snap.modes)
                        .contains(TermMode::CURSOR_BLINKING);
                    let pane_cursor_opacity_key = if layout.is_focused {
                        if snap_blinking && self.blinking_active {
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
                    let row_state = PaneRowState {
                        resolved_cursor_visible: if layout.is_focused && snap.cursor.visible {
                            Some(RenderableCursor {
                                line: snap.cursor.row as usize,
                                column: Column(snap.cursor.col as usize),
                                shape: CursorShape::from(snap.cursor.shape),
                                visible: true,
                            })
                        } else {
                            None
                        },
                        selection_snapshot: selection_damage,
                        hovered_cell: hovered_cell_key,
                        mark_cursor: mark_cursor_key,
                        cursor_opacity_bits: pane_cursor_opacity_key.to_bits(),
                        block_cursor_color_exclusion_active: false,
                        preedit_revision: if layout.is_focused {
                            self.ime.preedit_revision
                        } else {
                            0
                        },
                        window_focused: pane_focused,
                        hovered_url_segments_hash: if layout.is_focused && !url_segments.is_empty()
                        {
                            use std::collections::hash_map::DefaultHasher;
                            use std::hash::{Hash, Hasher};
                            let mut h = DefaultHasher::new();
                            url_segments.hash(&mut h);
                            h.finish()
                        } else {
                            0
                        },
                    };
                    compute_pane_damage_key(&dispatch_inputs, &row_state)
                };

                let mut cache_hit =
                    !dirty_content && ctx.pane_cache.is_cached(pane_id, layout, damage_key);

                if cache_hit {
                    // Cache hit candidate — must verify that every image
                    // texture referenced by the cached pane is still
                    // resident in `image_texture_cache`. `evict_over_limit`
                    // in `finish_image_frame` can drop textures across
                    // frames when total GPU memory exceeds the limit; if
                    // any of THIS pane's images were evicted, the cached
                    // `ImageQuad`s would silently skip at draw time
                    // (`render_helpers.rs::draw_image_quads` continues
                    // past `get_bind_group(quad.image_id) == None` quads).
                    //
                    // `touch_cached_pane_images` returns `false` when one
                    // or more referenced images are missing — in that
                    // case invalidate the pane cache and fall through to
                    // the cache-miss branch to re-upload via
                    // `prepare_pane_into`.
                    let cached = ctx
                        .pane_cache
                        .get_cached(pane_id)
                        .expect("is_cached verified");
                    if renderer.touch_cached_pane_images(cached) {
                        renderer.prepared.extend_from(cached);
                    } else {
                        ctx.pane_cache.invalidate(pane_id);
                        cache_hit = false;
                    }
                }

                if !cache_hit {
                    // Cache miss — must extract + annotate + prepare into cache.
                    let pane_viewport = ViewportSize::new(
                        layout.pixel_rect.width as u32,
                        layout.pixel_rect.height as u32,
                    );

                    // Snapshot already refreshed at top of loop iteration when
                    // needs_refresh was true. The pre-refresh covers the
                    // damage_key computation; we DO NOT re-refresh here (the
                    // old inner `is_pane_snapshot_dirty` check returned true
                    // because refresh_pane_snapshot does not clear the dirty
                    // bit — only `clear_pane_snapshot_dirty` does that, which
                    // happens below at line 198). Double-refresh was a
                    // TOCTOU/efficiency: avoid double-refresh of the snapshot.
                    let mux = self.mux.as_mut().expect("mux checked");
                    let content_refreshed = dirty_content;

                    // Steps 3-6 of the pane-content refresh skeleton — SSOT
                    // shared with the single-pane redraw driver. Caller-
                    // specific concerns (scratch_frame_pane tracking,
                    // window_focused) remain here. The reextract gate uses
                    // `helpers::should_reextract_scratch_frame` to handle
                    // the shared-scratch-buffer contamination case where
                    // the buffer currently holds another pane's content.
                    let reextract_gate = helpers::should_reextract_scratch_frame(
                        content_refreshed,
                        ctx.frame.is_none(),
                        scratch_frame_pane == Some(pane_id),
                    );
                    let outcome = super::draw_helpers::try_swap_or_extract_pane_content(
                        mux.as_mut(),
                        &mut ctx.frame,
                        pane_id,
                        super::draw_helpers::PaneContentRequest {
                            viewport: pane_viewport,
                            cell,
                            swap_gate: content_refreshed,
                            reextract_gate,
                        },
                    );
                    let Some(outcome) = outcome else {
                        log::warn!("multi-pane: no snapshot for pane {pane_id:?}");
                        ctx.root.mark_dirty();
                        continue;
                    };
                    match outcome {
                        super::draw_helpers::PaneExtractOutcome::Swapped => {
                            let frame = ctx.frame.as_mut().expect("frame populated by swap");
                            frame.window_focused = true;
                            scratch_frame_pane = Some(pane_id);
                        }
                        super::draw_helpers::PaneExtractOutcome::Reextracted => {
                            scratch_frame_pane = Some(pane_id);
                        }
                        super::draw_helpers::PaneExtractOutcome::Reused => {}
                    }

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

                    let cached = ctx.pane_cache.get_or_prepare(
                        crate::gpu::PaneCacheKey {
                            pane_id,
                            layout,
                            dirty: true,
                            damage_key,
                        },
                        |target| {
                            renderer.prepare_pane_into(
                                frame,
                                crate::gpu::PanePrepare {
                                    gpu,
                                    pipelines,
                                    origin,
                                    cursor_opacity: pane_cursor_opacity,
                                },
                                target,
                            );
                        },
                    );
                    renderer.prepared.extend_from(cached);
                }

                if layout.is_focused {
                    focused_rect = Some(layout.pixel_rect);
                }
            }

            // Focused-pane chrome state was computed once at the top of
            // this function (focused_cols, focused_rows, focused_search)
            // alongside the bg clear color, sharing a single snapshot
            // lookup. ChromeParams consumes those owned locals here.

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
                    renderer.append_focus_border(
                        rect,
                        accent_color,
                        crate::gpu::window_renderer::physical_border_width(scale),
                    );
                }
            }

            // Finalize image-texture-cache lifecycle for this visual frame.
            // Pairs with `begin_multi_pane_frame()` called at the top.
            // Runs evict_unused + evict_over_limit exactly once per frame.
            // A naive per-pane finish would tighten the retention window
            // to THRESHOLD / pane_count.
            renderer.finish_multi_pane_frame();

            // Chrome: tab bar, overlays, search bar, status bar, window border.
            let super::chrome::ChromeRenderResult {
                needs_full_render,
                tab_bar_animating,
            } = super::chrome::render_chrome(
                ctx,
                &self.config,
                &self.ui_theme,
                gpu,
                &super::chrome::ChromeParams {
                    pane_count: layouts.len(),
                    content_cols: focused_cols,
                    content_rows: focused_rows,
                    search: focused_search.as_ref(),
                },
            );

            // Re-borrow renderer for GPU submission (prior borrow ended
            // when render_chrome returned via NLL).
            let renderer = ctx.renderer.as_mut().expect("renderer checked");

            ctx.window.apply_pending_surface_resize(gpu);

            let result =
                renderer.render_to_surface(gpu, pipelines, ctx.window.surface(), needs_full_render);

            super::draw_helpers::apply_post_render_ui_stale(ctx, &result, tab_bar_animating);

            (result, blinking_now)
        };

        self.finish_render(render_result, blinking_now, None);
    }
}

#[cfg(test)]
mod tests;
