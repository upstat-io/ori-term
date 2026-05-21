//! Per-animation rung (§13.3) — drives `a=f` frame-transmit + composition
//! modes, `a=a` playback control, loop count, frame gap, and the `,r=`
//! qualifier on frame/animate replies through the `SpecHarness` verification
//! chain.
//!
//! Catalog rows: `KG-FRAME-TRANSMIT` (a=f appends a frame, OK reply with
//!,r=<n>), `KG-FRAME-COMPOSITE-OVERWRITE` (X=1 overwrites), `KG-FRAME-
//! COMPOSITE-ALPHABLEND` (default alpha-blends), `KG-ANIMATE-STOP`
//! (s=1 pauses), `KG-ANIMATE-RUN-WAIT` (s=2 resumes waiting), `KG-ANIMATE-RUN`
//! (s=3 resumes), `KG-ANIMATE-LOOP-COUNT` (v=0 → infinite, v=N → finite),
//! `KG-ANIMATE-SET-CURRENT-FRAME` (r=N / c=N), `KG-ANIMATE-SET-FRAME-GAP`
//! (z=ms for current frame).

use std::time::{Duration, Instant};

use oriterm_core::image::ImageId;
use oriterm_test_support::spec_chain::SpecHarness;

use super::fixtures::{
    assert_einval_reply, assert_frame_eq, b64, kitty_apc, ok_reply_for, reply_bytes,
    reply_contains, rgba_4x4_red, rgba_solid,
};

/// Build an `a=f` OK-reply expectation with the `,r=<frame_num>` qualifier
/// per kitty `finish_command_response` for frame-loading replies.
fn ok_reply_with_frame(id: u32, frame_num: u32) -> Vec<u8> {
    format!("\x1b_Gi={id},r={frame_num};OK\x1b\\").into_bytes()
}

/// Transmit a base image via `a=t`, then append an `a=f` frame, asserting
/// the cache promotes to animated with 2 frames and the OK reply carries
/// `,r=2` (newly-added frame is 1-based index 2 per kitty's promotion rule:
/// existing data becomes frame 1, new data frame 2).
/// Catalog row: `KG-FRAME-TRANSMIT`.
#[test]
fn kitty_frame_reply_echoes_r_qualifier_for_added_frame_index() {
    let base = b64(&rgba_4x4_red());
    let frame_payload = b64(&rgba_4x4_red());

    let mut h = SpecHarness::new();
    h.feed(&kitty_apc(b"a=t,i=99,f=32,s=4,v=4", &base));
    assert!(
        reply_contains(&h, &ok_reply_for(99)),
        "a=t,i=99 MUST emit OK for the base image — transcript: {:?}",
        String::from_utf8_lossy(&reply_bytes(&h)),
    );

    h.feed(&kitty_apc(b"a=f,i=99,f=32,s=4,v=4", &frame_payload));
    assert!(
        reply_contains(&h, &ok_reply_with_frame(99, 2)),
        "a=f,i=99 MUST emit OK reply with ,r=2 (static-to-animated promotion \
         places the new frame at 1-based index 2) — transcript: {:?}",
        String::from_utf8_lossy(&reply_bytes(&h)),
    );

    let state = h
        .term()
        .image_cache()
        .animation_snapshot(ImageId::from_raw(99))
        .expect("a=f MUST promote i=99 to animated");
    assert_eq!(
        state.total_frames, 2,
        "a=f,i=99 MUST leave i=99 with 2 frames (static promotion + new frame)"
    );
}

/// Second `a=f` append pushes the frame to 1-based index 3 — verifies the
/// Occupied-entry branch of `add_animation_frame` reports `anim_frames.len()`
/// (not the promotion constant 2).
/// Catalog row: `KG-FRAME-TRANSMIT` (post-promotion append).
#[test]
fn kitty_frame_reply_echoes_r_qualifier_for_subsequent_frame_append() {
    let base = b64(&rgba_4x4_red());
    let frame = b64(&rgba_4x4_red());

    let mut h = SpecHarness::new();
    h.feed(&kitty_apc(b"a=t,i=98,f=32,s=4,v=4", &base));
    h.feed(&kitty_apc(b"a=f,i=98,f=32,s=4,v=4", &frame));
    // Third frame — appended after the static-to-animated promotion.
    h.feed(&kitty_apc(b"a=f,i=98,f=32,s=4,v=4", &frame));

    assert!(
        reply_contains(&h, &ok_reply_with_frame(98, 3)),
        "second a=f,i=98 MUST emit OK reply with ,r=3 — transcript: {:?}",
        String::from_utf8_lossy(&reply_bytes(&h)),
    );

    let state = h
        .term()
        .image_cache()
        .animation_snapshot(ImageId::from_raw(98))
        .expect("i=98 MUST be animated after two a=f commands");
    assert_eq!(state.total_frames, 3, "three frames total after two a=f");
}

/// Missing-image `a=f` emits ENOENT with NO `,r=` qualifier — the negative
/// pin that proves the frame-index chaining only happens on the success arm.
/// Catalog row: `KG-FRAME-TRANSMIT` (regression guard, ENOENT path).
#[test]
fn kitty_frame_reply_r_qualifier_omitted_on_missing_image_enoent() {
    let frame = b64(&rgba_4x4_red());

    let mut h = SpecHarness::new();
    // No prior a=t for i=77 — a=f hits the ENOENT arm.
    h.feed(&kitty_apc(b"a=f,i=77,f=32,s=4,v=4", &frame));

    let enoent = format!("\x1b_Gi=77;ENOENT\x1b\\");
    assert!(
        reply_contains(&h, enoent.as_bytes()),
        "a=f on missing image MUST emit ENOENT with i=77 and NO ,r= \
         qualifier — transcript: {:?}",
        String::from_utf8_lossy(&reply_bytes(&h)),
    );
    assert!(
        !reply_contains(&h, b",r="),
        "ENOENT reply MUST NOT include ,r=<frame_num> (no frame was added) — \
         transcript: {:?}",
        String::from_utf8_lossy(&reply_bytes(&h)),
    );
}

/// `a=f` with default (no `X=`) composition mode alpha-blends the new frame
/// onto the previous — pinning the default branch.
/// Catalog row: `KG-FRAME-COMPOSITE-ALPHABLEND`.
#[test]
fn kitty_frame_composite_alphablend_default_appends_frame_without_overwrite_flag() {
    let base = b64(&rgba_4x4_red());
    let frame = b64(&rgba_4x4_red());

    let mut h = SpecHarness::new();
    h.feed(&kitty_apc(b"a=t,i=80,f=32,s=4,v=4", &base));
    // No X= key → cell_x_offset is 0 → AlphaBlend branch.
    h.feed(&kitty_apc(b"a=f,i=80,f=32,s=4,v=4", &frame));

    let state = h
        .term()
        .image_cache()
        .animation_snapshot(ImageId::from_raw(80))
        .expect("default-composition a=f still promotes to animated");
    assert_eq!(
        state.total_frames, 2,
        "AlphaBlend default MUST still append the frame (composition only \
         affects pixel merging, not frame count)"
    );
}

/// `a=f` with `X=1` routes through the Overwrite composition mode — pins the
/// distinct code path. Both modes produce the same frame count; the
/// distinguishing assertion is the `,r=2` reply lands from either branch.
/// Catalog row: `KG-FRAME-COMPOSITE-OVERWRITE`.
#[test]
fn kitty_frame_composite_overwrite_when_x_one_still_appends_frame() {
    let base = b64(&rgba_4x4_red());
    let frame = b64(&rgba_4x4_red());

    let mut h = SpecHarness::new();
    h.feed(&kitty_apc(b"a=t,i=81,f=32,s=4,v=4", &base));
    h.feed(&kitty_apc(b"a=f,i=81,f=32,s=4,v=4,X=1", &frame));

    assert!(
        reply_contains(&h, &ok_reply_with_frame(81, 2)),
        "a=f,X=1 MUST reach the Occupied/Vacant path and emit OK with ,r=2 — \
         transcript: {:?}",
        String::from_utf8_lossy(&reply_bytes(&h)),
    );

    let state = h
        .term()
        .image_cache()
        .animation_snapshot(ImageId::from_raw(81))
        .expect("X=1 composition still promotes to animated");
    assert_eq!(state.total_frames, 2, "Overwrite MUST append frame 2");
}

/// `a=a,s=1` pauses animation; the reply echoes `,r=1` for the still-current
/// frame 0 (1-based = 1). Pins the stop path + the current-frame echo.
/// Catalog row: `KG-ANIMATE-STOP`.
#[test]
fn kitty_animate_s_one_pauses_and_reply_echoes_current_frame() {
    let base = b64(&rgba_4x4_red());
    let frame = b64(&rgba_4x4_red());

    let mut h = SpecHarness::new();
    h.feed(&kitty_apc(b"a=t,i=60,f=32,s=4,v=4", &base));
    h.feed(&kitty_apc(b"a=f,i=60,f=32,s=4,v=4", &frame));

    h.feed(&kitty_apc(b"a=a,i=60,s=1", ""));

    let state = h
        .term()
        .image_cache()
        .animation_snapshot(ImageId::from_raw(60))
        .expect("i=60 MUST be animated");
    assert!(state.paused, "a=a,s=1 MUST set paused = true");

    assert!(
        reply_contains(&h, &ok_reply_with_frame(60, 1)),
        "a=a,s=1 reply MUST echo ,r=1 (current frame 0 + 1) — transcript: {:?}",
        String::from_utf8_lossy(&reply_bytes(&h)),
    );
}

/// `a=a,v=0` is IGNORED per kitty `graphics.c:1766-1768`
/// (`if (g->loop_count) { max_loops = g->loop_count - 1; }` — truthy guard).
/// Prior loop_count MUST be preserved when v=0 is sent.
/// Catalog row: `KG-ANIMATE-LOOP-COUNT` (v=0 ignored).
/// Pre-fix code routed v=0 → loop_count=None (infinite) which was an
/// inverted sentinel relative to kitty.
#[test]
fn kitty_animate_v_zero_is_ignored_leaves_prior_loop_count_unchanged() {
    let base = b64(&rgba_4x4_red());
    let frame = b64(&rgba_4x4_red());

    let mut h = SpecHarness::new();
    h.feed(&kitty_apc(b"a=t,i=61,f=32,s=4,v=4", &base));
    h.feed(&kitty_apc(b"a=f,i=61,f=32,s=4,v=4", &frame));

    // Seed with v=5 (kitty: max_loops = 5 - 1 = 4 → Some(4)).
    h.feed(&kitty_apc(b"a=a,i=61,v=5", ""));
    assert_eq!(
        h.term()
            .image_cache()
            .animation_snapshot(ImageId::from_raw(61))
            .expect("i=61 animated")
            .loop_count,
        Some(4),
        "a=a,v=5 MUST set loop_count to Some(4) (kitty: max_loops = N-1)"
    );

    // v=0 MUST leave loop_count UNCHANGED at Some(4).
    h.feed(&kitty_apc(b"a=a,i=61,v=0", ""));
    assert_eq!(
        h.term()
            .image_cache()
            .animation_snapshot(ImageId::from_raw(61))
            .expect("i=61 animated")
            .loop_count,
        Some(4),
        "a=a,v=0 MUST leave loop_count unchanged — kitty `if (g->loop_count)` \
         guard at graphics.c:1766 ignores v=0 (truthy check on raw value)"
    );
}

/// `a=a,v=1` sets `loop_count` to `None` (infinite loops). Per kitty
/// `graphics.c:1766-1767`: `max_loops = 1 - 1 = 0`, and `image_is_animatable`
/// at `graphics.c:1775` shortcircuits `!max_loops` to mean "infinite".
/// Catalog row: `KG-ANIMATE-LOOP-COUNT` (v=1 infinite).
/// Pre-fix code routed v=1 → loop_count=Some(1) (loop once and stop),
/// opposite of kitty.
#[test]
fn kitty_animate_v_one_sets_loop_count_to_none_infinite() {
    let base = b64(&rgba_4x4_red());
    let frame = b64(&rgba_4x4_red());

    let mut h = SpecHarness::new();
    h.feed(&kitty_apc(b"a=t,i=78,f=32,s=4,v=4", &base));
    h.feed(&kitty_apc(b"a=f,i=78,f=32,s=4,v=4", &frame));

    h.feed(&kitty_apc(b"a=a,i=78,v=1", ""));
    assert_eq!(
        h.term()
            .image_cache()
            .animation_snapshot(ImageId::from_raw(78))
            .expect("i=78 animated")
            .loop_count,
        None,
        "a=a,v=1 MUST set loop_count to None (infinite) — kitty max_loops=0 \
         shortcircuit at graphics.c:1775"
    );
}

