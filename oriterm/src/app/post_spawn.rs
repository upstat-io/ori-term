//! Post-spawn pane setup helper.
//!
//! Consolidates the propagation block that runs at every
//! `MuxBackend::spawn_pane` / `adopt_pane` site (theme, image config,
//! bold-is-bright, answerback, cell metrics). Adding a new per-pane
//! config knob becomes a one-line change here instead of one per
//! spawn site.

use oriterm_core::{Palette, Theme};
use oriterm_mux::{MuxBackend, PaneId};

use crate::config::Config;

/// Geometry-and-theme arguments for `apply_post_spawn_setup`.
///
/// Grouped into a struct so the helper signature stays under the
/// 4-parameter limit (the rest of the per-pane state is read from
/// `Config` via the `&Config` argument). Each field is a direct
/// mux-backend setter argument that doesn't live on `Config`.
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
    mux.set_answerback(pane_id, config.behavior.answerback.clone().into_bytes());
    mux.set_cell_dimensions(pane_id, args.cell_w, args.cell_h);
}
