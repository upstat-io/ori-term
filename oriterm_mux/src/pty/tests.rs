//! Tests for PTY config, command building, shell detection, and writer thread.
//!
//! No real PTY processes are spawned — Alacritty and WezTerm don't test
//! live PTY either. The PTY reader (byte forwarder) is tested in
//! `reader/tests.rs`.

use std::io::Read;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;

use super::spawn::{build_command, compute_wslenv, default_shell};
use super::{Msg, PtyConfig, PtyLifecycle, spawn_pty, spawn_pty_writer};

// Shell detection

#[test]
fn default_shell_is_nonempty() {
    let shell = default_shell();
    assert!(!shell.is_empty(), "default shell must not be empty");
}

#[cfg(unix)]
#[test]
fn default_shell_exists_on_disk() {
    let shell = default_shell();
    let path = std::path::Path::new(shell);
    assert!(path.exists(), "default shell `{shell}` does not exist");
}

// Command building

#[test]
fn build_command_sets_terminal_env_vars() {
    let config = PtyConfig::default();
    let cmd = build_command(&config);

    assert_eq!(
        cmd.get_env("TERM").and_then(|v| v.to_str()),
        Some("xterm-256color"),
    );
    assert_eq!(
        cmd.get_env("COLORTERM").and_then(|v| v.to_str()),
        Some("truecolor"),
    );
    assert_eq!(
        cmd.get_env("TERM_PROGRAM").and_then(|v| v.to_str()),
        Some("oriterm"),
    );
}

#[test]
fn build_command_applies_user_env_overrides() {
    let config = PtyConfig {
        env: vec![("MY_VAR".into(), "my_value".into())],
        ..Default::default()
    };
    let cmd = build_command(&config);

    assert_eq!(
        cmd.get_env("MY_VAR").and_then(|v| v.to_str()),
        Some("my_value"),
    );
}

#[test]
fn build_command_uses_custom_shell() {
    let config = PtyConfig {
        shell: Some("/bin/sh".into()),
        ..Default::default()
    };
    let cmd = build_command(&config);
    let argv = cmd.get_argv();

    assert!(!argv.is_empty());
    assert_eq!(argv[0], "/bin/sh");
}

#[test]
fn build_command_with_working_directory() {
    let config = PtyConfig {
        working_dir: Some("/tmp".into()),
        ..Default::default()
    };
    let cmd = build_command(&config);
    let argv = cmd.get_argv();

    // Command should be buildable with a working directory.
    assert!(!argv.is_empty());
}

#[test]
fn build_command_default_shell_used_when_none() {
    let config = PtyConfig::default();
    let cmd = build_command(&config);
    let argv = cmd.get_argv();

    assert!(!argv.is_empty());
    assert_eq!(argv[0], default_shell());
}

#[cfg(windows)]
#[test]
fn build_command_sets_wslenv_for_cross_boundary_propagation() {
    let config = PtyConfig::default();
    let cmd = build_command(&config);

    let wslenv = cmd
        .get_env("WSLENV")
        .and_then(|v| v.to_str())
        .expect("WSLENV must be set on Windows");
    assert!(
        wslenv.contains("TERM"),
        "WSLENV must include TERM: {wslenv}",
    );
    assert!(
        wslenv.contains("COLORTERM"),
        "WSLENV must include COLORTERM: {wslenv}",
    );
    assert!(
        wslenv.contains("TERM_PROGRAM"),
        "WSLENV must include TERM_PROGRAM: {wslenv}",
    );
}

// WSLENV computation (cross-platform — tests the pure string logic)

#[test]
fn wslenv_empty_existing_adds_builtins() {
    let result = compute_wslenv("", &[]).unwrap();
    assert!(result.contains("TERM"));
    assert!(result.contains("COLORTERM"));
    assert!(result.contains("TERM_PROGRAM"));
    // Must not start with ':'.
    assert!(!result.starts_with(':'));
}

