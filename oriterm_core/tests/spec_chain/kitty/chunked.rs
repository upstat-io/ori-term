//! Per-chunk rung (§13.2) — drives `m=1` / `m=0` chunked transmission through
//! `kitty_accumulate_chunk` + `kitty_finalize_payload` and the malformed-base64
//! reply path at `handle_kitty_graphics`.
//!
//! Catalog rows: `KG-TRANSMIT-CHUNKED-COALESCE` (new — chunked coalesce),
//! `KG-TRANSMIT-CHUNKED-SIZE-LIMIT` (new — oversize discard without reply),
//! `KG-TRANSMIT-CHUNKED-MALFORMED-BASE64` (new — EINVAL reply on InvalidBase64
//! via §13.2 Option A). Strengthens `KG-RESPONSE-EINVAL` by pinning the
//! base64-decode arm that was uncited by §13.1.

use oriterm_core::effect::{Effect, PtyEffect, PtyWriteKind};
use oriterm_test_support::spec_chain::SpecHarness;

use super::fixtures::{
    b64, count_replies_exact, kitty_apc, ok_reply_for, placement_count, reply_bytes,
    reply_contains, rgba_4x4_red,
};

/// Build the EINVAL base64 reply bytes emitted by §13.2's Option A wiring.
/// Mirrors the 3-arm reply framing in `kitty_respond`: echoes `i=<id>`
/// when image_id != 0, `i=<id>,I=<num>` when both are present, or
/// `I=<num>` when only image_number is present; falls back to the `i=0`
/// sentinel when neither identifier is provided (ori_term deviation
/// pinned by `kitty_malformed_base64_without_i_key_falls_back_to_i0_sentinel`).
fn einval_base64_reply(image_id: u32, image_number: Option<u32>) -> Vec<u8> {
    let head = match (image_id, image_number) {
        (0, Some(n)) => format!("I={n}"),
        (id, Some(n)) => format!("i={id},I={n}"),
        (id, None) => format!("i={id}"),
    };
    format!("\x1b_G{head};EINVAL: base64 decode failed\x1b\\").into_bytes()
}

// Chunked coalesce + decode

/// Catalog row: `KG-TRANSMIT-CHUNKED-COALESCE` (m=1 → m=1 → m=0 coalesces).
///
/// Split a 64-byte RGBA payload across 3 APC commands at 4-char base64
/// boundaries so per-chunk `decode_base64` is lossless. Final `m=0` chunk
/// triggers `kitty_finalize_payload` which concatenates the accumulated
/// decoded bytes. Assert one placement + OK reply for i=50 — proves the
/// coalesce path through `kitty_accumulate_chunk` → `kitty_finalize_payload`
/// → `kitty_store_image` with an intact 64-byte RGBA buffer.
#[test]
fn kitty_chunked_transmit_m1_m0_coalesces_into_single_placement() {
    let encoded = b64(&rgba_4x4_red());
    // 64 bytes → 88 chars with `=` padding. Split at 4-char boundaries
    // (40 + 40 + 8) so each chunk's `decode_base64` emits a whole-byte
    // prefix; otherwise straddled sextets are silently truncated.
    let (first, rest) = encoded.split_at(40);
    let (middle, last) = rest.split_at(40);

    let mut h = SpecHarness::new();
    h.feed(&kitty_apc(b"a=T,i=50,f=32,s=4,v=4,m=1", first));
    h.feed(&kitty_apc(b"a=T,i=50,m=1", middle));
    h.feed(&kitty_apc(b"a=T,i=50,m=0", last));

    assert_eq!(
        placement_count(&h),
        1,
        "chunked a=T,i=50 MUST coalesce into one placement — transcript: {:?}",
        String::from_utf8_lossy(&reply_bytes(&h)),
    );
    assert!(
        reply_contains(&h, &ok_reply_for(50)),
        "chunked a=T,i=50 success MUST emit OK reply for i=50 — got {:?}",
        String::from_utf8_lossy(&reply_bytes(&h)),
    );
}

