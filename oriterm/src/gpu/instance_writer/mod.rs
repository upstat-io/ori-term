//! GPU instance buffer writer for batched quad rendering.
//!
//! Each visible element (background rect, glyph, cursor, underline) becomes
//! one 96-byte instance record in a GPU buffer. [`InstanceWriter`] accumulates
//! these records on the CPU side, then the Render phase uploads the backing
//! `Vec<u8>` to a `wgpu::Buffer` in one copy.
//!
//! The 96-byte layout is designed for a single `VertexBufferLayout` with
//! known offsets — no padding, no alignment surprises. All multi-byte fields
//! are little-endian (matching GPU expectations on all target platforms).

use oriterm_core::Rgb;

use super::srgb_to_linear;

/// Screen-space rectangle for GPU instance positioning.
#[derive(Debug, Clone, Copy)]
pub struct ScreenRect {
    /// Pixel X of top-left corner.
    pub x: f32,
    /// Pixel Y of top-left corner.
    pub y: f32,
    /// Width in pixels.
    pub w: f32,
    /// Height in pixels.
    pub h: f32,
}

impl ScreenRect {
    /// Scale all coordinates by a factor (e.g. logical → physical pixels).
    pub fn scaled(self, s: f32) -> Self {
        Self {
            x: self.x * s,
            y: self.y * s,
            w: self.w * s,
            h: self.h * s,
        }
    }
}

/// Bytes per instance record in the GPU buffer.
pub const INSTANCE_SIZE: usize = 96;

// Field offsets within the 96-byte record.
const OFF_POS_X: usize = 0; //  f32  — pixel X
const OFF_POS_Y: usize = 4; //  f32  — pixel Y
const OFF_SIZE_W: usize = 8; //  f32  — width in pixels
const OFF_SIZE_H: usize = 12; //  f32  — height in pixels
const OFF_UV_X: usize = 16; //  f32  — atlas U left
const OFF_UV_Y: usize = 20; //  f32  — atlas V top
const OFF_UV_W: usize = 24; //  f32  — atlas U width
const OFF_UV_H: usize = 28; //  f32  — atlas V height
const OFF_FG_R: usize = 32; //  f32  — foreground R [0..1]
const OFF_FG_G: usize = 36; //  f32  — foreground G [0..1]
const OFF_FG_B: usize = 40; //  f32  — foreground B [0..1]
const OFF_FG_A: usize = 44; //  f32  — foreground A [0..1]
const OFF_BG_R: usize = 48; //  f32  — background R [0..1]
const OFF_BG_G: usize = 52; //  f32  — background G [0..1]
const OFF_BG_B: usize = 56; //  f32  — background B [0..1]
const OFF_BG_A: usize = 60; //  f32  — background A [0..1]
const OFF_KIND: usize = 64; //  u32  — instance kind (rect/glyph/cursor)
const OFF_ATLAS_PAGE: usize = 68; //  u32  — atlas texture array layer index
const OFF_CLIP_X: usize = 80; //  f32  — clip rect origin X
const OFF_CLIP_Y: usize = 84; //  f32  — clip rect origin Y
const OFF_CLIP_W: usize = 88; //  f32  — clip rect width
const OFF_CLIP_H: usize = 92; //  f32  — clip rect height

// Compile-time check: the last field's end must equal the declared record size.
const _: () = assert!(OFF_CLIP_H + 4 == INSTANCE_SIZE);

/// Clip rect that never discards any fragment (used by terminal-tier instances).
///
/// Uses large finite values instead of infinity to avoid NaN from
/// `clip.xy + clip.zw` in the shader (`-INF + INF = NaN`). DX12/HLSL
/// treats NaN comparisons as `true`, which would discard every fragment.
pub const CLIP_UNCLIPPED: [f32; 4] = [-100_000.0, -100_000.0, 200_000.0, 200_000.0];

/// Instance kind tag written into the record at offset 64.
///
/// The shader uses this to select between solid-fill (rect/cursor) and
/// texture-sampled (glyph) rendering.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum InstanceKind {
    /// Solid-color background rectangle.
    Rect = 0,
    /// Texture-sampled glyph from the atlas.
    Glyph = 1,
    /// Cursor rectangle (may blend differently).
    Cursor = 2,
}

/// CPU-side accumulator for GPU instance records.
///
/// Maintains a `Vec<u8>` that grows as instances are pushed. The buffer
/// never shrinks — [`clear`](InstanceWriter::clear) resets the length but
/// retains allocated capacity for the next frame.
pub struct InstanceWriter {
    /// Backing byte buffer. Length is always a multiple of [`INSTANCE_SIZE`].
    buf: Vec<u8>,
}

impl InstanceWriter {
    /// Create an empty writer with no pre-allocated capacity.
    pub fn new() -> Self {
        Self { buf: Vec::new() }
    }