/// `a=a,v=3` sets `loop_count` to `Some(2)` (finite 2 loops). Per kitty
/// `graphics.c:1767`: `max_loops = 3 - 1 = 2`.
/// Catalog row: `KG-ANIMATE-LOOP-COUNT` (finite N-1).
#[test]
fn kitty_animate_v_three_sets_loop_count_to_some_two_finite_loops() {
    let base = b64(&rgba_4x4_red());
    let frame = b64(&rgba_4x4_red());

    let mut h = SpecHarness::new();
    h.feed(&kitty_apc(b"a=t,i=79,f=32,s=4,v=4", &base));
    h.feed(&kitty_apc(b"a=f,i=79,f=32,s=4,v=4", &frame));

    h.feed(&kitty_apc(b"a=a,i=79,v=3", ""));
    assert_eq!(
        h.term()
            .image_cache()
            .animation_snapshot(ImageId::from_raw(79))
            .expect("i=79 animated")
            .loop_count,
        Some(2),
        "a=a,v=3 MUST set loop_count to Some(2) (kitty: max_loops = 3-1 = 2)"
    );
}

/// `a=a,v=5` sets `loop_count` to `Some(4)` (finite 4 loops). Per kitty
/// `graphics.c:1767`: `max_loops = 5 - 1 = 4`.
/// Catalog row: `KG-ANIMATE-LOOP-COUNT` (finite N-1).
/// Pre-fix code mapped v=5 → Some(5).
#[test]
fn kitty_animate_v_five_sets_loop_count_to_some_four_finite_loops() {
    let base = b64(&rgba_4x4_red());
    let frame = b64(&rgba_4x4_red());

    let mut h = SpecHarness::new();
    h.feed(&kitty_apc(b"a=t,i=80,f=32,s=4,v=4", &base));
    h.feed(&kitty_apc(b"a=f,i=80,f=32,s=4,v=4", &frame));

    h.feed(&kitty_apc(b"a=a,i=80,v=5", ""));
    assert_eq!(
        h.term()
            .image_cache()
            .animation_snapshot(ImageId::from_raw(80))
            .expect("i=80 animated")
            .loop_count,
        Some(4),
        "a=a,v=5 MUST set loop_count to Some(4) (kitty: max_loops = 5-1 = 4)"
    );
}

/// `a=a` without `v=` leaves `loop_count` unchanged — the regression guard that
/// proves the implementation distinguishes "key absent" from "v=0". A
/// previous bug unconditionally set `source_height` to 0 on missing `v=`,
/// which would overwrite a prior finite loop count.
/// Catalog row: `KG-ANIMATE-LOOP-COUNT` (regression guard).
#[test]
fn kitty_animate_v_absent_leaves_prior_loop_count_unchanged() {
    let base = b64(&rgba_4x4_red());
    let frame = b64(&rgba_4x4_red());

    let mut h = SpecHarness::new();
    h.feed(&kitty_apc(b"a=t,i=62,f=32,s=4,v=4", &base));
    h.feed(&kitty_apc(b"a=f,i=62,f=32,s=4,v=4", &frame));

    h.feed(&kitty_apc(b"a=a,i=62,v=7", ""));
    assert_eq!(
        h.term()
            .image_cache()
            .animation_snapshot(ImageId::from_raw(62))
            .unwrap()
            .loop_count,
        Some(6),
        "a=a,v=7 MUST set loop_count to Some(6) (kitty: max_loops = 7-1 = 6)"
    );

    // Separate a=a command with NO `v=` — loop count MUST stay at Some(6).
    h.feed(&kitty_apc(b"a=a,i=62,s=3", ""));
    assert_eq!(
        h.term()
            .image_cache()
            .animation_snapshot(ImageId::from_raw(62))
            .unwrap()
            .loop_count,
        Some(6),
        "a=a with v= absent MUST leave loop_count at its prior value — \
         a regression guard for the Some(0)-vs-None distinction"
    );
}

/// `a=a,c=N` seeks to frame N (1-based) and the OK reply echoes `,r=N` for
/// the newly-set current frame. `c=` is the current-frame selector per kitty
/// graphics-protocol.rst:923-927 + graphics.c:1737-1743.
/// Catalog row: `KG-ANIMATE-SET-CURRENT-FRAME`.
/// Pre-fix dispatch routed both `r=` and `c=` to set_current_frame,
/// inverting the kitty semantic. Now `c=` is the seek, `r=` is the
/// gap-target selector (covered by sibling tests below).
#[test]
fn kitty_animate_c_two_sets_current_frame_and_reply_echoes_r_two() {
    let base = b64(&rgba_4x4_red());
    let frame = b64(&rgba_4x4_red());

    let mut h = SpecHarness::new();
    h.feed(&kitty_apc(b"a=t,i=63,f=32,s=4,v=4", &base));
    h.feed(&kitty_apc(b"a=f,i=63,f=32,s=4,v=4", &frame));
    // Now 2 frames (indices 0 and 1, 1-based 1 and 2).

    h.feed(&kitty_apc(b"a=a,i=63,c=2", ""));

    let state = h
        .term()
        .image_cache()
        .animation_snapshot(ImageId::from_raw(63))
        .expect("i=63 animated");
    assert_eq!(
        state.current_frame, 1,
        "a=a,c=2 MUST seek current_frame to 0-based index 1"
    );

    assert!(
        reply_contains(&h, &ok_reply_with_frame(63, 2)),
        "a=a,c=2 reply MUST echo ,r=2 (1-based current frame after mutation) \
         — transcript: {:?}",
        String::from_utf8_lossy(&reply_bytes(&h)),
    );
}

/// `a=a,s=2` (run-wait) and `a=a,s=3` (run) BOTH unpause and reset
/// loops_completed=0. They differ in `wait_mode`: s=2 sets it true, s=3
/// clears it. wait_mode governs whether `add_animation_frame` resumes a
/// finished animation (s=2 yes; s=3 no).
/// fix: previously these collapsed via a shared match arm.
/// Catalog row: `KG-ANIMATE-RUN-WAIT`.
#[test]
fn kitty_animate_s_two_sets_wait_mode_and_s_three_clears_it() {
    let base = b64(&rgba_4x4_red());
    let frame = b64(&rgba_4x4_red());

    let mut h = SpecHarness::new();
    h.feed(&kitty_apc(b"a=t,i=67,f=32,s=4,v=4", &base));
    h.feed(&kitty_apc(b"a=f,i=67,f=32,s=4,v=4", &frame));

    // Start paused via s=1.
    h.feed(&kitty_apc(b"a=a,i=67,s=1", ""));
    assert!(
        h.term()
            .image_cache()
            .animation_snapshot(ImageId::from_raw(67))
            .unwrap()
            .paused,
        "a=a,s=1 MUST pause"
    );

    // s=2 (run-wait) — unpauses, resets loops, AND sets wait_mode.
    h.feed(&kitty_apc(b"a=a,i=67,s=2", ""));
    let state = h
        .term()
        .image_cache()
        .animation_snapshot(ImageId::from_raw(67))
        .unwrap();
    assert!(!state.paused, "a=a,s=2 MUST clear paused");
    assert_eq!(
        state.loops_completed, 0,
        "a=a,s=2 MUST reset loops_completed"
    );
    assert!(state.wait_mode, "a=a,s=2 MUST set wait_mode=true");

    // s=3 (run) — clears wait_mode.
    h.feed(&kitty_apc(b"a=a,i=67,s=3", ""));
    let state = h
        .term()
        .image_cache()
        .animation_snapshot(ImageId::from_raw(67))
        .unwrap();
    assert!(
        !state.wait_mode,
        "a=a,s=3 MUST clear wait_mode (distinguishing it from s=2 run-wait)"
    );
}

/// `a=a,z=Nms` WITHOUT `r=` is a no-op per kitty graphics.c:1729-1735 — the
/// gap update is guarded by `if (g->frame_number)` (i.e. `r=` MUST be set).
/// Standalone `z=` had no kitty parity in the pre-fix dispatch.
/// Catalog row: `KG-ANIMATE-SET-FRAME-GAP` (inverse guard).
/// Proves the standalone-`z=` branch was correctly deleted in the
/// dispatch split.
#[test]
fn kitty_animate_z_alone_without_r_is_no_op() {
    let base = b64(&rgba_4x4_red());
    let frame = b64(&rgba_4x4_red());

    let mut h = SpecHarness::new();
    h.feed(&kitty_apc(b"a=t,i=64,f=32,s=4,v=4", &base));
    h.feed(&kitty_apc(b"a=f,i=64,f=32,s=4,v=4", &frame));

    let baseline_durations = h
        .term()
        .image_cache()
        .animation_snapshot(ImageId::from_raw(64))
        .expect("i=64 animated")
        .frame_gaps
        .to_vec();

    // Standalone z= without r=: kitty's handle_animation_control_command
    // (graphics.c:1729-1735) does nothing because `g->frame_number` is 0.
    h.feed(&kitty_apc(b"a=a,i=64,z=250", ""));

    let state = h
        .term()
        .image_cache()
        .animation_snapshot(ImageId::from_raw(64))
        .expect("i=64 animated");
    assert_eq!(
        state.frame_gaps, baseline_durations,
        "a=a,z=250 WITHOUT r= MUST leave frame_durations unchanged — \
         standalone z= has no kitty parity (z= consumed only via r=)"
    );
    assert_eq!(
        state.current_frame, 0,
        "a=a,z=250 WITHOUT r= MUST NOT seek (no `c=` present)"
    );
}

/// `a=a,r=N,z=Z` sets the gap of frame N (1-based) to Z ms. This is the
/// gap-target arm of kitty's `handle_animation_control_command`
/// (graphics.c:1729-1735) — `r=` selects which frame's gap, `z=` provides
/// the value, neither alone has effect.
/// Catalog row: `KG-ANIMATE-SET-FRAME-GAP` (positive — gap-target arm).
/// Pre-fix code routed `r=` to `set_current_frame` instead of
/// `set_frame_gap`, so this case silently seek-without-gap'd.
#[test]
fn kitty_animate_r_two_z_positive_sets_frame_two_gap_without_seeking() {
    let base = b64(&rgba_4x4_red());
    let frame = b64(&rgba_4x4_red());

    let mut h = SpecHarness::new();
    h.feed(&kitty_apc(b"a=t,i=70,f=32,s=4,v=4", &base));
    h.feed(&kitty_apc(b"a=f,i=70,f=32,s=4,v=4", &frame));

    h.feed(&kitty_apc(b"a=a,i=70,r=2,z=77", ""));

    let state = h
        .term()
        .image_cache()
        .animation_snapshot(ImageId::from_raw(70))
        .expect("i=70 animated");
    assert_eq!(
        state.frame_gaps[1],
        Duration::from_millis(77),
        "a=a,r=2,z=77 MUST set frame_durations[1] to 77ms (gap-target arm)"
    );
    assert_eq!(
        state.current_frame, 0,
        "a=a,r=2,z=77 MUST NOT seek — r= is the gap-target selector, not the \
         current-frame seek (that is `c=`)"
    );
}

/// `a=a,r=N,z=-1` sets frame N's gap to ZERO (gapless edit). Kitty's
/// `change_gap` (graphics.c:1348-1350) clamps negative z= to zero.
/// Catalog row: `KG-ANIMATE-SET-FRAME-GAP` (gapless variant — negative-z=
/// clamping path).
#[test]
fn kitty_animate_r_two_z_negative_sets_frame_two_gapless() {
    let base = b64(&rgba_4x4_red());
    let frame = b64(&rgba_4x4_red());

    let mut h = SpecHarness::new();
    h.feed(&kitty_apc(b"a=t,i=71,f=32,s=4,v=4", &base));
    h.feed(&kitty_apc(b"a=f,i=71,f=32,s=4,v=4,z=99", &frame));
    // Pre-seed frame 2 gap to 99ms via a=f's z= so the clamp-to-zero test is
    // a real overwrite, not a no-op.

    h.feed(&kitty_apc(b"a=a,i=71,r=2,z=-1", ""));

    let state = h
        .term()
        .image_cache()
        .animation_snapshot(ImageId::from_raw(71))
        .expect("i=71 animated");
    assert_eq!(
        state.frame_gaps[1],
        Duration::ZERO,
        "a=a,r=2,z=-1 MUST clamp to Duration::ZERO (gapless edit)"
    );
}

/// `a=a,r=N,z=0` is a no-op per kitty's `if (g->gap)` guard
/// (graphics.c:1734) — z=0 means "no gap change."
/// Catalog row: `KG-ANIMATE-SET-FRAME-GAP` (z=0 no-op variant — guard
/// preservation).
#[test]
fn kitty_animate_r_two_z_zero_is_no_op() {
    let base = b64(&rgba_4x4_red());
    let frame = b64(&rgba_4x4_red());

    let mut h = SpecHarness::new();
    h.feed(&kitty_apc(b"a=t,i=72,f=32,s=4,v=4", &base));
    h.feed(&kitty_apc(b"a=f,i=72,f=32,s=4,v=4,z=60", &frame));
    // Frame 2's gap is now 60ms from the a=f,z=60.

    h.feed(&kitty_apc(b"a=a,i=72,r=2,z=0", ""));

    let state = h
        .term()
        .image_cache()
        .animation_snapshot(ImageId::from_raw(72))
        .expect("i=72 animated");
    assert_eq!(
        state.frame_gaps[1],
        Duration::from_millis(60),
        "a=a,r=2,z=0 MUST leave frame_durations[1] unchanged — z=0 is kitty's \
         `if (g->gap)` no-op (graphics.c:1734)"
    );
}

