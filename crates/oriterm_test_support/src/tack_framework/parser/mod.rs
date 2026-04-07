//! Per-scenario screen parsers and shared tokenized grid helpers.
//!
//! `ScreenFacts` is the typed-extraction container; per-scenario
//! parsers fill the fields they care about. The default parser only
//! extracts the screen header. The `tokens` submodule (added in
//! 04.1.b) contains whitespace-bounded match helpers
//! (`grid_has_token`, etc.) that every per-scenario parser MUST use
//! instead of blind `grid.contains(short_label)` checks — see the M3
//! fix in the section-04 plan for why short-substring `contains` is
//! unsafe (e.g., `"am"` matches inside `"name"`).

pub mod tokens;

/// Structured facts extracted from a tack screen by a per-scenario
/// parser. The default parser populates only `header_text`; custom
/// parsers populate the typed fields they care about.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ScreenFacts {
    /// First non-blank line of the captured grid — the screen header.
    /// e.g. `"modes"`, `"ACS graphic rendition"`, `"color"`.
    pub header_text: String,

    /// Capability labels found on the screen (for modes/glitches and
    /// SGR test screens that show literal cap names like `am`, `bce`,
    /// `bw`).
    pub capability_labels: Vec<String>,

    /// Free-form notes the parser wants to record. Snapshotted as
    /// part of the outcome but not asserted automatically.
    pub notes: Vec<String>,
}

/// Function pointer type for per-scenario screen parsers.
///
/// Function pointer (not closure) so [`super::spec::ScenarioSpec`]
/// can be `Copy` and `const`-constructible.
pub type ScreenParserFn = fn(&str) -> ScreenFacts;

/// Default parser: extracts the first non-blank line as `header_text`
/// and leaves all other fields empty. Suitable for snapshot-only
/// scenarios that don't need typed assertions.
#[must_use]
pub fn default_parser(grid: &str) -> ScreenFacts {
    let header = grid
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .unwrap_or("")
        .to_string();
    ScreenFacts {
        header_text: header,
        ..ScreenFacts::default()
    }
}

#[cfg(test)]
mod tests;
