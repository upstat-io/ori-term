//! Teseq test harness — scenario loading, event capture, and assertions.

#![allow(unused_imports)]

pub mod assertions;
pub mod events;
pub mod loader;
pub mod reseq;
pub mod runner;

pub use assertions::{
    analyze_response, assert_cursor, assert_event_snapshot, assert_grid_snapshot,
    assert_pty_writes, assert_response_snapshot, assert_scrollback_empty, assert_spec,
    pipe_through_command,
};
pub use events::{RecordedEvent, RecordedListener};
pub use loader::ScenarioSpec;
pub use reseq::{compile_teseq, reseq_available, teseq_available};
pub use runner::{ScenarioOutcome, TeseqHarness};