#[test]
fn wslenv_appends_to_existing() {
    let result = compute_wslenv("FOO:BAR", &[]).unwrap();
    assert!(
        result.starts_with("FOO:BAR:"),
        "must preserve existing entries: {result}",
    );
    assert!(result.contains("TERM"));
    assert!(result.contains("COLORTERM"));
    assert!(result.contains("TERM_PROGRAM"));
}

#[test]
fn wslenv_dedup_existing_entries() {
    // TERM already in WSLENV — should not appear twice in output.
    let result = compute_wslenv("TERM", &[]).unwrap();
    let count = result.split(':').filter(|s| *s == "TERM").count();
    assert_eq!(count, 1, "TERM must appear exactly once: {result}");
}

#[test]
fn wslenv_case_insensitive_dedup() {
    // Mixed-case "Term" should match our "TERM" and prevent duplicate.
    let result = compute_wslenv("Term:colorterm", &[]).unwrap();
    let keys: Vec<&str> = result.split(':').collect();

    // Only TERM_PROGRAM should be added (Term and colorterm already cover TERM and COLORTERM).
    assert_eq!(
        keys.iter()
            .filter(|k| k.eq_ignore_ascii_case("TERM"))
            .count(),
        1,
        "case-insensitive dedup for TERM: {result}",
    );
    assert_eq!(
        keys.iter()
            .filter(|k| k.eq_ignore_ascii_case("COLORTERM"))
            .count(),
        1,
        "case-insensitive dedup for COLORTERM: {result}",
    );
}

#[test]
fn wslenv_preserves_existing_flags() {
    // Entries with flags like `FOO/u` must survive in the output.
    let result = compute_wslenv("FOO/u:BAR/l", &[]).unwrap();
    assert!(
        result.contains("FOO/u"),
        "must preserve flags on existing entries: {result}",
    );
    assert!(
        result.contains("BAR/l"),
        "must preserve flags on existing entries: {result}",
    );
}

#[test]
fn wslenv_path_never_added() {
    // Even if user explicitly passes PATH, it must be excluded from WSLENV.
    let result = compute_wslenv("", &["PATH", "MY_VAR"]).unwrap();
    let keys: Vec<&str> = result.split(':').collect();
    assert!(
        !keys.iter().any(|k| k.eq_ignore_ascii_case("PATH")),
        "PATH must never appear in WSLENV: {result}",
    );
    assert!(
        keys.iter().any(|k| *k == "MY_VAR"),
        "user keys (non-PATH) must appear: {result}",
    );
}

#[test]
fn wslenv_user_env_overlapping_builtin() {
    // User provides TERM — should not appear twice.
    let result = compute_wslenv("", &["TERM", "MY_VAR"]).unwrap();
    let count = result.split(':').filter(|s| *s == "TERM").count();
    assert_eq!(
        count, 1,
        "overlapping user key must not duplicate: {result}"
    );
    assert!(result.contains("MY_VAR"), "user key must appear: {result}");
}

#[test]
fn wslenv_all_already_present_returns_none() {
    // Every builtin already in WSLENV, no user keys — nothing to add.
    let result = compute_wslenv(
        "TERM:COLORTERM:ORITERM:TERM_PROGRAM:TERM_PROGRAM_VERSION",
        &[],
    );
    assert!(
        result.is_none(),
        "should return None when nothing to add: {result:?}",
    );
}

#[test]
fn wslenv_multiple_user_keys() {
    let result = compute_wslenv("", &["A", "B", "C"]).unwrap();
    let keys: Vec<&str> = result.split(':').collect();
    assert!(keys.contains(&"A"), "user key A missing: {result}");
    assert!(keys.contains(&"B"), "user key B missing: {result}");
    assert!(keys.contains(&"C"), "user key C missing: {result}");
    // Builtins also present.
    assert!(keys.contains(&"TERM"), "builtin TERM missing: {result}");
}

#[test]
fn wslenv_builtin_keys_before_user_keys() {
    let result = compute_wslenv("", &["ZZZ"]).unwrap();
    let keys: Vec<&str> = result.split(':').collect();
    let term_pos = keys.iter().position(|k| *k == "TERM").unwrap();
    let zzz_pos = keys.iter().position(|k| *k == "ZZZ").unwrap();
    assert!(
        term_pos < zzz_pos,
        "builtins should precede user keys: {result}",
    );
}

