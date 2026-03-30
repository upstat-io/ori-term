//! Multi-pane rendering: prepare multiple panes into a single GPU frame.
//!
//! The key insight is that [`fill_frame_shaped`] appends instances to a
//! [`PreparedFrame`] without clearing — so calling it once per pane with
//! different origins accumulates all panes into one frame for a single
//! GPU submission.

use oriterm_core::Rgb;

use crate::gpu::builtin_glyphs;
use crate::gpu::frame_input::{FrameInput, ViewportSize};
use crate::gpu::instance_writer::{CLIP_UNCLIPPED, ScreenRect};
use crate::gpu::prepare;
use crate::gpu::prepared_frame::PreparedFrame;
use crate::gpu::state::GpuState;
use crate::session::{DividerLayout, Rect};

use super::helpers::{ensure_glyphs_cached, grid_raster_keys, shape_frame};
use super::{CombinedAtlasLookup, WindowRenderer};

impl WindowRenderer {
    /// Begin a multi-pane frame: reset atlases, clear instance buffers, set viewport.
    ///
    /// Call once before [`prepare_pane_into`] calls. Sets the viewport and clear
    /// color for the entire window, then clears all instance buffers so pane
    /// instances can accumulate cleanly.
    pub(crate) fn begin_multi_pane_frame(
        &mut self,
        viewport: ViewportSize,
        background: Rgb,
        opacity: f64,
    ) {
        self.atlas.begin_frame();
        self.subpixel_atlas.begin_frame();
        self.color_atlas.begin_frame();

        self.prepared.clear();
        self.prepared.viewport = viewport;
        self.prepared.set_clear_color(background, opacity);
    }

