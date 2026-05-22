//! Terminal state machine.
//!
//! `Term<S: EffectSink>` owns two grids (primary + alternate), mode flags,
//! color palette, charset state, and processes escape sequences via the
//! `vte::ansi::Handler` trait. Generic over `EffectSink` for decoupling
//! from the UI layer.

mod alt_screen;
pub mod charset;
mod colors_state;
mod dec_observable;
pub(crate) mod handler;
mod image_config;
mod iterm2_state;
pub mod mode;
pub mod renderable;
mod resize;
mod shell_state;
mod snapshot;
mod visual_state;

pub use charset::CharsetState;
pub use dec_observable::AceMode;
pub use mode::TermMode;
pub use mode::encode_enter_base;
pub use renderable::{
    DamageLine, RenderableCell, RenderableContent, RenderableCursor, RenderableImageData,
    RenderablePlacement, TermDamage, maybe_shrink_vec,
};
pub use shell_state::{Notification, PendingMarks, PromptMarker, PromptState};

use std::collections::{HashMap, VecDeque};

use vte::ansi::KeyboardModes;
use vte::ansi::cursor_icon::CursorIcon;

use crate::color::Palette;
use crate::effect::sink::EffectSink;
use crate::grid::{CursorShape, Grid};
use crate::image::ImageCache;
use crate::image::sixel::SixelParser;
use crate::term::colors_state::TermColorsState;
use crate::term::iterm2_state::Iterm2State;
use crate::theme::Theme;

/// Maximum depth for title stack (xterm push/pop title).
///
/// Prevents OOM from malicious PTY input pushing unlimited titles.
/// Matches Alacritty's cap. Enforced in the VTE handler's `push_title`.
const TITLE_STACK_MAX_DEPTH: usize = 4096;

/// Maximum depth for Kitty keyboard enhancement mode stacks.
///
/// Prevents OOM from malicious PTY input. Matches Alacritty's cap.
/// Enforced in the VTE handler's `push_keyboard_mode`.
pub const KEYBOARD_MODE_STACK_MAX_DEPTH: usize = 4096;

