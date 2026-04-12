//! Tack version gate (Section 05.0.c).
//!
//! Section 05's tack catalog is pinned against tack v1.08 (the
//! version on the dev host). Every menu key, prompt string, and
//! screen layout in the catalog is verified against that exact
//! build. A future system upgrade to tack v6.x or v2.0 could
//! change the menu structure entirely. Without a version gate,
//! the discovery test would fail loudly — but the dozens of
//! downstream scenarios would also fail in ways that pollute CI
//! noise. The gate skips them cleanly with a concrete
//! "tack version not supported, skipping" message and a single
//! fix path: pin the new version, re-run discovery, update
//! inventory.
//!
//! # Module split rationale
//!
//! The version gate originally landed in `session/mod.rs` as part
//! of 05.0.c. Codex M1 TPR flagged this as a hygiene violation
//! because `session/mod.rs` exceeded the 500-line limit after the
//! addition, so it was extracted into a flat
//! `session/version_gate.rs` leaf. A subsequent iter
//! converted the leaf to a directory module: `version_gate/mod.rs`
//! plus its sibling `version_gate/tests.rs`, per the
//! `.claude/rules/test-organization.md` "one tests.rs per source
//! file" rule. The runtime tool-availability probes
//! (`tool_available`, `tack_available`, etc.) were extracted in
//! the same wave into `session/tools/mod.rs` (also a
//! directory module in a subsequent refactor).
//!
//! The public API surface is unchanged — `session/mod.rs`
//! re-exports every item via `pub use version_gate::*;` so
//! external callers still see `crate::session::tack_version_supported()`
//! etc.

/// Lowest tack major version Section 05's catalog has been pinned
/// against. Bump in lockstep with [`TACK_PINNED_MINOR`] when the
/// catalog is re-verified against a newer tack release.
pub const TACK_PINNED_MAJOR: u32 = 1;

/// Lowest tack minor version Section 05's catalog has been pinned
/// against. The minor version is checked EXACTLY — every minor
/// bump requires a re-discovery pass via 05.0.
pub const TACK_PINNED_MINOR: u32 = 8;

/// Pure parser for `tack -V` output. Returns `Some((major, minor))`
/// when the version banner can be located and parsed, `None`
/// otherwise.
///
/// Tack 1.08 prints to stdout. Future versions might split streams
/// (stderr-only), so the parser scans the concatenation. The parse
/// shape is `<...>version <maj>.<min><...>` where `<maj>` and
/// `<min>` are decimal integers (zero-padded `min` is fine —
/// `parse::<u32>` handles `"08"`).
///
/// Pure (no I/O) so unit tests can exercise the version-string
/// matrix without depending on a host-installed tack.
#[must_use]
pub fn parse_tack_version(stdout: &str, stderr: &str) -> Option<(u32, u32)> {
    let combined = format!("{stdout}{stderr}");
    let pos = combined.find("version ")?;
    // `.get(..)` rather than `&combined[..]` so clippy's
    // `string_slice` lint stays clean: `find` always returns a UTF-8
    // boundary so the slice is provably safe, but the lint can't see
    // that across functions, and a `.get()` + `?` is just as concise.
    let after = combined.get(pos + "version ".len()..)?;
    // Split on either `.` (major/minor separator) or whitespace
    // (terminates the minor number before any trailing build date).
    // The first two non-empty fragments are the major and minor.
    let mut parts = after.split(|c: char| c == '.' || c.is_ascii_whitespace());
    let maj_str = parts.next()?;
    let min_str = parts.next()?;
    let maj = maj_str.parse::<u32>().ok()?;
    let min = min_str.parse::<u32>().ok()?;
    Some((maj, min))
}

/// Build the loud-skip diagnostic text for an unsupported tack
/// version. Pure helper so tests can pin the keyword set without
/// observing the side-effecting `eprintln!`.
///
/// Operators see this string when CI hosts upgrade tack out from
/// under the catalog. The text names the observed version, the
/// pinned version, and the four-step upgrade path so the operator
/// can fix the gate without grepping the source.
#[must_use]
pub fn unsupported_tack_diagnostic(observed_maj: u32, observed_min: u32) -> String {
    format!(
        "tack {observed_maj}.{observed_min:02} installed but Section 05's catalog is pinned to \
         tack {pmaj}.{pmin:02}. Tack scenarios will SKIP. To re-pin: \
         (1) update TACK_PINNED_MAJOR/MINOR in session/version_gate/mod.rs, \
         (2) run `INSTA_UPDATE=1 cargo test -p oriterm_core --test tack -- \
         test_menu::begin_testing_inventory` to capture the new menu, \
         (3) update BEGIN_TESTING_INVENTORY in \
         tack_framework/scenarios/begin_testing_inventory/mod.rs, \
         (4) re-run the full test_menu suite to update affected snapshots.",
        pmaj = TACK_PINNED_MAJOR,
        pmin = TACK_PINNED_MINOR,
    )
}