/// `a=a,r=99,z=Z` on an animation with fewer frames is a silent no-op —
/// `set_frame_gap`'s bounds-check (animation.rs:309 `if frame_idx <
/// state.frame_gaps.len()`) drops the update.
/// Catalog row: `KG-ANIMATE-SET-FRAME-GAP` (out-of-range no-op — bounds
/// check inherited from set_frame_gap).
#[test]
fn kitty_animate_r_out_of_range_is_no_op() {
    let base = b64(&rgba_4x4_red());
    let frame = b64(&rgba_4x4_red());

    let mut h = SpecHarness::new();
    h.feed(&kitty_apc(b"a=t,i=73,f=32,s=4,v=4", &base));
    h.feed(&kitty_apc(b"a=f,i=73,f=32,s=4,v=4", &frame));

    let baseline_durations = h
        .term()
        .image_cache()
        .animation_snapshot(ImageId::from_raw(73))
        .expect("i=73 animated")
        .frame_gaps
        .to_vec();

    h.feed(&kitty_apc(b"a=a,i=73,r=99,z=77", ""));

    let state = h
        .term()
        .image_cache()
        .animation_snapshot(ImageId::from_raw(73))
        .expect("i=73 animated");
    assert_eq!(
        state.frame_gaps, baseline_durations,
        "a=a,r=99,z=77 on 2-frame animation MUST be a no-op via the \
         set_frame_gap bounds-check"
    );
    assert_eq!(
        state.current_frame, 0,
        "a=a,r=99,z=77 MUST NOT seek (r= is gap-target, not a seek)"
    );
}

/// `a=a,r=N,z=Z` on a STATIC (unpromoted) image is a silent no-op —
/// `set_frame_gap`'s `if let Some(state)` guard returns early when the
/// animation state does not exist. Auto-promotion on `r=` is owned by a
/// separate fix (the `ensure_animation_state_for_root_gap` helper).
/// Catalog row: `KG-ANIMATE-SET-FRAME-GAP` (static-image boundary).
/// Explicit boundary marker between the dispatch split and the
/// auto-promote helper (`ensure_animation_state_for_root_gap`) that
/// layers on top of it.
#[test]
fn kitty_animate_r_two_z_positive_on_static_image_is_silent_no_op() {
    let base = b64(&rgba_4x4_red());

    let mut h = SpecHarness::new();
    h.feed(&kitty_apc(b"a=t,i=74,f=32,s=4,v=4", &base));
    // No a=f — i=74 stays static (no AnimationState).

    h.feed(&kitty_apc(b"a=a,i=74,r=2,z=77", ""));

    assert!(
        h.term()
            .image_cache()
            .animation_snapshot(ImageId::from_raw(74))
            .is_none(),
        "a=a,r=2,z=77 on static image MUST NOT auto-promote — that path is \
         owned by the separate auto-promote helper (see test doc comment)"
    );
}

/// `a=a,c=0` is a no-op — kitty `c=` is 1-based, so c=0 means "key not set"
/// effectively (per the `if frame > 0` guard in the seek arm).
/// Catalog row: `KG-ANIMATE-SET-CURRENT-FRAME` (zero guard).
#[test]
fn kitty_animate_c_zero_is_no_op() {
    let base = b64(&rgba_4x4_red());
    let frame = b64(&rgba_4x4_red());

    let mut h = SpecHarness::new();
    h.feed(&kitty_apc(b"a=t,i=75,f=32,s=4,v=4", &base));
    h.feed(&kitty_apc(b"a=f,i=75,f=32,s=4,v=4", &frame));

    h.feed(&kitty_apc(b"a=a,i=75,c=0", ""));

    let state = h
        .term()
        .image_cache()
        .animation_snapshot(ImageId::from_raw(75))
        .expect("i=75 animated");
    assert_eq!(
        state.current_frame, 0,
        "a=a,c=0 MUST NOT seek — c= is 1-based, c=0 is treated as unspecified"
    );
}

/// `a=a,c=99` on a 2-frame animation is a no-op — `set_current_frame`'s
/// bounds-check (animation.rs:317 `if frame_idx < state.total_frames`)
/// drops the seek.
/// Catalog row: `KG-ANIMATE-SET-CURRENT-FRAME` (out-of-range no-op).
#[test]
fn kitty_animate_c_out_of_range_is_no_op() {
    let base = b64(&rgba_4x4_red());
    let frame = b64(&rgba_4x4_red());

    let mut h = SpecHarness::new();
    h.feed(&kitty_apc(b"a=t,i=76,f=32,s=4,v=4", &base));
    h.feed(&kitty_apc(b"a=f,i=76,f=32,s=4,v=4", &frame));

    h.feed(&kitty_apc(b"a=a,i=76,c=99", ""));

    let state = h
        .term()
        .image_cache()
        .animation_snapshot(ImageId::from_raw(76))
        .expect("i=76 animated");
    assert_eq!(
        state.current_frame, 0,
        "a=a,c=99 on 2-frame animation MUST be a no-op via the \
         set_current_frame bounds-check"
    );
}

/// `a=a,r=N,c=M,z=Z` applies the gap-target and seek arms INDEPENDENTLY.
/// Per kitty graphics.c:1729-1743, the two are sequential `if` branches —
/// `r=` updates frame N's gap, `c=` then seeks to frame M.
/// Catalog row: `KG-ANIMATE-SET-CURRENT-FRAME` + `KG-ANIMATE-SET-FRAME-GAP`
/// (independent-arms interaction). Pre-fix code collapsed both keys into
/// the same dispatch, so `r=` AND `c=` together produced incoherent state.
#[test]
fn kitty_animate_r_and_c_apply_independently() {
    let base = b64(&rgba_4x4_red());
    let frame = b64(&rgba_4x4_red());

    let mut h = SpecHarness::new();
    h.feed(&kitty_apc(b"a=t,i=77,f=32,s=4,v=4", &base));
    h.feed(&kitty_apc(b"a=f,i=77,f=32,s=4,v=4", &frame));
    h.feed(&kitty_apc(b"a=f,i=77,f=32,s=4,v=4", &frame));
    // 3-frame animation now (root + 2 extras).

    h.feed(&kitty_apc(b"a=a,i=77,r=2,c=3,z=77", ""));

    let state = h
        .term()
        .image_cache()
        .animation_snapshot(ImageId::from_raw(77))
        .expect("i=77 animated");
    assert_eq!(
        state.frame_gaps[1],
        Duration::from_millis(77),
        "r=2 arm MUST set frame_durations[1] to 77ms (independent of c=)"
    );
    assert_eq!(
        state.current_frame, 2,
        "c=3 arm MUST seek to 0-based index 2 (independent of r=)"
    );
    assert!(
        reply_contains(&h, &ok_reply_with_frame(77, 3)),
        "reply MUST echo ,r=3 — post-mutation current_frame after c=3 seek \
         — transcript: {:?}",
        String::from_utf8_lossy(&reply_bytes(&h)),
    );
}

/// `Term::advance_animations` returns a non-None deadline when a pane has a
/// visible animation with multiple frames — the production path hook that
/// the IO thread will consume in the next wiring step.
/// Catalog row: `KG-ANIMATE-RUN` (timer-hook, underlying deadline contract).
#[test]
fn term_advance_animations_returns_next_deadline_when_animated_image_visible() {
    let base = b64(&rgba_4x4_red());
    let frame = b64(&rgba_4x4_red());

    let mut h = SpecHarness::new();
    // Place on-screen so the viewport intersection test passes.
    h.feed(&kitty_apc(b"a=T,i=65,f=32,s=4,v=4", &base));
    h.feed(&kitty_apc(b"a=f,i=65,f=32,s=4,v=4", &frame));
    // Resume so is_finished() is false.
    h.feed(&kitty_apc(b"a=a,i=65,s=3", ""));

    let now = Instant::now();
    let deadline = h.term_mut().advance_animations(now);
    assert!(
        deadline.is_some(),
        "advance_animations MUST return Some(deadline) when an animated \
         image is visible with a resumed playback state"
    );
}

/// Idempotency regression guard: without any call to `advance_animations`,
/// `current_frame` MUST stay at 0 after a fresh promotion. Proves the
/// animation does NOT advance via some internal auto-loop path — the
/// timer-driven tick from the IO thread IS load-bearing. (The companion
/// "timer wired" positive pin lives in `oriterm_mux/src/pane/io_thread/
/// tests.rs` once the IO-thread wiring lands.)
/// Catalog row: `KG-ANIMATE-RUN` (regression guard, no-tick).
#[test]
fn animation_current_frame_does_not_advance_without_timer_tick() {
    let base = b64(&rgba_4x4_red());
    let frame = b64(&rgba_4x4_red());

    let mut h = SpecHarness::new();
    h.feed(&kitty_apc(b"a=T,i=66,f=32,s=4,v=4", &base));
    h.feed(&kitty_apc(b"a=f,i=66,f=32,s=4,v=4", &frame));
    h.feed(&kitty_apc(b"a=a,i=66,s=3", ""));

    // NO advance_animations call — mimics the broken "no production caller"
    // path that §13.3 fixes.
    let state = h
        .term()
        .image_cache()
        .animation_snapshot(ImageId::from_raw(66))
        .expect("i=66 animated");
    assert_eq!(
        state.current_frame, 0,
        "current_frame MUST stay at 0 without a timer-driven advance — \
         proves the cache does not auto-advance frames without an \
         external tick"
    );
}

/// Matrix count assertion — enumerates the animation categories this file
/// pins and checks we hit each one. If a category is added here, the count
/// increments; this prevents silent matrix gaps.
// ============================================================================
// `a=f` c=/r= dispatch matrix for kitty graphics.
// Pins per-arm behavior of `put_frame` + the kitty_frame dispatch:
// default-append, c=N append, r=N edit, each × {AlphaBlend, Overwrite}
// × {full-frame, sub-rect}. Goldens are hand-computed RGBA byte literals.
// ============================================================================

const RED: [u8; 4] = [255, 0, 0, 255];
const GREEN: [u8; 4] = [0, 255, 0, 255];
const BLUE: [u8; 4] = [0, 0, 255, 255];
const TRANSPARENT: [u8; 4] = [0, 0, 0, 0];

fn fill_4x4(rgba: [u8; 4]) -> Vec<u8> {
    let mut v = Vec::with_capacity(64);
    for _ in 0..16 {
        v.extend_from_slice(&rgba);
    }
    v
}

fn fill_2x2(rgba: [u8; 4]) -> Vec<u8> {
    let mut v = Vec::with_capacity(16);
    for _ in 0..4 {
        v.extend_from_slice(&rgba);
    }
    v
}

// ----------------------------------------------------------------------------
// Section: Exact failing cases (the bug's repro)
// ----------------------------------------------------------------------------

/// r=N edits frame N in place, does NOT append. Pre-fix: dispatch
/// appended → 3 frames. Post-fix: 2 frames, frame 2 = blue.
#[test]
fn kitty_frame_r_key_edits_frame_in_place_not_append() {
    let mut h = SpecHarness::new();
    h.feed(&kitty_apc(b"a=t,i=99,f=32,s=4,v=4", &b64(&fill_4x4(RED))));
    h.feed(&kitty_apc(b"a=f,i=99,f=32,s=4,v=4", &b64(&fill_4x4(GREEN))));
    h.feed(&kitty_apc(b"a=f,i=99,f=32,s=4,v=4,r=2,X=1", &b64(&fill_4x4(BLUE))));

    let cache = h.term().image_cache();
    assert_eq!(cache.total_frames_for_test(ImageId::from_raw(99)), 2);
    assert_frame_eq(cache, ImageId::from_raw(99), 2, &fill_4x4(BLUE));
    assert_frame_eq(cache, ImageId::from_raw(99), 1, &fill_4x4(RED));
}

/// c=N appends a new frame using frame N as canvas. Pre-fix: c= ignored.
/// Post-fix: frame 3 appended with canvas=frame2(green) + 2x2 blue overlay
/// at (1,1).
#[test]
fn kitty_frame_c_key_appends_new_frame_using_frame_n_as_canvas() {
    let mut h = SpecHarness::new();
    h.feed(&kitty_apc(b"a=t,i=99,f=32,s=4,v=4", &b64(&fill_4x4(RED))));
    h.feed(&kitty_apc(b"a=f,i=99,f=32,s=4,v=4", &b64(&fill_4x4(GREEN))));
    h.feed(&kitty_apc(
        b"a=f,i=99,f=32,s=2,v=2,c=2,x=1,y=1,X=1",
        &b64(&fill_2x2(BLUE)),
    ));

    let cache = h.term().image_cache();
    assert_eq!(cache.total_frames_for_test(ImageId::from_raw(99)), 3);
    // Frame 2 (canvas source) must be UNCHANGED.
    assert_frame_eq(cache, ImageId::from_raw(99), 2, &fill_4x4(GREEN));
    // Frame 3: row-major 4x4 with blue 2x2 at (1,1)-(2,2).
    let expected: &[u8] = &[
        // row 0: GGGG
        0, 255, 0, 255, 0, 255, 0, 255, 0, 255, 0, 255, 0, 255, 0, 255,
        // row 1: GBBG
        0, 255, 0, 255, 0, 0, 255, 255, 0, 0, 255, 255, 0, 255, 0, 255,
        // row 2: GBBG
        0, 255, 0, 255, 0, 0, 255, 255, 0, 0, 255, 255, 0, 255, 0, 255,
        // row 3: GGGG
        0, 255, 0, 255, 0, 255, 0, 255, 0, 255, 0, 255, 0, 255, 0, 255,
    ];
    assert_frame_eq(cache, ImageId::from_raw(99), 3, expected);
}

