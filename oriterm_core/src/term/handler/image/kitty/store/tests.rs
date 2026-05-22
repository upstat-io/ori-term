//! Tests for `kitty_store_from_file` — bounded file reading, temp-file cleanup, and TOCTOU/special-file rejection.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use crate::effect::VoidEffectSink;
use crate::term::Term;
use crate::term::handler::image::kitty::{KittyStoreParams, KittyTransmission};
use crate::theme::Theme;

/// Fixture builder: creates a temporary file with specified content.
struct TempFileFixture {
    path: PathBuf,
    _keep_alive: bool,
}

impl TempFileFixture {
    /// Create a temporary file with `content` bytes in a unique directory.
    fn new(test_name: &str, content: &[u8]) -> std::io::Result<Self> {
        let dir = std::env::temp_dir();
        let test_dir_name = format!("ori_kitty_test_{}_{}", test_name, std::process::id());
        let test_dir = dir.join(&test_dir_name);

        // Create the test directory
        fs::create_dir_all(&test_dir)?;

        let path = test_dir.join("testfile");

        let mut file = fs::File::create(&path)?;
        file.write_all(content)?;
        file.sync_all()?;
        drop(file);

        Ok(Self {
            path,
            _keep_alive: true,
        })
    }

    fn path(&self) -> &Path {
        &self.path
    }

    /// Returns the path as a UTF-8 string for the fixture payload.
    fn path_str(&self) -> String {
        self.path.to_string_lossy().to_string()
    }

    /// Clean up the test directory and file
    fn cleanup(&self) -> std::io::Result<()> {
        if let Some(parent) = self.path.parent() {
            let _ = fs::remove_file(&self.path);
            let _ = fs::remove_dir(parent);
        }
        Ok(())
    }
}

impl Drop for TempFileFixture {
    fn drop(&mut self) {
        let _ = self.cleanup();
    }
}

/// Kitty graphics-protocol.rst: oversize file → `EBIG`; ENOMEM is reserved
/// for allocator failure. Catalog row `KG-RESPONSE-EBIG`.
#[test]
fn kitty_store_from_file_oversized_t_eq_f_returns_ebig_with_bounded_read() {
    // t=f (File): source never removed on rejection.
    let mut term = Term::new(24, 80, 1000, Theme::default(), VoidEffectSink);
    const MAX_BYTES: usize = 64;
    term.set_image_limits(usize::MAX, MAX_BYTES);

    // Create a 65-byte file (one beyond the limit)
    let fixture = TempFileFixture::new("oversized_t_eq_f", &[0xABu8; 65])
        .expect("failed to create temp file");

    let p = KittyStoreParams {
        image_id: 1,
        image_number: None,
        payload: fixture.path_str().into_bytes(),
        format: 32,
        width: 0,
        height: 0,
        transmission: KittyTransmission::File,
        compression: None,
    };

    let result = term.kitty_store_image(p);
    assert!(result.is_err(), "oversized file should return error");
    let err = result.unwrap_err();
    assert!(
        err.to_string().contains("EBIG"),
        "error should be EBIG (kitty spec for oversize), got: {}",
        err
    );

    // Verify the file still exists (t=f never removes)
    assert!(fixture.path().exists(), "t=f should NOT remove the file");
}