/// The terminal state machine.
///
/// Owns two grids (primary + alternate screen), terminal mode flags, color
/// palette, charset state, title, and keyboard mode stacks. Generic over
/// `S: EffectSink` so tests can use `VoidEffectSink` while the real app
/// routes effects through a legacy adapter or queuing sink.
#[derive(Debug)]
#[allow(
    clippy::struct_excessive_bools,
    reason = "terminal state naturally has independent boolean flags \
              (selection_dirty, has_explicit_title, title_dirty)"
)]
pub struct Term<S: EffectSink> {
    /// Primary grid (active when not in alt screen).
    grid: Grid,
    /// Alternate grid (active during alt screen; no scrollback).
    /// Lazily allocated on first alt screen entry (DECSET 47/1047/1049).
    /// Most terminals never enter alt screen, saving ~28 KB per terminal.
    alt_grid: Option<Grid>,
    /// Terminal mode flags (DECSET/DECRST).
    mode: TermMode,
    /// Color palette (270 entries).
    palette: Palette,
    /// Active color theme (dark/light).
    theme: Theme,
    /// Character set translation state (G0–G3).
    charset: CharsetState,
    /// DECSC-saved charset state (active screen, restored by DECRC).
    saved_charset: Option<CharsetState>,
    /// DECSC-saved origin mode flag (active screen, restored by DECRC).
    saved_origin_mode: Option<bool>,
    /// DECSC-saved charset state (inactive screen — swapped on alt screen toggle).
    inactive_saved_charset: Option<CharsetState>,
    /// DECSC-saved origin mode flag (inactive screen — swapped on alt screen toggle).
    inactive_saved_origin_mode: Option<bool>,
    /// Window title (set by OSC 0/2).
    title: String,
    /// Icon name (set by OSC 0/1).
    icon_name: String,
    /// Current working directory (set by OSC 7 shell integration).
    cwd: Option<String>,
    /// Pushed title stack (xterm extension). Capped at [`TITLE_STACK_MAX_DEPTH`].
    title_stack: VecDeque<String>,
    /// Cursor shape for rendering.
    cursor_shape: CursorShape,
    /// Kitty keyboard enhancement mode stack (active screen).
    /// Capped at [`KEYBOARD_MODE_STACK_MAX_DEPTH`].
    keyboard_mode_stack: VecDeque<KeyboardModes>,
    /// Kitty keyboard enhancement mode stack (inactive screen).
    /// Capped at [`KEYBOARD_MODE_STACK_MAX_DEPTH`].
    inactive_keyboard_mode_stack: VecDeque<KeyboardModes>,
    /// Full snapshot of `keyboard_mode_stack` taken at OSC 133 ; C
    /// (command-start) on the ACTIVE screen. Restored on the next
    /// OSC 133 ; A or ; D so kitty keyboard modes pushed, popped, OR
    /// evicted (at `KEYBOARD_MODE_STACK_MAX_DEPTH`) by a subprocess
    /// that exited without cleanly popping don't persist or erase shell
    /// state. `None` means no snapshot active for this screen.
    ///
    /// Contents-based (not depth-based) so a child that over-pops
    /// shell-held modes or pushes past max-depth (evicting shell modes
    /// from the front) is fully reversed at the next prompt boundary.
    /// Paired with [`inactive_pre_command_kb_stack_snapshot`]; swapped
    /// alongside the stacks in `toggle_alt_common`. See.
    pre_command_kb_stack_snapshot: Option<VecDeque<KeyboardModes>>,
    /// Paired snapshot for the inactive (non-visible) screen's keyboard
    /// mode stack. Taken alongside [`pre_command_kb_stack_snapshot`] at
    /// OSC 133 ; C so a child that enters the alt screen, pushes kitty
    /// modes, and exits without popping does not leak state into the
    /// non-visible stack. See.
    inactive_pre_command_kb_stack_snapshot: Option<VecDeque<KeyboardModes>>,
    /// Snapshot of the ACTIVE Kitty-keyboard-protocol `TermMode` bits
    /// taken at OSC 133 ; C, paired with [`pre_command_kb_stack_snapshot`].
    ///
    /// Required because shells may use `CSI = Ps u` (SET without push) to
    /// enable kitty modes — that path updates `TermMode` bits via
    /// `dcs_set_keyboard_mode` WITHOUT pushing to the stack. Snapshotting
    /// stack contents alone loses the set-only bits; at restore time the
    /// top-of-stack would be `NO_MODE` and the shell's set bits would be
    /// cleared. Taking a paired bits snapshot and applying it at restore
    /// preserves shell-held kitty state for both push-path and set-path
    /// integrations. See review round-1 F1.
    pre_command_kb_mode_bits_snapshot: Option<KeyboardModes>,
    /// Live Kitty-keyboard-protocol bits for the INACTIVE screen.
    ///
    /// `TermMode::KITTY_KEYBOARD_PROTOCOL` inside `self.mode` reflects
    /// only the active screen; the inactive screen's effective kitty
    /// bits are stored here. Swapped alongside the paired stacks in
    /// `toggle_alt_common` so set-only bits enabled via `CSI = Ps u`
    /// survive alt-screen toggles even when no shell integration (no
    /// OSC 133 snapshot) is present. See review round-3 F1/F2.
    inactive_keyboard_mode_bits: KeyboardModes,
    /// Paired inactive-screen bits snapshot — captured at OSC 133 ; C
    /// alongside [`pre_command_kb_mode_bits_snapshot`]. Required because
    /// the command-boundary snapshot belongs to the screen where `;C`
    /// was emitted; an alt-screen toggle mid-command must carry the
    /// snapshot along, so restore on the owning screen applies the
    /// correct bits. Live `inactive_keyboard_mode_bits` tracks runtime
    /// per-screen state; this field tracks the per-screen restore
    /// target separately. See review round-4.
    inactive_pre_command_kb_mode_bits_snapshot: Option<KeyboardModes>,
    /// Effect sink for boundary-crossing side effects.
    effect_sink: S,
    /// Set by content-modifying VTE handler operations (character printing,
    /// erase, insert/delete, scroll). Checked by the owning layer to decide
    /// whether to clear an active selection.
    selection_dirty: bool,
    /// Shell integration prompt lifecycle state (OSC 133).
    prompt_state: PromptState,
    /// Deferred OSC 133 row marking (A/B/C). Cleared after both VTE
    /// parsers finish processing when actual grid marking occurs.
    pending_marks: PendingMarks,
    /// Prompt lifecycle markers (OSC 133 A/B/C positions).
    /// Used for jump-to-prompt navigation and semantic zone selection.
    /// Pruned when scrollback eviction removes old rows.
    prompt_markers: Vec<PromptMarker>,
    /// When OSC 133;C (output start) was received — marks command execution start.
    command_start: Option<std::time::Instant>,
    /// Duration of the last completed command (OSC 133;D − OSC 133;C).
    last_command_duration: Option<std::time::Duration>,
    /// Whether the current title was explicitly set via OSC 0/2.
    ///
    /// Set by the VTE handler when OSC 0/2 arrives, cleared on OSC 7
    /// (CWD change) and on full terminal reset. `Pane::has_explicit_title`
    /// mirrors this via `MuxEvent::PaneTitleChanged` — both must agree.
    has_explicit_title: bool,
    /// Title dirty flag — set when CWD or explicit title changes.
    title_dirty: bool,
    /// Whether bold text should use bright ANSI colors (SGR 1 → colors 8–15).
    ///
    /// When `true` (default), BOLD + ANSI color 0–7 promotes to 8–15.
    /// When `false`, bold only affects font weight, not color.
    bold_is_bright: bool,
    /// XTSAVE/XTRESTORE: saved private mode values (single save per mode).
    saved_private_modes: HashMap<u16, bool>,
    /// Image cache for the primary screen.
    image_cache: ImageCache,
    /// Image cache for the alternate screen (lazily allocated with alt grid).
    alt_image_cache: Option<ImageCache>,
    /// In-progress chunked Kitty image transmission.
    loading_image: Option<crate::image::kitty::LoadingImage>,
    /// In-progress sixel image (active during DCS sixel sequence).
    sixel_parser: Option<SixelParser>,
    /// Cell width in pixels (set by GUI after font metrics are known).
    cell_pixel_width: u16,
    /// Cell height in pixels (set by GUI after font metrics are known).
    cell_pixel_height: u16,
    /// Whether image protocols (Kitty, Sixel, iTerm2) are enabled.
    image_protocol_enabled: bool,
    /// XTSMGRAPHICS Pi=1 current color-register count.
    ///
    /// Defaults to [`crate::image::sixel::COLOR_REGISTERS_MAX`] (256 — the
    /// SSOT for the sixel decoder palette size). Mutated on
    /// `CSI ? 1 ; 3 ; <Pv> S` (set) when `Pv > 1 && Pv <= MAX`. Reset to
    /// default on RIS (`ESC c`) by `term::handler::esc::esc_reset_state`.
    /// Snapshotted into `SixelParser::new` at DCS-hook time so the active
    /// decoder honors the negotiated count via the xterm modulo wrap on
    /// color register indices in `crate::image::sixel::SixelParser::apply_color`,
    /// matching xterm `graphics_sixel.c:697-698`. In-flight XTSMGRAPHICS
    /// mutations during an active DCS sequence do NOT retroactively change
    /// the snapshotted count.
    color_register_count: u16,
    /// Default column count for DECCOLM reset (CSI ? 3 l).
    ///
    /// When DECCOLM is reset, the grid restores to this width instead of
    /// hardcoded 80. Updated on external resize so it always reflects the
    /// window's native column count.
    deccolm_default_cols: usize,
    /// Mouse cursor icon requested by the shell (OSC 22).
    ///
    /// `None` until the application sets an icon. The renderer reads this
    /// via `RenderableContent::mouse_cursor_icon` and updates the OS pointer.
    mouse_cursor_icon: Option<CursorIcon>,
    /// Last command line reported by VS Code OSC 633;E.
    ///
    /// `None` until the shell integration sends OSC 633;E with the raw typed
    /// command. The `E` sub-command is interceptor-only — it is NOT routed
    /// through the `Handler` trait because the high-level `vte::ansi::Processor`
    /// does not dispatch OSC 633.
    last_command_line: Option<String>,
    /// iTerm2 OSC 1337 non-image sub-op state (`RemoteHost`, user variables,
    /// shell integration version). See `iterm2_state.rs`.
    iterm2_state: Iterm2State,
    /// OSC 3 / 5 / 6 / 13 / 14 / 17 / 19 terminal-level color + property
    /// state. See `colors_state.rs`.
    colors_state: TermColorsState,
    /// XTCHECKSUM (`CSI Ps # y`) flag bitmask consumed by DECRQCRA.
    ///
    /// Bit layout matches xterm patch-336 (`csPOSITIVE=1`, `csATTRIBS=2`,
    /// `csNOTRIM=4`, `csDRAWN=8`, `csBYTE=16`). Default `0` means
    /// negate-on, attribs-included, trim, DEC-translate — which matches
    /// xterm's default DECRQCRA reply.
    checksum_flags: u16,
    /// DECSCA per-character protection for SUBSEQUENTLY written cells.
    ///
    /// `false` (default) = cells written unprotected. `true` = cells
    /// written with `CellFlags::PROTECTED` set, surviving DECSERA.
    /// Flipped by DECSCA (`CSI Ps " q`): Ps=0 or 2 → false, Ps=1 → true.
    char_protection: bool,
    /// DECSCL conformance level (VT100/VT200/VT300/VT400).
    ///
    /// Observable state only — the parser's C1 dispatch is not gated
    /// on this field (parser scope is out of Term's reach per §09A.8).
    /// Default `64` matches the DA1 response (VT420 conformance).
    conformance_level: u16,
    /// DECSCL C1 mode: `false` = 8-bit C1 (Pc=0 or 2), `true` = 7-bit
    /// C1 (Pc=1). Observable only — does not suppress parser C1
    /// recognition; see §09A.8 deviation note.
    c1_7bit: bool,
    /// DECSASD active status display target.
    ///
    /// 0 = main display (default), 1 = status line. `ori_term` does
    /// not render a status line; the field is stored as observable
    /// state only.
    active_status_display: u16,
    /// DECSSDT status line type.
    ///
    /// 0 = off (default), 1 = indicator, 2 = host-writable.
    /// `ori_term` does not render a status line; stored as observable
    /// state only.
    status_line_type: u16,
    /// DECSACE (`CSI Ps * x`) attribute-change extent mode.
    ///
    /// Governs whether DECCARA / DECRARA operate on a stream of cells
    /// (Ps=0 or 1 — default, wraps across row boundaries inside the
    /// rectangle's top/bottom rows) or a strict rectangle (Ps=2 — every
    /// affected cell lies within the rectangle's left/right columns).
    /// Stored on Term per the §09A.6/09A.8 LEAK guard: this is a mode
    /// concern, never a grid concern.
    ace_mode: AceMode,
    /// Configured answerback string emitted on `ENQ` (`0x05`).
    /// Empty by default; emission suppressed when empty (matches `WezTerm`
    /// `term/src/terminalstate/performer.rs:473-479`). Terminal-global
    /// config — survives RIS / DECSTR / DECSC/DECRC / alt-screen toggle.
    answerback: Vec<u8>,
}

