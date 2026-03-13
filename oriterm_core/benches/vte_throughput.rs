//! VTE parser throughput benchmarks.
//!
//! Measures end-to-end throughput: raw bytes → VTE parser → `Term` handler →
//! grid cells. Three workloads model the spectrum of real PTY output:
//!
//! - **ASCII-only**: `cat large_file.txt`, compiler output, `git log`.
//! - **Mixed**: realistic terminal with interleaved SGR color + cursor movement.
//! - **Heavy escape**: worst case — dense escape sequences every few characters.

use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};

use oriterm_core::event::VoidListener;
use oriterm_core::term::Term;
use oriterm_core::theme::Theme;
use vte::ansi::Processor;

/// Terminal sizes for benchmarks.
const SIZES: [(usize, usize); 2] = [
    (120, 50), // Modern split pane.
    (240, 80), // Full-screen 4K.
];

/// Target throughput: >100 MB/s ASCII, >50 MB/s mixed.
///
/// Baseline (2026-03-12, WSL2, Ryzen, pre-fast-path):
///   ASCII-only:    ~71 MiB/s (120x50), ~66 MiB/s (240x80)
///   Mixed:         ~77 MiB/s (120x50), ~66 MiB/s (240x80)
///   Heavy escape:  ~145 MiB/s (120x50), ~142 MiB/s (240x80)
///
/// After fast ASCII path (23.2):
///   ASCII-only:    ~86 MiB/s (120x50), ~83 MiB/s (240x80)
///   Mixed:         ~112 MiB/s (120x50), ~115 MiB/s (240x80)
///   Heavy escape:  ~159 MiB/s (120x50), ~167 MiB/s (240x80)
const BUFFER_SIZE: usize = 1024 * 1024; // 1 MB

// ---------------------------------------------------------------------------
// Input generators
// ---------------------------------------------------------------------------

/// 1 MB of printable ASCII (0x20–0x7E) with newlines every ~120 chars.
///
/// Models `cat large_file.txt` — the most common terminal workload.
fn ascii_buffer(cols: usize) -> Vec<u8> {
    let mut buf = Vec::with_capacity(BUFFER_SIZE);
    let mut col = 0;
    while buf.len() < BUFFER_SIZE {
        if col >= cols {
            buf.push(b'\n');
            col = 0;
        } else {
            // Cycle through printable ASCII.
            buf.push(b' ' + (buf.len() % 95) as u8);
            col += 1;
        }
    }
    buf
}

/// 1 MB of terminal output with interleaved SGR color and cursor movement.
///
/// Pattern: ~10 chars of text, then an SGR 256-color sequence, then ~10 chars,
/// then a cursor-right. Models colorized compiler output (`cargo build`,
/// `gcc -fdiagnostics-color`).
fn mixed_buffer(cols: usize) -> Vec<u8> {
    let mut buf = Vec::with_capacity(BUFFER_SIZE);
    let mut col = 0;
    let mut seq_counter: u8 = 0;
    while buf.len() < BUFFER_SIZE {
        if col >= cols {
            buf.push(b'\n');
            col = 0;
            continue;
        }
        // Every 10 chars, insert an SGR 256-color sequence.
        if col % 10 == 0 && col > 0 {
            // ESC[38;5;Nm — set foreground to indexed color N.
            let color = seq_counter;
            seq_counter = seq_counter.wrapping_add(1);
            buf.extend_from_slice(b"\x1b[38;5;");
            buf.extend_from_slice(color.to_string().as_bytes());
            buf.push(b'm');
        }
        // Every 20 chars, insert a cursor-right instead of text.
        if col % 20 == 19 {
            buf.extend_from_slice(b"\x1b[C"); // CUF — cursor forward.
            col += 1;
            continue;
        }
        buf.push(b'a' + (col % 26) as u8);
        col += 1;
    }
    buf
}