#[test]
fn kitty_store_from_file_within_size_succeeds_t_eq_t_removes_source() {
    // File ≤ max_bytes, t=t (TempFile), success path: stores image AND removes source
    let mut term = Term::new(24, 80, 1000, Theme::default(), VoidEffectSink);
    const MAX_BYTES: usize = 64;
    term.set_image_limits(usize::MAX, MAX_BYTES);

    // Create a 64-byte file (exactly at the limit) with valid RGBA data
    // format=32 (raw RGBA), width=4, height=4 means 4×4×4 bytes = 64 bytes
    let mut rgba_data = vec![0u8; 64];
    // Fill with recognizable pattern
    for i in 0..64 {
        rgba_data[i] = (i as u8).wrapping_mul(42);
    }

    let fixture =
        TempFileFixture::new("within_size_t_eq_t", &rgba_data).expect("failed to create temp file");
    let fixture_path = fixture.path().to_path_buf();

    let p = KittyStoreParams {
        image_id: 1,
        image_number: None,
        payload: fixture.path_str().into_bytes(),
        format: 32,
        width: 4,
        height: 4,
        transmission: KittyTransmission::TempFile, // t=t
        compression: None,
    };

    let result = term.kitty_store_image(p);
    assert!(
        result.is_ok(),
        "file within size should succeed, got: {:?}",
        result
    );

    // Verify the file was removed by the RAII guard
    assert!(
        !fixture_path.exists(),
        "t=t should remove the file on success"
    );
}

/// Kitty graphics-protocol.rst: oversize file → `EBIG`. With t=t the
/// RAII guard MUST remove the source even on rejection. Catalog row
/// `KG-RESPONSE-EBIG`.
#[test]
fn kitty_store_from_file_oversized_t_eq_t_returns_ebig_and_removes_source() {
    let mut term = Term::new(24, 80, 1000, Theme::default(), VoidEffectSink);
    const MAX_BYTES: usize = 64;
    term.set_image_limits(usize::MAX, MAX_BYTES);

    // Create a 65-byte file (one beyond the limit)
    let fixture = TempFileFixture::new("oversized_t_eq_t", &[0xCDu8; 65])
        .expect("failed to create temp file");
    let fixture_path = fixture.path().to_path_buf();

    let p = KittyStoreParams {
        image_id: 1,
        image_number: None,
        payload: fixture.path_str().into_bytes(),
        format: 32,
        width: 0,
        height: 0,
        transmission: KittyTransmission::TempFile, // t=t
        compression: None,
    };

    let result = term.kitty_store_image(p);
    assert!(result.is_err(), "oversized file should return error");
    let err = result.unwrap_err();
    assert!(
        err.to_string().contains("EBIG"),
        "error should be EBIG (kitty spec for oversize), got: {}",
        err
    );

    // Verify the file WAS removed by the RAII guard (t=t cleanup on error)
    assert!(
        !fixture_path.exists(),
        "t=t should remove the file even on EBIG rejection"
    );
}

/// Kitty graphics-protocol.rst: descriptor-open failure → `EBADF`.
/// EIO is reserved for mid-stream partial-read failures. Catalog row
/// `KG-RESPONSE-EBADF`.
#[test]
fn kitty_store_from_file_metadata_unavailable_returns_ebadf() {
    let mut term = Term::new(24, 80, 1000, Theme::default(), VoidEffectSink);

    let p = KittyStoreParams {
        image_id: 1,
        image_number: None,
        payload: "/nonexistent/path/that/definitely/does/not/exist"
            .as_bytes()
            .to_vec(),
        format: 32,
        width: 0,
        height: 0,
        transmission: KittyTransmission::File,
        compression: None,
    };

    let result = term.kitty_store_image(p);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(
        err.to_string().contains("EBADF") || err.to_string().contains("EINVAL"),
        "error should be EBADF or EINVAL (path not found), got: {}",
        err
    );
}

#[test]
#[cfg(unix)]
fn kitty_store_from_file_directory_path_returns_einval() {
    // Path points at a directory: EINVAL "path is not a regular file"
    use std::fs;

    let mut term = Term::new(24, 80, 1000, Theme::default(), VoidEffectSink);

    let dir = std::env::temp_dir();
    let test_dir_name = format!("kitty_test_dir_einval_{}", std::process::id());
    let test_dir = dir.join(&test_dir_name);

    // Create a temporary directory
    let _ = fs::create_dir(&test_dir);

    let p = KittyStoreParams {
        image_id: 1,
        image_number: None,
        payload: test_dir.to_string_lossy().to_string().into_bytes(),
        format: 32,
        width: 0,
        height: 0,
        transmission: KittyTransmission::File,
        compression: None,
    };

    let result = term.kitty_store_image(p);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(
        err.to_string().contains("EINVAL") && err.to_string().contains("regular file"),
        "error should be EINVAL with regular file message, got: {}",
        err
    );

    // Clean up
    let _ = fs::remove_dir(&test_dir);
}