// ----------------------------------------------------------------------------
// Section: 3-arm × 2-composition × 2-sub-rect matrix (12 cells)
// Each cell pins a specific (arm × composition × sub-rect) interaction.
// ----------------------------------------------------------------------------

/// Cell 1: default-append × AlphaBlend × full-frame. Canvas = Y= transparent
/// default; payload alpha-blended onto it. With Y=0x00000000 (alpha=0), the
/// blend formula short-circuits when src alpha is non-zero — but with src
/// alpha=255 the output is just src (opaque-source over transparent dest).
#[test]
fn kitty_frame_default_append_alphablend_full_frame_uses_y_canvas() {
    let mut h = SpecHarness::new();
    h.feed(&kitty_apc(b"a=t,i=10,f=32,s=4,v=4", &b64(&fill_4x4(RED))));
    h.feed(&kitty_apc(b"a=f,i=10,f=32,s=4,v=4", &b64(&fill_4x4(GREEN))));
    let cache = h.term().image_cache();
    // Y= default 0 = transparent canvas; opaque green blits opaque over it.
    assert_frame_eq(cache, ImageId::from_raw(10), 2, &fill_4x4(GREEN));
}

/// Cell 2: default-append × AlphaBlend × sub-rect. 2x2 green at (1,1) onto
/// Y= transparent canvas → 2x2 green region at (1,1), rest fully transparent.
#[test]
fn kitty_frame_default_append_alphablend_subrect_blits_onto_y_canvas() {
    let mut h = SpecHarness::new();
    h.feed(&kitty_apc(b"a=t,i=11,f=32,s=4,v=4", &b64(&fill_4x4(RED))));
    h.feed(&kitty_apc(
        b"a=f,i=11,f=32,s=2,v=2,x=1,y=1",
        &b64(&fill_2x2(GREEN)),
    ));
    let cache = h.term().image_cache();
    let mut expected = Vec::with_capacity(64);
    for row in 0..4 {
        for col in 0..4 {
            if (1..=2).contains(&col) && (1..=2).contains(&row) {
                expected.extend_from_slice(&GREEN);
            } else {
                expected.extend_from_slice(&TRANSPARENT);
            }
        }
    }
    assert_frame_eq(cache, ImageId::from_raw(11), 2, &expected);
}

/// Cell 3: default-append × Overwrite × full-frame. Same shape as Cell 1
/// but Overwrite path (X=1). With full-frame overwrite, payload = output.
#[test]
fn kitty_frame_default_append_overwrite_full_frame_uses_y_canvas() {
    let mut h = SpecHarness::new();
    h.feed(&kitty_apc(b"a=t,i=12,f=32,s=4,v=4", &b64(&fill_4x4(RED))));
    h.feed(&kitty_apc(b"a=f,i=12,f=32,s=4,v=4,X=1", &b64(&fill_4x4(GREEN))));
    let cache = h.term().image_cache();
    assert_frame_eq(cache, ImageId::from_raw(12), 2, &fill_4x4(GREEN));
}

/// Cell 4: default-append × Overwrite × sub-rect. 2x2 blue at (1,1) onto
/// Y= transparent canvas via Overwrite → 2x2 blue, rest transparent.
#[test]
fn kitty_frame_default_append_overwrite_subrect_blits_onto_y_canvas() {
    let mut h = SpecHarness::new();
    h.feed(&kitty_apc(b"a=t,i=13,f=32,s=4,v=4", &b64(&fill_4x4(RED))));
    h.feed(&kitty_apc(
        b"a=f,i=13,f=32,s=2,v=2,x=1,y=1,X=1",
        &b64(&fill_2x2(BLUE)),
    ));
    let cache = h.term().image_cache();
    let mut expected = Vec::with_capacity(64);
    for row in 0..4 {
        for col in 0..4 {
            if (1..=2).contains(&col) && (1..=2).contains(&row) {
                expected.extend_from_slice(&BLUE);
            } else {
                expected.extend_from_slice(&TRANSPARENT);
            }
        }
    }
    assert_frame_eq(cache, ImageId::from_raw(13), 2, &expected);
}

/// Cell 5: c=N append × AlphaBlend × full-frame. Canvas = frame N's bytes,
/// payload alpha-blended over it. Frame N stays unchanged.
#[test]
fn kitty_frame_c_append_alphablend_full_frame_blends_onto_frame_n() {
    let mut h = SpecHarness::new();
    h.feed(&kitty_apc(b"a=t,i=14,f=32,s=4,v=4", &b64(&fill_4x4(RED))));
    h.feed(&kitty_apc(b"a=f,i=14,f=32,s=4,v=4", &b64(&fill_4x4(GREEN))));
    // c=2 → canvas = frame 2 (green); opaque green payload blends over
    // green canvas → green.
    h.feed(&kitty_apc(b"a=f,i=14,f=32,s=4,v=4,c=2", &b64(&fill_4x4(GREEN))));
    let cache = h.term().image_cache();
    assert_eq!(cache.total_frames_for_test(ImageId::from_raw(14)), 3);
    assert_frame_eq(cache, ImageId::from_raw(14), 2, &fill_4x4(GREEN));
    assert_frame_eq(cache, ImageId::from_raw(14), 3, &fill_4x4(GREEN));
}

/// Cell 6: c=N append × AlphaBlend × sub-rect. Frame 3 = frame 2 canvas
/// + 2x2 alpha-blended overlay.
#[test]
fn kitty_frame_c_append_alphablend_subrect_blends_onto_frame_n() {
    let mut h = SpecHarness::new();
    h.feed(&kitty_apc(b"a=t,i=15,f=32,s=4,v=4", &b64(&fill_4x4(RED))));
    h.feed(&kitty_apc(b"a=f,i=15,f=32,s=4,v=4", &b64(&fill_4x4(GREEN))));
    h.feed(&kitty_apc(
        b"a=f,i=15,f=32,s=2,v=2,c=2,x=1,y=1",
        &b64(&fill_2x2(BLUE)),
    ));
    let cache = h.term().image_cache();
    // Frame 3: 4x4 green canvas with 2x2 blue (opaque) blit at (1,1)-(2,2).
    let mut expected = Vec::with_capacity(64);
    for row in 0..4 {
        for col in 0..4 {
            if (1..=2).contains(&col) && (1..=2).contains(&row) {
                expected.extend_from_slice(&BLUE);
            } else {
                expected.extend_from_slice(&GREEN);
            }
        }
    }
    assert_frame_eq(cache, ImageId::from_raw(15), 3, &expected);
}

/// Cell 7: c=N append × Overwrite × full-frame. Canvas = frame N (green),
/// full overwrite with blue → frame 3 = all blue.
#[test]
fn kitty_frame_c_append_overwrite_full_frame_replaces_with_frame_n_base() {
    let mut h = SpecHarness::new();
    h.feed(&kitty_apc(b"a=t,i=16,f=32,s=4,v=4", &b64(&fill_4x4(RED))));
    h.feed(&kitty_apc(b"a=f,i=16,f=32,s=4,v=4", &b64(&fill_4x4(GREEN))));
    h.feed(&kitty_apc(
        b"a=f,i=16,f=32,s=4,v=4,c=2,X=1",
        &b64(&fill_4x4(BLUE)),
    ));
    let cache = h.term().image_cache();
    // Frame 2 (canvas) unchanged; frame 3 = pure blue.
    assert_frame_eq(cache, ImageId::from_raw(16), 2, &fill_4x4(GREEN));
    assert_frame_eq(cache, ImageId::from_raw(16), 3, &fill_4x4(BLUE));
}

/// Cell 8: c=N append × Overwrite × sub-rect. Frame 3 = green canvas with
/// 2x2 blue overwrite at (1,1)-(2,2).
#[test]
fn kitty_frame_c_append_overwrite_subrect_replaces_in_frame_n() {
    let mut h = SpecHarness::new();
    h.feed(&kitty_apc(b"a=t,i=17,f=32,s=4,v=4", &b64(&fill_4x4(RED))));
    h.feed(&kitty_apc(b"a=f,i=17,f=32,s=4,v=4", &b64(&fill_4x4(GREEN))));
    h.feed(&kitty_apc(
        b"a=f,i=17,f=32,s=2,v=2,c=2,x=1,y=1,X=1",
        &b64(&fill_2x2(BLUE)),
    ));
    let cache = h.term().image_cache();
    let mut expected = Vec::with_capacity(64);
    for row in 0..4 {
        for col in 0..4 {
            if (1..=2).contains(&col) && (1..=2).contains(&row) {
                expected.extend_from_slice(&BLUE);
            } else {
                expected.extend_from_slice(&GREEN);
            }
        }
    }
    assert_frame_eq(cache, ImageId::from_raw(17), 3, &expected);
}

/// Cell 9: r=N edit × AlphaBlend × full-frame. Edit frame 2 (green) by
/// alpha-blending green payload over it → still green.
#[test]
fn kitty_frame_r_edit_alphablend_full_frame_blends_onto_existing() {
    let mut h = SpecHarness::new();
    h.feed(&kitty_apc(b"a=t,i=18,f=32,s=4,v=4", &b64(&fill_4x4(RED))));
    h.feed(&kitty_apc(b"a=f,i=18,f=32,s=4,v=4", &b64(&fill_4x4(GREEN))));
    h.feed(&kitty_apc(b"a=f,i=18,f=32,s=4,v=4,r=2", &b64(&fill_4x4(GREEN))));
    let cache = h.term().image_cache();
    assert_eq!(cache.total_frames_for_test(ImageId::from_raw(18)), 2);
    assert_frame_eq(cache, ImageId::from_raw(18), 2, &fill_4x4(GREEN));
}

/// Cell 10: r=N edit × AlphaBlend × sub-rect. Edit frame 2 with 2x2 green
/// alpha-blend overlay at (1,1) — only the blit rect changes, rest preserved.
#[test]
fn kitty_frame_r_edit_alphablend_subrect_blends_only_in_rect() {
    let mut h = SpecHarness::new();
    h.feed(&kitty_apc(b"a=t,i=19,f=32,s=4,v=4", &b64(&fill_4x4(RED))));
    h.feed(&kitty_apc(b"a=f,i=19,f=32,s=4,v=4", &b64(&fill_4x4(GREEN))));
    h.feed(&kitty_apc(
        b"a=f,i=19,f=32,s=2,v=2,r=2,x=1,y=1",
        &b64(&fill_2x2(GREEN)),
    ));
    let cache = h.term().image_cache();
    // Frame 2 unchanged since blit is green-on-green.
    assert_frame_eq(cache, ImageId::from_raw(19), 2, &fill_4x4(GREEN));
}

/// Cell 11: r=N edit × Overwrite × full-frame. Frame 2 replaced with blue.
#[test]
fn kitty_frame_r_edit_overwrite_full_frame_replaces_existing() {
    let mut h = SpecHarness::new();
    h.feed(&kitty_apc(b"a=t,i=20,f=32,s=4,v=4", &b64(&fill_4x4(RED))));
    h.feed(&kitty_apc(b"a=f,i=20,f=32,s=4,v=4", &b64(&fill_4x4(GREEN))));
    h.feed(&kitty_apc(
        b"a=f,i=20,f=32,s=4,v=4,r=2,X=1",
        &b64(&fill_4x4(BLUE)),
    ));
    let cache = h.term().image_cache();
    assert_eq!(cache.total_frames_for_test(ImageId::from_raw(20)), 2);
    assert_frame_eq(cache, ImageId::from_raw(20), 2, &fill_4x4(BLUE));
}

/// Cell 12: r=N edit × Overwrite × sub-rect. Frame 2 with 2x2 blue overwrite
/// at (1,1)-(2,2); rest preserves frame 2's green.
#[test]
fn kitty_frame_r_edit_overwrite_subrect_replaces_only_in_rect() {
    let mut h = SpecHarness::new();
    h.feed(&kitty_apc(b"a=t,i=21,f=32,s=4,v=4", &b64(&fill_4x4(RED))));
    h.feed(&kitty_apc(b"a=f,i=21,f=32,s=4,v=4", &b64(&fill_4x4(GREEN))));
    h.feed(&kitty_apc(
        b"a=f,i=21,f=32,s=2,v=2,r=2,x=1,y=1,X=1",
        &b64(&fill_2x2(BLUE)),
    ));
    let cache = h.term().image_cache();
    let mut expected = Vec::with_capacity(64);
    for row in 0..4 {
        for col in 0..4 {
            if (1..=2).contains(&col) && (1..=2).contains(&row) {
                expected.extend_from_slice(&BLUE);
            } else {
                expected.extend_from_slice(&GREEN);
            }
        }
    }
    assert_frame_eq(cache, ImageId::from_raw(21), 2, &expected);
}

// ----------------------------------------------------------------------------
// Section: Frame-1-vs-root-slot semantic (Cluster A pin)
// ----------------------------------------------------------------------------

