//! Per-action rung (§13.1) — drives every `KittyAction` variant through
//! parser → dispatch → state/effect apex.
//!
//! Each test strengthens its assertion beyond "emits OK" so a regression
//! that misroutes an action to a different arm (animate / query) cannot
//! silently pass — the per-action clamps are addressed in the §13.1 TPR
//! round 0 codex F1/F2 + codex F3 / gemini F2 agreement findings.

use oriterm_test_support::spec_chain::SpecHarness;

use super::fixtures::{
    b64, count_replies_exact, kitty_apc, ok_reply_for, placement_count, reply_bytes,
    reply_contains, rgba_4x4_red,
};

/// Catalog row: `KG-ACTION-TRANSMIT` (a=t).
///
/// Pure transmit stores the image without creating a placement. Strengthened
/// per §13.1 TPR round 0 codex F1: a follow-up `a=p,i=1` proves the image
/// was actually stored — this fails with ENOENT if `a=t` had been rerouted
/// to a no-op action (query / animate) that emitted OK without storing.
#[test]
fn kitty_action_transmit_stores_image_without_placement() {
    let mut h = SpecHarness::new();
    h.feed(&kitty_apc(b"a=t,i=1,f=32,s=4,v=4", &b64(&rgba_4x4_red())));

    assert_eq!(
        placement_count(&h),
        0,
        "a=t MUST NOT create a placement — stores image only",
    );
    assert!(
        reply_contains(&h, &ok_reply_for(1)),
        "a=t success emits OK reply for i=1 — got {:?}",
        String::from_utf8_lossy(&reply_bytes(&h)),
    );

    // Prove storage actually landed: a=p,i=1 MUST create a placement.
    // If a=t had been rerouted to kitty_query/kitty_animate (no-ops that
    // also emit OK), no image would be in the cache and a=p would emit
    // ENOENT instead of succeeding.
    h.feed(&kitty_apc(b"a=p,i=1", ""));
    assert_eq!(
        placement_count(&h),
        1,
        "a=p,i=1 MUST create a placement — proving a=t stored the image",
    );
    let enoent_bytes = b"\x1b_Gi=1;ENOENT";
    assert!(
        !reply_contains(&h, enoent_bytes),
        "a=p,i=1 MUST NOT emit ENOENT — proves a=t reached kitty_store_image \
         and not a cache-less no-op; transcript: {:?}",
        String::from_utf8_lossy(&reply_bytes(&h)),
    );
}

/// Catalog row: `KG-ACTION-TRANSMIT-AND-PLACE` (a=T — also the parser-default fallback arm for unknown a= values).
///
/// Transmit+place stores AND creates one placement, plus emits OK reply.
#[test]
fn kitty_action_transmit_and_place_creates_placement() {
    let mut h = SpecHarness::new();
    h.feed(&kitty_apc(b"a=T,i=2,f=32,s=4,v=4", &b64(&rgba_4x4_red())));

    assert_eq!(placement_count(&h), 1, "a=T MUST create one placement",);
    assert!(
        reply_contains(&h, &ok_reply_for(2)),
        "a=T success emits OK reply for i=2 — got {:?}",
        String::from_utf8_lossy(&reply_bytes(&h)),
    );
}

/// Catalog row: `KG-ACTION-PLACE` (a=p).
///
/// A `Place` after a prior `Transmit` creates one placement. Exercises the
/// cache-lookup path at `kitty_place:17` (`get_no_touch`) + success-branch
/// at `kitty_place:23-26`.
#[test]
fn kitty_action_place_creates_placement_for_prior_transmit() {
    let mut h = SpecHarness::new();
    h.feed(&kitty_apc(b"a=t,i=3,f=32,s=4,v=4", &b64(&rgba_4x4_red())));
    h.feed(&kitty_apc(b"a=p,i=3", ""));

    assert_eq!(
        placement_count(&h),
        1,
        "a=p after a=t MUST create one placement",
    );
    assert!(
        reply_contains(&h, &ok_reply_for(3)),
        "a=p success emits OK reply for i=3 — got {:?}",
        String::from_utf8_lossy(&reply_bytes(&h)),
    );
}

