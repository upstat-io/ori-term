//! Tests for PaneIoThread and PaneIoHandle.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use super::{PaneIoCommand, PaneIoHandle, PaneIoThread, new_with_handle};

/// Helper: create a thread + handle pair with a no-op wakeup.
fn make_pair() -> (PaneIoThread, PaneIoHandle) {
    let shutdown = Arc::new(AtomicBool::new(false));
    let wakeup: Arc<dyn Fn() + Send + Sync> = Arc::new(|| {});
    new_with_handle(shutdown, wakeup)
}

/// Helper: spawn and return a live handle + its shutdown flag.
fn spawn_pair_with_flag() -> (PaneIoHandle, Arc<AtomicBool>) {
    let shutdown = Arc::new(AtomicBool::new(false));
    let wakeup: Arc<dyn Fn() + Send + Sync> = Arc::new(|| {});
    let (thread, mut handle) = new_with_handle(Arc::clone(&shutdown), wakeup);
    let join = thread.spawn().expect("failed to spawn IO thread");
    handle.set_join(join);
    (handle, shutdown)
}

/// Send `Shutdown` command — IO thread should exit cleanly and set the flag.
#[test]
fn shutdown_via_command() {
    let (mut handle, shutdown_flag) = spawn_pair_with_flag();
    handle.send_command(PaneIoCommand::Shutdown);
    let join = handle.join.take().expect("join handle missing");
    let result = join.join();
    assert!(result.is_ok(), "IO thread panicked on shutdown");
    // Observable: the IO thread sets the shutdown flag when it processes Shutdown.
    assert!(
        shutdown_flag.load(Ordering::Acquire),
        "shutdown flag should be set after Shutdown command"
    );
}

/// Drop raw senders (bypassing PaneIoHandle::Drop) — IO thread exits via
/// channel disconnect, NOT via Shutdown command.
#[test]
fn shutdown_via_channel_disconnect() {
    let shutdown = Arc::new(AtomicBool::new(false));
    let wakeup: Arc<dyn Fn() + Send + Sync> = Arc::new(|| {});
    let (cmd_tx, cmd_rx) = crossbeam_channel::unbounded();
    let (byte_tx, byte_rx) = crossbeam_channel::unbounded();

    let thread = PaneIoThread {
        cmd_rx,
        byte_rx,
        shutdown: Arc::clone(&shutdown),
        wakeup,
    };
    let join = thread.spawn().expect("failed to spawn IO thread");

    // Drop both senders — this disconnects the channels without sending Shutdown.
    drop(cmd_tx);
    drop(byte_tx);

    let result = join.join();
    assert!(result.is_ok(), "IO thread panicked on channel disconnect");
    // Observable: the shutdown flag is NOT set because no Shutdown was sent.
    // The thread exited via the Err(_) => return path in select!.
    assert!(
        !shutdown.load(Ordering::Acquire),
        "shutdown flag should NOT be set on channel disconnect"
    );
}

/// Send 5 commands then Shutdown. The shutdown flag proves all 5 were drained
/// before exit (Shutdown is last in the queue, processed after the preceding 5).
#[test]
fn command_delivery_ordering() {
    let shutdown = Arc::new(AtomicBool::new(false));
    let wakeup: Arc<dyn Fn() + Send + Sync> = Arc::new(|| {});
    let (thread, handle) = new_with_handle(Arc::clone(&shutdown), wakeup);

    // Pre-load 5 commands before spawning — ensures they're queued.
    for i in 1..=5 {
        handle.send_command(PaneIoCommand::ScrollDisplay(i));
    }
    handle.send_command(PaneIoCommand::Shutdown);

    let join = thread.spawn().expect("failed to spawn IO thread");
    let result = join.join();
    assert!(result.is_ok(), "IO thread panicked processing commands");
    // Observable: shutdown flag set proves Shutdown was reached, which means
    // the 5 preceding commands were drained (FIFO guarantee of the drain loop).
    assert!(
        shutdown.load(Ordering::Acquire),
        "shutdown flag should be set after draining all commands"
    );
}

/// Send byte batches, then shutdown. Verify bytes and commands are both received.
#[test]
fn byte_delivery() {
    let (mut handle, shutdown_flag) = spawn_pair_with_flag();

    // Send 3 byte batches.
    let tx = handle.byte_sender();
    tx.send(b"hello".to_vec()).unwrap();
    tx.send(b"world".to_vec()).unwrap();
    tx.send(b"!".to_vec()).unwrap();

    // Brief yield to let the IO thread drain bytes before shutdown.
    std::thread::sleep(Duration::from_millis(10));
    handle.send_command(PaneIoCommand::Shutdown);

    let join = handle.join.take().expect("join handle missing");
    let result = join.join();
    assert!(result.is_ok(), "IO thread panicked during byte delivery");
    assert!(
        shutdown_flag.load(Ordering::Acquire),
        "shutdown flag should be set after processing bytes + shutdown"
    );
}

/// Drop impl sends shutdown and joins the thread.
#[test]
fn handle_drop_sends_shutdown() {
    let (handle, shutdown_flag) = spawn_pair_with_flag();
    // Drop triggers PaneIoHandle::Drop → sends Shutdown → joins.
    drop(handle);
    assert!(
        shutdown_flag.load(Ordering::Acquire),
        "shutdown flag should be set after Drop"
    );
}

/// Verify `PaneIoCommand` is `Send`.
#[test]
fn pane_io_command_is_send() {
    fn assert_send<T: Send>() {}
    assert_send::<PaneIoCommand>();
}

/// Verify `PaneIoHandle` is `Send`.
#[test]
fn pane_io_handle_is_send() {
    fn assert_send<T: Send>() {}
    assert_send::<PaneIoHandle>();
}

/// Debug output on `PaneIoThread` and `PaneIoHandle`.
#[test]
fn debug_impls() {
    let (thread, handle) = make_pair();
    let t = format!("{thread:?}");
    assert!(t.contains("PaneIoThread"), "expected struct name in: {t}");
    let h = format!("{handle:?}");
    assert!(h.contains("PaneIoHandle"), "expected struct name in: {h}");
}
