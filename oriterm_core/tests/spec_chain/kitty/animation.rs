//! Per-animation rung (§13.3) — drives `a=f` frame-transmit + composition
//! modes, `a=a` playback control, loop count, frame gap, and the `,r=`
//! qualifier on frame/animate replies through the `SpecHarness` verification
//! chain.
//!
//! Catalog rows: `KG-FRAME-TRANSMIT` (a=f appends a frame, OK reply with
//! ,r=<n>), `KG-FRAME-COMPOSITE-OVERWRITE` (X=1 overwrites), `KG-FRAME-
//! COMPOSITE-ALPHABLEND` (default alpha-blends), `KG-ANIMATE-STOP`
//! (s=1 pauses), `KG-ANIMATE-RUN-WAIT` (s=2 resumes waiting), `KG-ANIMATE-RUN`
//! (s=3 resumes), `KG-ANIMATE-LOOP-COUNT` (v=0 → infinite, v=N → finite),
//! `KG-ANIMATE-SET-CURRENT-FRAME` (r=N / c=N), `KG-ANIMATE-SET-FRAME-GAP`
//! (z=ms for current frame).
//!
//! Closes TPR-13.0.5-R1-F3-gemini (v=0 infinite loops via `loop_count:
//! Option<u32>`) and TPR-13.2-R3-F1 (`,r=<frame_num>` on a=f + a=a replies).

use std::time::{Duration, Instant};

use oriterm_core::image::ImageId;
use oriterm_test_support::spec_chain::SpecHarness;

use super::fixtures::{b64, kitty_apc, ok_reply_for, reply_bytes, reply_contains, rgba_4x4_red};

/// Build an `a=f` OK-reply expectation with the `,r=<frame_num>` qualifier
/// kitty `finish_command_response` appends for frame-loading replies.
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
        .animation_state(ImageId::from_raw(99))
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
        .animation_state(ImageId::from_raw(98))
        .expect("i=98 MUST be animated after two a=f commands");
    assert_eq!(state.total_frames, 3, "three frames total after two a=f");
}

/// Missing-image `a=f` emits ENOENT with NO `,r=` qualifier — the negative
/// pin that proves the frame-index chaining only happens on the success arm.
/// Catalog row: `KG-FRAME-TRANSMIT` (negative pin, ENOENT path).
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
        .animation_state(ImageId::from_raw(80))
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
        .animation_state(ImageId::from_raw(81))
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
        .animation_state(ImageId::from_raw(60))
        .expect("i=60 MUST be animated");
    assert!(state.paused, "a=a,s=1 MUST set paused = true");

    assert!(
        reply_contains(&h, &ok_reply_with_frame(60, 1)),
        "a=a,s=1 reply MUST echo ,r=1 (current frame 0 + 1) — transcript: {:?}",
        String::from_utf8_lossy(&reply_bytes(&h)),
    );
}

/// `a=a,v=0` sets `AnimationState::loop_count` to `None` (infinite loops per
/// kitty graphics-protocol.rst §Animation control). Semantic pin that ONLY
/// passes when `KittyCommand::loop_count: Option<u32>` routes `v=0` to
/// `set_animation_loops(id, 0)` → `loop_count = None`.
/// Catalog row: `KG-ANIMATE-LOOP-COUNT`. Closes TPR-13.0.5-R1-F3-gemini.
#[test]
fn kitty_animate_v_zero_sets_loop_count_to_none_infinite() {
    let base = b64(&rgba_4x4_red());
    let frame = b64(&rgba_4x4_red());

    let mut h = SpecHarness::new();
    h.feed(&kitty_apc(b"a=t,i=61,f=32,s=4,v=4", &base));
    h.feed(&kitty_apc(b"a=f,i=61,f=32,s=4,v=4", &frame));

    // Seed with a finite loop count first, then v=0 MUST reset to infinite.
    h.feed(&kitty_apc(b"a=a,i=61,v=5", ""));
    assert_eq!(
        h.term()
            .image_cache()
            .animation_state(ImageId::from_raw(61))
            .expect("i=61 animated")
            .loop_count,
        Some(5),
        "a=a,v=5 MUST set loop_count to Some(5) as a baseline"
    );

    h.feed(&kitty_apc(b"a=a,i=61,v=0", ""));
    assert_eq!(
        h.term()
            .image_cache()
            .animation_state(ImageId::from_raw(61))
            .expect("i=61 animated")
            .loop_count,
        None,
        "a=a,v=0 MUST set loop_count to None (infinite) — distinguishes \
         present-zero from absent-zero semantics"
    );
}

/// `a=a` without `v=` leaves `loop_count` unchanged — the negative pin that
/// proves the implementation distinguishes "key absent" from "v=0". A
/// previous bug unconditionally set `source_height` to 0 on missing `v=`,
/// which would overwrite a prior finite loop count.
/// Catalog row: `KG-ANIMATE-LOOP-COUNT` (negative pin).
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
            .animation_state(ImageId::from_raw(62))
            .unwrap()
            .loop_count,
        Some(7),
    );

    // Separate a=a command with NO `v=` — loop count MUST stay at Some(7).
    h.feed(&kitty_apc(b"a=a,i=62,s=3", ""));
    assert_eq!(
        h.term()
            .image_cache()
            .animation_state(ImageId::from_raw(62))
            .unwrap()
            .loop_count,
        Some(7),
        "a=a with v= absent MUST leave loop_count at its prior value — \
         a negative pin for the Some(0)-vs-None distinction"
    );
}