/// Windows-only: `File::open` on a directory fails with `ACCESS_DENIED`
/// which the store layer maps to kitty's `EBADF` (open-failure code per
/// graphics-protocol.rst). Catalog row `KG-RESPONSE-EBADF`.
#[test]
#[cfg(windows)]
fn kitty_store_from_file_directory_path_returns_ebadf() {
    use std::fs;

    let mut term = Term::new(24, 80, 1000, Theme::default(), VoidEffectSink);

    let dir = std::env::temp_dir();
    let test_dir_name = format!("kitty_test_dir_ebadf_{}", std::process::id());
    let test_dir = dir.join(&test_dir_name);

    // Create a temporary directory
    let _ = fs::create_dir(&test_dir);

    let p = KittyStoreParams {
        image_id: 1,
        image_number: None,
        payload: test_dir.to_string_lossy().to_string().into_bytes(),
        format: 32,
        width: 0,
        height: 0,
        transmission: KittyTransmission::File,
        compression: None,
    };

    let result = term.kitty_store_image(p);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(
        err.to_string().contains("EBADF") && err.to_string().contains("failed to open"),
        "error should be EBADF from File::open, got: {}",
        err
    );

    // Clean up
    let _ = fs::remove_dir(&test_dir);
}

#[test]
fn kitty_store_from_file_empty_file_einval() {
    // 0-byte file: bounded read returns empty, kitty_decode_pixels rejects with EINVAL
    let mut term = Term::new(24, 80, 1000, Theme::default(), VoidEffectSink);

    let fixture = TempFileFixture::new("empty_file", &[]).expect("failed to create temp file");

    let p = KittyStoreParams {
        image_id: 1,
        image_number: None,
        payload: fixture.path_str().into_bytes(),
        format: 32,
        width: 4,
        height: 4,
        transmission: KittyTransmission::File,
        compression: None,
    };

    let result = term.kitty_store_image(p);
    assert!(result.is_err());
    let err = result.unwrap_err();
    // Empty file causes RGBA decode to fail (size mismatch)
    assert!(
        err.to_string().contains("EINVAL"),
        "error should be EINVAL from pixel decode, got: {}",
        err
    );
}

#[test]
fn kitty_store_from_file_exactly_max_bytes_succeeds() {
    // File exactly max_bytes: bounded read fills to exactly max_bytes,
    // post-read check does not trigger (max_bytes > max_bytes is false)
    let mut term = Term::new(24, 80, 1000, Theme::default(), VoidEffectSink);
    const MAX_BYTES: usize = 64;
    term.set_image_limits(usize::MAX, MAX_BYTES);

    // Create a 64-byte file with valid RGBA data
    let mut rgba_data = vec![0u8; 64];
    for i in 0..64 {
        rgba_data[i] = (i as u8).wrapping_mul(42);
    }

    let fixture =
        TempFileFixture::new("exactly_max_bytes", &rgba_data).expect("failed to create temp file");

    let p = KittyStoreParams {
        image_id: 1,
        image_number: None,
        payload: fixture.path_str().into_bytes(),
        format: 32,
        width: 4,
        height: 4,
        transmission: KittyTransmission::File,
        compression: None,
    };

    let result = term.kitty_store_image(p);
    assert!(
        result.is_ok(),
        "file exactly max_bytes should succeed, got: {:?}",
        result
    );
}

