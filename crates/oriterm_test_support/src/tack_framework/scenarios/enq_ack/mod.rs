//! Section 06.4 scenario const + parser for tack's
//! `u) test ENQ/ACK (DA1) handshake` tool screen.
//!
//! # Empirical reality (tack v1.08 on 2026-04-08)
//!
//! After navigating `t -> u` from tack's main menu, tack's
//! `sync_handshake()` (tack-1.11/sync.c:534) calls `verify_time()`
//! (tack-1.11/sync.c:309) which in turn invokes `probe_enq_ok()`
//! (tack-1.11/sync.c:140). The probe writes `Testing ENQ/ACK,
//! standby...`, sends the cap declared as `u9` (per
//! `extra/ori_term.info:115` `u9=\E[c` — DA1 query), and waits up
//! to 400 ms for an ACK response.
//!
//! `ori_term`'s DA1 handler at
//! `oriterm_core/src/term/handler/status.rs:121-148` responds with
//! the literal bytes `\x1b[?64;6;4c` (a VT420-class device
//! attributes report). tack reads this as the ACK, sets the
//! `ACK_terminator` to the trailing `c`, then verifies `tty_ACK`
//! against the `u8` regex `\E[?%[;0123456789]c` (per
//! `extra/ori_term.info:115`). The regex matches, so tack reports
//! the success path output (NOT the failure-path diagnostic the
//! original 06.4 plan expected).
//!
//! Verbatim post-`u` capture (tack v1.08 against `ori_term`, 80x24):
//!
//! ```text
//! Testing ENQ/ACK, standby...
//!
//! ACK terminating character: c
//! ```
//!
//! After the success report, tack's `verify_time()` checks
//! `tty_baud_rate == 0` and runs `sync_home()` if so — the baud-
//! rate test scrolls additional content onto the screen. Section
//! 06.4 captures the screen at the
//! `ACK terminating character: c` anchor, BEFORE the baud-rate
//! test starts producing scrolling output.
//!
//! # Plan deviation — success path, not failure path
//!
//! The 06.4 plan as authored anticipated the FAILURE-path diagnostic
//! (`ENQ sequence from (u9):`, `ACK received:`, `Length of ACK...`,
//! `Terminating character found in (u8):`) — that's the output
//! `probe_enq_ok()` produces when the terminal does NOT respond
//! within 400 ms or the ACK is empty. The plan author assumed
//! `ori_term` would not respond to ENQ.
//!
//! Empirically, `ori_term` DOES respond to the DA1 query (the byte
//! sequence tack actually sends as `tty_ENQ` because `u9=\E[c` is
//! declared in `extra/ori_term.info`), so the probe takes the
//! SUCCESS branch and prints `ACK terminating character: c`. This
//! is the CORRECT outcome — it proves tack's ENQ/ACK probe round-
//! trips through `ori_term`'s DA1 handler exactly as terminfo
//! declares. The mission criterion (validate the ENQ/ACK handshake)
//! is honored more strongly by the success-path output than the
//! failure-path output, because the success-path output proves
//! end-to-end correctness (`u9` send → handler → `u8` regex match
//! → ACK terminator extraction).
//!
//! The parser is therefore aligned with the empirical success path.
//! It extracts the `ACK terminating character` value and records it
//! as a structured note. A future `ori_term` regression that broke
//! the DA1 handler (or a terminfo edit that changed `u9`/`u8`)
//! would produce a different captured screen and the parser would
//! return a different note value, surfacing the regression with a
//! diagnostic-friendly snapshot.

use crate::tack_framework::parser::tokens::grid_find_field;
use crate::tack_framework::{MenuStep, ScenarioSpec, ScreenFacts};

/// Section 06.4 scenario: tack's ENQ/ACK handshake test.
///
/// Menu path: `t -> u`. tack writes `Testing ENQ/ACK, standby...`,
/// sends `u9` (DA1 query) to `ori_term`, reads the DA1 response,
/// validates it against the `u8` regex, and prints the
/// `ACK terminating character: <c>` line on success. The capture
/// anchor is the success-path label so the screen is captured
/// BEFORE the follow-on baud-rate test (`sync_home`) starts
/// scrolling additional content.
pub const TACK_TOOLS_ENQ_ACK: ScenarioSpec = ScenarioSpec {
    id: "tack_tools_enq_ack",
    screen_id: "tack_tools_enq_ack",
    menu_path: &[
        MenuStep::new(b"t", "tack/tools [q] >"),
        // After `u`, tack runs probe_enq_ok() which prints
        // `Testing ENQ/ACK, standby...` then either the success or
        // failure-path output. `ACK terminating character` is the
        // success-path success label — unique to the post-`u`
        // success screen and not present on the tools menu.
        MenuStep::new(b"u", "ACK terminating character"),
    ],
    ready_anchor: "ACK terminating character",
    // Empirically, tack v1.08's `sync_handshake()` returns the
    // user to the tools menu prompt automatically after the
    // success-path report (`tty_baud_rate` is non-zero by the
    // time `verify_time()` runs because tack initialised it from
    // the PTY descriptor at startup, so the `sync_home()` baud-
    // rate test does NOT execute). The captured grid shows the
    // tools menu re-drawn after the ENQ/ACK output, so the
    // canonical `quit_tack(5)` (sending `q` at the tools menu
    // prompt) is the right exit chain — no custom quit_path
    // needed.
    quit_path: None,
    parser: parse_enq_ack_screen,
};

/// Parse the ENQ/ACK success-path screen and record the ACK
/// terminating character as a structured note.
///
/// The `header_text` is the first non-blank line for human-readable
/// diagnostics; tack's screen begins with `Testing ENQ/ACK,
/// standby...`. The single note format is
/// `ack_terminator=<char>`; the test wrapper consumes this exact
/// prefix.
pub fn parse_enq_ack_screen(grid: &str) -> ScreenFacts {
    let header = grid
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .unwrap_or("")
        .to_string();
    let mut notes = Vec::new();
    if let Some(terminator) = grid_find_field(grid, "character:") {
        notes.push(format!("ack_terminator={terminator}"));
    }
    ScreenFacts {
        header_text: header,
        capability_labels: Vec::new(),
        notes,
    }
}

#[cfg(test)]
mod tests;
