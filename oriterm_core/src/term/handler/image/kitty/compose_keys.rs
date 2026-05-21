//! Per-arm key extraction for kitty `a=c` Compose.
//!
//! Mirrors `frame_keys.rs` — materializes the union-shaped
//! [`KittyCommand`] fields into a typed struct so the dispatcher reads
//! from named fields. Compose's union semantics (per kitty
//! `graphics.c:1820-1832`):
//!
//! - `display_rows`   (`r=`) → source frame number
//! - `display_cols`   (`c=`) → destination frame number
//! - `width_px`       (`w=`) → rect width (0 = image width)
//! - `height_px`      (`h=`) → rect height (0 = image height)
//! - `cell_x_offset`  (`X=`) → source rect X offset
//! - `cell_y_offset`  (`Y=`) → source rect Y offset
//! - `source_x`       (`x=`) → destination rect X offset
//! - `source_y`       (`y=`) → destination rect Y offset
//! - `no_cursor_move` (`C=`) → composition mode
//!   (true = `Overwrite`, false/absent = `AlphaBlend`)
//!
//! Note: the protocol-doc prose at `graphics-protocol.rst:977-979`
//! claims `x,y` are SOURCE offsets and `X,Y` are DEST offsets — that
//! prose is the doc bug. The kitty C source at `graphics.c:1830` AND
//! the spec's worked example at line 984 confirm the table above.

use crate::image::CompositionMode;
use crate::image::kitty::KittyCommand;

/// Materialized per-arm dispatch keys for `a=c` Compose.
#[derive(Debug, Clone, Copy)]
pub(super) struct KittyComposeKeys {
    /// Source frame number, 1-based (0 = absent, ENOENT at dispatch).
    pub(super) src_frame: u32,
    /// Destination frame number, 1-based.
    pub(super) dst_frame: u32,
    /// Rect width in pixels (0 = use image width per graphics.c:1830).
    pub(super) width: u32,
    /// Rect height in pixels (0 = use image height per graphics.c:1831).
    pub(super) height: u32,
    /// Source rect X offset.
    pub(super) src_x: u32,
    /// Source rect Y offset.
    pub(super) src_y: u32,
    /// Destination rect X offset.
    pub(super) dst_x: u32,
    /// Destination rect Y offset.
    pub(super) dst_y: u32,
    /// Composition mode (0=AlphaBlend, 1=Overwrite per graphics.c:1862).
    pub(super) mode: CompositionMode,
}

/// Extract per-arm dispatch keys from a merged `a=c` command.
pub(super) fn extract_a_c_keys(cmd: &KittyCommand) -> KittyComposeKeys {
    KittyComposeKeys {
        src_frame: cmd.display_rows.unwrap_or(0),
        dst_frame: cmd.display_cols.unwrap_or(0),
        width: cmd.width_px.unwrap_or(0),
        height: cmd.height_px.unwrap_or(0),
        src_x: cmd.cell_x_offset,
        src_y: cmd.cell_y_offset,
        dst_x: cmd.source_x,
        dst_y: cmd.source_y,
        mode: if cmd.no_cursor_move {
            CompositionMode::Overwrite
        } else {
            CompositionMode::AlphaBlend
        },
    }
}
