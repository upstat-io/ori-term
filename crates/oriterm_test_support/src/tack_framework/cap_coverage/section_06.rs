//! Section 06's cap-coverage contribution.
//!
//! Section 06 (tools menu) has not landed yet. This file lands
//! with `covered: &[]` and a populated `exempt` list containing
//! every cap that Section 06 will eventually own. When Section 06
//! lands a tools-menu scenario for, e.g., `u6`/`u7`/`u8`/`u9`,
//! the implementer MUST move those caps from `exempt` to
//! `covered`. The stale-exemption negative pin in
//! `oriterm_core/tests/tack/test_menu/cap_coverage_matrix.rs`
//! fires if a cap is in both lists.

use super::CapCoverageContribution;

pub const CONTRIBUTION: CapCoverageContribution = CapCoverageContribution {
    section: "06",
    covered: &[
        // EMPTY until Section 06 lands. Section 06's completion
        // checklist will populate this.
    ],
    exempt: &[
        // ----- Status-report u-cap family — covered by Section 06
        // status_reports scenarios (DA / DSR / ENQ-ACK).
        (
            "u6",
            "deferred to Section 06 status_reports DSR/DA scenario",
        ),
        (
            "u7",
            "deferred to Section 06 status_reports DSR/DA scenario",
        ),
        (
            "u8",
            "deferred to Section 06 status_reports DSR/DA scenario",
        ),
        (
            "u9",
            "deferred to Section 06 status_reports + ENQ/ACK scenario",
        ),
        // ----- OSC color/cursor/clipboard caps — covered by Section
        // 06 osc_queries scenario.
        (
            "Cr",
            "deferred to Section 06 osc_queries scenario (OSC 112 cursor reset)",
        ),
        (
            "Cs",
            "deferred to Section 06 osc_queries scenario (OSC 12 cursor color)",
        ),
        (
            "Ms",
            "deferred to Section 06 osc_queries scenario (OSC 52 clipboard)",
        ),
        // ----- SGR extensions and synchronized output — covered
        // by Section 06 sgr_modes scenario.
        (
            "Smulx",
            "deferred to Section 06 sgr_modes scenario (kitty colon underline style)",
        ),
        (
            "Setulc",
            "deferred to Section 06 sgr_modes scenario (underline color)",
        ),
        (
            "Sync",
            "deferred to Section 06 sgr_modes scenario (mode 2026 synchronized output)",
        ),
        // ----- Bracketed paste — deferred to Section 06 sgr_modes
        // scenario or its sibling.
        (
            "BD",
            "deferred to Section 06 sgr_modes / paste scenario (bracketed paste off)",
        ),
        (
            "BE",
            "deferred to Section 06 sgr_modes / paste scenario (bracketed paste on)",
        ),
        (
            "PS",
            "deferred to Section 06 sgr_modes / paste scenario (paste start marker)",
        ),
        (
            "PE",
            "deferred to Section 06 sgr_modes / paste scenario (paste end marker)",
        ),
        // ----- Status line — deferred to Section 06 osc_queries
        // scenario.
        (
            "hs",
            "deferred to Section 06 osc_queries scenario (status line bool)",
        ),
        (
            "dsl",
            "deferred to Section 06 osc_queries scenario (disable status line)",
        ),
        (
            "fsl",
            "deferred to Section 06 osc_queries scenario (finish status line)",
        ),
        (
            "tsl",
            "deferred to Section 06 osc_queries scenario (to status line)",
        ),
        // ----- DECSCUSR cursor style — deferred to Section 06
        // sgr_modes / cursor scenario.
        (
            "Se",
            "deferred to Section 06 sgr_modes / cursor scenario (DECSCUSR reset)",
        ),
        (
            "Ss",
            "deferred to Section 06 sgr_modes / cursor scenario (DECSCUSR set)",
        ),
        // ----- Focus reporting — deferred to Section 06
        // osc_queries / focus scenario.
        (
            "XF",
            "deferred to Section 06 osc_queries scenario (focus event support bool)",
        ),
        (
            "kxIN",
            "deferred to Section 06 osc_queries scenario (focus-in marker)",
        ),
        (
            "kxOUT",
            "deferred to Section 06 osc_queries scenario (focus-out marker)",
        ),
        // ----- Truecolor / RGB advertisement — deferred to Section
        // 06 sgr_modes scenario.
        (
            "Tc",
            "deferred to Section 06 sgr_modes scenario (truecolor support bool)",
        ),
        (
            "RGB",
            "deferred to Section 06 sgr_modes scenario (direct-color marker)",
        ),
        // ----- OSC title support markers (TPR-05-029 fix). Both
        // are exercised via OSC 0/1/2 title-set in
        // oriterm_core/src/term/handler/osc.rs:22 (osc_set_title)
        // which Section 06's tools-menu work will pin via OSC
        // round-trip scenarios. Section 05's modes/color/cursor
        // scenarios do not exercise OSC title-set paths.
        (
            "AX",
            "deferred to Section 06 osc_queries scenario (xterm BCE behavior advertisement, paired with title support)",
        ),
        (
            "XT",
            "deferred to Section 06 osc_queries scenario (xterm extension marker, paired with title support)",
        ),
    ],
};
