//! Free functions for the shaping pipeline and GPU buffer management.
//!
//! These are free functions (not methods) so the borrow checker can see
//! that different fields of [`WindowRenderer`](super::WindowRenderer) are
//! borrowed independently — e.g. `font_collection` immutably while
//! `scratch` is borrowed mutably.

use std::collections::HashSet;

use wgpu::{
    BindGroup, Buffer, BufferDescriptor, BufferUsages, Device, Queue, RenderPass, RenderPipeline,
};

use super::super::atlas::GlyphAtlas;
use super::super::frame_input::FrameInput;
use super::super::maybe_shrink_vec;
use super::super::prepare::ShapedFrame;
use crate::font::{
    FontCollection, FontRealm, GlyphFormat, GlyphStyle, RasterKey, build_col_glyph_map,
    prepare_line, shape_prepared_runs, size_key,
};

use super::super::prepare::AtlasLookup;

// Atlas lookup bridge

/// Bridges all atlases (mono, subpixel, color) into the [`AtlasLookup`] trait.
///
/// During the Prepare phase, glyph lookups probe the monochrome atlas first
/// (most glyphs are mono text), then the subpixel atlas, then the color atlas.
/// Each entry carries an [`AtlasKind`](super::super::atlas::AtlasKind) that the
/// prepare phase uses to route glyphs to the correct instance buffer.
pub(super) struct CombinedAtlasLookup<'a> {
    pub(super) mono: &'a GlyphAtlas,
    pub(super) subpixel: &'a GlyphAtlas,
    pub(super) color: &'a GlyphAtlas,
}

impl AtlasLookup for CombinedAtlasLookup<'_> {
    fn lookup_key(&self, key: RasterKey) -> Option<&super::super::atlas::AtlasEntry> {
        self.mono
            .lookup(key)
            .or_else(|| self.subpixel.lookup(key))
            .or_else(|| self.color.lookup(key))
    }
}

/// Reusable per-frame scratch buffers for the shaping pipeline.
///
/// Stored on [`WindowRenderer`](super::WindowRenderer) and cleared each frame to
/// avoid per-frame allocation of the shaping intermediaries and output.
pub(super) struct ShapingScratch {
    /// Shaped frame output (glyph positions + col maps).
    pub(super) frame: ShapedFrame,
    /// Shaping run segments for the current row.
    runs: Vec<crate::font::ShapingRun>,
    /// Shaped glyphs for the current row.
    glyphs: Vec<oriterm_ui::text::ShapedGlyph>,
    /// Parallel `col_starts` for the current row.
    col_starts: Vec<usize>,
    /// Column-to-glyph map for the current row.
    col_map: Vec<Option<usize>>,
    /// Rustybuzz buffer reused across frames to avoid per-frame allocation.
    unicode_buffer: Option<rustybuzz::UnicodeBuffer>,
    /// Rustybuzz Face objects reused across frames.
    ///
    /// Stored with `'static` lifetime because `ShapingScratch` has no lifetime
    /// parameter. Filled via [`FontCollection::fill_shaping_faces`] which
    /// transmutes the actual `'a` borrow to `'static`. This is sound because
    /// the Vec is cleared before every fill and only accessed while
    /// `FontCollection` is borrowed (within `shape_frame`).
    faces_buf: Vec<Option<rustybuzz::Face<'static>>>,
}

impl ShapingScratch {
    pub(super) fn new() -> Self {
        Self {
            frame: ShapedFrame::new(0, 0),
            runs: Vec::new(),
            glyphs: Vec::new(),
            col_starts: Vec::new(),
            col_map: Vec::new(),
            unicode_buffer: None,
            faces_buf: Vec::new(),
        }
    }