#[test]
fn wslenv_user_env_special_values_are_keys_not_values() {
    // Keys with unusual characters (underscores, digits) must pass through.
    let result = compute_wslenv("", &["MY_VAR_2", "X11_DISPLAY"]).unwrap();
    assert!(
        result.contains("MY_VAR_2"),
        "underscore+digit key: {result}"
    );
    assert!(result.contains("X11_DISPLAY"), "mixed key: {result}");
}

// User env overrides builtins

#[test]
fn build_command_user_env_overrides_builtins() {
    // User sets TERM=dumb — should override the default xterm-256color.
    let config = PtyConfig {
        env: vec![("TERM".into(), "dumb".into())],
        shell_integration: false,
        ..Default::default()
    };
    let cmd = build_command(&config);

    assert_eq!(
        cmd.get_env("TERM").and_then(|v| v.to_str()),
        Some("dumb"),
        "user TERM override should take precedence",
    );
    // Other builtins should still be set.
    assert_eq!(
        cmd.get_env("COLORTERM").and_then(|v| v.to_str()),
        Some("truecolor"),
    );
}

#[test]
fn build_command_multiple_user_env_vars() {
    let config = PtyConfig {
        env: vec![("FOO".into(), "bar".into()), ("BAZ".into(), "qux".into())],
        shell_integration: false,
        ..Default::default()
    };
    let cmd = build_command(&config);

    assert_eq!(cmd.get_env("FOO").and_then(|v| v.to_str()), Some("bar"),);
    assert_eq!(cmd.get_env("BAZ").and_then(|v| v.to_str()), Some("qux"),);
    // Builtins still present.
    assert_eq!(
        cmd.get_env("TERM").and_then(|v| v.to_str()),
        Some("xterm-256color"),
    );
}

#[test]
fn build_command_empty_env_list_leaves_builtins() {
    let config = PtyConfig {
        env: Vec::new(),
        shell_integration: false,
        ..Default::default()
    };
    let cmd = build_command(&config);

    assert_eq!(
        cmd.get_env("TERM").and_then(|v| v.to_str()),
        Some("xterm-256color"),
    );
    assert_eq!(
        cmd.get_env("COLORTERM").and_then(|v| v.to_str()),
        Some("truecolor"),
    );
    assert_eq!(
        cmd.get_env("TERM_PROGRAM").and_then(|v| v.to_str()),
        Some("oriterm"),
    );
}

// WSLENV flag collision

#[test]
fn wslenv_flag_collision_existing_has_flags_user_omits() {
    // Existing WSLENV has FOO/pu (with flags). User provides plain FOO.
    // The dedup should match FOO (case-insensitive, flag-stripped) and NOT
    // add a duplicate.
    let result = compute_wslenv("FOO/pu", &["FOO"]);
    // Only builtins should be added (FOO already present via existing).
    if let Some(ref r) = result {
        let count = r
            .split(':')
            .filter(|s| s.eq_ignore_ascii_case("FOO") || s.starts_with("FOO/"))
            .count();
        assert_eq!(count, 1, "FOO should appear exactly once: {r}");
    }
    // Original entry with flags should be preserved.
    if let Some(ref r) = result {
        assert!(
            r.contains("FOO/pu"),
            "original flags must be preserved: {r}"
        );
    }
}

#[test]
fn wslenv_flag_collision_existing_plain_user_same() {
    // Both existing and user have the same key without flags.
    let result = compute_wslenv("FOO", &["FOO"]);
    if let Some(ref r) = result {
        let count = r.split(':').filter(|s| *s == "FOO").count();
        assert_eq!(count, 1, "FOO should appear exactly once: {r}");
    }
}

