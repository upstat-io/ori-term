//! Catalog row `KG-ACTION-COMPOSE` matrix — drives `compose_frame` through
//! `SpecHarness` and asserts reply codes + byte-exact dest-frame bytes
//! against the `graphics.c:1819 handle_compose_command` reference.

use oriterm_core::image::ImageId;
use oriterm_test_support::spec_chain::SpecHarness;

use super::fixtures::{
    assert_einval_reply, assert_frame_eq, b64, count_replies_exact, kitty_apc, ok_reply_for,
    reply_bytes, reply_contains, rgba_solid,
};

// ----------------------------------------------------------------------------
// Setup helpers — distinct-color two-frame image.
// ----------------------------------------------------------------------------

/// Build a kitty image (id=`id`) with a 4×4 RED root frame + a 4×4 GREEN
/// appended frame. Returns the `SpecHarness` ready for compose tests.
///
/// Setup commands carry `q=2` (suppress all replies) so the OK-reply
/// transcript ONLY contains replies from the compose under test — counting
/// OK replies via `count_replies_exact` distinguishes "compose emitted OK"
/// from "setup OK still in transcript" (closes the ghost-test gap where
/// `reply_contains(&h, &ok_reply_for(id))` matched setup OKs).
///
/// After this returns: frame 1 = pure red (opaque), frame 2 = pure green (opaque).
fn setup_2frame_red_green(id: u32) -> SpecHarness {
    let red = rgba_solid(4, 4, 255, 0, 0, 255);
    let green = rgba_solid(4, 4, 0, 255, 0, 255);

    let mut h = SpecHarness::new();
    h.feed(&kitty_apc(
        format!("a=t,i={id},f=32,s=4,v=4,q=2").as_bytes(),
        &b64(&red),
    ));
    h.feed(&kitty_apc(
        format!("a=f,i={id},f=32,s=4,v=4,q=2").as_bytes(),
        &b64(&green),
    ));
    h
}

/// Build a kitty image with semi-transparent RED source (frame 1) and
/// opaque BLUE dest (frame 2). Lets composition-mode tests distinguish
/// `C=1` (Overwrite) from `C=0` (AlphaBlend): with `sa=128`, the modes
/// produce byte-DIFFERENT outputs, so byte-exact assertions actually pin
/// the chosen mode (closes the `sa=255` short-circuit gap where opaque
/// source produced identical bytes for both modes).
fn setup_semi_red_over_blue(id: u32) -> SpecHarness {
    let semi_red = rgba_solid(4, 4, 255, 0, 0, 128);
    let blue = rgba_solid(4, 4, 0, 0, 255, 255);

    let mut h = SpecHarness::new();
    h.feed(&kitty_apc(
        format!("a=t,i={id},f=32,s=4,v=4,q=2").as_bytes(),
        &b64(&semi_red),
    ));
    h.feed(&kitty_apc(
        format!("a=f,i={id},f=32,s=4,v=4,q=2").as_bytes(),
        &b64(&blue),
    ));
    h
}

// ----------------------------------------------------------------------------
// Byte-exact mutation — compose actually rewrites destination frame.
// ----------------------------------------------------------------------------