/// Catalog row: `KG-ACTION-DELETE` (a=d,d=a smoke — per-specifier coverage is §13.0.5's delete/tests.rs matrix).
///
/// Transmit+place → one placement → `a=d,d=a` drops visible placements →
/// zero placements. No reply emitted on the delete path.
#[test]
fn kitty_action_delete_removes_visible_placement() {
    let mut h = SpecHarness::new();
    h.feed(&kitty_apc(b"a=T,i=4,f=32,s=4,v=4", &b64(&rgba_4x4_red())));
    assert_eq!(placement_count(&h), 1, "precondition: a=T placed image");

    h.feed(&kitty_apc(b"a=d,d=a", ""));
    assert_eq!(
        placement_count(&h),
        0,
        "a=d,d=a MUST drop the visible placement",
    );
}

/// Catalog row: `KG-ACTION-FRAME` (a=f — full verification in §13.3; §13.1 smoke-tests dispatch).
///
/// Frame on a prior-transmitted image adds an animation frame and emits OK.
/// Strengthened per §13.1 TPR round 0 codex F3 / gemini F2 agreement: the
/// count_replies_exact check asserts the second command produced its OWN
/// OK reply, not just that the prior transmit's OK is still in the transcript.
#[test]
fn kitty_action_frame_on_prior_transmit_emits_ok_reply() {
    let mut h = SpecHarness::new();
    h.feed(&kitty_apc(b"a=t,i=5,f=32,s=4,v=4", &b64(&rgba_4x4_red())));
    assert_eq!(
        count_replies_exact(&h, &ok_reply_for(5)),
        1,
        "precondition: a=t emitted exactly one OK reply for i=5",
    );
    assert_eq!(
        placement_count(&h),
        0,
        "precondition: a=t stored image but did NOT create a placement",
    );

    h.feed(&kitty_apc(b"a=f,i=5,f=32,s=4,v=4", &b64(&rgba_4x4_red())));

    // §13.3 added the `,r=<frame_num>` qualifier per kitty
    // finish_command_response (graphics.c:802-806). The a=f OK reply for
    // the freshly-added frame carries `,r=2` (static-to-animated promotion
    // places the new frame at 1-based index 2). The precondition a=t OK
    // remains `i=5;OK` with no frame qualifier.
    let ok_reply_frame2 = format!("\x1b_Gi=5,r=2;OK\x1b\\").into_bytes();
    assert_eq!(
        count_replies_exact(&h, &ok_reply_for(5)),
        1,
        "a=t emits `i=5;OK` (unchanged); a=f emits `i=5,r=2;OK` (new \
         ,r= qualifier), so the bare-i=5 form stays at 1 — transcript: {:?}",
        String::from_utf8_lossy(&reply_bytes(&h)),
    );
    assert_eq!(
        count_replies_exact(&h, &ok_reply_frame2),
        1,
        "a=f MUST emit its OWN OK reply with `,r=2` qualifier for the \
         newly-added frame — transcript: {:?}",
        String::from_utf8_lossy(&reply_bytes(&h)),
    );
    // Dispatch-identity pin: a=f adds animation frames, NOT placements.
    // If a=f had been rerouted to kitty_place, placement_count would be 1.
    // This uniquely distinguishes frame from place at the existing-image apex
    // (addresses §13.1 TPR round 2 codex F1 + gemini F1 agreement finding).
    assert_eq!(
        placement_count(&h),
        0,
        "a=f MUST NOT create a placement — if rerouted to kitty_place, \
         placement_count would be 1",
    );
}

/// Catalog row: `KG-ACTION-ANIMATE` (a=a — full verification in §13.3; §13.1 smoke-tests dispatch).
///
/// Animate on a prior-transmitted image applies playback state and emits
/// OK. Same transcript-accumulation strengthening as the frame test.
#[test]
fn kitty_action_animate_on_prior_transmit_emits_ok_reply() {
    let mut h = SpecHarness::new();
    h.feed(&kitty_apc(b"a=t,i=6,f=32,s=4,v=4", &b64(&rgba_4x4_red())));
    assert_eq!(
        count_replies_exact(&h, &ok_reply_for(6)),
        1,
        "precondition: a=t emitted exactly one OK reply for i=6",
    );
    assert_eq!(
        placement_count(&h),
        0,
        "precondition: a=t stored image but did NOT create a placement",
    );

    h.feed(&kitty_apc(b"a=a,i=6,s=1", ""));

    assert_eq!(
        count_replies_exact(&h, &ok_reply_for(6)),
        2,
        "a=a MUST emit its OWN OK reply for i=6 (total should now be 2) — \
         transcript: {:?}",
        String::from_utf8_lossy(&reply_bytes(&h)),
    );
    // Dispatch-identity pin: a=a mutates animation playback state, NOT
    // placements. If a=a had been rerouted to kitty_place, placement_count
    // would be 1 (addresses §13.1 TPR round 2 gemini F2 finding).
    assert_eq!(
        placement_count(&h),
        0,
        "a=a MUST NOT create a placement — if rerouted to kitty_place, \
         placement_count would be 1",
    );
}