/// Kitty graphics-protocol.rst: oversize file → `EBIG`. Metadata preflight
/// rejects fast-path before the bounded read; ENOMEM is reserved for
/// allocator failure. Catalog row `KG-RESPONSE-EBIG`.
#[test]
fn kitty_store_from_file_max_bytes_plus_one_rejects() {
    let mut term = Term::new(24, 80, 1000, Theme::default(), VoidEffectSink);
    const MAX_BYTES: usize = 64;
    term.set_image_limits(usize::MAX, MAX_BYTES);

    // Create a 65-byte file
    let fixture = TempFileFixture::new("max_bytes_plus_one", &[0xDEu8; 65])
        .expect("failed to create temp file");

    let p = KittyStoreParams {
        image_id: 1,
        image_number: None,
        payload: fixture.path_str().into_bytes(),
        format: 32,
        width: 0,
        height: 0,
        transmission: KittyTransmission::File,
        compression: None,
    };

    let result = term.kitty_store_image(p);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(
        err.to_string().contains("EBIG"),
        "error should be EBIG from preflight (kitty spec for oversize), got: {}",
        err
    );
}

#[test]
fn kitty_store_from_file_max_bytes_usize_max_does_not_panic() {
    // max_bytes == usize::MAX: saturating_add returns u64::MAX,
    // take(u64::MAX) reads the entire file, no panic
    let mut term = Term::new(24, 80, 1000, Theme::default(), VoidEffectSink);
    term.set_image_limits(usize::MAX, usize::MAX);

    // Create a small file (no USIZE_MAX allocation)
    let small_data = [0xABu8; 64];
    let fixture = TempFileFixture::new("max_bytes_usize_max", &small_data)
        .expect("failed to create temp file");

    let p = KittyStoreParams {
        image_id: 1,
        image_number: None,
        payload: fixture.path_str().into_bytes(),
        format: 32,
        width: 4,
        height: 4,
        transmission: KittyTransmission::File,
        compression: None,
    };

    let result = term.kitty_store_image(p);
    assert!(
        result.is_ok(),
        "max_bytes=usize::MAX should not panic, got: {:?}",
        result
    );
}

#[test]
fn kitty_store_from_file_path_traversal_rejected() {
    // Path with ".." component: EINVAL "path traversal not allowed"
    let mut term = Term::new(24, 80, 1000, Theme::default(), VoidEffectSink);

    let p = KittyStoreParams {
        image_id: 1,
        image_number: None,
        payload: b"../../etc/passwd".to_vec(),
        format: 32,
        width: 0,
        height: 0,
        transmission: KittyTransmission::File,
        compression: None,
    };

    let result = term.kitty_store_image(p);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(
        err.to_string().contains("path traversal"),
        "path traversal should be rejected, got: {}",
        err
    );
}

/// Post-cure pin replacing the legacy rejection assertion
/// `kitty_compression_oz_constructs_typed_variant_with_pinned_reply_bytes`
/// — `o=z` is no longer rejected with `KittyError::CompressionNotSupported`;
/// it round-trips through the shared `prepare_image_bytes` helper). Feeds
/// a 1x1 RGBA payload zlib-compressed with `o=z` and asserts the cache
/// contains an image whose decoded bytes match the original RGBA byte-
/// for-byte.
/// See: bug-tracker/plans/BUG-06-086/section-03b-tdd-matrix.md
/// §"NEW-1.3 — kitty_store_image integration tests".
///
/// EXPECTED-FAIL pre-Phase 4 (helper stub returns STUB error). Phase 4
/// fills in the Some(b'z') branch and this pin goes green.
#[test]
fn kitty_store_image_oz_f32_round_trips_decompressed_rgba() {
    use crate::image::ImageId;
    use flate2::Compression;
    use flate2::write::ZlibEncoder;
    use std::io::Write;

    let mut term = Term::new(24, 80, 1000, Theme::default(), VoidEffectSink);

    let raw_rgba = vec![0xFF, 0x00, 0x00, 0xFF]; // 1x1 red pixel.
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(&raw_rgba).expect("encode");
    let compressed = encoder.finish().expect("finish");

    let p = KittyStoreParams {
        image_id: 1,
        image_number: None,
        payload: compressed,
        format: 32,
        width: 1,
        height: 1,
        transmission: KittyTransmission::Direct,
        compression: Some(b'z'),
    };

    term.kitty_store_image(p)
        .expect("o=z MUST round-trip post-cure (Phase 4)");

    let stored = term
        .image_cache()
        .get_no_touch(ImageId(1))
        .expect("image MUST land in cache");
    assert_eq!(
        stored.width, 1,
        "decoded width MUST match s=1; got {}",
        stored.width
    );
    assert_eq!(
        stored.height, 1,
        "decoded height MUST match v=1; got {}",
        stored.height
    );
    assert_eq!(
        *stored.data, raw_rgba,
        "decompressed RGBA MUST equal the pre-compression bytes",
    );
}

