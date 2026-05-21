//! Per-arm key extraction for kitty `a=f` dispatch.
//!
//! Materializes the union-shaped `KittyCommand` field semantics for `a=f`
//! into a typed struct so the dispatch site in `frame.rs` reads from named
//! fields instead of mis-interpreting key-positional fields.
//!
//! `KittyCommand` field reinterpretation for `a=f` (kitty graphics.h:15-21
//! union shape): `cell_x_offset` IS the composition mode (0=blend,
//! 1=overwrite); `cell_y_offset` IS the `Y=` RGBA background color;
//! `display_cols` IS `c=` (canvas frame source); `display_rows` IS `r=`
//! (target frame).

use std::time::Duration;

use crate::image::CompositionMode;
use crate::image::kitty::KittyCommand;

/// Default gap for an unspecified `z=` on the append arm (40ms, matching
/// `DEFAULT_GAP` at kitty graphics.c:1597).
const DEFAULT_FRAME_GAP: Duration = Duration::from_millis(40);

/// Materialized per-arm dispatch keys for `a=f`.
#[derive(Debug, Clone, Copy)]
pub(super) struct KittyFrameKeys {
    /// `c=N` — 1-based canvas frame source. `None` when key is absent or
    /// `c=0` (treated as unspecified per spec).
    pub(super) canvas_frame: Option<u32>,
    /// `r=N` — 1-based target frame. `None` when key is absent or `r=0`.
    pub(super) target_frame: Option<u32>,
    /// `X=` — composition mode (0=AlphaBlend, 1=Overwrite).
    pub(super) compose_mode: CompositionMode,
    /// `Y=` — RGBA solid background (default 0x00000000 = transparent).
    pub(super) background_rgba: u32,
    /// Append-arm gap (z-normalized): z>0 → from_millis; z<0 → ZERO; z==0
    /// → DEFAULT_FRAME_GAP. Per kitty graphics.c:1597.
    pub(super) append_gap: Duration,
    /// Edit-arm gap-update intent: None when z==0 (do NOT touch existing
    /// frame's gap); Some(ZERO) when z<0; Some(from_millis) when z>0. Per
    /// kitty graphics.c:1651.
    pub(super) edit_gap_update: Option<Duration>,
    /// Blit width — decoded payload dimension (NOT `s=`). For PNG
    /// (`f=100`), `s=` and decoded width can disagree because
    /// `decode_to_rgba` reads dims from the PNG header.
    pub(super) blit_w: u32,
    /// Blit height — decoded payload dimension (NOT `v=`).
    pub(super) blit_h: u32,
    /// Destination X offset in the canvas (`x=` key).
    pub(super) blit_x: u32,
    /// Destination Y offset in the canvas (`y=` key).
    pub(super) blit_y: u32,
}

/// Extract per-arm dispatch keys from a merged `a=f` command.
pub(super) fn extract_a_f_keys(
    cmd: &KittyCommand,
    decoded_w: u32,
    decoded_h: u32,
) -> KittyFrameKeys {
    let canvas_frame = cmd.display_cols.filter(|&v| v > 0);
    let target_frame = cmd.display_rows.filter(|&v| v > 0);
    let compose_mode = if cmd.cell_x_offset == 1 {
        CompositionMode::Overwrite
    } else {
        CompositionMode::AlphaBlend
    };

    let append_gap = match cmd.z_index.cmp(&0) {
        std::cmp::Ordering::Greater => Duration::from_millis(cmd.z_index as u64),
        std::cmp::Ordering::Less => Duration::ZERO,
        std::cmp::Ordering::Equal => DEFAULT_FRAME_GAP,
    };
    let edit_gap_update = match cmd.z_index.cmp(&0) {
        std::cmp::Ordering::Greater => Some(Duration::from_millis(cmd.z_index as u64)),
        std::cmp::Ordering::Less => Some(Duration::ZERO),
        std::cmp::Ordering::Equal => None,
    };

    KittyFrameKeys {
        canvas_frame,
        target_frame,
        compose_mode,
        background_rgba: cmd.cell_y_offset,
        append_gap,
        edit_gap_update,
        blit_w: decoded_w,
        blit_h: decoded_h,
        blit_x: cmd.source_x,
        blit_y: cmd.source_y,
    }
}

/// Convert a kitty `a=a` z-index to an `ensure_animation_state_for_root_gap`
/// gap-update payload per kitty graphics.c:1729-1735 + 1348-1350:
/// - `z == 0` → `None` (leave existing gap unchanged)
/// - `z > 0` → `Some(Duration::from_millis(z))`
/// - `z < 0` → `Some(Duration::ZERO)` (gapless; kitty clamps to zero)
pub(super) fn a_a_z_to_gap_update(z: i32) -> Option<Duration> {
    match z.cmp(&0) {
        std::cmp::Ordering::Greater => Some(Duration::from_millis(z as u64)),
        std::cmp::Ordering::Less => Some(Duration::ZERO),
        std::cmp::Ordering::Equal => None,
    }
}
