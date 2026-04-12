//! Pinned classification of every key on tack's `t) tools` submenu.
//!
//! The discovery test in
//! `oriterm_core/tests/tack/tools_menu/tools_menu_inventory.rs`
//! captures the live menu via insta and asserts the discovered key
//! set matches [`TOOLS_MENU_INVENTORY`]. Drift in either direction
//! (new key in tack output without an inventory entry, or a removed
//! key) fails the test.
//!
//! Empirically verified against tack v1.08 (2026-04-08): the tools
//! menu exposes `s) ANSI status reports`, `g) ANSI SGR modes`,
//! `c) ANSI character sets`, `h) enable hex output on echo tool`,
//! `e) echo tool`, `r) reply tool`, `p) performance testing`,
//! `i) send reset and init`, `u) test ENQ/ACK handshake`,
//! `d) change debug level`, `q) quit`, `?) help`.
//!
//! Every Section 06 scenario subsection cites a key from this
//! inventory rather than inventing one — the inventory is the SSOT.
//!
//! # Drift-gate algorithm
//!
//! The integration test uses
//! [`super::menu_inventory::assert_menu_drift`] as the canonical
//! drift-gate helper, NOT a local reimplementation. Section 05's
//! `begin_testing_inventory::assert_inventory_drift` is intentionally
//! NOT migrated to the shared helper (see that module's rustdoc for
//! the stability-over-DRY rationale).

/// One row of the tools menu inventory.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct ToolsMenuKey {
    /// The literal key shown in tack's tools menu (e.g. `'s'`,
    /// `'g'`, `'q'`).
    pub key: char,
    /// Tack's label for this entry, transcribed from the captured
    /// snapshot. Carries no semantic meaning for the drift check
    /// (which is purely on `key`) but documents the menu graph for
    /// human readers.
    pub label: &'static str,
    /// How Section 06 treats this entry.
    pub status: ToolsMenuStatus,
}

/// How a tools menu key is handled by the Section 06 catalog.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum ToolsMenuStatus {
    /// Has a corresponding `ScenarioSpec` in
    /// `tack_framework::scenarios::*` (or will, once the relevant
    /// 06.x subsection lands).
    Scenario,

    /// Covered by a different section (e.g. `p) performance testing`
    /// overlaps with Section 05's begin-testing `p) test padding`
    /// coverage; `i) send reset and init` overlaps with Section 05's
    /// begin-testing `i)` stub).
    DelegatedToSection {
        /// The section that owns the coverage (e.g. `"05"`).
        section: &'static str,
    },

    /// Cannot be automated end-to-end via tack — interactive screens
    /// that block waiting for the user to type things (echo tool,
    /// reply tool, hex-output toggle, debug-level toggle). MUST have
    /// a doc-only stub in `oriterm_core/tests/tack/tools_menu/`.
    ExcludedInteractive {
        /// The doc-only stub file (relative to
        /// `tests/tack/tools_menu/`) that explains why the screen is
        /// excluded.
        stub_file: &'static str,
    },

    /// Menu meta-key — not a tool, but reachable from the prompt
    /// (`q) quit`, `?) help`). The drift gate REQUIRES these be
    /// classified; they do NOT get a `ScenarioSpec` or stub file.
    MenuMeta,
}

/// The pinned inventory of tack v1.08's tools submenu.
///
/// **Ordering** mirrors the captured menu (top to bottom in the
/// snapshot at
/// `oriterm_core/tests/tack/tools_menu/snapshots/tack__tools_menu__tools_menu_inventory__tack_tools_menu_80x24.snap`)
/// so the table reads in the same visual order as the snapshot.
/// Drift detection is set-based
/// ([`super::menu_inventory::assert_menu_drift`]), so order is not
/// load-bearing for correctness — it is load-bearing for
/// readability.
pub const TOOLS_MENU_INVENTORY: &[ToolsMenuKey] = &[
    ToolsMenuKey {
        key: 's',
        label: "ANSI status reports",
        status: ToolsMenuStatus::Scenario,
    },
    ToolsMenuKey {
        key: 'g',
        label: "ANSI SGR modes (bold, underline, reverse)",
        status: ToolsMenuStatus::Scenario,
    },
    ToolsMenuKey {
        key: 'c',
        label: "ANSI character sets",
        status: ToolsMenuStatus::Scenario,
    },
    ToolsMenuKey {
        key: 'h',
        label: "enable hex output on echo tool",
        status: ToolsMenuStatus::ExcludedInteractive {
            stub_file: "hex_output.rs",
        },
    },
    ToolsMenuKey {
        key: 'e',
        label: "echo tool",
        status: ToolsMenuStatus::ExcludedInteractive {
            stub_file: "echo_tool.rs",
        },
    },
    ToolsMenuKey {
        key: 'r',
        label: "reply tool",
        status: ToolsMenuStatus::ExcludedInteractive {
            stub_file: "reply_tool.rs",
        },
    },
    ToolsMenuKey {
        key: 'p',
        label: "performance testing",
        status: ToolsMenuStatus::DelegatedToSection { section: "05" },
    },
    ToolsMenuKey {
        key: 'i',
        label: "send reset and init",
        status: ToolsMenuStatus::DelegatedToSection { section: "05" },
    },
    ToolsMenuKey {
        key: 'u',
        label: "test ENQ/ACK handshake",
        status: ToolsMenuStatus::Scenario,
    },
    ToolsMenuKey {
        key: 'd',
        label: "change debug level",
        status: ToolsMenuStatus::ExcludedInteractive {
            stub_file: "change_debug_level.rs",
        },
    },
    ToolsMenuKey {
        key: 'q',
        label: "quit",
        status: ToolsMenuStatus::MenuMeta,
    },
    ToolsMenuKey {
        key: '?',
        label: "help",
        status: ToolsMenuStatus::MenuMeta,
    },
];

#[cfg(test)]
mod tests;
