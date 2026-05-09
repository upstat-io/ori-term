//! Unit tests for the icon-size registry.

use std::collections::HashSet;

use oriterm_ui::icons::IconId;

use super::WindowRenderer;

/// Every `IconId` variant appears exactly once in `ICON_SIZES`.
///
/// Prevents drift between the pre-resolution list and actual icon definitions.
/// If a new `IconId` variant is added without a corresponding `ICON_SIZES`
/// entry, this test fails.
#[test]
fn icon_sizes_covers_all_icon_ids() {
    let resolved: HashSet<IconId> = WindowRenderer::ICON_SIZES
        .iter()
        .map(|&(id, _)| id)
        .collect();
    for &id in IconId::ALL {
        assert!(
            resolved.contains(&id),
            "{id:?} missing from ICON_SIZES — add an entry in window_renderer/icons.rs"
        );
    }
    assert_eq!(
        resolved.len(),
        IconId::ALL.len(),
        "ICON_SIZES has {} entries but IconId::ALL has {} — check for duplicates",
        resolved.len(),
        IconId::ALL.len()
    );
}

/// No duplicate `(IconId, size)` pairs in `ICON_SIZES`.
#[test]
fn icon_sizes_no_duplicates() {
    let mut seen = HashSet::new();
    for &(id, size) in &WindowRenderer::ICON_SIZES {
        assert!(
            seen.insert((id, size)),
            "duplicate ICON_SIZES entry: ({id:?}, {size})"
        );
    }
}
