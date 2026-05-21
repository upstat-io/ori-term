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
use std::time::Duration;

pub use cache::ImageCache;
pub use decode::{
    GifFrames, ImageFormat, decode_gif_frames, decode_to_rgba, detect_format, rgb_to_rgba,
};

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
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ImageId(pub(crate) u32);

impl ImageId {
    /// Construct an `ImageId` from a raw u32 value.
    pub fn from_raw(val: u32) -> Self {
        Self(val)
    }

    /// Get the underlying u32 value.
    pub fn as_u32(self) -> u32 {
        self.0
    }
}

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
    /// GPU layer receives `&[u8]` via [`RenderableImageData::data`] —
    /// never clone the `Arc` across the core-to-GPU boundary.
    pub(crate) data: Arc<Vec<u8>>,
    /// Original format before decode.
    pub format: ImageFormat,
    /// How the image was transmitted.
    pub source: ImageSource,
    /// Monotonic counter for LRU eviction ordering.
    pub last_accessed: u64,
    /// Client-supplied image number (`I=` in kitty graphics protocol).
    ///
    /// Distinct from `id`: clients can transmit multiple images with the
    /// same `I=` and the newest one is resolved by `d=n/N` deletes.
    /// `None` for non-kitty sources or kitty commands without `I=`.
    pub image_number: Option<u32>,
}

/// How a placement's display dimensions are determined on resize.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlacementSizing {
    /// Display size = `cols * cell_width` × `rows * cell_height`.
    ///
    /// Scales proportionally when cell dimensions change (e.g. font
    /// size change). Used when the protocol explicitly specifies cell
    /// counts (Kitty `c=`/`r=`, iTerm2 cell-count mode).
    CellCount,

    /// Display size is fixed at these pixel dimensions.
    ///
    /// `cols`/`rows` are recomputed via `ImageCache::update_cell_coverage`
    /// when cell dimensions change so viewport intersection and region
    /// queries remain correct. Used for Sixel, Kitty auto-sized, and
    /// iTerm2 pixel/auto/percent modes.
    FixedPixels {
        /// Display width in pixels.
        width: u32,
        /// Display height in pixels.
        height: u32,
    },
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
    /// How display dimensions are determined on resize.
    pub sizing: PlacementSizing,
}

impl ImagePlacement {
    /// Whether this placement overlaps the viewport row range `[top, bottom]`.
    ///
    /// Zero-width or zero-height placements occupy no cells and therefore
    /// overlap no viewport — returns `false` without applying the
    /// `saturating_sub(1)` origin-row fallback (which would falsely claim
    /// the origin row is occupied).
    pub(crate) fn intersects_viewport(
        &self,
        viewport_top: StableRowIndex,
        viewport_bottom: StableRowIndex,
    ) -> bool {
        if self.cols == 0 || self.rows == 0 {
            return false;
        }
        let bottom = StableRowIndex(self.cell_row.0 + (self.rows - 1) as u64);
        self.cell_row <= viewport_bottom && bottom >= viewport_top
    }
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

/// Kitty frame composition mode (`X=` key in `a=f` commands).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompositionMode {
    /// Alpha-blend new frame data over the previous frame.
    AlphaBlend,
    /// New frame data overwrites the target region entirely.
    Overwrite,
}

/// Minimum frame duration for animated images (60fps cap).
///
/// Prevents abusive GIFs with 0ms or 10ms frame durations from burning
/// CPU/GPU. Matches `WezTerm`'s approach.
const MIN_FRAME_DURATION: Duration = Duration::from_millis(16);

/// Per-image animation state for multi-frame images (GIF, Kitty animated).
///
/// Stored in `ImageCache::animations` keyed by `ImageId`. Multiple
/// placements of the same image share one animation state (same frame
/// displayed everywhere).
#[derive(Debug, Clone)]
pub struct AnimationState {
    /// Index of the currently displayed frame.
    pub current_frame: usize,
    /// Duration for each frame (0-indexed, parallel to frame data).
    pub frame_durations: Vec<Duration>,
    /// Total number of frames.
    pub total_frames: usize,
    /// Number of loops (None = infinite).
    pub loop_count: Option<u32>,
    /// Number of complete loops performed so far.
    pub loops_completed: u32,
    /// Whether animation is paused (Kitty `a=s`).
    pub paused: bool,
    /// Whether the animation is in `s=2` wait-mode: when loops exhaust,
    /// the animation halts at the last frame and waits for new frames
    /// (via `a=f`) before resuming. `s=3` (run) clears this flag so a
    /// later `add_animation_frame` does NOT auto-resume — distinguishing
    /// "wait for new frames" () from "play once and stop."
    pub wait_mode: bool,
}

