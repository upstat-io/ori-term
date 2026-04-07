use portable_pty::ExitStatus;

use crate::session::PtySession;

use super::parser::ScreenParserFn;

/// A single navigation step: send these bytes, then wait until the
/// PTY grid contains the primary anchor string (or one of the
/// alternates).
///
/// `wait_for` is the deterministic synchronization primitive — it
/// replaces fixed sleeps that race in CI. The anchor is a literal
/// substring expected in the grid AFTER tack processes `send`.
///
/// **Pre-existing-anchor rule.** The anchor MUST NOT already be
/// present in the grid BEFORE the send. The navigator checks this
/// (see `TackNavigator::navigate` in 04.2) and panics if it is.
/// Picking an anchor that's already on the prior screen makes
/// `wait_for` return immediately and the next keystroke goes to the
/// wrong state — pick a SUBMENU-specific string (a sub-menu header,
/// a key prompt unique to the destination screen) instead.
///
/// `or_wait_for` is the alternate-anchor extension point: real tack
/// flows hit pagers ("press any key"), `--more--` prompts, and
/// alternate sub-menu wording across distros. Listing alternates
/// here lets one `MenuStep` handle either case without branching in
/// the navigator.
///
/// Example:
///
/// ```ignore
/// MenuStep::new(b"m", "tack [m] >");
/// ```
///
/// — sends `m` (the change-modes choice) and waits until the
/// modes-submenu prompt `tack [m] >` appears. NOTE: do NOT use
/// `"modes"` as the anchor here — the word "modes" appears on the
/// main menu's `m) change modes` line and the pre-existing-anchor
/// guard will reject it. Use the sub-menu PROMPT, not a word that is
/// already on the main menu.
#[derive(Copy, Clone, Debug)]
pub struct MenuStep {
    /// Bytes to write to the PTY.
    pub send: &'static [u8],
    /// Primary anchor — must appear in the grid AFTER `send` lands
    /// AND must NOT already be present BEFORE `send` (pre-existing-
    /// anchor guard, enforced by `TackNavigator::navigate`).
    pub wait_for: &'static str,
    /// Alternate anchors. Empty by default. The navigator builds a
    /// combined `[wait_for, ...or_wait_for]` slice and calls
    /// `PtySession::wait_for_any` so all anchors race against the
    /// same step deadline.
    pub or_wait_for: &'static [&'static str],
}

impl MenuStep {
    /// Convenience constructor with no alternate anchors.
    #[must_use]
    pub const fn new(send: &'static [u8], wait_for: &'static str) -> Self {
        Self {
            send,
            wait_for,
            or_wait_for: &[],
        }
    }
}

/// Static description of a single tack scenario.
///
/// Constructible as `const` so test catalogs can list scenarios in
/// arrays. The whole spec is data — no closures, no I/O — until the
/// `parser` and (optional) `quit_path` function pointers are invoked
/// by `ScenarioRunner`.
#[derive(Copy, Clone, Debug)]
pub struct ScenarioSpec {
    /// Semantic ID, e.g. `"tack_modes_am"`. Used as the
    /// `scenario_id` field of `ScenarioOutcome` (NOT directly as the
    /// snapshot name — the snapshot name is built from
    /// `screen_id`+`cols`x`rows` so size-matrix runs share goldens
    /// when navigation produces the same screen).
    ///
    /// Convention: `tack_<menu>_<screen>_<assertion>` lowercase
    /// `snake_case`.
    pub id: &'static str,

    /// Screen identity for snapshot/golden deduplication. Multiple
    /// scenarios that visit the SAME tack screen share the same
    /// `screen_id` so they snapshot once. Convention:
    /// `tack_<menu>_<screen>` (e.g., `"tack_modes"` for every modes
    /// scenario regardless of which cap it asserts).
    pub screen_id: &'static str,

    /// Sequence of navigation steps from tack's main menu to the
    /// target screen. Each step sends one or more bytes and waits
    /// for an anchor string to appear in the grid.
    ///
    /// Example for the modes screen (`n` -> `m`). Note both anchors
    /// are SUB-menu prompts unique to their destination — neither is
    /// a substring of the prior screen.
    ///
    /// ```ignore
    /// &[
    ///   MenuStep::new(b"n", "tack [n] >"),
    ///   MenuStep::new(b"m", "tack [m] >"),
    /// ]
    /// ```
    pub menu_path: &'static [MenuStep],

    /// Final readiness anchor. After the last `MenuStep` lands, the
    /// runner calls `session.wait_for(ready_anchor, ...)` once more
    /// to make sure the screen has fully painted before `grid_text`
    /// is captured.
    ///
    /// Same pre-existing-anchor rule as [`MenuStep::wait_for`]: the
    /// anchor must be SCREEN-specific, not a word that's already on
    /// the prior menu.
    pub ready_anchor: &'static str,

    /// Per-scenario quit override. `None` means use the canonical
    /// `PtySession::quit_tack(5)` introduced in 04.0.b.4. A scenario
    /// that needs a different escape path (e.g., a sub-menu that
    /// only exits on `\x1b`, or a screen that needs a single `q`
    /// without nesting) provides a custom function pointer.
    pub quit_path: Option<fn(&mut PtySession) -> ExitStatus>,

    /// Per-scenario screen parser. Takes the captured `grid_text`
    /// and extracts structured facts (which capability labels are
    /// present, what the cursor reports look like, etc.). The
    /// returned [`super::parser::ScreenFacts`] is asserted by the
    /// test.
    pub parser: ScreenParserFn,
}

impl ScenarioSpec {
    /// Convenience constructor for tests that just snapshot and
    /// don't need a custom parser.
    #[must_use]
    pub const fn snapshot_only(
        id: &'static str,
        screen_id: &'static str,
        menu_path: &'static [MenuStep],
        ready_anchor: &'static str,
    ) -> Self {
        Self {
            id,
            screen_id,
            menu_path,
            ready_anchor,
            quit_path: None,
            parser: super::parser::default_parser,
        }
    }
}