/// Catalog row: `KG-ACTION-COMPOSE` (Overwrite-mode byte-exact mutation).
///
/// Uses a semi-transparent RED source (`sa=128`) over an opaque BLUE dest
/// so AlphaBlend and Overwrite produce byte-DIFFERENT outputs (the
/// `sa==255` short-circuit in `blend_pixel_over` would otherwise mask
/// `C=1` vs `C=0`). With `C=1` Overwrite, frame 2 MUST become byte-equal
/// to the semi-red source. With `C=0` AlphaBlend, frame 2 would be the
/// Porter-Duff blend (see `kitty_compose_c0_alpha_blend_matches_porter_duff`).
#[test]
fn kitty_compose_overwrite_writes_source_bytes_verbatim_to_dest_frame() {
    let id = 401;
    let mut h = setup_semi_red_over_blue(id);

    h.feed(&kitty_apc(
        format!("a=c,i={id},r=1,c=2,w=4,h=4,X=0,Y=0,x=0,y=0,C=1").as_bytes(),
        "",
    ));

    // Setup ran with q=2; ONE OK reply must come from the compose.
    assert_eq!(
        count_replies_exact(&h, &ok_reply_for(id)),
        1,
        "a=c overwrite MUST emit exactly one OK reply (setup is q=2-suppressed) — transcript: {:?}",
        String::from_utf8_lossy(&reply_bytes(&h)),
    );

    // C=1 Overwrite: dest becomes byte-equal to semi-transparent red source.
    // C=0 AlphaBlend would have produced (128, 0, 127, 255) per Porter-Duff
    // — the byte-exact RGBA(255, 0, 0, 128) check distinguishes the modes.
    let expected_semi_red = rgba_solid(4, 4, 255, 0, 0, 128);
    assert_frame_eq(
        h.term().image_cache(),
        ImageId::from_raw(id),
        2,
        &expected_semi_red,
    );
}

// ----------------------------------------------------------------------------
// Reject path absent — old EINVAL-reject route MUST NOT fire.
// ----------------------------------------------------------------------------

/// Catalog row: `KG-ACTION-COMPOSE` (reject-path absence).
///
/// `a=c` against a valid image MUST NOT emit `EINVAL: action `c` not
/// implemented` (the pre-fix reject route). Locks the dispatcher away
/// from the deleted `kitty_compose_reject` helper.
#[test]
fn kitty_compose_does_not_emit_legacy_einval_action_c_not_implemented() {
    let id = 402;
    let mut h = setup_2frame_red_green(id);

    h.feed(&kitty_apc(
        format!("a=c,i={id},r=1,c=2,w=4,h=4,X=0,Y=0,x=0,y=0,C=1").as_bytes(),
        "",
    ));

    let replies = reply_bytes(&h);
    let s = String::from_utf8_lossy(&replies);
    assert!(
        !s.contains("action `c` not implemented"),
        "a=c MUST NOT emit legacy EINVAL reject reply — transcript: {s:?}",
    );
}

// ----------------------------------------------------------------------------
// Edge cases — default w/h, partial rect, same-frame variants.
// ----------------------------------------------------------------------------

/// Catalog row: `KG-ACTION-COMPOSE` (default w/h).
///
/// `w=0,h=0` (or w/h absent) defaults to full image dims per kitty
/// `graphics.c:1830-1831`. Test exercises the w=0/h=0 path explicitly.
#[test]
fn kitty_compose_w0_h0_defaults_to_full_image() {
    let id = 403;
    let mut h = setup_2frame_red_green(id);

    h.feed(&kitty_apc(
        format!("a=c,i={id},r=1,c=2,w=0,h=0,X=0,Y=0,x=0,y=0,C=1").as_bytes(),
        "",
    ));

    assert_eq!(
        count_replies_exact(&h, &ok_reply_for(id)),
        1,
        "w=0,h=0 default compose MUST emit exactly one OK reply",
    );
    let expected_red = rgba_solid(4, 4, 255, 0, 0, 255);
    assert_frame_eq(
        h.term().image_cache(),
        ImageId::from_raw(id),
        2,
        &expected_red,
    );
}

/// Catalog row: `KG-ACTION-COMPOSE` (absent w/h).
///
/// `w=`/`h=` absent ALSO defaults to full image dims — parser maps
/// `Option<u32>::None` to `Some(0)` semantics at `extract_a_c_keys`.
#[test]
fn kitty_compose_absent_w_h_defaults_to_full_image() {
    let id = 404;
    let mut h = setup_2frame_red_green(id);

    h.feed(&kitty_apc(
        format!("a=c,i={id},r=1,c=2,X=0,Y=0,x=0,y=0,C=1").as_bytes(),
        "",
    ));

    assert_eq!(
        count_replies_exact(&h, &ok_reply_for(id)),
        1,
        "compose with absent w/h MUST emit exactly one OK reply",
    );
    let expected_red = rgba_solid(4, 4, 255, 0, 0, 255);
    assert_frame_eq(
        h.term().image_cache(),
        ImageId::from_raw(id),
        2,
        &expected_red,
    );
}

