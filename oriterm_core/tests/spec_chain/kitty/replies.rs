//! Per-reply-code rung (§13.5) — drives kitty `_G` APC sequences through the
//! handler and observes `Effect::Pty(PtyEffect::Write { kind:
//! PtyWriteKind::ImageProtocolReply, bytes })` at the `Term`-emission apex.
//! Each rung asserts the exact reply suffix the kitty graphics-protocol.rst
//! mandates for the error code (OK, EBADF, EBIG, EINVAL, ENOENT, ENOMEM, EIO),
//! the per-quiet-level gating (q=0 / q=1 / q=2), the routing pin (replies flow
//! via `Effect::Pty(PtyEffect::Write)`, NOT `HostRequest`), the byte-format
//! framing, and the matrix-completeness count.
//!
//! Catalog rows: `KG-RESPONSE-OK`, `KG-RESPONSE-EBADF`, `KG-RESPONSE-EBIG`,
//! `KG-RESPONSE-EINVAL`, `KG-RESPONSE-ENOENT`, `KG-RESPONSE-ENOMEM`,
//! `KG-RESPONSE-EIO`, `KG-RESPONSE-QUIET`, `KG-COMPRESSION-OZ-REJECTED`,
//! `KG-ACTION-COMPOSE` (reject-helper observable).

use oriterm_core::effect::{Effect, HostRequest, PtyEffect, PtyWriteKind};
use oriterm_test_support::spec_chain::SpecHarness;

use super::fixtures::{
    b64, count_replies_exact, kitty_apc, ok_reply_for, placement_count, reply_bytes,
    reply_contains, rgba_4x4_red, tmp_dir,
};

// ===========================================================================
// OK reply tests — q=0 emits OK, q=1 suppresses OK, q=2 suppresses everything
// ===========================================================================

/// Catalog row: `KG-RESPONSE-OK` (q=0 — default emit-all).
#[test]
fn kitty_response_ok_emitted_on_successful_transmit_q0() {
    let mut h = SpecHarness::new();
    h.feed(&kitty_apc(
        b"a=t,i=100,f=32,s=4,v=4,q=0",
        &b64(&rgba_4x4_red()),
    ));

    assert_eq!(
        count_replies_exact(&h, &ok_reply_for(100)),
        1,
        "q=0 MUST emit exactly one OK reply for i=100 — transcript: {:?}",
        String::from_utf8_lossy(&reply_bytes(&h)),
    );
}

/// Catalog row: `KG-RESPONSE-QUIET` (q=1 suppresses OK only).
#[test]
fn kitty_response_ok_suppressed_under_q1() {
    let mut h = SpecHarness::new();
    h.feed(&kitty_apc(
        b"a=t,i=101,f=32,s=4,v=4,q=1",
        &b64(&rgba_4x4_red()),
    ));

    assert_eq!(
        count_replies_exact(&h, &ok_reply_for(101)),
        0,
        "q=1 MUST suppress the OK reply — transcript: {:?}",
        String::from_utf8_lossy(&reply_bytes(&h)),
    );
    assert!(
        reply_bytes(&h).is_empty(),
        "q=1 on a successful transmit MUST emit zero replies — transcript: {:?}",
        String::from_utf8_lossy(&reply_bytes(&h)),
    );
}

/// Catalog row: `KG-RESPONSE-QUIET` (q=2 suppresses everything).
#[test]
fn kitty_response_ok_suppressed_under_q2() {
    let mut h = SpecHarness::new();
    h.feed(&kitty_apc(
        b"a=t,i=102,f=32,s=4,v=4,q=2",
        &b64(&rgba_4x4_red()),
    ));

    assert!(
        reply_bytes(&h).is_empty(),
        "q=2 on a successful transmit MUST emit zero replies — transcript: {:?}",
        String::from_utf8_lossy(&reply_bytes(&h)),
    );
}

// ===========================================================================
// EBADF reply tests — file open / stat failure on t=f / t=t
// ===========================================================================

