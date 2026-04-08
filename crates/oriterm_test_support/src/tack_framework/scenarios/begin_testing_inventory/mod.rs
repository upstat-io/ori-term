//! Pinned classification of every key on tack's begin-testing
//! submenu.
//!
//! The discovery test in
//! `oriterm_core/tests/tack/test_menu/begin_testing_inventory.rs`
//! captures the live menu via insta and asserts the discovered key
//! set matches [`BEGIN_TESTING_INVENTORY`]. Drift in either direction
//! (new key in tack output without an inventory entry, or a removed
//! key) fails the test.
//!
//! ## Why this is the SSOT for Section 05
//!
//! Every other Section 05 subsection (05.1 modes phase capture,
//! 05.2 ACS / graphic rendition, 05.3 color, 05.4 cursor movement,
//! 05.4b remaining navigable screens) cites a key from this table
//! instead of inventing one. Earlier drafts of Section 05 guessed
//! keys from a tack v6.x manual that does not match the pinned tack
//! v1.08 (`extra/ori_term.info` is compiled against ncurses 6.x +
//! tack v1.08); the guesses included `a/c/u/p/l/k/e/f/o/s/b` and
//! none of them were correct. The discovery + drift gate is the
//! forcing function that prevents that class of error from
//! recurring.
//!
//! ## Algorithmic-DRY drift gate
//!
//! [`assert_inventory_drift`] is the canonical drift-gate algorithm.
//! Both the integration test in `oriterm_core/tests/tack/` and the
//! sibling unit test in [`tests`] call this helper, so a future
//! refactor that weakens the assertion to `assert!(true)` would
//! break both consumers at once instead of silently passing on the
//! call site that wasn't updated.

use std::collections::BTreeSet;

/// One row of the begin-testing menu inventory.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct BeginTestingKey {
    /// The literal key shown in tack's menu (e.g. `'x'`, `'m'`).
    /// Lowercase ASCII; the discovery scanner lowercases before
    /// comparing so the table is case-insensitive in practice.
    pub key: char,
    /// Tack's prompt or menu label for this entry, transcribed from
    /// the captured snapshot. Carries no semantic meaning for the
    /// drift check — the drift check is purely on `key` — but
    /// documents the menu graph for human readers.
    pub label: &'static str,
    /// How Section 05 / 06 / 08 treat this entry.
    pub status: BeginTestingStatus,
}

/// How a begin-testing key is handled by the test catalog.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum BeginTestingStatus {
    /// Has a corresponding `ScenarioSpec` or `PhaseSpec` (or will
    /// have once the relevant Section 05 subsection lands).
    Scenario,

    /// Covered by a different section (e.g. function keys are
    /// covered by Section 08's in-crate sibling test, not by tack).
    DelegatedToSection {
        /// The section that owns the coverage (e.g. `"08"`).
        section: &'static str,
    },

    /// Cannot be automated end-to-end via tack — interactive screens
    /// that block waiting for the user to type things (function key
    /// probe, edit terminfo). MUST have a doc-only stub in
    /// `oriterm_core/tests/tack/test_menu/`.
    ExcludedInteractive {
        /// The doc-only stub file (relative to `tests/tack/test_menu/`)
        /// that explains why the screen is excluded.
        stub_file: &'static str,
    },

    /// Overlaps with another entry — pick one, document the other.
    Duplicate {
        /// The key that owns the canonical scenario.
        covered_by: &'static str,
    },
}

