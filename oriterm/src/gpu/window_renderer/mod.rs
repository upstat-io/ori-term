//! Per-window GPU renderer: owns fonts, atlases, shaping caches, and instance buffers.
//!
//! [`WindowRenderer`] holds all GPU resources specific to a single window.
//! Each window gets its own renderer so DPI scaling, atlas caches, and
//! shaping state are fully isolated — no cross-window contamination.

mod error;
mod font_config;
mod helpers;
mod icons;
mod multi_pane;
mod render;
mod render_helpers;
mod scene_append;
mod ui_only;

pub use error::SurfaceError;
pub use ui_only::RendererMode;

use std::collections::HashSet;
use wgpu::{Buffer, Device};

use oriterm_core::Rgb;

use oriterm_ui::icons::ResolvedIcons;

use super::atlas::GlyphAtlas;
use super::bind_groups::{AtlasBindGroup, UniformBuffer};
use super::frame_input::FrameInput;
use super::icon_rasterizer::IconCache;
use super::image_render::ImageTextureCache;
use super::pipelines::GpuPipelines;
use super::prepare::{self, AtlasLookup};
use super::prepared_frame::PreparedFrame;
use super::state::GpuState;
use crate::font::{CellMetrics, FontCollection, RasterKey, UiFontSizes};
use crate::gpu::frame_input::ViewportSize;
use helpers::{
    ShapingScratch, create_atlases, ensure_glyphs_cached, grid_raster_keys, shape_frame,
};

/// Maximum entries in `empty_keys` before clearing to prevent unbounded growth.
const EMPTY_KEYS_CAP: usize = 10_000;

// Atlas lookup bridge

/// Bridges all atlases (mono, subpixel, color) into the [`AtlasLookup`] trait.
///
/// During the Prepare phase, glyph lookups probe the monochrome atlas first
/// (most glyphs are mono text), then the subpixel atlas, then the color atlas.
/// Each entry carries an [`AtlasKind`](super::atlas::AtlasKind) that the
/// prepare phase uses to route glyphs to the correct instance buffer.
struct CombinedAtlasLookup<'a> {
    mono: &'a GlyphAtlas,
    subpixel: &'a GlyphAtlas,
    color: &'a GlyphAtlas,
}

impl AtlasLookup for CombinedAtlasLookup<'_> {
    fn lookup_key(&self, key: RasterKey) -> Option<&super::atlas::AtlasEntry> {
        self.mono
            .lookup(key)
            .or_else(|| self.subpixel.lookup(key))
            .or_else(|| self.color.lookup(key))
    }
}

/// Per-window GPU renderer: owns fonts, atlases, and instance buffers.
///
/// Created per-window at window creation time. Holds the bind groups,
/// glyph atlases (monochrome + subpixel + color), font collection, and
/// per-frame GPU buffers. Pipelines are shared via [`GpuPipelines`].
pub struct WindowRenderer {
    /// Rendering mode: full terminal or UI-only (dialogs).
    mode: RendererMode,

    // Bind groups (per-window, created with layouts from GpuPipelines).
    uniform_buffer: UniformBuffer,
    atlas_bind_group: AtlasBindGroup,
    subpixel_atlas_bind_group: AtlasBindGroup,
    color_atlas_bind_group: AtlasBindGroup,

    // Atlases + fonts (per-window, own DPI/rasterization).
    atlas: GlyphAtlas,
    subpixel_atlas: GlyphAtlas,
    color_atlas: GlyphAtlas,
    /// Last-seen atlas generations for bind group staleness detection.
    atlas_generation: u64,
    subpixel_atlas_generation: u64,
    color_atlas_generation: u64,
    /// Keys known to produce zero-size glyphs (spaces, non-printing chars).
    ///
    /// Cross-atlas: a glyph that fails rasterization produces no bitmap
    /// regardless of target atlas. Owned here rather than per-atlas so
    /// all three atlases share a single authoritative set.
    empty_keys: HashSet<RasterKey>,
    font_collection: FontCollection,
    /// Per-size UI font registry (proportional sans-serif) for tab bar, labels, and overlays.
    ///
    /// `None` if no UI font was found — falls back to terminal font.
    ui_font_sizes: Option<UiFontSizes>,

    // Per-frame reusable scratch buffers.
    ui_raster_keys: Vec<RasterKey>,
    shaping: ShapingScratch,
    /// GPU-ready instances for the current frame.
    ///
    /// Exposed to `app::redraw` so the pane render cache can merge cached
    /// per-pane instances into the aggregate frame.
    pub(crate) prepared: PreparedFrame,