    /// Shrink per-row scratch buffers if capacity vastly exceeds usage.
    ///
    /// Called after rendering to bound memory waste. Only fires when
    /// capacity > 4× length AND > 4096 elements.
    pub(super) fn maybe_shrink(&mut self) {
        self.frame.maybe_shrink();
        maybe_shrink_vec(&mut self.runs);
        maybe_shrink_vec(&mut self.glyphs);
        maybe_shrink_vec(&mut self.col_starts);
        maybe_shrink_vec(&mut self.col_map);
        maybe_shrink_vec(&mut self.faces_buf);
    }
}

/// Shape all visible rows into the scratch `ShapedFrame`.
pub(super) fn shape_frame(
    input: &FrameInput,
    fonts: &FontCollection,
    scratch: &mut ShapingScratch,
) {
    let cols = input.columns();
    let size_q6 = size_key(fonts.size_px());
    let hinted = fonts.hinting_mode().hint_flag();
    scratch.frame.clear(cols, size_q6, hinted);
    if cols == 0 {
        return;
    }
    // Clamp rows to actual cell data — viewport dimensions may race ahead
    // of the terminal grid during async resize.
    let rows = input.rows().min(input.content.cells.len() / cols);
    fonts.fill_shaping_faces(&mut scratch.faces_buf);

    for row_idx in 0..rows {
        let start = row_idx * cols;
        let end = start + cols;
        let row_cells = &input.content.cells[start..end];

        prepare_line(row_cells, cols, fonts, &mut scratch.runs);
        shape_prepared_runs(
            &scratch.runs,
            &scratch.faces_buf,
            fonts,
            &mut scratch.glyphs,
            &mut scratch.col_starts,
            &mut scratch.unicode_buffer,
        );
        build_col_glyph_map(&scratch.col_starts, cols, &mut scratch.col_map);
        scratch
            .frame
            .push_row(&scratch.glyphs, &scratch.col_starts, &scratch.col_map);
    }
}

/// Ensure all glyphs from the given keys are cached in the appropriate atlas.
///
/// Routes glyphs by format:
/// - [`GlyphFormat::Color`] → `color_atlas`.
/// - [`GlyphFormat::SubpixelRgb`] / [`GlyphFormat::SubpixelBgr`] → `subpixel_atlas`.
/// - [`GlyphFormat::Alpha`] → `mono_atlas`.
///
/// `empty_keys` is a cross-atlas set of keys known to produce zero-size
/// glyphs. A glyph that fails rasterization produces no bitmap regardless
/// of target atlas, so this set is shared across all three.
///
/// Callers build [`RasterKey`] iterators from their specific context:
/// - Grid caller: iterates `ShapedFrame::all_glyphs()`, builds keys with
///   [`FontRealm::Terminal`] and `subpx_bin(glyph.x_offset)`.
/// - UI caller: iterates Scene text runs, builds keys with
///   [`FontRealm::Ui`] and `subpx_bin(cursor_x + glyph.x_offset)`.
#[expect(
    clippy::too_many_arguments,
    reason = "three atlases + empty set + fonts + device + queue for glyph routing"
)]
pub(super) fn ensure_glyphs_cached(
    keys: impl Iterator<Item = RasterKey>,
    mono_atlas: &mut GlyphAtlas,
    subpixel_atlas: &mut GlyphAtlas,
    color_atlas: &mut GlyphAtlas,
    empty_keys: &mut HashSet<RasterKey>,
    fonts: &mut FontCollection,
    device: &Device,
    queue: &Queue,
) {
    for key in keys {
        if mono_atlas.lookup_touch(key).is_some()
            || subpixel_atlas.lookup_touch(key).is_some()
            || color_atlas.lookup_touch(key).is_some()
        {
            continue;
        }
        if empty_keys.contains(&key) {
            continue;
        }
        let rasterized = if key.font_realm == FontRealm::Ui {
            fonts.rasterize_with_weight(key, key.weight)
        } else {
            fonts.rasterize(key)
        };
        if let Some(rasterized) = rasterized {
            match rasterized.format {
                GlyphFormat::Color => {
                    color_atlas.insert(key, rasterized, device, queue);
                }
                GlyphFormat::SubpixelRgb | GlyphFormat::SubpixelBgr => {
                    subpixel_atlas.insert(key, rasterized, device, queue);
                }
                GlyphFormat::Alpha => {
                    mono_atlas.insert(key, rasterized, device, queue);
                }
            }
        } else {
            empty_keys.insert(key);
        }
    }
}