/// Catalog row: `KG-TRANSMIT-CHUNKED-COALESCE` (arrival-order pin — byte-level).
///
/// kitty graphics-protocol.rst is silent on per-chunk ordering; ori_term's
/// `kitty_accumulate_chunk` at `transmit.rs` appends in arrival order. This
/// test pins that contract at the BYTE LEVEL by transmitting a 4×1 RGBA
/// image whose four pixels are four distinct colors (red / green / blue /
/// yellow, 16 bytes total), split across two chunks at a 4-char base64
/// boundary (chunk 1 carries the first 6 bytes of decoded payload, chunk 2
/// the remaining 10). After the chunked transmit lands, the test reads the
/// decoded pixel buffer from `RenderableContent::image_data` and asserts
/// the bytes match the original payload EXACTLY — a length-preserving byte
/// reorder (swap, reverse, per-chunk shuffle) would pass the weaker
/// placement-only check but fails this byte-equality clamp per
/// §Matrix Clamping.
#[test]
fn kitty_chunked_arrival_order_pin_preserves_append_sequence() {
    // 4×1 RGBA, 4 distinct colors so any byte reorder is visible.
    let payload: Vec<u8> = vec![
        255, 0, 0, 255, // red
        0, 255, 0, 255, // green
        0, 0, 255, 255, // blue
        255, 255, 0, 255, // yellow
    ];
    assert_eq!(payload.len(), 16, "4×1 RGBA = 16 bytes");
    let encoded = b64(&payload);
    // 16 bytes → 24 base64 chars (with `=` padding or 22 without + the
    // trailing `=`). Split at a 4-char boundary (8) so each chunk's
    // per-chunk base64 decoder emits whole bytes — mid-sextet splits
    // silently truncate and would mask a real byte-order bug.
    let (first, last) = encoded.split_at(8);

    let mut h = SpecHarness::new();
    h.feed(&kitty_apc(b"a=t,i=51,f=32,s=4,v=1,m=1", first));
    h.feed(&kitty_apc(b"a=t,i=51,m=0", last));

    assert!(
        reply_contains(&h, &ok_reply_for(51)),
        "chunked a=t,i=51 MUST emit OK for the transmit — got {:?}",
        String::from_utf8_lossy(&reply_bytes(&h)),
    );

    // Create a placement so the image surfaces in the RenderableContent
    // snapshot (image_data is populated from visible placements, not bare
    // cache entries).
    h.feed(&kitty_apc(b"a=p,i=51", ""));
    assert_eq!(
        placement_count(&h),
        1,
        "a=p,i=51 MUST create a placement — proves store path ran; \
         transcript: {:?}",
        String::from_utf8_lossy(&reply_bytes(&h)),
    );

    // BYTE-LEVEL ARRIVAL-ORDER CLAMP — query the decoded RGBA buffer and
    // assert the coalesced bytes equal the pre-chunking payload exactly.
    // A reverse, swap, or per-chunk shuffle yields a length-preserving
    // mismatch that ONLY this check catches.
    let snapshot = h.term().renderable_content();
    let image = snapshot
        .image_data
        .iter()
        .find(|img| img.id.as_u32() == 51)
        .unwrap_or_else(|| {
            panic!(
                "image id=51 MUST be in RenderableContent::image_data \
                 after chunked transmit + place"
            )
        });
    assert_eq!(image.width, 4, "width matches s=4");
    assert_eq!(image.height, 1, "height matches v=1");
    assert_eq!(
        image.data.as_ref(),
        &payload,
        "chunked coalesce MUST preserve byte order — expected {:?}, got {:?}",
        &payload,
        &image.data[..],
    );
}

