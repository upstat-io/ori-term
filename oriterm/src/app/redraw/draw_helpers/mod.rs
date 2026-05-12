//! Draw helper methods for tab bar, overlays, chrome rendering, and widget
//! pipeline phases.
//!
//! Extracted from `mod.rs` to keep the module under the 500-line limit.

use std::collections::HashMap;
use std::time::Instant;

use oriterm_mux::backend::MuxBackend;
use oriterm_mux::id::PaneId;
use oriterm_ui::animation::FrameRequestFlags;
use oriterm_ui::draw::{DamageTracker, Scene, build_scene};
use oriterm_ui::geometry::Rect;
use oriterm_ui::interaction::InteractionManager;
use oriterm_ui::invalidation::DirtyKind;
use oriterm_ui::layout::compute_layout;
use oriterm_ui::overlay::OverlayManager;
use oriterm_ui::pipeline::collect_layout_bounds;
use oriterm_ui::theme::UiTheme;
use oriterm_ui::widget_id::WidgetId;
use oriterm_ui::widgets::status_bar::StatusBarData;
use oriterm_ui::widgets::tab_bar::TabBarWidget;
use oriterm_ui::widgets::{DrawCtx, LayoutCtx, Widget};
use oriterm_ui::window_root::WindowRoot;

use crate::app::App;
use crate::app::widget_pipeline;
use crate::font::{CachedTextMeasurer, CellMetrics, TextShapeCache};
use crate::gpu::frame_input::{FrameInput, ViewportSize};
use crate::gpu::state::GpuState;
use crate::gpu::window_renderer::WindowRenderer;
use crate::gpu::{extract_frame_from_snapshot, extract_frame_from_snapshot_into, snapshot_palette};

/// Outcome of [`try_swap_or_extract_pane_content`].
///
/// Callers use this to apply caller-specific extra state — single-pane
/// defers `window_focused` resolution; multi-pane tracks
/// `scratch_frame_pane` and sets `window_focused = true` on the swap path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum PaneExtractOutcome {
    /// Zero-copy fast path: `swap_renderable_content` succeeded.
    Swapped,
    /// Full extract path: `extract_frame_from_snapshot[_into]` ran.
    Reextracted,
    /// No-op: cursor-blink-only frame, existing `ctx.frame` reused as-is.
    Reused,
}