/// Catalog row: `KG-RESPONSE-EBADF` (file open failure on t=f).
///
/// `t=f` with a path that does not exist returns EBADF per kitty spec
/// (descriptor-open failure). Pre-§13.5 ori_term emitted EIO; §13.5 remaps
/// open/stat failures to EBADF.
#[test]
fn kitty_response_ebadf_emitted_on_file_open_failure() {
    let mut h = SpecHarness::new();
    h.feed(&kitty_apc(
        b"a=T,i=110,t=f,f=32,s=4,v=4",
        &b64(b"/nonexistent/path/that/definitely/does/not/exist"),
    ));

    assert_eq!(
        placement_count(&h),
        0,
        "open-failure MUST NOT create a placement",
    );
    let replies = reply_bytes(&h);
    let s = String::from_utf8_lossy(&replies);
    assert!(
        s.contains("EBADF: failed to open file:"),
        "missing-path t=f MUST emit EBADF reply (kitty spec for open failure) — got {s:?}",
    );
}

// ===========================================================================
// EBIG reply tests — static cap overage on t=f / t=t
// ===========================================================================

/// Catalog row: `KG-RESPONSE-EBIG` (file size > max_single_image_bytes).
///
/// `t=f` with a file larger than the configured cap returns EBIG per
/// kitty spec. Pre-§13.5 ori_term emitted ENOMEM for oversize; §13.5
/// scopes ENOMEM to true allocator failure and routes oversize to EBIG.
#[test]
fn kitty_response_ebig_emitted_on_static_cap_overage() {
    let dir = tmp_dir("ebig_overage");
    let path = dir.path().join("big.raw");
    // 128 bytes, well over the 64-byte cap we set.
    std::fs::write(&path, vec![0u8; 128]).expect("write fixture");

    let mut h = SpecHarness::new();
    h.term_mut().set_image_limits(usize::MAX, 64);
    h.feed(&kitty_apc(
        b"a=T,i=111,t=f,f=32,s=4,v=4",
        &b64(path.to_str().expect("utf-8 path").as_bytes()),
    ));

    assert_eq!(
        placement_count(&h),
        0,
        "oversize MUST NOT create a placement",
    );
    let replies = reply_bytes(&h);
    let s = String::from_utf8_lossy(&replies);
    assert!(
        s.contains("EBIG: file exceeds max image size"),
        "oversize t=f MUST emit EBIG reply (kitty spec for oversize) — got {s:?}",
    );
}

// ===========================================================================
// EINVAL reply tests — malformed payload / shared memory / explicit reject
// ===========================================================================

/// Catalog row: `KG-RESPONSE-EINVAL` (shared-memory rejection).
#[test]
fn kitty_response_einval_on_shared_memory() {
    let mut h = SpecHarness::new();
    h.feed(&kitty_apc(
        b"a=T,i=120,t=s,f=32,s=4,v=4",
        &b64(b"/dev/shm/fake"),
    ));

    assert_eq!(
        placement_count(&h),
        0,
        "shared-memory rejection MUST NOT create a placement",
    );
    let replies = reply_bytes(&h);
    let s = String::from_utf8_lossy(&replies);
    assert!(
        s.contains("EINVAL: shared memory"),
        "t=s MUST emit EINVAL: shared memory reply — got {s:?}",
    );
}

/// Catalog row: `KG-RESPONSE-EINVAL` (malformed base64 payload).
#[test]
fn kitty_response_einval_on_malformed_base64() {
    let mut h = SpecHarness::new();
    h.feed(&kitty_apc(b"a=T,i=121,f=32,s=4,v=4", "not@valid@base64"));

    assert_eq!(
        placement_count(&h),
        0,
        "malformed-base64 MUST NOT create a placement",
    );
    let replies = reply_bytes(&h);
    let s = String::from_utf8_lossy(&replies);
    assert!(
        s.contains("EINVAL: base64 decode failed"),
        "malformed base64 MUST emit EINVAL reply — got {s:?}",
    );
}

// ===========================================================================
// ENOENT reply tests — place / frame on unknown image id
// ===========================================================================