    /// Create a writer pre-allocated for `capacity` instances.
    #[cfg(test)]
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            buf: Vec::with_capacity(capacity * INSTANCE_SIZE),
        }
    }

    /// Number of instance records currently stored.
    pub fn len(&self) -> usize {
        self.buf.len() / INSTANCE_SIZE
    }

    /// Whether the writer contains zero instances.
    pub fn is_empty(&self) -> bool {
        self.buf.is_empty()
    }

    /// Raw byte slice for GPU upload.
    pub fn as_bytes(&self) -> &[u8] {
        &self.buf
    }

    /// Total bytes currently stored.
    pub fn byte_len(&self) -> usize {
        self.buf.len()
    }

    /// Reset to zero instances, retaining allocated memory.
    pub fn clear(&mut self) {
        self.buf.clear();
    }

    /// Shrink the backing buffer if capacity vastly exceeds usage.
    ///
    /// Called after rendering to bound memory waste to 2x actual usage.
    /// Only fires when capacity > 4× length AND > 4096 bytes, so small
    /// buffers and normal high-water-mark reuse are untouched.
    pub fn maybe_shrink(&mut self) {
        let cap = self.buf.capacity();
        let len = self.buf.len();
        if cap > 4 * len && cap > 4096 {
            self.buf.shrink_to(len * 2);
        }
    }

    /// Append all instances from `other` into this writer.
    pub fn extend_from(&mut self, other: &Self) {
        self.buf.extend_from_slice(&other.buf);
    }

    /// Append a byte range from a saved buffer.
    ///
    /// `start` and `end` are byte offsets (not instance indices). The range
    /// must be aligned to [`INSTANCE_SIZE`] boundaries.
    pub fn extend_from_byte_range(&mut self, src: &[u8], start: usize, end: usize) {
        debug_assert!(start.is_multiple_of(INSTANCE_SIZE) && end.is_multiple_of(INSTANCE_SIZE));
        if start < end && end <= src.len() {
            self.buf.extend_from_slice(&src[start..end]);
        }
    }

    /// Swap the backing buffer with an external `Vec`.
    ///
    /// Used by the incremental prepare path to save the previous frame's
    /// instances for clean-row reuse without cloning.
    pub fn swap_buf(&mut self, other: &mut Vec<u8>) {
        std::mem::swap(&mut self.buf, other);
    }

    /// Push a solid-color rectangle instance.
    ///
    /// UV coordinates are zeroed (no texture sampling for rects).
    pub fn push_rect(&mut self, rect: ScreenRect, bg: Rgb, alpha: f32) {
        self.push_instance(
            rect.x,
            rect.y,
            rect.w,
            rect.h,
            [0.0, 0.0, 0.0, 0.0],
            [0.0, 0.0, 0.0, 0.0],
            rgb_to_floats(bg, alpha),
            InstanceKind::Rect,
            0,
        );
    }

    /// Push a texture-sampled glyph instance.
    ///
    /// `uv` is `[u_left, v_top, u_width, v_height]` in atlas texture
    /// coordinates (0..1). `atlas_page` selects the texture array layer.
    /// `clip` is `[x, y, w, h]` in physical pixels for per-fragment clipping.
    #[expect(
        clippy::too_many_arguments,
        reason = "glyph instance: screen rect, UV coords, color, atlas page, clip"
    )]
    pub fn push_glyph(
        &mut self,
        rect: ScreenRect,
        uv: [f32; 4],
        fg: Rgb,
        alpha: f32,
        atlas_page: u32,
        clip: [f32; 4],
    ) {
        self.push_instance(
            rect.x,
            rect.y,
            rect.w,
            rect.h,
            uv,
            rgb_to_floats(fg, alpha),
            [0.0, 0.0, 0.0, 0.0],
            InstanceKind::Glyph,
            atlas_page,
        );
        // Overwrite the default unclipped values with the provided clip.
        let start = self.buf.len() - INSTANCE_SIZE;
        let rec = &mut self.buf[start..];
        write_f32(rec, OFF_CLIP_X, clip[0]);
        write_f32(rec, OFF_CLIP_Y, clip[1]);
        write_f32(rec, OFF_CLIP_W, clip[2]);
        write_f32(rec, OFF_CLIP_H, clip[3]);
    }

    /// Push a texture-sampled glyph instance with background color.
    ///
    /// Like [`push_glyph`](Self::push_glyph) but also writes the cell's
    /// background color into the `bg_color` instance field. The subpixel
    /// fragment shader reads `bg_color` for per-channel `mix()` blending.
    /// Mono and color pipelines ignore the `bg_color` field.
    /// `clip` is `[x, y, w, h]` in physical pixels for per-fragment clipping.
    #[expect(
        clippy::too_many_arguments,
        reason = "glyph instance: screen rect, UV coords, fg/bg colors, atlas page, clip"
    )]
    pub fn push_glyph_with_bg(
        &mut self,
        rect: ScreenRect,
        uv: [f32; 4],
        fg: Rgb,
        bg: Rgb,
        alpha: f32,
        atlas_page: u32,
        clip: [f32; 4],
    ) {
        self.push_instance(
            rect.x,
            rect.y,
            rect.w,
            rect.h,
            uv,
            rgb_to_floats(fg, alpha),
            rgb_to_floats(bg, 1.0),
            InstanceKind::Glyph,
            atlas_page,
        );
        // Overwrite the default unclipped values with the provided clip.
        let start = self.buf.len() - INSTANCE_SIZE;
        let rec = &mut self.buf[start..];
        write_f32(rec, OFF_CLIP_X, clip[0]);
        write_f32(rec, OFF_CLIP_Y, clip[1]);
        write_f32(rec, OFF_CLIP_W, clip[2]);
        write_f32(rec, OFF_CLIP_H, clip[3]);
    }

    /// Push a cursor rectangle instance.
    ///
    /// Color is written to the `bg_color` field (same as rects) so cursors
    /// render correctly with the background pipeline (solid-fill shader).
    pub fn push_cursor(&mut self, rect: ScreenRect, color: Rgb, alpha: f32) {
        self.push_instance(
            rect.x,
            rect.y,
            rect.w,
            rect.h,
            [0.0, 0.0, 0.0, 0.0],
            [0.0, 0.0, 0.0, 0.0],
            rgb_to_floats(color, alpha),
            InstanceKind::Cursor,
            0,
        );
    }

    /// Push a raw pre-encoded instance record.
    ///
    /// # Panics
    ///
    /// Panics if `bytes.len() != INSTANCE_SIZE`.
    #[allow(dead_code, reason = "instance writer methods for later sections")]
    pub fn push_raw(&mut self, bytes: &[u8]) {
        assert_eq!(
            bytes.len(),
            INSTANCE_SIZE,
            "raw instance must be exactly {INSTANCE_SIZE} bytes",
        );
        self.buf.extend_from_slice(bytes);
    }

    /// Encode and append one 96-byte instance record.
    #[expect(
        clippy::too_many_arguments,
        reason = "private 96-byte GPU record encoder: position, UV, colors, kind, page"
    )]
    fn push_instance(
        &mut self,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        uv: [f32; 4],
        fg: [f32; 4],
        bg: [f32; 4],
        kind: InstanceKind,
        atlas_page: u32,
    ) {
        let start = self.buf.len();
        self.buf.resize(start + INSTANCE_SIZE, 0);
        let rec = &mut self.buf[start..];

        write_f32(rec, OFF_POS_X, x);
        write_f32(rec, OFF_POS_Y, y);
        write_f32(rec, OFF_SIZE_W, w);
        write_f32(rec, OFF_SIZE_H, h);

        write_f32(rec, OFF_UV_X, uv[0]);
        write_f32(rec, OFF_UV_Y, uv[1]);
        write_f32(rec, OFF_UV_W, uv[2]);
        write_f32(rec, OFF_UV_H, uv[3]);

        write_f32(rec, OFF_FG_R, fg[0]);
        write_f32(rec, OFF_FG_G, fg[1]);
        write_f32(rec, OFF_FG_B, fg[2]);
        write_f32(rec, OFF_FG_A, fg[3]);

        write_f32(rec, OFF_BG_R, bg[0]);
        write_f32(rec, OFF_BG_G, bg[1]);
        write_f32(rec, OFF_BG_B, bg[2]);
        write_f32(rec, OFF_BG_A, bg[3]);

        write_u32(rec, OFF_KIND, kind as u32);
        write_u32(rec, OFF_ATLAS_PAGE, atlas_page);
        // Corner radius / border width zeroed by resize.
        // Clip: unclipped by default (terminal-tier instances).
        write_f32(rec, OFF_CLIP_X, CLIP_UNCLIPPED[0]);
        write_f32(rec, OFF_CLIP_Y, CLIP_UNCLIPPED[1]);
        write_f32(rec, OFF_CLIP_W, CLIP_UNCLIPPED[2]);
        write_f32(rec, OFF_CLIP_H, CLIP_UNCLIPPED[3]);
    }
}

impl Default for InstanceWriter {
    fn default() -> Self {
        Self::new()
    }
}

/// Convert `Rgb` + alpha to linear-light `[f32; 4]` for the GPU.
///
/// Each sRGB byte is decoded via [`srgb_to_linear`] so the values are
/// truly linear when sent to the `*Srgb` render target.
fn rgb_to_floats(c: Rgb, a: f32) -> [f32; 4] {
    [
        srgb_to_linear(c.r),
        srgb_to_linear(c.g),
        srgb_to_linear(c.b),
        a,
    ]
}

/// Write a little-endian `f32` at the given byte offset.
fn write_f32(buf: &mut [u8], offset: usize, val: f32) {
    buf[offset..offset + 4].copy_from_slice(&val.to_le_bytes());
}

/// Write a little-endian `u32` at the given byte offset.
fn write_u32(buf: &mut [u8], offset: usize, val: u32) {
    buf[offset..offset + 4].copy_from_slice(&val.to_le_bytes());
}

#[cfg(test)]
mod tests;