/// Build [`RasterKey`] iterator from shaped terminal frame glyphs.
///
/// `hinted` should be `fonts.hinting_mode().hint_flag()` — passed as a
/// value so the caller can release the `FontCollection` borrow before
/// passing `&mut fonts` to [`ensure_glyphs_cached`].
pub(super) fn grid_raster_keys(
    shaped: &ShapedFrame,
    hinted: bool,
    subpixel_positioning: bool,
) -> impl Iterator<Item = RasterKey> + '_ {
    let size_q6 = shaped.size_q6();
    shaped.all_glyphs().iter().map(move |glyph| RasterKey {
        glyph_id: glyph.glyph_id,
        face_idx: crate::font::FaceIdx(glyph.face_index),
        weight: 0,
        size_q6,
        synthetic: crate::font::SyntheticFlags::from_bits_truncate(glyph.synthetic),
        hinted,
        subpx_x: if subpixel_positioning {
            crate::font::subpx_bin(glyph.x_offset)
        } else {
            0
        },
        font_realm: FontRealm::Terminal,
    })
}

/// Collect [`RasterKey`]s from Scene text runs into `keys`.
///
/// Each text run carries its own `size_q6` (stamped by the shaper from the
/// exact-size `FontCollection`), enabling mixed-size text in one scene.
/// The caller owns the buffer and should `clear()` before calling.
pub(super) fn scene_raster_keys(
    scene: &oriterm_ui::draw::Scene,
    hinted: bool,
    scale: f32,
    keys: &mut Vec<RasterKey>,
    subpixel_positioning: bool,
) {
    for text_run in scene.text_runs() {
        let run_size_q6 = text_run.shaped.size_q6;
        let realm = match text_run.shaped.font_source {
            oriterm_ui::text::FontSource::Terminal => FontRealm::Terminal,
            oriterm_ui::text::FontSource::Ui => FontRealm::Ui,
        };
        let mut cursor_x = text_run.position.x * scale;
        let cursor_x_ref = &mut cursor_x;
        for glyph in &text_run.shaped.glyphs {
            let advance = glyph.x_advance;
            if glyph.glyph_id == 0 {
                *cursor_x_ref += advance;
                continue;
            }
            let cx = if subpixel_positioning {
                *cursor_x_ref
            } else {
                cursor_x_ref.round()
            };
            keys.push(RasterKey {
                glyph_id: glyph.glyph_id,
                face_idx: crate::font::FaceIdx(glyph.face_index),
                weight: text_run.shaped.weight,
                size_q6: run_size_q6,
                synthetic: crate::font::SyntheticFlags::from_bits_truncate(glyph.synthetic),
                hinted,
                subpx_x: if subpixel_positioning {
                    crate::font::subpx_bin(cx + glyph.x_offset)
                } else {
                    0
                },
                font_realm: realm,
            });
            *cursor_x_ref += advance;
        }
    }
}

/// Ensure a GPU buffer exists, is large enough, and upload `data` to it.
///
/// No-ops when `data` is empty. Grows the buffer (power-of-2 amortized) when
/// needed, then writes the data in a single `write_buffer` call.
pub(super) fn upload_buffer(
    device: &Device,
    queue: &Queue,
    slot: &mut Option<Buffer>,
    data: &[u8],
    label: &'static str,
) {
    if data.is_empty() {
        return;
    }

    let needed = data.len() as u64;
    let should_recreate = match slot {
        Some(buf) => buf.size() < needed,
        None => true,
    };

    if should_recreate {
        // Round up to next power of 2 for amortized growth.
        let size = needed.next_power_of_two().max(256);
        *slot = Some(device.create_buffer(&BufferDescriptor {
            label: Some(label),
            size,
            usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        }));
    }

    if let Some(buf) = slot.as_ref() {
        queue.write_buffer(buf, 0, data);
    }
}

