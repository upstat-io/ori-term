//! Clip segment output for GPU scissor rect support.
//!
//! When [`convert_draw_list`](super::convert_draw_list) encounters `PushClip`/`PopClip`
//! commands, it emits [`ClipSegment`]s recording where in the instance stream
//! the scissor rect changes. The render pass consumes these segments to call
//! `set_scissor_rect` at the right points and split draw calls into sub-ranges.

use oriterm_ui::geometry::Rect;

use super::super::instance_writer::InstanceWriter;
use super::TextContext;

/// A clip state change at a specific point in the instance stream.
///
/// Each segment records the instance index where the scissor rect changes.
/// The render pass iterates segments in order, issuing `draw(start..end)`
/// sub-ranges between each change.
#[derive(Debug, Clone, Copy)]
pub struct ClipSegment {
    /// Instance index (not byte offset) where this clip takes effect.
    ///
    /// Corresponds to `InstanceWriter::len()` at the point the clip was emitted.
    pub instance_offset: u32,
    /// Scissor rect in physical pixels `[x, y, w, h]`, or `None` for full viewport.
    pub rect: Option<[u32; 4]>,
}

/// Clip segments for all 4 writers in a tier.
///
/// Each `convert_draw_list` call writes to up to 4 instance writers
/// simultaneously (rects, mono glyphs, subpixel glyphs, color glyphs).
/// A single clip change must be recorded in ALL active writers at their
/// respective instance offsets — the render pass splits each draw call
/// independently.
#[derive(Debug, Clone, Default)]
pub struct TierClips {
    /// Clip segments for the rect writer.
    pub rects: Vec<ClipSegment>,
    /// Clip segments for the mono glyph writer.
    pub mono: Vec<ClipSegment>,
    /// Clip segments for the subpixel glyph writer.
    pub subpixel: Vec<ClipSegment>,
    /// Clip segments for the color glyph writer.
    pub color: Vec<ClipSegment>,
}

impl TierClips {
    /// Clear all segment vectors, retaining allocated memory.
    pub fn clear(&mut self) {
        self.rects.clear();
        self.mono.clear();
        self.subpixel.clear();
        self.color.clear();
    }

    /// Shift all instance offsets by the given bases (one per writer).
    pub fn shift_offsets(&mut self, bases: [u32; 4]) {
        for seg in &mut self.rects {
            seg.instance_offset += bases[0];
        }
        for seg in &mut self.mono {
            seg.instance_offset += bases[1];
        }
        for seg in &mut self.subpixel {
            seg.instance_offset += bases[2];
        }
        for seg in &mut self.color {
            seg.instance_offset += bases[3];
        }
    }

    /// Append all segments from `other`, shifting instance offsets by the
    /// current writer lengths in the target tier.
    pub fn extend_from(&mut self, other: &Self, bases: [u32; 4]) {
        extend_shifted(&mut self.rects, &other.rects, bases[0]);
        extend_shifted(&mut self.mono, &other.mono, bases[1]);
        extend_shifted(&mut self.subpixel, &other.subpixel, bases[2]);
        extend_shifted(&mut self.color, &other.color, bases[3]);
    }
}

/// Clip context for tracking scissor rect state during draw list conversion.
///
/// Pass to [`convert_draw_list`](super::convert_draw_list) to enable
/// `PushClip`/`PopClip` processing. When `None`, clip commands are ignored.
pub struct ClipContext<'a> {
    /// Output clip segments for all 4 writers.
    pub clips: &'a mut TierClips,
    /// Reusable clip stack (caller provides storage, cleared at call start).
    pub stack: &'a mut Vec<Rect>,
    /// Viewport width in physical pixels (for clamping).
    pub viewport_w: u32,
    /// Viewport height in physical pixels (for clamping).
    pub viewport_h: u32,
}

/// Handle a `PushClip` command: intersect with current clip and emit segments.
pub(super) fn push_clip(
    rect: Rect,
    ui_writer: &InstanceWriter,
    text_ctx: Option<&mut TextContext<'_>>,
    ctx: &mut ClipContext<'_>,
    scale: f32,
) {
    // Scale to physical pixels.
    let phys = Rect::new(
        rect.x() * scale,
        rect.y() * scale,
        rect.width() * scale,
        rect.height() * scale,
    );

    // Intersect with current clip (top of stack).
    let clipped = if let Some(&top) = ctx.stack.last() {
        phys.intersection(top)
    } else {
        phys
    };

    ctx.stack.push(clipped);

    // Clamp to viewport and emit segments.
    let scissor = clamp_to_viewport(clipped, ctx.viewport_w, ctx.viewport_h);
    emit_clip_segment(ui_writer, text_ctx, ctx.clips, Some(scissor));
}

/// Handle a `PopClip` command: restore previous clip and emit segments.
pub(super) fn pop_clip(
    ui_writer: &InstanceWriter,
    text_ctx: Option<&mut TextContext<'_>>,
    ctx: &mut ClipContext<'_>,
) {
    if ctx.stack.pop().is_none() {
        log::warn!("PopClip without matching PushClip — ignoring");
        return;
    }

    // Restore to previous clip, or None (full viewport) if stack is empty.
    let scissor = ctx
        .stack
        .last()
        .map(|&r| clamp_to_viewport(r, ctx.viewport_w, ctx.viewport_h));
    emit_clip_segment(ui_writer, text_ctx, ctx.clips, scissor);
}

/// Emit a [`ClipSegment`] into all active writers at their current offsets.
fn emit_clip_segment(
    ui_writer: &InstanceWriter,
    text_ctx: Option<&mut TextContext<'_>>,
    clips: &mut TierClips,
    rect: Option<[u32; 4]>,
) {
    clips.rects.push(ClipSegment {
        instance_offset: ui_writer.len() as u32,
        rect,
    });

    if let Some(ctx) = text_ctx {
        clips.mono.push(ClipSegment {
            instance_offset: ctx.mono_writer.len() as u32,
            rect,
        });
        clips.subpixel.push(ClipSegment {
            instance_offset: ctx.subpixel_writer.len() as u32,
            rect,
        });
        clips.color.push(ClipSegment {
            instance_offset: ctx.color_writer.len() as u32,
            rect,
        });
    }
}

/// Clamp a physical-pixel rect to viewport bounds for `set_scissor_rect`.
fn clamp_to_viewport(rect: Rect, vw: u32, vh: u32) -> [u32; 4] {
    let vw_f = vw as f32;
    let vh_f = vh as f32;
    let x = rect.x().max(0.0).min(vw_f);
    let y = rect.y().max(0.0).min(vh_f);
    let r = (rect.x() + rect.width()).max(0.0).min(vw_f);
    let b = (rect.y() + rect.height()).max(0.0).min(vh_f);
    [
        x as u32,
        y as u32,
        (r - x).max(0.0) as u32,
        (b - y).max(0.0) as u32,
    ]
}

/// Append clip segments with instance offsets shifted by `base`.
fn extend_shifted(dst: &mut Vec<ClipSegment>, src: &[ClipSegment], base: u32) {
    dst.reserve(src.len());
    for seg in src {
        dst.push(ClipSegment {
            instance_offset: seg.instance_offset + base,
            rect: seg.rect,
        });
    }
}