/// `o=z` over `f=24` (RGB packed) decodes + rgb_to_rgba expands to RGBA in
/// cache. EXPECTED-FAIL pre-Phase 4.
#[test]
fn kitty_store_image_oz_f24_round_trips_rgb_to_rgba() {
    use crate::image::ImageId;
    use flate2::Compression;
    use flate2::write::ZlibEncoder;
    use std::io::Write;

    let mut term = Term::new(24, 80, 1000, Theme::default(), VoidEffectSink);

    // 1x1 RGB green pixel (3 bytes).
    let raw_rgb = vec![0x00, 0xFF, 0x00];
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(&raw_rgb).expect("encode");
    let compressed = encoder.finish().expect("finish");

    let p = KittyStoreParams {
        image_id: 2,
        image_number: None,
        payload: compressed,
        format: 24,
        width: 1,
        height: 1,
        transmission: KittyTransmission::Direct,
        compression: Some(b'z'),
    };

    term.kitty_store_image(p)
        .expect("o=z + f=24 MUST round-trip post-cure");

    let stored = term
        .image_cache()
        .get_no_touch(ImageId(2))
        .expect("image MUST land in cache");
    assert_eq!(stored.data.as_ref(), &[0x00, 0xFF, 0x00, 0xFF]);
}

/// `o=z` over `f=100` (PNG) — payload is a zlib-compressed PNG; helper
/// decompresses to the PNG bytes, then `decode_to_rgba` parses to RGBA.
/// EXPECTED-FAIL pre-Phase 4.
#[test]
fn kitty_store_image_oz_f100_round_trips_png() {
    use crate::image::ImageId;
    use flate2::Compression;
    use flate2::write::ZlibEncoder;
    use std::io::Write;

    let mut term = Term::new(24, 80, 1000, Theme::default(), VoidEffectSink);

    // 1x1 PNG (red pixel) — minimum-valid PNG byte sequence.
    let png_red_1x1: Vec<u8> = vec![
        0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, // signature
        0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52, // IHDR len + tag
        0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, // w=1, h=1
        0x08, 0x02, 0x00, 0x00, 0x00, 0x90, 0x77, 0x53, // depth=8, color=2 (RGB)
        0xDE, // IHDR CRC
        0x00, 0x00, 0x00, 0x0C, 0x49, 0x44, 0x41, 0x54, // IDAT len + tag
        0x08, 0x99, 0x63, 0xF8, 0xCF, 0xC0, 0x00, 0x00, 0x00, 0x03, 0x00, 0x01, // deflated red px
        0x9A, 0xF6, 0x4A, 0xE8, // IDAT CRC
        0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4E, 0x44, // IEND
        0xAE, 0x42, 0x60, 0x82,
    ];

    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(&png_red_1x1).expect("encode");
    let compressed = encoder.finish().expect("finish");

    let p = KittyStoreParams {
        image_id: 3,
        image_number: None,
        payload: compressed,
        format: 100,
        width: 1,
        height: 1,
        transmission: KittyTransmission::Direct,
        compression: Some(b'z'),
    };

    term.kitty_store_image(p)
        .expect("o=z + f=100 MUST round-trip post-cure");

    let stored = term
        .image_cache()
        .get_no_touch(ImageId(3))
        .expect("PNG image MUST land in cache");
    assert_eq!(stored.width, 1);
    assert_eq!(stored.height, 1);
}