/// Catalog row: `KG-ACTION-QUERY` (a=q).
///
/// Query emits OK without state change. Strengthened per §13.1 TPR round 0
/// codex F2: a prior transmit seeds a known image, the query is sent on a
/// DIFFERENT id, and we assert (a) the prior image is still fully intact
/// (follow-up a=p on the seeded id creates a placement) AND (b) the query
/// produced its own OK reply. This distinguishes query from a rerouted
/// `a=p` (which would emit ENOENT on an unknown id) or `a=t` (which would
/// emit EINVAL on empty payload).
#[test]
fn kitty_action_query_emits_ok_reply_without_state_change() {
    let mut h = SpecHarness::new();
    // Seed a stored image at i=50 so we can verify query doesn't mutate it.
    h.feed(&kitty_apc(b"a=t,i=50,f=32,s=4,v=4", &b64(&rgba_4x4_red())));
    assert!(
        reply_contains(&h, &ok_reply_for(50)),
        "precondition: seed transmit OK"
    );

    // Query a DIFFERENT id. No prior transmit for i=7.
    h.feed(&kitty_apc(b"a=q,i=7", ""));

    assert_eq!(placement_count(&h), 0, "a=q MUST NOT create a placement",);
    assert!(
        reply_contains(&h, &ok_reply_for(7)),
        "a=q emits OK reply for i=7 — got {:?}",
        String::from_utf8_lossy(&reply_bytes(&h)),
    );
    // Query MUST emit OK even for an id with no prior transmit — this
    // distinguishes it from a misroute to `a=p` which emits ENOENT, or
    // `a=t` which emits EINVAL for empty-payload + missing s=/v=.
    let enoent_7 = b"\x1b_Gi=7;ENOENT";
    let einval_7 = b"\x1b_Gi=7;EINVAL";
    assert!(
        !reply_contains(&h, enoent_7),
        "a=q on unknown id MUST NOT emit ENOENT (that's place's behavior)",
    );
    assert!(
        !reply_contains(&h, einval_7),
        "a=q MUST NOT emit EINVAL (that would be a misroute to transmit with empty payload)",
    );

    // Verify the seeded image at i=50 is still intact — a=q MUST NOT
    // mutate cached state.
    h.feed(&kitty_apc(b"a=p,i=50", ""));
    assert_eq!(
        placement_count(&h),
        1,
        "a=p,i=50 MUST succeed after a=q — proves a=q did NOT evict or \
         corrupt the seeded image (which a rerouted delete would have)",
    );
}

// Semantic + negative pins.

/// Catalog row: `KG-ACTION-FALLBACK-TRANSMITANDPLACE` (semantic pin for a=c fallback).
///
/// `a=c` (unknown action value) MUST fall through parser `parse.rs:199`
/// to `KittyAction::TransmitAndPlace`, so a placement is created AND an
/// OK reply is emitted. A regression that flips the fallback arm to `Err`
/// or routes unknowns to `Transmit` (no placement) fails this pin.
#[test]
fn kitty_action_fallback_transmit_and_place_on_unknown_a_value() {
    let mut h = SpecHarness::new();
    h.feed(&kitty_apc(b"a=c,i=30,f=32,s=4,v=4", &b64(&rgba_4x4_red())));

    assert_eq!(
        placement_count(&h),
        1,
        "unknown a=c MUST fall back to TransmitAndPlace (creates placement)",
    );
    assert!(
        reply_contains(&h, &ok_reply_for(30)),
        "unknown a=c fallback still emits OK reply — got {:?}",
        String::from_utf8_lossy(&reply_bytes(&h)),
    );
}

