//! Marker types for type-safe coordinate spaces.
//!
//! These zero-sized types tag [`Point`](super::Point), [`Size`](super::Size),
//! and [`Rect`](super::Rect) with a coordinate space so the compiler prevents
//! accidental mixing of logical, physical, and screen-space values.

/// Logical (DPI-independent) coordinates.
///
/// The default coordinate space. All layout calculations happen in logical
/// pixels; conversion to physical pixels occurs at render time via a
/// [`Scale`](crate::scale::Scale) transform.
pub struct Logical;

/// Physical (device) coordinates.
///
/// Pixels after DPI scaling. Used for GPU buffer uploads and platform
/// window APIs that expect physical pixel values.
pub struct Physical;

/// Screen (compositor) coordinates.
///
/// Absolute position on the display, relative to the primary monitor's
/// top-left corner. Used for window positioning and multi-monitor layout.
pub struct Screen;