/// Corrupt zlib payload under `o=z` returns EINVAL via the store layer's
/// `Reply` variant; the cache stays empty. EXPECTED-FAIL pre-Phase 4 (stub
/// returns `"STUB: …"` not `"EINVAL: …"`).
#[test]
fn kitty_store_image_oz_corrupt_returns_einval_via_reply() {
    use crate::image::ImageId;

    let mut term = Term::new(24, 80, 1000, Theme::default(), VoidEffectSink);

    // Random non-zlib bytes — decoder rejects mid-stream.
    let garbage = vec![0xFF, 0xFE, 0xFD, 0xFC, 0xFB, 0xFA, 0xF9, 0xF8];

    let p = KittyStoreParams {
        image_id: 4,
        image_number: None,
        payload: garbage,
        format: 32,
        width: 1,
        height: 1,
        transmission: KittyTransmission::Direct,
        compression: Some(b'z'),
    };

    let err = term
        .kitty_store_image(p)
        .expect_err("corrupt o=z MUST be rejected");
    assert!(
        err.to_string().contains("EINVAL"),
        "corrupt o=z MUST emit EINVAL — got {err}",
    );
    assert!(
        term.image_cache().get_no_touch(ImageId(4)).is_none(),
        "rejected o=z MUST NOT populate cache",
    );
}

/// `o=x` (unknown compression code) → EINVAL `unsupported compression o=x`
/// emitted via `KittyStoreError::Reply`; cache stays empty. EXPECTED-FAIL
/// pre-Phase 4. Fixes the silent-treat-as-uncompressed shape — pre-helper,
/// `Some(b'x')` would have flowed through to the decoder as if uncompressed.
#[test]
fn kitty_store_image_unknown_compression_returns_einval() {
    use crate::image::ImageId;

    let mut term = Term::new(24, 80, 1000, Theme::default(), VoidEffectSink);

    let p = KittyStoreParams {
        image_id: 5,
        image_number: None,
        payload: vec![0xFF, 0x00, 0x00, 0xFF],
        format: 32,
        width: 1,
        height: 1,
        transmission: KittyTransmission::Direct,
        compression: Some(b'x'),
    };

    let err = term
        .kitty_store_image(p)
        .expect_err("unknown compression MUST be rejected");
    let msg = err.to_string();
    assert!(
        msg.contains("EINVAL") && msg.contains("o=x"),
        "unknown o=x MUST emit EINVAL with offending value — got {msg:?}",
    );
    assert!(
        term.image_cache().get_no_touch(ImageId(5)).is_none(),
        "rejected unknown compression MUST NOT populate cache",
    );
}

// ===========================================================================
// NEW-1.5 — kitty_store_from_file integration with o=z
// ===========================================================================