/// Catalog row: `KG-TRANSMIT-FORMAT-32` (chunked negative — size mismatch).
///
/// Regression guard for `kitty_chunked_transmit_m1_m0_coalesces_into_single_placement`
/// if coalesce drops bytes or the store path skips size validation, this
/// test would pass silently. Send a chunked RGBA payload that coalesces to
/// only 32 bytes (half of s=4,v=4 → 64 expected) and assert the
/// `EINVAL: RGBA payload size` reply fires with the mismatched actual size.
#[test]
fn kitty_chunked_coalesced_undersize_emits_einval_size_reply() {
    // Only 32 bytes worth of red pixels — half of what s=4,v=4,f=32 expects.
    let half_payload: Vec<u8> = std::iter::repeat_n([255u8, 0, 0, 255], 8)
        .flatten()
        .collect();
    let encoded = b64(&half_payload);
    let (first, last) = encoded.split_at(20);

    let mut h = SpecHarness::new();
    h.feed(&kitty_apc(b"a=T,i=52,f=32,s=4,v=4,m=1", first));
    h.feed(&kitty_apc(b"a=T,i=52,m=0", last));

    assert_eq!(
        placement_count(&h),
        0,
        "undersize chunked transmit MUST NOT create a placement",
    );
    let replies = reply_bytes(&h);
    let s = String::from_utf8_lossy(&replies);
    assert!(
        s.contains("EINVAL: RGBA payload size"),
        "coalesced 32-byte payload vs 64-byte expected MUST emit \
         EINVAL size reply — got {s:?}",
    );
}

// Chunked size-limit rejection (no reply emitted on this path)

/// Catalog row: `KG-TRANSMIT-CHUNKED-SIZE-LIMIT` (oversize accumulation).
///
/// Exceeding `max_single_image_bytes` during accumulation drops
/// `loading_image` at `transmit.rs` with a `warn!`. No reply is emitted on
/// this path (current code; §13.0 audit decision). The silent discard is
/// intentional — the spec is silent on chunked-size limits, and the
/// discarded transmission never reached a finalize boundary where a reply
/// could carry an image id.
#[test]
fn kitty_chunked_oversize_accumulation_discards_loading_state_without_reply() {
    let mut h = SpecHarness::new();
    // Shrink the per-image cap so we can trigger the discard with tiny
    // payloads; default is 64 MiB. 128 B keeps the test payload small while
    // still being large enough for multiple 64-byte chunks to accumulate.
    h.term_mut().set_image_limits(usize::MAX, 128);

    let chunk_payload = b64(&rgba_4x4_red()); // 64 B decoded × 3 = 192 B > 128 B.

    h.feed(&kitty_apc(b"a=T,i=60,f=32,s=8,v=8,m=1", &chunk_payload));
    h.feed(&kitty_apc(b"a=T,i=60,m=1", &chunk_payload));
    h.feed(&kitty_apc(b"a=T,i=60,m=1", &chunk_payload));

    // Discard path emits NO reply. No OK, no EINVAL, no EBIG.
    assert!(
        reply_bytes(&h).is_empty(),
        "oversize chunked accumulation MUST NOT emit any reply — \
         transcript: {:?}",
        String::from_utf8_lossy(&reply_bytes(&h)),
    );

    // Follow-up a=p,i=60 MUST fail with ENOENT — proves loading_image was
    // discarded and nothing landed in the cache.
    h.feed(&kitty_apc(b"a=p,i=60", ""));
    let replies = reply_bytes(&h);
    let s = String::from_utf8_lossy(&replies);
    assert!(
        s.contains("\x1b_Gi=60;ENOENT"),
        "a=p,i=60 after oversize discard MUST emit ENOENT — got {s:?}",
    );
}

/// Catalog row: `KG-TRANSMIT-CHUNKED-SIZE-LIMIT` (regression guard).
///
/// Pairs with `kitty_chunked_oversize_accumulation_discards_loading_state_without_reply`:
/// if the discard code path ever starts emitting an error reply (drift
/// toward `EBIG` or `ENOMEM`), this test fails. Also clamps the
/// `verified-with-deviation` status on the size-limit catalog row —
/// changing the status to plain `verified` would require the spec to
/// define a reply, which it does not.
#[test]
fn kitty_chunked_oversize_negative_pin_does_not_emit_einval_reply() {
    let mut h = SpecHarness::new();
    h.term_mut().set_image_limits(usize::MAX, 128);

    let chunk_payload = b64(&rgba_4x4_red());
    h.feed(&kitty_apc(b"a=T,i=61,f=32,s=8,v=8,m=1", &chunk_payload));
    h.feed(&kitty_apc(b"a=T,i=61,m=1", &chunk_payload));
    h.feed(&kitty_apc(b"a=T,i=61,m=1", &chunk_payload));

    let replies = reply_bytes(&h);
    let s = String::from_utf8_lossy(&replies);
    assert!(
        !s.contains("EINVAL"),
        "oversize chunked discard MUST NOT emit EINVAL — got {s:?}",
    );
    assert!(
        !s.contains("EBIG"),
        "oversize chunked discard MUST NOT emit EBIG — got {s:?}",
    );
    assert!(
        !s.contains("ENOMEM"),
        "oversize chunked discard MUST NOT emit ENOMEM — got {s:?}",
    );
}

