//! Shared drift-gate algorithm for Section 06's menu inventories.
//!
//! Consumed by [`super::tools_menu_inventory`] (06.0) and
//! [`super::status_reports_inventory`] (06.0.b). Each caller owns its
//! own PINNED `BTreeSet<char>` and builds the DISCOVERED set from a
//! captured grid; this helper is the set-compare + diagnostic-diff
//! skeleton plus the `<key>) <label>` row scanner every menu
//! inventory uses to build its discovered set.
//!
//! # Intentional non-consumer
//!
//! Section 05's [`super::begin_testing_inventory::assert_inventory_drift`]
//! implements the same skeleton against its own pinned
//! `BEGIN_TESTING_INVENTORY` const, and the Section 05 integration
//! test at `oriterm_core/tests/tack/test_menu/begin_testing_inventory.rs`
//! has a private `collect_menu_keys` scanner. Both are intentionally
//! NOT migrated to this helper during Section 06 work — refactoring a
//! green Section 05 integration test while Section 06 is actively
//! landing is needless blast radius (Codex midpoint review, Section
//! 06 Agent 3 review pass). The ~15 lines of duplication between
//! Section 05's helpers and this module are accepted under the
//! stability-over-DRY rule. If a future section introduces a fourth
//! drift-gate consumer, that work may consolidate the two helpers at
//! that point. Until then, Section 05 keeps its own.

use std::collections::BTreeSet;

/// Compare a discovered menu-key set against a pinned inventory set
/// and return `Err(diff_message)` on mismatch.
///
/// `source_label` names the inventory in the diagnostic message
/// (e.g. `"tools menu"`, `"status reports sub-submenu"`) so a failure
/// panic points directly at the inventory that drifted instead of
/// forcing the reader to guess from the stack trace.
///
/// # Errors
///
/// Returns `Err` whenever `discovered` and `pinned` disagree on any
/// key. The error message includes both sets and a symmetric
/// difference (`only_in_discovered` + `only_in_pinned`) so the
/// caller can show the diff alongside whatever extra context (grid,
/// snapshot path) it has.
pub fn assert_menu_drift(
    discovered: &BTreeSet<char>,
    pinned: &BTreeSet<char>,
    source_label: &str,
) -> Result<(), String> {
    if discovered == pinned {
        return Ok(());
    }
    let only_in_discovered: BTreeSet<&char> = discovered.difference(pinned).collect();
    let only_in_pinned: BTreeSet<&char> = pinned.difference(discovered).collect();
    Err(format!(
        "{source_label} drift detected.\n\
         Discovered: {discovered:?}\n\
         Pinned:     {pinned:?}\n\
         Only in discovered (new keys, add to inventory): {only_in_discovered:?}\n\
         Only in pinned (removed keys, drop from inventory): {only_in_pinned:?}"
    ))
}

/// Scan a captured grid for `<key>) <label>` menu entries and return
/// the discovered key set.
///
/// Tack's menu format is fixed: `<spaces><key>) <label>` per row. We
/// trim leading whitespace, take the first character, and accept it
/// only when followed by `)` and not whitespace itself. Other formats
/// (table headers, prompt lines, blank rows) are ignored.
///
/// **Case-sensitive on purpose.** Tack's tools submenu does not use
/// case-paired keys the way the begin-testing submenu does
/// (`p)` test padding vs `P)` test printer), but future tack releases
/// might. Lowercasing here would silently collapse any future
/// case-paired entries; keeping the scanner case-sensitive lets the
/// drift gate surface the change instead of hiding it.
///
/// **Punctuation keys accepted on purpose.** Tack uses `?) help` and
/// (in the begin-testing submenu) `/) test a specific capability`.
/// Both are real menu keys with the same `<key>)` syntax. Restricting
/// to ASCII-alphabetic would drop them from the discovered set and
/// silently hide drift in either entry.
///
/// **Digit keys accepted on purpose.** Not currently used by tack
/// v1.08's tools submenu, but the character-sets tool's bank prompt
/// uses digit keys (`0..9`) for private-use sets. If that prompt is
/// ever inventoried, the scanner already handles it.
#[must_use]
pub fn collect_menu_keys(grid: &str) -> BTreeSet<char> {
    grid.lines()
        .filter_map(|line| {
            let trimmed = line.trim_start();
            let mut chars = trimmed.chars();
            let key = chars.next()?;
            if chars.next() == Some(')')
                && (key.is_ascii_alphabetic() || key.is_ascii_digit() || "/?".contains(key))
            {
                Some(key)
            } else {
                None
            }
        })
        .collect()
}

#[cfg(test)]
mod tests;
