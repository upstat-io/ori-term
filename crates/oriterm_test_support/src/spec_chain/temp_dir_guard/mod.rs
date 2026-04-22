//! RAII temp-dir guard for `spec_chain` tests.
//!
//! Wraps `tempfile::TempDir` with a named prefix for debuggability and
//! exposes a borrowed path accessor. The inner `TempDir::drop` removes
//! the directory on every exit path — normal return, `?` error
//! propagation, and panic unwinding — which is the invariant
//! `.claude/rules/code-hygiene.md` §Temporal Coupling & RAII Guards
//! requires for paired setup/cleanup operations.
//!
//! Consumers should hold the guard for the full test lifetime and let
//! `Drop` handle cleanup; never call manual `fs::remove_*` tails that
//! only run on the happy path.

use std::path::Path;

use tempfile::{Builder, TempDir};

/// RAII wrapper around `tempfile::TempDir` for `spec_chain` fixtures.
///
/// The guard owns a unique `/tmp`-backed directory for the test
/// lifetime; dropping the guard removes the directory via
/// `tempfile::TempDir::drop`, which runs on every exit path including
/// panic unwinding.
pub struct TempDirGuard {
    dir: TempDir,
}

impl TempDirGuard {
    /// Create a new temp directory under `std::env::temp_dir()` with the
    /// given human-readable prefix. The actual dir name appends a random
    /// suffix chosen by `tempfile`, so parallel `cargo test` threads
    /// never collide.
    pub fn new(prefix: &str) -> Self {
        let dir = Builder::new()
            .prefix(&format!("oriterm_spec_{prefix}_"))
            .tempdir()
            .expect("create spec_chain tempdir");
        Self { dir }
    }

    /// Borrow the guard's on-disk path. The path is valid only while the
    /// guard is alive; once it drops, the directory is removed.
    pub fn path(&self) -> &Path {
        self.dir.path()
    }
}

#[cfg(test)]
mod tests;