/// 1 MB of dense escape sequences — worst case for the VTE parser.
///
/// Every 5 characters has a color change (ESC[3Nm). Models heavy TUI redraws
/// where almost every cell has a different color.
fn heavy_escape_buffer(cols: usize) -> Vec<u8> {
    let mut buf = Vec::with_capacity(BUFFER_SIZE);
    let mut col = 0;
    let mut fg: u8 = 0;
    while buf.len() < BUFFER_SIZE {
        if col >= cols {
            buf.push(b'\n');
            col = 0;
            continue;
        }
        // Color change every 5 chars.
        if col % 5 == 0 {
            // ESC[38;2;R;G;Bm — truecolor, most expensive SGR.
            buf.extend_from_slice(b"\x1b[38;2;");
            buf.extend_from_slice(fg.to_string().as_bytes());
            buf.push(b';');
            buf.extend_from_slice(fg.wrapping_add(50).to_string().as_bytes());
            buf.push(b';');
            buf.extend_from_slice(fg.wrapping_add(100).to_string().as_bytes());
            buf.push(b'm');
            fg = fg.wrapping_add(1);
        }
        buf.push(b'X');
        col += 1;
    }
    buf
}

// ---------------------------------------------------------------------------
// Benchmarks
// ---------------------------------------------------------------------------

/// ASCII-only throughput: printable characters + newlines, no escape sequences.
///
/// This is the fast path — `Processor::advance` batches consecutive printable
/// bytes into a single `input(&str)` call, then `Term::input()` calls
/// `grid.put_char()` per character. Target: >100 MB/s.
fn bench_vte_ascii_only(c: &mut Criterion) {
    let mut group = c.benchmark_group("vte/ascii_only");
    group.throughput(criterion::Throughput::Bytes(BUFFER_SIZE as u64));
    for &(cols, lines) in &SIZES {
        let buf = ascii_buffer(cols);
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{cols}x{lines}")),
            &(cols, lines, &buf),
            |b, &(cols, lines, buf)| {
                let mut term = Term::new(lines, cols, 1000, Theme::default(), VoidListener);
                let mut proc: Processor = Processor::new();
                b.iter(|| {
                    proc.advance(&mut term, black_box(buf));
                });
            },
        );
    }
    group.finish();
}

/// Mixed throughput: text + SGR color sequences + cursor movement.
///
/// Models realistic colorized compiler output. Exercises the CSI parser and
/// the SGR handler alongside the fast `input` path. Target: >50 MB/s.
fn bench_vte_mixed(c: &mut Criterion) {
    let mut group = c.benchmark_group("vte/mixed");
    group.throughput(criterion::Throughput::Bytes(BUFFER_SIZE as u64));
    for &(cols, lines) in &SIZES {
        let buf = mixed_buffer(cols);
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{cols}x{lines}")),
            &(cols, lines, &buf),
            |b, &(cols, lines, buf)| {
                let mut term = Term::new(lines, cols, 1000, Theme::default(), VoidListener);
                let mut proc: Processor = Processor::new();
                b.iter(|| {
                    proc.advance(&mut term, black_box(buf));
                });
            },
        );
    }
    group.finish();
}

/// Heavy escape throughput: dense truecolor SGR on nearly every cell.
///
/// Worst case for the parser — more time in state machine transitions than
/// in `input()`. Models TUI frameworks that set truecolor on every cell.
fn bench_vte_heavy_escape(c: &mut Criterion) {
    let mut group = c.benchmark_group("vte/heavy_escape");
    group.throughput(criterion::Throughput::Bytes(BUFFER_SIZE as u64));
    for &(cols, lines) in &SIZES {
        let buf = heavy_escape_buffer(cols);
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{cols}x{lines}")),
            &(cols, lines, &buf),
            |b, &(cols, lines, buf)| {
                let mut term = Term::new(lines, cols, 1000, Theme::default(), VoidListener);
                let mut proc: Processor = Processor::new();
                b.iter(|| {
                    proc.advance(&mut term, black_box(buf));
                });
            },
        );
    }
    group.finish();
}

criterion_group!(
    benches,
    bench_vte_ascii_only,
    bench_vte_mixed,
    bench_vte_heavy_escape,
);
criterion_main!(benches);