/// Pure version check with an injected diagnostic emitter.
///
/// Takes pre-captured `tack -V` stdout/stderr and a closure used to
/// emit the loud-skip diagnostic on mismatch. Used by both
/// [`tack_version_supported`] (with `eprintln!` as the emitter) and
/// the unit tests (with a `String` accumulator). Splitting the I/O
/// from the policy means the loud-skip emit + silent-on-match pins
/// can run without spawning a subprocess and without depending on
/// the host-installed tack version.
///
/// Returns `true` iff the parsed version exactly matches
/// ([`TACK_PINNED_MAJOR`], [`TACK_PINNED_MINOR`]). Calls `emit` with
/// the [`unsupported_tack_diagnostic`] text on mismatch — never on
/// match (silent-on-match invariant).
#[must_use]
pub fn check_tack_version_with_emit(
    stdout: &str,
    stderr: &str,
    emit: &mut dyn FnMut(String),
) -> bool {
    let Some((maj, min)) = parse_tack_version(stdout, stderr) else {
        return false;
    };
    let supported = maj == TACK_PINNED_MAJOR && min == TACK_PINNED_MINOR;
    if !supported {
        emit(unsupported_tack_diagnostic(maj, min));
    }
    supported
}

/// Returns `true` iff `tack -V` reports a version compatible with
/// the begin-testing menu inventory pinned by Section 05.
///
/// Probe is `tack -V`. Output looks like `tack version 1.08
/// (20170726)`. Anything that doesn't parse, or any (major, minor)
/// tuple that doesn't match the pinned values, returns false.
/// Section 05 / 06 / 08 scenarios use this to skip cleanly when
/// running on an unpinned tack — the alternative is dozens of
/// cascading scenario failures that obscure the real issue (a tack
/// upgrade requires re-running discovery).
///
/// Returns `false` (not panic) on missing tack so this gate is
/// safe to call from `ScenarioRunner::available()` without an
/// extra existence check.
///
/// **Loud-skip discipline.** When `tack` IS installed but reports
/// a non-pinned version (e.g., a CI host upgraded to tack 2.x),
/// the function calls `eprintln!` with an actionable message
/// naming the observed version, the pinned version, and the
/// upgrade path. The `eprintln!` is the *only* loud signal — the
/// function still returns `false` so dozens of scenarios skip
/// cleanly instead of cascading failures. Without the loud
/// signal, an upgrade goes unnoticed and the test catalog quietly
/// stops covering anything.
///
/// **Upgrade path.** When a CI host upgrades tack:
/// 1. Update [`TACK_PINNED_MAJOR`] / [`TACK_PINNED_MINOR`] in
///    `crates/oriterm_test_support/src/session/version_gate/mod.rs`.
/// 2. Run `INSTA_UPDATE=1 cargo test -p oriterm_core --test tack
///    -- test_menu::begin_testing_inventory` to capture the new
///    menu graph.
/// 3. Update `BEGIN_TESTING_INVENTORY` in
///    `crates/oriterm_test_support/src/tack_framework/scenarios/begin_testing_inventory/mod.rs`
///    to match the new inventory.
/// 4. Re-run the full `test_menu` suite to update any snapshots
///    affected by changed menu wording.
#[must_use]
pub fn tack_version_supported() -> bool {
    let Ok(out) = std::process::Command::new("tack")
        .arg("-V")
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .output()
    else {
        return false;
    };
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    check_tack_version_with_emit(&stdout, &stderr, &mut |msg| eprintln!("{msg}"))
}

/// Pure AND-combine of the three test-availability gates.
///
/// Used by [`crate::tack_framework::ScenarioRunner::available`] and
/// pinned by a unit test that injects all 8 boolean combinations to
/// catch a regression that flips AND to OR.
#[must_use]
pub fn tack_runner_available_combine(tack: bool, tic: bool, version_supported: bool) -> bool {
    tack && tic && version_supported
}

#[cfg(test)]
mod tests;