/// Try the zero-copy swap fast path, fall through to extract, then clear
/// the snapshot dirty bit. SSOT for the pane-content refresh skeleton
/// shared by single-pane and multi-pane redraw paths.
///
/// Steps:
///
/// 3. If `swap_gate && ctx_frame.is_some()`, attempt
///    `mux.swap_renderable_content`. The embedded backend swaps the
///    cached `RenderableContent` directly with the caller's
///    `FrameInput.content`, bypassing the `WireCell` round-trip.
/// 4. On successful swap: set the basic post-swap state — `viewport`,
///    `cell_size`, `content_cols`/`content_rows`, `palette`,
///    `clear_transient_fields`. Caller-specific extras
///    (`window_focused`, `scratch_frame_pane`) remain caller
///    responsibilities.
/// 5. Otherwise, if `reextract_gate`, run extract-or-replace via
///    `extract_frame_from_snapshot_into` (existing frame) or
///    `extract_frame_from_snapshot` (first frame).
/// 6. Always call `mux.clear_pane_snapshot_dirty(pane_id)` before
///    returning.
///
/// Returns `None` when `mux.pane_snapshot(pane_id)` is None — the
/// caller decides whether to `return`, `continue`, or mark dirty (the
/// snapshot-missing control flow differs between single-pane and
/// multi-pane). Steps 1 (content-changed detection) and 2 (initial
/// refresh decision) remain caller responsibilities because the
/// predicate inputs differ materially across the two paths.
#[allow(
    clippy::too_many_arguments,
    reason = "pane-content refresh helper threads mux + frame + pane id + viewport + metrics + two gates — each is a distinct caller-side input"
)]
pub(super) fn try_swap_or_extract_pane_content(
    mux: &mut dyn MuxBackend,
    ctx_frame: &mut Option<FrameInput>,
    pane_id: PaneId,
    viewport: ViewportSize,
    cell: CellMetrics,
    swap_gate: bool,
    reextract_gate: bool,
) -> Option<PaneExtractOutcome> {
    let swapped = swap_gate
        && ctx_frame
            .as_mut()
            .is_some_and(|f| mux.swap_renderable_content(pane_id, &mut f.content));

    let snapshot = mux.pane_snapshot(pane_id)?;

    let outcome = if swapped {
        let frame = ctx_frame.as_mut().expect("frame exists when swapped");
        frame.viewport = viewport;
        frame.cell_size = cell;
        frame.content_cols = snapshot.cols as usize;
        frame.content_rows = snapshot.cells.len();
        frame.palette = snapshot_palette(snapshot);
        frame.clear_transient_fields();
        PaneExtractOutcome::Swapped
    } else if reextract_gate {
        // Build a closure that resolves cached image bytes through the
        // MuxBackend trait surface. Daemon-mode `MuxClient` returns a cheap
        // `Arc` clone from its `image_cache`; embedded backends return None
        // (extract is never invoked when their `swap_renderable_content`
        // succeeds, so the closure is effectively unreachable for them).
        let image_lookup = |id| mux.pane_image_data(pane_id, id);
        match ctx_frame {
            Some(existing) => {
                extract_frame_from_snapshot_into(
                    snapshot,
                    existing,
                    viewport,
                    cell,
                    &image_lookup,
                );
            }
            slot @ None => {
                *slot = Some(extract_frame_from_snapshot(
                    snapshot,
                    viewport,
                    cell,
                    &image_lookup,
                ));
            }
        }
        PaneExtractOutcome::Reextracted
    } else {
        PaneExtractOutcome::Reused
    };

    mux.clear_pane_snapshot_dirty(pane_id);
    Some(outcome)
}

