//! Section 05's cap-coverage contribution.
//!
//! `covered` lists ONLY the caps that a Section 05 `#[test] fn`
//! wrapper directly asserts via `outcome.grid_text.contains(...)`.
//! Per (Codex review-work
//! findings), the looser interpretation — "any cap tack might
//! invoke during a Section 05 test path" — was REJECTED:
//! claiming caps as covered without an assertion that catches a
//! per-cap regression makes the cap-coverage matrix hide real
//! gaps. Snapshot-pinning is a form of regression-catching but
//! it does not provide per-cap traceability the way wrapper
//! assertions do.
//!
//! The honest covered list is therefore tiny: only the 4 caps
//! whose `(cap)` parenthesized form (or full name) appears in
//! a wrapper assertion. Every other cap that tack invokes
//! during a Section 05 test path lives in `exempt` with a
//! reason explaining where the honest coverage will eventually
//! come from (Section 07 GPU goldens, vttest, or a future tack
//! release).
//!
//! Wrapper-asserted caps (the only entries in `covered`):
//! - `bel` — `(bel)` asserted by `tack_acs_graphic_chars` and
//!   `tack_graphic_rendition_sgr` (same screen on tack v1.08).
//! - `colors` — `(colors)` asserted by all 3 `tack_color_*` size
//!   wrappers.
//! - `pairs` — `(pairs)` asserted by all 3 `tack_color_*` size
//!   wrappers.
//! - `clear` — `(clear)` asserted by all 3 `tack_cursor_movement_*`
//!   size wrappers.
//!
//! Note: `os` is asserted by `tack_modes_am` (via the parser's
//! `KNOWN: &["os"]`), but `os` is NOT declared in
//! `extra/ori_term.info` so it does not appear in any
//! contribution slice — the matrix only checks declared caps.
//!
//! Note: `rs1` is asserted by `tack_padding`, but `rs1` is also
//! NOT declared in `extra/ori_term.info` (the test reports it
//! as "not present"). Same exclusion.

use super::CapCoverageContribution;

