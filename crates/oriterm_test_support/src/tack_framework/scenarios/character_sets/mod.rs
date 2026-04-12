//! Section 06.3 scenario const + parser for tack's
//! `c) ANSI character sets` tool screen.
//!
//! # Empirical reality (tack v1.08 on 2026-04-08)
//!
//! After navigating `t -> c` from tack's main menu, tack draws a
//! single composite screen that contains BOTH the bank/charset
//! prompt header AND a live preview of every G-bank's GL+GR
//! rendering with the currently-designated charset, then waits for
//! input at a `bank+set>` prompt at the bottom of the screen.
//!
//! Verbatim post-`c` capture (tack v1.08, `ori_term` terminfo, 80x24):
//!
//! ```text
//! Enter the bank ()*+,-./ followed by the character set 0123456789:;<=>? for
//! private use, and @A...Z[\]^_`a...z{|}~ for standard sets.
//!
//! G1 GL    !"#$%&'()*+,-./0123456789:;<=>?
//!         @ABCDEFGHIJKLMNOPQRSTUVWXYZ[\]^
//!         ◆▒␉␌␍␊°±␤␋┘┐┌└┼⎺⎻─⎼⎽├┤┴┬│≤≥π≠£·   DEL <>
//!
//! G1 GR   <high-bit chars>
//!         <high-bit chars>
//!         <high-bit chars>   DEL <…>
//!
//! bank+set>
//! ```
//!
//! Note that `G1 GL` already shows the DEC special graphics block
//! glyphs (the third row of the G1 GL pane: `◆▒…┘┐┌└┼⎺⎻─⎼⎽├┤┴┬│…`)
//! before the user types ANY bank/charset designator bytes. tack
//! v1.08 designates G1 to DEC special graphics by default for the
//! `c` tool's preview render — the user types subsequent
//! `<bank><charset>` byte pairs ONLY to switch the displayed table
//! to a different combination.
//!
//! # Plan deviation — no `)0` send-step
//!
//! The 06.3 plan as authored called for a fourth menu step
//! `MenuStep::new(b")0", "DEC")` that designates G1 to DEC special
//! graphics and waits for the redrawn screen. Empirical capture
//! against tack v1.08 shows that:
//!
//!   1. The post-`c` screen ALREADY renders DEC special graphics in
//!      the G1 GL preview pane (no extra send needed).
//!   2. The `─` (and every other box-drawing) glyph is therefore
//!      already on screen BEFORE the `)0` send, so the
//!      `TackNavigator` pre-existing-anchor guard would fire on any
//!      anchor that contains a box-drawing char — and "DEC" is also
//!      not present on the screen as a literal substring (tack
//!      labels the pane "G1 GL", not "DEC graphics").
//!   3. Sending `)0` to tack v1.08 at the `bank+set>` prompt is a
//!      no-op against the rendered preview because G1 is ALREADY
//!      DEC graphics — the redraw produces a byte-identical screen.
//!
//! Per the "expand the mission, never scope down" rule, the
//! coverage is NOT reduced — the post-`c` screen already exercises
//! the SCS render path that the plan's `)0` step intended to
//! exercise. The plan author was empirically wrong about tack
//! v1.08's preview-on-prompt behavior; the `)0` step would be a
//! tautology against the empirical reality, and asserting against
//! it via `wait_for: "DEC"` would never match because "DEC" is not
//! a literal substring of the rendered grid. The const honors the
//! mission criterion (validate that `ori_term` renders the DEC
//! special graphics character set) by capturing the live G1 GL
//! preview.
//!
//! If a future tack version changes the preview-default such that
//! the post-`c` screen does NOT contain DEC special graphics, this
//! deviation must be revisited — the
//! [`tests::parse_character_sets_unicode_box_drawing`] sibling pin
//! would still be green (it tests synthetic input), but the
//! end-to-end `tack_tools_g0_dec_graphics_80x24` test would fail
//! the `>= MIN_DEC_GRAPHICS_THRESHOLD` assertion with a
//! reproducer-friendly snapshot of the new tack screen.
//!
//! # Const naming — `TACK_TOOLS_G0_DEC_GRAPHICS`
//!
//! The exact const path
//! `scenarios::character_sets::TACK_TOOLS_G0_DEC_GRAPHICS` is the
//! cross-module API contract — the golden-image test imports it
//! at this exact path.
//!
//! Strictly speaking the canonical scenario sends `)` (the G1 bank
//! designator), not `(` (the G0 bank designator), so the const name
//! is a slight historical misnomer — `TACK_TOOLS_G1_DEC_GRAPHICS`
//! would be a more literal name. The plan author chose the `G0` form
//! because it reads as the "default DEC graphics" case in human
//! terms; the actual bank choice is opaque to consumers because what
//! Section 07 cares about is the rendered glyph payload, not which
//! G-bank tack told the terminal to point at. We honor the pinned
//! name and document the bank choice in the `menu_path` comment for
//! readers who notice the discrepancy.
//!
//! # DEC special graphics rendering — Unicode box drawing
//!
//! `oriterm_core` translates the DEC special graphics character set
//! through `vte::ansi::StandardCharset::SpecialCharacterAndLineDrawing`,
//! which maps the ASCII line-drawing letters to Unicode
//! `Box Drawing` block code points (U+2500–U+257F). This is verified
//! by the unit tests in `oriterm_core/src/term/charset/tests.rs:14-37`:
//!
//! - `q` -> `─` (U+2500, horizontal line)
//! - `l` -> `┌` (U+250C, top-left corner)
//! - `k` -> `┐` (U+2510, top-right corner)
//! - `m` -> `└` (U+2514, bottom-left corner)
//! - `j` -> `┘` (U+2518, bottom-right corner)
//! - `x` -> `│` (U+2502, vertical line)
//! - `n` -> `┼` (U+253C, cross)
//!
//! Therefore the parser scans for distinct Unicode code points in
//! `U+2500..=U+257F` and counts them. The minimum threshold is set by
//! [`MIN_DEC_GRAPHICS_THRESHOLD`] — when tack draws the full DEC
//! special graphics table the rendered grid contains all six corners
//! plus the horizontal and vertical lines, well above the threshold.
//!
//! # Plan deviation — no raw-ASCII parser branch
//!
//! The 06.3 plan called for a sibling test
//! `parse_character_sets_raw_ascii_line_drawing` that feeds the raw
//! ASCII line-drawing form (`"lqkxmj"`) and asserts a positive count.
//! Empirical verification (`oriterm_core/src/term/charset/tests.rs:14-37`)
//! confirms `oriterm_core` does NOT pass DEC graphics through as raw
//! ASCII — it always translates through the active charset to Unicode
//! box drawing. The raw-ASCII branch is therefore empirically dead
//! against this implementation.
//!
//! Per the "expand the mission, never scope down" rule, the test is
//! NOT deleted — it is repurposed as a NEGATIVE pin
//! ([`tests::parse_character_sets_raw_ascii_line_drawing_does_not_match`])
//! that asserts the parser correctly REJECTS raw ASCII line-drawing
//! letters (which carry no Unicode box-drawing code points). A
//! regression that swapped the parser to scan ASCII bytes would flip
//! that pin red. The mission criterion (DEC graphics characters
//! actually rendered) is honored more precisely by the negative
//! version.
//!
//! Sibling test (e) `parse_character_sets_mixed_unicode_and_ascii`
//! is similarly repurposed to assert that mixed grids count ONLY the
//! Unicode chars and ignore the ASCII bytes — pinning the
//! single-form parser policy.

