use super::helpers::should_reextract_scratch_frame;

#[test]
fn reextracts_when_shared_scratch_belongs_to_another_pane() {
    assert!(should_reextract_scratch_frame(false, false, false));
}

#[test]
fn skips_reextract_only_when_scratch_already_matches_clean_pane() {
    assert!(!should_reextract_scratch_frame(false, false, true));
}

#[test]
fn reextracts_when_content_changed_or_frame_missing() {
    assert!(should_reextract_scratch_frame(true, false, true));
    assert!(should_reextract_scratch_frame(false, true, true));
}

/// Regression: BUG-06-056 — `render_chrome` MUST read all chrome-relevant
/// focused-pane data (`content_cols`, `content_rows`, `search`) from
/// `ChromeParams`, NOT from `ctx.frame`. Reading `ctx.frame` after the
/// per-pane scratch loop returns the last-iterated pane's state in
/// multi-pane mode, which displays wrong cols/rows in the status bar when
/// the focused pane is not last in iteration order.
///
/// See: bug-tracker/plans/BUG-06-056/section-01-root-cause-analysis.md
#[test]
fn render_chrome_source_scan_excludes_ctx_frame_chrome_field_reads() {
    let src = include_str!("../chrome.rs");
    // Search must flow through ChromeParams, not be read from ctx.frame.
    assert!(
        !src.contains("f.search.as_ref()"),
        "render_chrome must read `search` from ChromeParams, not ctx.frame.search"
    );
    // Status-bar dimensions must come from params, not ctx.frame.
    assert!(
        !src.contains("f.content_cols"),
        "render_chrome must read `content_cols` from ChromeParams, not ctx.frame"
    );
    assert!(
        !src.contains("f.content_rows"),
        "render_chrome must read `content_rows` from ChromeParams, not ctx.frame"
    );
}
