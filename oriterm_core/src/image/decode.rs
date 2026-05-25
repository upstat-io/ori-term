//! Image format detection and RGBA decoding.

/// Known image formats for terminal image protocols.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageFormat {
    /// Raw RGBA pixels (32-bit, 4 bytes per pixel).
    Rgba,
    /// Raw RGB pixels (24-bit, 3 bytes per pixel).
    Rgb,
    /// PNG compressed image.
    Png,
    /// JPEG compressed image.
    Jpeg,
    /// GIF image (possibly animated).
    Gif,
    /// BMP image.
    Bmp,
    /// WebP image.
    WebP,
}

/// Detect image format from magic bytes at the start of the data.
///
/// Returns `None` if the format is not recognized.
pub fn detect_format(data: &[u8]) -> Option<ImageFormat> {
    if data.len() < 4 {
        return None;
    }

    // PNG: 89 50 4E 47
    if data.starts_with(&[0x89, 0x50, 0x4E, 0x47]) {
        return Some(ImageFormat::Png);
    }

    // JPEG: FF D8 FF
    if data.starts_with(&[0xFF, 0xD8, 0xFF]) {
        return Some(ImageFormat::Jpeg);
    }

    // GIF: GIF87a or GIF89a
    if data.starts_with(b"GIF8") {
        return Some(ImageFormat::Gif);
    }

    // BMP: BM
    if data.starts_with(b"BM") {
        return Some(ImageFormat::Bmp);
    }

    // WebP: RIFF....WEBP
    if data.len() >= 12 && data.starts_with(b"RIFF") && &data[8..12] == b"WEBP" {
        return Some(ImageFormat::WebP);
    }

    None
}

/// Decode raw RGB data to RGBA by adding alpha=255 to each pixel.
///
/// Returns `None` if the data length is not a multiple of 3.
/// Used by Kitty Graphics Protocol (format `f=24`).
pub fn rgb_to_rgba(data: &[u8]) -> Option<Vec<u8>> {
    if !data.len().is_multiple_of(3) {
        return None;
    }
    let pixel_count = data.len() / 3;
    let mut rgba = Vec::with_capacity(pixel_count * 4);
    for chunk in data.chunks_exact(3) {
        rgba.push(chunk[0]);
        rgba.push(chunk[1]);
        rgba.push(chunk[2]);
        rgba.push(255);
    }
    Some(rgba)
}

/// Decode image data from a compressed format to RGBA pixels.
///
/// Returns `(rgba_data, width, height)` on success.
///
/// `max_total_bytes` bounds the decoder's allocation so a decompression
/// bomb (small compressed payload, huge decoded buffer — e.g. a tiny
/// solid PNG declaring 50000×50000) is rejected at decode time rather
/// than allocating gigabytes before the cache's store-time cap rejects
/// it. Callers pass `ImageCache::max_single_image_bytes()`.
///
/// Requires the `image-protocol` cargo feature (which enables the `image` crate).
/// Without the feature, this always returns `Err(ImageFormat not supported)`.
#[cfg(feature = "image-protocol")]
pub fn decode_to_rgba(
    data: &[u8],
    max_total_bytes: usize,
) -> Result<(Vec<u8>, u32, u32), super::ImageError> {
    use image::{ImageReader, Limits};
    use std::io::Cursor;

    let mut reader = ImageReader::new(Cursor::new(data))
        .with_guessed_format()
        .map_err(|e| super::ImageError::DecodeFailed(e.to_string()))?;
    let mut limits = Limits::default();
    limits.max_alloc = Some(max_total_bytes as u64);
    reader.limits(limits);

    let img = reader
        .decode()
        .map_err(|e| super::ImageError::DecodeFailed(e.to_string()))?;

    let rgba = img.to_rgba8();
    let width = rgba.width();
    let height = rgba.height();
    Ok((rgba.into_raw(), width, height))
}

/// Stub when `image-protocol` feature is disabled.
#[cfg(not(feature = "image-protocol"))]
pub fn decode_to_rgba(
    data: &[u8],
    max_total_bytes: usize,
) -> Result<(Vec<u8>, u32, u32), super::ImageError> {
    let _ = (data, max_total_bytes);
    Err(super::ImageError::DecodeFailed(
        "image-protocol feature not enabled".to_string(),
    ))
}

/// Fallback delay (ms) used only when a GIF frame's `Delay` has a zero
/// denominator — an upstream contract violation the `image` crate never
/// produces (its GIF decoder always builds `Delay` with denominator 1).
/// Zero-*numerator* (0-centisecond) frames decode faithfully to `0ms`;
/// the gapless animation policy is owned by the animation layer
/// (`advance_animations` all-gapless guard), not by this decoder.
const DEFAULT_GIF_FRAME_DELAY_MS: u32 = 100;

/// Hard ceiling on decoded GIF frame count.
///
/// Bounds CPU/iteration cost for a pathological many-frame payload whose
/// individual frames each fit the per-image byte budget (a frame-count
/// decompression bomb). Far above any real terminal-image animation.
pub const MAX_GIF_FRAMES: usize = 4096;