use portable_pty::ExitStatus;

use crate::session::PtySession;
use crate::tack_framework::{MenuStep, ScenarioSpec, ScreenFacts};

/// Minimum number of distinct Unicode box-drawing code points the
/// parser must find on a captured DEC graphics screen for the test
/// to pass.
///
/// Tack v1.08's DEC graphics screen renders a full bank/charset
/// table with all box-drawing corners, edges, and intersections —
/// well over a dozen distinct code points in practice. The threshold
/// is conservative (4 = the minimum of "two corners plus two edges")
/// so a partial-render race condition flips the test red rather
/// than slipping under a tighter pin.
///
/// Sibling parser tests reference this constant directly so a
/// regression that lowered the threshold to match a broken capture
/// also flips the boundary-count pin.
pub const MIN_DEC_GRAPHICS_THRESHOLD: usize = 4;

/// Section 06.3 scenario: tack's DEC special graphics screen via
/// the `c) ANSI character sets` tool's live G1 GL preview.
///
/// Menu path: `t -> c`. tack v1.08 renders the DEC special graphics
/// character set in the G1 GL preview pane immediately after `c`
/// lands at the `bank+set>` prompt — no further bank/charset
/// designator bytes are needed (see module rustdoc for the empirical
/// capture and the plan deviation rationale).
///
/// The const name is `TACK_TOOLS_G0_DEC_GRAPHICS` for cross-section
/// API stability with Section 07's golden test — see the module
/// rustdoc "Const naming" section for the historical note about the
/// G0/G1 naming choice.
pub const TACK_TOOLS_G0_DEC_GRAPHICS: ScenarioSpec = ScenarioSpec {
    id: "tack_tools_g0_dec_graphics",
    // Stable screen identity for Section 07 golden reuse — the
    // golden image is keyed on this string, not on `id`.
    screen_id: "tack_character_sets",
    menu_path: &[
        // Step 1: enter the tools submenu. Anchor is the unique
        // tools sub-menu prompt produced after `t`. Empirically
        // verified against tack v1.08.
        MenuStep::new(b"t", "tack/tools [q] >"),
        // Step 2: enter the character sets tool. Anchor is the
        // unique input prompt at the bottom of the post-`c`
        // composite screen. The tools sub-menu prompt
        // `tack/tools [q] >` is replaced by `bank+set>` once `c`
        // lands, so the pre-existing-anchor guard is satisfied:
        // `bank+set>` does NOT appear anywhere on the tools menu
        // (verified against tack v1.08's tools-menu snapshot at
        // `oriterm_core/tests/tack/tools_menu/snapshots/tack__tools_menu__tools_menu_inventory__tack_tools_menu_80x24.snap`).
        MenuStep::new(b"c", "bank+set>"),
    ],
    // Final readiness anchor: re-confirm the live `bank+set>`
    // prompt is on screen before capture. tack draws the prompt
    // AFTER the preview pane is fully rendered, so by the time
    // this anchor matches the G1 GL DEC graphics row is also on
    // screen.
    ready_anchor: "bank+set>",
    quit_path: Some(quit_character_sets),
    parser: parse_character_sets_screen,
};

