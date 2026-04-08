//! Parses `extra/ori_term.info` at test time and builds a coverage
//! matrix against the Section 05 / 06 / 08 scenario catalog.
//!
//! Embedded via `include_str!` so the test does not need a working
//! directory or filesystem access — runs cross-platform.
//!
//! # Owner-partitioned exemption sources (Pivot 5)
//!
//! Each consuming section owns its own [`CapCoverageContribution`]
//! in a sibling submodule (`section_05`, `section_06`, `section_08`).
//! The matrix test sums them. This avoids the
//! single-flat-`EXEMPT_CAPS`-junk-drawer pattern an earlier draft
//! had. Adding a new consuming section = new file, not a new
//! entry in a 200-line flat list.
//!
//! # Why this is a separate concern from screen coverage
//!
//! Sections 05/06 ensure every navigable screen has a scenario.
//! That is necessary but not sufficient: tack can drift from the
//! terminfo without test failures if a NEW cap is added to
//! `extra/ori_term.info` and no scenario exercises it. The
//! cap-coverage matrix closes that gap by parsing the SSOT
//! terminfo file at test time and asserting every cap is exercised
//! by at least one scenario (or is on a per-section
//! [`CapCoverageContribution::exempt`] slice with a comment
//! explaining why).
//!
//! # Stale-exemption negative pin
//!
//! A cap appearing in BOTH any section's `covered` AND any
//! section's `exempt` (its own OR another section's) makes the
//! matrix test fail loudly. This forces the cleanup hand-off —
//! when Section 06 adds its tools-menu caps to `section_06`'s
//! `covered`, it MUST also remove them from any section's
//! `exempt`. The negative pin protects against the SSOT decay
//! this whole structure exists to prevent.

use std::collections::BTreeSet;

pub mod section_05;
pub mod section_06;
pub mod section_08;

/// The pinned terminfo source, embedded at compile time. Same
/// `include_str!` pattern as `TerminfoEnv::compile()`.
const TERMINFO_SRC: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../extra/ori_term.info"
));

/// One section's contribution to the cap-coverage matrix.
///
/// `covered` is the list of caps this section's scenarios actively
/// exercise; an entry here means at least one `#[test] fn` in this
/// section drives a tack scenario whose execution path invokes the
/// cap (the cap is part of the test pipeline that produces the
/// captured grid + `Done` terminator). `exempt` is the list of caps
/// this section intentionally does NOT cover, with a `(cap, reason)`
/// tuple per entry. The matrix test sums `covered` and `exempt`
/// across every section's contribution.
///
/// Adding a cap to `exempt` is a code-review event — it bypasses
/// the coverage gate and requires explicit justification. Adding a
/// cap to `covered` means a test exists.
///
/// The stale-exemption negative pin (in the matrix test in
/// `oriterm_core/tests/tack/test_menu/cap_coverage_matrix.rs`) fires
/// loudly when a cap appears in BOTH any section's `covered` AND
/// any section's `exempt`. This forces Sections 06 / 08 to clean up
/// their exemptions in lockstep with adding their `covered` entries
/// — the SSOT decay protection.
pub struct CapCoverageContribution {
    /// Section identifier (e.g. `"05"`, `"06"`, `"08"`).
    pub section: &'static str,
    /// Caps actively exercised by this section's scenarios.
    pub covered: &'static [&'static str],
    /// Caps this section deliberately exempts, with reasons.
    pub exempt: &'static [(&'static str, &'static str)],
}

/// All section contributions, in declaration order. Adding a new
/// section = adding one entry to this array AND a new sibling file.
pub const ALL_CONTRIBUTIONS: &[&CapCoverageContribution] = &[
    &section_05::CONTRIBUTION,
    &section_06::CONTRIBUTION,
    &section_08::CONTRIBUTION,
];

/// Sum every section's `covered` slice into a `BTreeSet<String>`.
///
/// Iterates [`ALL_CONTRIBUTIONS`] so adding a new section is a
/// one-line edit to the array.
#[must_use]
pub fn covered_caps() -> BTreeSet<String> {
    let mut s = BTreeSet::new();
    for contrib in ALL_CONTRIBUTIONS {
        for cap in contrib.covered {
            s.insert((*cap).to_string());
        }
    }
    s
}

/// Sum every section's `exempt` slice into a `BTreeSet<String>`.
/// The matrix test uses this for the stale-exemption negative pin.
#[must_use]
pub fn exempt_caps() -> BTreeSet<String> {
    let mut s = BTreeSet::new();
    for contrib in ALL_CONTRIBUTIONS {
        for (cap, _reason) in contrib.exempt {
            s.insert((*cap).to_string());
        }
    }
    // Iterator-built keyboard exemptions (Section 08) so we don't
    // hand-write 60+ rows in `section_08.rs`. The kf cap range and
    // modified-key bases are stable across tack versions.
    for cap in expand_kf_caps() {
        s.insert(cap);
    }
    for cap in expand_modified_key_caps() {
        s.insert(cap);
    }
    s
}