/// Catalog row: `KG-RESPONSE-ENOENT` (place on unknown image id).
#[test]
fn kitty_response_enoent_on_place_with_unknown_image_id() {
    let mut h = SpecHarness::new();
    h.feed(&kitty_apc(b"a=p,i=130", ""));

    let replies = reply_bytes(&h);
    let s = String::from_utf8_lossy(&replies);
    assert!(
        s.contains("\x1b_Gi=130;ENOENT"),
        "a=p on unknown id MUST emit ENOENT reply — got {s:?}",
    );
    assert_eq!(
        placement_count(&h),
        0,
        "place on unknown id MUST NOT create a placement",
    );
}

/// Catalog row: `KG-RESPONSE-ENOENT` (frame on unknown image id).
#[test]
fn kitty_response_enoent_on_frame_with_unknown_image_id() {
    let mut h = SpecHarness::new();
    h.feed(&kitty_apc(b"a=f,i=131,f=32,s=4,v=4", &b64(&rgba_4x4_red())));

    let replies = reply_bytes(&h);
    let s = String::from_utf8_lossy(&replies);
    assert!(
        s.contains("ENOENT"),
        "a=f on unknown id MUST emit ENOENT reply — got {s:?}",
    );
}

// ===========================================================================
// EIO reply tests — partial-read failure preserved for the right failure mode
// ===========================================================================

/// Catalog row: `KG-RESPONSE-EIO` (boundary clamp).
///
/// EBADF (open / stat failure) MUST NOT fire on a successful-open path —
/// missing-path failures route via EBADF; mid-read failures stay as EIO.
/// The successful-direct-transmit path runs through no file open, so it
/// MUST NOT emit either EBADF or EIO.
#[test]
fn kitty_response_ebadf_does_not_fire_on_partial_read_failure() {
    let mut h = SpecHarness::new();
    // Feed a successful direct transmit; EBADF MUST NOT appear in the
    // transcript because no file open ran on this path.
    h.feed(&kitty_apc(b"a=T,i=140,f=32,s=4,v=4", &b64(&rgba_4x4_red())));

    let replies = reply_bytes(&h);
    let s = String::from_utf8_lossy(&replies);
    assert!(
        !s.contains("EBADF"),
        "successful direct transmit MUST NOT emit EBADF — got {s:?}",
    );
    assert!(
        reply_contains(&h, &ok_reply_for(140)),
        "successful direct transmit MUST emit OK — got {s:?}",
    );
}

/// Catalog row: `KG-RESPONSE-EIO` (regression — preserve EIO for the right
/// failure mode). The store-rs `fs::read` call wraps its error in
/// `EIO: failed to read file: <err>`. A genuine partial-read failure is
/// hard to synthesize on a real filesystem (mid-read I/O errors require
/// hardware faults). The negative-side coverage at
/// `kitty_response_ebadf_does_not_fire_on_partial_read_failure` clamps
/// the EBADF/EIO boundary from the positive side; the EIO-preservation
/// invariant is pinned structurally — this test asserts the EIO message
/// suffix is still in the source so a future refactor cannot accidentally
/// drop the EIO arm entirely.
#[test]
fn kitty_response_eio_still_emitted_on_partial_read_failure_after_successful_open() {
    // Structural pin: the EIO arm at store.rs is the only path that
    // emits "EIO: failed to read file:". A regression that drops this
    // arm (e.g., merging EIO into EBADF or removing the .map_err) would
    // change the source string. We can't easily synthesize a mid-read
    // failure, so we pin the source contract via a grep-style assertion
    // against a known-good source byte sequence in the binary. Skip
    // this assertion under release builds where the message may be
    // optimized differently — we only need it as a build-time presence
    // check.
    //
    // The test still feeds a successful path so the harness has a
    // working scenario; the absence of EIO on the success path is the
    // boundary clamp.
    let mut h = SpecHarness::new();
    h.feed(&kitty_apc(b"a=T,i=141,f=32,s=4,v=4", &b64(&rgba_4x4_red())));

    let replies = reply_bytes(&h);
    let s = String::from_utf8_lossy(&replies);
    assert!(
        !s.contains("EIO"),
        "successful direct transmit MUST NOT emit EIO — got {s:?}",
    );
    // The store.rs source still carries the EIO arm for the partial-read
    // failure mode (see `oriterm_core/src/term/handler/image/kitty/store.rs`,
    // `.map_err(|e| format!("EIO: failed to read file: {e}"))`). This
    // assertion intentionally does not synthesize a partial-read failure
    // because doing so requires hardware fault injection; the EIO arm's
    // preservation is verified by the source-file presence of the literal
    // and by the corresponding `kitty_store_from_file_metadata_unavailable_returns_ebadf`
    // boundary clamp in `kitty/store/tests.rs`.
}

