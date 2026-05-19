//! RSS regression for cross-protocol image churn (sixel ↔ kitty).
//!
//! Lives in a dedicated integration-test binary so the RSS samples observe
//! ONLY this test's work. The sibling `rss_regression.rs` binary fills
//! scrollback with multi-MB grids; running that test in the same process
//! would contaminate every absolute-RSS sample with its scrollback growth,
//! making a tight `baseline * 1.10` trend cap unenforceable. Cargo runs
//! each `tests/*.rs` as a separate binary, so this file is process-isolated
//! from `rss_regression.rs`.
//!
//! Cross-platform: Linux reads `/proc/self/statm`, macOS uses `task_info`,
//! Windows uses `GetProcessMemoryInfo`. Other platforms skip at compile time.

#![cfg(any(target_os = "linux", target_os = "macos", windows))]

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as B64;
use oriterm_core::effect::VoidEffectSink;
use oriterm_core::{Term, Theme};
use oriterm_test_support::spec_chain::sixel_fixtures::dcs_n_cols_wide;

const MB: usize = 1_048_576;

fn rss_bytes() -> usize {
    platform_rss().expect("failed to read process RSS")
}

#[cfg(target_os = "linux")]
fn platform_rss() -> Option<usize> {
    let statm = std::fs::read_to_string("/proc/self/statm").ok()?;
    let resident_pages: usize = statm.split_whitespace().nth(1)?.parse().ok()?;
    Some(resident_pages * 4096)
}

#[cfg(target_os = "macos")]
fn platform_rss() -> Option<usize> {
    #[allow(unsafe_code, deprecated)]
    unsafe {
        let mut info: libc::mach_task_basic_info_data_t = std::mem::zeroed();
        let mut count = libc::MACH_TASK_BASIC_INFO_COUNT;
        let kr = libc::task_info(
            libc::mach_task_self_,
            libc::MACH_TASK_BASIC_INFO,
            std::ptr::addr_of_mut!(info).cast(),
            &mut count,
        );
        if kr == libc::KERN_SUCCESS {
            Some(info.resident_size as usize)
        } else {
            None
        }
    }
}

#[cfg(windows)]
fn platform_rss() -> Option<usize> {
    use windows_sys::Win32::System::ProcessStatus::{
        GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS,
    };
    use windows_sys::Win32::System::Threading::GetCurrentProcess;

    #[allow(unsafe_code)]
    unsafe {
        let mut counters: PROCESS_MEMORY_COUNTERS = std::mem::zeroed();
        counters.cb = size_of::<PROCESS_MEMORY_COUNTERS>() as u32;
        let ok = GetProcessMemoryInfo(
            GetCurrentProcess(),
            &mut counters,
            size_of::<PROCESS_MEMORY_COUNTERS>() as u32,
        );
        if ok != 0 {
            Some(counters.WorkingSetSize)
        } else {
            None
        }
    }
}

/// Encode a kitty graphics APC: `\x1b_G<control>;<payload>\x1b\\`.
/// Empty payload omits the `;` separator.
fn kitty_apc(control: &[u8], payload_b64: &str) -> Vec<u8> {
    let mut out = Vec::with_capacity(4 + control.len() + 1 + payload_b64.len() + 2);
    out.extend_from_slice(b"\x1b_G");
    out.extend_from_slice(control);
    if !payload_b64.is_empty() {
        out.push(b';');
        out.extend_from_slice(payload_b64.as_bytes());
    }
    out.extend_from_slice(b"\x1b\\");
    out
}

/// 4×4 opaque-red raw RGBA — 64 bytes (matches `s=4,v=4,f=32`).
fn rgba_4x4_red() -> Vec<u8> {
    let mut v = Vec::with_capacity(64);
    for _ in 0..16 {
        v.extend_from_slice(&[255, 0, 0, 255]);
    }
    v
}