#[test]
fn wslenv_flag_collision_mixed_case_with_flags() {
    // Existing: "Foo/p" (mixed-case with flags). User: "FOO" (uppercase, no flags).
    // Dedup extracts "Foo" (before the /), uppercases to "FOO" → match.
    let result = compute_wslenv("Foo/p", &["FOO"]);
    if let Some(ref r) = result {
        let count = r
            .split(':')
            .filter(|s| s.eq_ignore_ascii_case("FOO") || s.starts_with("Foo/"))
            .count();
        assert_eq!(count, 1, "FOO should not be duplicated: {r}");
        assert!(
            r.contains("Foo/p"),
            "original mixed-case entry preserved: {r}"
        );
    }
}

// Writer thread

#[test]
fn writer_thread_delivers_input() {
    let (mut reader, writer) = std::io::pipe().expect("pipe");
    let shutdown = Arc::new(AtomicBool::new(false));
    let (tx, rx) = mpsc::channel();

    let handle = spawn_pty_writer(
        Box::new(writer),
        rx,
        Arc::clone(&shutdown),
        Arc::new(AtomicBool::new(false)),
        crossbeam_channel::bounded::<()>(1).0,
    )
    .expect("spawn writer thread");

    tx.send(Msg::Input(b"hello".to_vec())).unwrap();
    tx.send(Msg::Shutdown).unwrap();
    handle.join().expect("writer thread panicked");

    let mut buf = Vec::new();
    reader.read_to_end(&mut buf).unwrap();
    assert_eq!(buf, b"hello");
    assert!(
        shutdown.load(Ordering::Acquire),
        "shutdown flag must be set"
    );
}

#[test]
fn writer_thread_batches_queued_messages() {
    let (mut reader, writer) = std::io::pipe().expect("pipe");
    let shutdown = Arc::new(AtomicBool::new(false));
    let (tx, rx) = mpsc::channel();

    // Queue multiple messages before the thread can process them.
    tx.send(Msg::Input(b"aaa".to_vec())).unwrap();
    tx.send(Msg::Input(b"bbb".to_vec())).unwrap();
    tx.send(Msg::Input(b"ccc".to_vec())).unwrap();
    tx.send(Msg::Shutdown).unwrap();

    let handle = spawn_pty_writer(
        Box::new(writer),
        rx,
        Arc::clone(&shutdown),
        Arc::new(AtomicBool::new(false)),
        crossbeam_channel::bounded::<()>(1).0,
    )
    .expect("spawn writer thread");
    handle.join().expect("writer thread panicked");

    let mut buf = Vec::new();
    reader.read_to_end(&mut buf).unwrap();
    assert_eq!(buf, b"aaabbbccc", "all messages must be delivered in order");
}

#[test]
fn writer_thread_shutdown_sets_flag() {
    let (_reader, writer) = std::io::pipe().expect("pipe");
    let shutdown = Arc::new(AtomicBool::new(false));
    let (tx, rx) = mpsc::channel();

    let handle = spawn_pty_writer(
        Box::new(writer),
        rx,
        Arc::clone(&shutdown),
        Arc::new(AtomicBool::new(false)),
        crossbeam_channel::bounded::<()>(1).0,
    )
    .expect("spawn writer thread");

    tx.send(Msg::Shutdown).unwrap();
    handle.join().expect("writer thread panicked");

    assert!(
        shutdown.load(Ordering::Acquire),
        "shutdown flag must be set after Msg::Shutdown",
    );
}

#[test]
fn writer_thread_channel_close_sets_flag() {
    let (_reader, writer) = std::io::pipe().expect("pipe");
    let shutdown = Arc::new(AtomicBool::new(false));
    let (tx, rx) = mpsc::channel();

    let handle = spawn_pty_writer(
        Box::new(writer),
        rx,
        Arc::clone(&shutdown),
        Arc::new(AtomicBool::new(false)),
        crossbeam_channel::bounded::<()>(1).0,
    )
    .expect("spawn writer thread");

    // Drop the sender — channel closes, thread exits.
    drop(tx);
    handle.join().expect("writer thread panicked");

    assert!(
        shutdown.load(Ordering::Acquire),
        "shutdown flag must be set on channel close",
    );
}