// Malformed-base64 reply path (§13.2 Option A implementation)

/// Catalog row: `KG-TRANSMIT-CHUNKED-MALFORMED-BASE64` (property — Option A).
///
/// A malformed base64 payload (containing `@`, which is outside the base64
/// alphabet) causes `parse_kitty_command_into` to return
/// `Err(KittyError::InvalidBase64)` via `decode_base64` at `parse.rs:297`.
/// `parse_kitty_command_into` parses control data BEFORE the base64 decode
/// runs, so `cmd.image_id` is populated when the error fires. §13.2 Option A
/// wires `handle_kitty_graphics` to emit
/// `\x1b_Gi=<parsed_i>;EINVAL: base64 decode failed\x1b\\`, matching
/// kitty's `finish_command_response` convention for echoing the sender's id.
#[test]
fn kitty_malformed_base64_emits_einval_reply_echoing_parsed_i() {
    let mut h = SpecHarness::new();
    h.feed(&kitty_apc(b"a=T,i=70,f=32,s=4,v=4", "not@valid@base64"));

    let replies = reply_bytes(&h);
    let s = String::from_utf8_lossy(&replies);
    assert!(
        s.contains("EINVAL: base64 decode failed"),
        "malformed base64 MUST emit EINVAL reply — got {s:?}",
    );
    assert_eq!(
        count_replies_exact(&h, &einval_base64_reply(70, None)),
        1,
        "exactly one EINVAL base64 reply echoing i=70 MUST be emitted — \
         transcript: {:?}",
        String::from_utf8_lossy(&reply_bytes(&h)),
    );
    assert_eq!(
        count_replies_exact(&h, &einval_base64_reply(0, None)),
        0,
        "reply MUST NOT fall back to i=0 when the sender provided i=70 — \
         transcript: {:?}",
        String::from_utf8_lossy(&reply_bytes(&h)),
    );
}

/// Catalog row: `KG-TRANSMIT-CHUNKED-MALFORMED-BASE64` (i=0 sentinel fallback).
///
/// When the sender omits both `i=` and `I=`, ori_term synthesizes the `i=0`
/// sentinel so the client still gets a correlatable EINVAL they can match
/// against a recently-sent command. This is a documented deviation from
/// kitty's `finish_command_response` (which suppresses the reply); the
/// deviation is pinned by the catalog row as `verified-with-deviation`
/// and by the `kitty_action_query_without_image_id_emits_ok_for_id_zero`
/// dispatch-identity clamp in `actions.rs`.
#[test]
fn kitty_malformed_base64_without_i_key_falls_back_to_i0_sentinel() {
    let mut h = SpecHarness::new();
    // Omit both i= and I= so the i=0 fallback fires.
    h.feed(&kitty_apc(b"a=T,f=32,s=4,v=4", "not@valid"));

    assert_eq!(
        count_replies_exact(&h, &einval_base64_reply(0, None)),
        1,
        "missing i= AND I= MUST emit EINVAL reply with i=0 sentinel \
         (ori_term deviation from kitty's suppress behavior; preserves \
         correlatable recovery signal) — transcript: {:?}",
        String::from_utf8_lossy(&reply_bytes(&h)),
    );
}

/// Catalog row: `KG-TRANSMIT-CHUNKED-MALFORMED-BASE64` (q= quiet gate).
///
/// `kitty_respond` honors the parsed `quiet` level: q=2 suppresses every
/// reply including errors; q=1 suppresses only OK. Since the EINVAL reply
/// goes through `kitty_respond`, q=2 on a malformed base64 payload MUST
/// suppress the reply entirely — a regression where the EINVAL path
/// bypasses `kitty_respond` and emits the bytes directly would trip this.
#[test]
fn kitty_malformed_base64_with_quiet_2_suppresses_einval_reply() {
    let mut h = SpecHarness::new();
    h.feed(&kitty_apc(b"a=T,i=72,q=2,f=32,s=4,v=4", "bad@base64"));

    assert_eq!(
        count_replies_exact(&h, &einval_base64_reply(72, None)),
        0,
        "q=2 MUST suppress the EINVAL base64 reply — transcript: {:?}",
        String::from_utf8_lossy(&reply_bytes(&h)),
    );
    assert!(
        reply_bytes(&h).is_empty(),
        "q=2 on malformed base64 MUST produce zero replies — transcript: {:?}",
        String::from_utf8_lossy(&reply_bytes(&h)),
    );
}

