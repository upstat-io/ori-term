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
    /// combined `[wait_for,...or_wait_for]` slice and calls
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
    /// MenuStep::new(b"n", "tack [n] >"),
    /// MenuStep::new(b"m", "tack [m] >"),
    /// ]
    /// ```
    pub menu_path: &'static [MenuStep],

    /// Final readiness anchor. After the last `MenuStep` lands, the
    /// runner calls `session.wait_for(ready_anchor,...)` once more
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

/// Static description of a single phase-capture tack scenario.
///
/// Used for tack screens where the fact of interest is visible only
/// briefly mid-run — typically when tack is sweeping through a list
/// of capabilities and printing a line per cap before moving on.
/// The stable-screen [`ScenarioSpec`] cannot capture these because
/// the 300 ms `send` quiesce + 200 ms `wait_for_*` quiesce lets the
/// line scroll off before `grid_text()` is read.
///
/// `PhaseSpec` reuses the same spawn + main-menu wait + terminfo
/// lifetime as [`ScenarioSpec`], but the navigation + capture path
/// uses [`crate::session::PtySession::send_raw`] (no built-in
/// quiesce) and polls for the `phase_anchor` with NO post-match
/// quiesce.
///
/// **Pre-existing-anchor rule.** Same as [`MenuStep::wait_for`]:
/// `phase_anchor` MUST NOT already be present in the grid before
/// the phase-trigger keystroke lands. The runner enforces this.
///
/// **NOT for stable screens.** Stable screens (color, cursor,
/// `graphic_rendition`) MUST continue to use [`ScenarioSpec`] so they
/// inherit the proven 500 ms quiesce contract that the existing
/// 198 vttest tests rely on. Phase capture is ONLY for mid-flow
/// content.
#[derive(Copy, Clone, Debug)]
pub struct PhaseSpec {
    /// Semantic ID, e.g. `"tack_modes_phase_am"`.
    pub id: &'static str,

    /// Screen identity for snapshot/golden naming. MUST be UNIQUE
    /// across phase scenarios — distinct phase captures are distinct
    /// screen states, NOT the same screen with different facts. A
    /// shared `screen_id` would cause `outcome.snapshot_name()` to
    /// produce duplicate names and silently overwrite snapshots.
    pub screen_id: &'static str,

    /// Sequence of pre-phase navigation steps. Same semantics as
    /// [`ScenarioSpec::menu_path`] — stable nav, uses
    /// [`crate::session::PtySession::send`] (with the 300 ms
    /// quiesce) so the navigator anchors land cleanly. The
    /// phase-capture loop only kicks in AFTER `menu_path` lands
    /// at `phase_setup_anchor`.
    pub menu_path: &'static [MenuStep],

    /// Anchor that confirms `menu_path` has landed and tack is at
    /// the screen JUST BEFORE the phase test runs. Same
    /// pre-existing-anchor rule as [`MenuStep::wait_for`].
    pub phase_setup_anchor: &'static str,

    /// Bytes that TRIGGER the phase test (e.g. `b"n"` to start the
    /// modes-test sweep from the modes-controls screen). Sent via
    /// [`crate::session::PtySession::send_raw`] — NO 300 ms
    /// quiesce — so the phase loop can begin polling for the phase
    /// anchor immediately.
    pub phase_trigger: &'static [u8],

    /// The phase anchor: the literal substring that signals the
    /// captured fact has appeared in the viewport. For modes-family
    /// per-cap captures this is `(am)`, `(bce)`, etc. The runner
    /// polls `grid_text()` for this anchor on the canonical
    /// bounded-poll skeleton with NO post-match quiesce, then
    /// captures.
    pub phase_anchor: &'static str,

    /// Hard timeout for the phase loop, in milliseconds. If the
    /// phase anchor does not appear within this budget, the runner
    /// panics with the captured grid for diagnostics. A value of
    /// `0` means "use `runner::phase::PHASE_DEFAULT_TIMEOUT_MS`".
    pub phase_timeout_ms: u64,

