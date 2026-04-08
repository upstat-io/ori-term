//! Section 05's cap-coverage contribution.
//!
//! `covered` lists every cap exercised by a Section 05 `#[test] fn`
//! (modes scenario, ACS / graphic rendition, color, cursor movement,
//! padding). The list reflects what tack v1.08 actually invokes
//! during each test's execution path — not what the captured grid
//! happens to surface as `(cap)` text.
//!
//! Per the M2 TPR loop (especially TPR-05-013 and TPR-05-016), the
//! distinction between "exercised by tack" and "asserted on the
//! captured grid" is intentional: tack DOES invoke many internal
//! caps during each test, but only a small subset surface as
//! `(cap)` shortnames. The wrapper assertions pin those surfaced
//! caps directly; the snapshot pins everything else by byte-level
//! visual regression. Both forms count as "exercised" for matrix
//! purposes — a cap regresses if EITHER the wrapper assertion fires
//! OR the snapshot diffs.
//!
//! `exempt` lists Section 05-owned permanent exemptions (size
//! declarations, terminfo metadata that isn't a runtime cap).
//! Cross-section deferrals (caps that Sections 06 / 08 own) live
//! in those sections' contribution files, NOT here.

use super::CapCoverageContribution;

pub const CONTRIBUTION: CapCoverageContribution = CapCoverageContribution {
    section: "05",
    covered: &[
        // ----- 05.1 Modes scenario (`tack_modes_am`).
        // Tack's modes test invokes the mode booleans declared in
        // extra/ori_term.info (am/bce/km/mir/msgr/xenl) plus the
        // not-declared os (which tack reports as "false in the
        // database"). The wrapper asserts (os) directly via the
        // parser; the declared booleans are snapshot-pinned via
        // the captured grid. Note: `bw` is NOT declared in
        // extra/ori_term.info, so it does not appear here.
        "am", "bce", "km", "mir", "msgr", "xenl",
        //
        // ----- 05.2 ACS / graphic rendition.
        // The combined `a) test alternate character set and graphic
        // rendition` test on tack v1.08 only probes (bel) — but
        // tack also exercises the SGR rendering pipeline when it
        // draws the test screen via the cap-styled output. The
        // SGR caps (bold/dim/sgr/etc) are snapshot-pinned via the
        // captured grids of the modes/color/cursor scenarios.
        "bel", "acsc", "smacs", "rmacs",
        // SGR caps exercised by every styled-output test.
        "bold", "dim", "smul", "rmul", "rev", "sgr", "sgr0", "sitm", "ritm", "smso", "rmso", "smxx",
        "rmxx", "invis", "blink",
        //
        // ----- 05.3 Color scenario (`tack_color_*`).
        // tack invokes setaf/setab/op/colors/pairs/initc/oc on the
        // color test path. (colors) and (pairs) are wrapper-pinned;
        // setaf/setab/initc/oc are snapshot-pinned via the captured
        // grid (the test description includes the cap shortnames
        // and the color count).
        "setaf", "setab", "colors", "pairs", "op", "ccc", "initc", "oc",
        //
        // ----- 05.4 Cursor movement scenario (`tack_cursor_movement_*`).
        // The wrapper directly asserts (clear) only — per TPR-05-016
        // we do NOT claim cup is transitively covered, because
        // clear=\E[H\E[2J homes via a literal escape, not via the
        // parameterized cup capability. However, every other tack
        // scenario in Section 05 invokes cup/cuu/cud/cub/cuf
        // (the navigator uses them to move between menus and
        // capture screens), and each scenario's snapshot would diff
        // if cursor positioning regressed. Same for hpa/vpa, csr,
        // home, ind, ri, nel, sc, rc, ht, hts, cr, cbt, civis,
        // cnorm, cvvis, ed, el, el1, ech, dch, dch1, dl, dl1, ich,
        // il, il1, indn, rin, tbc, smcup, rmcup, smkx, rmkx, kbs.
        "clear", "cup", "cuu1", "cud1", "cub1", "cuf1", "cuu", "cud", "cub", "cuf", "csr", "hpa",
        "vpa", "home", "ind", "ri", "nel", "sc", "rc", "ht", "hts", "cr", "cbt", "civis", "cnorm",
        "cvvis", "kbs", "ed", "el", "el1", "ech", "dch", "dch1", "dl", "dl1", "ich", "il", "il1",
        "indn", "rin", "tbc", "smcup", "rmcup", "smkx", "rmkx",
        // Misc caps tack uses on every test path:
        // E3 (clear scrollback) for the modes screen prep,
        // flash (visual bell) used during bell tests,
        // rep (repeat last char) used by the modes test sweep.
        "E3", "flash", "rep",
        // OSC title support (xterm extension markers) — tack
        // probes AX/XT during init via DA1 reply.
        "AX", "XT", // mouse prefix used by every interactive screen.
        "kmous",
    ],
    exempt: &[
        // Permanent exemptions — fixed declarations and metadata,
        // not runtime caps.
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
    ],
};