/// Catalog row: `KG-TRANSMIT-CHUNKED-MALFORMED-BASE64` (regression guard — no storage).
///
/// Parse failure must not leave any side effects in the image cache. A
/// follow-up `a=p,i=70` must emit ENOENT because the image was never
/// stored. If the malformed path ever started a store attempt before
/// bailing, a placement or a zero-byte image could land silently.
#[test]
fn kitty_malformed_base64_does_not_create_placement_or_image() {
    let mut h = SpecHarness::new();
    h.feed(&kitty_apc(b"a=T,i=71,f=32,s=4,v=4", "@@@@"));

    assert_eq!(
        placement_count(&h),
        0,
        "malformed base64 MUST NOT create a placement",
    );

    h.feed(&kitty_apc(b"a=p,i=71", ""));
    let replies = reply_bytes(&h);
    let s = String::from_utf8_lossy(&replies);
    assert!(
        s.contains("\x1b_Gi=71;ENOENT"),
        "a=p,i=71 after malformed base64 MUST emit ENOENT — proves \
         no image landed in the cache; got {s:?}",
    );
}

/// Catalog row: `KG-TRANSMIT-CHUNKED-MALFORMED-BASE64` (regression guard — valid payload).
///
/// Counterpart to `kitty_malformed_base64_emits_einval_reply`: a valid
/// base64 payload MUST NOT emit the EINVAL base64 reply. Fails if the
/// Option A wiring is ever broadened to fire unconditionally or the match
/// arm captures a non-`InvalidBase64` error variant.
#[test]
fn kitty_valid_base64_does_not_emit_einval_base64_reply() {
    let mut h = SpecHarness::new();
    h.feed(&kitty_apc(b"a=T,i=72,f=32,s=4,v=4", &b64(&rgba_4x4_red())));

    let replies = reply_bytes(&h);
    let s = String::from_utf8_lossy(&replies);
    assert!(
        s.contains(";OK"),
        "valid base64 payload MUST emit OK reply — got {s:?}",
    );
    assert!(
        !s.contains("EINVAL: base64 decode failed"),
        "valid base64 payload MUST NOT emit EINVAL base64 reply — got {s:?}",
    );
    assert_eq!(
        count_replies_exact(&h, &einval_base64_reply(72, None)),
        0,
        "valid payload emits zero EINVAL base64 replies for the sender's i=72",
    );
    assert_eq!(
        count_replies_exact(&h, &einval_base64_reply(0, None)),
        0,
        "valid payload emits zero EINVAL base64 replies for the i=0 fallback",
    );
}

// Chunked control-key inheritance (first-chunk semantics survive to finalize)

/// Catalog row: `KG-TRANSMIT-CHUNKED-COALESCE` (control-key inheritance — `q=`).
///
/// Kitty's chunked protocol says all control keys are authoritative on the
/// FIRST chunk; later chunks carry only `m=` + payload bytes. `q=1` on the
/// first chunk MUST suppress the final OK reply even though the m=0 tail
/// chunk omits `q=`. Pre-fix: `kitty_transmit_and_place` read `cmd.quiet`
/// from the m=0 tail (default 0) and emitted OK unexpectedly.
#[test]
fn kitty_chunked_quiet_on_first_chunk_suppresses_ok_reply_on_terminal_chunk() {
    let encoded = b64(&rgba_4x4_red());
    let (first, last) = encoded.split_at(44);

    let mut h = SpecHarness::new();
    // q=1 on the start chunk — suppresses OK, does NOT suppress errors.
    h.feed(&kitty_apc(b"a=T,i=90,q=1,f=32,s=4,v=4,m=1", first));
    h.feed(&kitty_apc(b"a=T,i=90,m=0", last));

    // The store succeeded (no EINVAL in transcript), so the only reply
    // that COULD fire is the OK. q=1 on the first chunk MUST suppress it.
    let replies = reply_bytes(&h);
    let s = String::from_utf8_lossy(&replies);
    assert!(
        !s.contains(";OK"),
        "first-chunk q=1 MUST suppress the successful-transmit OK reply \
         even though the m=0 tail omits q= — transcript: {s:?}",
    );
    // The placement DID happen — q=1 only silences replies, not state.
    assert_eq!(
        placement_count(&h),
        1,
        "q=1 only silences replies; placement MUST still land",
    );
}

