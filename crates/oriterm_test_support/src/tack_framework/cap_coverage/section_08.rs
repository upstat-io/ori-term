//! Section 08's cap-coverage contribution.
//!
//! Section 08 (keyboard / function key tests) has not landed yet.
//! `covered` is empty until Section 08 lands; the `kf1..=kf63`
//! family and the modified arrow / Home / End / editing key
//! family are exempted via the iterator-built helpers in
//! `cap_coverage::exempt_caps()` so this file does not have to
//! hand-write 100+ rows.

use super::CapCoverageContribution;

pub const CONTRIBUTION: CapCoverageContribution = CapCoverageContribution {
    section: "08",
    covered: &[
        // EMPTY until Section 08 lands.
    ],
    exempt: &[
        // Cursor + editing keys — deferred to Section 08
        // terminfo_xcheck. (kf1-kf63 + modified-key family are
        // exempted via the iterator-built expansion in
        // `cap_coverage::exempt_caps()` — see expand_kf_caps and
        // expand_modified_key_caps.)
        (
            "kcub1",
            "deferred to Section 08 keyboard terminfo_xcheck (cursor keys)",
        ),
        ("kcud1", "deferred to Section 08 keyboard terminfo_xcheck"),
        ("kcuf1", "deferred to Section 08 keyboard terminfo_xcheck"),
        ("kcuu1", "deferred to Section 08 keyboard terminfo_xcheck"),
        ("khome", "deferred to Section 08 keyboard terminfo_xcheck"),
        ("kend", "deferred to Section 08 keyboard terminfo_xcheck"),
        (
            "kpp",
            "deferred to Section 08 keyboard terminfo_xcheck (PageUp)",
        ),
        (
            "knp",
            "deferred to Section 08 keyboard terminfo_xcheck (PageDn)",
        ),
        (
            "kdch1",
            "deferred to Section 08 keyboard terminfo_xcheck (Delete)",
        ),
        (
            "kich1",
            "deferred to Section 08 keyboard terminfo_xcheck (Insert)",
        ),
        // ----- TPR-05-029 fix: kbs and kmous belong to Section 08's
        // keyboard family per Section 08's frontmatter and 08.4
        // editing/navigation key scenario. Section 05's tests
        // exercise them indirectly (tack reads input) but do not
        // surface them as (cap) shortnames. Section 08 will pin
        // both via terminfo_xcheck when it lands.
        (
            "kbs",
            "deferred to Section 08 keyboard terminfo_xcheck (Backspace) — Section 08.4 editing-key scenario",
        ),
        (
            "kmous",
            "deferred to Section 08 keyboard terminfo_xcheck (mouse prefix \\E[M) — Section 08 owns mouse-input encoding",
        ),
    ],
};
