//! Tack graphic-rendition (SGR) scenario const + parser.
//!
//! On tack v1.08 the graphic-rendition test shares the same menu
//! key as the ACS test (`a) test alternate character set and
//! graphic rendition`) and lands at the same `tack/test/acs [n] >`
//! sub-menu. Pressing `n` runs a single combined test that
//! probes only the `bel` capability — see the empirical evidence
//! recorded in
//! [`super::acs`]'s module rustdoc and in
//! `oriterm_core/tests/tack/test_menu/modes.rs` (same pattern).
//!
//! [`TACK_GRAPHIC_RENDITION_SGR`] is the second of the two 05.2
//! `ScenarioSpec` consts the plan envisioned. It points at the same
//! captured grid as [`super::acs::TACK_ACS_GRAPHIC_CHARS`] but
//! parses for SGR-style labels (bold/dim/underline/blink/reverse/
//! invis) instead of line-drawing characters. On tack v1.08
//! neither parser will find any matches because the test screen
//! only contains the `(bel) Done` terminator — the parser is
//! preserved as forward-compatible infrastructure for a future
//! tack release that emits SGR sample text on the ACS screen.
//!
//! The two `ScenarioSpec` consts have DIFFERENT `screen_id`s
//! (`tack_acs_graphic_chars` vs `tack_graphic_rendition_sgr`) so
//! their snapshots do not collide. Both share the same captured
//! grid content against tack v1.08, but the file separation
//! preserves the plan's spec structure and lets a future runner
//! gain per-aspect coverage without restructuring.

use crate::tack_framework::parser::tokens::grid_has_token;
use crate::tack_framework::{MenuStep, ScenarioSpec, ScreenFacts};

/// Parser for the graphic-rendition screen: scans for SGR style
/// labels tack would draw (`bold`, `dim`, `underline`, `blink`,
/// `reverse`, `invis`).
///
/// Uses [`grid_has_token`] (whitespace-bounded) because `bold`,
/// `dim`, `blink` are short labels that would false-positive
/// against any English-word substring (e.g. `bold` would match
/// inside `embolden`, `dim` inside `dimmer`, `blink` inside
/// `blinking`). The tokenized helper is the M3 fix from Section
/// 04 and is mandatory.
///
/// **Empirical caveat (tack v1.08).** As of tack v1.08 the ACS
/// screen does NOT emit any of these SGR labels — the test only
/// outputs `Testing bell (bel) ... (bel) Done`. The parser
/// returns an empty `capability_labels` list against the live
/// host. This is preserved as forward-compatible infrastructure
/// for a future tack release that draws SGR sample text on the
/// graphic-rendition screen.
///
/// Style RENDERING (the actual bold pixels, the actual italic
/// slant) is the domain of Section 07's GPU goldens — this
/// parser only verifies the LABELS are present.
pub fn parse_graphic_rendition_screen(grid: &str) -> ScreenFacts {
    const SGR_LABELS: &[&str] = &["bold", "dim", "underline", "blink", "reverse", "invis"];
    let mut found = Vec::new();
    for label in SGR_LABELS {
        if grid_has_token(grid, label) {
            found.push((*label).to_string());
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

/// Tack graphic-rendition (SGR) scenario.
///
/// Same `n -> a -> n` navigation as [`super::acs::TACK_ACS_GRAPHIC_CHARS`]
/// — the two scenarios point at the same screen with different
/// parsers. Anchor strings empirically verified against tack v1.08
/// (2026-04-08) — see the module rustdoc.
pub const TACK_GRAPHIC_RENDITION_SGR: ScenarioSpec = ScenarioSpec {
    id: "tack_graphic_rendition_sgr",
    screen_id: "tack_graphic_rendition_sgr",
    menu_path: &[
        MenuStep::new(b"n", "tack/test [n] >"),
        MenuStep::new(b"a", "tack/test/acs [n] >"),
        MenuStep::new(b"n", "Done"),
    ],
    ready_anchor: "Done",
    quit_path: None,
    parser: parse_graphic_rendition_screen,
};

#[cfg(test)]
mod tests;
