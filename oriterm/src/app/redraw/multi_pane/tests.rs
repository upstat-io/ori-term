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