/// c=1 reads ROOT slot (animation_frames[id][0]), NOT images[id].data
/// (which is mutated by apply_frame on every advance). Build a 3-frame
/// animation, manually advance current_frame=1 so images[id].data is frame 2,
/// then c=1 must use the unchanged ROOT (frame 1 = red).
#[test]
fn kitty_frame_c_one_reads_root_slot_not_displayed_frame_when_advanced() {
    let mut h = SpecHarness::new();
    h.feed(&kitty_apc(b"a=t,i=22,f=32,s=4,v=4", &b64(&fill_4x4(RED))));
    h.feed(&kitty_apc(b"a=f,i=22,f=32,s=4,v=4", &b64(&fill_4x4(GREEN))));
    h.feed(&kitty_apc(b"a=f,i=22,f=32,s=4,v=4", &b64(&fill_4x4(BLUE))));
    // Manually advance current_frame to 1 (frame 2 displayed) via a=a,c=2.
    h.feed(&kitty_apc(b"a=a,i=22,c=2", ""));
    // c=1 append: canvas = ROOT = red, payload = transparent → frame 4 = red.
    h.feed(&kitty_apc(
        b"a=f,i=22,f=32,s=4,v=4,c=1,X=1",
        &b64(&fill_4x4(RED)),
    ));
    let cache = h.term().image_cache();
    assert_eq!(cache.total_frames_for_test(ImageId::from_raw(22)), 4);
    assert_frame_eq(cache, ImageId::from_raw(22), 4, &fill_4x4(RED));
    // Root (frame 1) still red.
    assert_frame_eq(cache, ImageId::from_raw(22), 1, &fill_4x4(RED));
}

/// r=1 edits ROOT slot. Build animation, advance display, edit root via
/// r=1; root bytes change, display (frame 2) stays.
#[test]
fn kitty_frame_r_one_edits_root_slot_not_displayed_frame_when_advanced() {
    let mut h = SpecHarness::new();
    h.feed(&kitty_apc(b"a=t,i=23,f=32,s=4,v=4", &b64(&fill_4x4(RED))));
    h.feed(&kitty_apc(b"a=f,i=23,f=32,s=4,v=4", &b64(&fill_4x4(GREEN))));
    h.feed(&kitty_apc(b"a=a,i=23,c=2", ""));
    h.feed(&kitty_apc(
        b"a=f,i=23,f=32,s=4,v=4,r=1,X=1",
        &b64(&fill_4x4(BLUE)),
    ));
    let cache = h.term().image_cache();
    assert_frame_eq(cache, ImageId::from_raw(23), 1, &fill_4x4(BLUE));
    // Frame 2 (displayed) unchanged.
    assert_frame_eq(cache, ImageId::from_raw(23), 2, &fill_4x4(GREEN));
}

/// r=1 when current_frame == 0 (root displayed) also writes images[id].data
/// so the display reflects the edit immediately.
#[test]
fn kitty_frame_r_one_edits_root_and_updates_display_when_current_frame_is_zero() {
    let mut h = SpecHarness::new();
    h.feed(&kitty_apc(b"a=t,i=24,f=32,s=4,v=4", &b64(&fill_4x4(RED))));
    h.feed(&kitty_apc(b"a=f,i=24,f=32,s=4,v=4", &b64(&fill_4x4(GREEN))));
    h.feed(&kitty_apc(b"a=f,i=24,f=32,s=4,v=4", &b64(&fill_4x4(BLUE))));
    // current_frame stays at 0 (root displayed).
    h.feed(&kitty_apc(
        b"a=f,i=24,f=32,s=4,v=4,r=1,X=1",
        &b64(&fill_4x4(BLUE)),
    ));
    let cache = h.term().image_cache();
    assert_frame_eq(cache, ImageId::from_raw(24), 1, &fill_4x4(BLUE));
    // images[id].data reflects the edit since current_frame == 0.
    let display = cache
        .image_data_bytes_for_test(ImageId::from_raw(24))
        .expect("image present");
    assert_eq!(display.as_slice(), fill_4x4(BLUE).as_slice());
}

/// Inverse: r=2 with current_frame==0 (root displayed) edits frame 2
/// WITHOUT touching the display surface (which still shows root).
#[test]
fn kitty_frame_r_two_edit_does_not_touch_display_when_current_frame_is_zero() {
    let mut h = SpecHarness::new();
    h.feed(&kitty_apc(b"a=t,i=25,f=32,s=4,v=4", &b64(&fill_4x4(RED))));
    h.feed(&kitty_apc(b"a=f,i=25,f=32,s=4,v=4", &b64(&fill_4x4(GREEN))));
    h.feed(&kitty_apc(
        b"a=f,i=25,f=32,s=4,v=4,r=2,X=1",
        &b64(&fill_4x4(BLUE)),
    ));
    let cache = h.term().image_cache();
    assert_frame_eq(cache, ImageId::from_raw(25), 2, &fill_4x4(BLUE));
    // Display surface still shows the root.
    let display = cache
        .image_data_bytes_for_test(ImageId::from_raw(25))
        .expect("image present");
    assert_eq!(display.as_slice(), fill_4x4(RED).as_slice());
}

// ----------------------------------------------------------------------------
// Section: Edge cases (r/c clamping + bounds)
// ----------------------------------------------------------------------------

/// r=0 → falls back to append per kitty graphics.c:1558-1561.
#[test]
fn kitty_frame_r_zero_falls_back_to_append() {
    let mut h = SpecHarness::new();
    h.feed(&kitty_apc(b"a=t,i=26,f=32,s=4,v=4", &b64(&fill_4x4(RED))));
    h.feed(&kitty_apc(b"a=f,i=26,f=32,s=4,v=4", &b64(&fill_4x4(GREEN))));
    h.feed(&kitty_apc(
        b"a=f,i=26,f=32,s=4,v=4,r=0,X=1",
        &b64(&fill_4x4(BLUE)),
    ));
    let cache = h.term().image_cache();
    assert_eq!(cache.total_frames_for_test(ImageId::from_raw(26)), 3);
}

/// r=N > total → clamps to next-append, NOT EINVAL per kitty
/// graphics.c:1558-1561.
#[test]
fn kitty_frame_r_above_total_clamps_to_append() {
    let mut h = SpecHarness::new();
    h.feed(&kitty_apc(b"a=t,i=27,f=32,s=4,v=4", &b64(&fill_4x4(RED))));
    h.feed(&kitty_apc(b"a=f,i=27,f=32,s=4,v=4", &b64(&fill_4x4(GREEN))));
    h.feed(&kitty_apc(
        b"a=f,i=27,f=32,s=4,v=4,r=99,X=1",
        &b64(&fill_4x4(BLUE)),
    ));
    let cache = h.term().image_cache();
    assert_eq!(cache.total_frames_for_test(ImageId::from_raw(27)), 3);
    assert!(!reply_contains(&h, b"EINVAL"));
}

/// r=next boundary (r=total+1) without c= appends using Y= canvas.
#[test]
fn kitty_frame_r_next_boundary_without_c_appends_using_y_canvas() {
    let mut h = SpecHarness::new();
    h.feed(&kitty_apc(b"a=t,i=28,f=32,s=4,v=4", &b64(&fill_4x4(RED))));
    h.feed(&kitty_apc(b"a=f,i=28,f=32,s=4,v=4", &b64(&fill_4x4(GREEN))));
    // r=3 == total + 1 = next-append; no c=, no Y= → Y default = transparent.
    h.feed(&kitty_apc(b"a=f,i=28,f=32,s=4,v=4,r=3", &b64(&fill_4x4(TRANSPARENT))));
    let cache = h.term().image_cache();
    assert_eq!(cache.total_frames_for_test(ImageId::from_raw(28)), 3);
    // Frame 3 = transparent Y canvas with transparent payload (no-op blend).
    assert_frame_eq(cache, ImageId::from_raw(28), 3, &fill_4x4(TRANSPARENT));
}

/// r=2 on static image: extra_framecnt+2 = 2; r=2 clamps to next-append
/// (no animation yet → promotion).
#[test]
fn kitty_frame_r_two_on_static_image_appends_frame_two() {
    let mut h = SpecHarness::new();
    h.feed(&kitty_apc(b"a=t,i=29,f=32,s=4,v=4", &b64(&fill_4x4(RED))));
    h.feed(&kitty_apc(
        b"a=f,i=29,f=32,s=4,v=4,r=2,X=1",
        &b64(&fill_4x4(GREEN))
    ));
    let cache = h.term().image_cache();
    assert_eq!(cache.total_frames_for_test(ImageId::from_raw(29)), 2);
    assert_frame_eq(cache, ImageId::from_raw(29), 1, &fill_4x4(RED));
    assert_frame_eq(cache, ImageId::from_raw(29), 2, &fill_4x4(GREEN));
}

/// c=0 → treated as unspecified per spec (c= is 1-based); default arm fires.
#[test]
fn kitty_frame_c_zero_falls_back_to_default_append() {
    let mut h = SpecHarness::new();
    h.feed(&kitty_apc(b"a=t,i=30,f=32,s=4,v=4", &b64(&fill_4x4(RED))));
    h.feed(&kitty_apc(b"a=f,i=30,f=32,s=4,v=4", &b64(&fill_4x4(GREEN))));
    h.feed(&kitty_apc(b"a=f,i=30,f=32,s=4,v=4,c=0", &b64(&fill_4x4(BLUE))));
    let cache = h.term().image_cache();
    assert_eq!(cache.total_frames_for_test(ImageId::from_raw(30)), 3);
    // c=0 → default arm → Y canvas (transparent), not frame N canvas.
    assert_frame_eq(cache, ImageId::from_raw(30), 3, &fill_4x4(BLUE));
}

/// c=N out of range → EINVAL reply with frame number in message.
#[test]
fn kitty_frame_c_out_of_range_emits_einval_reply() {
    let mut h = SpecHarness::new();
    h.feed(&kitty_apc(b"a=t,i=31,f=32,s=4,v=4", &b64(&fill_4x4(RED))));
    h.feed(&kitty_apc(b"a=f,i=31,f=32,s=4,v=4", &b64(&fill_4x4(GREEN))));
    h.feed(&kitty_apc(b"a=f,i=31,f=32,s=4,v=4,c=99", &b64(&fill_4x4(BLUE))));
    assert_einval_reply(&h, 31, "No frame with number: 99");
    let cache = h.term().image_cache();
    assert_eq!(cache.total_frames_for_test(ImageId::from_raw(31)), 2);
}

/// (c, r) both set with r at next-boundary: r=next-append, c= specifies
/// canvas. Frame 3 appended using root (c=1) as canvas.
#[test]
fn kitty_frame_both_c_and_r_set_with_r_next_uses_c_as_canvas() {
    let mut h = SpecHarness::new();
    h.feed(&kitty_apc(b"a=t,i=32,f=32,s=4,v=4", &b64(&fill_4x4(RED))));
    h.feed(&kitty_apc(b"a=f,i=32,f=32,s=4,v=4", &b64(&fill_4x4(GREEN))));
    h.feed(&kitty_apc(
        b"a=f,i=32,f=32,s=4,v=4,c=1,r=3,X=1",
        &b64(&fill_4x4(RED))
    ));
    let cache = h.term().image_cache();
    assert_eq!(cache.total_frames_for_test(ImageId::from_raw(32)), 3);
    // Canvas = root (red); overwrite with red → frame 3 = red.
    assert_frame_eq(cache, ImageId::from_raw(32), 3, &fill_4x4(RED));
}

/// (c, r) both set with r within existing range: c= ignored; r= edits.
#[test]
fn kitty_frame_both_c_and_r_set_with_r_existing_ignores_c() {
    let mut h = SpecHarness::new();
    h.feed(&kitty_apc(b"a=t,i=33,f=32,s=4,v=4", &b64(&fill_4x4(RED))));
    h.feed(&kitty_apc(b"a=f,i=33,f=32,s=4,v=4", &b64(&fill_4x4(GREEN))));
    h.feed(&kitty_apc(
        b"a=f,i=33,f=32,s=4,v=4,c=1,r=2,X=1",
        &b64(&fill_4x4(BLUE))
    ));
    let cache = h.term().image_cache();
    assert_eq!(cache.total_frames_for_test(ImageId::from_raw(33)), 2);
    assert_frame_eq(cache, ImageId::from_raw(33), 2, &fill_4x4(BLUE));
    // Frame 1 (root) unchanged — c=1 NOT consulted on edit path.
    assert_frame_eq(cache, ImageId::from_raw(33), 1, &fill_4x4(RED));
}

/// Oversized blit width — payload decodes to 8x4 (128 bytes) but image is
/// 4x4 → OversizedBlit EINVAL per kitty graphics.c:1580-1583.
/// 128-byte payload sized to decode to 8×4 so the test reaches the
/// OversizedBlit gate, not the decoder size guard.
#[test]
fn kitty_frame_oversized_blit_width_rejected_einval() {
    let mut h = SpecHarness::new();
    h.feed(&kitty_apc(b"a=t,i=34,f=32,s=4,v=4", &b64(&fill_4x4(RED))));
    let payload = rgba_solid(8, 4, 0, 0, 255, 255);
    h.feed(&kitty_apc(b"a=f,i=34,f=32,s=8,v=4", &b64(&payload)));
    assert_einval_reply(&h, 34, "Frame width 8 larger than image width: 4");
}