/// Decoded GIF animation frames.
pub struct GifFrames {
    /// Width of the animation canvas.
    pub width: u32,
    /// Height of the animation canvas.
    pub height: u32,
    /// RGBA pixel data for each frame.
    pub frames: Vec<Vec<u8>>,
    /// Duration of each frame.
    pub durations: Vec<std::time::Duration>,
    /// Number of loops (None = infinite, Some(0) = infinite per GIF spec).
    pub loop_count: Option<u32>,
}

/// Decode a GIF into individual animation frames.
///
/// Returns `None` if the data is not a GIF, has only one frame
/// (single-frame GIFs should use `decode_to_rgba` instead), contains a
/// frame that fails to decode, or whose running decoded-RGBA total
/// exceeds `max_total_bytes`.
///
/// A per-frame decode error fails the whole decode rather than silently
/// truncating the animation: a corrupt frame mid-stream returns `None`,
/// letting the caller fall back to `decode_to_rgba` (the first valid
/// frame) instead of storing a partial animation.
///
/// `max_total_bytes` bounds the cumulative decoded-RGBA size while frames
/// are materialized one at a time, so a decompression bomb (small
/// compressed payload, huge decoded total) cannot allocate without limit
/// before the cache's store-time cap rejects it. Callers pass
/// `ImageCache::max_single_image_bytes()` so the early bound mirrors the
/// authoritative store-time cap.
#[cfg(feature = "image-protocol")]
pub fn decode_gif_frames(data: &[u8], max_total_bytes: usize) -> Option<GifFrames> {
    use image::codecs::gif::GifDecoder;
    use image::metadata::LoopCount;
    use image::{AnimationDecoder, ImageDecoder, Limits};
    use std::io::Cursor;
    use std::time::Duration;

    let mut decoder = GifDecoder::new(Cursor::new(data)).ok()?;
    // Bound decoder allocation against the per-image cap BEFORE any frame
    // buffer is materialized, so a decompression bomb cannot allocate past
    // the budget during `into_frames()`. The cumulative `running_bytes`
    // guard below is a second bound against the sum of returned frames.
    let mut limits = Limits::default();
    limits.max_alloc = Some(max_total_bytes as u64);
    decoder.set_limits(limits).ok()?;

    // Read the GIF89a / NETSCAPE2.0 loop block from the decoder.
    // `LoopCount::Infinite` → `None`; `Finite(n)` → `Some(n)` (`n >= 1`,
    // since `Finite` wraps `NonZeroU32` and a 0-loop GIF maps to `Infinite`).
    let loop_count = match decoder.loop_count() {
        LoopCount::Infinite => None,
        LoopCount::Finite(n) => Some(n.get()),
    };

    let mut frames: Vec<image::Frame> = Vec::new();
    let mut running_bytes: usize = 0;
    for frame in decoder.into_frames() {
        // Frame-count bomb guard: a many-tiny-frame payload can stay under
        // the byte budget yet exhaust CPU. Cap the iteration count.
        if frames.len() >= MAX_GIF_FRAMES {
            return None;
        }
        let frame = frame.ok()?;
        running_bytes = running_bytes.saturating_add(frame.buffer().as_raw().len());
        if running_bytes > max_total_bytes {
            return None;
        }
        frames.push(frame);
    }

    if frames.len() <= 1 {
        return None;
    }

    let width = frames[0].buffer().width();
    let height = frames[0].buffer().height();

    let mut rgba_frames = Vec::with_capacity(frames.len());
    let mut durations = Vec::with_capacity(frames.len());

    for frame in frames {
        // Read the delay before `into_buffer()` consumes the frame. The
        // `image` GIF decoder always builds `Delay` with denominator 1; a
        // zero denominator would be an upstream contract violation.
        let (numer, denom) = frame.delay().numer_denom_ms();
        debug_assert!(denom > 0, "image crate Delay denominator must be non-zero");
        let ms = numer
            .checked_div(denom)
            .unwrap_or(DEFAULT_GIF_FRAME_DELAY_MS);
        durations.push(Duration::from_millis(u64::from(ms)));

        // The GIF decoder composites every frame onto the logical-screen
        // canvas (`image` crate contract), so all frame buffers share
        // frame-0 dimensions. Assert the invariant, then consume the frame
        // buffer without cloning.
        let buf = frame.into_buffer();
        debug_assert_eq!(
            (buf.width(), buf.height()),
            (width, height),
            "image crate must composite GIF frames to canvas size"
        );
        rgba_frames.push(buf.into_raw());
    }

    Some(GifFrames {
        width,
        height,
        frames: rgba_frames,
        durations,
        loop_count,
    })
}

/// Stub when `image-protocol` feature is disabled.
#[cfg(not(feature = "image-protocol"))]
pub fn decode_gif_frames(_data: &[u8], _max_total_bytes: usize) -> Option<GifFrames> {
    None
}
