//! Tests for PtyEventLoop.

use std::sync::mpsc;
use std::sync::Arc;
use std::time::Duration;

use oriterm_core::{FairMutex, Term, VoidListener};

use super::{PtyEventLoop, READ_BUFFER_SIZE};
use crate::pty::{Msg, PtyConfig, spawn_pty};

#[test]
fn event_loop_processes_pty_output() {
    let config = PtyConfig::default();
    let mut handle = spawn_pty(&config).expect("spawn_pty");

    let reader = handle.take_reader().expect("reader");
    let writer = handle.take_writer().expect("writer");
    let master = handle.take_master().expect("master");

    let terminal = Arc::new(FairMutex::new(Term::new(24, 80, 1000, VoidListener)));
    let (tx, rx) = mpsc::channel();

    let event_loop = PtyEventLoop::new(
        Arc::clone(&terminal),
        reader,
        writer,
        rx,
        master,
    );

    let join = event_loop.spawn();

    // Wait for shell to produce some output (prompt).
    std::thread::sleep(Duration::from_millis(500));

    // The terminal grid should have some content from the shell.
    {
        let term = terminal.lock();
        let grid = term.grid();
        let first_row = &grid[oriterm_core::Line(0)];
        let has_content = (0..80).any(|col| {
            first_row[oriterm_core::Column(col)].ch != ' '
                && first_row[oriterm_core::Column(col)].ch != '\0'
        });
        assert!(has_content, "terminal should have content from shell");
    }

    // Kill child so read() returns EOF, then shutdown.
    let _ = handle.kill();
    let _ = handle.wait();
    let _ = tx.send(Msg::Shutdown);
    let _ = join.join();
}

#[test]
fn event_loop_shutdown_on_eof() {
    let config = PtyConfig {
        shell: Some("/bin/sh".into()),
        ..Default::default()
    };
    let mut handle = spawn_pty(&config).expect("spawn_pty");

    let reader = handle.take_reader().expect("reader");
    let writer = handle.take_writer().expect("writer");
    let master = handle.take_master().expect("master");

    let terminal = Arc::new(FairMutex::new(Term::new(24, 80, 1000, VoidListener)));
    let (_tx, rx) = mpsc::channel();

    let event_loop = PtyEventLoop::new(
        Arc::clone(&terminal),
        reader,
        writer,
        rx,
        master,
    );

    let join = event_loop.spawn();

    // Kill child — PTY read returns EOF, thread exits.
    let _ = handle.kill();
    let _ = handle.wait();

    // Thread should exit within a reasonable time.
    join.join().expect("reader thread should exit on EOF");
}

#[test]
fn read_buffer_size_is_64kb() {
    assert_eq!(READ_BUFFER_SIZE, 65536);
}
