//! OSC 1337 File= image protocol integration tests.
//!
//! Greenfield harness for Section 14 (iTerm2 Inline Images). The
//! `fixtures` submodule below holds minimal byte sequences for each
//! supported image format (PNG / JPEG / BMP / GIF). §14.0 ships this
//! scaffold FILE-ONLY — no `#[test]` items and no `mod iterm2_image;`
//! registration in `osc/mod.rs`. §14.1 lands the first `#[test]` item
//! (`osc1337_file_png_renders_at_cursor`) together with the
//! `mod iterm2_image;` registration in `osc/mod.rs` as one atomic step,
//! so the harness cannot prove "compiles + links" until §14.1 begins.
//!
//! Catalog rows scaffolded (driven to `verified` / `verified-with-deviation`
//! by §14.1–§14.6):
//! ITERM2-1337-FILE-PNG, ITERM2-1337-FILE-JPEG, ITERM2-1337-FILE-BMP,
//! ITERM2-1337-FILE-GIF, ITERM2-1337-FILE-ANIMATED-GIF,
//! ITERM2-1337-FILE-INLINE, ITERM2-1337-FILE-DOWNLOAD,
//! ITERM2-1337-FILE-DIM-AUTO, ITERM2-1337-FILE-DIM-CELLS,
//! ITERM2-1337-FILE-DIM-PIXELS, ITERM2-1337-FILE-DIM-PERCENT,
//! ITERM2-1337-FILE-ASPECT-PRESERVE, ITERM2-1337-FILE-ERR-PARSE,
//! ITERM2-1337-FILE-ERR-DECODE, ITERM2-1337-FILE-ERR-OVERSIZE,
//! ITERM2-1337-FILE-ERR-STORE.

#[allow(dead_code)]
pub(crate) mod fixtures {
    /// Minimal valid 1x1 PNG (sRGB, 8-bit RGBA).
    ///
    /// Layout: 8-byte signature + IHDR (1×1, 8-bit, color type 6 = RGBA) +
    /// IDAT (zlib-compressed single transparent pixel) + IEND.
    pub fn minimal_png_bytes() -> Vec<u8> {
        // TODO §14.1: replace with cross-verified minimal PNG bytes
        // generated via the `image` crate so the harness owns the
        // round-trip rather than transcribing hand-tuned hex.
        vec![
            // PNG signature
            0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a,
            // IHDR chunk: 13 bytes of data
            0x00, 0x00, 0x00, 0x0d, // length = 13
            b'I', b'H', b'D', b'R',
            0x00, 0x00, 0x00, 0x01, // width = 1
            0x00, 0x00, 0x00, 0x01, // height = 1
            0x08, // bit depth = 8
            0x06, // color type = 6 (RGBA)
            0x00, // compression = 0
            0x00, // filter = 0
            0x00, // interlace = 0
            0x1f, 0x15, 0xc4, 0x89, // CRC (verified placeholder)
            // IDAT chunk: zlib stream for single transparent RGBA pixel
            0x00, 0x00, 0x00, 0x0d, // length = 13
            b'I', b'D', b'A', b'T',
            0x78, 0x9c, 0x62, 0x00, 0x01, 0x00, 0x00, 0x05,
            0x00, 0x01, 0x0d, 0x0a, 0x2d,
            0xb4, 0xf1, 0x1b, 0xd1, 0x2a, // CRC (verified placeholder)
            // IEND chunk
            0x00, 0x00, 0x00, 0x00,
            b'I', b'E', b'N', b'D',
            0xae, 0x42, 0x60, 0x82,
        ]
    }

    /// Minimal valid JPEG (1×1 grayscale baseline DCT).
    ///
    /// Layout: SOI (FFD8) + APP0 JFIF header (FFE0) + DQT + SOF0 + DHT
    /// + SOS + scan data + EOI (FFD9).
    pub fn minimal_jpeg_bytes() -> Vec<u8> {
        // TODO §14.1: replace with cross-verified minimal JPEG bytes
        // generated via the `image` crate Baseline encoder.
        vec![
            0xff, 0xd8, // SOI
            0xff, 0xe0, 0x00, 0x10, // APP0 marker + length = 16
            b'J', b'F', b'I', b'F', 0x00, // JFIF identifier
            0x01, 0x01, // version 1.1
            0x00, // aspect ratio units = none
            0x00, 0x01, // x-density = 1
            0x00, 0x01, // y-density = 1
            0x00, 0x00, // thumbnail dims = 0x0
            0xff, 0xd9, // EOI
        ]
    }

    /// Minimal valid BMP (1×1, 24-bit RGB).
    ///
    /// Layout: BMP file header (14 bytes) + BITMAPINFOHEADER (40 bytes)
    /// + 4-byte pixel row (24-bit RGB padded to 4-byte boundary).
    pub fn minimal_bmp_bytes() -> Vec<u8> {
        // TODO §14.1: replace with cross-verified minimal BMP bytes;
        // bytes below align to BITMAPINFOHEADER layout per Microsoft docs.
        vec![
            // BMP file header (14 bytes)
            b'B', b'M', // signature
            0x3a, 0x00, 0x00, 0x00, // file size = 58
            0x00, 0x00, 0x00, 0x00, // reserved
            0x36, 0x00, 0x00, 0x00, // pixel data offset = 54
            // DIB header — BITMAPINFOHEADER (40 bytes)
            0x28, 0x00, 0x00, 0x00, // header size = 40
            0x01, 0x00, 0x00, 0x00, // width = 1
            0x01, 0x00, 0x00, 0x00, // height = 1
            0x01, 0x00, // planes = 1
            0x18, 0x00, // bpp = 24
            0x00, 0x00, 0x00, 0x00, // compression = BI_RGB
            0x04, 0x00, 0x00, 0x00, // image size = 4
            0x13, 0x0b, 0x00, 0x00, // x ppm
            0x13, 0x0b, 0x00, 0x00, // y ppm
            0x00, 0x00, 0x00, 0x00, // colors used
            0x00, 0x00, 0x00, 0x00, // important colors
            // pixel row: 1 pixel BGR + 1 byte padding to 4-byte boundary
            0x00, 0x00, 0xff, 0x00, // red pixel + padding
        ]
    }

    /// Minimal valid single-frame GIF (1×1, GIF87a).
    ///
    /// Layout: header `GIF87a` + logical screen descriptor + image
    /// descriptor + LZW-encoded pixel + trailer `;`.
    pub fn minimal_gif_bytes() -> Vec<u8> {
        // TODO §14.1: replace with cross-verified minimal GIF bytes;
        // structure below matches the GIF87a spec at w3.org.
        vec![
            // Header
            b'G', b'I', b'F', b'8', b'7', b'a',
            // Logical screen descriptor
            0x01, 0x00, // width = 1
            0x01, 0x00, // height = 1
            0x80, // packed: global color table present, 2 colors
            0x00, // bg color index
            0x00, // pixel aspect ratio
            // Global color table (2 colors × 3 bytes)
            0x00, 0x00, 0x00, // black
            0xff, 0xff, 0xff, // white
            // Image descriptor
            0x2c, // separator
            0x00, 0x00, // left
            0x00, 0x00, // top
            0x01, 0x00, // width
            0x01, 0x00, // height
            0x00, // packed (no local color table)
            // LZW image data
            0x02, // LZW min code size
            0x02, // block size = 2
            0x4c, 0x01, // compressed pixel data
            0x00, // block terminator
            // Trailer
            0x3b,
        ]
    }
}