pub const CONTRIBUTION: CapCoverageContribution = CapCoverageContribution {
    section: "05",
    covered: &[
        // Wrapper-asserted caps. See module rustdoc for the
        // exact assertion sites.
        "bel", "colors", "pairs", "clear",
    ],
    exempt: &[
        // ----- Permanent exemptions: fixed declarations and
        // metadata, not runtime caps.
        ("cols", "fixed dimension declaration, not a runtime cap"),
        ("lines", "fixed dimension declaration, not a runtime cap"),
        ("it", "tab width declaration, not a runtime cap"),
        // setb/setf — legacy 8-color set commands, cancelled in
        // ori_term and ori_term-direct via setb@ / setf@. The
        // cancellation declares the NAME but no value; tack does
        // not exercise them.
        (
            "setb",
            "cancelled (legacy 8-color setb@ in ori_term/ori_term-direct entries)",
        ),
        (
            "setf",
            "cancelled (legacy 8-color setf@ in ori_term/ori_term-direct entries)",
        ),
        // ----- Mode booleans (05.1 modes scenario). Tack invokes
        // all 6 declared mode booleans internally during the modes
        // test path but only emits `(os)` to the captured grid (per
        // the 05.1 empirical finding). The 7 per-cap phase scenarios
        // are #[ignore]'d. Honest per-cap coverage requires a future
        // tack release that emits per-cap labels OR a vttest cross-
        // check.
        (
            "am",
            "tack modes test invokes internally but does not surface as (cap) shortname; per 05.1 empirical finding tack v1.08 only emits (os) on the modes screen — see scenarios/modes/mod.rs rustdoc",
        ),
        (
            "bce",
            "tack modes test invokes internally but does not surface as (cap) shortname (05.1 empirical finding)",
        ),
        (
            "km",
            "tack modes test invokes internally but does not surface as (cap) shortname (05.1 empirical finding)",
        ),
        (
            "mir",
            "tack modes test invokes internally but does not surface as (cap) shortname (05.1 empirical finding)",
        ),
        (
            "msgr",
            "tack modes test invokes internally but does not surface as (cap) shortname (05.1 empirical finding)",
        ),
        (
            "xenl",
            "tack modes test invokes internally but does not surface as (cap) shortname (05.1 empirical finding)",
        ),
        // ----- ACS / character set caps (05.2). Per the 05.2
        // empirical finding, tack v1.08's combined ACS+graphic-
        // rendition test only probes (bel) — no DEC line-drawing
        // chars or SGR sample text appears on the captured grid.
        // Honest coverage for `acsc` requires vttest menu3 (which
        // already pins line-drawing chars at oriterm_core/tests/
        // vttest/menu3.rs) or Section 07 GPU goldens.
        (
            "acsc",
            "tack v1.08 ACS test does not draw DEC line-drawing chars (05.2 empirical finding); honest coverage in oriterm_core/tests/vttest/menu3.rs (assert_has_line_drawing_chars)",
        ),
        (
            "smacs",
            "alt-charset enter — exercised when tack switches to ACS but not surfaced as (cap) shortname; honest coverage requires Section 07 GPU goldens",
        ),
        ("rmacs", "alt-charset exit — same as smacs"),
        // ----- SGR family. Per the 05.2 empirical finding, tack
        // v1.08 does not emit visible SGR sample text on the
        // graphic-rendition screen. Honest coverage requires
        // Section 07 GPU goldens (for actual bold/italic/underline
        // pixel rendering) or vttest menu5 (which exercises SGR
        // codes via direct ESC sequences).
        (
            "bold",
            "tack v1.08 graphic-rendition test does not emit visible SGR labels (05.2 empirical finding); honest coverage requires Section 07 GPU goldens or vttest menu5",
        ),
        ("dim", "see bold"),
        ("smul", "see bold"),
        ("rmul", "see bold"),
        ("rev", "see bold"),
        ("sgr", "see bold"),
        ("sgr0", "see bold"),
        ("sitm", "see bold"),
        ("ritm", "see bold"),
        ("smso", "see bold"),
        ("rmso", "see bold"),
        ("smxx", "see bold (strikethrough)"),
        ("rmxx", "see bold"),
        ("invis", "see bold"),
        ("blink", "see bold"),
        // ----- Color caps beyond colors/pairs (05.3). Per the
        // 05.3 empirical finding, tack v1.08's color test only
        // surfaces (colors) and (pairs) on the captured grid —
        // setaf/setab/initc/oc are exercised internally but never
        // appear as `(cap)` shortnames. Honest coverage for the
        // actual setaf/setab pixel rendering requires Section 07
        // GPU goldens (color/palette tests).
        (
            "setaf",
            "tack v1.08 color test only emits (colors)/(pairs) (05.3 empirical finding); honest coverage requires Section 07 GPU goldens",
        ),
        ("setab", "see setaf"),
        ("op", "see setaf"),
        (
            "ccc",
            "ccc declaration is exercised by initc; not surfaced as (cap) shortname",
        ),
        ("initc", "see setaf"),
        ("oc", "OSC 104 reset palette — see setaf"),
        // ----- Cursor / movement caps beyond `clear` (05.4).
        // Per the wrapper only directly asserts (clear);
        // cup is exercised by tack's menu navigation but the
        // observable home behavior comes from clear=\E[H\E[2J's
        // literal escape, NOT from the parameterized cup capability.
        // Honest per-cap coverage requires Section 07 GPU goldens
        // or vttest cursor-movement tests.
        (
            "cup",
            "wrapper does not directly assert; clear=\\E[H\\E[2J explains home behavior; honest coverage requires Section 07 GPU goldens or vttest",
        ),
        ("cuu1", "see cup"),
        ("cud1", "see cup"),
        ("cub1", "see cup"),
        ("cuf1", "see cup"),
        ("cuu", "see cup"),
        ("cud", "see cup"),
        ("cub", "see cup"),
        ("cuf", "see cup"),
        ("csr", "see cup (scroll region)"),
        ("hpa", "see cup (horizontal position absolute)"),
        ("vpa", "see cup (vertical position absolute)"),
        ("home", "see cup"),
        (
            "ind",
            "scroll forward — exercised internally, not surfaced as (cap) shortname (see cup)",
        ),
        ("ri", "reverse index — see ind"),
        ("nel", "next line — see ind"),
        ("sc", "save cursor — see ind"),
        ("rc", "restore cursor — see ind"),
        (
            "ht",
            "tab — exercised by tack output but not surfaced as (cap) shortname (see cup)",
        ),
        ("hts", "tab set — see ht"),
        ("cr", "carriage return — see ht"),
        ("cbt", "back tab — see ht"),
        (
            "civis",
            "cursor invisible — exercised when tack hides cursor but not surfaced as (cap) shortname",
        ),
        ("cnorm", "cursor normal — see civis"),
        ("cvvis", "cursor very visible — see civis"),
        // ----- Erase / insert / delete caps. Same rationale as
        // cursor: tack invokes these internally but does not
        // surface them as (cap) shortnames on any captured grid.
        (
            "ed",
            "erase display — exercised by tack screen clears (e.g. clear=\\E[H\\E[2J) but not surfaced as (cap) shortname",
        ),
        ("el", "erase line — see ed"),
        ("el1", "erase to start of line — see ed"),
        ("ech", "erase chars — see ed"),
        ("dch", "delete chars — see ed"),
        ("dch1", "see dch"),
        ("dl", "delete lines — see ed"),
        ("dl1", "see dl"),
        ("ich", "insert chars — see ed"),
        ("il", "insert lines — see ed"),
        ("il1", "see il"),
        ("indn", "scroll forward N — see ed"),
        ("rin", "scroll back N — see ed"),
        ("tbc", "tab clear — see ed"),
        // ----- Alt-screen / keypad caps. Tack uses alt-screen via
        // 1049 when entering its main menu and exits on quit.
        // smkx/rmkx toggle keypad mode. None are surfaced as (cap)
        // shortnames.
        (
            "smcup",
            "tack enters alt-screen via 1049 but does not surface (smcup) as a (cap) shortname",
        ),
        ("rmcup", "see smcup"),
        (
            "smkx",
            "tack enters keypad mode but does not surface (smkx) as a (cap) shortname",
        ),
        ("rmkx", "see smkx"),
        // ----- Misc test-path caps that tack uses internally.
        (
            "E3",
            "clear scrollback — exercised by tack but not surfaced as (cap) shortname",
        ),
        (
            "flash",
            "visual bell — exercised when tack tests flash but not surfaced as (cap) shortname",
        ),
        (
            "rep",
            "repeat last char — exercised by tack output but not surfaced as (cap) shortname",
        ),
    ],
};
