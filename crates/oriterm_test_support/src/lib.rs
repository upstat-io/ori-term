//! Shared PTY/Term/VTE test driver for oriterm conformance suites.
//!
//! This crate is a dev-time helper, never published. It provides a single
//! canonical [`PtySession`] type used by:
//!   - `oriterm_core/tests/vttest/` (text-grid snapshot tests)
//!   - `oriterm/src/gpu/visual_regression/vttest/` (GPU golden image tests)
//!   - `oriterm_core/tests/tack/` (tack scenario catalog — Section 04+)
//!
//! Before this crate existed, the PTY/Term/VTE plumbing was duplicated
//! byte-for-byte between two `VtTestSession` definitions. See
//! `plans/tack-conformance/section-01-shared-pty-session.md` for the
//! deduplication history.

pub mod session;

pub use session::{PtyResponder, PtySession, tool_available, vttest_available};