/// File-backed transmit with `o=z`: the file contains zlib-compressed RGBA;
/// helper decompresses + decode_pixels produces the original RGBA in cache.
/// EXPECTED-FAIL pre-Phase 4 (no helper insertion at the file-read site yet).
#[test]
fn kitty_store_from_file_oz_decompresses_file_bytes() {
    use crate::image::ImageId;
    use flate2::Compression;
    use flate2::write::ZlibEncoder;
    use std::io::Write;

    let mut term = Term::new(24, 80, 1000, Theme::default(), VoidEffectSink);
    term.set_image_limits(usize::MAX, 64 * 1024);

    // 4×4 RGBA pattern (64 bytes raw), zlib-compressed onto disk.
    let raw_rgba: Vec<u8> = (0..64u8).map(|i| i.wrapping_mul(7)).collect();
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(&raw_rgba).expect("encode");
    let compressed = encoder.finish().expect("finish");
    let fixture = TempFileFixture::new("oz_decompress", &compressed)
        .expect("temp file write");

    let p = KittyStoreParams {
        image_id: 10,
        image_number: None,
        payload: fixture.path_str().into_bytes(),
        format: 32,
        width: 4,
        height: 4,
        transmission: KittyTransmission::File,
        compression: Some(b'z'),
    };

    term.kitty_store_image(p)
        .expect("t=f + o=z MUST decompress file bytes post-cure");

    let stored = term
        .image_cache()
        .get_no_touch(ImageId(10))
        .expect("image MUST land in cache");
    assert_eq!(stored.width, 4);
    assert_eq!(stored.height, 4);
    assert_eq!(*stored.data, raw_rgba);
    assert!(fixture.path().exists(), "t=f MUST NOT remove the file");
}

/// Corrupt zlib payload in a file → EINVAL emitted; cache stays empty.
/// `TempFileGuard` cleanup still fires on the rejection path (the post-
/// helper `?` short-circuits but the guard's Drop runs). EXPECTED-FAIL
/// pre-Phase 4.
#[test]
fn kitty_store_from_file_oz_corrupt_returns_einval() {
    use crate::image::ImageId;

    let mut term = Term::new(24, 80, 1000, Theme::default(), VoidEffectSink);
    term.set_image_limits(usize::MAX, 64 * 1024);

    let fixture = TempFileFixture::new("oz_corrupt_file", &vec![0xFFu8; 32])
        .expect("temp file write");
    let fixture_path = fixture.path().to_path_buf();

    let p = KittyStoreParams {
        image_id: 11,
        image_number: None,
        payload: fixture.path_str().into_bytes(),
        format: 32,
        width: 4,
        height: 4,
        transmission: KittyTransmission::TempFile, // t=t — guard arms removal
        compression: Some(b'z'),
    };

    let err = term
        .kitty_store_image(p)
        .expect_err("corrupt zlib file MUST be rejected");
    assert!(
        err.to_string().contains("EINVAL"),
        "corrupt zlib in file MUST emit EINVAL — got {err}",
    );
    assert!(
        term.image_cache().get_no_touch(ImageId(11)).is_none(),
        "rejected file transmit MUST NOT populate cache",
    );
    // t=t cleanup invariant — guard fires on every exit path including
    // the new helper-error path.
    assert!(
        !fixture_path.exists(),
        "t=t MUST remove the file even on helper rejection",
    );
}

/// Clamp added per Plan TPR R3 — uncompressed file transmits MUST still
/// land in cache post-cure. Pins that the helper insertion at the file-
/// read site does not regress the existing `compression: None` path.
/// PASSES today (no helper at file-read site yet) AND must still pass
/// after Phase 4 wires the helper.
#[test]
fn kitty_store_from_file_no_compression_still_reads_and_stores_raw_file() {
    use crate::image::ImageId;

    let mut term = Term::new(24, 80, 1000, Theme::default(), VoidEffectSink);
    term.set_image_limits(usize::MAX, 64 * 1024);

    let raw_rgba: Vec<u8> = (0..64u8).map(|i| i.wrapping_mul(13)).collect();
    let fixture = TempFileFixture::new("no_compress_file", &raw_rgba)
        .expect("temp file write");

    let p = KittyStoreParams {
        image_id: 12,
        image_number: None,
        payload: fixture.path_str().into_bytes(),
        format: 32,
        width: 4,
        height: 4,
        transmission: KittyTransmission::File,
        compression: None,
    };

    term.kitty_store_image(p)
        .expect("uncompressed t=f MUST round-trip");

    let stored = term
        .image_cache()
        .get_no_touch(ImageId(12))
        .expect("image MUST land in cache");
    assert_eq!(*stored.data, raw_rgba);
}
