//! Test infrastructure for widget integration testing.
//!
//! Provides [`MockMeasurer`] for deterministic text measurement,
//! [`test_theme`] for consistent theming, and [`WidgetTestHarness`]
//! for headless widget integration testing. Gated behind
//! `#[cfg(any(test, feature = "testing"))]` so test infrastructure
//! is not shipped in release builds.

mod harness;
mod harness_dispatch;
mod harness_input;
mod harness_inspect;
mod mock_measurer;
mod query;
pub mod render_assert;

pub use harness::WidgetTestHarness;
pub use harness_inspect::WidgetRef;
pub use mock_measurer::MockMeasurer;

use crate::theme::UiTheme;

/// Shared dark theme constant for widget tests.
pub const TEST_THEME: UiTheme = UiTheme::dark();

/// Returns a dark theme suitable for widget tests.
pub fn test_theme() -> UiTheme {
    UiTheme::dark()
}

#[cfg(test)]
mod tests;
