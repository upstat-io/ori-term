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
    ],
};