impl App {
    /// Draw the tab bar (unified chrome bar).
    ///
    /// Tab bar coordinates are in logical pixels, positioned at y=0.
    /// Uses [`append_ui_scene_with_text`](crate::gpu::WindowRenderer::append_ui_scene_with_text)
    /// because tab titles are rendered as shaped text.
    ///
    /// Returns `true` if the tab bar has running animations (e.g. bell pulse).
    #[expect(
        clippy::too_many_arguments,
        reason = "tab bar drawing: widget, renderer, scene, bounds, scale, GPU, theme, cache, interaction, frame_requests, damage"
    )]
    pub(in crate::app::redraw) fn draw_tab_bar(
        tab_bar: Option<&TabBarWidget>,
        renderer: &mut WindowRenderer,
        scene: &mut Scene,
        bounds: Rect,
        scale: f32,
        gpu: &GpuState,
        theme: &UiTheme,
        text_cache: &TextShapeCache,
        interaction: &InteractionManager,
        frame_requests: &FrameRequestFlags,
        damage_tracker: &mut DamageTracker,
    ) -> bool {
        let Some(tab_bar) = tab_bar else {
            return false;
        };
        if tab_bar.tab_count() == 0 {
            return false;
        }

        let measurer = CachedTextMeasurer::new(renderer.ui_measurer(scale), text_cache, scale);
        let icons = renderer.resolved_icons();

        let mut ctx = DrawCtx {
            measurer: &measurer,
            scene,
            bounds,
            now: Instant::now(),
            theme,
            icons: Some(icons),
            interaction: Some(interaction),
            widget_id: None,
            frame_requests: Some(frame_requests),
        };
        build_scene(tab_bar, &mut ctx);
        damage_tracker.compute_damage(scene);
        log::debug!(
            "damage: has_damage={}, dirty_regions={}",
            damage_tracker.has_damage(),
            damage_tracker.dirty_regions().len()
        );
        let animating = frame_requests.anim_frame_requested();

        // Tab bar contains text — use text-aware conversion to rasterize
        // tab title glyphs into the chrome tier.
        renderer.append_ui_scene_with_text(scene, scale, 1.0, gpu);

        // Dragged tab overlay: render in the overlay tier (draws 10-13) so it
        // paints ON TOP of all chrome text. Without this, regular tab text from
        // the chrome tier (draw 7) would show through the dragged tab's bg.
        if tab_bar.has_drag_overlay() {
            scene.clear();
            let measurer = CachedTextMeasurer::new(renderer.ui_measurer(scale), text_cache, scale);
            let icons = renderer.resolved_icons();
            let mut overlay_ctx = DrawCtx {
                measurer: &measurer,
                scene,
                bounds,
                now: Instant::now(),
                theme,
                icons: Some(icons),
                interaction: Some(interaction),
                widget_id: None,
                frame_requests: Some(frame_requests),
            };
            tab_bar.draw_drag_overlay(&mut overlay_ctx);
            renderer.append_overlay_scene_with_text(scene, scale, 1.0, gpu);
        }

        animating
    }

    /// Draw overlays (active + dismissing) with per-overlay compositor opacity.
    ///
    /// Each overlay is drawn individually so its compositor layer opacity
    /// can be applied independently (e.g. during simultaneous fade-in/fade-out).
    /// Modal dim rects are emitted before their content overlay.
    ///
    /// Returns `true` if overlays have running animations (fade-in/fade-out).
    #[expect(
        clippy::too_many_arguments,
        reason = "overlay drawing: manager, renderer, scene, viewport, scale, GPU, tree, theme, cache, interaction, frame_requests"
    )]
    pub(in crate::app::redraw) fn draw_overlays(
        overlays: &mut OverlayManager,
        renderer: &mut WindowRenderer,
        scene: &mut Scene,
        logical_size: (f32, f32),
        scale: f32,
        gpu: &GpuState,
        tree: &oriterm_ui::compositor::layer_tree::LayerTree,
        theme: &UiTheme,
        text_cache: &TextShapeCache,
        interaction: &InteractionManager,
        frame_requests: &FrameRequestFlags,
    ) -> bool {
        let count = overlays.draw_count();
        if count == 0 {
            return false;
        }

        let bounds = Rect::new(0.0, 0.0, logical_size.0, logical_size.1);
        let mut animating = false;

        // Layout + draw phase: measurer borrows renderer immutably, then
        // drops before the mutable append_ui_scene_with_text call.
        // Opacity is collected per overlay and appended after the borrow ends.
        {
            let measurer = CachedTextMeasurer::new(renderer.ui_measurer(scale), text_cache, scale);
            overlays.layout_overlays(&measurer, theme);
        }

        for i in 0..count {
            scene.clear();
            // Re-create measurer per iteration — cheap (no allocation), and
            // the immutable borrow drops before the mutable append below.
            let measurer = CachedTextMeasurer::new(renderer.ui_measurer(scale), text_cache, scale);
            let icons = renderer.resolved_icons();
            let mut ctx = DrawCtx {
                measurer: &measurer,
                scene,
                bounds,
                now: Instant::now(),
                theme,
                icons: Some(icons),
                interaction: Some(interaction),
                widget_id: None,
                frame_requests: Some(frame_requests),
            };
            let opacity = overlays.draw_overlay_at(i, &mut ctx, tree);

            // If opacity is < 1.0 an animation is running.
            if opacity < 1.0 - f32::EPSILON {
                animating = true;
            }

            // measurer (immutable borrow on renderer) is dropped here by NLL.
            // Overlays write to the overlay tier (draws 10-13) so their
            // backgrounds render ON TOP of chrome text (draws 7-9).
            renderer.append_overlay_scene_with_text(scene, scale, opacity, gpu);
        }

        animating || frame_requests.anim_frame_requested()
    }

    /// Draw the status bar at the bottom of the window.
    ///
    /// The status bar is non-interactive — no hover, focus, or animation
    /// state. It renders terminal metadata (shell name, pane count, grid
    /// dimensions, encoding, term type) into the chrome scene.
    #[expect(
        clippy::too_many_arguments,
        reason = "status bar drawing: widget, renderer, scene, bounds, scale, GPU, theme, cache"
    )]
    pub(in crate::app::redraw) fn draw_status_bar(
        status_bar: &oriterm_ui::widgets::status_bar::StatusBarWidget,
        renderer: &mut WindowRenderer,
        scene: &mut Scene,
        bounds: Rect,
        scale: f32,
        gpu: &GpuState,
        theme: &UiTheme,
        text_cache: &TextShapeCache,
    ) {
        let measurer = CachedTextMeasurer::new(renderer.ui_measurer(scale), text_cache, scale);
        scene.clear();
        let mut ctx = DrawCtx {
            measurer: &measurer,
            scene,
            bounds,
            now: Instant::now(),
            theme,
            icons: None,
            interaction: None,
            widget_id: None,
            frame_requests: None,
        };
        status_bar.paint(&mut ctx);
        renderer.append_ui_scene_with_text(scene, scale, 1.0, gpu);
    }
}

