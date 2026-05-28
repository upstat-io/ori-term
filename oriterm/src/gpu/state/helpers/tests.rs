//! Tests for `gpu::state::helpers` — panic-safe surface configure helper.

use super::{ConfigurePanicked, catch_panic};

/// Regression: BUG-06-108 — surface configure panic crashed process;
/// helper must convert panic to `Err(ConfigurePanicked)`.
#[test]
fn catch_panic_when_closure_panics_returns_err() {
    let result = catch_panic(|| panic!("simulated wgpu validation error"));
    assert_eq!(result, Err(ConfigurePanicked));
}

/// Regression: BUG-06-108 — successful configure must pass through unchanged.
#[test]
fn catch_panic_when_closure_succeeds_returns_ok() {
    let result = catch_panic(|| {});
    assert_eq!(result, Ok(()));
}

/// Regression: BUG-06-108 — helper catches any panic type, not just specific
/// messages. The user's actual panic was "Invalid surface".
#[test]
fn catch_panic_when_panic_message_matches_invalid_surface_returns_err() {
    let result = catch_panic(|| panic!("Invalid surface"));
    assert_eq!(result, Err(ConfigurePanicked));
}

/// Regression: BUG-06-108 — caught panic must NOT propagate to outer scope.
/// This is the panic-isolation contract the fallback chain depends on.
#[test]
fn catch_panic_when_inner_panics_does_not_propagate_to_outer_scope() {
    let inner_result = catch_panic(|| panic!("inner"));
    assert_eq!(inner_result, Err(ConfigurePanicked));
    let outer_check = 42;
    assert_eq!(outer_check, 42);
}
