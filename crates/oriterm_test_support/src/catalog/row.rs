//! Catalog row schema — the 10-column markdown table definition.

use core::fmt;

/// The number of columns in the catalog row schema.
pub const CATALOG_COLUMN_COUNT: usize = 10;

/// The canonical column names in order. Every catalog file's header
/// row MUST match this sequence exactly (case-sensitive, trimmed).
pub const CATALOG_COLUMNS: [&str; CATALOG_COLUMN_COUNT] = [
    "ID",
    "Spec source",
    "Sequence",
    "Description",
    "Implementation",
    "Apex layer",
    "Test chain",
    "Verification",
    "De-facto ref",
    "Notes",
];

/// One parsed catalog row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
    pub id: String,
    pub spec_source: String,
    pub sequence: String,
    pub description: String,
    pub implementation: String,
    pub apex_layer: String,
    pub test_chain: String,
    pub verification: Verification,
    pub de_facto_ref: String,
    pub notes: String,

    /// Source file path the row was parsed from (for diagnostics).
    pub source_file: String,
    /// 1-based line number in the source file.
    pub line_number: usize,
}

/// Verification status — constrained enumeration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Verification {
    Missing,
    Stub,
    ImplementedUnverified,
    VerifiedPartial,
    Verified,
    VerifiedWithDeviation,
}

impl Verification {
    /// Returns true for statuses that `--bootstrap-mode` forbids
    /// (Section 01 never bootstraps verified rows — those statuses
    /// are earned by the verification chain harness in Sections
    /// 04–20). See `plans/spec-conformance/section-01-catalog-
    /// bootstrap.md §01.1.g` "NEGATIVE CRITERION".
    #[must_use]
    pub const fn is_bootstrap_forbidden(self) -> bool {
        matches!(
            self,
            Self::VerifiedPartial | Self::Verified | Self::VerifiedWithDeviation
        )
    }

    /// Parse a raw cell value. Accepts the canonical forms listed in
    /// `00-overview.md §Catalog Row Schema`. Unknown values return
    /// `None` — the parser treats that as a schema violation.
    #[must_use]
    pub fn from_cell(raw: &str) -> Option<Self> {
        match raw.trim() {
            "missing" => Some(Self::Missing),
            "stub" => Some(Self::Stub),
            "implemented-unverified" => Some(Self::ImplementedUnverified),
            "verified-partial" => Some(Self::VerifiedPartial),
            "verified" => Some(Self::Verified),
            "verified-with-deviation" => Some(Self::VerifiedWithDeviation),
            _ => None,
        }
    }
}

impl fmt::Display for Verification {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::Missing => "missing",
            Self::Stub => "stub",
            Self::ImplementedUnverified => "implemented-unverified",
            Self::VerifiedPartial => "verified-partial",
            Self::Verified => "verified",
            Self::VerifiedWithDeviation => "verified-with-deviation",
        };
        f.write_str(s)
    }
}