/// Catalog row: `KG-TRANSMIT-CHUNKED-COALESCE` (control-key inheritance — `U=`).
///
/// U=1 on the first chunk defers placement to unicode-placeholder cells
/// (§13.4). The m=0 tail omits `U=`, so a pre-fix read of `cmd.unicode_placeholder`
/// from the tail yielded `false` and created a regular placement — bypassing
/// the U=1 contract. Post-fix: `kitty_finalize_payload` returns a merged
/// cmd with `unicode_placeholder: true` from the start chunk.
#[test]
fn kitty_chunked_unicode_placeholder_on_first_chunk_suppresses_placement() {
    let encoded = b64(&rgba_4x4_red());
    let (first, last) = encoded.split_at(44);

    let mut h = SpecHarness::new();
    h.feed(&kitty_apc(b"a=T,i=91,U=1,f=32,s=4,v=4,m=1", first));
    h.feed(&kitty_apc(b"a=T,i=91,m=0", last));

    assert_eq!(
        placement_count(&h),
        0,
        "U=1 on the start chunk MUST suppress placement creation even \
         though the m=0 tail omits U= — pre-fix: placement leaked",
    );
    // OK reply still fires because q= is default.
    assert!(
        reply_contains(&h, &ok_reply_for(91)),
        "chunked a=T,i=91 with U=1 MUST still emit OK for the transmit; \
         transcript: {:?}",
        String::from_utf8_lossy(&reply_bytes(&h)),
    );
}

// I= echo in malformed-base64 reply (§13.2 Option A + response-format)

/// Catalog row: `KG-TRANSMIT-CHUNKED-MALFORMED-BASE64` + `KG-RESPONSE-EINVAL`
/// (I= echo).
///
/// Per kitty's `finish_command_response`, replies echo `I=` when the
/// sender provided `I=` but omitted `i=`. ori_term's `kitty_respond` now
/// emits `\x1b_GI=<num>;EINVAL: base64 decode failed\x1b\\` in this case
/// (pre-fix: replied with `i=0` and lost the `I=` echo entirely).
#[test]
fn kitty_malformed_base64_echoes_image_number_when_i_absent() {
    let mut h = SpecHarness::new();
    h.feed(&kitty_apc(b"a=T,I=42,f=32,s=4,v=4", "bad@base64"));

    assert_eq!(
        count_replies_exact(&h, &einval_base64_reply(0, Some(42))),
        1,
        "malformed base64 with I=42 (no i=) MUST emit exactly one \
         `\\x1b_GI=42;EINVAL: base64 decode failed\\x1b\\\\` reply — \
         transcript: {:?}",
        String::from_utf8_lossy(&reply_bytes(&h)),
    );
    // Negative: MUST NOT fall back to i=0 sentinel when I= is provided.
    assert_eq!(
        count_replies_exact(&h, &einval_base64_reply(0, None)),
        0,
        "reply MUST NOT fabricate i=0 sentinel when I= was provided — \
         transcript: {:?}",
        String::from_utf8_lossy(&reply_bytes(&h)),
    );
}