    // Per-frame GPU instance buffers (grow-only, never shrink).
    bg_buffer: Option<Buffer>,
    fg_buffer: Option<Buffer>,
    subpixel_fg_buffer: Option<Buffer>,
    color_fg_buffer: Option<Buffer>,
    cursor_buffer: Option<Buffer>,
    ui_rect_buffer: Option<Buffer>,
    ui_fg_buffer: Option<Buffer>,
    ui_subpixel_fg_buffer: Option<Buffer>,
    ui_color_fg_buffer: Option<Buffer>,
    overlay_rect_buffer: Option<Buffer>,
    overlay_fg_buffer: Option<Buffer>,
    overlay_subpixel_fg_buffer: Option<Buffer>,
    overlay_color_fg_buffer: Option<Buffer>,

    // Icon rendering.
    icon_cache: IconCache,
    /// Reusable scratch buffer for pre-resolved icon atlas entries (avoids per-frame allocation).
    resolved_icons: ResolvedIcons,

    // Image rendering.
    image_texture_cache: ImageTextureCache,
    image_instance_buffer: Option<Buffer>,
    /// Reusable scratch buffer for image quad instance data (avoids per-frame allocation).
    image_instance_data: Vec<u8>,

    // Content cache: offscreen texture holding the fully rendered frame
    // without the cursor. On cursor-blink-only redraws, the cache is
    // copied to the surface and only the cursor is re-rendered on top,
    // avoiding the full GPU submission.
    content_cache: Option<wgpu::Texture>,
    content_cache_view: Option<wgpu::TextureView>,
    content_cache_size: (u32, u32),
}

impl WindowRenderer {
    /// Create a per-window renderer using shared layouts from [`GpuPipelines`].
    pub fn new(
        gpu: &GpuState,
        pipelines: &GpuPipelines,
        mut font_collection: FontCollection,
        mut ui_font_sizes: Option<UiFontSizes>,
    ) -> Self {
        let t0 = std::time::Instant::now();
        let device = &gpu.device;
        let queue = &gpu.queue;

        // Uniform buffer.
        let uniform_buffer = UniformBuffer::new(device, &pipelines.uniform_layout);

        // Atlases: mono + subpixel (with ASCII pre-cached) + color (empty).
        let (mut atlas, mut subpixel_atlas, color_atlas) =
            create_atlases(device, queue, &mut font_collection);

        // Pre-cache common UI font sizes so the first dialog/tab-bar frame
        // doesn't hitch on glyph rasterization.
        if let Some(ref mut sizes) = ui_font_sizes {
            let t_ui = std::time::Instant::now();
            helpers::prewarm_ui_font_sizes(sizes, &mut atlas, &mut subpixel_atlas, device, queue);
            log::info!("UI font prewarm: {:?}", t_ui.elapsed());
        }

        // Bind groups.
        let atlas_bind_group = AtlasBindGroup::new(device, &pipelines.atlas_layout, atlas.view());
        let subpixel_atlas_bind_group =
            AtlasBindGroup::new(device, &pipelines.atlas_layout, subpixel_atlas.view());
        let color_atlas_bind_group =
            AtlasBindGroup::new(device, &pipelines.atlas_layout, color_atlas.view());

        log::info!("window renderer init: total={:?}", t0.elapsed(),);

        Self {
            mode: RendererMode::Terminal,
            uniform_buffer,
            atlas_bind_group,
            subpixel_atlas_bind_group,
            color_atlas_bind_group,
            atlas,
            subpixel_atlas,
            color_atlas,
            atlas_generation: 0,
            subpixel_atlas_generation: 0,
            color_atlas_generation: 0,
            empty_keys: HashSet::new(),
            font_collection,
            ui_font_sizes,
            ui_raster_keys: Vec::new(),
            shaping: ShapingScratch::new(),
            prepared: PreparedFrame::new(ViewportSize::new(1, 1), Rgb { r: 0, g: 0, b: 0 }, 1.0),
            bg_buffer: None,
            fg_buffer: None,
            subpixel_fg_buffer: None,
            color_fg_buffer: None,
            cursor_buffer: None,
            ui_rect_buffer: None,
            ui_fg_buffer: None,
            ui_subpixel_fg_buffer: None,
            ui_color_fg_buffer: None,
            overlay_rect_buffer: None,
            overlay_fg_buffer: None,
            overlay_subpixel_fg_buffer: None,
            overlay_color_fg_buffer: None,
            icon_cache: IconCache::new(),
            resolved_icons: ResolvedIcons::new(),
            image_texture_cache: ImageTextureCache::new(device),
            image_instance_buffer: None,
            image_instance_data: Vec::new(),
            content_cache: None,
            content_cache_view: None,
            content_cache_size: (0, 0),
        }
    }

