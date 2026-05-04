//! Self-pipe used to wake the reader thread on shutdown.
//!
//! The reader thread blocks in `poll(2)` waiting for socket readability;
//! `Drop::drop` writes a single byte into the pipe so the poll returns
//! immediately and the thread observes `alive = false`.

#[cfg(unix)]
use std::io;
#[cfg(unix)]
use std::os::fd::RawFd;

/// Create a non-blocking, close-on-exec self-pipe pair `(read_fd, write_fd)`.
///
/// On Linux, uses `pipe2(2)` to atomically set `O_NONBLOCK | O_CLOEXEC`.
/// On other Unix targets, falls back to `pipe(2)` followed by `fcntl(2)`
/// to set the same flags (still safe — only the test code path observes
/// the brief window before the flags are applied).
#[cfg(unix)]
pub(super) fn create_self_pipe() -> io::Result<(RawFd, RawFd)> {
    let mut fds = [0i32; 2];
    #[cfg(target_os = "linux")]
    #[allow(unsafe_code, reason = "pipe2(2) to create non-blocking self-pipe")]
    let ret = unsafe { libc::pipe2(fds.as_mut_ptr(), libc::O_NONBLOCK | libc::O_CLOEXEC) };
    #[cfg(not(target_os = "linux"))]
    #[allow(unsafe_code, reason = "pipe(2) to create self-pipe (non-Linux)")]
    let ret = unsafe { libc::pipe(fds.as_mut_ptr()) };
    if ret != 0 {
        return Err(io::Error::last_os_error());
    }
    #[cfg(not(target_os = "linux"))]
    for &fd in &fds {
        #[allow(unsafe_code, reason = "fcntl(2) to set O_NONBLOCK and FD_CLOEXEC")]
        unsafe {
            if libc::fcntl(fd, libc::F_SETFL, libc::O_NONBLOCK) == -1 {
                log::warn!(
                    "fcntl(F_SETFL, O_NONBLOCK) failed: {}",
                    io::Error::last_os_error()
                );
            }
            if libc::fcntl(fd, libc::F_SETFD, libc::FD_CLOEXEC) == -1 {
                log::warn!(
                    "fcntl(F_SETFD, FD_CLOEXEC) failed: {}",
                    io::Error::last_os_error()
                );
            }
        }
    }
    #[expect(
        clippy::tuple_array_conversions,
        reason = "pipe returns [i32; 2], need (RawFd, RawFd)"
    )]
    Ok((fds[0], fds[1]))
}