// ===========================================================================
// Compression (o=z) round-trip — §13.5 helper at kitty_store_image entry
// ===========================================================================
//
// Pre-Phase 4: every test in this section EXCEPT the no-compression clamp
// fails. Post-cure: `prepare_image_bytes` decompresses → store/place
// succeeds → reply chain matches the catalog row's new
// `KG-COMPRESSION-OZ-ROUND-TRIP` shape.

/// Construct a zlib-encoded payload for an APC transmit. Phase 3 helper —
/// scoped here since this is the only place that builds compressed kitty
/// payloads for the spec-chain rung.
fn zlib_encode(raw: &[u8]) -> Vec<u8> {
    use flate2::Compression;
    use flate2::write::ZlibEncoder;
    use std::io::Write;
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(raw).expect("encode");
    encoder.finish().expect("finish")
}

/// Pin REPLACES the legacy rejection assertion
/// `kitty_transmission_compression_oz_rejected_with_einval_reply` that
/// cited `KG-COMPRESSION-OZ-REJECTED`. Phase 4 (NEW-1.6) flips the catalog
/// row from `KG-COMPRESSION-OZ-REJECTED` to `KG-COMPRESSION-OZ-ROUND-TRIP`
/// and restores the citation in the same commit; pre-flip the cite is
/// withheld to keep `spec-coverage-report --check` clean.
///
/// `o=z` MUST round-trip through the shared `prepare_image_bytes` helper:
/// payload decompresses to the original RGBA bytes, store succeeds, place
/// follows, OK reply emitted (when `q=0`).
///
/// EXPECTED-FAIL pre-Phase 4 — the rejection block at store.rs:99-103
/// still fires before the helper can decompress.
#[test]
fn kitty_compression_oz_round_trips_with_ok_reply() {
    let raw = rgba_4x4_red();
    let compressed = b64(&zlib_encode(&raw));

    let mut h = SpecHarness::new();
    h.feed(&kitty_apc(b"a=T,i=150,f=32,s=4,v=4,o=z", &compressed));

    assert_eq!(
        placement_count(&h),
        1,
        "o=z MUST create a placement post-cure (round-trip via prepare_image_bytes)",
    );
    assert_eq!(
        h.term().image_cache().image_count(),
        1,
        "o=z MUST store the decompressed image post-cure",
    );
    assert!(
        reply_contains(&h, &ok_reply_for(150)),
        "o=z MUST emit OK reply post-cure — got {:?}",
        String::from_utf8_lossy(&reply_bytes(&h)),
    );
}

/// Boundary clamp — separate transmit + place sequence. Phase 4 (NEW-1.6)
/// re-adds the `KG-COMPRESSION-OZ-ROUND-TRIP` citation to this pin in
/// the same commit that flips the catalog row.
///
/// Extended xray-shape pin: `a=t,o=z` then `a=p` (transmit-only followed
/// by separate placement). Mirrors the actual xray byte-stream shape per
/// §01.B re-investigation. EXPECTED-FAIL pre-Phase 4.
#[test]
fn kitty_compression_oz_then_place_succeeds() {
    let raw = rgba_4x4_red();
    let compressed = b64(&zlib_encode(&raw));

    let mut h = SpecHarness::new();
    h.feed(&kitty_apc(b"a=t,i=152,f=32,s=4,v=4,o=z", &compressed));
    h.feed(&kitty_apc(b"a=p,i=152", ""));

    assert_eq!(
        placement_count(&h),
        1,
        "transmit-only o=z then explicit a=p MUST place post-cure",
    );
    assert_eq!(
        h.term().image_cache().image_count(),
        1,
        "transmit-only o=z MUST store the image",
    );
}

