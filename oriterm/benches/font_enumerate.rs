//! Benchmarks for the OS-font enumeration hot path.
//!
//! Measures `enumerate_mono_families_from_roots()` against synthetic
//! tempdir-backed font fixtures so the benchmark exercises the cache-free
//! path (the public `enumerate_mono_families()` is `OnceLock`-cached and
//! would only measure on first call).
//!
//! Default `cargo test` does NOT run benches. Invoke via `cargo bench
//! --bench font_enumerate`. Linux/macOS only — DirectWrite enumeration on
//! Windows is benched indirectly via the public cached entry point because
//! it has no path-roots concept.

#![cfg(any(target_os = "linux", target_os = "macos"))]

use std::path::PathBuf;

use criterion::{Criterion, black_box, criterion_group, criterion_main};
use oriterm::bench_seam::enumerate_mono_families_from_roots;
use tempfile::TempDir;

const JBM_REGULAR: &[u8] = include_bytes!("../fonts/JetBrainsMono-Regular.ttf");

fn make_synthetic_fixture(file_count: usize) -> TempDir {
    let dir = tempfile::tempdir().expect("tempdir for bench fixture");
    for i in 0..file_count {
        let path = dir.path().join(format!("font-{i}.ttf"));
        std::fs::write(&path, JBM_REGULAR).expect("write fixture file");
    }
    dir
}

fn bench_enumerate_synthetic_50(c: &mut Criterion) {
    let fixture = make_synthetic_fixture(50);
    let roots: Vec<PathBuf> = vec![fixture.path().to_path_buf()];
    c.bench_function("enumerate_mono_families_50_synthetic_files", |b| {
        b.iter(|| {
            let entries = enumerate_mono_families_from_roots(black_box(&roots));
            black_box(entries);
        });
    });
}

fn bench_enumerate_synthetic_500(c: &mut Criterion) {
    let fixture = make_synthetic_fixture(500);
    let roots: Vec<PathBuf> = vec![fixture.path().to_path_buf()];
    c.bench_function("enumerate_mono_families_500_synthetic_files", |b| {
        b.iter(|| {
            let entries = enumerate_mono_families_from_roots(black_box(&roots));
            black_box(entries);
        });
    });
}

/// Real-system-fonts benchmark — exercises the same hot path against the
/// host's actual font directories. On Linux this typically includes one or
/// more `.ttc` collection files (Noto CJK, DejaVu fallback chains), so this
/// covers the "large-TTC" perf surface called out in §03 without requiring
/// a synthetic `.ttc` fixture in the repo. Skips silently when no system
/// font dirs exist on the runner (CI sandbox without `/usr/share/fonts`).
fn bench_enumerate_system_fonts(c: &mut Criterion) {
    // Linux + macOS standard font directories. The `is_dir()` filter drops
    // platform-irrelevant paths automatically — running on Linux skips the
    // `/Library/Fonts` and `/System/Library/Fonts` paths, and vice versa on
    // macOS, with no platform `cfg` needed.
    let mut candidate_roots: Vec<PathBuf> = Vec::new();
    if let Some(home) = std::env::var_os("HOME") {
        candidate_roots.push(PathBuf::from(home.clone()).join(".local/share/fonts"));
        candidate_roots.push(PathBuf::from(home).join("Library/Fonts"));
    }
    candidate_roots.extend([
        PathBuf::from("/usr/share/fonts"),
        PathBuf::from("/usr/local/share/fonts"),
        PathBuf::from("/Library/Fonts"),
        PathBuf::from("/System/Library/Fonts"),
        PathBuf::from("/System/Library/Fonts/Supplemental"),
    ]);
    candidate_roots.retain(|p| p.is_dir());
    if candidate_roots.is_empty() {
        eprintln!(
            "SKIP bench_enumerate_system_fonts: no system font directories found on this runner"
        );
        return;
    }
    c.bench_function("enumerate_mono_families_real_system_fonts", |b| {
        b.iter(|| {
            let entries = enumerate_mono_families_from_roots(black_box(&candidate_roots));
            black_box(entries);
        });
    });
}

criterion_group!(
    benches,
    bench_enumerate_synthetic_50,
    bench_enumerate_synthetic_500,
    bench_enumerate_system_fonts,
);
criterion_main!(benches);
