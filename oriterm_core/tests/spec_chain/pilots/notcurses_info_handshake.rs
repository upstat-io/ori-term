//! Replay the captured `notcurses-info` startup query bundle and assert
//! every reply ori_term emits that notcurses-info's `display_logo()`
//! gate depends on.
//!
//! See:
//!
//! Gate chain (per `notcurses/src/info/main.c:524-530` +
//! `notcurses/src/lib/notcurses.c:1156-1161`):
//!
//! ```c
//! if(notcurses_canpixel(nc)){
//! display_logo(stdn, path); // wordmark rendering path
//! }
//! // canpixel returns nc->tcache.pixel_implementation iff
//! // tcache.cellpxy != 0 && tcache.cellpxx != 0
//! ```
//!
//! `pixel_implementation` is set from the kitty graphics `a=q` reply
//! (NCPIXEL_KITTY_*) or from XTSMGRAPHICS Pi=2 reply with non-zero
//! sixel geometry (NCPIXEL_SIXEL). `cellpxy`/`cellpxx` come from
//! `CSI 14t` reply pixel dims divided by `CSI 18t` reply cell dims,
//! or `tiocgwinsz` `ws_xpixel`/`ws_ypixel` (typically 0 over a PTY).
//!
//! Each assertion below is its own `#[test]` so failures point at the
//! specific reply that's missing (no assertion-roulette per `tests.md
//! §Test Hygiene`).

use oriterm_core::effect::{Effect, PtyEffect};
use oriterm_test_support::spec_chain::{SpecHarness, pty_writes};

/// Load the captured notcurses-info startup query stream from the
/// wrapper repo. Three-state return distinguishes (a) wrapper absent
/// → graceful skip, (b) capture readable, (c) wrapper present but
/// capture unreadable (propagate I/O error).
fn notcurses_info_startup_bytes() -> std::io::Result<Option<Vec<u8>>> {
    let Some(captures) = oriterm_test_support::paths::captures_dir() else {
        return Ok(None);
    };
    let path = captures.join("notcurses-info-startup.cap");
    std::fs::read(&path).map(Some)
}

/// Feed the captured bytes through `SpecHarness` and concatenate every
/// PTY-write reply ori_term emits, in emission order.
fn replay_and_collect_replies() -> Option<Vec<u8>> {
    let bytes = match notcurses_info_startup_bytes() {
        Ok(Some(b)) => b,
        Ok(None) => {
            eprintln!("SKIP: notcurses-info-startup.cap not present (no wrapper)");
            return None;
        }
        Err(e) => panic!("failed to read notcurses-info-startup.cap: {e}"),
    };
    let mut h = SpecHarness::new();
    h.feed(&bytes);
    let mut out = Vec::new();
    for (bytes, _) in pty_writes(&h) {
        out.extend_from_slice(bytes);
    }
    Some(out)
}

/// Parse the `Ph` and `Pw` parameters out of a `CSI 4 ; Ph ; Pw t` reply
/// somewhere in `stream`. Returns `None` if no such reply is present.
fn extract_csi14t_reply(stream: &[u8]) -> Option<(u32, u32)> {
    // Find `\x1b[4;` literally, then parse `<num>;<num>t`.
    let needle = b"\x1b[4;";
    let mut i = 0;
    while i + needle.len() < stream.len() {
        if &stream[i..i + needle.len()] == needle {
            let after = &stream[i + needle.len()..];
            let mut j = 0;
            while j < after.len() && after[j].is_ascii_digit() {
                j += 1;
            }
            if j == 0 || j >= after.len() || after[j] != b';' {
                i += 1;
                continue;
            }
            let h: u32 = std::str::from_utf8(&after[..j]).ok()?.parse().ok()?;
            let rest = &after[j + 1..];
            let mut k = 0;
            while k < rest.len() && rest[k].is_ascii_digit() {
                k += 1;
            }
            if k == 0 || k >= rest.len() || rest[k] != b't' {
                i += 1;
                continue;
            }
            let w: u32 = std::str::from_utf8(&rest[..k]).ok()?.parse().ok()?;
            return Some((h, w));
        }
        i += 1;
    }
    None
}

/// Parse `Pwidth` and `Pheight` out of an XTSMGRAPHICS Pi=2 reply
/// (`\x1b[?2;0;Pwidth;Pheight S`) somewhere in `stream`. Returns
/// `None` if absent or has non-success status.
fn extract_xtsmgraphics_sixel_geometry(stream: &[u8]) -> Option<(u32, u32)> {
    let needle = b"\x1b[?2;0;";
    let mut i = 0;
    while i + needle.len() < stream.len() {
        if &stream[i..i + needle.len()] == needle {
            let after = &stream[i + needle.len()..];
            let mut j = 0;
            while j < after.len() && after[j].is_ascii_digit() {
                j += 1;
            }
            if j == 0 || j >= after.len() || after[j] != b';' {
                i += 1;
                continue;
            }
            let w: u32 = std::str::from_utf8(&after[..j]).ok()?.parse().ok()?;
            let rest = &after[j + 1..];
            let mut k = 0;
            while k < rest.len() && rest[k].is_ascii_digit() {
                k += 1;
            }
            if k == 0 || k >= rest.len() || rest[k] != b'S' {
                i += 1;
                continue;
            }
            let h: u32 = std::str::from_utf8(&rest[..k]).ok()?.parse().ok()?;
            return Some((w, h));
        }
        i += 1;
    }
    None
}

