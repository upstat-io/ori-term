//! The per-cell renderable type.
//!
//! [`RenderableCell`] is one fully-resolved grid cell ready for the GPU emit
//! path: palette lookups, bold-as-bright, dim, and inverse are already
//! applied, and per-channel alpha (SGR mode-6) is carried as concrete bytes.

use crate::cell::CellFlags;
use crate::color::Rgb;
use crate::index::Column;

/// A single cell ready for rendering.
///
/// Colors are fully resolved (palette lookups, bold-as-bright, dim,
/// inverse all applied). The renderer never needs the raw `Color` enum.
///
/// Per-channel alpha (`fg_alpha` / `bg_alpha` / `underline_alpha`) carries the
/// concrete SGR mode-6 alpha bytes (0–255). Cells without explicit alpha
/// default to [`OPAQUE_ALPHA`](crate::cell::OPAQUE_ALPHA) on every channel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderableCell {
    /// Line index in the visible viewport (0 = top).
    pub line: usize,
    /// Column index (0-based).
    pub column: Column,
    /// Display character.
    pub ch: char,
    /// Resolved foreground color.
    pub fg: Rgb,
    /// Resolved background color.
    pub bg: Rgb,
    /// Cell attribute flags.
    pub flags: CellFlags,
    /// Resolved underline color (if custom underline color is set).
    pub underline_color: Option<Rgb>,
    /// Concrete foreground alpha (0–255). [`OPAQUE_ALPHA`](crate::cell::OPAQUE_ALPHA) when no SGR mode-6
    /// alpha was set on the source cell's foreground channel.
    pub fg_alpha: u8,
    /// Concrete background alpha (0–255). [`OPAQUE_ALPHA`](crate::cell::OPAQUE_ALPHA) when no SGR mode-6
    /// alpha was set on the source cell's background channel.
    pub bg_alpha: u8,
    /// Concrete underline alpha (0–255). [`OPAQUE_ALPHA`](crate::cell::OPAQUE_ALPHA) when no SGR mode-6
    /// alpha was set on the source cell's underline channel.
    pub underline_alpha: u8,
    /// Whether this cell has an OSC 8 hyperlink attached.
    pub has_hyperlink: bool,
    /// OSC 8 hyperlink URI, if present.
    ///
    /// Populated during `renderable_content_into()` so IO-thread snapshots
    /// carry the URI without needing Grid access later.
    pub hyperlink_uri: Option<String>,
    /// Zero-width combining characters appended to this cell.
    pub zerowidth: Vec<char>,
}

// `RenderableCell` populates the per-frame `RenderableContent::cells` Vec
// (`lines * cols` entries, reused each snapshot). Pin its size so an
// accidental field addition that grows the hot per-cell array is caught at
// compile time.
const _: () = assert!(size_of::<RenderableCell>() == 88);
