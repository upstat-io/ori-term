//! Kitty Graphics Protocol implementation.
//!
//! Parses APC sequences with `G` prefix into structured commands,
//! then executes them against the `ImageCache`. Supports transmit,
//! place, delete, query, and animation operations.

mod parse;

pub use parse::{KittyAction, KittyCommand, KittyError, KittyTransmission, parse_kitty_command};

/// In-progress chunked image transmission.
///
/// Kitty protocol allows splitting large images across multiple APC
/// sequences using `m=1` (more) / `m=0` (final). This struct
/// accumulates the decoded payload and metadata across chunks.
#[derive(Debug)]
pub struct LoadingImage {
    /// Image ID assigned to this transmission.
    pub image_id: u32,
    /// Image number (alternative to ID).
    pub image_number: Option<u32>,
    /// Accumulated base64-decoded payload.
    pub payload: Vec<u8>,
    /// Expected pixel format (`f=` key).
    pub format: u32,
    /// Pixel width from first chunk (`s=` key).
    pub width: u32,
    /// Pixel height from first chunk (`v=` key).
    pub height: u32,
    /// Compression mode (`o=` key).
    pub compression: Option<u8>,
    /// Transmission method from first chunk.
    pub transmission: KittyTransmission,
}

#[cfg(test)]
mod tests;