/// Catalog row: `KG-ACTION-COMPOSE` (same-frame disjoint).
///
/// `r=c` (same frame) with disjoint source/dest rects (no x-overlap or
/// no y-overlap) is permitted per kitty `graphics.c:1841` (strict-less-
/// than allows adjacent rects). Pinned to prove the overlap check uses
/// half-open intersection.
#[test]
fn kitty_compose_same_frame_disjoint_rects_succeed() {
    let id = 405;
    let mut h = setup_2frame_red_green(id);

    // Source: top-left 2×2 of frame 2 (green); Dest: bottom-right 2×2 of
    // frame 2. No x-overlap (src x=0..2, dst x=2..4). Adjacent rects.
    h.feed(&kitty_apc(
        format!("a=c,i={id},r=2,c=2,w=2,h=2,X=0,Y=0,x=2,y=2,C=1").as_bytes(),
        "",
    ));

    assert_eq!(
        count_replies_exact(&h, &ok_reply_for(id)),
        1,
        "same-frame disjoint compose MUST emit exactly one OK reply (setup is q=2-suppressed) — transcript: {:?}",
        String::from_utf8_lossy(&reply_bytes(&h)),
    );
}

/// Catalog row: `KG-ACTION-COMPOSE` (same-frame overlap).
///
/// `r=c` (same frame) with overlapping rects MUST emit EINVAL per kitty
/// `graphics.c:1841-1849`.
#[test]
fn kitty_compose_same_frame_overlapping_rects_emit_einval() {
    let id = 406;
    let mut h = setup_2frame_red_green(id);

    // Source: 0..2 x 0..2 of frame 2; Dest: 1..3 x 1..3 of frame 2.
    // x-overlap (max(0,1)=1 < min(0,1)+2=2) AND y-overlap → EINVAL.
    h.feed(&kitty_apc(
        format!("a=c,i={id},r=2,c=2,w=2,h=2,X=0,Y=0,x=1,y=1,C=1").as_bytes(),
        "",
    ));

    assert_einval_reply(&h, id, "overlap");
}

// ----------------------------------------------------------------------------
// Error responses — ENOENT for missing image/frame; EINVAL for OOB rect.
// ----------------------------------------------------------------------------

/// Catalog row: `KG-ACTION-COMPOSE` (ENOENT missing image).
#[test]
fn kitty_compose_missing_image_emits_enoent() {
    let mut h = SpecHarness::new();
    h.feed(&kitty_apc(b"a=c,i=999,r=1,c=1,w=4,h=4,C=1", ""));

    let prefix = b"\x1b_Gi=999;ENOENT";
    assert!(
        reply_contains(&h, prefix),
        "missing image MUST emit ENOENT — transcript: {:?}",
        String::from_utf8_lossy(&reply_bytes(&h)),
    );
}

/// Catalog row: `KG-ACTION-COMPOSE` (ENOENT missing source frame).
#[test]
fn kitty_compose_missing_source_frame_emits_enoent() {
    let id = 407;
    let mut h = setup_2frame_red_green(id);

    // Image has 2 frames; ask for frame 5 as source.
    h.feed(&kitty_apc(
        format!("a=c,i={id},r=5,c=1,w=4,h=4,C=1").as_bytes(),
        "",
    ));

    let prefix = format!("\x1b_Gi={id};ENOENT");
    assert!(
        reply_contains(&h, prefix.as_bytes()),
        "missing source frame MUST emit ENOENT — transcript: {:?}",
        String::from_utf8_lossy(&reply_bytes(&h)),
    );
}