/// Record a single instanced draw call into the render pass.
///
/// Sets the pipeline, bind groups, and vertex buffer, then issues an
/// instanced `draw(0..4, 0..instance_count)`. No-ops when `instance_count`
/// is zero or the buffer slot is empty.
#[expect(
    clippy::too_many_arguments,
    reason = "GPU render pass recording: pipeline, bind groups, buffer, count"
)]
pub(super) fn record_draw(
    pass: &mut RenderPass<'_>,
    pipeline: &RenderPipeline,
    uniform_bg: &BindGroup,
    atlas_bg: Option<&BindGroup>,
    buffer: Option<&Buffer>,
    instance_count: u32,
) {
    if instance_count == 0 {
        return;
    }
    let Some(buf) = buffer else { return };
    pass.set_pipeline(pipeline);
    pass.set_bind_group(0, uniform_bg, &[]);
    if let Some(atlas) = atlas_bg {
        pass.set_bind_group(1, atlas, &[]);
    }
    pass.set_vertex_buffer(0, buf.slice(..));
    pass.draw(0..4, 0..instance_count);
}

/// Record an instanced draw call for a sub-range `[start..end)`.
///
/// Like [`record_draw`] but draws only instances in `[start..end)`.
/// Used for overlay draw ranges where each overlay occupies a contiguous
/// sub-range of the shared buffer.
#[expect(
    clippy::too_many_arguments,
    reason = "GPU render pass + range: pipeline, bind groups, buffer, range"
)]
pub(super) fn record_draw_range(
    pass: &mut RenderPass<'_>,
    pipeline: &RenderPipeline,
    uniform_bg: &BindGroup,
    atlas_bg: Option<&BindGroup>,
    buffer: Option<&Buffer>,
    start: u32,
    end: u32,
) {
    if start >= end {
        return;
    }
    let Some(buf) = buffer else { return };
    pass.set_pipeline(pipeline);
    pass.set_bind_group(0, uniform_bg, &[]);
    if let Some(atlas) = atlas_bg {
        pass.set_bind_group(1, atlas, &[]);
    }
    pass.set_vertex_buffer(0, buf.slice(..));
    pass.draw(0..4, start..end);
}

/// Pre-cache printable ASCII glyphs (Regular + Bold) into the given atlas.
///
/// Iterates 0x20–0x7E for Regular, then again for Bold if the collection has
/// a real Bold face. Used by both `WindowRenderer::new()` and `clear_and_recache()`.
/// Create mono, subpixel, and color atlases with ASCII pre-cached.
///
/// Routes the pre-cache into the subpixel atlas when the font format is
/// subpixel, otherwise into the mono atlas. Shared by `WindowRenderer::new()`
/// and `new_ui_only()`.
pub(super) fn create_atlases(
    device: &Device,
    queue: &Queue,
    font_collection: &mut FontCollection,
) -> (GlyphAtlas, GlyphAtlas, GlyphAtlas) {
    let format = font_collection.format();
    // The active atlas gets a full 2048² page with ASCII pre-cached.
    // The inactive atlas is lazy (1×1 placeholder) — materialized on first insert.
    let (atlas, subpixel_atlas) = if format.is_subpixel() {
        let atlas = GlyphAtlas::new_lazy(device, GlyphFormat::Alpha);
        let mut sp_atlas = GlyphAtlas::new(device, format);
        pre_cache_atlas(
            &mut sp_atlas,
            font_collection,
            FontRealm::Terminal,
            device,
            queue,
        );
        (atlas, sp_atlas)
    } else {
        let mut atlas = GlyphAtlas::new(device, GlyphFormat::Alpha);
        let sp_atlas = GlyphAtlas::new_lazy(device, GlyphFormat::SubpixelRgb);
        pre_cache_atlas(
            &mut atlas,
            font_collection,
            FontRealm::Terminal,
            device,
            queue,
        );
        (atlas, sp_atlas)
    };
    // Color atlas is lazy — no emoji at startup.
    let color_atlas = GlyphAtlas::new_lazy(device, GlyphFormat::Color);
    (atlas, subpixel_atlas, color_atlas)
}

