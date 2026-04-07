//! `PtyLifecycle` trait — unifies child-process lifecycle across spawned and
//! adopted PTYs.
//!
//! Spawned PTYs ([`PtyHandle`](super::PtyHandle)) own a `portable_pty::Child`
//! and delegate `kill`/`wait`/`process_id` to it. Adopted PTYs (introduced
//! for Section 03.9 Windows Default Terminal handoff) receive pre-existing
//! OS handles from the console host and have no spawned child of their
//! own — the console host owns the child process lifecycle.
//!
//! Both shapes need a uniform interface so [`Pane`](crate::pane::Pane) can
//! own either kind through a single trait object.

use std::io;

use super::ExitStatus;

/// Child-process lifecycle for a PTY-backed pane.
///
/// Implementors:
/// - [`PtyHandle`](super::PtyHandle): delegates to its owned `portable_pty::Child`.
/// - `AdoptedPtyHandle` (Phase 1B): no owned child; lifecycle is driven by
///   the console host on the other end of the adopted handles. `kill()` is a
///   no-op, `wait()` blocks until the reader thread observes EOF.
///
/// All methods take `&mut self` because the underlying child handle requires
/// exclusive access in `portable-pty` (the `Child` trait is `&mut self`-based).
pub trait PtyLifecycle: Send {
    /// Kill the child process.
    ///
    /// For spawned PTYs, terminates the child via `Child::kill()`. For
    /// adopted PTYs, returns `Ok(())` because `ori_term` did not spawn the
    /// process — the console host owns the child lifecycle.
    fn kill(&mut self) -> io::Result<()>;

    /// Wait for the child process to exit (blocking).
    ///
    /// For spawned PTYs, blocks on `Child::wait()`. For adopted PTYs, blocks
    /// on an internal event signaled when the reader thread observes EOF.
    fn wait(&mut self) -> io::Result<ExitStatus>;

    /// Non-blocking check for child exit.
    ///
    /// Returns `Ok(Some(status))` if the child has exited, `Ok(None)` if
    /// still running, or `Err` on failure.
    #[allow(dead_code, reason = "used when pane reports child exit to UI")]
    fn try_wait(&mut self) -> io::Result<Option<ExitStatus>>;

    /// Get the child process ID, if available.
    ///
    /// Used for direct signal delivery when the PTY writer is stalled.
    fn process_id(&self) -> Option<u32>;
}