    /// Shape, cache, and fill one pane into a separate `PreparedFrame`.
    ///
    /// Fills into `target` (a per-pane cached frame) rather than `self.prepared`.
    /// The caller merges cached frames into `self.prepared` after all panes are
    /// prepared. The `origin` offset positions this pane's cells at its
    /// layout-computed pixel rect.
    #[expect(
        clippy::too_many_arguments,
        reason = "pane prepare: input, GPU state, origin offset, cursor flag, target frame"
    )]
    pub(crate) fn prepare_pane_into(
        &mut self,
        input: &FrameInput,
        gpu: &GpuState,
        origin: (f32, f32),
        cursor_blink_visible: bool,
        target: &mut PreparedFrame,
    ) {
        // Off-screen culling in the prepare phase uses the target frame's
        // viewport, so multi-pane cached frames must inherit the full window
        // viewport rather than the pane-local extraction viewport.
        target.viewport = self.prepared.viewport;

        // Phase A: Shape all rows for this pane.
        shape_frame(input, &self.font_collection, &mut self.shaping);

        // Phase B: Ensure shaped glyphs cached.
        ensure_glyphs_cached(
            grid_raster_keys(
                &self.shaping.frame,
                self.font_collection.hinting_mode().hint_flag(),
            ),
            &mut self.atlas,
            &mut self.subpixel_atlas,
            &mut self.color_atlas,
            &mut self.empty_keys,
            &mut self.font_collection,
            &gpu.device,
            &gpu.queue,
        );

        // Phase B2: Built-in geometric glyphs + decoration patterns.
        builtin_glyphs::ensure_builtins_cached(
            input,
            self.shaping.frame.size_q6(),
            &mut self.atlas,
            &mut self.empty_keys,
            &gpu.device,
            &gpu.queue,
        );

        // Phase C: Fill into the target prepared frame.
        let bridge = CombinedAtlasLookup {
            mono: &self.atlas,
            subpixel: &self.subpixel_atlas,
            color: &self.color_atlas,
        };
        prepare::fill_frame_shaped(
            input,
            &bridge,
            &self.shaping.frame,
            target,
            origin,
            cursor_blink_visible,
        );
    }

    /// Append divider rectangles to the backgrounds buffer.
    ///
    /// Dividers are solid-color rectangles between split panes. Drawn into
    /// the background layer so they appear behind glyphs and cursors.
    /// The hovered divider (if any) uses `hover_color` instead of `color`.
    pub(crate) fn append_dividers(
        &mut self,
        dividers: &[DividerLayout],
        color: Rgb,
        hover_color: Rgb,
        hovered: Option<DividerLayout>,
    ) {
        for div in dividers {
            let c = if hovered == Some(*div) {
                hover_color
            } else {
                color
            };
            self.prepared.backgrounds.push_rect(
                ScreenRect {
                    x: div.rect.x,
                    y: div.rect.y,
                    w: div.rect.width,
                    h: div.rect.height,
                },
                c,
                1.0,
            );
        }
    }

    /// Append decoration around a floating pane: drop shadow and border.
    ///
    /// The shadow is a semi-transparent rectangle offset 2px down-right,
    /// drawn into the backgrounds layer. The border is a 1px accent-colored
    /// frame drawn into the UI rects layer with slight corner radius.
    pub(crate) fn append_floating_decoration(&mut self, rect: &Rect, accent: Rgb) {
        let shadow_offset = 2.0_f32;
        let shadow_expand = 4.0_f32;

        // Drop shadow (behind content, in backgrounds layer).
        self.prepared.backgrounds.push_rect(
            ScreenRect {
                x: rect.x - shadow_expand + shadow_offset,
                y: rect.y - shadow_expand + shadow_offset,
                w: rect.width + 2.0 * shadow_expand,
                h: rect.height + 2.0 * shadow_expand,
            },
            Rgb { r: 0, g: 0, b: 0 },
            0.3,
        );

        // Border (in UI rects layer, renders on top of content).
        let border_color = [
            f32::from(accent.r) / 255.0,
            f32::from(accent.g) / 255.0,
            f32::from(accent.b) / 255.0,
            1.0,
        ];
        self.prepared.ui_rects.push_ui_rect(
            ScreenRect {
                x: rect.x,
                y: rect.y,
                w: rect.width,
                h: rect.height,
            },
            [0.0, 0.0, 0.0, 0.0], // transparent fill
            [1.0; 4],             // uniform border width
            [2.0; 4],             // uniform corner radius
            [border_color; 4],    // same color all sides
            CLIP_UNCLIPPED,
        );
    }

    /// Append a focus border around the active pane.
    ///
    /// Draws four thin rectangles (top, bottom, left, right) into the cursor
    /// layer so the border renders on top of cell backgrounds and glyphs.
    /// `border_width` is in physical pixels (pass `(2.0 * scale).round()` for
    /// 2 logical pixels at any DPI).
    pub(crate) fn append_focus_border(&mut self, rect: &Rect, color: Rgb, border_width: f32) {
        let border = border_width;
        let bx = rect.x;
        let by = rect.y;
        let bw = rect.width;
        let bh = rect.height;

        // Top edge.
        self.prepared.cursors.push_cursor(
            ScreenRect {
                x: bx,
                y: by,
                w: bw,
                h: border,
            },
            color,
            1.0,
        );
        // Bottom edge.
        self.prepared.cursors.push_cursor(
            ScreenRect {
                x: bx,
                y: by + bh - border,
                w: bw,
                h: border,
            },
            color,
            1.0,
        );
        // Left edge.
        self.prepared.cursors.push_cursor(
            ScreenRect {
                x: bx,
                y: by,
                w: border,
                h: bh,
            },
            color,
            1.0,
        );
        // Right edge.
        self.prepared.cursors.push_cursor(
            ScreenRect {
                x: bx + bw - border,
                y: by,
                w: border,
                h: bh,
            },
            color,
            1.0,
        );
    }

    /// Append a window-edge border into the cursor layer.
    ///
    /// Draws four thin rectangles at the viewport edges, on top of all content.
    /// Used for the brutal design's 2px `border-strong` frame. Should be
    /// skipped when the window is maximized. Not used on macOS (native shadow).
    #[cfg(not(target_os = "macos"))]
    pub(crate) fn append_window_border(
        &mut self,
        viewport_w: u32,
        viewport_h: u32,
        color: Rgb,
        border_width: f32,
    ) {
        let w = viewport_w as f32;
        let h = viewport_h as f32;
        let b = border_width;
        for rect in [
            ScreenRect {
                x: 0.0,
                y: 0.0,
                w,
                h: b,
            },
            ScreenRect {
                x: 0.0,
                y: h - b,
                w,
                h: b,
            },
            ScreenRect {
                x: 0.0,
                y: 0.0,
                w: b,
                h,
            },
            ScreenRect {
                x: w - b,
                y: 0.0,
                w: b,
                h,
            },
        ] {
            self.prepared.cursors.push_cursor(rect, color, 1.0);
        }
    }
}