/// Computes prepaint layout bounds for a tab bar widget.
///
/// Runs the layout solver on the tab bar at its known position (y=0, full
/// logical width) and collects per-widget bounds into a `HashMap`. The
/// resulting map is passed to `prepaint_widget_tree` so that
/// `PrepaintCtx::bounds` reflects real screen positions.
#[expect(
    clippy::too_many_arguments,
    reason = "prepaint bounds: tab bar, renderer, cache, theme, scale, width"
)]
pub(in crate::app::redraw) fn collect_tab_bar_prepaint_bounds(
    tab_bar: &TabBarWidget,
    renderer: &WindowRenderer,
    text_cache: &TextShapeCache,
    theme: &UiTheme,
    scale: f32,
    tab_bar_bounds: Rect,
) -> HashMap<WidgetId, Rect> {
    let tab_bar_rect = tab_bar_bounds;
    let measurer = CachedTextMeasurer::new(renderer.ui_measurer(scale), text_cache, scale);
    let layout_ctx = LayoutCtx {
        measurer: &measurer,
        theme,
    };
    let mut bounds = HashMap::new();
    let tab_layout = compute_layout(&Widget::layout(tab_bar, &layout_ctx), tab_bar_rect);
    collect_layout_bounds(&tab_layout, &mut bounds);
    bounds
}

/// Run widget prepare and prepaint if the tree has pending dirty state.
///
/// Shared by both single-pane and multi-pane redraw paths. Drains
/// lifecycle events, checks dirty level, and if `>= Prepaint`, runs
/// the full prepare → prepaint pipeline on the tab bar and overlay
/// widget trees.
#[expect(
    clippy::too_many_arguments,
    reason = "phase gating: root, tab_bar, bounds, renderer, text_cache, theme, scale, stale"
)]
pub(super) fn phase_gate_widgets(
    root: &mut WindowRoot,
    tab_bar: &mut TabBarWidget,
    tab_bar_phys_rect: Rect,
    renderer: &WindowRenderer,
    text_cache: &TextShapeCache,
    ui_theme: &UiTheme,
    scale: f32,
    ui_stale: bool,
) {
    let now = Instant::now();
    let lifecycle_events = root.interaction_mut().drain_events();
    let widget_dirty = {
        let mut d = root.invalidation().max_dirty_kind();
        if !lifecycle_events.is_empty() {
            d = d.merge(DirtyKind::Prepaint);
        }
        if ui_stale {
            d = d.merge(DirtyKind::Prepaint);
        }
        d
    };
    root.frame_requests_mut().reset();

    log::debug!("phase gating: widget_dirty={widget_dirty:?}");

    if widget_dirty >= DirtyKind::Prepaint {
        let (interaction, invalidation, flags) =
            root.interaction_invalidation_and_frame_requests_mut();
        widget_pipeline::prepare_widget_tree(
            tab_bar,
            interaction,
            Some(invalidation),
            &lifecycle_events,
            None,
            Some(flags),
            now,
        );
        root.prepare_overlay_widgets(&lifecycle_events, now);

        let prepaint_tab_bounds = Rect::new(
            tab_bar_phys_rect.x() / scale,
            tab_bar_phys_rect.y() / scale,
            tab_bar_phys_rect.width() / scale,
            tab_bar_phys_rect.height() / scale,
        );
        let prepaint_bounds = collect_tab_bar_prepaint_bounds(
            tab_bar,
            renderer,
            text_cache,
            ui_theme,
            scale,
            prepaint_tab_bounds,
        );
        let (interaction, flags) = root.interaction_and_frame_requests();
        let invalidation = root.invalidation();
        widget_pipeline::prepaint_widget_tree(
            tab_bar,
            &prepaint_bounds,
            Some(interaction),
            ui_theme,
            now,
            Some(flags),
            Some(invalidation),
        );
        root.prepaint_overlay_widgets(&prepaint_bounds, ui_theme, now);
    }
}

