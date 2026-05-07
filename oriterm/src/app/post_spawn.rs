//! Post-spawn pane setup helper.
//!
//! Consolidates the 4-call propagation block that historically appeared
//! at every `MuxBackend::spawn_pane` / `adopt_pane` site. Adding a new
//! per-pane config knob (e.g. answerback) becomes a one-line change here
//! instead of six.
//!
//! See `bug-tracker/plans/completed/BUG-05-001/` for the rationale and
//! the consensus that drove this extraction (impl-hygiene
//! `LEAK:algorithmic-duplication` per
//! `.claude/rules/impl-hygiene.md §Algorithmic DRY`).

use oriterm_core::{Palette, Theme};
use oriterm_mux::{MuxBackend, PaneId};

use crate::config::Config;

/// Geometry-and-theme arguments for `apply_post_spawn_setup`.
///
/// Grouped into a struct to satisfy the `>4 parameters → options struct`
/// rule from `.claude/rules/impl-hygiene.md §Parameter Hygiene`. Each
/// field is a direct mux-backend setter argument that doesn't live on
/// `Config`.
pub(crate) struct PostSpawnArgs {
    /// Resolved theme to apply to the pane's terminal palette.
    pub theme: Theme,
    /// Resolved color palette derived from `Config.colors`.
    pub palette: Palette,
    /// Cell width in physical pixels — seeds image-protocol `FixedPixels`
    /// coverage from the first frame.
    pub cell_w: u16,
    /// Cell height in physical pixels — seeds image-protocol
    /// `FixedPixels` coverage from the first frame.
    pub cell_h: u16,
}

/// Apply the standard post-spawn config propagation to a freshly created pane.
///
/// Call this immediately after a successful `MuxBackend::spawn_pane` /
/// `adopt_pane` to wire theme, image-protocol config, bold-is-bright,
/// and cell metrics into the IO thread's `Term`.
///
/// The free-function shape (rather than an `impl App` method) lets
/// callers pass an already-borrowed `&mut dyn MuxBackend` from
/// `App.mux.as_mut()` without conflicting with the held mutable borrow
/// of `App.mux`.
pub(crate) fn apply_post_spawn_setup(
    mux: &mut dyn MuxBackend,
    config: &Config,
    pane_id: PaneId,
    args: PostSpawnArgs,
) {
    mux.set_pane_theme(pane_id, args.theme, args.palette);
    mux.set_image_config(pane_id, config.terminal.image_config());
    mux.set_bold_is_bright(pane_id, config.behavior.bold_is_bright);
    mux.set_cell_dimensions(pane_id, args.cell_w, args.cell_h);
}
