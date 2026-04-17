//! Terminal state machine.
//!
//! `Term<S: EffectSink>` owns two grids (primary + alternate), mode flags,
//! color palette, charset state, and processes escape sequences via the
//! `vte::ansi::Handler` trait. Generic over `EffectSink` for decoupling
//! from the UI layer.

mod alt_screen;
pub mod charset;
mod handler;
mod image_config;
pub mod mode;
pub mod renderable;
mod resize;
mod shell_state;
mod snapshot;

pub use charset::CharsetState;
pub use mode::TermMode;
pub use renderable::{
    DamageLine, RenderableCell, RenderableContent, RenderableCursor, RenderableImageData,
    RenderablePlacement, TermDamage, maybe_shrink_vec,
};

use std::collections::{HashMap, VecDeque};

use vte::ansi::KeyboardModes;

use crate::color::Palette;
use crate::effect::sink::EffectSink;
use crate::effect::{Effect, PtyEffect, PtyWriteKind};
use crate::grid::{CursorShape, Grid};
use crate::image::ImageCache;
use crate::image::sixel::SixelParser;
use crate::theme::Theme;

/// Shell integration prompt lifecycle state.
///
/// Tracks transitions from OSC 133 sub-parameters:
/// `None` → `PromptStart` (A) → `CommandStart` (B) → `OutputStart` (C) → `None` (D).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PromptState {
    /// No prompt activity or command completed (after D marker).
    #[default]
    None,
    /// Prompt is being displayed (after A marker).
    PromptStart,
    /// User is typing a command (after B marker).
    CommandStart,
    /// Command output is being produced (after C marker).
    OutputStart,
}

/// A single prompt lifecycle's boundary rows (absolute row indices).
///
/// Associates the OSC 133 sub-marker rows for one prompt: where the prompt
/// started (A), where the command line started (B), and where command output
/// started (C). Used for semantic zone navigation and selection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PromptMarker {
    /// Absolute row where OSC 133;A (prompt start) was received.
    pub prompt: usize,
    /// Absolute row where OSC 133;B (command start) was received.
    pub command: Option<usize>,
    /// Absolute row where OSC 133;C (output start) was received.
    pub output: Option<usize>,
}

/// Desktop notification from the shell (OSC 9/99/777).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Notification {
    /// Notification title (may be empty for OSC 9/99).
    pub title: String,
    /// Notification body text.
    pub body: String,
}

/// Maximum depth for title stack (xterm push/pop title).
///
/// Prevents OOM from malicious PTY input pushing unlimited titles.
/// Matches Alacritty's cap. Enforced in the VTE handler's `push_title`.
const TITLE_STACK_MAX_DEPTH: usize = 4096;

/// Maximum depth for Kitty keyboard enhancement mode stacks.
///
/// Prevents OOM from malicious PTY input. Matches Alacritty's cap.
/// Enforced in the VTE handler's `push_keyboard_mode`.
pub(crate) const KEYBOARD_MODE_STACK_MAX_DEPTH: usize = 4096;

bitflags::bitflags! {
    /// Deferred OSC 133 marking actions.
    ///
    /// These flags are set when the corresponding OSC 133 sequence arrives
    /// and cleared after both VTE parsers finish processing, when the actual
    /// grid row marking occurs.
    #[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
    pub struct PendingMarks: u8 {
        /// OSC 133;A received — prompt row marking deferred.
        const PROMPT = 1;
        /// OSC 133;B received — command start row marking deferred.
        const COMMAND_START = 2;
        /// OSC 133;C received — output start row marking deferred.
        const OUTPUT_START = 4;
    }
}

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
    /// Default column count for DECCOLM reset (CSI ? 3 l).
    ///
    /// When DECCOLM is reset, the grid restores to this width instead of
    /// hardcoded 80. Updated on external resize so it always reflects the
    /// window's native column count.
    deccolm_default_cols: usize,
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
            deccolm_default_cols: cols,
        }
    }

    /// Effect sink for boundary-crossing side effects.
    pub fn effect_sink(&self) -> &S {
        &self.effect_sink
    }

    /// Whether bold text promotes ANSI colors 0–7 to bright 8–15.
    pub fn bold_is_bright(&self) -> bool {
        self.bold_is_bright
    }

    /// Set the bold-as-bright behavior.
    pub fn set_bold_is_bright(&mut self, enabled: bool) {
        self.bold_is_bright = enabled;
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

    /// Reference to the color palette.
    pub fn palette(&self) -> &Palette {
        &self.palette
    }

    /// Mutable reference to the color palette (for config overrides).
    pub fn palette_mut(&mut self) -> &mut Palette {
        &mut self.palette
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

    /// Current color theme.
    pub fn theme(&self) -> Theme {
        self.theme
    }

    /// Switch the active color theme.
    ///
    /// Rebuilds the palette for the new theme and marks all lines dirty so
    /// the renderer repaints with the new colors. No-op if the theme is
    /// unchanged.
    pub fn set_theme(&mut self, theme: Theme) {
        if self.theme == theme {
            return;
        }
        self.theme = theme;
        self.palette = Palette::for_theme(theme);
        self.grid_mut().dirty_mut().mark_all();

        // Mode 2031: notify child process of color scheme change.
        if self.mode.contains(TermMode::COLOR_SCHEME_UPDATE) {
            let notification = match theme {
                Theme::Dark => Some(b"\x1b[?997;1n".as_slice()),
                Theme::Light => Some(b"\x1b[?997;2n".as_slice()),
                Theme::Unknown => None,
            };
            if let Some(bytes) = notification {
                self.effect_sink.push(Effect::Pty(PtyEffect::Write {
                    bytes: bytes.to_vec(),
                    kind: PtyWriteKind::Other,
                }));
            }
        }
    }

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

    /// Current cursor shape.
    pub fn cursor_shape(&self) -> CursorShape {
        self.cursor_shape
    }

    /// Override the cursor shape (config-driven, not VTE-driven).
    pub fn set_cursor_shape(&mut self, shape: CursorShape) {
        self.cursor_shape = shape;
    }

    /// Reference to the charset state.
    pub fn charset(&self) -> &CharsetState {
        &self.charset
    }

    /// The title stack (xterm push/pop title).
    #[cfg(test)]
    pub(crate) fn title_stack(&self) -> &VecDeque<String> {
        &self.title_stack
    }

    /// Current keyboard mode stack (Kitty keyboard protocol).
    #[cfg(test)]
    pub(crate) fn keyboard_mode_stack(&self) -> &VecDeque<KeyboardModes> {
        &self.keyboard_mode_stack
    }

    // Rendering snapshot methods (renderable_content, renderable_content_into,
    // damage, reset_damage) are in `snapshot.rs`.

    // Resize method (resize, image-cache lifecycle) is in `resize.rs`.

    // Alt screen swap methods (swap_alt, swap_alt_no_cursor, swap_alt_clear)
    // are in `alt_screen.rs`.
}

// `cwd_short_path` lives in `shell_state.rs` alongside other shell
// integration helpers. Re-exported here for public API compatibility.
pub use shell_state::cwd_short_path;

#[cfg(test)]
mod tests;