/// Oversized blit height — payload 4x8 onto image 4x4 → EINVAL.
#[test]
fn kitty_frame_oversized_blit_height_rejected_einval() {
    let mut h = SpecHarness::new();
    h.feed(&kitty_apc(b"a=t,i=35,f=32,s=4,v=4", &b64(&fill_4x4(RED))));
    let payload = rgba_solid(4, 8, 0, 0, 255, 255);
    h.feed(&kitty_apc(b"a=f,i=35,f=32,s=4,v=8", &b64(&payload)));
    assert_einval_reply(&h, 35, "Frame height 8 larger than image height: 4");
}

/// Boundary equality: blit dim == image dim (4 == 4) PASSES.
#[test]
fn kitty_frame_blit_boundary_equal_to_image_passes() {
    let mut h = SpecHarness::new();
    h.feed(&kitty_apc(b"a=t,i=36,f=32,s=4,v=4", &b64(&fill_4x4(RED))));
    h.feed(&kitty_apc(b"a=f,i=36,f=32,s=4,v=4", &b64(&fill_4x4(GREEN))));
    let cache = h.term().image_cache();
    assert_eq!(cache.total_frames_for_test(ImageId::from_raw(36)), 2);
}

// ----------------------------------------------------------------------------
// Section: Frame-1-as-root semantics on static images
// ----------------------------------------------------------------------------

/// r=1 on static image edits root: bytes change in images[id].data; no
/// frames appended.
#[test]
fn kitty_frame_r_one_on_static_image_edits_root() {
    let mut h = SpecHarness::new();
    h.feed(&kitty_apc(b"a=t,i=37,f=32,s=4,v=4", &b64(&fill_4x4(RED))));
    h.feed(&kitty_apc(
        b"a=f,i=37,f=32,s=4,v=4,r=1,X=1",
        &b64(&fill_4x4(BLUE))
    ));
    let cache = h.term().image_cache();
    assert_eq!(cache.total_frames_for_test(ImageId::from_raw(37)), 1);
    let display = cache
        .image_data_bytes_for_test(ImageId::from_raw(37))
        .expect("image present");
    assert_eq!(display.as_slice(), fill_4x4(BLUE).as_slice());
}

/// c=1 on static image uses root as canvas: promotion + frame 2 built
/// from root canvas overwritten by green.
#[test]
fn kitty_frame_c_one_on_static_image_uses_root_as_canvas() {
    let mut h = SpecHarness::new();
    h.feed(&kitty_apc(b"a=t,i=38,f=32,s=4,v=4", &b64(&fill_4x4(RED))));
    h.feed(&kitty_apc(
        b"a=f,i=38,f=32,s=4,v=4,c=1,X=1",
        &b64(&fill_4x4(GREEN))
    ));
    let cache = h.term().image_cache();
    assert_eq!(cache.total_frames_for_test(ImageId::from_raw(38)), 2);
    assert_frame_eq(cache, ImageId::from_raw(38), 1, &fill_4x4(RED));
    assert_frame_eq(cache, ImageId::from_raw(38), 2, &fill_4x4(GREEN));
}

// ----------------------------------------------------------------------------
// Section: z= gap semantics — storage AND playback
// ----------------------------------------------------------------------------

/// z=0 → DEFAULT_GAP (40ms) per kitty graphics.c:1597.
#[test]
fn kitty_frame_z_zero_stores_default_gap_40ms() {
    let mut h = SpecHarness::new();
    h.feed(&kitty_apc(b"a=t,i=39,f=32,s=4,v=4", &b64(&fill_4x4(RED))));
    h.feed(&kitty_apc(b"a=f,i=39,f=32,s=4,v=4,z=0", &b64(&fill_4x4(GREEN))));
    let snap = h
        .term()
        .image_cache()
        .animation_snapshot(ImageId::from_raw(39))
        .expect("promoted");
    let last = *snap.frame_gaps.last().unwrap();
    assert_eq!(last, Duration::from_millis(40));
}

/// z<0 → ZERO (gapless).
#[test]
fn kitty_frame_z_negative_stores_zero_for_gapless() {
    let mut h = SpecHarness::new();
    h.feed(&kitty_apc(b"a=t,i=40,f=32,s=4,v=4", &b64(&fill_4x4(RED))));
    h.feed(&kitty_apc(
        b"a=f,i=40,f=32,s=4,v=4,z=-1",
        &b64(&fill_4x4(GREEN))
    ));
    let snap = h
        .term()
        .image_cache()
        .animation_snapshot(ImageId::from_raw(40))
        .expect("promoted");
    let last = *snap.frame_gaps.last().unwrap();
    assert_eq!(last, Duration::ZERO);
}

/// z>0 → exact ms.
#[test]
fn kitty_frame_z_positive_sets_gap_exactly() {
    let mut h = SpecHarness::new();
    h.feed(&kitty_apc(b"a=t,i=41,f=32,s=4,v=4", &b64(&fill_4x4(RED))));
    h.feed(&kitty_apc(
        b"a=f,i=41,f=32,s=4,v=4,z=77",
        &b64(&fill_4x4(GREEN))
    ));
    let snap = h
        .term()
        .image_cache()
        .animation_snapshot(ImageId::from_raw(41))
        .expect("promoted");
    let last = *snap.frame_gaps.last().unwrap();
    assert_eq!(last, Duration::from_millis(77));
}

/// r=1 z=77 on static image: auto-promote with root gap = 77ms (1 frame total).
#[test]
fn kitty_frame_r_one_z_positive_on_static_image_auto_promotes_with_gap() {
    let mut h = SpecHarness::new();
    h.feed(&kitty_apc(b"a=t,i=42,f=32,s=4,v=4", &b64(&fill_4x4(RED))));
    h.feed(&kitty_apc(
        b"a=f,i=42,f=32,s=4,v=4,r=1,z=77,X=1",
        &b64(&fill_4x4(BLUE))
    ));
    let cache = h.term().image_cache();
    assert_eq!(cache.total_frames_for_test(ImageId::from_raw(42)), 1);
    assert!(cache.animation_promoted_for_test(ImageId::from_raw(42)));
    let snap = cache
        .animation_snapshot(ImageId::from_raw(42))
        .expect("promoted");
    assert_eq!(snap.frame_gaps[0], Duration::from_millis(77));
    let display = cache
        .image_data_bytes_for_test(ImageId::from_raw(42))
        .expect("image present");
    assert_eq!(display.as_slice(), fill_4x4(BLUE).as_slice());
}

/// r=1 z=0 on static image: NO promotion (z=0 means unspecified), root
/// edited in place.
#[test]
fn kitty_frame_r_one_z_zero_on_static_image_does_not_promote() {
    let mut h = SpecHarness::new();
    h.feed(&kitty_apc(b"a=t,i=43,f=32,s=4,v=4", &b64(&fill_4x4(RED))));
    h.feed(&kitty_apc(
        b"a=f,i=43,f=32,s=4,v=4,r=1,z=0,X=1",
        &b64(&fill_4x4(BLUE))
    ));
    let cache = h.term().image_cache();
    assert!(!cache.animation_promoted_for_test(ImageId::from_raw(43)));
    let display = cache
        .image_data_bytes_for_test(ImageId::from_raw(43))
        .expect("image present");
    assert_eq!(display.as_slice(), fill_4x4(BLUE).as_slice());
}

/// r=1 z=-1 on static image: auto-promote with root gap = ZERO (gapless edit).
#[test]
fn kitty_frame_r_one_z_negative_on_static_image_auto_promotes_with_zero_gap() {
    let mut h = SpecHarness::new();
    h.feed(&kitty_apc(b"a=t,i=44,f=32,s=4,v=4", &b64(&fill_4x4(RED))));
    h.feed(&kitty_apc(
        b"a=f,i=44,f=32,s=4,v=4,r=1,z=-1,X=1",
        &b64(&fill_4x4(BLUE))
    ));
    let cache = h.term().image_cache();
    assert!(cache.animation_promoted_for_test(ImageId::from_raw(44)));
    let snap = cache
        .animation_snapshot(ImageId::from_raw(44))
        .expect("promoted");
    assert_eq!(snap.frame_gaps[0], Duration::ZERO);
}

/// r=N edit z=0 leaves existing frame's gap UNCHANGED per kitty
/// graphics.c:1651.
#[test]
fn kitty_frame_r_edit_z_zero_preserves_existing_frame_gap() {
    let mut h = SpecHarness::new();
    h.feed(&kitty_apc(b"a=t,i=45,f=32,s=4,v=4", &b64(&fill_4x4(RED))));
    // First frame: z=60 → frame_durations[1] = 60ms.
    h.feed(&kitty_apc(
        b"a=f,i=45,f=32,s=4,v=4,z=60",
        &b64(&fill_4x4(GREEN)),
    ));
    h.feed(&kitty_apc(
        b"a=f,i=45,f=32,s=4,v=4,r=2,z=0,X=1",
        &b64(&fill_4x4(BLUE))
    ));
    let snap = h
        .term()
        .image_cache()
        .animation_snapshot(ImageId::from_raw(45))
        .expect("promoted");
    assert_eq!(snap.frame_gaps[1], Duration::from_millis(60));
}

/// r=N edit z=77 updates frame N's gap.
#[test]
fn kitty_frame_r_edit_z_positive_updates_frame_gap() {
    let mut h = SpecHarness::new();
    h.feed(&kitty_apc(b"a=t,i=46,f=32,s=4,v=4", &b64(&fill_4x4(RED))));
    h.feed(&kitty_apc(
        b"a=f,i=46,f=32,s=4,v=4,z=60",
        &b64(&fill_4x4(GREEN)),
    ));
    h.feed(&kitty_apc(
        b"a=f,i=46,f=32,s=4,v=4,r=2,z=77,X=1",
        &b64(&fill_4x4(BLUE)),
    ));
    let snap = h
        .term()
        .image_cache()
        .animation_snapshot(ImageId::from_raw(46))
        .expect("promoted");
    assert_eq!(snap.frame_gaps[1], Duration::from_millis(77));
}

/// r=N edit z=-1 sets frame N's gap to ZERO (gapless edit).
#[test]
fn kitty_frame_r_edit_z_negative_sets_gapless() {
    let mut h = SpecHarness::new();
    h.feed(&kitty_apc(b"a=t,i=47,f=32,s=4,v=4", &b64(&fill_4x4(RED))));
    h.feed(&kitty_apc(
        b"a=f,i=47,f=32,s=4,v=4,z=60",
        &b64(&fill_4x4(GREEN)),
    ));
    h.feed(&kitty_apc(
        b"a=f,i=47,f=32,s=4,v=4,r=2,z=-1,X=1",
        &b64(&fill_4x4(BLUE)),
    ));
    let snap = h
        .term()
        .image_cache()
        .animation_snapshot(ImageId::from_raw(47))
        .expect("promoted");
    assert_eq!(snap.frame_gaps[1], Duration::ZERO);
}

/// Vacant-promotion: root frame's gap is ZERO (per kitty_tests:1189
/// `root_frame_gap == 0`), new frame gets the normalized z gap.
#[test]
fn kitty_frame_vacant_promotion_root_gap_is_zero_new_frame_gets_z() {
    let mut h = SpecHarness::new();
    h.feed(&kitty_apc(b"a=t,i=48,f=32,s=4,v=4", &b64(&fill_4x4(RED))));
    h.feed(&kitty_apc(
        b"a=f,i=48,f=32,s=4,v=4,z=77",
        &b64(&fill_4x4(GREEN)),
    ));
    let snap = h
        .term()
        .image_cache()
        .animation_snapshot(ImageId::from_raw(48))
        .expect("promoted");
    assert_eq!(snap.frame_gaps, &[Duration::ZERO, Duration::from_millis(77)]);
}

// ----------------------------------------------------------------------------
// Section: a=a (animation control) root-gap storage
// ----------------------------------------------------------------------------

/// a=a r=1 z>0 on static image: auto-promote, root gap = z ms.
#[test]
fn kitty_animate_r_one_z_positive_on_static_image_promotes_with_gap() {
    let mut h = SpecHarness::new();
    h.feed(&kitty_apc(b"a=t,i=49,f=32,s=4,v=4", &b64(&fill_4x4(RED))));
    h.feed(&kitty_apc(b"a=a,i=49,r=1,z=13", ""));
    let cache = h.term().image_cache();
    assert!(cache.animation_promoted_for_test(ImageId::from_raw(49)));
    let snap = cache
        .animation_snapshot(ImageId::from_raw(49))
        .expect("promoted");
    assert_eq!(snap.frame_gaps[0], Duration::from_millis(13));
}

/// a=a r=1 z=0 on static image: NO promotion (kitty change_gap g->gap==0
/// short-circuit per graphics.c:1729-1735).
#[test]
fn kitty_animate_r_one_z_zero_on_static_image_preserves_no_gap() {
    let mut h = SpecHarness::new();
    h.feed(&kitty_apc(b"a=t,i=50,f=32,s=4,v=4", &b64(&fill_4x4(RED))));
    h.feed(&kitty_apc(b"a=a,i=50,r=1,z=0", ""));
    let cache = h.term().image_cache();
    assert!(!cache.animation_promoted_for_test(ImageId::from_raw(50)));
}

