//! Layout constants for window chrome (title bar + controls).
//!
//! Values follow Chrome's `OpaqueBrowserFrameViewLayout` adapted for a
//! terminal emulator. All dimensions are in logical pixels — the caller
//! multiplies by scale factor for physical coordinates.

/// Caption bar height when the window is in restored (non-maximized) state.
pub const CAPTION_HEIGHT: f32 = 36.0;

/// Condensed caption bar height when the window is maximized.
///
/// Maximized windows omit the resize border, so the caption is slightly
/// shorter to match Windows conventions.
pub const CAPTION_HEIGHT_MAXIMIZED: f32 = 32.0;

/// Width of each window control button (minimize, maximize, close).
///
/// Chrome uses 46px per button on Windows — we match that.
pub const CONTROL_BUTTON_WIDTH: f32 = 46.0;

/// Resize border width in logical pixels.
///
/// Used by hit testing to determine the draggable edge zone.
pub const RESIZE_BORDER_WIDTH: f32 = 6.0;

/// Symbol stroke width for window control glyphs (logical pixels).
pub const SYMBOL_STROKE_WIDTH: f32 = 1.0;

/// Size of the glyph symbols in control buttons (logical pixels).
///
/// The minimize dash, maximize square, and close X are drawn within a
/// square of this side length, centered in the button.
pub const SYMBOL_SIZE: f32 = 10.0;
