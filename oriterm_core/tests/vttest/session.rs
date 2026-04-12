//! Adapter that re-exports the shared `PtySession` infrastructure so
//! existing vttest menu tests don't have to change their import paths
//! beyond `super::session::*`.
//!
//! The session implementation lives in `crates/oriterm_test_support`.

pub use oriterm_test_support::{PtySession, vttest_available};