/// Pre-cache printable ASCII for all UI font sizes into the appropriate atlas.
///
/// Iterates every collection in the [`UiFontSizes`] registry and calls
/// [`pre_cache_atlas`] for each. Routes into the subpixel atlas when the
/// font format is subpixel, otherwise the mono atlas.
pub(super) fn prewarm_ui_font_sizes(
    sizes: &mut crate::font::UiFontSizes,
    atlas: &mut GlyphAtlas,
    subpixel_atlas: &mut GlyphAtlas,
    device: &Device,
    queue: &Queue,
) {
    let is_subpixel = sizes.format().is_subpixel();
    let target = if is_subpixel { subpixel_atlas } else { atlas };
    for fc in sizes.collections_mut() {
        pre_cache_atlas(target, fc, FontRealm::Ui, device, queue);
    }
}

/// Pre-cache printable ASCII glyphs (Regular + Bold) into the given atlas.
///
/// Iterates 0x20–0x7E for Regular, then again for Bold if the collection has
/// a real Bold face. `realm` sets the [`FontRealm`] on each raster key so
/// cached entries match the lookup realm at render time.
///
/// For [`FontRealm::Ui`], keys carry the collection's configured weight (Regular)
/// or 700 (Bold), and rasterization uses [`FontCollection::rasterize_with_weight`]
/// so the prewarmed entries match the weight-aware keys produced by
/// [`scene_raster_keys`] at render time.
pub(super) fn pre_cache_atlas(
    atlas: &mut GlyphAtlas,
    fc: &mut FontCollection,
    realm: FontRealm,
    device: &Device,
    queue: &Queue,
) {
    let size_q6 = size_key(fc.size_px());
    let hinted = fc.hinting_mode().hint_flag();
    let is_ui = realm == FontRealm::Ui;
    let regular_weight = if is_ui { fc.weight() } else { 0 };

    for ch in ' '..='~' {
        let resolved = fc.resolve(ch, GlyphStyle::Regular);
        let mut key = RasterKey::from_resolved(resolved, size_q6, hinted, 0).with_realm(realm);
        key.weight = regular_weight;
        let glyph = if is_ui {
            fc.rasterize_with_weight(key, regular_weight)
        } else {
            fc.rasterize(key)
        };
        if let Some(glyph) = glyph {
            atlas.insert(key, glyph, device, queue);
        }
    }
    if fc.has_bold() {
        let bold_weight = if is_ui { 700 } else { 0 };
        for ch in ' '..='~' {
            let resolved = fc.resolve(ch, GlyphStyle::Bold);
            let mut key = RasterKey::from_resolved(resolved, size_q6, hinted, 0).with_realm(realm);
            key.weight = bold_weight;
            let glyph = if is_ui {
                fc.rasterize_with_weight(key, bold_weight)
            } else {
                fc.rasterize(key)
            };
            if let Some(glyph) = glyph {
                atlas.insert(key, glyph, device, queue);
            }
        }
    }
    // Prewarm Regular-slot 700-weight keys for variable fonts that express
    // weight via wght axis. resolve_ui_weight() always uses Regular slot when
    // has_wght_axis() is true, regardless of whether a Bold face also exists,
    // so these keys must be prewarmed for any wght-capable font.
    if is_ui && fc.has_wght_axis() {
        for ch in ' '..='~' {
            let resolved = fc.resolve(ch, GlyphStyle::Regular);
            let mut key = RasterKey::from_resolved(resolved, size_q6, hinted, 0).with_realm(realm);
            key.weight = 700;
            if let Some(glyph) = fc.rasterize_with_weight(key, 700) {
                atlas.insert(key, glyph, device, queue);
            }
        }
    }
}
