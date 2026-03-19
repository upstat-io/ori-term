use super::should_reextract_multi_pane_scratch;

#[test]
fn reextracts_when_shared_scratch_belongs_to_another_pane() {
    assert!(should_reextract_multi_pane_scratch(false, false, false));
}

#[test]
fn skips_reextract_only_when_scratch_already_matches_clean_pane() {
    assert!(!should_reextract_multi_pane_scratch(false, false, true));
}

#[test]
fn reextracts_when_content_changed_or_frame_missing() {
    assert!(should_reextract_multi_pane_scratch(true, false, true));
    assert!(should_reextract_multi_pane_scratch(false, true, true));
}
