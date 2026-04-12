//! Tack ACS / graphic-rendition scenario const + parser.
//!
//! The `a) test alternate character set and graphic rendition`
//! menu key on tack's begin-testing submenu navigates to the
//! `tack/test/acs [n] >` sub-menu (verified by 05.0's
//! `BEGIN_TESTING_INVENTORY` discovery and the 05.2 empirical
//! probe). Pressing `n` from there runs the standard ACS test.
//!
//! # Empirical reality (tack v1.08)
//!
//! Tack v1.08's "alternate character set" test does NOT draw the
//! DEC line-drawing block (U+2500..=U+257F) that the plan's
//! [`parse_acs_screen`] parser was designed for. The captured
//! output (verified via `expect` against tack v1.08 / xterm-256color
//! on 2026-04-08) is:
//!
//! ```text
//! \x1B[H\x1B[2JTesting bell (bel)
//! If you did not hear the Bell then (bel) has failed.  (bel) Done
//! ```
//!
//! That is — the ACS test on tack v1.08 actually probes only the
//! `bel` capability, then reports `Done`. No box-drawing characters,
//! no `acsc` rendering, no extended-character set probing visible
//! to the test driver. Tack tests `acsc` INTERNALLY (the bell test
//! itself relies on the bel string from terminfo) but doesn't
//! emit any visual character-set sample to the screen.
//!
//! This mirrors the 05.1 modes-test discovery where tack v1.08
//! emits only `(os) Done` for the modes sweep — see
//! `oriterm_core/tests/tack/test_menu/modes.rs` rustdoc for the
//! full evidence chain.
//!
//! # Hybrid coverage strategy
//!
//! [`TACK_ACS_GRAPHIC_CHARS`] is coded to the plan's spec — the
//! `menu_path` navigates the verified `n -> a -> n` path, the
//! `ready_anchor` is `Done`, the parser is the line-drawing-char
//! counter the plan envisioned. The `#[test] fn` wrapper in
//! `oriterm_core/tests/tack/test_menu/acs.rs` runs the scenario
//! against real tack v1.08 and asserts the grid contains `Done`
//! (proving the test ran end-to-end) plus snapshots the captured
//! grid for visual regression. The wrapper does NOT assert on
//! the parser's `distinct_line_drawing_chars=N` note because that
//! count is always 0 against tack v1.08.
//!
//! The parser is preserved as forward-compatible infrastructure:
//! a future tack release that DOES draw line-drawing chars on the
//! ACS screen, or a downstream consumer that runs a stricter
//! ACS-rendering test, can use it without further code changes.

use crate::tack_framework::{MenuStep, ScenarioSpec, ScreenFacts};

/// Parser for the ACS screen: counts distinct DEC line-drawing
/// chars (Unicode block U+2500..=U+257F) present in the grid.
///
/// Returns the count via the `notes` field
/// (`"distinct_line_drawing_chars=N"`). The current tack v1.08
/// ACS test does NOT emit any chars in this range — see the
/// module rustdoc for the empirical evidence — so the count is
/// 0 against the live host. The parser is preserved for forward
/// compatibility with a future tack release or alternate ACS
/// runner that draws line-drawing characters.
///
/// **Why a count, not `grid.contains`.** The DEC line-drawing
/// block is sparse and the codepoints are unambiguous. A count
/// assertion is more robust than testing for a specific
/// codepoint that tack's chosen sample set may or may not
/// include.
pub fn parse_acs_screen(grid: &str) -> ScreenFacts {
    let mut chars: std::collections::BTreeSet<char> = std::collections::BTreeSet::new();
    for ch in grid.chars() {
        if ('\u{2500}'..='\u{257F}').contains(&ch) {
            chars.insert(ch);
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
        capability_labels: Vec::new(),
        notes: vec![format!("distinct_line_drawing_chars={}", chars.len())],
    }
}

/// Tack ACS / graphic-character-set scenario.
///
/// Navigates the verified path from 05.0's
/// `BEGIN_TESTING_INVENTORY`: `n` (enter test menu) → `a` (enter
/// ACS sub-menu, prompt becomes `tack/test/acs [n] >`) → `n`
/// (run the standard ACS test, terminator is `Done`).
///
/// Anchor strings empirically verified against tack v1.08
/// (2026-04-08) — see the module rustdoc for the captured-output
/// evidence.
pub const TACK_ACS_GRAPHIC_CHARS: ScenarioSpec = ScenarioSpec {
    id: "tack_acs_graphic_chars",
    screen_id: "tack_acs_graphic_chars",
    menu_path: &[
        MenuStep::new(b"n", "tack/test [n] >"),
        MenuStep::new(b"a", "tack/test/acs [n] >"),
        MenuStep::new(b"n", "Done"),
    ],
    ready_anchor: "Done",
    quit_path: None,
    parser: parse_acs_screen,
};

#[cfg(test)]
mod tests;
