//! Frame-load request types for `ImageCache::put_frame`.
//!
//! Captures the three kitty `a=f` arms in a single unified request struct
//! that crosses the dispatch → cache boundary:
//! - default-append: canvas = `Y=` solid color
//! - `c=N`: canvas = frame N's pixels, new frame appended
//! - `r=N`: edit existing frame N in place

use std::sync::Arc;
use std::time::Duration;

use super::{CompositionMode, ImageId};

/// Where the canvas bytes for a `put_frame` operation come from.
///
/// Resolved by `ImageCache::put_frame` against the target image: the canvas
/// is a full-image-sized buffer that the frame payload is then blit'd onto
/// before being stored or pushed into the animation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CanvasSource {
    /// Solid-color RGBA canvas (`Y=` key on the default append arm).
    ///
    /// Byte layout matches kitty `g->bgcolor`: R is the most-significant
    /// byte, so `0xff_00_00_ff` decodes to `[0xff, 0x00, 0x00, 0xff]`
    /// (opaque red, R=MSB). Default `Y=0` → fully transparent
    /// `[0x00, 0x00, 0x00, 0x00]`.
    SolidColor(u32),
    /// Canvas is frame `N`'s pixel data (`c=N` arm, 1-based). Per kitty
    /// graphics.c:1334-1344, frame 1 IS the root image (`images[id].data`)
    /// when the image has not yet been promoted to animated.
    Frame(u32),
    /// Canvas IS the target frame being edited (`r=N` arm). The target
    /// frame's existing pixel data is the starting canvas; the payload
    /// is composed on top per `composition_mode`.
    EditTarget,
}

/// Target slot for a `put_frame` operation — disambiguates append vs edit
/// and carries the per-arm gap payload.
///
/// The per-arm payload lives inside the variants (not as flat siblings) so
/// the compiler enforces mutually-exclusive `gap` vs `gap_update` semantics
/// at construction time. Per kitty graphics.c:1597 (append) and
/// graphics.c:1651 (edit), the gap rules differ between arms:
/// - **Append**: `z>0` → `from_millis(z)`; `z<0` → `ZERO` (gapless);
///   `z==0` → `DEFAULT_GAP` (40ms).
/// - **Edit**: `z>0` → `Some(from_millis(z))`; `z<0` → `Some(ZERO)`;
///   `z==0` → `None` (do NOT update existing frame's gap).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameTarget {
    /// Append a new frame at index `total + 1`. `gap` is the z-normalized
    /// duration for the new frame.
    Append {
        /// Per-arm gap for the appended frame (already z-normalized per the
        /// append-arm rules above).
        gap: Duration,
    },
    /// Edit the existing 1-based frame `frame_num`. Pre-validated by
    /// dispatch to be in `1..=total_frames`. `gap_update` is the
    /// per-arm gap-update intent: `None` leaves the existing frame's gap
    /// untouched; `Some(gap)` overwrites it.
    Edit {
        /// 1-based target frame index (validated to be within range).
        frame_num: u32,
        /// Per-arm gap-update intent (already z-normalized).
        gap_update: Option<Duration>,
    },
}

/// Destination rectangle for a sub-rect blit, in canvas coordinates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BlitRect {
    /// Destination X offset in canvas pixels (`x=` key on `a=f`).
    pub dest_x: u32,
    /// Destination Y offset in canvas pixels (`y=` key on `a=f`).
    pub dest_y: u32,
    /// Blit width in pixels (load-bearing: decoded payload dim, NOT `s=`).
    pub width: u32,
    /// Blit height in pixels (load-bearing: decoded payload dim, NOT `v=`).
    pub height: u32,
}

/// Unified request to load a frame into the image cache (`put_frame`).
///
/// One struct captures the three kitty `a=f` arms — default-append
/// (canvas = `Y=` solid color), `c=N` (canvas = frame N's pixels, new
/// frame appended), `r=N` (edit existing frame N in place) — keeping the
/// function signature narrow.
#[derive(Debug, Clone)]
pub struct FrameLoadRequest {
    /// Target image id (`i=` key).
    pub image_id: ImageId,
    /// Append-or-edit + per-arm gap payload.
    pub target: FrameTarget,
    /// Where the canvas bytes come from.
    pub canvas: CanvasSource,
    /// Destination rectangle (where in the canvas to apply the payload).
    pub blit: BlitRect,
    /// Decoded RGBA payload (length == `blit.width * blit.height * 4`).
    pub frame_data: Arc<Vec<u8>>,
    /// How the payload combines with the canvas (X=0=blend, X=1=overwrite).
    pub composition_mode: CompositionMode,
}