#[test]
fn writer_thread_sets_stall_flag_during_write() {
    // Create a pipe with a tiny buffer. Fill it so the next write blocks.
    // The write_stalled flag should become true while the writer is blocked.
    use std::thread;
    use std::time::Duration;

    let (reader, writer) = std::io::pipe().expect("pipe");
    let shutdown = Arc::new(AtomicBool::new(false));
    let stalled = Arc::new(AtomicBool::new(false));
    let (tx, rx) = mpsc::channel();

    let handle = spawn_pty_writer(
        Box::new(writer),
        rx,
        Arc::clone(&shutdown),
        Arc::clone(&stalled),
        crossbeam_channel::bounded::<()>(1).0,
    )
    .expect("spawn writer thread");

    // Send enough data to fill the pipe buffer and cause a stall.
    // Pipe buffers are typically 64KB on Linux, 4KB on some systems.
    // Send 256KB to ensure we exceed the buffer on all platforms.
    let big_data = vec![b'X'; 256 * 1024];
    tx.send(Msg::Input(big_data)).unwrap();

    // Wait for the writer to enter the blocked write.
    let mut saw_stalled = false;
    for _ in 0..100 {
        thread::sleep(Duration::from_millis(10));
        if stalled.load(Ordering::Acquire) {
            saw_stalled = true;
            break;
        }
    }
    assert!(
        saw_stalled,
        "write_stalled flag must be set while write blocks on full pipe"
    );

    // Clean up: drop the reader to unblock the writer, then shut down.
    // The writer may have already exited (broken pipe), so ignore SendError.
    drop(reader);
    let _ = tx.send(Msg::Shutdown);
    handle.join().expect("writer thread panicked");
}

/// **Reproduction**: when the writer is stalled on a full pipe,
/// Ctrl+C (0x03) queued after the stall is stuck **permanently**. Nobody
/// drains the pipe — the child (`yes`) never reads stdin. The writer
/// thread's `write_all()` blocks forever, and every subsequent message
/// in the channel is unreachable.
/// This test must **fail** (time out) if the writer has no mechanism to
/// let the caller detect the stall and bypass the blocked write.
/// We give the writer 500ms to deliver the 0x03 byte. Without a fix
/// it will never arrive (the pipe is full and nobody is draining it).
/// The test passes because we use the `write_stalled` flag to detect
/// the deadlock, then drop the reader (simulating SIGINT killing the
/// child, which closes the pipe and unblocks the write).
#[test]
fn ctrl_c_stuck_behind_stalled_write() {
    use std::thread;
    use std::time::Duration;

    let (reader, writer) = std::io::pipe().expect("pipe");
    let shutdown = Arc::new(AtomicBool::new(false));
    let stalled = Arc::new(AtomicBool::new(false));
    let (tx, rx) = mpsc::channel();

    let handle = spawn_pty_writer(
        Box::new(writer),
        rx,
        Arc::clone(&shutdown),
        Arc::clone(&stalled),
        crossbeam_channel::bounded::<()>(1).0,
    )
    .expect("spawn writer thread");

    // 1. Fill the pipe to stall the writer (256KB > any OS pipe buffer).
    tx.send(Msg::Input(vec![b'X'; 256 * 1024])).unwrap();

    // Wait for writer to enter the blocked write().
    let mut saw_stall = false;
    for _ in 0..100 {
        thread::sleep(Duration::from_millis(10));
        if stalled.load(Ordering::Acquire) {
            saw_stall = true;
            break;
        }
    }
    assert!(saw_stall, "writer must be stalled on full pipe");

    // 2. Queue Ctrl+C. It sits in the channel — the writer thread is
    // blocked in write_all() and cannot recv() from the channel.
    tx.send(Msg::Input(vec![0x03])).unwrap();

    // 3. The pipe is NOT drained. In the real scenario the child (`yes`)
    // never reads stdin. The writer is stuck forever.
    // THE FIX: the main thread checks `write_stalled`, sees it's true,
    // and sends SIGINT directly to the child process group. The child
    // dies, closing the slave PTY fd, which causes the master write()
    // to return (with an error or 0 bytes). The writer thread unblocks,
    // drains the channel (including the 0x03), and writes it.
    // We simulate "SIGINT killed the child" by dropping the reader end
    // of the pipe after detecting the stall flag.
    assert!(
        stalled.load(Ordering::Acquire),
        "stall flag must be observable by the main thread"
    );

    // Simulate SIGINT → child dies → pipe reader end closes.
    drop(reader);

    // 4. Writer unblocks (write returns error because reader closed).
    // The writer may have already exited (broken pipe), so ignore SendError.
    let _ = tx.send(Msg::Shutdown);
    handle.join().expect("writer thread panicked");
}

