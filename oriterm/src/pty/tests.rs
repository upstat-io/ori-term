use std::io::Write;
use std::sync::mpsc;
use std::time::{Duration, Instant};

use super::spawn::{PtyHandle, build_command, default_shell};
use super::{PtyConfig, PtyEvent, PtyReader, spawn_pty};

// ---------------------------------------------------------------------------
// Shell detection
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// Command building
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// PTY spawning
// ---------------------------------------------------------------------------

#[test]
fn spawn_pty_default_succeeds() {
    let config = PtyConfig::default();
    let mut handle = spawn_pty(&config).expect("spawn_pty with defaults failed");

    // Reader and writer should be available.
    assert!(handle.take_reader().is_some(), "reader missing");
    assert!(handle.take_writer().is_some(), "writer missing");

    // Second take returns None.
    assert!(handle.take_reader().is_none());
    assert!(handle.take_writer().is_none());

    cleanup(&mut handle);
}

#[test]
fn spawn_pty_custom_shell_succeeds() {
    let config = PtyConfig {
        shell: Some("/bin/sh".into()),
        ..Default::default()
    };
    let mut handle = spawn_pty(&config).expect("spawn_pty with /bin/sh failed");
    cleanup(&mut handle);
}

#[test]
fn pty_resize_succeeds() {
    let config = PtyConfig::default();
    let mut handle = spawn_pty(&config).expect("spawn_pty failed");
    handle.resize(40, 120).expect("resize failed");
    cleanup(&mut handle);
}

#[test]
fn pty_process_id_is_some() {
    let config = PtyConfig::default();
    let mut handle = spawn_pty(&config).expect("spawn_pty failed");
    assert!(handle.process_id().is_some(), "process_id should be Some");
    cleanup(&mut handle);
}

// ---------------------------------------------------------------------------
// Reader thread
// ---------------------------------------------------------------------------

#[test]
fn reader_receives_data() {
    let config = PtyConfig::default();
    let mut handle = spawn_pty(&config).expect("spawn_pty failed");

    let reader = handle.take_reader().expect("no reader");
    let (tx, rx) = mpsc::channel();
    let _reader_thread = PtyReader::spawn(reader, tx);

    // The shell should emit at least a prompt or initial output.
    let saw_data = wait_for_event(&rx, Duration::from_secs(5), |e| {
        matches!(e, PtyEvent::Data(d) if !d.is_empty())
    });
    assert!(saw_data, "reader thread should receive data from PTY");

    cleanup(&mut handle);
}

#[test]
fn reader_detects_close_after_exit() {
    let config = PtyConfig {
        shell: Some("/bin/sh".into()),
        ..Default::default()
    };
    let mut handle = spawn_pty(&config).expect("spawn_pty failed");

    let reader = handle.take_reader().expect("no reader");
    let mut writer = handle.take_writer().expect("no writer");
    let (tx, rx) = mpsc::channel();
    let _reader_thread = PtyReader::spawn(reader, tx);

    // Tell the shell to exit.
    let _ = writer.write_all(b"exit\n");

    let saw_close = wait_for_event(&rx, Duration::from_secs(5), |e| {
        matches!(e, PtyEvent::Closed)
    });
    assert!(saw_close, "reader thread should detect PTY close");

    let _ = handle.wait();
}

// ---------------------------------------------------------------------------
// SIGCHLD (Unix only)
// ---------------------------------------------------------------------------

#[cfg(unix)]
#[test]
fn sigchld_handler_registers() {
    super::signal::init().expect("SIGCHLD init failed");
    // Second call should be a no-op, not an error.
    super::signal::init().expect("SIGCHLD re-init failed");
}

#[cfg(unix)]
#[test]
fn sigchld_fires_on_child_exit() {
    super::signal::init().expect("SIGCHLD init failed");
    // Clear any prior signal.
    let _ = super::signal::check();

    let config = PtyConfig {
        shell: Some("/bin/sh".into()),
        ..Default::default()
    };
    let mut handle = spawn_pty(&config).expect("spawn_pty failed");

    let _ = handle.kill();
    let _ = handle.wait();

    // Give the signal a moment to arrive.
    std::thread::sleep(Duration::from_millis(100));

    assert!(
        super::signal::check(),
        "SIGCHLD should fire when child exits",
    );
}

// ---------------------------------------------------------------------------
// Error handling
// ---------------------------------------------------------------------------

#[test]
fn spawn_with_invalid_shell_returns_error() {
    let config = PtyConfig {
        shell: Some("/nonexistent/shell/path".into()),
        ..Default::default()
    };
    assert!(spawn_pty(&config).is_err(), "invalid shell should fail");
}

// ---------------------------------------------------------------------------
// Exit code propagation
// ---------------------------------------------------------------------------

#[test]
fn wait_returns_success_exit_code() {
    let config = PtyConfig {
        shell: Some("/bin/sh".into()),
        ..Default::default()
    };
    let mut handle = spawn_pty(&config).expect("spawn_pty failed");

    let reader = handle.take_reader().expect("no reader");
    let mut writer = handle.take_writer().expect("no writer");
    let (tx, _rx) = mpsc::channel();
    let _drain = PtyReader::spawn(reader, tx);

    let _ = writer.write_all(b"exit 0\n");

    let status = handle.wait().expect("wait failed");
    assert!(status.success(), "exit 0 should report success");
    assert_eq!(status.exit_code(), 0);
}

