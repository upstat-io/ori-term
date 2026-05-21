//! §13.6.1 item 16: eviction-filter realpath gate.
//!
//! §13.6's cross-stack eviction tests (canonical home
//! `oriterm_core/tests/spec_chain/kitty/cross_stack_regression.rs`) MUST
//! drive real protocol byte streams through the production handler — they
//! must NOT synthesize cache entries directly via `ImageCache::store` or
//! `ImageCache::insert`. The eviction-filter at
//! `oriterm_core/src/image/cache/eviction.rs:25-42` skips
//! placed-or-anchored images; synthesizing cache entries without exercising
//! the protocol's placement-create path leaves the entry unplaced, so the
//! eviction test passes vacuously (the synthesized entry was always
//! eligible for eviction; nothing was clamped by the real
//! `placed.contains(id)` check).
//!
//! This gate scans the cross-stack regression test file (when it exists)
//! and asserts zero direct cache-mutation calls. The file is authored
//! under §13.6 — until then, this gate is vacuously clean (target file
//! absent). The contract: when §13.6 lands the file, this gate fires on
//! any drift into the synthesize-cache anti-pattern.
//!
//! Plan body cross-reference:
//! `plans/spec-conformance/section-13-kitty-graphics.md §13.6` line 541
//! (`cross_stack_eviction_tests_drive_real_protocol_bytes`).

use std::path::PathBuf;

/// Architectural pin: cross-stack eviction tests must drive real protocol
/// bytes; direct `ImageCache::store|insert` calls in the cross-stack
/// regression file are BANNED.
///
/// Today, the cross-stack regression file does not yet exist (§13.6
/// territory). The gate's purpose is to FIRE THE MOMENT the file is
/// authored if it contains the banned shape — pre-emptive cohesion guard.
#[test]
fn cross_stack_eviction_tests_drive_real_protocol_bytes() {
    let path = locate_cross_stack_regression_file();

    if !path.exists() {
        // §13.6 has not authored the file yet. Gate is vacuously clean.
        // When the file is authored, this test re-fires automatically.
        eprintln!(
            "SKIP: {} does not exist yet — §13.6 has not authored the \
             cross-stack regression file. Gate remains operational and \
             will fire on the first commit that introduces the banned \
             shape.",
            path.display()
        );
        return;
    }

    let contents = std::fs::read_to_string(&path).unwrap_or_else(|err| {
        panic!(
            "failed to read {} for cross-stack regression gate: {err}",
            path.display()
        )
    });

    let banned_patterns = [
        "ImageCache::store",
        "ImageCache::insert",
        ".image_cache_mut().store(",
        ".image_cache_mut().insert(",
        ".image_cache().store(",
    ];

    let mut hits: Vec<(usize, &str, &str)> = Vec::new();
    for (lineno, line) in contents.lines().enumerate() {
        // Skip comment-prefix lines (`//`, `///`, `//!`) — docstrings
        // explaining the contract legitimately mention the banned API
        // names; the gate's purpose is to catch real code calls.
        if line.trim_start().starts_with("//") {
            continue;
        }
        for pat in &banned_patterns {
            if line.contains(pat) {
                hits.push((lineno + 1, pat, line.trim()));
            }
        }
    }

    assert!(
        hits.is_empty(),
        "§13.6.1 item 16 BANNED PATTERN: {} contains direct cache-mutation \
         calls that bypass the production placement path.\n\
         Cross-stack eviction tests MUST drive `\\x1b_G...` (kitty) and \
         `\\x1bP...q...` (sixel) byte streams through the production \
         protocol handler so the eviction-filter's `placed.contains(id)` \
         check sees actual placed images. Synthesizing entries via \
         `ImageCache::store` leaves them unplaced — the eviction test \
         passes vacuously.\n\
         Hits:\n{}",
        path.display(),
        hits.iter()
            .map(|(line, pat, src)| format!("  line {line}: {pat} in `{src}`"))
            .collect::<Vec<_>>()
            .join("\n")
    );
}

/// Locate the cross-stack regression file relative to the workspace root.
/// Uses `oriterm_test_support::paths::term_workspace_root()` — the
/// canonical path-discovery helper for tests that cross the
/// term_repo/wrapper boundary.
fn locate_cross_stack_regression_file() -> PathBuf {
    oriterm_test_support::paths::term_workspace_root()
        .join("oriterm_core/tests/spec_chain/kitty/cross_stack_regression.rs")
}
