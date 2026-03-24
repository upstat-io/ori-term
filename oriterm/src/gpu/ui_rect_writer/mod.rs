//! Dedicated GPU instance buffer writer for UI rect rendering.
//!
//! Each UI rect (styled rectangle, line segment, shadow) becomes one 144-byte
//! instance record. The layout carries per-side border widths, per-side border
//! colors, and four independent corner radii — enough for the UI-rect SDF
//! shader to render any CSS-style border configuration in a single draw call.
//!
//! This writer is separate from [`InstanceWriter`](super::instance_writer::InstanceWriter)
//! which handles terminal backgrounds, glyphs, and cursors at 96 bytes per instance.

use super::instance_writer::ScreenRect;

/// Bytes per UI rect instance record.
pub const UI_RECT_INSTANCE_SIZE: usize = 144;

// Field offsets within the 144-byte record.
const OFF_POS: usize = 0; //  vec2<f32> — pixel position
const OFF_SIZE: usize = 8; //  vec2<f32> — pixel size
const OFF_CLIP: usize = 16; //  vec4<f32> — clip rect [x, y, w, h]
const OFF_FILL: usize = 32; //  vec4<f32> — fill color RGBA linear
const OFF_BORDER_WIDTHS: usize = 48; //  vec4<f32> — [top, right, bottom, left]
const OFF_CORNER_RADII: usize = 64; //  vec4<f32> — [tl, tr, br, bl]
const OFF_BORDER_TOP: usize = 80; //  vec4<f32> — top border RGBA linear
const OFF_BORDER_RIGHT: usize = 96; //  vec4<f32> — right border RGBA linear
const OFF_BORDER_BOTTOM: usize = 112; //  vec4<f32> — bottom border RGBA linear
const OFF_BORDER_LEFT: usize = 128; //  vec4<f32> — left border RGBA linear

// Compile-time check: last field end == declared size.
const _: () = assert!(OFF_BORDER_LEFT + 16 == UI_RECT_INSTANCE_SIZE);

/// CPU-side accumulator for UI rect GPU instance records.
///
/// Maintains a `Vec<u8>` that grows as instances are pushed. [`clear`](Self::clear)
/// resets the length but retains allocated capacity.
pub struct UiRectWriter {
    buf: Vec<u8>,
}

impl UiRectWriter {
    /// Create an empty writer.
    pub fn new() -> Self {
        Self { buf: Vec::new() }
    }

    /// Number of instance records currently stored.
    pub fn len(&self) -> usize {
        self.buf.len() / UI_RECT_INSTANCE_SIZE
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
    #[cfg(test)]
    pub fn byte_len(&self) -> usize {
        self.buf.len()
    }

    /// Reset to zero instances, retaining allocated memory.
    pub fn clear(&mut self) {
        self.buf.clear();
    }

    /// Shrink the backing buffer if capacity vastly exceeds usage.
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

    /// Push a UI rect instance with full per-side border data.
    ///
    /// `border_widths` is `[top, right, bottom, left]` in physical pixels.
    /// `corner_radii` is `[tl, tr, br, bl]` in physical pixels.
    /// `border_colors` is `[top, right, bottom, left]` each as `[r, g, b, a]` linear.
    /// `clip` is `[x, y, w, h]` in physical pixels.
    #[expect(
        clippy::too_many_arguments,
        reason = "UI rect instance: rect, fill, border widths, corner radii, border colors, clip"
    )]
    pub fn push_ui_rect(
        &mut self,
        rect: ScreenRect,
        fill: [f32; 4],
        border_widths: [f32; 4],
        corner_radii: [f32; 4],
        border_colors: [[f32; 4]; 4],
        clip: [f32; 4],
    ) {
        let start = self.buf.len();
        self.buf.resize(start + UI_RECT_INSTANCE_SIZE, 0);
        let rec = &mut self.buf[start..];

        write_f32x2(rec, OFF_POS, rect.x, rect.y);
        write_f32x2(rec, OFF_SIZE, rect.w, rect.h);
        write_f32x4(rec, OFF_CLIP, clip);
        write_f32x4(rec, OFF_FILL, fill);
        write_f32x4(rec, OFF_BORDER_WIDTHS, border_widths);
        write_f32x4(rec, OFF_CORNER_RADII, corner_radii);
        write_f32x4(rec, OFF_BORDER_TOP, border_colors[0]);
        write_f32x4(rec, OFF_BORDER_RIGHT, border_colors[1]);
        write_f32x4(rec, OFF_BORDER_BOTTOM, border_colors[2]);
        write_f32x4(rec, OFF_BORDER_LEFT, border_colors[3]);
    }
}

impl Default for UiRectWriter {
    fn default() -> Self {
        Self::new()
    }
}

/// Write two little-endian `f32`s at the given byte offset.
fn write_f32x2(buf: &mut [u8], offset: usize, a: f32, b: f32) {
    buf[offset..offset + 4].copy_from_slice(&a.to_le_bytes());
    buf[offset + 4..offset + 8].copy_from_slice(&b.to_le_bytes());
}

/// Write four little-endian `f32`s at the given byte offset.
fn write_f32x4(buf: &mut [u8], offset: usize, v: [f32; 4]) {
    buf[offset..offset + 4].copy_from_slice(&v[0].to_le_bytes());
    buf[offset + 4..offset + 8].copy_from_slice(&v[1].to_le_bytes());
    buf[offset + 8..offset + 12].copy_from_slice(&v[2].to_le_bytes());
    buf[offset + 12..offset + 16].copy_from_slice(&v[3].to_le_bytes());
}

/// Read a little-endian `f32` from the given byte offset.
#[cfg(test)]
fn read_f32(buf: &[u8], offset: usize) -> f32 {
    f32::from_le_bytes(buf[offset..offset + 4].try_into().unwrap())
}

#[cfg(test)]
mod tests;
