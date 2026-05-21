//! Compose request payload for the kitty `a=c` Compose action.
//!
//! Parallel to `frame_load.rs` — the input contract for
//! [`super::cache::ImageCache::compose_frame`]. Kept at the image-module
//! level (not inside `cache/`) because `mod cache;` is private; matches
//! the [`super::FrameLoadRequest`] re-export pattern.
//!
//! Key mapping (per kitty graphics.c:1820-1832 `handle_compose_command` +
//! worked example at docs/graphics-protocol.rst:984):
//!
//! | Field        | kitty key | Semantic                           |
//! |--------------|-----------|------------------------------------|
//! | `image_id`   | `i=`      | Image to operate on                |
//! | `src_frame`  | `r=`      | Source frame number (1-based)      |
//! | `dst_frame`  | `c=`      | Destination frame number (1-based) |
//! | `width`      | `w=`      | Rect width (0 = image width)       |
//! | `height`     | `h=`      | Rect height (0 = image height)     |
//! | `src_x`      | `X=`      | Source rect X offset               |
//! | `src_y`      | `Y=`      | Source rect Y offset               |
//! | `dst_x`      | `x=`      | Destination rect X offset          |
//! | `dst_y`      | `y=`      | Destination rect Y offset          |
//! | `mode`       | `C=`      | 0=AlphaBlend, 1=Overwrite          |
//!
//! Note: kitty's doc prose at `graphics-protocol.rst:977-979` claims
//! `x,y` are SOURCE offsets and `X,Y` are DEST offsets — that prose is
//! incorrect; the C source at `graphics.c:1830` AND the spec's own
//! worked example at line 984 confirm the mapping in the table above.

use super::{CompositionMode, ImageId};

/// Parameters for a kitty Compose operation.
///
/// Both frames are 1-based per kitty `frame_for_number` semantics. Rect
/// dims of 0 mean "use full image dims" per kitty graphics.c:1830-1831.
#[derive(Debug, Clone, Copy)]
pub struct ComposeRequest {
    /// Target image (`i=` key).
    pub image_id: ImageId,
    /// Source frame number, 1-based (`r=` key).
    pub src_frame: u32,
    /// Destination frame number, 1-based (`c=` key).
    pub dst_frame: u32,
    /// Rect width in pixels (`w=` key). `0` means use the full image width.
    pub width: u32,
    /// Rect height in pixels (`h=` key). `0` means use the full image height.
    pub height: u32,
    /// Source rect X offset in pixels (`X=` key — uppercase).
    pub src_x: u32,
    /// Source rect Y offset in pixels (`Y=` key — uppercase).
    pub src_y: u32,
    /// Destination rect X offset in pixels (`x=` key — lowercase).
    pub dst_x: u32,
    /// Destination rect Y offset in pixels (`y=` key — lowercase).
    pub dst_y: u32,
    /// Composition mode (`C=` key): 0 = `AlphaBlend` (default), 1 = `Overwrite`.
    pub mode: CompositionMode,
}