/// Catalog row: `KG-TRANSMIT-CHUNKED-MALFORMED-BASE64` + `KG-RESPONSE-EINVAL`
/// (both-i-and-I echo).
///
/// When the sender provides both `i=` and `I=`, kitty's response echoes
/// both. ori_term's `kitty_respond` emits `i=<id>,I=<num>` under those
/// conditions — pre-fix: only `i=` was echoed, `I=` was silently dropped.
#[test]
fn kitty_malformed_base64_echoes_both_i_and_image_number_when_present() {
    let mut h = SpecHarness::new();
    h.feed(&kitty_apc(b"a=T,i=5,I=42,f=32,s=4,v=4", "bad@base64"));

    assert_eq!(
        count_replies_exact(&h, &einval_base64_reply(5, Some(42))),
        1,
        "malformed base64 with both i=5 and I=42 MUST emit exactly one \
         `\\x1b_Gi=5,I=42;EINVAL: base64 decode failed\\x1b\\\\` reply — \
         transcript: {:?}",
        String::from_utf8_lossy(&reply_bytes(&h)),
    );
}

// Matrix completeness

/// Assert the §13.2 matrix actually exercises all four categories the
/// plan enumerates (coalesce, arrival-order, size-limit, malformed-base64)
/// by driving a canonical input for each category and counting the number
/// of categories whose invariant fires. Unlike a bare `assert_eq!(N, 4)`
/// array-length check, this walks real category probes through
/// `SpecHarness` — deleting a category from the module below also drops
/// this category's probe from the matrix, breaking the count.
///
/// Per §Matrix Testing Rule + §Matrix Clamping
/// the self-verifying matrix must prove cells were exercised, not just
/// name them.
#[test]
fn kitty_chunked_category_matrix_completeness() {
    let mut categories_exercised = 0usize;

    // Category 1 — coalesce: chunked m=1→m=0 produces an OK reply.
    {
        let encoded = b64(&rgba_4x4_red());
        let (first, rest) = encoded.split_at(40);
        let (middle, last) = rest.split_at(40);
        let mut h = SpecHarness::new();
        h.feed(&kitty_apc(b"a=T,i=80,f=32,s=4,v=4,m=1", first));
        h.feed(&kitty_apc(b"a=T,i=80,m=1", middle));
        h.feed(&kitty_apc(b"a=T,i=80,m=0", last));
        if reply_contains(&h, &ok_reply_for(80)) {
            categories_exercised += 1;
        }
    }

    // Category 2 — arrival-order: byte-equal decoded buffer after chunked
    // transmit of a distinct-pattern payload (4×1 RGBA, 4 colors, split
    // at a 4-char boundary).
    {
        let payload: Vec<u8> = vec![
            255, 0, 0, 255, // red
            0, 255, 0, 255, // green
            0, 0, 255, 255, // blue
            255, 255, 0, 255, // yellow
        ];
        let encoded = b64(&payload);
        let (first, last) = encoded.split_at(8);
        let mut h = SpecHarness::new();
        h.feed(&kitty_apc(b"a=t,i=81,f=32,s=4,v=1,m=1", first));
        h.feed(&kitty_apc(b"a=t,i=81,m=0", last));
        h.feed(&kitty_apc(b"a=p,i=81", ""));
        let snapshot = h.term().renderable_content();
        if let Some(image) = snapshot.image_data.iter().find(|img| img.id.as_u32() == 81)
            && image.data.as_ref() == &payload
        {
            categories_exercised += 1;
        }
    }

    // Category 3 — size-limit: over-cap accumulation discards without reply.
    {
        let mut h = SpecHarness::new();
        h.term_mut().set_image_limits(usize::MAX, 128);
        let chunk_payload = b64(&rgba_4x4_red());
        h.feed(&kitty_apc(b"a=T,i=82,f=32,s=8,v=8,m=1", &chunk_payload));
        h.feed(&kitty_apc(b"a=T,i=82,m=1", &chunk_payload));
        h.feed(&kitty_apc(b"a=T,i=82,m=1", &chunk_payload));
        if reply_bytes(&h).is_empty() {
            categories_exercised += 1;
        }
    }

    // Category 4 — malformed-base64: invalid byte triggers EINVAL reply.
    {
        let mut h = SpecHarness::new();
        h.feed(&kitty_apc(b"a=T,i=83,f=32,s=4,v=4", "not@valid@base64"));
        if reply_contains(&h, b"EINVAL: base64 decode failed") {
            categories_exercised += 1;
        }
    }

    // Category 5 — chunked control-key inheritance (q=): first-chunk q=1
    // suppresses the terminal OK reply even when the tail chunk omits q=.
    {
        let encoded = b64(&rgba_4x4_red());
        let (first, last) = encoded.split_at(44);
        let mut h = SpecHarness::new();
        h.feed(&kitty_apc(b"a=T,i=84,q=1,f=32,s=4,v=4,m=1", first));
        h.feed(&kitty_apc(b"a=T,i=84,m=0", last));
        let replies = reply_bytes(&h);
        let s = String::from_utf8_lossy(&replies);
        if !s.contains(";OK") && placement_count(&h) == 1 {
            categories_exercised += 1;
        }
    }

    // Category 6 — chunked control-key inheritance (U=): first-chunk U=1
    // suppresses placement even when the tail chunk omits U=.
    {
        let encoded = b64(&rgba_4x4_red());
        let (first, last) = encoded.split_at(44);
        let mut h = SpecHarness::new();
        h.feed(&kitty_apc(b"a=T,i=85,U=1,f=32,s=4,v=4,m=1", first));
        h.feed(&kitty_apc(b"a=T,i=85,m=0", last));
        if placement_count(&h) == 0 && reply_contains(&h, &ok_reply_for(85)) {
            categories_exercised += 1;
        }
    }

    assert_eq!(
        categories_exercised, 6,
        "§13.2 enumerates 6 categories (coalesce, arrival-order, \
         size-limit, malformed-base64, chunked-q-inheritance, \
         chunked-U-inheritance) — each MUST fire its canonical \
         invariant. Drop any category's tests and this count falls short.",
    );
}