/// Regression: `\x1b[c` (DA1) reply must be present. notcurses uses DA1
/// as the end-of-handshake marker; without it notcurses blocks waiting
/// for the reply and never proceeds to `display_logo()`.
#[test]
fn notcurses_info_replies_include_da1() {
    let Some(stream) = replay_and_collect_replies() else {
        return;
    };
    assert!(
        find_subslice(&stream, b"\x1b[?64;6;4c").is_some()
            || find_subslice(&stream, b"\x1b[?6").is_some(),
        "DA1 reply (\\x1b[?64;6;4c) missing from notcurses-info handshake replies. \
 Full stream:\n{:?}",
        String::from_utf8_lossy(&stream)
    );
}

/// Regression: `\x1b_Gi=1,a=q;\x1b\\` (kitty graphics query) must be
/// answered with `\x1b_Gi=1;OK\x1b\\` so notcurses sets
/// `pixel_implementation = NCPIXEL_KITTY_*`. Without this reply
/// notcurses falls through to sixel detection only.
#[test]
fn notcurses_info_replies_include_kitty_graphics_ok() {
    let Some(stream) = replay_and_collect_replies() else {
        return;
    };
    assert!(
        find_subslice(&stream, b"\x1b_Gi=1;OK\x1b\\").is_some(),
        "Kitty graphics OK reply (\\x1b_Gi=1;OK\\x1b\\\\) missing from \
 notcurses-info handshake replies. Full stream:\n{:?}",
        String::from_utf8_lossy(&stream)
    );
}

/// Regression: `CSI 14 t` (text area in pixels) reply must be present
/// with non-zero height AND width. notcurses derives `cellpxy` and
/// `cellpxx` from `Ph / rows` and `Pw / cols`; if either component is
/// zero, `notcurses_check_pixel_support()` early-returns
/// `NCPIXEL_NONE` and `display_logo()` is skipped.
#[test]
fn notcurses_info_replies_include_nonzero_csi14t() {
    let Some(stream) = replay_and_collect_replies() else {
        return;
    };
    let (h, w) = extract_csi14t_reply(&stream).unwrap_or_else(|| {
        panic!(
            "CSI 14t reply (\\x1b[4;H;Wt) missing from notcurses-info handshake \
 replies. Full stream:\n{:?}",
            String::from_utf8_lossy(&stream)
        )
    });
    assert!(
        h > 0 && w > 0,
        "CSI 14t reply must have non-zero pixel dims; got h={h} w={w}. \
 Zero dims would make notcurses cellpxy/cellpxx=0 → canpixel=false."
    );
}

/// Regression: XTSMGRAPHICS Pi=2 Pa=1 (read sixel geometry) reply must
/// report non-zero sixel max width/height with success status (Ps=0).
/// This is the fallback path notcurses uses when the kitty graphics OK
/// reply is absent — without non-zero sixel geometry, notcurses sets
/// `pixel_implementation = NCPIXEL_NONE`.
#[test]
fn notcurses_info_replies_include_nonzero_xtsmgraphics_sixel() {
    let Some(stream) = replay_and_collect_replies() else {
        return;
    };
    let (w, h) = extract_xtsmgraphics_sixel_geometry(&stream).unwrap_or_else(|| {
        panic!(
            "XTSMGRAPHICS Pi=2 success reply (\\x1b[?2;0;W;H S) missing. \
 Full stream:\n{:?}",
            String::from_utf8_lossy(&stream)
        )
    });
    assert!(
        w > 0 && h > 0,
        "XTSMGRAPHICS Pi=2 reply must have non-zero sixel max dims; got w={w} h={h}."
    );
}

/// Diagnostic: dump every reply the handshake emits. Pinned as
/// `#[test]` so it runs alongside the assertion tests above and the
/// dump is captured in test output via `cargo test -- --nocapture`.
#[test]
fn notcurses_info_handshake_dump() {
    let bytes = match notcurses_info_startup_bytes() {
        Ok(Some(b)) => b,
        Ok(None) => {
            eprintln!("SKIP: notcurses-info-startup.cap not present");
            return;
        }
        Err(e) => panic!("failed to read notcurses-info-startup.cap: {e}"),
    };
    let mut h = SpecHarness::new();
    h.feed(&bytes);
    let mut pty_count = 0usize;
    let mut total_bytes = 0usize;
    eprintln!("--- notcurses-info handshake reply dump ---");
    for (i, eff) in h.outcome().effects_emitted.iter().enumerate() {
        if let Effect::Pty(PtyEffect::Write { bytes, kind }) = eff {
            pty_count += 1;
            total_bytes += bytes.len();
            eprintln!(
                " [{i}] kind={:?} ({} bytes): {:?}",
                kind,
                bytes.len(),
                String::from_utf8_lossy(bytes),
            );
        }
    }
    eprintln!("--- {pty_count} PTY replies, {total_bytes} total bytes ---");
    assert!(
        pty_count > 0,
        "notcurses-info handshake must emit at least one PTY reply"
    );
}

/// Find `needle` as a contiguous substring of `haystack`. Returns the
/// start index of the first match, or `None`.
fn find_subslice(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() || needle.len() > haystack.len() {
        return None;
    }
    haystack.windows(needle.len()).position(|w| w == needle)
}
