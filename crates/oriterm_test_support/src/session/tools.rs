//! Runtime tool-availability probes for the test framework.
//!
//! Each helper shells out to the named binary with a `--version` (or
//! `--help`) argument and reports whether the invocation succeeded.
//! Used by integration tests to skip cleanly when a required tool
//! (`vttest`, `tack`, `tic`, `infocmp`, ...) is not installed.
//!
//! Extracted from `session/mod.rs` in the M1 TPR cleanup
//! (TPR-05-002) to keep `session/mod.rs` under the 500-line file
//! hygiene limit. The public API is unchanged — `session/mod.rs`
//! re-exports each helper via `pub use tools::*;` so external
//! callers still see `crate::session::tack_available()` etc.

/// Check if `name` is installed and runnable on PATH.
///
/// Used by integration tests to skip cleanly when a required tool
/// (`vttest`, `tack`, `tic`, `reseq`, ...) is not available. The
/// `--version` argument is the convention every well-behaved CLI
/// supports; some (`vttest`) prefer `--help` — pass that explicitly.
#[must_use]
pub fn tool_available(name: &str, version_arg: &str) -> bool {
    std::process::Command::new(name)
        .arg(version_arg)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok()
}

/// Convenience: vttest specifically uses `--help` (it has no `--version`).
#[must_use]
pub fn vttest_available() -> bool {
    tool_available("vttest", "--help")
}

/// Check if `tic` (terminfo compiler) is installed.
///
/// Probe is `tic -V`, which is the version flag every ncurses build
/// (BSD and GNU) supports. A too-old `tic` (ncurses < 6.0) may exist
/// and still fail to compile modern extension caps; in that case
/// `TerminfoEnv::compile()` panics with the tic stderr output, which
/// IS the failure contract — the user sees the message and upgrades
/// their ncurses package.
#[must_use]
pub fn tic_available() -> bool {
    tool_available("tic", "-V")
}

/// Check if `tack` (terminfo action checker, ncurses) is installed.
///
/// Tack ships with ncurses on Linux/macOS, not on native Windows.
/// Use this gate at the top of every test that spawns tack so the
/// suite skips cleanly on platforms missing the tool.
#[must_use]
pub fn tack_available() -> bool {
    tool_available("tack", "-V")
}

/// Check if `infocmp` (terminfo decompiler / inspector) is installed.
///
/// Probe is `infocmp -V`. Used by `terminfo` round-trip tests to
/// gate cleanly when `infocmp` is missing. `TerminfoEnv::compile()`
/// itself never depends on `infocmp` — that gate is enforced by
/// keeping the `compile()` constructor pure-`tic`.
#[must_use]
pub fn infocmp_available() -> bool {
    tool_available("infocmp", "-V")
}