impl<S: EffectSink> Term<S> {
    /// Create a new terminal with the given dimensions and scrollback capacity.
    pub fn new(lines: usize, cols: usize, scrollback: usize, theme: Theme, effect_sink: S) -> Self {
        Self {
            grid: Grid::with_scrollback(lines, cols, scrollback),
            alt_grid: None,
            mode: TermMode::default(),
            palette: Palette::for_theme(theme),
            theme,
            charset: CharsetState::default(),
            saved_charset: None,
            saved_origin_mode: None,
            inactive_saved_charset: None,
            inactive_saved_origin_mode: None,
            title: String::new(),
            icon_name: String::new(),
            cwd: None,
            title_stack: VecDeque::new(),
            cursor_shape: CursorShape::default(),
            keyboard_mode_stack: VecDeque::new(),
            inactive_keyboard_mode_stack: VecDeque::new(),
            pre_command_kb_stack_snapshot: None,
            inactive_pre_command_kb_stack_snapshot: None,
            pre_command_kb_mode_bits_snapshot: None,
            inactive_keyboard_mode_bits: KeyboardModes::NO_MODE,
            inactive_pre_command_kb_mode_bits_snapshot: None,
            effect_sink,
            selection_dirty: false,
            prompt_state: PromptState::None,
            pending_marks: PendingMarks::empty(),
            prompt_markers: Vec::new(),
            command_start: None,
            last_command_duration: None,
            has_explicit_title: false,
            title_dirty: false,
            bold_is_bright: true,
            saved_private_modes: HashMap::new(),
            image_cache: ImageCache::new(),
            alt_image_cache: None,
            loading_image: None,
            sixel_parser: None,
            cell_pixel_width: 8,
            cell_pixel_height: 16,
            image_protocol_enabled: true,
            color_register_count: crate::image::sixel::COLOR_REGISTERS_MAX,
            deccolm_default_cols: cols,
            mouse_cursor_icon: None,
            last_command_line: None,
            iterm2_state: Iterm2State::new(),
            colors_state: TermColorsState::new(),
            checksum_flags: 0,
            char_protection: false,
            conformance_level: 64,
            c1_7bit: true,
            active_status_display: 0,
            status_line_type: 0,
            ace_mode: AceMode::default(),
            answerback: Vec::new(),
        }
    }