#[test]
fn wait_returns_nonzero_exit_code() {
    let config = PtyConfig {
        shell: Some("/bin/sh".into()),
        ..Default::default()
    };
    let mut handle = spawn_pty(&config).expect("spawn_pty failed");

    let reader = handle.take_reader().expect("no reader");
    let mut writer = handle.take_writer().expect("no writer");
    let (tx, _rx) = mpsc::channel();
    let _drain = PtyReader::spawn(reader, tx);

    let _ = writer.write_all(b"exit 42\n");

    let status = handle.wait().expect("wait failed");
    assert!(!status.success(), "exit 42 should not report success");
    assert_eq!(status.exit_code(), 42);
}

// ---------------------------------------------------------------------------
// Working directory
// ---------------------------------------------------------------------------

#[test]
fn spawn_with_working_directory() {
    let config = PtyConfig {
        shell: Some("/bin/sh".into()),
        working_dir: Some("/tmp".into()),
        ..Default::default()
    };
    let mut handle = spawn_pty(&config).expect("spawn_pty failed");

    let reader = handle.take_reader().expect("no reader");
    let mut writer = handle.take_writer().expect("no writer");
    let (tx, rx) = mpsc::channel();
    let _reader_thread = PtyReader::spawn(reader, tx);

    let _ = writer.write_all(b"pwd\nexit\n");

    let output = collect_output(&rx, Duration::from_secs(5));
    assert!(
        output.contains("/tmp"),
        "working dir should be /tmp, got: {output}",
    );
    let _ = handle.wait();
}

// ---------------------------------------------------------------------------
// Large data throughput
// ---------------------------------------------------------------------------

#[test]
fn large_data_throughput_no_deadlock() {
    let config = PtyConfig {
        shell: Some("/bin/sh".into()),
        ..Default::default()
    };
    let mut handle = spawn_pty(&config).expect("spawn_pty failed");

    let reader = handle.take_reader().expect("no reader");
    let mut writer = handle.take_writer().expect("no writer");
    let (tx, rx) = mpsc::channel();
    let _reader_thread = PtyReader::spawn(reader, tx);

    // seq 1 10000 produces ~49KB of output.
    let _ = writer.write_all(b"seq 1 10000\nexit\n");

    let mut total = 0usize;
    let start = Instant::now();
    while start.elapsed() < Duration::from_secs(10) {
        match rx.recv_timeout(Duration::from_millis(200)) {
            Ok(PtyEvent::Data(d)) => total += d.len(),
            Ok(PtyEvent::Closed) => break,
            Err(mpsc::RecvTimeoutError::Timeout) => continue,
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
    }

    // 10000 lines of numbers should be well over 10KB.
    assert!(
        total > 10_000,
        "expected >10KB of output, got {total} bytes",
    );
    let _ = handle.wait();
}

// ---------------------------------------------------------------------------
// Output drain after child exit
// ---------------------------------------------------------------------------

#[test]
fn output_received_before_close_event() {
    let config = PtyConfig {
        shell: Some("/bin/sh".into()),
        ..Default::default()
    };
    let mut handle = spawn_pty(&config).expect("spawn_pty failed");

    let reader = handle.take_reader().expect("no reader");
    let mut writer = handle.take_writer().expect("no writer");
    let (tx, rx) = mpsc::channel();
    let _reader_thread = PtyReader::spawn(reader, tx);

    // Emit a marker then immediately exit.
    let _ = writer.write_all(b"echo drain-marker-7719\nexit\n");

    let output = collect_output(&rx, Duration::from_secs(5));
    assert!(
        output.contains("drain-marker-7719"),
        "output before exit should still be received: {output}",
    );
    let _ = handle.wait();
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Kill and reap the child process.
fn cleanup(handle: &mut PtyHandle) {
    let _ = handle.kill();
    let _ = handle.wait();
}

/// Poll the channel until `predicate` matches or `timeout` expires.
fn wait_for_event(
    rx: &mpsc::Receiver<PtyEvent>,
    timeout: Duration,
    predicate: impl Fn(&PtyEvent) -> bool,
) -> bool {
    let start = Instant::now();
    while start.elapsed() < timeout {
        match rx.recv_timeout(Duration::from_millis(100)) {
            Ok(ref event) if predicate(event) => return true,
            Ok(_) => {}
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => return false,
        }
    }
    false
}

/// Collect all PTY output into a string until `Closed` or `timeout`.
fn collect_output(rx: &mpsc::Receiver<PtyEvent>, timeout: Duration) -> String {
    let mut buf = Vec::new();
    let start = Instant::now();
    while start.elapsed() < timeout {
        match rx.recv_timeout(Duration::from_millis(100)) {
            Ok(PtyEvent::Data(d)) => buf.extend_from_slice(&d),
            Ok(PtyEvent::Closed) => break,
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
    }
    String::from_utf8_lossy(&buf).into_owned()
}
