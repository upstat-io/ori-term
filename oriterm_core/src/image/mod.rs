//! Image storage, placement, and cache for inline image protocols.
//!
//! Supports Kitty Graphics Protocol, Sixel, and iTerm2 image protocol.
//! Images are stored as decoded RGBA pixel data with reference-counted
//! sharing across placements. Memory-managed with configurable limits
//! and LRU eviction.

mod cache;
mod decode;
pub mod iterm2;
pub mod kitty;
pub mod sixel;

use std::sync::Arc;

pub use cache::ImageCache;
pub use decode::{ImageFormat, decode_to_rgba, detect_format, rgb_to_rgba};

use crate::grid::StableRowIndex;

/// Kitty virtual placeholder character (U+10EEEE).
///
/// Programs using Kitty's unicode placeholder mode (`U=1`) write this
/// character into grid cells to reserve space for images. Selection
/// text extraction skips these characters.
pub const KITTY_PLACEHOLDER: char = '\u{10EEEE}';

/// Unique image identifier within a terminal instance.
///
/// IDs start at `2_147_483_647` (mid-range u32) for auto-assigned images
/// to avoid collisions with client-assigned IDs that typically start at 1.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ImageId(pub u32);

/// Source of image data (how it was transmitted).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImageSource {
    /// Data sent directly in the protocol payload.
    Direct,
    /// Data loaded from a file path.
    File(std::path::PathBuf),
    /// Data loaded from shared memory (platform-specific).
    SharedMemory,
}

/// Decoded image pixel data.
#[derive(Debug, Clone)]
pub struct ImageData {
    /// Unique image identifier.
    pub id: ImageId,
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// Decoded RGBA pixel data (shared across placements).
    ///
    /// GPU layer receives `&[u8]` via `data.as_slice()` — never clone
    /// the `Arc` across the core-to-GPU boundary.
    pub data: Arc<Vec<u8>>,
    /// Original format before decode.
    pub format: ImageFormat,
    /// How the image was transmitted.
    pub source: ImageSource,
    /// Monotonic counter for LRU eviction ordering.
    pub last_accessed: u64,
}

/// A placed instance of an image on the terminal grid.
#[derive(Debug, Clone)]
pub struct ImagePlacement {
    /// Reference to image data.
    pub image_id: ImageId,
    /// Kitty placement ID (for updates/deletes).
    pub placement_id: Option<u32>,
    /// Pixel offset within image (source rect origin).
    pub source_x: u32,
    /// Pixel offset within image (source rect origin).
    pub source_y: u32,
    /// Source rect size in pixels.
    pub source_w: u32,
    /// Source rect size in pixels.
    pub source_h: u32,
    /// Grid column (top-left cell).
    pub cell_col: usize,
    /// Grid row as stable row index (survives scrollback eviction).
    pub cell_row: StableRowIndex,
    /// Number of columns the image spans.
    pub cols: usize,
    /// Number of rows the image spans.
    pub rows: usize,
    /// Layer ordering: negative = below text, positive = above text.
    pub z_index: i32,
    /// Sub-cell pixel offset (Kitty `X=` param).
    pub cell_x_offset: u16,
    /// Sub-cell pixel offset (Kitty `Y=` param).
    pub cell_y_offset: u16,
}

/// Errors from image operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImageError {
    /// Single image exceeds `max_single_image_bytes`.
    OversizedImage,
    /// Image format not recognized or not supported.
    InvalidFormat,
    /// Image decoding failed (corrupt data, truncated, etc.).
    DecodeFailed(String),
    /// Total image memory would exceed cache limit even after eviction.
    MemoryLimitExceeded,
}

impl std::fmt::Display for ImageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::OversizedImage => write!(f, "image exceeds maximum size limit"),
            Self::InvalidFormat => write!(f, "unrecognized image format"),
            Self::DecodeFailed(msg) => write!(f, "image decode failed: {msg}"),
            Self::MemoryLimitExceeded => write!(f, "image memory limit exceeded"),
        }
    }
}

impl std::error::Error for ImageError {}

#[cfg(test)]
mod tests;