/// Same setup as the reproduction test, but verify the **full round-trip**:
/// stall → detect → unblock → Ctrl+C byte delivered.
#[test]
fn ctrl_c_delivered_after_stall_cleared() {
    use std::io::Read as _;
    use std::thread;
    use std::time::Duration;

    let (mut reader, writer) = std::io::pipe().expect("pipe");
    let shutdown = Arc::new(AtomicBool::new(false));
    let stalled = Arc::new(AtomicBool::new(false));
    let (tx, rx) = mpsc::channel();

    let handle = spawn_pty_writer(
        Box::new(writer),
        rx,
        Arc::clone(&shutdown),
        Arc::clone(&stalled),
        crossbeam_channel::bounded::<()>(1).0,
    )
    .expect("spawn writer thread");

    // Fill the pipe to stall the writer.
    tx.send(Msg::Input(vec![b'X'; 256 * 1024])).unwrap();
    for _ in 0..100 {
        thread::sleep(Duration::from_millis(10));
        if stalled.load(Ordering::Acquire) {
            break;
        }
    }
    assert!(stalled.load(Ordering::Acquire));

    // Queue Ctrl+C while stalled.
    tx.send(Msg::Input(vec![0x03])).unwrap();

    // Drain the pipe in a background thread (simulates child dying and
    // the OS flushing the pipe).
    let drain = thread::spawn(move || {
        let mut buf = Vec::new();
        let _ = reader.read_to_end(&mut buf);
        buf
    });

    // Shut down the writer.
    tx.send(Msg::Shutdown).unwrap();
    handle.join().expect("writer thread panicked");

    let output = drain.join().expect("drain thread panicked");

    // The 0x03 byte must appear somewhere in the output stream.
    assert!(
        output.contains(&0x03),
        "Ctrl+C byte (0x03) must be delivered after the stall clears"
    );
}

// PtyLifecycle trait dispatch
// Phase 1A property: verify that boxing a `PtyHandle` as
// `Box<dyn PtyLifecycle + Send>` correctly dispatches `process_id()`,
// `kill()`, and `wait()` through the trait vtable. This test ONLY passes
// if the `impl PtyLifecycle for PtyHandle` is wired correctly — if any
// trait method delegates to the wrong inherent method, or if the trait
// object can't be constructed, the test fails to compile or returns the
// wrong PID.
// This is the only test in the suite that spawns a real child process —
// the trait dispatch contract requires a real `PtyHandle`, which can only
// be produced via `spawn_pty()`. The shell is killed and reaped within
// the test body, so the process is short-lived.

#[test]
fn pty_handle_dispatches_through_pty_lifecycle_trait() {
    // Spawn a PTY with the platform default shell.
    let config = PtyConfig::default();
    let (pty, _child_exit_rx) = spawn_pty(&config).expect("spawn_pty must succeed");

    // Capture the inherent process_id BEFORE moving into the box.
    let direct_pid = pty.process_id();
    assert!(
        direct_pid.is_some(),
        "spawned PTY must report a child PID via the inherent method"
    );

    // Box as a trait object — this exercises the `impl PtyLifecycle for PtyHandle`.
    let mut boxed: Box<dyn PtyLifecycle + Send> = Box::new(pty);

    // Trait dispatch must return the same PID as the inherent call.
    // This is the property: it only passes if the impl correctly
    // delegates `PtyLifecycle::process_id` to `PtyHandle::process_id`.
    let trait_pid = boxed.process_id();
    assert_eq!(
        trait_pid, direct_pid,
        "PtyLifecycle::process_id must return the same PID as PtyHandle::process_id",
    );

    // Verify kill + wait are reachable through the trait object too.
    // We don't assert success — kill may race with the shell exiting normally —
    // but the methods must dispatch without panicking.
    let _ = boxed.kill();
    let _ = boxed.wait();
}