    /// Effect sink for boundary-crossing side effects.
    pub fn effect_sink(&self) -> &S {
        &self.effect_sink
    }

    /// Whether grid content was modified since the last check.
    ///
    /// Set by content-modifying VTE handler operations (character printing,
    /// erase, insert/delete, scroll). The owning layer should check this
    /// after terminal output and clear any active selection when true.
    pub fn is_selection_dirty(&self) -> bool {
        self.selection_dirty
    }

    /// Reset the selection-dirty flag after handling invalidation.
    pub fn clear_selection_dirty(&mut self) {
        self.selection_dirty = false;
    }

    /// Reference to the active grid.
    ///
    /// Returns the alternate grid when `ALT_SCREEN` mode is active, falling
    /// back to the primary grid if the alt grid was not allocated (race
    /// condition at mode transition or malformed escape sequence).
    pub fn grid(&self) -> &Grid {
        if self.mode.contains(TermMode::ALT_SCREEN) {
            debug_assert!(
                self.alt_grid.is_some(),
                "ALT_SCREEN set but alt_grid not allocated"
            );
            self.alt_grid.as_ref().unwrap_or(&self.grid)
        } else {
            &self.grid
        }
    }

    /// Mutable reference to the active grid.
    ///
    /// Returns the alternate grid when `ALT_SCREEN` mode is active, falling
    /// back to the primary grid if the alt grid was not allocated.
    pub fn grid_mut(&mut self) -> &mut Grid {
        if self.mode.contains(TermMode::ALT_SCREEN) {
            debug_assert!(
                self.alt_grid.is_some(),
                "ALT_SCREEN set but alt_grid not allocated"
            );
            self.alt_grid.as_mut().unwrap_or(&mut self.grid)
        } else {
            &mut self.grid
        }
    }