/// The pinned inventory of tack's begin-testing submenu, as captured
/// against tack v1.08 / ncurses 6.x via the discovery test in
/// `oriterm_core/tests/tack/test_menu/begin_testing_inventory.rs`.
///
/// **Ordering** mirrors the captured menu (top to bottom in the
/// snapshot at
/// `oriterm_core/tests/tack/test_menu/snapshots/tack__test_menu__begin_testing_inventory__tack_begin_testing_menu_80x24.snap`)
/// so the file reads in the same visual order as the snapshot.
/// Drift detection is set-based ([`assert_inventory_drift`]),
/// so order is not load-bearing.
///
/// **Plan/reality mismatches surfaced by 05.0 discovery** (recorded
/// here so 05.4b and 05.5 can address them):
///
/// - `a) test alternate character set and graphic rendition` is a
///   COMBINED entry. The plan envisioned ACS and SGR as separate
///   submenu entries; tack v1.08 merges them. One scenario covers
///   both mission criteria items.
/// - `p) test padding and string capabilities` is a COMBINED entry
///   covering BOTH "pad timing" and "send strings" from the plan's
///   mission criterion. One scenario covers both.
/// - There is NO `l) test labels` entry in tack v1.08. The plan's
///   mission criterion mentions "labels" but the menu does not
///   expose one. 05.4b reconciles this by either dropping labels
///   from the criterion (if it never existed in tack) or by
///   verifying labels are part of `a)` / `p)` coverage.
/// - The plan also mentions an "output" interactive screen; tack
///   v1.08's begin-testing submenu has no `o)` or output entry.
///   This is recorded as a non-issue.
pub const BEGIN_TESTING_INVENTORY: &[BeginTestingKey] = &[
    BeginTestingKey {
        key: 'e',
        label: "edit terminfo",
        status: BeginTestingStatus::ExcludedInteractive {
            stub_file: "edit_terminfo.rs",
        },
    },
    BeginTestingKey {
        key: 'i',
        label: "send reset and init",
        status: BeginTestingStatus::ExcludedInteractive {
            stub_file: "send_reset_init.rs",
        },
    },
    BeginTestingKey {
        key: 'x',
        label: "test modes and glitches",
        status: BeginTestingStatus::Scenario,
    },
    BeginTestingKey {
        key: 'a',
        label: "test alternate character set and graphic rendition",
        status: BeginTestingStatus::Scenario,
    },
    BeginTestingKey {
        key: 'c',
        label: "test color",
        status: BeginTestingStatus::Scenario,
    },
    BeginTestingKey {
        key: 'm',
        label: "test cursor movement",
        status: BeginTestingStatus::Scenario,
    },
    BeginTestingKey {
        key: 'f',
        label: "test function keys",
        status: BeginTestingStatus::DelegatedToSection { section: "08" },
    },
    BeginTestingKey {
        key: 'p',
        label: "test padding and string capabilities",
        status: BeginTestingStatus::Scenario,
    },
    BeginTestingKey {
        key: 'P',
        label: "test printer",
        status: BeginTestingStatus::ExcludedInteractive {
            stub_file: "test_printer.rs",
        },
    },
    BeginTestingKey {
        key: '/',
        label: "test a specific capability",
        status: BeginTestingStatus::ExcludedInteractive {
            stub_file: "test_specific_cap.rs",
        },
    },
    BeginTestingKey {
        key: 't',
        label: "auto generate pad delays",
        status: BeginTestingStatus::ExcludedInteractive {
            stub_file: "auto_pad_delays.rs",
        },
    },
    BeginTestingKey {
        key: 'n',
        label: "run standard tests",
        status: BeginTestingStatus::Duplicate {
            covered_by: "x, a, c, m, p (run standard tests sequences each individual test)",
        },
    },
    BeginTestingKey {
        key: 'r',
        label: "repeat test",
        status: BeginTestingStatus::ExcludedInteractive {
            stub_file: "repeat_test.rs",
        },
    },
    BeginTestingKey {
        key: 's',
        label: "skip to next test",
        status: BeginTestingStatus::ExcludedInteractive {
            stub_file: "skip_to_next_test.rs",
        },
    },
    BeginTestingKey {
        key: 'q',
        label: "quit",
        status: BeginTestingStatus::ExcludedInteractive {
            stub_file: "quit.rs",
        },
    },
    BeginTestingKey {
        key: '?',
        label: "help",
        status: BeginTestingStatus::Scenario,
    },
];

/// Compare a discovered key set against [`BEGIN_TESTING_INVENTORY`]
/// and return `Err(diff_message)` on any mismatch.
///
/// **Algorithmic-DRY canonical home.** Both the integration test in
/// `oriterm_core/tests/tack/test_menu/begin_testing_inventory.rs`
/// and the sibling unit test in [`tests::begin_testing_inventory_drift_gate_pin`]
/// call this helper. There is exactly one place where the drift gate
/// algorithm lives. Adding a third consumer = re-export, never
/// re-implement.
///
/// # Errors
///
/// Returns `Err` whenever `discovered` and the pinned inventory
/// disagree on any key. The error message includes both the
/// discovered set and the pinned set so the caller can show the
/// diff alongside whatever extra context (grid, snapshot path) it
/// has.
pub fn assert_inventory_drift(discovered: &BTreeSet<char>) -> Result<(), String> {
    let pinned: BTreeSet<char> = BEGIN_TESTING_INVENTORY.iter().map(|k| k.key).collect();
    if discovered == &pinned {
        return Ok(());
    }
    let only_in_discovered: BTreeSet<&char> = discovered.difference(&pinned).collect();
    let only_in_pinned: BTreeSet<&char> = pinned.difference(discovered).collect();
    Err(format!(
        "begin-testing menu drift detected.\n\
         Discovered: {discovered:?}\n\
         Pinned:     {pinned:?}\n\
         Only in discovered (new keys, add to inventory): {only_in_discovered:?}\n\
         Only in pinned (removed keys, drop from inventory): {only_in_pinned:?}"
    ))
}

#[cfg(test)]
mod tests;
