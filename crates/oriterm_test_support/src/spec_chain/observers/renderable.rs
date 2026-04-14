//! Renderable rung observer (rung 4).
//!
//! Asserts that the terminal's `RenderableContent` matches expected
//! properties. Lives in `oriterm_test_support` because `RenderableContent`
//! is in `oriterm_core` — no GPU types needed.
//!
//! This is a stub — the full renderable observer (cells, palette, image
//! placements, hyperlinks, cursor, mode bits) will be expanded as pilots
//! and sections 08+ exercise it.

use oriterm_core::Term;
use oriterm_core::effect::sink::QueueingEffectSink;

use crate::spec_chain::api::RungResult;
use crate::spec_chain::scenario::{RenderableExpectation, RungName};

/// Observe the renderable rung: does `RenderableContent` match expected?
///
/// Currently a stub that always passes — expanded as pilots exercise it.
/// Takes the `Term` directly for `renderable_content()` access.
pub fn observe_renderable(
    _term: &Term<QueueingEffectSink>,
    _expected: RenderableExpectation,
) -> RungResult {
    // Stub: always passes until pilots define concrete expectations.
    // The renderable observer will check cells, cursor shape/position,
    // damage lines, image placements, hyperlink ranges, etc.
    RungResult::pass(RungName::Renderable)
}
