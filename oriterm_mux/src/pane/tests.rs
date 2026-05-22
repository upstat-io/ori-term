use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

/// Lock-free mode cache: store and load round-trip.
#[test]
fn mode_cache_round_trip() {
    let cache = Arc::new(AtomicU64::new(0));

    // Simulate IO thread updating mode bits.
    cache.store(0x1234, Ordering::Release);
    assert_eq!(cache.load(Ordering::Acquire), 0x1234);

    // Update again.
    cache.store(0x5678, Ordering::Release);
    assert_eq!(cache.load(Ordering::Acquire), 0x5678);
}

/// Cross-thread atomic visibility (simulated with sequential ops).
#[test]
fn dirty_flag_cross_thread_pattern() {
    let dirty = Arc::new(AtomicBool::new(false));
    let dirty2 = Arc::clone(&dirty);

    // "IO thread" sets dirty.
    std::thread::spawn(move || {
        dirty2.store(true, Ordering::Release);
    })
    .join()
    .unwrap();

    // "Main thread" reads dirty.
    assert!(dirty.load(Ordering::Acquire));
}

/// Unseen output flag: set and clear round-trip (simulated with bool).
/// Mirrors the `has_bell` pattern: set on background output, clear on focus.
#[test]
fn unseen_output_set_and_clear() {
    // Starts false (no unseen output).
    let flag = AtomicBool::new(false);
    assert!(!flag.load(Ordering::Acquire));

    // Background output arrives → set.
    flag.store(true, Ordering::Release);
    assert!(flag.load(Ordering::Acquire));

    // Idempotent: setting again is harmless.
    flag.store(true, Ordering::Release);
    assert!(flag.load(Ordering::Acquire));

    // Pane gains focus → clear.
    flag.store(false, Ordering::Release);
    assert!(!flag.load(Ordering::Acquire));
}

/// Selection-dirty flag: swap-based clear returns previous value.
#[test]
fn selection_dirty_swap_clear() {
    let flag = Arc::new(AtomicBool::new(false));

    // IO thread sets dirty.
    flag.store(true, Ordering::Release);
    assert!(flag.load(Ordering::Acquire));

    // Main thread clears via swap — gets true back.
    let was_dirty = flag.swap(false, Ordering::AcqRel);
    assert!(was_dirty);
    assert!(!flag.load(Ordering::Acquire));
}

/// Regression: `resolve_target_pgid` with no master fd must fall
/// back to the shell PID and report `resolved_via_tcgetpgrp = false` so the
/// caller does NOT apply ESRCH-as-success on the fallback path.
#[cfg(unix)]
#[test]
fn resolve_target_pgid_with_no_master_fd_returns_shell_pid() {
    let (pgid, resolved_via_tcgetpgrp) = super::resolve_target_pgid(12345, None);
    assert_eq!(pgid, 12345);
    assert!(
        !resolved_via_tcgetpgrp,
        "no master_fd must report resolved_via_tcgetpgrp = false"
    );
}

/// Regression: `resolve_target_pgid` with a non-TTY master fd
/// (a pipe) must fall back to the shell PID. `tcgetpgrp` on a pipe returns
/// -1 with `ENOTTY`; the helper must report `resolved_via_tcgetpgrp = false`
/// so the caller does NOT apply ESRCH-as-success on the fallback path.
#[cfg(unix)]
#[test]
#[allow(
    unsafe_code,
    reason = "libc::pipe + OwnedFd::from_raw_fd require unsafe"
)]
fn resolve_target_pgid_with_non_tty_master_fd_falls_back_to_shell_pid() {
    use std::io;
    use std::os::unix::io::{FromRawFd, OwnedFd};
    let mut fds = [0_i32; 2];
    // SAFETY: pipe() is a standard POSIX syscall; we own both fds and
    // immediately wrap them in OwnedFd which closes on drop.
    let result = unsafe { libc::pipe(fds.as_mut_ptr()) };
    assert_eq!(result, 0, "pipe() failed: {}", io::Error::last_os_error());
    // SAFETY: OwnedFd takes ownership of both fds and drops them.
    let read_end = unsafe { OwnedFd::from_raw_fd(fds[0]) };
    let _write_end = unsafe { OwnedFd::from_raw_fd(fds[1]) };
    let (pgid, resolved_via_tcgetpgrp) = super::resolve_target_pgid(12345, Some(&read_end));
    assert_eq!(pgid, 12345, "non-TTY fd must fall back to shell PID");
    assert!(
        !resolved_via_tcgetpgrp,
        "non-TTY fd must report fallback, not tcgetpgrp success"
    );
}
