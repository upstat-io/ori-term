//! Tack padding-and-string-capabilities scenario const + parser.
//!
//! Tack v1.08 combines what the original 05.4b plan envisioned as
//! THREE separate screens (`pad_timing`, `send_strings`, `labels`)
//! into a single `p) test padding and string capabilities` entry on
//! the begin-testing menu. This module covers that combined entry.
//! `labels` is unreachable on tack v1.08 (no `l)` key on the
//! begin-testing menu — see the inventory rustdoc for the
//! plan/reality reconciliation).
//!
//! # Empirical reality (tack v1.08)
//!
//! Pressing `p` from the begin-testing menu first triggers an
//! interactive ENQ/ACK probe — tack writes `Testing ENQ/ACK,
//! standby...\x1B[c` (a DA1 query) and waits for the terminal to
//! respond. Our [`crate::session::PtySession`] feeds tack output
//! through [`oriterm_core::Term`] which writes the DA1 response
//! (`\x1B[?64;6;4c`) back to the PTY automatically via the
//! `Event::PtyWrite` handler in
//! `crates/oriterm_test_support/src/session/sync/mod.rs:99-107`.
//! After the handshake, tack reports `ACK terminating character: c`
//! and enters the `tack/test/pad [n] >` sub-menu.
//!
//! Pressing `n` from the sub-menu runs the standard padding test.
//! On tack v1.08 against `extra/ori_term.info`, the captured grid
//! after the test is:
//!
//! ```text
//! (rs1) reset_1string, not present.  (rs1) Done
//! ```
//!
//! That is — tack v1.08's padding test only probes the `rs1`
//! capability (`reset_1string`), reports it is NOT PRESENT in
//! `extra/ori_term.info`, and reports `Done`. This is the same
//! single-cap-shortname pattern as 05.1 modes (only `(os)`),
//! 05.2 ACS/SGR (only `(bel)`), 05.3 color (`(colors)`+`(pairs)`),
//! 05.4 cursor movement (only `(clear)`).
//!
//! The `not present` part is also a finding — `extra/ori_term.info`
//! declares NO reset-string capabilities at all (neither `rs1`,
//! `rs2`, nor `rs3`). Whether that is a deliberate omission or an
//! oversight is for Section 05.5's cap-coverage matrix to settle.
//! The wrapper does NOT assert on the `not present` substring
//! because that is a property of the current `extra/ori_term.info`,
//! not of the padding test itself. (Earlier draft of this rustdoc
//! incorrectly said "delegate to `rs2`" corrected the
//! factual error against the pinned terminfo source.)
//!
//! # Hybrid coverage strategy
//!
//! [`TACK_PADDING`] is coded with the verified `menu_path`
//! (`n -> p -> n`) and `ready_anchor` (`Done`). The parser
//! [`parse_padding_screen`] scans for the small set of
//! string-capability short names tack might emit (`rs1`, `rs2`,
//! `rs3`, `rmcup`, `smcup`, `rmkx`, `smkx`, `is1`, `is2`, `is3`)
//! using [`grid_has_token`] to avoid substring collisions
//! (`rmcup` would never collide, but `is1`/`is2` would match
//! inside arbitrary letter/digit sequences). The parser is
//! preserved as forward-compatible infrastructure — against tack
//! v1.08 it returns only `rs1` because that's the only cap the
//! current `ori_term.info` exposes a probe target for.
//!
//! The wrapper at `oriterm_core/tests/tack/test_menu/padding.rs`
//! uses the hybrid strategy: assert `Done` (proves end-to-end
//! pipeline including the DA1 handshake) plus the testable
//! semantic facts (`(rs1)`, `reset_1string`) and snapshots the
//! captured grid for visual regression. 80x24 only — pad timing
//! is intrinsically size-independent (the test does not depend
//! on viewport dimensions for the cap probes).
//!
//! Section 05.5's cap-coverage matrix should record `rs1` as
//! probed by 05.4b — even though the result is "not present", the
//! probe IS exercised end-to-end. The string-capability set
//! (`rs1`/`rs2`/`rs3`/`is1`/`is2`/`is3`/`smcup`/`rmcup`/`smkx`/`rmkx`)
//! that tack would test if they were declared in `ori_term.info` is
//! the responsibility of the cap-coverage matrix to enforce, not
//! this wrapper.