    /// Current terminal mode flags.
    pub fn mode(&self) -> TermMode {
        self.mode
    }

    /// Reference to the active screen's image cache.
    ///
    /// Mirrors [`Self::grid`]: returns `alt_image_cache` when
    /// `ALT_SCREEN` is active, primary `image_cache` otherwise. The
    /// semantic field convention is load-bearing: `image_cache` always
    /// holds primary-screen placements, `alt_image_cache` always holds
    /// alt-screen placements. `Term::resize` pairs `self.grid` with
    /// `self.image_cache` and `self.alt_grid` with `self.alt_image_cache`
    /// — that pairing is correct only when neither field is swapped,
    /// so do NOT reintroduce the cache swap in `toggle_alt_common`.
    pub fn image_cache(&self) -> &ImageCache {
        if self.mode.contains(TermMode::ALT_SCREEN) {
            debug_assert!(
                self.alt_image_cache.is_some(),
                "ALT_SCREEN set but alt_image_cache not allocated"
            );
            self.alt_image_cache.as_ref().unwrap_or(&self.image_cache)
        } else {
            &self.image_cache
        }
    }

    /// Mutable reference to the active screen's image cache.
    ///
    /// See [`Self::image_cache`] for the routing contract.
    pub fn image_cache_mut(&mut self) -> &mut ImageCache {
        if self.mode.contains(TermMode::ALT_SCREEN) {
            debug_assert!(
                self.alt_image_cache.is_some(),
                "ALT_SCREEN set but alt_image_cache not allocated"
            );
            self.alt_image_cache
                .as_mut()
                .unwrap_or(&mut self.image_cache)
        } else {
            &mut self.image_cache
        }
    }

    // Image protocol configuration and animation methods are in `image_config.rs`.

    /// Current window title (raw OSC 0/2 value).
    pub fn title(&self) -> &str {
        &self.title
    }

    /// Current icon name (set by OSC 0/1).
    pub fn icon_name(&self) -> &str {
        &self.icon_name
    }

    /// Current working directory (set by OSC 7).
    pub fn cwd(&self) -> Option<&str> {
        self.cwd.as_deref()
    }

    // Shell integration methods (prompt state, CWD, title resolution,
    // notifications, prompt navigation) are in `shell_state.rs`.

    /// Reference to the charset state.
    pub fn charset(&self) -> &CharsetState {
        &self.charset
    }

    /// The title stack (xterm push/pop title).
    #[cfg(test)]
    pub(crate) fn title_stack(&self) -> &VecDeque<String> {
        &self.title_stack
    }

    // Other `impl Term<S>` blocks live in sibling submodules: keyboard-mode
    // stack + OSC 133/633 in `shell_state.rs`; renderable_content + damage in
    // `snapshot.rs`; resize in `resize.rs`; alt-screen swap in `alt_screen.rs`;
    // visual/presentation (theme/palette/cursor/icon) in `visual_state.rs`;
    // DEC private observable state in `dec_observable.rs`.
}

// `cwd_short_path` lives in `shell_state.rs` alongside other shell
// integration helpers. Re-exported here for public API compatibility.
pub use shell_state::cwd_short_path;

#[cfg(test)]
mod tests;
