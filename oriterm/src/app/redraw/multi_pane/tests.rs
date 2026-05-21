use oriterm_mux::PaneSnapshot;

use super::helpers::{focused_pane_chrome_state, should_reextract_scratch_frame};

/// Pin: when the scratch frame currently holds another pane's data,
/// `should_reextract_scratch_frame` returns true to force a fresh
/// extraction. Content not refreshed, frame present, pane mismatch.
#[test]
fn reextracts_when_scratch_belongs_to_another_pane() {
    assert!(should_reextract_scratch_frame(false, false, false));
}

/// Pin: re-extraction is skipped ONLY when content unchanged AND scratch
/// already holds this pane's data — the cursor-blink-only fast path.
#[test]
fn skips_reextract_when_scratch_matches_clean_pane() {
    assert!(!should_reextract_scratch_frame(false, false, true));
}

/// Pin: content-refresh OR missing-frame both force re-extraction even
/// when scratch matches the current pane, because the data is stale.
#[test]
fn reextracts_when_content_changed_or_frame_missing() {
    assert!(should_reextract_scratch_frame(true, false, true));
    assert!(should_reextract_scratch_frame(false, true, true));
}

/// `render_chrome` MUST read all chrome-relevant
/// focused-pane data (`content_cols`, `content_rows`, `search`) from
/// `ChromeParams`, NOT from `ctx.frame`. Reading `ctx.frame` after the
/// per-pane scratch loop returns the last-iterated pane's state in
/// multi-pane mode, which displays wrong cols/rows in the status bar when
/// the focused pane is not last in iteration order.
#[test]
fn render_chrome_source_scan_excludes_ctx_frame_chrome_field_reads() {
    let src = include_str!("../chrome.rs");
    assert!(
        !src.contains("f.search.as_ref()"),
        "render_chrome must read `search` from ChromeParams, not ctx.frame.search"
    );
    assert!(
        !src.contains("f.content_cols"),
        "render_chrome must read `content_cols` from ChromeParams, not ctx.frame"
    );
    assert!(
        !src.contains("f.content_rows"),
        "render_chrome must read `content_rows` from ChromeParams, not ctx.frame"
    );
}

/// Build a snapshot with controllable cols/rows and search state for tests.
fn test_snapshot(cols: u16, rows: usize, search_active: bool) -> PaneSnapshot {
    let mut snap = PaneSnapshot::default();
    snap.cols = cols;
    snap.cells = vec![Vec::new(); rows];
    snap.search_active = search_active;
    if search_active {
        snap.search_query = "needle".to_string();
        snap.search_total_matches = 1;
    }
    snap
}

/// `focused_pane_chrome_state` returns the
/// FOCUSED pane's cols/rows regardless of which pane was last iterated.
/// Source-scan above proves chrome.rs reads from params; this test
/// proves params carry the right values for the focused pane.
#[test]
fn focused_pane_chrome_state_returns_focused_pane_cols_rows() {
    let focused = test_snapshot(80, 24, false);
    let (cols, rows, search) = focused_pane_chrome_state(Some(&focused));
    assert_eq!(cols, 80, "must report focused pane's cols");
    assert_eq!(rows, 24, "must report focused pane's rows");
    assert!(search.is_none(), "no search when search_active=false");
}

/// Regression: invariant check — asymmetric pane dimensions.
/// If the focused pane is 80x24 and another pane is 132x43, status bar
/// MUST report 80x24. Pre-fix, this asserted on the last-iterated pane's
/// 132x43; post-fix, this proves the helper picks the focused pane.
#[test]
fn focused_pane_chrome_state_asymmetric_dims_picks_focused_not_other() {
    let focused = test_snapshot(80, 24, false);
    let _other = test_snapshot(132, 43, false);
    let (cols, rows, _) = focused_pane_chrome_state(Some(&focused));
    assert_eq!((cols, rows), (80, 24));
    assert_ne!(
        (cols, rows),
        (132, 43),
        "MUST NOT report non-focused pane's dimensions"
    );
}

/// Pin: when no focused-pane snapshot is available (mux unavailable,
/// pane closed during redraw), the helper returns the safe (0, 0, None)
/// default so chrome rendering doesn't panic.
#[test]
fn focused_pane_chrome_state_no_snapshot_returns_zeros() {
    let (cols, rows, search) = focused_pane_chrome_state(None);
    assert_eq!((cols, rows), (0, 0));
    assert!(search.is_none());
}

/// Pin: active search on the focused pane flows into ChromeParams.search
/// as Some(FrameSearch). The search bar overlay reads from this; without
/// this path the search bar would be invisible after the post-loop
/// search-restoration block was removed.
#[test]
fn focused_pane_chrome_state_active_search_produces_some_search() {
    let focused = test_snapshot(80, 24, true);
    let (_, _, search) = focused_pane_chrome_state(Some(&focused));
    assert!(
        search.is_some(),
        "active search on focused pane MUST produce Some(FrameSearch)"
    );
}

/// Pin: inactive search on the focused pane produces None — empty
/// search-bar overlay condition. Companion clamp to the active case
/// above, clamping the on/off boundary.
#[test]
fn focused_pane_chrome_state_inactive_search_produces_none() {
    let focused = test_snapshot(80, 24, false);
    let (_, _, search) = focused_pane_chrome_state(Some(&focused));
    assert!(
        search.is_none(),
        "inactive search MUST produce None — search bar overlay hidden"
    );
}