/// Negative pin paired with the semantic pin above: unknown `a=x` routes
/// through the parser fallback to `TransmitAndPlace`, which MUST create
/// exactly one placement. If a future refactor narrows `parse.rs:199` to
/// reject unknown values or to route them to `Transmit` (zero placements),
/// this assertion fails — proving the fallback semantics are load-bearing.
#[test]
fn kitty_action_unknown_a_value_routes_to_transmit_and_place_fallback() {
    let mut h = SpecHarness::new();
    h.feed(&kitty_apc(b"a=x,i=31,f=32,s=4,v=4", &b64(&rgba_4x4_red())));

    assert_eq!(
        placement_count(&h),
        1,
        "unknown a=x MUST route to TransmitAndPlace (creates one placement); \
         if rerouted to Transmit the count would be 0, if rejected with Err \
         the count would also be 0",
    );
}

// Dispatch-identity pins — each action has a unique observable signature
// that distinguishes it from every other KittyAction at the public harness
// level. Addresses §13.1 TPR round 1 codex F1 + F2: the positive tests
// above share behavior with `a=q`/`a=a` (both emit OK on unknown images),
// so these negative pins close the clamp by asserting signatures no other
// action can produce.

/// Catalog row: `KG-ACTION-QUERY` (dispatch-identity pin — no-image-id signature).
///
/// `a=q` with NO `i=` key emits `OK` for `i=0` (per `query.rs:10`,
/// `cmd.image_id.unwrap_or(0)`). `a=a` without `i=` returns EARLY without
/// emitting any reply (per `animate.rs:20-23`). This pair uniquely
/// distinguishes query from animate at the public apex — a regression
/// that rerouted `a=q` to `kitty_animate` would fail this test.
#[test]
fn kitty_action_query_without_image_id_emits_ok_for_id_zero() {
    let mut h = SpecHarness::new();
    h.feed(&kitty_apc(b"a=q", ""));

    assert!(
        reply_contains(&h, &ok_reply_for(0)),
        "a=q without i= MUST emit OK reply for i=0 — transcript: {:?}. \
         This distinguishes query from animate (which returns early with \
         no reply when i= is absent) and from transmit/place/frame \
         (which emit error replies on the missing fields).",
        String::from_utf8_lossy(&reply_bytes(&h)),
    );
}

/// Catalog row: `KG-ACTION-ANIMATE` (dispatch-identity pin — ENOENT-on-missing-id signature).
///
/// `a=a` without `i=` (and without `I=` fallback) emits `ENOENT` per
/// BUG-08-024 fix — mirrors `kitty_place`'s missing-identifier path
/// (`place.rs:16-19`). Every kitty action handler now returns a reply
/// on error; silent drop is no longer a valid distinguishing signature.
#[test]
fn kitty_action_animate_without_image_id_emits_enoent() {
    let mut h = SpecHarness::new();
    h.feed(&kitty_apc(b"a=a", ""));

    let replies = reply_bytes(&h);
    let s = String::from_utf8_lossy(&replies);
    assert!(
        s.contains("ENOENT"),
        "a=a without i= MUST emit ENOENT (per BUG-08-024) — got {s:?}. \
         Mirrors kitty_place's missing-identifier path."
    );
}

/// Catalog row: `KG-ACTION-FRAME` (dispatch-identity pin — ENOENT-on-missing-image signature).
///
/// `a=f,i=<unknown>` emits `ENOENT` (per `frame.rs:31-34`, the cache
/// `get_no_touch` check). Query and animate would emit OK; place would
/// emit ENOENT too but places NO placement; delete emits no reply.
/// The ENOENT reply combined with placement_count == 0 and no
/// OK-for-the-id uniquely pins frame dispatch on a missing image.
#[test]
fn kitty_action_frame_on_missing_image_emits_enoent_reply() {
    let mut h = SpecHarness::new();
    // No prior transmit — image 99 doesn't exist.
    h.feed(&kitty_apc(b"a=f,i=99,f=32,s=4,v=4", &b64(&rgba_4x4_red())));

    let replies = reply_bytes(&h);
    let s = String::from_utf8_lossy(&replies);
    assert!(
        s.contains("\u{1b}_Gi=99;ENOENT"),
        "a=f on missing image MUST emit ENOENT reply for i=99 — got {s:?}. \
         This distinguishes frame from query/animate (which emit OK for \
         unknown ids) at the dispatch apex.",
    );
    assert!(
        !reply_contains(&h, &ok_reply_for(99)),
        "a=f on missing image MUST NOT emit OK for i=99 (that would be a \
         query/animate misroute) — transcript: {s:?}",
    );
}

// Matrix completeness.