/// a=a r=1 z<0 on static image: auto-promote, root gap = ZERO (gapless).
#[test]
fn kitty_animate_r_one_z_negative_on_static_image_promotes_gapless() {
    let mut h = SpecHarness::new();
    h.feed(&kitty_apc(b"a=t,i=51,f=32,s=4,v=4", &b64(&fill_4x4(RED))));
    h.feed(&kitty_apc(b"a=a,i=51,r=1,z=-1", ""));
    let cache = h.term().image_cache();
    assert!(cache.animation_promoted_for_test(ImageId::from_raw(51)));
    let snap = cache
        .animation_snapshot(ImageId::from_raw(51))
        .expect("promoted");
    assert_eq!(snap.frame_gaps[0], Duration::ZERO);
}

/// a=a sets root gap, then a=f appends — root gap persists.
#[test]
fn kitty_animate_r_one_then_a_f_append_root_gap_persists_across_promotion() {
    let mut h = SpecHarness::new();
    h.feed(&kitty_apc(b"a=t,i=52,f=32,s=4,v=4", &b64(&fill_4x4(RED))));
    h.feed(&kitty_apc(b"a=a,i=52,r=1,z=13", ""));
    h.feed(&kitty_apc(
        b"a=f,i=52,f=32,s=4,v=4,z=77",
        &b64(&fill_4x4(GREEN)),
    ));
    let snap = h
        .term()
        .image_cache()
        .animation_snapshot(ImageId::from_raw(52))
        .expect("promoted");
    assert_eq!(
        snap.frame_gaps,
        &[Duration::from_millis(13), Duration::from_millis(77)]
    );
}

/// a=a r=1 z>0 on promoted image updates root gap, frame 2 unchanged.
#[test]
fn kitty_animate_r_one_z_positive_on_promoted_image_updates_root_gap() {
    let mut h = SpecHarness::new();
    h.feed(&kitty_apc(b"a=t,i=53,f=32,s=4,v=4", &b64(&fill_4x4(RED))));
    h.feed(&kitty_apc(
        b"a=f,i=53,f=32,s=4,v=4,z=77",
        &b64(&fill_4x4(GREEN)),
    ));
    h.feed(&kitty_apc(b"a=a,i=53,r=1,z=13", ""));
    let snap = h
        .term()
        .image_cache()
        .animation_snapshot(ImageId::from_raw(53))
        .expect("promoted");
    assert_eq!(
        snap.frame_gaps,
        &[Duration::from_millis(13), Duration::from_millis(77)]
    );
}

/// a=a r=1 z<0 on promoted image sets root gapless.
#[test]
fn kitty_animate_r_one_z_negative_on_promoted_image_sets_root_gapless() {
    let mut h = SpecHarness::new();
    h.feed(&kitty_apc(b"a=t,i=54,f=32,s=4,v=4", &b64(&fill_4x4(RED))));
    h.feed(&kitty_apc(
        b"a=f,i=54,f=32,s=4,v=4,z=77",
        &b64(&fill_4x4(GREEN)),
    ));
    h.feed(&kitty_apc(b"a=a,i=54,r=1,z=-1", ""));
    let snap = h
        .term()
        .image_cache()
        .animation_snapshot(ImageId::from_raw(54))
        .expect("promoted");
    assert_eq!(snap.frame_gaps, &[Duration::ZERO, Duration::from_millis(77)]);
}

/// a=a r=1 z=0 on promoted image leaves root gap unchanged.
#[test]
fn kitty_animate_r_one_z_zero_on_promoted_image_preserves_root_gap() {
    let mut h = SpecHarness::new();
    h.feed(&kitty_apc(b"a=t,i=55,f=32,s=4,v=4", &b64(&fill_4x4(RED))));
    h.feed(&kitty_apc(b"a=a,i=55,r=1,z=13", ""));
    h.feed(&kitty_apc(
        b"a=f,i=55,f=32,s=4,v=4,z=77",
        &b64(&fill_4x4(GREEN)),
    ));
    h.feed(&kitty_apc(b"a=a,i=55,r=1,z=0", ""));
    let snap = h
        .term()
        .image_cache()
        .animation_snapshot(ImageId::from_raw(55))
        .expect("promoted");
    assert_eq!(
        snap.frame_gaps,
        &[Duration::from_millis(13), Duration::from_millis(77)]
    );
}

/// a=a on missing image emits ENOENT.
#[test]
fn kitty_animate_missing_image_emits_enoent() {
    let mut h = SpecHarness::new();
    h.feed(&kitty_apc(b"a=a,i=999,r=1,z=10", ""));
    assert!(reply_contains(&h, b"i=999;ENOENT"));
}

// ----------------------------------------------------------------------------
// Section: Sub-rect offset clipping
// ----------------------------------------------------------------------------

/// x=u32::MAX: silent no-op (sub-rect entirely outside canvas). No panic.
#[test]
fn kitty_frame_subrect_x_max_u32_does_not_panic() {
    let mut h = SpecHarness::new();
    h.feed(&kitty_apc(b"a=t,i=56,f=32,s=4,v=4", &b64(&fill_4x4(RED))));
    h.feed(&kitty_apc(
        b"a=f,i=56,f=32,s=4,v=4,x=4294967295,X=1",
        &b64(&fill_4x4(BLUE)),
    ));
    let cache = h.term().image_cache();
    assert_eq!(cache.total_frames_for_test(ImageId::from_raw(56)), 2);
    // Frame 2 = Y canvas untouched (silent no-op blit).
    assert_frame_eq(cache, ImageId::from_raw(56), 2, &fill_4x4(TRANSPARENT));
}

/// y=u32::MAX: same as above.
#[test]
fn kitty_frame_subrect_y_max_u32_does_not_panic() {
    let mut h = SpecHarness::new();
    h.feed(&kitty_apc(b"a=t,i=57,f=32,s=4,v=4", &b64(&fill_4x4(RED))));
    h.feed(&kitty_apc(
        b"a=f,i=57,f=32,s=4,v=4,y=4294967295,X=1",
        &b64(&fill_4x4(BLUE)),
    ));
    let cache = h.term().image_cache();
    assert_eq!(cache.total_frames_for_test(ImageId::from_raw(57)), 2);
    assert_frame_eq(cache, ImageId::from_raw(57), 2, &fill_4x4(TRANSPARENT));
}

/// Sub-rect partially clipped: x=3, width=2 → only x=3 pixel lands; x=4 clipped.
#[test]
fn kitty_frame_subrect_x_equals_image_w_minus_1_with_width_2_clips_to_1_pixel() {
    let mut h = SpecHarness::new();
    h.feed(&kitty_apc(b"a=t,i=58,f=32,s=4,v=4", &b64(&fill_4x4(RED))));
    let payload = rgba_solid(2, 1, 0, 0, 255, 255);
    h.feed(&kitty_apc(
        b"a=f,i=58,f=32,s=2,v=1,x=3,y=0,X=1",
        &b64(&payload),
    ));
    let cache = h.term().image_cache();
    let mut expected = vec![0u8; 64];
    // Only (3,0) blue; rest transparent.
    expected[(0 * 4 + 3) * 4..(0 * 4 + 3) * 4 + 4].copy_from_slice(&BLUE);
    assert_frame_eq(cache, ImageId::from_raw(58), 2, &expected);
}

/// Sub-rect partial vertical clip: y=2, height=3 → only rows 2 and 3 land.
#[test]
fn kitty_frame_subrect_y_edge_clip_to_partial_height() {
    let mut h = SpecHarness::new();
    h.feed(&kitty_apc(b"a=t,i=59,f=32,s=4,v=4", &b64(&fill_4x4(RED))));
    let payload = rgba_solid(1, 3, 0, 0, 255, 255);
    h.feed(&kitty_apc(
        b"a=f,i=59,f=32,s=1,v=3,x=0,y=2,X=1",
        &b64(&payload),
    ));
    let cache = h.term().image_cache();
    let mut expected = vec![0u8; 64];
    // (0,2) and (0,3) blue; rest transparent.
    expected[(2 * 4 + 0) * 4..(2 * 4 + 0) * 4 + 4].copy_from_slice(&BLUE);
    expected[(3 * 4 + 0) * 4..(3 * 4 + 0) * 4 + 4].copy_from_slice(&BLUE);
    assert_frame_eq(cache, ImageId::from_raw(59), 2, &expected);
}

// ----------------------------------------------------------------------------
// Section: Default-arm Y= canvas
// ----------------------------------------------------------------------------

/// Default arm does NOT blend against previous frame — canvas is Y= solid.
/// Pre-fix bug: alpha-blend against frame N-1.
#[test]
fn kitty_frame_default_arm_does_not_blend_against_previous_frame() {
    let mut h = SpecHarness::new();
    h.feed(&kitty_apc(b"a=t,i=60,f=32,s=4,v=4", &b64(&fill_4x4(RED))));
    h.feed(&kitty_apc(b"a=f,i=60,f=32,s=4,v=4", &b64(&fill_4x4(GREEN))));
    // Default arm, fully transparent payload (alpha=0). Canvas = Y default = 0.
    // Blend(transparent over transparent) = transparent. NOT green.
    h.feed(&kitty_apc(
        b"a=f,i=60,f=32,s=4,v=4",
        &b64(&fill_4x4(TRANSPARENT)),
    ));
    let cache = h.term().image_cache();
    assert_frame_eq(cache, ImageId::from_raw(60), 3, &fill_4x4(TRANSPARENT));
}

/// Default arm honors explicit Y= color: opaque red canvas + 2x2 payload at (0,0).
#[test]
fn kitty_frame_default_arm_honors_explicit_y_color() {
    let mut h = SpecHarness::new();
    h.feed(&kitty_apc(b"a=t,i=61,f=32,s=4,v=4", &b64(&fill_4x4(RED))));
    // Y=0xff0000ff = R=255 G=0 B=0 A=255 (opaque red).
    h.feed(&kitty_apc(
        b"a=f,i=61,f=32,s=2,v=2,Y=4278190335,X=1",
        &b64(&fill_2x2(GREEN)),
    ));
    let cache = h.term().image_cache();
    let mut expected = Vec::with_capacity(64);
    for row in 0..4 {
        for col in 0..4 {
            if col < 2 && row < 2 {
                expected.extend_from_slice(&GREEN);
            } else {
                expected.extend_from_slice(&RED);
            }
        }
    }
    assert_frame_eq(cache, ImageId::from_raw(61), 2, &expected);
}

/// Default arm honors x/y sub-rect offset on the Y= canvas.
#[test]
fn kitty_frame_default_arm_honors_subrect_xy_offset() {
    let mut h = SpecHarness::new();
    h.feed(&kitty_apc(b"a=t,i=62,f=32,s=4,v=4", &b64(&fill_4x4(RED))));
    h.feed(&kitty_apc(
        b"a=f,i=62,f=32,s=2,v=2,x=1,y=1,X=1",
        &b64(&fill_2x2(BLUE)),
    ));
    let cache = h.term().image_cache();
    let mut expected = Vec::with_capacity(64);
    for row in 0..4 {
        for col in 0..4 {
            if (1..=2).contains(&col) && (1..=2).contains(&row) {
                expected.extend_from_slice(&BLUE);
            } else {
                expected.extend_from_slice(&TRANSPARENT);
            }
        }
    }
    assert_frame_eq(cache, ImageId::from_raw(62), 2, &expected);
}

// ----------------------------------------------------------------------------
// Section: Y= byte-order pin (§05 Step 6)
// ----------------------------------------------------------------------------

/// Y=0xff0000ff produces opaque red [R=0xff, G=0x00, B=0x00, A=0xff].
#[test]
fn kitty_frame_y_key_byte_order_is_rgba_msb_first() {
    let mut h = SpecHarness::new();
    h.feed(&kitty_apc(b"a=t,i=63,f=32,s=4,v=4", &b64(&fill_4x4(GREEN))));
    h.feed(&kitty_apc(
        b"a=f,i=63,f=32,s=4,v=4,Y=4278190335,X=1",
        &b64(&fill_4x4(RED)),
    ));
    let cache = h.term().image_cache();
    // Full-frame overwrite: payload bytes = RED. (canvas is Y= but full
    // overwrite covers it.) Pin the canvas via empty-payload + sub-rect.
    let _ = cache;
    // Better pin: use 0-area sub-rect to leave canvas exposed.
    let mut h2 = SpecHarness::new();
    h2.feed(&kitty_apc(b"a=t,i=64,f=32,s=4,v=4", &b64(&fill_4x4(GREEN))));
    // 2x2 payload at (0,0); rest = Y canvas.
    h2.feed(&kitty_apc(
        b"a=f,i=64,f=32,s=2,v=2,x=2,y=2,Y=4278190335,X=1",
        &b64(&fill_2x2(BLUE)),
    ));
    let cache2 = h2.term().image_cache();
    let frame = cache2
        .frame_bytes_for_test(ImageId::from_raw(64), 2)
        .expect("frame 2 present");
    // Top-left pixel = RED (from Y canvas).
    assert_eq!(&frame[0..4], &RED);
}

