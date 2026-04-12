//! Section 08's cap-coverage contribution.
//!
//! Section 08 (keyboard / function key tests) validates the
//! `key_encoding::encode_key` pipeline against the pinned terminfo.
//! `covered` includes the named cursor/editing keys tested directly
//! in the `terminfo_xcheck` tables. The kf1-kf63 and modified-key
//! families are added programmatically via `covered_caps_08()`.

use super::{CapCoverageContribution, expand_kf_caps, expand_modified_key_caps};

pub const CONTRIBUTION: CapCoverageContribution = CapCoverageContribution {
    section: "08",
    covered: &[
        // Cursor keys — tested in 08.3 (navigation.rs).
        "kcub1", "kcud1", "kcuf1", "kcuu1",
        // Home/End — tested in 08.4 (navigation.rs, APP_CURSOR mode).
        "khome", "kend", // Editing keys — tested in 08.4 (navigation.rs).
        "kbs", "kpp", "knp", "kdch1", "kich1",
    ],
    exempt: &[
        // kmous (\E[M) is the mouse encoding prefix — it does NOT go
        // through key_encoding::encode_key and cannot be cross-checked
        // by the keyboard terminfo_xcheck tests. Belongs to the mouse
        // input subsystem (future roadmap section 10).
        (
            "kmous",
            "mouse encoding prefix \\E[M — not testable via key_encoding::encode_key; \
             belongs to mouse input subsystem (roadmap Section 10)",
        ),
    ],
};

/// Section 08's full covered cap set including programmatic expansions.
///
/// Returns the union of the static `CONTRIBUTION.covered` entries
/// with `expand_kf_caps()` (kf1-kf63) and `expand_modified_key_caps()`
/// (kLFT/kRIT/kUP/kDN/kHOM/kEND/kIC/kDC/kNXT/kPRV with suffixes,
/// plus kind/kri). This avoids a 130+ entry static slice.
#[must_use]
pub fn covered_caps_08() -> Vec<String> {
    let mut out: Vec<String> = CONTRIBUTION
        .covered
        .iter()
        .map(|c| (*c).to_string())
        .collect();
    out.extend(expand_kf_caps());
    out.extend(expand_modified_key_caps());
    out
}
