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
/// - UI caller: iterates `DrawList` text commands, builds keys with
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
        if let Some(rasterized) = fonts.rasterize(key) {
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
) -> impl Iterator<Item = RasterKey> + '_ {
    let size_q6 = shaped.size_q6();
    shaped.all_glyphs().iter().map(move |glyph| RasterKey {
        glyph_id: glyph.glyph_id,
        face_idx: crate::font::FaceIdx(glyph.face_index),
        size_q6,
        synthetic: crate::font::SyntheticFlags::from_bits_truncate(glyph.synthetic),
        hinted,
        subpx_x: crate::font::subpx_bin(glyph.x_offset),
        font_realm: FontRealm::Terminal,
    })
}

/// Collect [`RasterKey`]s from UI draw list text commands into `keys`.
///
/// The caller owns the buffer and should `clear()` before calling.
/// Reusing the same `Vec` across frames avoids per-frame allocation.
pub(super) fn ui_text_raster_keys(
    draw_list: &oriterm_ui::draw::DrawList,
    size_q6: u32,
    hinted: bool,
    scale: f32,
    keys: &mut Vec<RasterKey>,
) {
    for cmd in draw_list.commands() {
        let oriterm_ui::draw::DrawCommand::Text {
            position, shaped, ..
        } = cmd
        else {
            continue;
        };
        let mut cursor_x = position.x * scale;
        for glyph in &shaped.glyphs {
            let advance = glyph.x_advance;
            if glyph.glyph_id == 0 {
                cursor_x += advance;
                continue;
            }
            keys.push(RasterKey {
                glyph_id: glyph.glyph_id,
                face_idx: crate::font::FaceIdx(glyph.face_index),
                size_q6,
                synthetic: crate::font::SyntheticFlags::from_bits_truncate(glyph.synthetic),
                hinted,
                subpx_x: crate::font::subpx_bin(cursor_x + glyph.x_offset),
                font_realm: FontRealm::Ui,
            });
            cursor_x += advance;
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

/// Record an instanced draw call with scissor rect splitting.
///
/// Like [`record_draw`] but splits the draw into sub-ranges at each
/// [`ClipSegment`] boundary, calling `set_scissor_rect` between them.
/// Resets the scissor to the full viewport after the last segment.
///
/// When `clips` is empty, behaves identically to a single full draw.
#[expect(
    clippy::too_many_arguments,
    reason = "GPU render pass + clip segments: pipeline, bind groups, buffer, count, clips, viewport"
)]
pub(super) fn record_draw_clipped(
    pass: &mut RenderPass<'_>,
    pipeline: &RenderPipeline,
    uniform_bg: &BindGroup,
    atlas_bg: Option<&BindGroup>,
    buffer: Option<&Buffer>,
    instance_count: u32,
    clips: &[super::super::draw_list_convert::ClipSegment],
    viewport_w: u32,
    viewport_h: u32,
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

    if clips.is_empty() {
        pass.draw(0..4, 0..instance_count);
        return;
    }

    let mut cursor = 0u32;
    for seg in clips {
        // Draw instances before this clip change.
        if seg.instance_offset > cursor {
            pass.draw(0..4, cursor..seg.instance_offset);
        }
        // Apply new scissor.
        if let Some(r) = seg.rect {
            pass.set_scissor_rect(r[0], r[1], r[2], r[3]);
        } else {
            pass.set_scissor_rect(0, 0, viewport_w, viewport_h);
        }
        cursor = seg.instance_offset;
    }
    // Draw remaining instances after the last clip change.
    if cursor < instance_count {
        pass.draw(0..4, cursor..instance_count);
    }
    // Reset scissor to full viewport.
    pass.set_scissor_rect(0, 0, viewport_w, viewport_h);
}

/// Record an instanced draw call for a sub-range `[start..end)` with clips.
///
/// Like [`record_draw_clipped`] but draws only instances in `[start..end)`.
/// Clip segment offsets are absolute (matching the buffer's instance indices).
#[expect(
    clippy::too_many_arguments,
    reason = "GPU render pass + range: pipeline, bind groups, buffer, range, clips, viewport"
)]
pub(super) fn record_draw_range_clipped(
    pass: &mut RenderPass<'_>,
    pipeline: &RenderPipeline,
    uniform_bg: &BindGroup,
    atlas_bg: Option<&BindGroup>,
    buffer: Option<&Buffer>,
    start: u32,
    end: u32,
    clips: &[super::super::draw_list_convert::ClipSegment],
    viewport_w: u32,
    viewport_h: u32,
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

    if clips.is_empty() {
        pass.draw(0..4, start..end);
        return;
    }

    let mut cursor = start;
    for seg in clips {
        // Skip clips outside our range.
        if seg.instance_offset < start {
            // Apply the scissor but don't draw — it affects later segments.
            if let Some(r) = seg.rect {
                pass.set_scissor_rect(r[0], r[1], r[2], r[3]);
            } else {
                pass.set_scissor_rect(0, 0, viewport_w, viewport_h);
            }
            continue;
        }
        if seg.instance_offset >= end {
            break;
        }
        if seg.instance_offset > cursor {
            pass.draw(0..4, cursor..seg.instance_offset);
        }
        if let Some(r) = seg.rect {
            pass.set_scissor_rect(r[0], r[1], r[2], r[3]);
        } else {
            pass.set_scissor_rect(0, 0, viewport_w, viewport_h);
        }
        cursor = seg.instance_offset;
    }
    if cursor < end {
        pass.draw(0..4, cursor..end);
    }
    pass.set_scissor_rect(0, 0, viewport_w, viewport_h);
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
        pre_cache_atlas(&mut sp_atlas, font_collection, device, queue);
        (atlas, sp_atlas)
    } else {
        let mut atlas = GlyphAtlas::new(device, GlyphFormat::Alpha);
        let sp_atlas = GlyphAtlas::new_lazy(device, GlyphFormat::SubpixelRgb);
        pre_cache_atlas(&mut atlas, font_collection, device, queue);
        (atlas, sp_atlas)
    };
    // Color atlas is lazy — no emoji at startup.
    let color_atlas = GlyphAtlas::new_lazy(device, GlyphFormat::Color);
    (atlas, subpixel_atlas, color_atlas)
}

/// Pre-cache printable ASCII glyphs (Regular + Bold) into the given atlas.
///
/// Iterates 0x20–0x7E for Regular, then again for Bold if the collection has
/// a real Bold face. Used by both `WindowRenderer::new()` and `clear_and_recache()`.
pub(super) fn pre_cache_atlas(
    atlas: &mut GlyphAtlas,
    fc: &mut FontCollection,
    device: &Device,
    queue: &Queue,
) {
    let size_q6 = size_key(fc.size_px());
    let hinted = fc.hinting_mode().hint_flag();
    for ch in ' '..='~' {
        let resolved = fc.resolve(ch, GlyphStyle::Regular);
        let key = RasterKey::from_resolved(resolved, size_q6, hinted, 0);
        if let Some(glyph) = fc.rasterize(key) {
            atlas.insert(key, glyph, device, queue);
        }
    }
    if fc.has_bold() {
        for ch in ' '..='~' {
            let resolved = fc.resolve(ch, GlyphStyle::Bold);
            let key = RasterKey::from_resolved(resolved, size_q6, hinted, 0);
            if let Some(glyph) = fc.rasterize(key) {
                atlas.insert(key, glyph, device, queue);
            }
        }
    }
}