/// Resolve window opacity from surface alpha support and focus state.
///
/// Returns 1.0 when the surface doesn't support alpha (Vulkan Opaque on
/// Windows), the focused opacity when focused, and the unfocused opacity
/// otherwise.
pub(super) fn resolve_palette_opacity(
    surface_has_alpha: bool,
    focused: bool,
    config: &crate::config::Config,
) -> f32 {
    if !surface_has_alpha {
        1.0
    } else if focused {
        config.window.effective_opacity()
    } else {
        config.window.effective_unfocused_opacity()
    }
}

/// Threshold for blink opacity snap: above this the blink is fully visible,
/// at or below it the blink is fully hidden. Used when smooth fade is disabled.
pub(super) const BLINK_SNAP_THRESHOLD: f32 = 0.5;

/// Minimum change in blink opacity between frames that triggers a full content
/// cache re-render (avoids re-rendering for imperceptible sub-pixel changes).
pub(super) const BLINK_OPACITY_EPSILON: f32 = 0.001;

/// Compute the post-render `ctx.ui_stale` value via OR-fold semantics.
///
/// On a successful render (`render_err == false`), the stale bit was
/// consumed by the frame that just flushed — reset to
/// `tab_bar_animating` (the chrome-tier animation signal that lives
/// independently of frame success).
///
/// On a failed render (`render_err == true` — any `SurfaceError`
/// variant: `Outdated` / `Lost` / `OutOfMemory` / `Other` / `Timeout`),
/// the stale bit was NOT consumed because no pixels reached the
/// surface — preserved via OR-fold so the next frame issues a full
/// render and publishes the chrome state the failed frame intended.
#[inline]
pub(super) fn post_render_ui_stale(
    prev_stale: bool,
    render_err: bool,
    tab_bar_animating: bool,
) -> bool {
    (prev_stale && render_err) || tab_bar_animating
}

/// Apply the [`post_render_ui_stale`] OR-fold in place on `ctx.ui_stale`.
///
/// One-line wrapper that keeps the single-pane and multi-pane caller
/// sites concise. Called AFTER `render_to_surface` returns and BEFORE
/// the redraw closure exits.
#[inline]
pub(super) fn apply_post_render_ui_stale<T>(
    ctx: &mut crate::app::window_context::WindowContext,
    render_result: &Result<(), T>,
    tab_bar_animating: bool,
) {
    ctx.ui_stale = post_render_ui_stale(ctx.ui_stale, render_result.is_err(), tab_bar_animating);
}

/// Compute final blink opacity from the raw animation intensity.
///
/// When `use_fade` is true, returns the raw intensity for a smooth fade
/// effect. Otherwise snaps to binary (1.0 or 0.0) based on
/// [`BLINK_SNAP_THRESHOLD`].
pub(super) fn blink_opacity(raw: f32, use_fade: bool) -> f32 {
    if use_fade {
        raw
    } else if raw > BLINK_SNAP_THRESHOLD {
        1.0
    } else {
        0.0
    }
}

/// Build status bar data from pane count and grid dimensions.
pub(super) fn status_bar_data(pane_count: usize, cols: usize, rows: usize) -> StatusBarData {
    StatusBarData {
        shell_name: "shell".into(),
        pane_count: format!(
            "{pane_count} pane{}",
            if pane_count == 1 { "" } else { "s" }
        ),
        grid_size: format!("{cols}\u{00d7}{rows}"),
        encoding: "UTF-8".into(),
        term_type: "xterm-256color".into(),
    }
}

#[cfg(test)]
mod tests;