/// Self-verifying matrix completeness per `.claude/rules/tests.md`
/// §Self-Verifying Matrix Completeness — asserts each (action, format,
/// transmission) cell actually dispatched a typed `DispatchArgs::KittyApc`
/// with matching `cmd.action` / `cmd.format` / `cmd.transmission`. This
/// clamps per-cell dispatch identity (HYG-13.1-003 resolution): a cell
/// that mis-parses, drops to the `Other` arm, or re-routes to the wrong
/// action now fails the inner `assert!`, not just the outer count.
#[test]
fn action_format_transmission_matrix_completeness() {
    use oriterm_core::image::kitty::{KittyAction, KittyTransmission};
    use oriterm_test_support::spec_chain::DispatchArgs;

    const ACTIONS: &[u8] = b"tTpdfaq";
    const FORMATS: &[u32] = &[24, 32, 100];
    const TRANSMISSIONS: &[u8] = b"dfts";

    fn expected_action(act: u8) -> KittyAction {
        match act {
            b't' => KittyAction::Transmit,
            b'T' => KittyAction::TransmitAndPlace,
            b'p' => KittyAction::Place,
            b'd' => KittyAction::Delete,
            b'f' => KittyAction::Frame,
            b'a' => KittyAction::Animate,
            b'q' => KittyAction::Query,
            _ => unreachable!("unexpected action byte {act}"),
        }
    }

    fn expected_transmission(trans: u8) -> KittyTransmission {
        match trans {
            b'd' => KittyTransmission::Direct,
            b'f' => KittyTransmission::File,
            b't' => KittyTransmission::TempFile,
            b's' => KittyTransmission::SharedMemory,
            _ => unreachable!("unexpected transmission byte {trans}"),
        }
    }

    let mut count = 0usize;
    for &act in ACTIONS {
        for &fmt in FORMATS {
            for &trans in TRANSMISSIONS {
                let mut h = SpecHarness::new();
                let control = format!(
                    "a={act_ch},i=100,f={fmt},t={trans_ch},s=4,v=4",
                    act_ch = act as char,
                    trans_ch = trans as char,
                );
                h.feed(&kitty_apc(control.as_bytes(), &b64(&rgba_4x4_red())));

                let command = h
                    .outcome()
                    .dispatched_calls
                    .iter()
                    .find_map(|c| match &c.args {
                        DispatchArgs::KittyApc { command } => Some(command.as_ref()),
                        _ => None,
                    })
                    .unwrap_or_else(|| {
                        panic!(
                            "cell (a={ac},f={fmt},t={tc}) did not record DispatchArgs::KittyApc",
                            ac = act as char,
                            tc = trans as char,
                        )
                    });
                assert_eq!(
                    command.action,
                    expected_action(act),
                    "cell (a={ac},f={fmt},t={tc}) — action mismatch",
                    ac = act as char,
                    tc = trans as char,
                );
                assert_eq!(
                    command.format,
                    fmt,
                    "cell (a={ac},f={fmt},t={tc}) — format mismatch",
                    ac = act as char,
                    tc = trans as char,
                );
                assert_eq!(
                    command.transmission,
                    expected_transmission(trans),
                    "cell (a={ac},f={fmt},t={tc}) — transmission mismatch",
                    ac = act as char,
                    tc = trans as char,
                );
                // Post-delegate clamp: `DispatchArgs::KittyApc` is recorded
                // BEFORE `Handler::apc_dispatch` delegates to `Term`, so a
                // dispatch-only assertion cannot catch a routing failure
                // that lands AFTER the record. For actions that always
                // produce an effect under `q=0` (success OK reply or error
                // reply), clamp `effects_emitted.is_empty() == false`. The
                // `a=d` delegate correctly emits nothing when `d=a` matches
                // zero images in a fresh cache — that silent-no-op is the
                // verified-with-deviation behavior per `KG-ACTION-DELETE`,
                // so `a=d` cells rely on the dispatch-identity clamp above.
                if act != b'd' {
                    assert!(
                        !h.outcome().effects_emitted.is_empty(),
                        "cell (a={ac},f={fmt},t={tc}) — post-delegate effects_emitted is empty; dispatch was recorded but delegate produced no observable",
                        ac = act as char,
                        tc = trans as char,
                    );
                }
                count += 1;
            }
        }
    }

    assert_eq!(count, ACTIONS.len() * FORMATS.len() * TRANSMISSIONS.len());
    assert_eq!(
        count,
        7 * 3 * 4,
        "matrix shape is 7 actions × 3 formats × 4 transmissions = 84 cells",
    );
}