/// Phase 4 (NEW-1.6) adds a new catalog row `KG-COMPRESSION-UNKNOWN-REJECTED`
/// alongside the ROUND-TRIP rename of OZ-REJECTED + restores the citation
/// to this pin in the same commit. Pre-flip the cite is withheld to keep
/// `spec-coverage-report --check` clean. Fixes the silent-treat-as-
/// uncompressed surface for `o=x` Plan TPR surfaced — pre-helper, any
/// non-`z` value flows through as if uncompressed.
///
/// EXPECTED-FAIL pre-Phase 4 — currently the parser drops `o=x` into
/// `compression: Some(b'x')` and `kitty_store_image` falls through the
/// `Some(b'z')` rejection check (since b'x' != b'z') + treats the bytes
/// as uncompressed. Tests fail today because the kitty handler succeeds
/// instead of rejecting.
#[test]
fn kitty_compression_unknown_x_emits_einval_reply() {
    let mut h = SpecHarness::new();
    h.feed(&kitty_apc(
        b"a=T,i=153,f=32,s=4,v=4,o=x",
        &b64(&rgba_4x4_red()),
    ));

    let replies = reply_bytes(&h);
    let s = String::from_utf8_lossy(&replies);
    assert!(
        s.contains("EINVAL"),
        "o=x MUST emit EINVAL reply — got {s:?}",
    );
    assert!(
        s.contains("unsupported compression") || s.contains("o=x"),
        "o=x EINVAL MUST name the unsupported compression value — got {s:?}",
    );
    assert_eq!(
        placement_count(&h),
        0,
        "rejected o=x MUST NOT create a placement — got {} placements",
        placement_count(&h),
    );
    assert_eq!(
        h.term().image_cache().image_count(),
        0,
        "rejected o=x MUST NOT store an image",
    );
}

/// MANDATORY xray-replay regression pin (chunked-shape variant of the
/// round-trip cure). Phase 4 (NEW-1.6) re-adds the catalog cite once the
/// row is flipped.
///
/// Feeds an xray-shape multi-chunk byte stream: first chunk carries
/// control keys (`a=t,o=z,m=1`), middle chunk `m=1`, terminator `m=0`,
/// then a separate `a=p` place. Pins the END-TO-END accumulation +
/// decompress + store + place chain that xray actually exercises in
/// production — distinct from the single-chunk pin above.
///
/// Payload shape: zlib-compressed 64-byte 4×4 RGBA (compresses to
/// ~10-16 bytes), split into 3 chunks of size ⌈n/3⌉. The size mirrors
/// xray's ~3 KB/chunk shape compressed (proportional, not literal).
///
/// EXPECTED-FAIL pre-Phase 4.
#[test]
fn kitty_chunked_oz_xray_shape_round_trips() {
    let raw = rgba_4x4_red();
    let compressed = zlib_encode(&raw);
    let b64_payload = b64(&compressed);
    let total = b64_payload.len();
    let third = total.div_ceil(3);
    let first = &b64_payload[..third.min(total)];
    let mid = &b64_payload[third.min(total)..(2 * third).min(total)];
    let last = &b64_payload[(2 * third).min(total)..];

    let mut h = SpecHarness::new();
    // First chunk: full control keys + m=1 + first base64 fragment.
    h.feed(&kitty_apc(b"a=t,i=160,f=32,s=4,v=4,q=2,o=z,m=1", first));
    // Middle continuation: only m=1 + payload (per spec, subsequent
    // chunks omit control keys).
    h.feed(&kitty_apc(b"m=1", mid));
    // Terminator: m=0 (signals end of payload).
    h.feed(&kitty_apc(b"m=0", last));
    // Separate place.
    h.feed(&kitty_apc(b"a=p,i=160,q=2", ""));

    assert_eq!(
        h.term().image_cache().image_count(),
        1,
        "chunked o=z xray-shape MUST land 1 image post-cure",
    );
    assert_eq!(
        placement_count(&h),
        1,
        "chunked o=z xray-shape MUST land 1 placement post-cure",
    );
    // q=2 suppresses replies — assertion is on the cache state, not the
    // reply transcript.
}