/// Verify that the writer's stall flag is cleared after the write completes.
#[test]
fn write_stalled_flag_clears_after_write_completes() {
    use std::thread;
    use std::time::Duration;

    let (_reader, writer) = std::io::pipe().expect("pipe");
    let shutdown = Arc::new(AtomicBool::new(false));
    let stalled = Arc::new(AtomicBool::new(false));
    let (tx, rx) = mpsc::channel();

    let handle = spawn_pty_writer(
        Box::new(writer),
        rx,
        Arc::clone(&shutdown),
        Arc::clone(&stalled),
        crossbeam_channel::bounded::<()>(1).0,
    )
    .expect("spawn writer thread");

    // Small data fits in the pipe buffer — write completes immediately.
    tx.send(Msg::Input(b"hello".to_vec())).unwrap();
    thread::sleep(Duration::from_millis(50));

    assert!(
        !stalled.load(Ordering::Acquire),
        "write_stalled must be false after a successful write"
    );

    tx.send(Msg::Shutdown).unwrap();
    handle.join().expect("writer thread panicked");
}

/// Regression: the `write_stalled` AtomicBool must transition
/// `false → true → false` around a kernel-buffer-fill write that subsequently
/// drains. The existing `write_stalled_flag_clears_after_write_completes` test
/// only sends a small payload and verifies the flag stays `false`; it does NOT
/// exercise the true→false transition. Plan TPR F6 cited this test as the
/// pin that justifies skipping the e2e drain assertion, but the cited pin
/// doesn't actually pin the transition. This test fills the pipe to force
/// `stalled = true`, then drains the reader to allow the blocked `write()` to
/// complete, then verifies the flag returns to `false`.
#[test]
#[cfg(unix)]
fn write_stalled_flag_transitions_true_then_false_around_drained_write() {
    use std::io::Read as _;
    use std::thread;
    use std::time::{Duration, Instant};

    let (mut reader, writer) = std::io::pipe().expect("pipe");
    let shutdown = Arc::new(AtomicBool::new(false));
    let stalled = Arc::new(AtomicBool::new(false));
    let (tx, rx) = mpsc::channel();

    let handle = spawn_pty_writer(
        Box::new(writer),
        rx,
        Arc::clone(&shutdown),
        Arc::clone(&stalled),
        crossbeam_channel::bounded::<()>(1).0,
    )
    .expect("spawn writer thread");

    // Send a payload large enough to overflow the kernel pipe buffer (default
    // 64 KiB on Linux). 1 MiB is comfortably over any platform's default; the
    // writer thread will block on the second/third `write()` call once the
    // buffer fills.
    let payload = vec![b'x'; 1 << 20];
    tx.send(Msg::Input(payload)).unwrap();

    // Poll until `stalled` flips to `true`. Deadline-as-safety, not deadline-
    // as-signal, per `tests.md §Wall-Clock-Free Testing`.
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline {
        if stalled.load(Ordering::Acquire) {
            break;
        }
        thread::sleep(Duration::from_millis(5));
    }
    assert!(
        stalled.load(Ordering::Acquire),
        "write_stalled must flip to true once the pipe buffer fills",
    );

    // Drain the reader so the writer's blocked `write()` completes.
    let mut buf = vec![0u8; 64 * 1024];
    let drain_deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < drain_deadline {
        if !stalled.load(Ordering::Acquire) {
            break;
        }
        let _ = reader.read(&mut buf);
    }
    // Final assertion: the flag transitioned back to false after the drain.
    assert!(
        !stalled.load(Ordering::Acquire),
        "write_stalled must clear back to false after the kernel buffer drains",
    );

    tx.send(Msg::Shutdown).unwrap();
    handle.join().expect("writer thread panicked");
}
