//! Per-format rung (§13.1) — `f=24` RGB, `f=32` RGBA, `f=100` PNG.
//!
//! Drives the format dispatch at `kitty_decode_pixels` (`store.rs:46-77`)
//! through success + negative EINVAL paths.

use oriterm_test_support::spec_chain::SpecHarness;

#[cfg(feature = "image-protocol")]
use super::fixtures::png_1x1_red;
use super::fixtures::{
    b64, kitty_apc, ok_reply_for, placement_count, reply_bytes, reply_contains, rgb_4x4_red,
    rgba_4x4_red,
};

/// Catalog row: `KG-TRANSMIT-FORMAT-32` (raw RGBA, 4 bytes per pixel).
#[test]
fn kitty_format_32_rgba_stores_and_places() {
    let mut h = SpecHarness::new();
    h.feed(&kitty_apc(b"a=T,i=10,f=32,s=4,v=4", &b64(&rgba_4x4_red())));

    assert_eq!(placement_count(&h), 1);
    assert!(reply_contains(&h, &ok_reply_for(10)));
}

/// Catalog row: `KG-TRANSMIT-FORMAT-24` (raw RGB expanded to RGBA via rgb_to_rgba at store.rs:70-72).
#[test]
fn kitty_format_24_rgb_stores_and_places() {
    let mut h = SpecHarness::new();
    h.feed(&kitty_apc(b"a=T,i=11,f=24,s=4,v=4", &b64(&rgb_4x4_red())));

    assert_eq!(placement_count(&h), 1);
    assert!(reply_contains(&h, &ok_reply_for(11)));
}

/// Catalog row: `KG-TRANSMIT-FORMAT-100` (PNG decoded via decode_to_rgba at store.rs:74).
///
/// Feature-gated because `decode_to_rgba` at `oriterm_core/src/image/decode.rs:83`
/// is `#[cfg(feature = "image-protocol")]` — without the feature it returns a stub
/// `Err` and the success assertion would fail. The feature is in default features,
/// so normal `cargo test` runs exercise this test; `--no-default-features` cleanly
/// skips it.
#[cfg(feature = "image-protocol")]
#[test]
fn kitty_format_100_png_stores_and_places() {
    let mut h = SpecHarness::new();
    h.feed(&kitty_apc(b"a=T,i=12,f=100", &b64(&png_1x1_red())));

    assert_eq!(placement_count(&h), 1);
    assert!(reply_contains(&h, &ok_reply_for(12)));
}

/// Catalog row: `KG-TRANSMIT-FORMAT-32` (negative — payload-size EINVAL path at store.rs:58-62).
///
/// Pins the payload-size error path: declared `s=4,v=4,f=32` = 64 bytes but only
/// 32 payload bytes arrive → EINVAL reply with the specific suffix.
#[test]
fn kitty_format_32_wrong_payload_size_emits_einval_reply() {
    let mut h = SpecHarness::new();
    let short = &rgba_4x4_red()[..32];
    h.feed(&kitty_apc(b"a=t,i=13,f=32,s=4,v=4", &b64(short)));

    let replies = reply_bytes(&h);
    let s = String::from_utf8_lossy(&replies);
    assert!(
        s.contains("EINVAL: RGBA payload size"),
        "f=32 with wrong payload length emits EINVAL with the size-mismatch \
         suffix — got {s:?}",
    );
}

/// Catalog row: `KG-FORMAT-UNSUPPORTED` (store.rs:75 catchall emits EINVAL unsupported format).
#[test]
fn kitty_format_unsupported_emits_einval_reply() {
    let mut h = SpecHarness::new();
    h.feed(&kitty_apc(b"a=t,i=14,f=42,s=4,v=4", &b64(&rgba_4x4_red())));

    let replies = reply_bytes(&h);
    let s = String::from_utf8_lossy(&replies);
    assert!(
        s.contains("EINVAL: unsupported format"),
        "f=42 must emit EINVAL unsupported-format reply — got {s:?}",
    );
}
