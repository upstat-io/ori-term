//! Runtime tool-availability probes for the test framework.
//!
//! Each per-tool helper (`vttest_available`, `tack_available`, etc.)
//! shells out to the binary with that tool's known-zero-exit probe
//! flag and reports whether the invocation both spawned successfully
//! and exited with success status. Most tools accept `--version` or
//! `-V`, but several ncurses-era binaries are "odd ones out": `tack`
//! exits 1 from `-V` so its probe is `-h`; `vttest` exits 1 from
//! `--help` so its probe is `-V`. See each per-tool wrapper below for
//! the canonical probe flag — never invent your own probe at a call
//! site, always use the wrapper. Integration tests use the wrappers
//! to skip cleanly when a required tool is not installed.
//!
//! Extracted from `session/mod.rs` in the M1 TPR cleanup
//! to keep `session/mod.rs` under the 500-line file
//! hygiene limit. The public API is unchanged — `session/mod.rs`
//! re-exports each helper via `pub use tools::*;` so external
//! callers still see `crate::session::tack_available()` etc.

/// Check if `name` is installed and runnable on PATH.
///
/// Used by integration tests to skip cleanly when a required tool
/// (`vttest`, `tack`, `tic`, `reseq`, ...) is not available. Callers
/// pass each tool's known-zero-exit probe flag — most use `--version`,
/// but several ncurses-era tools are "odd ones out": `tack` exits 1
/// from `-V` so its probe is `-h`; `vttest` exits 1 from `--help` so
/// its probe is `-V`. See the per-tool `*_available` wrappers below
/// for the canonical probe flag for each.
///
/// **Returns true iff the probe BOTH spawns successfully AND exits
/// with success status.** A binary that spawns but exits non-zero
/// (e.g., wrong flag, missing terminfo path, broken install) is
/// NOT treated as available — that flow would slip past the skip
/// gate and fail downstream as a panic instead of a clean skip.
/// A prior version only checked `Command::status().is_ok()` which
/// is `true` whenever the spawn syscall succeeded regardless of
/// exit code; the current implementation also requires
/// `status.success()`.
#[must_use]
pub fn tool_available(name: &str, version_arg: &str) -> bool {
    std::process::Command::new(name)
        .arg(version_arg)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}

/// Check if `vttest` is installed.
///
/// **Probe is `vttest -V`, NOT `vttest --help`.** vttest prints its
/// usage banner to stdout when invoked with `--help` but EXITS with
/// status 1 (not 0), so the `tool_available` `status.success()` check
/// would report vttest as unavailable on every host that has it
/// installed. `vttest -V` (capital, NOT `--version` — vttest does not
/// recognize the long form) prints the version banner and exits 0.
/// Same antipattern family as the prior tack `-h`/`-V` fix above.
/// Closes BUG-07-020.
#[must_use]
pub fn vttest_available() -> bool {
    tool_available("vttest", "-V")
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
///
/// **Probe is `tack -h`, NOT `tack -V`.** Tack v1.08 prints its
/// version banner to stdout when invoked with `-V`, but the binary
/// then EXITS with status 1 (not 0). Other ncurses tools like
/// `tic` and `infocmp` exit 0 from `-V` — tack is the odd one out.
/// Switching the probe to `-h` (which prints usage to stderr and
/// exits 0) fixes the false-negative that the
/// `tool_available` tighten introduced (`tool_available` now
/// requires `status.success()`, so `tack -V`'s exit-1 was
/// misreporting tack as unavailable on every dev/CI host).
#[must_use]
pub fn tack_available() -> bool {
    tool_available("tack", "-h")
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

#[cfg(test)]
mod tests;