/// Catalog row: `KG-ACTION-COMPOSE` (ENOENT missing dest frame).
#[test]
fn kitty_compose_missing_dest_frame_emits_enoent() {
    let id = 408;
    let mut h = setup_2frame_red_green(id);

    // Image has 2 frames; ask for frame 5 as dest.
    h.feed(&kitty_apc(
        format!("a=c,i={id},r=1,c=5,w=4,h=4,C=1").as_bytes(),
        "",
    ));

    let prefix = format!("\x1b_Gi={id};ENOENT");
    assert!(
        reply_contains(&h, prefix.as_bytes()),
        "missing dest frame MUST emit ENOENT — transcript: {:?}",
        String::from_utf8_lossy(&reply_bytes(&h)),
    );
}

/// Catalog row: `KG-ACTION-COMPOSE` (EINVAL r=0 absent).
///
/// `r=` absent/0 MUST emit ENOENT per the dispatcher's mandatory-key check
/// — `keys.src_frame == 0` short-circuits before `compose_frame`.
#[test]
fn kitty_compose_r_absent_emits_enoent() {
    let id = 409;
    let mut h = setup_2frame_red_green(id);

    h.feed(&kitty_apc(
        format!("a=c,i={id},c=1,w=4,h=4,C=1").as_bytes(),
        "",
    ));

    let prefix = format!("\x1b_Gi={id};ENOENT");
    assert!(
        reply_contains(&h, prefix.as_bytes()),
        "r= absent MUST emit ENOENT — transcript: {:?}",
        String::from_utf8_lossy(&reply_bytes(&h)),
    );
}

/// Catalog row: `KG-ACTION-COMPOSE` (EINVAL dest out-of-bounds).
#[test]
fn kitty_compose_dest_out_of_bounds_emits_einval() {
    let id = 410;
    let mut h = setup_2frame_red_green(id);

    // Image is 4×4; ask for dest rect starting at x=3 with w=4 → would
    // extend to x=7 > image_w=4.
    h.feed(&kitty_apc(
        format!("a=c,i={id},r=1,c=2,w=4,h=4,X=0,Y=0,x=3,y=0,C=1").as_bytes(),
        "",
    ));

    let prefix = format!("\x1b_Gi={id};EINVAL");
    assert!(
        reply_contains(&h, prefix.as_bytes()),
        "dest out-of-bounds MUST emit EINVAL — transcript: {:?}",
        String::from_utf8_lossy(&reply_bytes(&h)),
    );
}

/// Catalog row: `KG-ACTION-COMPOSE` (EINVAL source out-of-bounds).
#[test]
fn kitty_compose_source_out_of_bounds_emits_einval() {
    let id = 411;
    let mut h = setup_2frame_red_green(id);

    // Source rect at X=3 with w=4 → extends to x=7 > image_w=4.
    h.feed(&kitty_apc(
        format!("a=c,i={id},r=1,c=2,w=4,h=4,X=3,Y=0,x=0,y=0,C=1").as_bytes(),
        "",
    ));

    let prefix = format!("\x1b_Gi={id};EINVAL");
    assert!(
        reply_contains(&h, prefix.as_bytes()),
        "source out-of-bounds MUST emit EINVAL — transcript: {:?}",
        String::from_utf8_lossy(&reply_bytes(&h)),
    );
}

// ----------------------------------------------------------------------------
// Composition mode coverage.
// ----------------------------------------------------------------------------

