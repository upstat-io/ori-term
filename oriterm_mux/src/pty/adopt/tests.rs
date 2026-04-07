//! Tests for `AdoptedPtyHandle` and `AdoptedSignal`.
//!
//! Phase 1B of Section 03.9: verify the adopt path's API contract before
//! the COM server in Phase 3 wires it to real handles. Tests are
//! cross-platform — `AdoptedSignal` is a stub on non-Windows.

use std::io::{Read, Write};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use super::{AdoptedPtyHandle, AdoptedSignal};
use crate::pty::PtyLifecycle;

/// Build an `AdoptedPtyHandle` with a fixed-bytes reader, in-memory writer,
/// stub signal, and a known PID.
fn handle_with_fixture(client_pid: u32) -> AdoptedPtyHandle {
    let reader: Box<dyn Read + Send> = Box::new(std::io::Cursor::new(b"hello".to_vec()));
    let writer: Box<dyn Write + Send> =
        Box::new(MutexWriter(Arc::new(Mutex::new(Vec::<u8>::new()))));
    let signal = AdoptedSignal::stub_for_tests();
    AdoptedPtyHandle::new(reader, writer, signal, Some(client_pid))
}

#[test]
fn new_handle_exposes_all_take_methods() {
    let mut handle = handle_with_fixture(1234);

    assert!(
        handle.take_reader().is_some(),
        "first take_reader returns Some"
    );
    assert!(
        handle.take_writer().is_some(),
        "first take_writer returns Some"
    );
    assert!(
        handle.take_signal().is_some(),
        "first take_signal returns Some"
    );
}

#[test]
fn take_methods_return_none_after_first_take() {
    let mut handle = handle_with_fixture(42);

    let _ = handle.take_reader();
    let _ = handle.take_writer();
    let _ = handle.take_signal();

    assert!(
        handle.take_reader().is_none(),
        "second take_reader returns None"
    );
    assert!(
        handle.take_writer().is_none(),
        "second take_writer returns None"
    );
    assert!(
        handle.take_signal().is_none(),
        "second take_signal returns None"
    );
}

#[test]
fn process_id_round_trips_constructor_value() {
    let handle = handle_with_fixture(9999);
    assert_eq!(handle.process_id(), Some(9999));
}

#[test]
fn process_id_returns_none_when_constructor_passed_none() {
    let reader: Box<dyn Read + Send> = Box::new(std::io::Cursor::new(Vec::new()));
    let writer: Box<dyn Write + Send> = Box::new(Vec::<u8>::new());
    let signal = AdoptedSignal::stub_for_tests();
    let handle = AdoptedPtyHandle::new(reader, writer, signal, None);
    assert_eq!(handle.process_id(), None);
}

#[test]
fn pty_lifecycle_kill_is_no_op() {
    // Semantic pin: ori_term did not spawn the process for an adopted PTY,
    // so `kill()` is intentionally a no-op that returns `Ok(())`. This
    // test ONLY passes if `PtyLifecycle::kill` is implemented as such —
    // if it accidentally calls `unimplemented!()` or returns an error, the
    // assertion fails.
    let mut handle = handle_with_fixture(1);
    assert!(
        PtyLifecycle::kill(&mut handle).is_ok(),
        "AdoptedPtyHandle::kill must be a successful no-op"
    );
}

#[test]
fn pty_lifecycle_try_wait_returns_none_before_signal() {
    let mut handle = handle_with_fixture(1);
    let result = PtyLifecycle::try_wait(&mut handle).expect("try_wait must not error");
    assert!(
        result.is_none(),
        "try_wait must return None before exit is signaled"
    );
}

#[test]
fn pty_lifecycle_try_wait_returns_some_after_signal() {
    let mut handle = handle_with_fixture(1);
    let signal = handle.clone_exit_signal();

    // Signal exit synchronously.
    AdoptedPtyHandle::deliver_exit(&signal);

    let result = PtyLifecycle::try_wait(&mut handle).expect("try_wait must not error");
    assert!(
        result.is_some(),
        "try_wait must return Some after exit is signaled"
    );
}

#[test]
fn pty_lifecycle_wait_blocks_until_signal_then_unblocks() {
    let mut handle = handle_with_fixture(1);
    let signal = handle.clone_exit_signal();

    // Spawn a helper that signals exit after 50ms.
    let helper = thread::spawn(move || {
        thread::sleep(Duration::from_millis(50));
        AdoptedPtyHandle::deliver_exit(&signal);
    });

    let started = Instant::now();
    let _ = PtyLifecycle::wait(&mut handle).expect("wait must not error");
    let elapsed = started.elapsed();

    helper.join().expect("helper thread panicked");

    assert!(
        elapsed >= Duration::from_millis(40),
        "wait must block for at least ~50ms (got {elapsed:?})"
    );
    assert!(
        elapsed < Duration::from_secs(2),
        "wait must unblock promptly after signal (got {elapsed:?})"
    );
}

#[test]
fn wait_returns_immediately_if_signal_already_delivered() {
    let mut handle = handle_with_fixture(1);
    let signal = handle.clone_exit_signal();
    AdoptedPtyHandle::deliver_exit(&signal);

    let started = Instant::now();
    let _ = PtyLifecycle::wait(&mut handle).expect("wait must not error");
    let elapsed = started.elapsed();

    assert!(
        elapsed < Duration::from_millis(50),
        "wait must return immediately when exit was pre-signaled (got {elapsed:?})"
    );
}

#[test]
fn taken_reader_yields_constructor_bytes() {
    // Semantic pin: the boxed reader stored in `AdoptedPtyHandle` must be
    // returned unchanged via `take_reader()`. If `new()` accidentally
    // wraps or substitutes the reader, the bytes won't match.
    let mut handle = handle_with_fixture(1);
    let mut reader = handle.take_reader().expect("reader present");
    let mut buf = Vec::new();
    reader.read_to_end(&mut buf).expect("read_to_end");
    assert_eq!(buf, b"hello");
}

#[test]
fn taken_writer_accepts_writes() {
    let mut handle = handle_with_fixture(1);
    let mut writer = handle.take_writer().expect("writer present");
    writer.write_all(b"world").expect("write_all");
    writer.flush().expect("flush");
}

#[test]
fn adopted_signal_resize_with_null_handle_errors() {
    // Semantic pin: AdoptedSignal::resize must reject the test stub
    // (null signal handle) so a misconfigured production wiring fails
    // loudly instead of silently no-op'ing every resize. The real
    // signal pipe wiring is tested via the IO thread integration in
    // pane/io_thread/tests.rs after process_resize routes through
    // AdoptedSignal — we can't write to a real conhost signal pipe
    // without conhost on the other end.
    let signal = AdoptedSignal::stub_for_tests();
    let result = signal.resize(24, 80);
    assert!(
        result.is_err(),
        "resize must error on a null signal handle, got {result:?}",
    );
}

/// Mutex-wrapped writer used in cross-thread tests where the test body
/// needs to inspect what was written. Implements `io::Write` by appending
/// under the mutex; the inner buffer can be cloned out via the `Arc`.
pub(crate) struct MutexWriter(pub Arc<Mutex<Vec<u8>>>);

impl Write for MutexWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0
            .lock()
            .map_err(|_| std::io::Error::other("MutexWriter poisoned"))?
            .extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
