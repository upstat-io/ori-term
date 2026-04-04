//! Per-window GPU renderer: owns fonts, atlases, shaping caches, and instance buffers.
//!
//! [`WindowRenderer`] holds all GPU resources specific to a single window.
//! Each window gets its own renderer so DPI scaling, atlas caches, and
//! shaping state are fully isolated — no cross-window contamination.

mod error;
mod font_config;
mod frame_prep;
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
use wgpu::{Buffer, Device, FilterMode};

use oriterm_core::Rgb;

use oriterm_ui::icons::ResolvedIcons;

use super::atlas::GlyphAtlas;
use super::bind_groups::{AtlasBindGroup, UniformBuffer};
use super::icon_rasterizer::IconCache;
use super::image_render::ImageTextureCache;
use super::pipelines::GpuPipelines;
use super::prepared_frame::PreparedFrame;
use super::state::GpuState;
use crate::font::{CellMetrics, FontCollection, RasterKey, UiFontSizes};
use crate::gpu::bind_groups::AtlasFiltering;
use crate::gpu::frame_input::ViewportSize;
use helpers::{CombinedAtlasLookup, ShapingScratch, create_atlases};

/// Maximum entries in `empty_keys` before clearing to prevent unbounded growth.
const EMPTY_KEYS_CAP: usize = 10_000;

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

    // Rendering configuration (resolved from user config + auto-detection).
    subpixel_positioning: bool,
    atlas_filtering: AtlasFiltering,

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

        // Inject terminal font's emoji fallback into UI font collections
        // so emoji renders at the correct UI text size (not the terminal's).
        if let Some(ref mut sizes) = ui_font_sizes {
            let emoji_data = font_collection.fallback_font_data();
            if !emoji_data.is_empty() {
                sizes.inject_fallbacks(&emoji_data);
            }
        }

        // Pre-cache common UI font sizes so the first dialog/tab-bar frame
        // doesn't hitch on glyph rasterization.
        if let Some(ref mut sizes) = ui_font_sizes {
            let t_ui = std::time::Instant::now();
            helpers::prewarm_ui_font_sizes(sizes, &mut atlas, &mut subpixel_atlas, device, queue);
            log::info!("UI font prewarm: {:?}", t_ui.elapsed());
        }

        // Bind groups (default to Linear; set_atlas_filtering() overrides after init).
        let filter = FilterMode::Linear;
        let atlas_bind_group =
            AtlasBindGroup::new(device, &pipelines.atlas_layout, atlas.view(), filter);
        let subpixel_atlas_bind_group = AtlasBindGroup::new(
            device,
            &pipelines.atlas_layout,
            subpixel_atlas.view(),
            filter,
        );
        let color_atlas_bind_group =
            AtlasBindGroup::new(device, &pipelines.atlas_layout, color_atlas.view(), filter);

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
            subpixel_positioning: true,
            atlas_filtering: AtlasFiltering::default(),
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
        .with_terminal_collection(&self.font_collection)
    }

    /// Monochrome glyph atlas.
    pub fn atlas(&self) -> &GlyphAtlas {
        &self.atlas
    }

    /// Subpixel glyph atlas.
    pub fn subpixel_atlas(&self) -> &GlyphAtlas {
        &self.subpixel_atlas
    }

    /// Color glyph atlas (emoji).
    pub fn color_atlas(&self) -> &GlyphAtlas {
        &self.color_atlas
    }

    /// UI font sizes registry (GPU-test accessor).
    #[cfg(feature = "gpu-tests")]
    #[allow(dead_code, reason = "used by gpu-tests feature gate")]
    pub(crate) fn ui_font_sizes(&self) -> Option<&UiFontSizes> {
        self.ui_font_sizes.as_ref()
    }

    /// Terminal font collection (GPU-test accessor).
    #[cfg(feature = "gpu-tests")]
    #[allow(dead_code, reason = "used by gpu-tests feature gate")]
    pub(crate) fn font_collection(&self) -> &FontCollection {
        &self.font_collection
    }

    /// Number of cached entries in the primary (grayscale) atlas.
    #[cfg(feature = "gpu-tests")]
    #[allow(dead_code, reason = "used by gpu-tests feature gate")]
    pub(crate) fn atlas_entry_count(&self) -> usize {
        self.atlas.len()
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

    // Frame preparation, image upload, and buffer shrinking live in `frame_prep.rs`.
}

#[cfg(test)]
mod tests;
