//! Decode-cost criterion bench for the kitty `o=z` zlib decompression
//! path.
//!
//! Three workloads exercise different size regimes:
//! - `zlib_decompress_xray_shape_2_25mb` — xray-geometry (999×562 RGBA,
//!   2.25 MB decoded) gradient payload, representative of notcurses-demo
//!   xray's per-frame size.
//! - `zlib_decompress_small_128b` — small-payload startup-cost variant.
//! - `zlib_decompress_small_4kb` — small-payload tail variant.
//!
//! Run via:
//! ```text
//! cargo bench -p oriterm_core --bench kitty_decode
//! ```

use criterion::{Criterion, black_box, criterion_group, criterion_main};

/// xray's per-frame geometry: 999×562 RGBA = 2.25 MB raw.
const XRAY_WIDTH: usize = 999;
const XRAY_HEIGHT: usize = 562;

/// Default zlib compression level (6) — matches `Compression::default()`
/// in flate2 and `notcurses`'s zlib invocation per the kitty protocol.
const ZLIB_LEVEL: u8 = 6;

/// Compress via `miniz_oxide` directly so the bench fixture is
/// independent of the flate2 backend feature under test. Cross-backend
/// bench comparison then measures pure decoder cost without
/// encoder-side variance.
fn compress_fixture(raw: &[u8]) -> Vec<u8> {
    miniz_oxide::deflate::compress_to_vec_zlib(raw, ZLIB_LEVEL)
}

/// Build a representative xray-shape compressed payload: a smoothly
/// varying RGBA gradient at xray's geometry. Mirror the test corpus
/// pattern (smooth x/y gradient) so the bench fixture's compression
/// ratio is realistic — a solid-fill payload compresses pathologically
/// well, which doesn't exercise the decoder at xray's real per-frame
/// size.
fn xray_compressed_payload() -> Vec<u8> {
    let raw = oriterm_test_support::fixtures::xray_gradient_rgba(XRAY_WIDTH, XRAY_HEIGHT);
    compress_fixture(&raw)
}

fn bench_zlib_decompress_xray_shape(c: &mut Criterion) {
    let compressed = xray_compressed_payload();
    let expected_size = XRAY_WIDTH * XRAY_HEIGHT * 4;

    c.bench_function("zlib_decompress_xray_shape_2_25mb", |b| {
        b.iter(|| {
            // Direct flate2 decode — mirrors what `prepare_image_bytes`
            // does internally. Measures the underlying decoder cost
            // independent of the helper's `Vec::with_capacity` + bounded-
            // read overhead.
            use flate2::read::ZlibDecoder;
            use std::io::Read;

            let mut decoder = ZlibDecoder::new(black_box(compressed.as_slice()));
            let mut out = Vec::with_capacity(expected_size);
            decoder.read_to_end(&mut out).expect("zlib decode");
            black_box(out);
        });
    });
}

/// Small-payload bench variants — SIMD startup-cost regression detection.
///
/// SIMD-aware zlib backends have a per-call startup cost; small kitty
/// transmits (under ~4 KB) may regress vs the scalar fallback. Notcurses
/// xray uses 2.25 MB frames so the primary workload is in the large
/// regime, but other workloads with small `o=z` transmits could regress.
///
/// 128-byte and 4-KB variants alongside the 2.25 MB bench surface any
/// startup-cost asymmetry. If >1.5× regression at small sizes lands, the
/// owning plan scope-expands with a hybrid-backend wrapper item.

fn small_payload(size: usize) -> Vec<u8> {
    // Synthetic small payload — pseudo-random pattern to avoid trivial
    // run-length compression. Backend behavior on actual small zlib
    // payloads matters; using a fixed seed keeps the bench deterministic.
    let raw: Vec<u8> =
        (0..size).map(|i| ((i.wrapping_mul(0x9E3779B9)) & 0xFF) as u8).collect();
    compress_fixture(&raw)
}

fn bench_zlib_decompress_small_128b(c: &mut Criterion) {
    let compressed = small_payload(128);

    c.bench_function("zlib_decompress_small_128b", |b| {
        b.iter(|| {
            use flate2::read::ZlibDecoder;
            use std::io::Read;

            let mut decoder = ZlibDecoder::new(black_box(compressed.as_slice()));
            let mut out = Vec::with_capacity(128);
            decoder.read_to_end(&mut out).expect("zlib decode");
            black_box(out);
        });
    });
}

fn bench_zlib_decompress_small_4kb(c: &mut Criterion) {
    let compressed = small_payload(4096);

    c.bench_function("zlib_decompress_small_4kb", |b| {
        b.iter(|| {
            use flate2::read::ZlibDecoder;
            use std::io::Read;

            let mut decoder = ZlibDecoder::new(black_box(compressed.as_slice()));
            let mut out = Vec::with_capacity(4096);
            decoder.read_to_end(&mut out).expect("zlib decode");
            black_box(out);
        });
    });
}

criterion_group!(
    benches,
    bench_zlib_decompress_xray_shape,
    bench_zlib_decompress_small_128b,
    bench_zlib_decompress_small_4kb
);
criterion_main!(benches);