// §13.5 EINVAL-flood coalescing — per-upload state machine
// =============================================================================

/// Catalog row: `KG-TRANSMIT-CHUNKED-MALFORMED-BASE64` (EINVAL flood pin).
///
/// When a chunked upload sequence (`m=1...m=0`) contains a malformed chunk,
/// the EINVAL reply MUST fire ONCE per failed upload — not once per chunk.
/// Pre-§13.5 the malformed-base64 reply path at `handle_kitty_graphics` fires
/// the reply on parse-error before any state-machine checks, so a flood of
/// malformed chunks for the same upload would produce one EINVAL per chunk
/// (per-chunk amplification). The fix tracks per-upload failure state on
/// `LoadingImage` so subsequent malformed chunks of an already-failed upload
/// are dropped silently.
///
/// This test feeds 5 chunked transmits where chunk 2 is malformed and
/// asserts `count_replies_exact(&h, EINVAL) == 1` (per-upload coalesce),
/// not 4 (per-chunk amplification).
#[test]
fn kitty_chunked_malformed_base64_emits_exactly_one_einval_reply_per_failed_upload() {
    let mut h = SpecHarness::new();

    // Chunk 1: valid base64 first-chunk control keys (i=200).
    let encoded = b64(&rgba_4x4_red());
    let (first, _rest) = encoded.split_at(40);
    h.feed(&kitty_apc(b"a=T,i=200,f=32,s=4,v=4,m=1", first));

    // Chunk 2: MALFORMED base64 mid-upload. EINVAL fires for this chunk.
    h.feed(&kitty_apc(b"a=T,i=200,m=1", "not@valid@base64"));

    // Chunks 3-5: malformed base64 continuations. Pre-§13.5 each chunk
    // emits its own EINVAL because parse-error reply fires before
    // upload-state coalesce.
    h.feed(&kitty_apc(b"a=T,i=200,m=1", "@@@@"));
    h.feed(&kitty_apc(b"a=T,i=200,m=1", "###"));
    h.feed(&kitty_apc(b"a=T,i=200,m=0", "!!!"));

    let einval = b"EINVAL: base64 decode failed";
    let count = h
        .outcome()
        .effects_emitted
        .iter()
        .filter(|e| {
            if let Effect::Pty(PtyEffect::Write {
                bytes,
                kind: PtyWriteKind::ImageProtocolReply,
            }) = e
            {
                bytes.windows(einval.len()).any(|w| w == einval)
            } else {
                false
            }
        })
        .count();

    assert_eq!(
        count,
        1,
        "chunked malformed-base64 MUST emit exactly ONE EINVAL reply per \
         failed upload — got {count}. transcript: {:?}",
        String::from_utf8_lossy(&reply_bytes(&h)),
    );
}