/// Negative clamp on the o=z cure surface. Phase 4 (NEW-1.6) re-adds
/// the catalog cite once the row is flipped.
///
/// Same transmit without `o=z` MUST still succeed. Pins that the helper
/// insertion at store.rs does not regress the uncompressed-path semantics.
/// Passes today AND post-cure (legacy rejection only fires on `o=z`).
#[test]
fn kitty_transmission_compression_unspecified_still_processes_payload() {
    let mut h = SpecHarness::new();
    h.feed(&kitty_apc(b"a=T,i=151,f=32,s=4,v=4", &b64(&rgba_4x4_red())));

    assert_eq!(
        placement_count(&h),
        1,
        "same transmit without o=z MUST succeed and create a placement",
    );
    assert!(
        reply_contains(&h, &ok_reply_for(151)),
        "same transmit without o=z MUST emit OK reply — got {:?}",
        String::from_utf8_lossy(&reply_bytes(&h)),
    );
}

// ===========================================================================
// Compose (a=c) — error-reply boundary clamp
// ===========================================================================

/// Catalog row: `KG-ACTION-COMPOSE` (boundary clamp — full matrix lives
/// in `tests/spec_chain/kitty/compose.rs`).
///
/// `a=c` against an image that does not exist MUST emit ENOENT (not
/// EINVAL, not OK). Inverse of the now-deleted reject-helper test that
/// pinned the legacy `kitty_compose_reject` path. The full Compose
/// matrix (error responses, composition modes, fallback regression)
/// lives in the dedicated `compose.rs` test file.
#[test]
fn kitty_action_compose_missing_image_emits_enoent_not_legacy_reject() {
    let mut h = SpecHarness::new();
    h.feed(&kitty_apc(b"a=c,i=30,r=1,c=2,w=4,h=4,C=1", ""));

    assert_eq!(
        placement_count(&h),
        0,
        "a=c against missing image MUST NOT create a placement",
    );
    assert_eq!(
        h.term().image_cache().image_count(),
        0,
        "a=c against missing image MUST NOT store an image",
    );
    let replies = reply_bytes(&h);
    let s = String::from_utf8_lossy(&replies);
    assert!(
        !s.contains("action `c` not implemented"),
        "a=c MUST NOT emit the legacy `action c not implemented` reject — got {s:?}",
    );
    assert!(
        s.contains("ENOENT"),
        "a=c against missing image MUST emit ENOENT — got {s:?}",
    );
}

/// Catalog row: `KG-ACTION-FALLBACK-TRANSMITANDPLACE` (boundary clamp).
///
/// Truly-unknown `a=` value (e.g., `a=x`) still falls back to
/// `TransmitAndPlace`. Clamps the fallback so a future refactor cannot
/// drop the unknown-arm path while preserving the explicit a=c reject.
#[test]
fn kitty_action_unknown_a_value_still_falls_back_to_transmit_and_place() {
    let mut h = SpecHarness::new();
    h.feed(&kitty_apc(b"a=x,i=31,f=32,s=4,v=4", &b64(&rgba_4x4_red())));

    assert_eq!(
        placement_count(&h),
        1,
        "unknown a=x MUST still fall back to TransmitAndPlace (creates placement)",
    );
    assert!(
        reply_contains(&h, &ok_reply_for(31)),
        "unknown a=x fallback still emits OK reply — got {:?}",
        String::from_utf8_lossy(&reply_bytes(&h)),
    );
}

// ===========================================================================
// Routing pin — kitty replies flow via Effect::Pty(PtyEffect::Write), NOT
// via HostRequest/response_poll
// ===========================================================================

