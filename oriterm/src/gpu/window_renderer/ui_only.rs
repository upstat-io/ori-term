//! `UiOnly` renderer mode for dialog windows.
//!
//! Dialog windows only need UI rendering (rects, text, icons) — no terminal
//! grid, cursor, or image rendering. This constructor creates a lighter
//! `WindowRenderer` that skips terminal-specific resources.

use std::collections::HashSet;

use oriterm_core::Rgb;
use oriterm_ui::icons::ResolvedIcons;

use super::WindowRenderer;
use super::helpers::ShapingScratch;
use crate::font::FontCollection;
use crate::gpu::bind_groups::{AtlasBindGroup, UniformBuffer};
use crate::gpu::frame_input::ViewportSize;
use crate::gpu::icon_rasterizer::IconCache;
use crate::gpu::image_render::ImageTextureCache;
use crate::gpu::pipelines::GpuPipelines;
use crate::gpu::prepared_frame::PreparedFrame;
use crate::gpu::state::GpuState;

/// Rendering mode discriminant.
///
/// Stored on [`WindowRenderer`] to select the render pipeline.
/// Terminal mode creates all buffers; `UiOnly` mode skips grid-specific
/// resources (terminal font shaping, cursor, image textures).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(
    dead_code,
    reason = "UiOnly variant used by Section 04 dialog window system"
)]
pub enum RendererMode {
    /// Full terminal renderer with grid, cursor, images.
    Terminal,
    /// UI-only renderer for dialog windows (rects, text, icons).
    UiOnly,
}

impl WindowRenderer {
    /// Create a UI-only renderer for dialog windows.
    ///
    /// Uses `ui_font_collection` as the primary font (the terminal font
    /// field). `active_ui_collection()` falls back to `font_collection`
    /// when `ui_font_collection` is `None`, so UI text shaping uses the
    /// same font either way.
    ///
    /// Terminal instance buffers remain `None` — the render pipeline
    /// naturally skips draws for absent buffers.
    pub fn new_ui_only(
        gpu: &GpuState,
        pipelines: &GpuPipelines,
        mut ui_font_collection: FontCollection,
    ) -> Self {
        let device = &gpu.device;
        let queue = &gpu.queue;

        let uniform_buffer = UniformBuffer::new(device, &pipelines.uniform_layout);

        // Atlases: mono + subpixel (with ASCII pre-cached) + color (empty).
        let (atlas, subpixel_atlas, color_atlas) =
            super::helpers::create_atlases(device, queue, &mut ui_font_collection);

        let atlas_bind_group = AtlasBindGroup::new(device, &pipelines.atlas_layout, atlas.view());
        let subpixel_atlas_bind_group =
            AtlasBindGroup::new(device, &pipelines.atlas_layout, subpixel_atlas.view());
        let color_atlas_bind_group =
            AtlasBindGroup::new(device, &pipelines.atlas_layout, color_atlas.view());

        log::info!("window renderer init (ui-only)");

        Self {
            mode: RendererMode::UiOnly,
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
            // UI font serves as primary — terminal shaping won't be invoked.
            font_collection: ui_font_collection,
            ui_font_collection: None,
            ui_raster_keys: Vec::new(),
            shaping: ShapingScratch::new(),
            prepared: PreparedFrame::new(ViewportSize::new(1, 1), Rgb { r: 0, g: 0, b: 0 }, 1.0),
            // Terminal buffers intentionally None — render skips absent draws.
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

    /// Whether this renderer is in UI-only mode (dialog windows).
    #[allow(dead_code, reason = "used by Section 04 dialog window system")]
    pub fn is_ui_only(&self) -> bool {
        self.mode == RendererMode::UiOnly
    }

    /// Prepare a UI-only frame for dialog rendering.
    ///
    /// Clears all instance buffers, sets the viewport and background color,
    /// and begins atlas frame tracking. After this call, the caller appends
    /// scenes via [`append_ui_scene_with_text`] then calls
    /// [`render_to_surface`].
    pub fn prepare_ui_frame(&mut self, width: u32, height: u32, background: Rgb, opacity: f64) {
        self.prepared.viewport = ViewportSize::new(width, height);
        self.prepared.set_clear_color(background, opacity);
        self.prepared.clear();
        self.atlas.begin_frame();
        self.subpixel_atlas.begin_frame();
        self.color_atlas.begin_frame();
    }
}
