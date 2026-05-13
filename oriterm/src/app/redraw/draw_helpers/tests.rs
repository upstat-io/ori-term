//! Tests for the per-frame redraw helpers.

use super::post_render_ui_stale;

/// Regression: BUG-06-049 — happy path with no chrome animation
/// resets ui_stale to false; the prior stale bit is consumed by the
/// successful frame.
/// See: bug-tracker/plans/BUG-06-049/00-overview.md
#[test]
fn ok_no_animation_resets_stale() {
    assert!(!post_render_ui_stale(false, false, false));
    assert!(!post_render_ui_stale(true, false, false));
}

/// Regression: BUG-06-049 — happy path with chrome animation sets
/// ui_stale to true regardless of prior state. Tab bar animation
/// keeps the cache stale across frames.
/// See: bug-tracker/plans/BUG-06-049/00-overview.md
#[test]
fn ok_with_animation_sets_stale() {
    assert!(post_render_ui_stale(false, false, true));
    assert!(post_render_ui_stale(true, false, true));
}

/// Regression: BUG-06-049 — error path with prior stale bit
/// preserves it. Pins against the pre-fix behavior where
/// `ctx.ui_stale` was clobbered to `tab_bar_animating` before
/// `render_to_surface` ran, silently dropping the prior bit on
/// `SurfaceError::Outdated`, `Lost`, `OutOfMemory`, `Other`, `Timeout`.
/// See: bug-tracker/plans/BUG-06-049/00-overview.md
#[test]
fn err_preserves_prior_stale_bit() {
    // No animation: prior stale survives the error.
    assert!(post_render_ui_stale(true, true, false));
    // Animation: result is true regardless.
    assert!(post_render_ui_stale(true, true, true));
}

/// Regression: BUG-06-049 — error path with NO prior stale bit and
/// NO animation results in false. The error did not synthesize a
/// stale signal — only the prior bit and the animation signal feed
/// into the post-fold value.
/// See: bug-tracker/plans/BUG-06-049/00-overview.md
#[test]
fn err_no_prior_stale_no_animation_returns_false() {
    assert!(!post_render_ui_stale(false, true, false));
}

/// Regression: BUG-06-049 — error path with NO prior stale bit but
/// WITH animation returns true. The animation signal is independent
/// of the error path.
/// See: bug-tracker/plans/BUG-06-049/00-overview.md
#[test]
fn err_no_prior_stale_with_animation_returns_true() {
    assert!(post_render_ui_stale(false, true, true));
}

/// Regression: BUG-06-049 — self-verifying matrix completeness:
/// every cell of the 2x2x2 truth table is exercised, and the function
/// is total.
/// See: bug-tracker/plans/BUG-06-049/00-overview.md
#[test]
fn matrix_is_total_and_self_verifying() {
    let mut count = 0;
    for prev in [false, true] {
        for err in [false, true] {
            for anim in [false, true] {
                let actual = post_render_ui_stale(prev, err, anim);
                let expected = (prev && err) || anim;
                assert_eq!(actual, expected, "prev={prev} err={err} anim={anim}");
                count += 1;
            }
        }
    }
    assert_eq!(
        count, 8,
        "must exercise all 8 cells of the 2x2x2 truth table"
    );
}
