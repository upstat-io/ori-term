//! Decode-cost criterion bench for the kitty `o=z` zlib decompression
//! path — Item 7 follow-on gate.
//!
//! See: bug-tracker/plans/BUG-06-086/section-03b-tdd-matrix.md
//! §"REQUIRED — decode-cost criterion benchmark".
//!
//! Phase 3 (TDD-first) ships this skeleton with a placeholder workload so
//! the bench compiles + runs end-to-end. Phase 4 wires the real cure
//! surface (the shared `prepare_image_bytes` helper) and replaces the
//! placeholder with an xray-shape compressed-RGBA workload (~199 KB
//! compressed → 2.25 MB decoded, mirroring xray's 999×562 per-frame
//! geometry). The post-Phase-4 bench drives Item 7's
//! `Vec::with_capacity ≥ 20%` attribution gate per §05b.
//!
//! Run via:
//! ```text
//! cargo bench -p oriterm_core --bench kitty_decode
//! ```

use std::io::Write;

use criterion::{Criterion, black_box, criterion_group, criterion_main};
use flate2::Compression;
use flate2::write::ZlibEncoder;

/// xray's per-frame geometry: 999×562 RGBA = 2.25 MB raw.
const XRAY_WIDTH: usize = 999;
const XRAY_HEIGHT: usize = 562;

/// Build a representative xray-shape compressed payload: solid-color
/// RGBA at xray's geometry, zlib-encoded. Solid fill compresses well
/// (~10× ratio) — representative of xray's high inter-pixel correlation.
fn xray_compressed_payload() -> Vec<u8> {
    let raw_size = XRAY_WIDTH * XRAY_HEIGHT * 4;
    let raw = vec![0x80u8; raw_size];
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(&raw).expect("zlib encode");
    encoder.finish().expect("zlib finish")
}

fn bench_zlib_decompress_xray_shape(c: &mut Criterion) {
    let compressed = xray_compressed_payload();
    let expected_size = XRAY_WIDTH * XRAY_HEIGHT * 4;

    c.bench_function("zlib_decompress_xray_shape_2_25mb", |b| {
        b.iter(|| {
            // Direct flate2 decode — mirrors what `prepare_image_bytes`
            // will do internally post-Phase 4. Pre-Phase 4 this measures
            // the underlying decoder cost so we can compare against the
            // helper's overhead once it lands.
            use flate2::read::ZlibDecoder;
            use std::io::Read;

            let mut decoder = ZlibDecoder::new(black_box(&compressed[..]));
            let mut out = Vec::with_capacity(expected_size);
            decoder.read_to_end(&mut out).expect("zlib decode");
            black_box(out);
        });
    });
}

criterion_group!(benches, bench_zlib_decompress_xray_shape);
criterion_main!(benches);