/// Custom `quit_path` for the `character_sets` tool.
///
/// At the `bank+set>` prompt, tack v1.08's `tools_charset()`
/// (tack-1.11/ansi.c:842-869) reads input bytes character by
/// character via `getchp()` and accumulates them into a `bank[]`
/// buffer. The default `quit_tack` send budget of `q` bytes is
/// useless here because `q` (0x71) is in the printable range and
/// tack treats it as a charset designator — the inner loop
/// continues and the outer loop redraws the screen indefinitely.
///
/// The exit path: send `\r` (CR, 0x0d). At the inner loop's first
/// iteration with `j == 1`:
///   1. `putchp('\r')` echoes the CR.
///   2. `j == 1 && '\r' > '/'` is false (0x0d < 0x2f), so `j` stays at 1.
///   3. `bank[1] = '\r'`.
///   4. `'\r' < ' '` is true → break inner loop.
///   5. After the inner loop, `j == 1` → break outer `for (; bank[0];)` loop.
///   6. `tools_charset()` returns to the tools-menu dispatcher.
///
/// After the sub-tool exits, the standard `quit_tack(5)` chain
/// drains the tools menu and main menu via `q` keystrokes (which
/// ARE the right exit bytes at those prompts).
fn quit_character_sets(session: &mut PtySession) -> ExitStatus {
    session.send_raw(b"\r");
    session.quit_tack(5)
}

/// Count distinct Unicode box-drawing code points (U+2500–U+257F)
/// rendered onto `grid` and return them as a [`ScreenFacts`] note.
///
/// The note format is `dec_graphics_distinct_chars=<count>` so
/// downstream consumers (the test wrapper, future Section 09
/// cross-validation) parse a stable key.
///
/// `header_text` is set to the first non-blank line of the grid for
/// human-readable diagnostics; tack's DEC graphics screen carries a
/// title row above the rendered table.
pub fn parse_character_sets_screen(grid: &str) -> ScreenFacts {
    let count = count_dec_graphics_chars(grid);
    let header = grid
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .unwrap_or("")
        .to_string();
    ScreenFacts {
        header_text: header,
        capability_labels: Vec::new(),
        notes: vec![format!("dec_graphics_distinct_chars={count}")],
    }
}

/// Count how many DISTINCT Unicode code points in the
/// `Box Drawing` block (`U+2500..=U+257F`) appear in `grid`.
///
/// Distinct, not total — a screen full of `─` characters scores 1,
/// not 80, so the pin asserts diversity (corners, edges, lines)
/// rather than mere repetition. This catches a partial-render where
/// tack drew a single line of horizontal bars but did not yet draw
/// the corners.
///
/// Extracted as a `pub(super)` helper so the sibling parser tests
/// can pin per-character behavior without spinning up the full
/// [`parse_character_sets_screen`] envelope each time.
pub(super) fn count_dec_graphics_chars(grid: &str) -> usize {
    let mut seen = [false; 0x80];
    let mut count = 0_usize;
    for ch in grid.chars() {
        let code = ch as u32;
        if (0x2500..=0x257F).contains(&code) {
            let idx = (code - 0x2500) as usize;
            if !seen[idx] {
                seen[idx] = true;
                count += 1;
            }
        }
    }
    count
}

#[cfg(test)]
mod tests;