    // Accessors

    /// Cell dimensions derived from the current font metrics.
    pub fn cell_metrics(&self) -> CellMetrics {
        self.font_collection.cell_metrics()
    }

    /// Primary font family name.
    pub fn family_name(&self) -> &str {
        self.font_collection.family_name()
    }

    /// Active UI font collection (proportional sans-serif, or terminal font fallback).
    ///
    /// Returns the default-size collection from the UI font registry,
    /// falling back to the terminal font if no UI fonts are available.
    pub fn active_ui_collection(&self) -> &FontCollection {
        self.ui_font_sizes
            .as_ref()
            .and_then(|s| s.default_collection())
            .unwrap_or(&self.font_collection)
    }

    /// Create a size-aware text measurer for UI widgets.
    ///
    /// The returned measurer selects the exact [`FontCollection`] for each
    /// `TextStyle.size` from the UI font registry. Falls back to the default
    /// UI collection (or terminal font) for sizes not in the registry.
    pub fn ui_measurer(&self, scale: f32) -> crate::font::UiFontMeasurer<'_> {
        crate::font::UiFontMeasurer::new(
            self.ui_font_sizes.as_ref(),
            self.active_ui_collection(),
            scale,
        )
    }

    /// Glyph atlas for cache statistics.
    #[allow(dead_code, reason = "atlas access for diagnostics and Section 6")]
    pub fn atlas(&self) -> &GlyphAtlas {
        &self.atlas
    }

    // Bind group staleness

    /// Rebuild atlas bind groups whose texture generation has advanced.
    ///
    /// Called at the start of render passes. When an atlas grows (new page
    /// allocated), its texture and view are replaced. The bind group
    /// referencing the old view becomes stale and must be recreated.
    pub(crate) fn rebuild_stale_atlas_bind_groups(
        &mut self,
        device: &Device,
        atlas_layout: &wgpu::BindGroupLayout,
    ) {
        if self.atlas.generation() != self.atlas_generation {
            self.atlas_bind_group
                .rebuild(device, atlas_layout, self.atlas.view());
            self.atlas_generation = self.atlas.generation();
        }
        if self.subpixel_atlas.generation() != self.subpixel_atlas_generation {
            self.subpixel_atlas_bind_group.rebuild(
                device,
                atlas_layout,
                self.subpixel_atlas.view(),
            );
            self.subpixel_atlas_generation = self.subpixel_atlas.generation();
        }
        if self.color_atlas.generation() != self.color_atlas_generation {
            self.color_atlas_bind_group
                .rebuild(device, atlas_layout, self.color_atlas.view());
            self.color_atlas_generation = self.color_atlas.generation();
        }
    }

    // Frame preparation

    /// Whether visual-only state (selection) changed since the last frame.
    ///
    /// Visual changes need a full instance rebuild but NOT re-shaping.
    fn has_visual_change(&self, input: &FrameInput) -> bool {
        let new_sel = input
            .selection
            .as_ref()
            .and_then(|s| s.damage_snapshot(input.rows()));
        new_sel != self.prepared.prev_selection_snapshot
    }

    /// Run the Prepare phase: shape text and build GPU instance buffers.
    ///
    /// Fills `self.prepared` via buffer reuse (no per-frame allocation after
    /// the first frame).
    ///
    /// The `origin` offset positions the grid on screen (from layout). The
    /// `cursor_blink_visible` flag gates cursor emission (from application
    /// blink state) — when `false`, no cursor instances are emitted even
    /// if the terminal reports the cursor as visible.
    ///
    /// When `content_changed` is false the shaping phase is skipped entirely,
    /// reusing the previous frame's [`ShapedFrame`]. Decorations (cursor,
    /// selection, URL hover) only affect the prepare phase, so they work
    /// correctly with cached shaping data.
    ///
    /// Three phases:
    /// 1. **Shape** — segment rows into runs and shape via rustybuzz.
    /// 2. **Cache** — rasterize and upload any missing shaped glyphs.
    /// 3. **Prepare** — emit GPU instances from shaped glyph positions.
    #[expect(
        clippy::too_many_arguments,
        reason = "origin + cursor blink + content_changed are pipeline context"
    )]
    pub fn prepare(
        &mut self,
        input: &FrameInput,
        gpu: &GpuState,
        pipelines: &GpuPipelines,
        origin: (f32, f32),
        cursor_blink_visible: bool,
        content_changed: bool,
    ) {
        // Cursor-blink-only fast path: when content hasn't changed and no
        // visual state (selection, search, hover) differs from the last
        // prepared frame, skip shaping, glyph caching, and the full instance
        // rebuild. Just update cursor/URL/prompt overlays.
        let cols = input.columns();
        let cached_valid = self.shaping.frame.rows() > 0 && self.shaping.frame.cols() == cols;
        let visual_changed = self.has_visual_change(input);
        if !content_changed && !visual_changed && cached_valid && self.prepared.has_terminal_data()
        {
            self.atlas.begin_frame();
            self.subpixel_atlas.begin_frame();
            self.color_atlas.begin_frame();
            self.prepared.clear_ephemeral_tiers();
            prepare::update_cursor_only(input, &mut self.prepared, origin, cursor_blink_visible);
            return;
        }

        self.atlas.begin_frame();
        self.subpixel_atlas.begin_frame();
        self.color_atlas.begin_frame();

        // Phase A: Shape all rows, or reuse cached shaping when content
        // hasn't changed (mouse hover, cursor blink, selection changes
        // only affect the prepare phase).
        if content_changed || !cached_valid {
            shape_frame(input, &self.font_collection, &mut self.shaping);
        }

        // Phase B: Ensure shaped glyphs cached (routes to mono, subpixel, or color atlas).
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

        // Phase B2: Ensure built-in geometric glyphs + decoration patterns cached.
        // Built-ins always go to the mono atlas (alpha-only bitmaps).
        super::builtin_glyphs::ensure_builtins_cached(
            input,
            self.shaping.frame.size_q6(),
            &mut self.atlas,
            &mut self.empty_keys,
            &gpu.device,
            &gpu.queue,
        );

        // Phase C: Fill prepared frame via combined atlas lookup bridge.
        let bridge = CombinedAtlasLookup {
            mono: &self.atlas,
            subpixel: &self.subpixel_atlas,
            color: &self.color_atlas,
        };
        prepare::prepare_frame_shaped_into(
            input,
            &bridge,
            &self.shaping.frame,
            &mut self.prepared,
            origin,
            cursor_blink_visible,
        );

        // Phase D: Ensure image textures uploaded.
        self.upload_image_textures(input, gpu, pipelines);

        log::trace!(
            "frame: cells={} bg_inst={} glyph_inst={} cursor_inst={} images={}",
            input.content.cells.len(),
            self.prepared.backgrounds.len(),
            self.prepared.glyphs.len(),
            self.prepared.cursors.len(),
            self.prepared.image_quads_below.len() + self.prepared.image_quads_above.len(),
        );
    }

    /// Upload image textures for the current frame.
    ///
    /// Ensures all images referenced by the prepared frame have GPU textures.
    /// Evicts textures that haven't been used recently.
    fn upload_image_textures(
        &mut self,
        input: &FrameInput,
        gpu: &GpuState,
        pipelines: &GpuPipelines,
    ) {
        self.image_texture_cache.begin_frame();

        // Upload textures for all visible images.
        for img_data in &input.content.image_data {
            self.image_texture_cache.ensure_uploaded(
                &gpu.device,
                &gpu.queue,
                &pipelines.image_texture_layout,
                img_data.id,
                &img_data.data,
                img_data.width,
                img_data.height,
            );
        }

        // Evict textures not used in the last 60 frames (~1 second at 60fps).
        self.image_texture_cache.evict_unused(60);
        self.image_texture_cache.evict_over_limit();
    }

    /// Update the GPU memory limit for image textures.
    ///
    /// Triggers immediate eviction if current usage exceeds the new limit.
    pub fn set_image_gpu_memory_limit(&mut self, limit: usize) {
        self.image_texture_cache.set_gpu_memory_limit(limit);
    }

    /// Shrink grow-only buffers if capacity vastly exceeds usage.
    ///
    /// Called after rendering to bound memory waste to 2× actual usage.
    /// Also caps `empty_keys` at 10,000 entries to prevent unbounded growth
    /// from pathological glyph-missing scenarios.
    pub fn maybe_shrink_buffers(&mut self) {
        self.prepared.maybe_shrink();
        self.shaping.maybe_shrink();
        if self.empty_keys.len() > EMPTY_KEYS_CAP {
            self.empty_keys.clear();
        }
    }
}

#[cfg(test)]
mod tests;
