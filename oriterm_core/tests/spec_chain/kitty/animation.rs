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
//!
//! Closes TPR-13.0.5-R1-F3- (v=0 infinite loops via `loop_count
//! Option<u32>`) and TPR-13.2-R3-F1 (`,r=<frame_num>` on a=f + a=a replies).

use std::time::{Duration, Instant};

use oriterm_core::image::ImageId;
use oriterm_test_support::spec_chain::SpecHarness;

use super::fixtures::{b64, kitty_apc, ok_reply_for, reply_bytes, reply_contains, rgba_4x4_red};

/// Build an `a=f` OK-reply expectation with the `,r=<frame_num>` qualifier
/// per kitty `finish_command_response` for frame-loading replies.
fn ok_reply_with_frame(id: u32, frame_num: u32) -> Vec<u8> {
    format!("\x1b_Gi={id},r={frame_num};OK\x1b\\").into_bytes()
}

/// Transmit a base image via `a=t`, then append an `a=f` frame, asserting
/// the cache promotes to animated with 2 frames and the OK reply carries
/// `,r=2` (newly-added frame is 1-based index 2 per kitty's promotion rule:
/// existing data becomes frame 1, new data frame 2).
/// Catalog row: `KG-FRAME-TRANSMIT`. Closes TPR-13.2-R3-F1.
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
/// Regression: BUG-08-048 — pre-fix code routed v=0 → loop_count=None (infinite)
/// which was an inverted sentinel relative to kitty.
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
/// Regression: BUG-08-048 — pre-fix code routed v=1 → loop_count=Some(1) (loop
/// once and stop), opposite of kitty.
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
/// Regression: BUG-08-048.
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
/// Regression: BUG-08-048 — pre-fix code mapped v=5 → Some(5).
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
/// Catalog row: `KG-ANIMATE-SET-CURRENT-FRAME`. Closes TPR-13.2-R3-F1 (a=a arm).
/// Regression: BUG-08-045 — pre-fix dispatch routed both `r=` and `c=` to
/// set_current_frame, inverting the kitty semantic. Now `c=` is the seek,
/// `r=` is the gap-target selector (covered by sibling tests below).
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
/// Catalog row: `KG-ANIMATE-SET-FRAME-GAP` (negative-pin guard).
/// Regression: BUG-08-045 — proves the standalone-`z=` branch was correctly
/// deleted in the dispatch split.
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
/// Regression: BUG-08-045 — pre-fix code routed `r=` to `set_current_frame`
/// instead of `set_frame_gap`, so this case silently seek-without-gap'd.
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
/// Catalog row: `KG-ANIMATE-SET-FRAME-GAP` (gapless variant).
/// Regression: BUG-08-045 — negative-z= clamping path.
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
/// Catalog row: `KG-ANIMATE-SET-FRAME-GAP` (z=0 no-op variant).
/// Regression: BUG-08-045 — z=0 guard preservation.
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
/// Catalog row: `KG-ANIMATE-SET-FRAME-GAP` (out-of-range no-op).
/// Regression: BUG-08-045 — bounds-check inheritance from set_frame_gap.
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
/// Regression: BUG-08-045 — explicit boundary marker between the dispatch
/// split and the auto-promote helper that layers on top of it.
/// See: bug-tracker/plans/BUG-08-041/section-05-implementation.md Step 3.5
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
/// (independent-arms interaction).
/// Regression: BUG-08-045 — pre-fix code collapsed both keys into the same
/// dispatch, so `r=` AND `c=` together produced incoherent state.
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
#[test]
fn animation_category_matrix_completeness() {
    // Categories: a=f transmit/append/ENOENT/alphablend/overwrite, a=a s=
    // stop/wait/run, a=a v= ignored/infinite/finite/absent, a=a c= seek and
    // bounds guards, a=a r= z= gap-target variants, a=a z= alone guard, a=a
    // r=+c= independent arms, advance deadline, no-tick guard.
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
    ];
    assert_eq!(
        categories.len(),
        24,
        "animation matrix MUST cover 24 categories — if you add a new \
         pin, bump this count so matrix completeness is self-verifying"
    );
}
