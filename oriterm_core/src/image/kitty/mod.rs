//! Kitty Graphics Protocol implementation.
//!
//! Parses APC sequences with `G` prefix into structured commands,
//! then executes them against the `ImageCache`. Supports transmit,
//! place, delete, query, and animation operations.

mod parse;

pub use parse::{
    KittyAction, KittyCommand, KittyError, KittyTransmission, parse_kitty_command,
    parse_kitty_command_into,
};

/// In-progress chunked image transmission.
///
/// Kitty protocol allows splitting large images across multiple APC
/// sequences using `m=1` (more) / `m=0` (final). Per kitty's protocol
/// documentation, the FIRST chunk carries all control keys (format,
/// dimensions, placement geometry, `q=`, `U=`, `C=`, `z=`, etc.) and
/// subsequent chunks carry only `m=` and the payload bytes. `start_cmd`
/// holds the first chunk's fully-parsed command; each non-first chunk
/// appends its `cmd.payload` into `start_cmd.payload` so at finalize
/// time the merged command carries first-chunk semantics with the
/// coalesced payload.
#[derive(Debug)]
pub struct LoadingImage {
    /// Resolved image id — `cmd.image_id` if provided, else auto-assigned
    /// from `ImageCache::next_image_id`. Kept as a separate field because
    /// `start_cmd.image_id` may be `None` (auto-id case) even after
    /// resolution.
    pub image_id: u32,
    /// Full first-chunk command; `payload` is the running accumulator.
    pub start_cmd: KittyCommand,
    /// Per-upload failure latch (§13.5 EINVAL-flood fix): once a malformed
    /// base64 chunk fires the first EINVAL reply, subsequent chunks of
    /// the same upload are dropped silently so the reply transcript
    /// contains exactly one EINVAL per failed upload instead of one
    /// per failed chunk.
    pub failed_upload: bool,
}

#[cfg(test)]
mod tests;