    /// Per-scenario quit override (same semantics as
    /// [`ScenarioSpec::quit_path`]). For modes-family scenarios
    /// that trigger an active sweep, the default `quit_tack(5)` is
    /// usually sufficient.
    pub quit_path: Option<fn(&mut PtySession) -> ExitStatus>,

    /// Per-scenario screen parser. Same `ScreenParserFn` type as
    /// [`ScenarioSpec::parser`].
    pub parser: ScreenParserFn,
}

/// Sentinel byte sequence for unverified menu keys.
///
/// Used by 05.2 / 05.3 / 05.4 / 05.4b `MenuStep::send` and
/// `PhaseSpec::phase_trigger` placeholders for menu keys that 05.0's
/// `BEGIN_TESTING_INVENTORY` discovery has not yet pinned.
///
/// Returns a non-printable, recognizable, byte sequence so that
/// `runner::assert_no_unverified_sentinels` can detect it BEFORE
/// writing it to tack's PTY. The runner panics with a referral to
/// 05.0 instead of silently sending garbage to tack.
///
/// **Why a runtime sentinel and not `compile_error!`.** An earlier
/// `/review-plan` draft used `compile_error!` directives in
/// 05.2 / 05.3 / 05.4 to gate the new scenarios on 05.0's discovery
/// work. That broke `cargo check` for the entire `oriterm_test_support`
/// crate while 05.0 was in flight, which in turn blocked unrelated
/// impl-hygiene review work in the same crate. The runtime sentinel
/// preserves the "no fake key bytes" intent — the const cannot be
/// used in a passing test until the implementer reads the
/// inventory and replaces it with a real key — but lets the
/// workspace continue to compile so concurrent work in adjacent
/// files is not blocked. The Codex midpoint review of `/review-plan`
/// flagged the `compile_error!` approach as too hostile in a
/// multi-agent flow (Pivot 3).
///
/// The sentinel is `b"\x00__UNVERIFIED__\x00"`. The leading and
/// trailing NUL bytes guarantee it cannot collide with any printable
/// navigation key (tack uses ASCII letter and punctuation keys), and
/// `__UNVERIFIED__` makes the panic message human-readable when the
/// runner reports the bytes it refused to send.
#[must_use]
pub const fn unverified_menu_key() -> &'static [u8] {
    b"\x00__UNVERIFIED__\x00"
}

/// Sentinel anchor string for unverified anchors.
///
/// Used by 05.2 / 05.3 / 05.4 / 05.4b `MenuStep::wait_for`,
/// `ScenarioSpec::ready_anchor`, `PhaseSpec::phase_setup_anchor`,
/// and `PhaseSpec::phase_anchor` placeholders that 05.0's discovery
/// has not yet pinned.
///
/// Same rationale as [`unverified_menu_key`]. Detected by
/// [`is_unverified_anchor`] before any `wait_for_*` call. The
/// string contains characters tack would never put on screen so it
/// cannot accidentally match a real anchor in the
/// pre-existing-anchor guard.
#[must_use]
pub const fn unverified_anchor() -> &'static str {
    "<UNVERIFIED ANCHOR — see 05.0>"
}

/// Predicate: does `bytes` look like the [`unverified_menu_key`]
/// sentinel?
///
/// Equality check, not substring — the sentinel is a sequence of
/// known length, and a substring check would false-match real keys
/// that happen to contain a NUL.
#[must_use]
pub fn is_unverified_menu_key(bytes: &[u8]) -> bool {
    bytes == unverified_menu_key()
}

/// Predicate: does `s` look like the [`unverified_anchor`]
/// sentinel?
///
/// Equality check.
#[must_use]
pub fn is_unverified_anchor(s: &str) -> bool {
    s == unverified_anchor()
}
