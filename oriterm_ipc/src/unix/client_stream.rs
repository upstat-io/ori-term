//! Client-side blocking IPC stream (Unix domain socket).

use std::io;
use std::net::Shutdown;
use std::os::fd::{AsRawFd, RawFd};
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::time::Duration;

/// Client-side IPC stream for blocking RPC communication.
///
/// Wraps a standard library `UnixStream` (blocking mode). The background
/// reader thread in `MuxClient` uses this for the Hello handshake and
/// subsequent read/write operations.
pub struct ClientStream(UnixStream);

impl ClientStream {
    /// Connect to a daemon at `path`.
    pub fn connect(path: &Path) -> io::Result<Self> {
        let stream = UnixStream::connect(path)?;
        Ok(Self(stream))
    }

    /// Set the read timeout for blocking reads.
    #[expect(
        clippy::needless_pass_by_ref_mut,
        reason = "API parity with Windows ClientStream"
    )]
    pub fn set_read_timeout(&mut self, timeout: Option<Duration>) -> io::Result<()> {
        self.0.set_read_timeout(timeout)
    }

    /// Set the write timeout for blocking writes.
    #[expect(
        clippy::needless_pass_by_ref_mut,
        reason = "API parity with Windows ClientStream"
    )]
    pub fn set_write_timeout(&mut self, timeout: Option<Duration>) -> io::Result<()> {
        self.0.set_write_timeout(timeout)
    }

    /// Duplicate the underlying socket handle for split-thread ownership.
    ///
    /// Returns a new `ClientStream` whose `UnixStream` is a `dup()` of this
    /// stream's fd. Both fds reference the same kernel socket; concurrent
    /// reads via one stream and writes via the other from different threads
    /// is safe — the kernel queues for read vs. write are independent.
    ///
    /// Used by the `MuxClient` transport to give a dedicated writer thread
    /// exclusive write ownership while the reader thread retains read
    /// ownership, eliminating the head-of-line block where a backpressured
    /// write starved RPC reply reads (BUG-11-047).
    pub fn try_clone(&self) -> io::Result<Self> {
        Ok(Self(self.0.try_clone()?))
    }

    /// Shut down the writing direction of the underlying socket.
    ///
    /// Used by `MuxClient::Drop` to break a writer thread that is
    /// blocked inside a backpressured `write()` so the join completes
    /// promptly even when the daemon has stopped draining (BUG-11-047
    /// follow-up — round 1 codex finding).
    ///
    /// `Shutdown::Write` is socket-scoped on Unix domain sockets, so
    /// calling it on any fd duplicate referring to the same kernel
    /// socket aborts the pending `write()` with `EPIPE`. Reads on the
    /// reader-thread duplicate are unaffected.
    pub fn shutdown_write(&self) -> io::Result<()> {
        self.0.shutdown(Shutdown::Write)
    }
}

impl AsRawFd for ClientStream {
    fn as_raw_fd(&self) -> RawFd {
        self.0.as_raw_fd()
    }
}

impl io::Read for ClientStream {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.0.read(buf)
    }
}

impl io::Write for ClientStream {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.0.flush()
    }
}