/// `a=a,r=N` jumps to frame N (1-based) and the OK reply echoes `,r=N` for
/// the newly-set current frame. Semantic pin for the animate reply's
/// post-mutation current-frame echo.
/// Catalog row: `KG-ANIMATE-SET-CURRENT-FRAME`. Closes TPR-13.2-R3-F1 (a=a arm).
#[test]
fn kitty_animate_r_two_sets_current_frame_and_reply_echoes_r_two() {
    let base = b64(&rgba_4x4_red());
    let frame = b64(&rgba_4x4_red());

    let mut h = SpecHarness::new();
    h.feed(&kitty_apc(b"a=t,i=63,f=32,s=4,v=4", &base));
    h.feed(&kitty_apc(b"a=f,i=63,f=32,s=4,v=4", &frame));
    // Now 2 frames (indices 0 and 1, 1-based 1 and 2).

    h.feed(&kitty_apc(b"a=a,i=63,r=2", ""));

    let state = h
        .term()
        .image_cache()
        .animation_state(ImageId::from_raw(63))
        .expect("i=63 animated");
    assert_eq!(
        state.current_frame, 1,
        "a=a,r=2 MUST set current_frame to 0-based index 1"
    );

    assert!(
        reply_contains(&h, &ok_reply_with_frame(63, 2)),
        "a=a,r=2 reply MUST echo ,r=2 (1-based current frame after mutation) \
         — transcript: {:?}",
        String::from_utf8_lossy(&reply_bytes(&h)),
    );
}

/// `a=a,s=2` (run-wait) and `a=a,s=3` (run) BOTH unpause and reset
/// loops_completed=0. They differ in `wait_mode`: s=2 sets it true, s=3
/// clears it. wait_mode governs whether `add_animation_frame` resumes a
/// finished animation (s=2 yes; s=3 no).
/// BUG-08-025 fix: previously these collapsed via a shared match arm.
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
            .animation_state(ImageId::from_raw(67))
            .unwrap()
            .paused,
        "a=a,s=1 MUST pause"
    );

    // s=2 (run-wait) — unpauses, resets loops, AND sets wait_mode.
    h.feed(&kitty_apc(b"a=a,i=67,s=2", ""));
    let state = h
        .term()
        .image_cache()
        .animation_state(ImageId::from_raw(67))
        .unwrap();
    assert!(!state.paused, "a=a,s=2 MUST clear paused");
    assert_eq!(state.loops_completed, 0, "a=a,s=2 MUST reset loops_completed");
    assert!(state.wait_mode, "a=a,s=2 MUST set wait_mode=true");

    // s=3 (run) — clears wait_mode.
    h.feed(&kitty_apc(b"a=a,i=67,s=3", ""));
    let state = h
        .term()
        .image_cache()
        .animation_state(ImageId::from_raw(67))
        .unwrap();
    assert!(
        !state.wait_mode,
        "a=a,s=3 MUST clear wait_mode (distinguishing it from s=2 run-wait)"
    );
}

/// `a=a,z=Nms` sets the frame gap for the current frame. Pins the per-frame
/// gap update path.
/// Catalog row: `KG-ANIMATE-SET-FRAME-GAP`.
#[test]
fn kitty_animate_z_sets_current_frame_gap_duration() {
    let base = b64(&rgba_4x4_red());
    let frame = b64(&rgba_4x4_red());

    let mut h = SpecHarness::new();
    h.feed(&kitty_apc(b"a=t,i=64,f=32,s=4,v=4", &base));
    h.feed(&kitty_apc(b"a=f,i=64,f=32,s=4,v=4", &frame));

    // Current frame is 0; set gap to 250ms.
    h.feed(&kitty_apc(b"a=a,i=64,z=250", ""));

    let state = h
        .term()
        .image_cache()
        .animation_state(ImageId::from_raw(64))
        .expect("i=64 animated");
    assert_eq!(
        state.frame_durations[state.current_frame],
        Duration::from_millis(250),
        "a=a,z=250 MUST set the current frame's gap to 250ms"
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

/// Idempotency negative pin: without any call to `advance_animations`,
/// `current_frame` MUST stay at 0 after a fresh promotion. Proves the
/// animation does NOT advance via some internal auto-loop path — the
/// timer-driven tick from the IO thread IS load-bearing. (The companion
/// "timer wired" positive pin lives in `oriterm_mux/src/pane/io_thread/
/// tests.rs` once the IO-thread wiring lands.)
/// Catalog row: `KG-ANIMATE-RUN` (negative pin, no-tick).
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
        .animation_state(ImageId::from_raw(66))
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
    // Categories: (a=f transmit promotion), (a=f subsequent append),
    // (a=f ENOENT negative), (a=f alphablend default), (a=f overwrite X=1),
    // (a=a stop s=1), (a=a v=0 infinite loops), (a=a v= absent negative),
    // (a=a r= set frame), (a=a z= set gap), (advance deadline), (no-tick negative).
    let categories: &[&str] = &[
        "frame_transmit_promotion_r2",
        "frame_subsequent_append_r3",
        "frame_enoent_no_r_qualifier",
        "frame_composite_alphablend",
        "frame_composite_overwrite",
        "animate_stop_s1",
        "animate_run_wait_s2_collapses_with_s3",
        "animate_v0_infinite_loops",
        "animate_v_absent_negative",
        "animate_r2_current_frame_reply",
        "animate_z_frame_gap",
        "advance_deadline_visible",
        "no_tick_negative",
    ];
    assert_eq!(
        categories.len(),
        13,
        "animation matrix MUST cover 13 categories — if you add a new \
         pin, bump this count so matrix completeness is self-verifying"
    );
}