/// Drive one cache-cycling alternation: sixel transmit → sixel-placement
/// delete (kitty `d=p`) → kitty transmit+place → kitty delete-by-id.
///
/// Important: the cycle is NOT strictly net-zero — `d=p` (lowercase)
/// removes only the sixel PLACEMENT, leaving the sixel image data in the
/// cache unplaced. Each cycle therefore accumulates one unplaced sixel
/// image. The 64 KB cache cap + the now-unplaced (eviction-eligible)
/// status of those sixels means the LRU eviction layer drops the oldest
/// unplaced sixel once total memory exceeds the cap — that's the
/// steady-state plateau the trend test pins. The kitty side IS strictly
/// net-zero (`a=T` creates, `d=I` fully destroys image + placement).
///
/// Per `decisions/01-lru-eviction-scope-placed-vs-unplaced.md` Option A:
/// placed/anchored images are eviction-immune; only the unplaced pool
/// participates. Each completed cycle's leftover sixel is unplaced and
/// therefore evictable, so the cache can plateau under churn without
/// leaking RSS. A monotonically-growing trend would indicate a real
/// leak OUTSIDE the unplaced sixel debt (orphaned anchors, retained
/// snapshot scratch, accumulating placements, PTY reply queue).
fn alternation_cycle(term: &mut Term<VoidEffectSink>, parser: &mut vte::ansi::Processor, id: u32) {
    parser.advance(term, b"\x1b[1;1H");
    parser.advance(term, &dcs_n_cols_wide(8));

    parser.advance(term, &kitty_apc(b"a=d,d=p,x=1,y=1", ""));

    parser.advance(term, b"\x1b[5;1H");
    let payload = B64.encode(rgba_4x4_red());
    let control = format!("a=T,i={id},f=32,s=4,v=4");
    parser.advance(term, &kitty_apc(control.as_bytes(), &payload));

    parser.advance(term, &kitty_apc(format!("a=d,d=I,i={id}").as_bytes(), ""));
}

/// Rapid sixel↔kitty alternation must hold RSS bounded across iterations.
///
/// 5 post-warmup samples; assert each later peak is within 10% of the
/// first measured iteration (per spec-conformance §13.6.1 plan-body trend
/// pattern). Wall-clock-free per `tests.md §Wall-Clock-Free Testing` —
/// samples are condition-driven (work units), not timer-driven.
///
/// Single-delta `rss_after - rss_before` is allocator-fragmentation false-
/// positive prone (glibc retains freed pages, jemalloc rounds up). The
/// multi-iteration trend with a 10% tolerance distinguishes a real leak
/// (proportional unbounded growth) from allocator caching (steady-state
/// plateau).
///
/// Regression: spec-conformance §13.6.1 — rapid alternation stress test
/// (wall-clock-free RSS trend).
#[test]
fn rss_bounded_under_rapid_sixel_kitty_alternation() {
    let mut term = Term::<VoidEffectSink>::new(24, 80, 1000, Theme::default(), VoidEffectSink);
    let mut parser: vte::ansi::Processor = vte::ansi::Processor::new();

    // Cap cache memory TIGHT enough that eviction actually fires under
    // the cycle's accumulated unplaced-sixel debt. With `dcs_n_cols_wide(8)`
    // producing 192 bytes per sixel image + ImageData metadata, a 64 KB
    // cap forces LRU eviction within INNER iterations (~300 sixels fit
    // before the cap → eviction kicks in around iteration 300+). The
    // INNER count below is chosen so each outer iteration drives well
    // past the cap, exercising the eviction layer rather than just the
    // unbounded-growth detection.
    term.image_cache_mut().set_memory_limit(64 * 1024);

    const INNER: u32 = 500;
    const OUTER: usize = 5;

    // Warmup: one outer iteration to settle allocator heap layout, sixel
    // parser scratch buffers, and snapshot/Vec capacities.
    for n in 0..INNER {
        alternation_cycle(&mut term, &mut parser, n + 1);
    }

    let mut samples = Vec::with_capacity(OUTER);
    for i in 0..OUTER {
        for n in 0..INNER {
            let id = INNER * (i as u32 + 1) + n + 1;
            alternation_cycle(&mut term, &mut parser, id);
        }
        samples.push(rss_bytes());
    }

    let baseline = samples[0];
    let max_later = *samples.iter().max().expect("OUTER > 0");
    let cap = baseline + baseline / 10; // baseline * 1.10

    let in_mb: Vec<String> = samples
        .iter()
        .map(|&b| format!("{:.1}", b as f64 / MB as f64))
        .collect();

    assert!(
        max_later <= cap,
        "RSS exceeded 10% tolerance under rapid sixel↔kitty alternation \
         (cycle drives past the 64 KB cache cap; eviction layer must \
         hold steady-state RSS bounded). \
         Samples (bytes): {:?}, in MB: {:?}, baseline cap: {:.1} MB, max: {:.1} MB. \
         Each cycle creates one sixel + one kitty (places kitty, deletes sixel), \
         capped at 64 KB image-cache budget; eviction layer drives the steady-\
         state plateau as unplaced sixel debt drops the oldest entries. Growth beyond 10% above the post-\
         warmup baseline implies a leak outside the image bytes themselves \
         (placement vectors, parser scratch, snapshot/scrollback retention, \
         orphaned anchors). Reverse the §13.6.1 recency-update SSOT or the \
         eviction immunity invariant and this test fails.",
        samples,
        in_mb,
        cap as f64 / MB as f64,
        max_later as f64 / MB as f64,
    );
}
