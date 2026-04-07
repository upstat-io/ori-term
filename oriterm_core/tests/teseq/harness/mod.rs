//! Teseq test harness — scenario loading, event capture, and assertions.

#![allow(unused_imports)]

pub mod assertions;
pub mod events;
pub mod loader;
pub mod reseq;
pub mod runner;

pub use assertions::{
    analyze_response, assert_cell_flags_contain, assert_cell_flags_not_contain, assert_cursor,
    assert_event_snapshot, assert_grid_cols, assert_grid_snapshot, assert_mode_contains,
    assert_mode_not_contains, assert_pty_writes, assert_response_snapshot, assert_scrollback_empty,
    assert_spec, cell_bg_at, cell_fg_at, cell_underline_color_at, pipe_through_command,
};
pub use events::{RecordedEvent, RecordedListener};
pub use loader::ScenarioSpec;
pub use reseq::{compile_teseq, reseq_available, teseq_available};
pub use runner::{ScenarioOutcome, TeseqHarness};

/// Compute DA2 version number from `CARGO_PKG_VERSION`.
///
/// Replicates `crate_version_number()` from `handler/helpers.rs` so test
/// assertions track version bumps automatically. Do NOT hardcode a version
/// number — it changes on every release.
pub fn compute_da2_version() -> usize {
    let mut result = 0usize;
    let version = env!("CARGO_PKG_VERSION");
    let version = version.split('-').next().unwrap_or(version);
    for (i, part) in version.split('.').rev().enumerate() {
        let n = part.parse::<usize>().unwrap_or(0);
        result += n * 100usize.pow(i as u32);
    }
    result
}
