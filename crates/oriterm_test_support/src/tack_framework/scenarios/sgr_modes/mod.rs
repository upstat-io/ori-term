//! Section 06.2 scenario const + parser for tack's
//! `g) ANSI SGR modes` tool screen.
//!
//! # Empirical reality (tack v1.08 on 2026-04-08)
//!
//! After navigating `t -> g` from tack's main menu, tack clears the
//! screen and draws a stable 8-row × 10-column `Mode 0`..`Mode 79`
//! grid with each label wearing its corresponding SGR attribute, then
//! prints a sub-prompt:
//!
//! ```text
//! tack/tools/sgr Enter =><?r [<cr>] >
//! ```
//!
//! The sub-prompt is waiting for one of `=`, `<`, `>`, `?`, `r`
//! (private-use SGR sub-form selectors) or bare `\r` to proceed. The
//! `Mode 0..79` grid is ALREADY rendered at this point — it is NOT
//! gated behind the sub-prompt. We capture at the `Mode 79` anchor
//! (last label on the grid) and skip sending any sub-prompt input.
//! `quit_tack` then sends bare `q` characters which tack interprets
//! as the default `\r` at the sgr sub-prompt (verified empirically:
//! `q` is NOT in the `=<?r` allowed set, so tack runs the default
//! "Test enter/exit attributes" sub-test and returns to the parent
//! tools menu prompt `tack/tools [q] >`). The second `q` quits the
//! tools menu, the third quits the main menu, and tack exits clean.
//!
//! # Plan deviation: no `TACK_TOOLS_SGR_QUESTION`
//!
//! The 06.2 plan called for an additional scenario that sends
//! `b"?\r"` at the sgr sub-prompt to exercise the `?` private-use
//! sub-form, asserting "the resulting screen renders the `Mode N ?`
//! entries". Empirical probe against tack v1.08 established that the
//! `?` variant does NOT produce visually distinct labels — it
//! re-renders the same `Mode 0`..`Mode 79` grid but uses `CSI ? N m`
//! (DEC-private SGR) escape sequences to set each mode's attributes
//! instead of the standard `CSI N m` sequences. `grid_text()`
//! captures the rendered text after VTE parsing, and the rendered
//! text is IDENTICAL for both variants — only the underlying
//! attribute-set escape sequences differ. A tack scenario for the
//! `?` variant would produce the same insta snapshot as the default
//! and add zero additional coverage at the text layer.
//!
//! DEC-private SGR parsing validation lives in Section 06.5
//! `tack_cap_xcheck` — direct-VTE round-trip tests in
//! `oriterm_core/src/term/handler/tack_cap_xcheck/` feed synthetic
//! `CSI ? N m` sequences into `Term<RecordingListener>` and assert
//! the parser accepts them without regressing cell attributes. That
//! is the correct canonical home for cap-level DEC-private SGR
//! coverage; duplicating it as a tack scenario with identical
//! `grid_text` would be ``.
//!
//! # Plan deviation: the `\r` step
//!
//! The original 06.2 plan specified a three-step menu path
//! `[t, g, \r]` with the `\r` step's `wait_for: "Mode 79"`. This
//! would fire the navigator's pre-existing-anchor guard at runtime:
//! `Mode 79` is drawn by the `g` step itself, so by the time the
//! guard checks the grid before sending `\r`, the anchor is already
//! present. Capturing at the `g` step with `wait_for: "Mode 79"` is
//! structurally identical to the plan's intent (the Mode 0..79 grid
//! is visible before `\r` is ever sent) and avoids the guard
//! violation.

use crate::tack_framework::parser::tokens::grid_has_token;
use crate::tack_framework::{MenuStep, ScenarioSpec, ScreenFacts};

/// Number of Mode labels tack draws on the SGR sub-screen (0..79).
/// Tests assert `found_modes_count >= MIN_EXPECTED_MODES` which pins
/// the full 80-mode capture against tack v1.08. The constant lives at
/// module scope so the sibling parser tests, the 80x24 wrapper, and
/// any future size-matrix wrapper reference the same threshold.
pub const MIN_EXPECTED_MODES: usize = 80;

/// Section 06.2 scenario: tack's default SGR mode table.
/// Navigates `t -> g` and captures at the first appearance of the
/// `Mode 79` label. The Mode 0..79 table is drawn IMMEDIATELY after
/// `g` lands; there is no `\r` step (see module rustdoc for the plan
/// deviation rationale).
pub const TACK_TOOLS_SGR: ScenarioSpec = ScenarioSpec {
 id: "tack_tools_sgr",
 screen_id: "tack_tools_sgr",
 menu_path: &[
 MenuStep::new(b"t", "tack/tools [q] >"),
 // After `g`, tack clears the screen and draws the 80-mode
 // grid followed by the sgr sub-prompt. `Mode 79` is unique to
 // this screen — no earlier menu contains it — so it is a
 // valid post-send anchor that cannot pre-exist on the tools
 // menu screen from the previous step.
 MenuStep::new(b"g", "Mode 79"),
 ],
 ready_anchor: "Mode 79",
 quit_path: None,
 parser: parse_sgr_modes_screen,
};

/// Count tack's 80 `Mode N` labels (N = 0..80) in `grid` as
/// whitespace-bounded tokens.
/// # Format
/// Tack formats each label as `Mode %2d` (printf-style, 2-char
/// right-aligned number), so single-digit modes have two spaces
/// between `Mode` and the digit (`Mode 0`) and double-digit modes
/// have one (`Mode 10`,.., `Mode 79`). The parser matches the exact
/// internal whitespace; [`grid_has_token`] enforces ASCII-whitespace
/// boundaries on the outside. This is collision-free: `Mode 10` at
/// position `i` cannot false-positive as `Mode 1` because the
/// trailing `0` would need to be ASCII whitespace to satisfy the
/// right-boundary check of a `Mode 1` match, but it is the digit
/// `0`.
/// # Returns
/// [`ScreenFacts`] with a single note `"found_modes_count=<count>"`
/// where `<count>` is the number of matched labels. Tests assert
/// `count >= MIN_EXPECTED_MODES`.
pub fn parse_sgr_modes_screen(grid: &str) -> ScreenFacts {
 let count = count_mode_labels(grid);
 let header = grid
 .lines()
 .map(str::trim)
 .find(|line| !line.is_empty())
 .unwrap_or("")
 .to_string();
 ScreenFacts {
 header_text: header,
 capability_labels: Vec::new(),
 notes: vec![format!("found_modes_count={count}")],
 }
}

/// Count how many `Mode N` labels for `N = 0..80` appear in `grid`
/// as whitespace-bounded tokens. Extracted so the sibling parser
/// tests can pin individual-label behavior without spinning up the
/// full [`parse_sgr_modes_screen`] envelope each time.
pub(super) fn count_mode_labels(grid: &str) -> usize {
 let mut count = 0_usize;
 for n in 0..80_u32 {
 // `{n:>2}` right-aligns the number in 2 chars, matching
 // tack's `printf("%2d", n)` output exactly:
 // n=0 -> "Mode 0" (two spaces before the digit)
 // n=10 -> "Mode 10" (one space, two-digit number)
 let token = format!("Mode {n:>2}");
 if grid_has_token(grid, &token) {
 count += 1;
 }
 }
 count
}

#[cfg(test)]
mod tests;
