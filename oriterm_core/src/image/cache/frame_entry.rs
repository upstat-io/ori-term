#![allow(
    dead_code,
    reason = "Phase 3 scaffolding (xray-scene lag cure) — FrameEntry / FrameId / chain-limit \
              constants land here so Phase 3 matrix tests can reference the final \
              shape; Phase 4 Item 2 wires them through put_frame + frame_for_number"
)]

//! Per-frame storage variants for animated images.
//!
//! `ImageCache::animation_frames` stores `Vec<FrameEntry>` instead of the
//! pre-fix `Vec<Arc<Vec<u8>>>` so kitty `_Ga=f, c=N` delta-encoded frames
//! can record `{base_frame_ref, sub_rect, payload}` (≤ ~1 KB typical) rather
//! than materializing a full RGBA canvas (~2.64 MB at xray's 1000×660) on
//! every append.
//!
//! Crate-private — the read API (`frame_for_number`) materializes on
//! demand, so the variant split never escapes `oriterm_core::image::cache`.

use std::sync::Arc;

use super::super::{BlitRect, CompositionMode};

/// Stable internal identifier for a stored frame entry.
///
/// Monotonic, never reused, allocated via `ImageCache::alloc_frame_id`.
/// Used as the `base` reference inside `FrameEntry::Delta` so deletions
/// that shift vector indices in `animation_frames[id]` do NOT invalidate
/// delta-chain back-references.
///
/// `u64` width mirrors `ImageData::pixel_generation` so neither field
/// can wrap before the other on absurd-but-finite continuous-append
/// workloads — a `u32` would wrap at roughly 828 days of sustained
/// 60 FPS `_Ga=f` traffic, violating the
/// `debug_assert!(base.0 < new.0)` cycle-detection invariant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct FrameId(pub(crate) u64);

/// Storage variant for a single frame in an animation.
///
/// `Materialized` carries a full RGBA canvas (root frame, edit-target
/// frame, keyframe-coalesced frame). `Delta` carries the sub-rect payload
/// plus a stable `FrameId` reference to the canvas it composes onto.
///
/// Chain-depth + cumulative-area guards (kitty `graphics.c:1544-1554`)
/// keep `frame_for_number` compose-on-demand O(1) on average:
/// - `depth` bounded by `MAX_DELTA_CHAIN_DEPTH = 3` (max 4 hops from
///   a Materialized base).
/// - `cumulative_drawn_area` bounded by
///   `MAX_CUMULATIVE_DRAWN_AREA_RATIO * image_w * image_h` (max 2×
///   the image area summed across the chain).
#[derive(Debug, Clone)]
pub(crate) enum FrameEntry {
    /// Full RGBA canvas stored inline. Used for the root frame, edit
    /// targets, and keyframe-coalesced frames produced by the chain-
    /// depth or cumulative-area guards in `put_frame`.
    Materialized {
        /// Stable internal identifier.
        id: FrameId,
        /// Full RGBA pixel data (length == `image_w * image_h * 4`).
        data: Arc<Vec<u8>>,
    },
    /// Delta-encoded sub-rect referencing an earlier frame's canvas.
    Delta {
        /// Stable internal identifier for this entry.
        id: FrameId,
        /// Stable FrameId of the entry this delta composes onto.
        ///
        /// Invariant: `base.0 < id.0` — verified via
        /// `debug_assert!` at append time. Cycles impossible by
        /// construction because `put_frame` mints a new FrameId only
        /// after the request's `c=N` resolves an earlier one.
        base: FrameId,
        /// Destination sub-rect in canvas coordinates.
        sub_rect: BlitRect,
        /// Sub-rect RGBA payload only — length ==
        /// `sub_rect.width * sub_rect.height * 4`. Compare with
        /// the full RGBA canvas size on `Materialized` to see the
        /// allocator-pressure delta on xray-style transmits.
        payload: Arc<Vec<u8>>,
        /// Composition kernel (`AlphaBlend` for `X=0`, `Overwrite`
        /// for `X=1`).
        compose_mode: CompositionMode,
        /// Chain depth from the nearest `Materialized` ancestor.
        /// `Delta` directly off a `Materialized` base has `depth == 0`.
        depth: u8,
        /// Cumulative sub-rect area summed across the chain from the
        /// nearest `Materialized` ancestor (inclusive of this entry).
        /// Force-materialize trigger when this exceeds
        /// `MAX_CUMULATIVE_DRAWN_AREA_RATIO * image_w * image_h`.
        cumulative_drawn_area: u32,
    },
}

impl FrameEntry {
    /// Stable internal identifier of this entry.
    pub(crate) fn id(&self) -> FrameId {
        match self {
            Self::Materialized { id, .. } | Self::Delta { id, .. } => *id,
        }
    }

    /// Chain depth from the nearest `Materialized` ancestor (0 for
    /// `Materialized` itself).
    pub(crate) fn depth(&self) -> u8 {
        match self {
            Self::Materialized { .. } => 0,
            Self::Delta { depth, .. } => *depth,
        }
    }

    /// Cumulative sub-rect area along the chain (0 for `Materialized`).
    pub(crate) fn cumulative_drawn_area(&self) -> u32 {
        match self {
            Self::Materialized { .. } => 0,
            Self::Delta {
                cumulative_drawn_area,
                ..
            } => *cumulative_drawn_area,
        }
    }

    /// Bytes the entry contributes to `ImageCache::memory_used`.
    pub(crate) fn memory_bytes(&self) -> usize {
        match self {
            Self::Materialized { data, .. } => data.len(),
            Self::Delta { payload, .. } => payload.len(),
        }
    }
}

/// Maximum chain depth before `put_frame` force-materializes.
///
/// Matches kitty `graphics.c:1544` `num >= 5` (4-hop chain length =
/// 1 Materialized base + 4 Deltas; depth 0..=3 stores as Delta, depth
/// 4 force-materializes). Reading the entry at depth 3 traverses three
/// recursive `frame_for_number_by_id` hops to reach the Materialized
/// canvas — bounded compose-on-demand cost.
pub(crate) const MAX_DELTA_CHAIN_DEPTH: u8 = 3;

/// Maximum `cumulative_drawn_area / (image_w * image_h)` ratio before
/// `put_frame` force-materializes. Matches kitty `graphics.c:1546`
/// `drawn_area >= image_w * image_h * 2` — when the chain has touched
/// the canvas twice over, the delta-storage win is gone and compose
/// cost dominates.
pub(crate) const MAX_CUMULATIVE_DRAWN_AREA_RATIO: u32 = 2;