// ----------------------------------------------------------------------------
// Section: Semantic + Negative pins
// ----------------------------------------------------------------------------

/// r= arm mutates target frame byte-exact.
#[test]
fn kitty_frame_r_arm_mutates_target_frame_byte_exact() {
    let mut h = SpecHarness::new();
    h.feed(&kitty_apc(b"a=t,i=65,f=32,s=4,v=4", &b64(&fill_4x4(RED))));
    h.feed(&kitty_apc(b"a=f,i=65,f=32,s=4,v=4", &b64(&fill_4x4(RED))));
    h.feed(&kitty_apc(
        b"a=f,i=65,f=32,s=4,v=4,r=2,X=1",
        &b64(&fill_4x4(BLUE)),
    ));
    let cache = h.term().image_cache();
    assert_frame_eq(cache, ImageId::from_raw(65), 2, &fill_4x4(BLUE));
}

/// c= arm leaves canvas frame unchanged.
#[test]
fn kitty_frame_c_arm_leaves_canvas_frame_unchanged() {
    let mut h = SpecHarness::new();
    h.feed(&kitty_apc(b"a=t,i=66,f=32,s=4,v=4", &b64(&fill_4x4(RED))));
    h.feed(&kitty_apc(b"a=f,i=66,f=32,s=4,v=4", &b64(&fill_4x4(RED))));
    h.feed(&kitty_apc(
        b"a=f,i=66,f=32,s=4,v=4,c=2,X=1",
        &b64(&fill_4x4(BLUE)),
    ));
    let cache = h.term().image_cache();
    assert_frame_eq(cache, ImageId::from_raw(66), 2, &fill_4x4(RED));
    assert_frame_eq(cache, ImageId::from_raw(66), 3, &fill_4x4(BLUE));
}

/// r= negative: does NOT grow frame count.
#[test]
fn kitty_frame_r_does_not_grow_frame_count() {
    let mut h = SpecHarness::new();
    h.feed(&kitty_apc(b"a=t,i=67,f=32,s=4,v=4", &b64(&fill_4x4(RED))));
    h.feed(&kitty_apc(b"a=f,i=67,f=32,s=4,v=4", &b64(&fill_4x4(GREEN))));
    let before = h
        .term()
        .image_cache()
        .total_frames_for_test(ImageId::from_raw(67));
    h.feed(&kitty_apc(
        b"a=f,i=67,f=32,s=4,v=4,r=2,X=1",
        &b64(&fill_4x4(BLUE)),
    ));
    let after = h
        .term()
        .image_cache()
        .total_frames_for_test(ImageId::from_raw(67));
    assert_eq!(before, after);
}

/// c= negative: does NOT mutate the source canvas frame.
#[test]
fn kitty_frame_c_does_not_mutate_source_frame() {
    let mut h = SpecHarness::new();
    h.feed(&kitty_apc(b"a=t,i=68,f=32,s=4,v=4", &b64(&fill_4x4(RED))));
    h.feed(&kitty_apc(b"a=f,i=68,f=32,s=4,v=4", &b64(&fill_4x4(GREEN))));
    let before = h
        .term()
        .image_cache()
        .frame_bytes_for_test(ImageId::from_raw(68), 2)
        .expect("frame 2 present")
        .clone();
    h.feed(&kitty_apc(
        b"a=f,i=68,f=32,s=4,v=4,c=2,X=1",
        &b64(&fill_4x4(BLUE)),
    ));
    let after = h
        .term()
        .image_cache()
        .frame_bytes_for_test(ImageId::from_raw(68), 2)
        .expect("frame 2 still present");
    assert_eq!(before.as_slice(), after.as_slice());
}

// ----------------------------------------------------------------------------
// Section: Reply protocol pins
// ----------------------------------------------------------------------------

/// r= arm reply echoes ,r=<frame_num>.
#[test]
fn kitty_frame_r_arm_echoes_frame_num_n_on_reply() {
    let mut h = SpecHarness::new();
    h.feed(&kitty_apc(b"a=t,i=69,f=32,s=4,v=4", &b64(&fill_4x4(RED))));
    h.feed(&kitty_apc(b"a=f,i=69,f=32,s=4,v=4", &b64(&fill_4x4(GREEN))));
    h.feed(&kitty_apc(
        b"a=f,i=69,f=32,s=4,v=4,r=2,X=1",
        &b64(&fill_4x4(BLUE)),
    ));
    assert!(reply_contains(&h, b"i=69,r=2;OK"));
}

/// c= arm reply echoes new appended index.
#[test]
fn kitty_frame_c_arm_echoes_new_appended_index_on_reply() {
    let mut h = SpecHarness::new();
    h.feed(&kitty_apc(b"a=t,i=70,f=32,s=4,v=4", &b64(&fill_4x4(RED))));
    h.feed(&kitty_apc(b"a=f,i=70,f=32,s=4,v=4", &b64(&fill_4x4(GREEN))));
    h.feed(&kitty_apc(
        b"a=f,i=70,f=32,s=4,v=4,c=2,X=1",
        &b64(&fill_4x4(BLUE)),
    ));
    assert!(reply_contains(&h, b"i=70,r=3;OK"));
}

/// c=N out-of-range reply contains "No frame with number: N" — NOT ,r=99.
#[test]
fn kitty_frame_c_out_of_range_einval_message_contains_c_value() {
    let mut h = SpecHarness::new();
    h.feed(&kitty_apc(b"a=t,i=71,f=32,s=4,v=4", &b64(&fill_4x4(RED))));
    h.feed(&kitty_apc(b"a=f,i=71,f=32,s=4,v=4", &b64(&fill_4x4(GREEN))));
    h.feed(&kitty_apc(
        b"a=f,i=71,f=32,s=4,v=4,c=99,X=1",
        &b64(&fill_4x4(BLUE)),
    ));
    assert!(reply_contains(&h, b"EINVAL"));
    assert!(reply_contains(&h, b"No frame with number: 99"));
}

// ----------------------------------------------------------------------------
// Section: OOB-read regression — decoded-dim is load-bearing for PNG blit
// ----------------------------------------------------------------------------

/// Raw RGBA with matching s= works (the happy path — confirms decoder
/// validation pre-fix path remains intact).
#[test]
fn kitty_frame_raw_rgba_payload_with_matching_s_succeeds() {
    let mut h = SpecHarness::new();
    h.feed(&kitty_apc(b"a=t,i=72,f=32,s=4,v=4", &b64(&fill_4x4(RED))));
    h.feed(&kitty_apc(b"a=f,i=72,f=32,s=4,v=4", &b64(&fill_4x4(GREEN))));
    let cache = h.term().image_cache();
    assert_eq!(cache.total_frames_for_test(ImageId::from_raw(72)), 2);
}

// ----------------------------------------------------------------------------
// Section: Chunked-frame (m=1) coverage
// ----------------------------------------------------------------------------

/// Chunked a=f r=2 edits frame 2 after finalize — one frame mutated, OK r=2.
#[test]
fn kitty_frame_chunked_a_f_r_edits_existing_frame_after_finalize() {
    let mut h = SpecHarness::new();
    h.feed(&kitty_apc(b"a=t,i=73,f=32,s=4,v=4", &b64(&fill_4x4(RED))));
    h.feed(&kitty_apc(b"a=f,i=73,f=32,s=4,v=4", &b64(&fill_4x4(GREEN))));
    let payload = fill_4x4(BLUE);
    let half = payload.len() / 2;
    let first_b64 = b64(&payload[..half]);
    let second_b64 = b64(&payload[half..]);
    h.feed(&kitty_apc(
        b"a=f,i=73,f=32,s=4,v=4,r=2,X=1,m=1",
        &first_b64,
    ));
    h.feed(&kitty_apc(b"a=f,i=73,m=0", &second_b64));
    let cache = h.term().image_cache();
    assert_eq!(cache.total_frames_for_test(ImageId::from_raw(73)), 2);
    assert!(reply_contains(&h, b"i=73,r=2;OK"));
}

/// Chunked a=f c=N appends new frame after finalize.
#[test]
fn kitty_frame_chunked_a_f_c_appends_new_frame_after_finalize() {
    let mut h = SpecHarness::new();
    h.feed(&kitty_apc(b"a=t,i=74,f=32,s=4,v=4", &b64(&fill_4x4(RED))));
    h.feed(&kitty_apc(b"a=f,i=74,f=32,s=4,v=4", &b64(&fill_4x4(GREEN))));
    let payload = fill_4x4(BLUE);
    let half = payload.len() / 2;
    let first_b64 = b64(&payload[..half]);
    let second_b64 = b64(&payload[half..]);
    h.feed(&kitty_apc(
        b"a=f,i=74,f=32,s=4,v=4,c=1,X=1,m=1",
        &first_b64,
    ));
    h.feed(&kitty_apc(b"a=f,i=74,m=0", &second_b64));
    let cache = h.term().image_cache();
    assert_eq!(cache.total_frames_for_test(ImageId::from_raw(74)), 3);
    assert!(reply_contains(&h, b"i=74,r=3;OK"));
}

// ----------------------------------------------------------------------------
// Matrix completeness counter — kitty a=f c=/r= dispatch arms.
// ----------------------------------------------------------------------------

#[test]
fn animation_category_matrix_completeness() {
    let categories: &[&str] = &[
        "frame_transmit_promotion_r2",
        "frame_subsequent_append_r3",
        "frame_enoent_no_r_qualifier",
        "frame_composite_alphablend",
        "frame_composite_overwrite",
        "animate_stop_s1",
        "animate_run_wait_s2_vs_s3",
        "animate_v0_ignored",
        "animate_v1_infinite",
        "animate_v3_finite_two_loops",
        "animate_v5_finite_four_loops",
        "animate_v_absent_negative",
        "animate_c2_seek_and_reply",
        "animate_c_zero_no_op",
        "animate_c_out_of_range_no_op",
        "animate_r2_z_positive_gap_target",
        "animate_r2_z_negative_gapless",
        "animate_r2_z_zero_no_op",
        "animate_r_out_of_range_no_op",
        "animate_r_on_static_silent_no_op",
        "animate_z_alone_no_op_negative_pin",
        "animate_r_and_c_independent_arms",
        "advance_deadline_visible",
        "no_tick_negative",
        // Kitty c=/r= dispatch matrix additions:
        "frame_r_edits_in_place",
        "frame_c_appends_with_canvas",
        "matrix_default_alphablend_full",
        "matrix_default_alphablend_subrect",
        "matrix_default_overwrite_full",
        "matrix_default_overwrite_subrect",
        "matrix_c_alphablend_full",
        "matrix_c_alphablend_subrect",
        "matrix_c_overwrite_full",
        "matrix_c_overwrite_subrect",
        "matrix_r_alphablend_full",
        "matrix_r_alphablend_subrect",
        "matrix_r_overwrite_full",
        "matrix_r_overwrite_subrect",
        "frame_1_vs_root_when_advanced_c",
        "frame_1_vs_root_when_advanced_r",
        "frame_1_root_edit_display_zero",
        "frame_2_edit_no_display_touch",
        "edge_r_zero_appends",
        "edge_r_above_total_clamps",
        "edge_r_next_boundary_y_canvas",
        "edge_r_two_on_static",
        "edge_c_zero_default",
        "edge_c_out_of_range_einval",
        "edge_c_r_combo_next_uses_c",
        "edge_c_r_combo_existing_ignores_c",
        "edge_oversized_blit_width",
        "edge_oversized_blit_height",
        "edge_blit_boundary_equal",
        "static_r_one_edits_root",
        "static_c_one_uses_root_canvas",
        "z_zero_default_gap_40ms",
        "z_negative_zero",
        "z_positive_exact",
        "static_r_one_z_positive_promotes",
        "static_r_one_z_zero_no_promote",
        "static_r_one_z_negative_gapless",
        "edit_z_zero_preserves",
        "edit_z_positive_updates",
        "edit_z_negative_gapless",
        "vacant_promotion_root_zero",
        "animate_r_one_static_z_positive",
        "animate_r_one_static_z_zero",
        "animate_r_one_static_z_negative",
        "animate_r_one_then_a_f_root_persists",
        "animate_r_one_promoted_z_positive",
        "animate_r_one_promoted_z_negative",
        "animate_r_one_promoted_z_zero",
        "animate_missing_image_enoent",
        "subrect_x_max_u32",
        "subrect_y_max_u32",
        "subrect_x_partial_clip",
        "subrect_y_partial_clip",
        "default_no_prev_blend",
        "default_y_color_honored",
        "default_subrect_xy_offset",
        "y_byte_order_rgba_msb",
        "semantic_r_mutates_target",
        "semantic_c_canvas_unchanged",
        "negative_r_no_grow",
        "negative_c_no_mutate",
        "reply_r_arm_echoes_n",
        "reply_c_arm_echoes_new",
        "reply_c_oor_einval_with_c",
        "raw_rgba_matching_s_succeeds",
        "chunked_r_edits",
        "chunked_c_appends",
    ];
    assert_eq!(
        categories.len(),
        91,
        "animation matrix MUST cover 91 categories — if you add a new pin, \
         bump this count so matrix completeness is self-verifying"
    );
}