use crate::tack_framework::parser::tokens::grid_has_paren_token;
use crate::tack_framework::{MenuStep, ScenarioSpec, ScreenFacts};

/// Parser for the padding-and-strings screen.
///
/// Scans for the string-capability short names tack would emit
/// (`rs1`/`rs2`/`rs3`/`is1`/`is2`/`is3`/`smcup`/`rmcup`/`smkx`/`rmkx`)
/// using [`grid_has_paren_token`] (matches tack's canonical
/// `(cap_name)` parenthesized format).
///
/// **Why parenthesized, not whitespace-bounded.** Tack v1.08's
/// padding test emits cap names wrapped in parens — e.g.
/// `(rs1) reset_1string, not present.  (rs1) Done`. This matches
/// the same parenthesized pattern as the modes test (`(am)`,
/// `(os)`, `(bel)`, `(colors)`, `(pairs)`, `(clear)`). Using
/// [`grid_has_paren_token`] (the canonical helper from
/// `parser/tokens/`) gives us the strongest collision resistance
/// — `is1`, `is2`, `is3`, `rs1`, `rs2`, `rs3` would otherwise
/// false-positive against arbitrary letter/digit sequences
/// (`is15`, `users2`, etc.) AND they would false-positive on the
/// other parts of tack's output that contain the cap name as a
/// SUBSTRING (e.g. `reset_1string` contains `s1`). Requiring the
/// parens enforces tack's canonical output format.
///
/// **Empirical caveat (tack v1.08).** As of tack v1.08 against
/// `extra/ori_term.info`, the padding test only probes `rs1` and
/// reports it as `not present`. The parser returns `["rs1"]` —
/// the only cap that appears as a parenthesized token on the
/// captured grid. The other caps in `STRING_CAPS` are preserved
/// as forward-compatible infrastructure for a future `ori_term.info`
/// that declares more reset/init/keypad caps.
pub fn parse_padding_screen(grid: &str) -> ScreenFacts {
    const STRING_CAPS: &[&str] = &[
        "rs1", "rs2", "rs3", "is1", "is2", "is3", "smcup", "rmcup", "smkx", "rmkx",
    ];
    let mut found = Vec::new();
    for cap in STRING_CAPS {
        if grid_has_paren_token(grid, cap) {
            found.push((*cap).to_string());
        }
    }
    let header = grid
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .unwrap_or("")
        .to_string();
    ScreenFacts {
        header_text: header,
        capability_labels: found,
        notes: Vec::new(),
    }
}

/// Tack padding-and-string-capabilities scenario.
///
/// Navigates the verified path: `n` (enter test menu) → `p`
/// (enter padding sub-menu — this triggers an interactive
/// ENQ/ACK / DA1 handshake that our `PtySession` answers via
/// [`oriterm_core::Term`]'s `Event::PtyWrite` handler) → `n`
/// (run the standard padding test, terminator is `Done`).
///
/// The post-`p` sub-menu prompt is `tack/test/pad [n] >` (tack's
/// short form for the combined padding+strings entry, NOT
/// `tack/test/padding`).
///
/// Anchor strings empirically verified against tack v1.08 +
/// `extra/ori_term.info` (2026-04-08) — see the module rustdoc.
pub const TACK_PADDING: ScenarioSpec = ScenarioSpec {
    id: "tack_padding",
    screen_id: "tack_padding",
    menu_path: &[
        MenuStep::new(b"n", "tack/test [n] >"),
        MenuStep::new(b"p", "tack/test/pad [n] >"),
        MenuStep::new(b"n", "Done"),
    ],
    ready_anchor: "Done",
    quit_path: None,
    parser: parse_padding_screen,
};

#[cfg(test)]
mod tests;
