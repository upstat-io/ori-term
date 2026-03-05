//! Pane shutdown and cleanup.
//!
//! Implements `Drop` for [`Pane`] with non-blocking cleanup: signals the
//! writer thread, kills the child process, and detaches thread handles.
//! The threads exit promptly on their own (writer on `Shutdown` message,
//! reader on EOF from the killed child). This avoids blocking the server
//! event loop during tab/window close operations.

use super::Pane;

impl Drop for Pane {
    fn drop(&mut self) {
        // 1. Signal the writer thread to stop.
        self.notifier.shutdown();

        // 2. Kill the child process to unblock any pending PTY read.
        let _ = self.pty.kill();

        // 3. Reap the child (blocking — but callers drop panes on background
        //    threads, so this doesn't block the event loop). Ensures the
        //    child process is fully terminated before PTY handles are closed,
        //    preventing rogue shell processes.
        let _ = self.pty.wait();

        // Thread handles are dropped without joining (detached). Both
        // threads exit promptly: writer on Shutdown message, reader on
        // EOF from the killed child process.
    }
}
