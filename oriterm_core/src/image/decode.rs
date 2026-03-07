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
/// Requires the `image-protocol` cargo feature (which enables the `image` crate).
/// Without the feature, this always returns `Err(ImageFormat not supported)`.
#[cfg(feature = "image-protocol")]
pub fn decode_to_rgba(data: &[u8]) -> Result<(Vec<u8>, u32, u32), super::ImageError> {
    use image::ImageReader;
    use std::io::Cursor;

    let reader = ImageReader::new(Cursor::new(data))
        .with_guessed_format()
        .map_err(|e| super::ImageError::DecodeFailed(e.to_string()))?;

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
pub fn decode_to_rgba(data: &[u8]) -> Result<(Vec<u8>, u32, u32), super::ImageError> {
    let _ = data;
    Err(super::ImageError::DecodeFailed(
        "image-protocol feature not enabled".to_string(),
    ))
}

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
/// Returns `None` if the data is not a GIF or has only one frame
/// (single-frame GIFs should use `decode_to_rgba` instead).
#[cfg(feature = "image-protocol")]
pub fn decode_gif_frames(data: &[u8]) -> Option<GifFrames> {
    use image::AnimationDecoder;
    use image::codecs::gif::GifDecoder;
    use std::io::Cursor;
    use std::time::Duration;

    let decoder = GifDecoder::new(Cursor::new(data)).ok()?;
    let frames: Vec<image::Frame> = decoder.into_frames().filter_map(Result::ok).collect();

    if frames.len() <= 1 {
        return None;
    }

    let width = frames[0].buffer().width();
    let height = frames[0].buffer().height();

    let mut rgba_frames = Vec::with_capacity(frames.len());
    let mut durations = Vec::with_capacity(frames.len());

    for frame in &frames {
        let buf = frame.buffer();
        // Resize frame to canvas size if needed (some GIFs have variable frame sizes).
        if buf.width() == width && buf.height() == height {
            rgba_frames.push(buf.as_raw().clone());
        } else {
            let resized = image::imageops::resize(buf, width, height, image::imageops::Nearest);
            rgba_frames.push(resized.into_raw());
        }

        let (numer, denom) = frame.delay().numer_denom_ms();
        let ms = if denom > 0 { numer / denom } else { 100 };
        durations.push(Duration::from_millis(u64::from(ms)));
    }

    Some(GifFrames {
        width,
        height,
        frames: rgba_frames,
        durations,
        loop_count: None, // GIF89a loop count not easily accessible via image crate.
    })
}

/// Stub when `image-protocol` feature is disabled.
#[cfg(not(feature = "image-protocol"))]
pub fn decode_gif_frames(_data: &[u8]) -> Option<GifFrames> {
    None
}