/// Helper expansion: every kfNN cap from kf1 through kf63.
///
/// Used by the matrix test (and re-exported for Section 08 to
/// compare against). Section 08's `covered_caps` extension MUST
/// produce the same range or the stale-exemption pin fires.
#[must_use]
pub fn expand_kf_caps() -> Vec<String> {
    (1..=63).map(|n| format!("kf{n}")).collect()
}

/// Helper expansion: every modified arrow / Home / End / editing
/// key cap that `extra/ori_term.info` declares.
///
/// The base names (`kLFT`, `kRIT`, `kUP`, `kDN`, `kEND`, `kHOM`,
/// `kIC`, `kDC`, `kNXT`, `kPRV`) appear with no suffix (the
/// shift-only form) and with mod-param suffixes 3..=7 (alt, alt+shift,
/// ctrl, ctrl+shift, ctrl+alt). Plus two specials (`kind`, `kri`)
/// that ncurses defines as the shift-down / shift-up cursor keys.
/// Mirrors the actual cap names declared at
/// `extra/ori_term.info:159-181` — the `expand_modified_key_caps_matches_terminfo`
/// test asserts the lists match.
#[must_use]
pub fn expand_modified_key_caps() -> Vec<String> {
    let bases = [
        "kLFT", "kRIT", "kUP", "kDN", "kEND", "kHOM", "kIC", "kDC", "kNXT", "kPRV",
    ];
    let mut out = Vec::new();
    for base in bases {
        out.push((*base).to_string());
        for suffix in 3..=7 {
            out.push(format!("{base}{suffix}"));
        }
    }
    out.push("kind".to_string());
    out.push("kri".to_string());
    out
}

/// Parse a terminfo source string and return the set of declared
/// cap names (boolean caps + numeric caps + string caps).
///
/// The parser handles tic format quirks:
/// - Comment lines (`# ...`) are skipped.
/// - Entry header lines (start in column 0, end with `,`) are
///   skipped — they declare the entry name and aliases, not caps.
/// - Continuation lines (cap value continues on next indented line)
///   are NOT parsed as cap declarations — only the line that STARTS
///   the cap is processed.
/// - `use=other_term` references are skipped (the leading `use`
///   token is not a cap name).
/// - Cancellation markers (`name@`) are KEPT in the set — `@` means
///   "cancel inherited cap" but the cap NAME is still part of the
///   entry's surface, so the matrix-coverage gate must account for it.
/// - Numeric caps (`name#256`) and string caps (`name=value`) are
///   both included; only the leading identifier (everything before
///   `=`/`#`/`@`/whitespace) is extracted as the cap name.
///
/// Pure function over the input string — no embedded source coupling.
/// The public [`parse_declared_caps`] wrapper calls this with
/// [`TERMINFO_SRC`]; the sibling tests in `tests.rs` call it with
/// synthetic terminfo strings to exercise each tic format quirk in
/// isolation. Single canonical home for the parser body — no
/// algorithmic duplication between production and test code.
pub(super) fn parse_terminfo_source(src: &str) -> BTreeSet<String> {
    let mut caps = BTreeSet::new();
    let mut in_continuation = false;
    for raw_line in src.lines() {
        let trimmed = raw_line.trim_start();
        if trimmed.starts_with('#') || trimmed.is_empty() {
            in_continuation = false;
            continue;
        }
        // Entry header lines start in column 0 (no leading whitespace).
        // They are the entry name + aliases, not cap declarations.
        if !raw_line.starts_with(char::is_whitespace) {
            in_continuation = false;
            continue;
        }
        // Cap declarations end with `,` on the line that DECLARES
        // the cap. Continuation lines (the cap value spans multiple
        // physical lines) do NOT end with `,` — they end mid-value.
        // Track continuation state: if the previous line did not
        // end with `,`, this line is a continuation and we skip
        // cap-name extraction.
        let line_ended_with_comma = raw_line.trim_end().ends_with(',');
        if in_continuation {
            in_continuation = !line_ended_with_comma;
            continue;
        }
        in_continuation = !line_ended_with_comma;
        // Cap declarations are comma-separated within a logical
        // line. A cap name is the leading identifier, optionally
        // followed by `=`, `#`, or `@` (cancellation).
        for token in trimmed.split(',') {
            let t = token.trim();
            if t.is_empty() || t.starts_with("use=") {
                continue;
            }
            let name: String = t
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
                .collect();
            if !name.is_empty() {
                caps.insert(name);
            }
        }
    }
    caps
}

/// Parse the embedded `extra/ori_term.info` and return the set of
/// declared cap names. Thin wrapper over [`parse_terminfo_source`]
/// that fixes the source to [`TERMINFO_SRC`] (the embedded file).
#[must_use]
pub fn parse_declared_caps() -> BTreeSet<String> {
    parse_terminfo_source(TERMINFO_SRC)
}

#[cfg(test)]
mod tests;