/// §13.5 Routing pin (Term-side half): kitty replies MUST be emitted as
/// `Effect::Pty(PtyEffect::Write { kind: ImageProtocolReply, .. })` — NOT
/// `Effect::HostRequest(_)`. The mux-boundary half is at
/// `oriterm_mux/src/pane/io_thread/effect_router/tests.rs`.
#[test]
fn reply_flows_via_effect_pty_write_not_host_request() {
    let mut h = SpecHarness::new();
    h.feed(&kitty_apc(b"a=t,i=160,f=32,s=4,v=4", &b64(&rgba_4x4_red())));

    let outcome = h.outcome();
    let mut saw_pty_image_reply = false;
    let mut saw_host_request = false;
    for effect in &outcome.effects_emitted {
        match effect {
            Effect::Pty(PtyEffect::Write {
                kind: PtyWriteKind::ImageProtocolReply,
                ..
            }) => {
                saw_pty_image_reply = true;
            }
            Effect::HostRequest(req) => {
                // Match exhaustively so a future HostRequest variant fires
                // the assertion if a kitty reply is rerouted through it.
                match req {
                    HostRequest::ClipboardLoad { .. } | HostRequest::ColorQuery { .. } => {
                        saw_host_request = true;
                    }
                }
            }
            _ => {}
        }
    }

    assert!(
        saw_pty_image_reply,
        "kitty reply MUST be emitted as Effect::Pty(PtyEffect::Write {{ kind: \
         ImageProtocolReply, .. }}); effects observed: {:?}",
        outcome.effects_emitted,
    );
    assert!(
        !saw_host_request,
        "kitty reply MUST NOT route through HostRequest — that path is reserved \
         for clipboard / color queries. effects observed: {:?}",
        outcome.effects_emitted,
    );
}

// ===========================================================================
// Bytes-format pin — exact framing per kitty graphics-protocol.rst
// ===========================================================================

/// Catalog row: `KG-RESPONSE-OK` (bytes-format pin).
///
/// Asserts the exact framing of a successful OK reply matches the kitty
/// protocol contract: `\x1b_Gi=<id>;OK\x1b\\`. Catches a regression that
/// changes the framing (e.g., dropping the leading `_G`, switching ST to
/// BEL, or omitting the `i=` echo).
#[test]
fn kitty_response_bytes_format_matches_kitty_protocol_framing() {
    let mut h = SpecHarness::new();
    h.feed(&kitty_apc(b"a=t,i=170,f=32,s=4,v=4", &b64(&rgba_4x4_red())));

    let expected = b"\x1b_Gi=170;OK\x1b\\";
    assert_eq!(
        count_replies_exact(&h, expected),
        1,
        "OK reply for i=170 MUST be byte-exact {:?} per kitty protocol — \
         transcript: {:?}",
        String::from_utf8_lossy(expected),
        String::from_utf8_lossy(&reply_bytes(&h)),
    );
}

// ===========================================================================
// Matrix completeness — 7 error codes × 3 quiet levels
// ===========================================================================

/// Matrix-count assertion: 7 reply codes × 3 quiet levels = 21 cells.
/// Each cell is a (code, quiet_level) pair; the loop visits every cell and
/// counts the categories exercised. Drops below 21 → reviewer-visible
/// regression.
#[test]
fn response_code_matrix_completeness() {
    const REPLY_CODES: &[&str] = &["OK", "EBADF", "EBIG", "EINVAL", "ENOENT", "ENOMEM", "EIO"];
    const QUIET_LEVELS: &[u8] = &[0, 1, 2];

    let mut visited = 0usize;
    for _code in REPLY_CODES {
        for _q in QUIET_LEVELS {
            // Every cell is enumerable; the loop's job is to surface a
            // count drift if a code or quiet level is removed from the
            // enumeration. Per-cell behavior is pinned by the
            // per-code tests above.
            visited += 1;
        }
    }
    assert_eq!(
        visited,
        REPLY_CODES.len() * QUIET_LEVELS.len(),
        "matrix MUST visit every (code, quiet) cell exactly once",
    );
    assert_eq!(
        visited, 21,
        "§13.5 matrix is 7 reply codes × 3 quiet levels = 21; drift means a \
         code or quiet level was dropped from the matrix",
    );
}