/// Catalog row: `KG-ACTION-COMPOSE` (C=0 AlphaBlend — hand-computed pin).
///
/// `C=0` (default) routes to `CompositionMode::AlphaBlend` per kitty
/// `graphics.c:1862`. Asserts the Porter-Duff source-over output
/// byte-exactly so the test pins the blend kernel's arithmetic, not just
/// "different from source bytes". For `semi_red (255, 0, 0, 128)` over
/// `blue (0, 0, 255, 255)`:
///
/// - `sa = 128`, `da = 255`, `inv_sa = 127`
/// - `oa = (sa + da * inv_sa / 255).min(255) = (128 + 127).min(255) = 255`
/// - `r = (255 * 128 + 0 * 255 * 127 / 255) / 255 = 32640 / 255 = 128`
/// - `g = (0 * 128 + 0 * 255 * 127 / 255) / 255 = 0`
/// - `b = (0 * 128 + 255 * 255 * 127 / 255) / 255 = 32388 / 255 = 127`
///
/// Expected blended RGBA: `(128, 0, 127, 255)`.
#[test]
fn kitty_compose_c0_alpha_blend_matches_porter_duff() {
    let id = 412;
    let mut h = setup_semi_red_over_blue(id);

    h.feed(&kitty_apc(
        format!("a=c,i={id},r=1,c=2,w=4,h=4,X=0,Y=0,x=0,y=0,C=0").as_bytes(),
        "",
    ));

    assert_eq!(
        count_replies_exact(&h, &ok_reply_for(id)),
        1,
        "a=c C=0 MUST emit exactly one OK reply — transcript: {:?}",
        String::from_utf8_lossy(&reply_bytes(&h)),
    );

    let expected_blended = rgba_solid(4, 4, 128, 0, 127, 255);
    assert_frame_eq(
        h.term().image_cache(),
        ImageId::from_raw(id),
        2,
        &expected_blended,
    );
}

/// Catalog row: `KG-ACTION-COMPOSE` (C absent defaults to AlphaBlend).
///
/// Proves the spec default — `C=` key omitted MUST route to AlphaBlend
/// (NOT Overwrite). Same Porter-Duff arithmetic as `C=0` pinned via
/// hand-computed expected bytes (see `kitty_compose_c0_alpha_blend_matches_porter_duff`).
#[test]
fn kitty_compose_c_absent_matches_c_zero_alpha_blend_output() {
    let id = 413;
    let mut h = setup_semi_red_over_blue(id);

    // C= key omitted entirely: must default to AlphaBlend (NOT Overwrite).
    h.feed(&kitty_apc(
        format!("a=c,i={id},r=1,c=2,w=4,h=4,X=0,Y=0,x=0,y=0").as_bytes(),
        "",
    ));

    assert_eq!(
        count_replies_exact(&h, &ok_reply_for(id)),
        1,
        "a=c with C absent MUST emit exactly one OK reply",
    );

    // Same Porter-Duff output as C=0 test.
    let expected_blended = rgba_solid(4, 4, 128, 0, 127, 255);
    assert_frame_eq(
        h.term().image_cache(),
        ImageId::from_raw(id),
        2,
        &expected_blended,
    );
}

// ----------------------------------------------------------------------------
// Fallback row regression — `a=Z` truly-unknown still TransmitAndPlace.
// ----------------------------------------------------------------------------

/// Catalog row: `KG-ACTION-FALLBACK-TRANSMITANDPLACE` (regression).
///
/// Compose handler landing MUST NOT widen the fallback scope or narrow
/// the explicit-`a=c` arm. `a=Z` (truly-unknown) MUST still fall back to
/// TransmitAndPlace per the existing fallback row.
#[test]
fn kitty_compose_landing_does_not_disturb_fallback_a_z() {
    let red = rgba_solid(4, 4, 255, 0, 0, 255);
    let id = 414;

    let mut h = SpecHarness::new();
    h.feed(&kitty_apc(
        format!("a=Z,i={id},f=32,s=4,v=4").as_bytes(),
        &b64(&red),
    ));

    assert!(
        reply_contains(&h, &ok_reply_for(id)),
        "a=Z fallback to TransmitAndPlace MUST still emit OK — transcript: {:?}",
        String::from_utf8_lossy(&reply_bytes(&h)),
    );
}