impl AnimationState {
    /// Create a new animation state.
    pub fn new(frame_durations: Vec<Duration>, loop_count: Option<u32>) -> Self {
        let total_frames = frame_durations.len();
        Self {
            current_frame: 0,
            frame_durations,
            total_frames,
            loop_count,
            loops_completed: 0,
            paused: false,
            wait_mode: false,
        }
    }

    /// Duration of the current frame (clamped to minimum).
    pub fn current_duration(&self) -> Duration {
        self.frame_durations
            .get(self.current_frame)
            .copied()
            .unwrap_or(MIN_FRAME_DURATION)
            .max(MIN_FRAME_DURATION)
    }

    /// Advance to the next frame. Returns `true` if the frame changed.
    ///
    /// Handles loop counting + stops at the final frame when loops are
    /// exhausted OR when `wait_mode` (Kitty `s=2` `ANIMATION_LOADING`) is set
    /// and we would wrap. Per kitty `graphics.c:1793-1796`, `ANIMATION_LOADING`
    /// halts at the last frame on wrap, waiting for a new `a=f` frame to
    /// extend the animation; `ANIMATION_RUNNING` (`s=3`) wraps normally.
    pub fn advance(&mut self) -> bool {
        if self.paused || self.total_frames <= 1 {
            return false;
        }

        let next = self.current_frame + 1;
        if next >= self.total_frames {
            // Wrap branch — loop counting + wait-mode halt.
            if self.wait_mode {
                // s=2 ANIMATION_LOADING: halt at the last frame; do not
                // wrap. A subsequent `a=f` will extend `total_frames` and
                // `add_animation_frame` clears wait_mode on resume per
                // existing logic.
                return false;
            }
            if let Some(max_loops) = self.loop_count {
                self.loops_completed += 1;
                if self.loops_completed >= max_loops {
                    return false; // All loops complete.
                }
            }
            self.current_frame = 0;
        } else {
            self.current_frame = next;
        }
        true
    }

    /// Whether the animation has finished all its loops.
    pub fn is_finished(&self) -> bool {
        if let Some(max_loops) = self.loop_count {
            self.loops_completed >= max_loops
        } else {
            false // Infinite loop.
        }
    }

    /// Advance to the next non-gapless frame in this tick.
    ///
    /// Per kitty `graphics.c:1798-1799` skip loop: after a single
    /// time-driven advance, consume consecutive zero-gap frames inside the
    /// same tick so they are never displayed. Gapless frames exist as base
    /// data for subsequent frames (per `graphics-protocol.rst:947-958`).
    ///
    /// Bounded by `total_frames - 1` to prevent runaway on all-gapless
    /// animations; `current_duration()` clamps to `MIN_FRAME_DURATION` as
    /// the deadline-scheduling safety net in that edge case.
    ///
    /// Returns `true` if `current_frame` changed.
    pub fn advance_consuming_gapless(&mut self) -> bool {
        if !self.advance() {
            return false;
        }
        let max_skips = self.total_frames.saturating_sub(1);
        let mut skips = 0;
        while skips < max_skips
            && self.is_current_frame_gapless()
            && !self.is_finished()
        {
            if !self.advance() {
                break;
            }
            skips += 1;
        }
        true
    }

    /// `true` when the current frame's raw gap is `Duration::ZERO`.
    fn is_current_frame_gapless(&self) -> bool {
        self.frame_durations
            .get(self.current_frame)
            .is_some_and(|d| *d == Duration::ZERO)
    }
}

/// Read-only snapshot of an animation's protocol-observable facts.
///
/// Stable public API that decouples consumers from the internal
/// `AnimationState` field layout. The frame-gap slice is borrowed (`&[Duration]`)
/// rather than owned to avoid per-call allocation; the snapshot is `Copy`
/// because references-to-slices are `Copy`.
#[derive(Debug, Clone, Copy)]
pub struct AnimationSnapshot<'a> {
    /// Index of the currently displayed frame (0-based).
    pub current_frame: usize,
    /// Total number of frames.
    pub total_frames: usize,
    /// Per-frame gaps (raw `Duration::ZERO` for gapless frames).
    pub frame_gaps: &'a [Duration],
    /// Number of loops (`None` = infinite per kitty `v=1` mapping).
    pub loop_count: Option<u32>,
    /// Number of complete loops performed so far.
    pub loops_completed: u32,
    /// Whether playback is paused (Kitty `s=1`).
    pub paused: bool,
    /// Whether the animation is in `s=2` wait-mode.
    pub wait_mode: bool,
}

impl<'a> From<&'a AnimationState> for AnimationSnapshot<'a> {
    fn from(state: &'a AnimationState) -> Self {
        Self {
            current_frame: state.current_frame,
            total_frames: state.total_frames,
            frame_gaps: &state.frame_durations,
            loop_count: state.loop_count,
            loops_completed: state.loops_completed,
            paused: state.paused,
            wait_mode: state.wait_mode,
        }
    }
}

#[cfg(test)]
mod tests;
