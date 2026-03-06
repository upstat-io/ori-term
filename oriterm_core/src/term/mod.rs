//! Terminal state machine.
//!
//! `Term<T: EventListener>` owns two grids (primary + alternate), mode flags,
//! color palette, charset state, and processes escape sequences via the
//! `vte::ansi::Handler` trait. Generic over `EventListener` for decoupling
//! from the UI layer.

mod alt_screen;
pub mod charset;
mod handler;
pub mod mode;
pub mod renderable;
mod shell_state;
mod snapshot;

pub use charset::CharsetState;
pub use mode::TermMode;
pub use renderable::{DamageLine, RenderableCell, RenderableContent, RenderableCursor, TermDamage};

use std::collections::{HashMap, VecDeque};

use vte::ansi::KeyboardModes;

use crate::color::Palette;
use crate::event::EventListener;
use crate::grid::{CursorShape, Grid, StableRowIndex};
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
/// `T: EventListener` so tests can use `VoidListener` while the real app
/// routes events through winit.
#[derive(Debug)]
#[allow(
    clippy::struct_excessive_bools,
    reason = "terminal state naturally has independent boolean flags \
              (selection_dirty, has_explicit_title, title_dirty)"
)]
pub struct Term<T: EventListener> {
    /// Primary grid (active when not in alt screen).
    grid: Grid,
    /// Alternate grid (active during alt screen; no scrollback).
    alt_grid: Grid,
    /// Terminal mode flags (DECSET/DECRST).
    mode: TermMode,
    /// Color palette (270 entries).
    palette: Palette,
    /// Active color theme (dark/light).
    theme: Theme,
    /// Character set translation state (G0–G3).
    charset: CharsetState,
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
    /// Event sink for terminal events.
    event_listener: T,
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
    /// Pending desktop notifications collected from OSC 9/99/777.
    pending_notifications: Vec<Notification>,
    /// When OSC 133;C (output start) was received — marks command execution start.
    command_start: Option<std::time::Instant>,
    /// Duration of the last completed command (OSC 133;D − OSC 133;C).
    last_command_duration: Option<std::time::Duration>,
    /// Whether the current title was explicitly set via OSC 0/2.
    /// When `false`, the tab bar should prefer CWD-based title.
    has_explicit_title: bool,
    /// Title dirty flag — set when CWD or explicit title changes.
    title_dirty: bool,
    /// XTSAVE/XTRESTORE: saved private mode values (single save per mode).
    saved_private_modes: HashMap<u16, bool>,
    /// Image cache for the primary screen.
    image_cache: ImageCache,
    /// Image cache for the alternate screen.
    alt_image_cache: ImageCache,
    /// In-progress chunked Kitty image transmission.
    loading_image: Option<crate::image::kitty::LoadingImage>,
    /// In-progress sixel image (active during DCS sixel sequence).
    sixel_parser: Option<SixelParser>,
    /// Cell width in pixels (set by GUI after font metrics are known).
    cell_pixel_width: u16,
    /// Cell height in pixels (set by GUI after font metrics are known).
    cell_pixel_height: u16,
}

impl<T: EventListener> Term<T> {
    /// Create a new terminal with the given dimensions and scrollback capacity.
    pub fn new(lines: usize, cols: usize, scrollback: usize, theme: Theme, listener: T) -> Self {
        Self {
            grid: Grid::with_scrollback(lines, cols, scrollback),
            alt_grid: Grid::with_scrollback(lines, cols, 0),
            mode: TermMode::default(),
            palette: Palette::for_theme(theme),
            theme,
            charset: CharsetState::default(),
            title: String::new(),
            icon_name: String::new(),
            cwd: None,
            title_stack: VecDeque::new(),
            cursor_shape: CursorShape::default(),
            keyboard_mode_stack: VecDeque::new(),
            inactive_keyboard_mode_stack: VecDeque::new(),
            event_listener: listener,
            selection_dirty: false,
            prompt_state: PromptState::None,
            pending_marks: PendingMarks::empty(),
            prompt_markers: Vec::new(),
            pending_notifications: Vec::new(),
            command_start: None,
            last_command_duration: None,
            has_explicit_title: false,
            title_dirty: false,
            saved_private_modes: HashMap::new(),
            image_cache: ImageCache::new(),
            alt_image_cache: ImageCache::new(),
            loading_image: None,
            sixel_parser: None,
            cell_pixel_width: 8,
            cell_pixel_height: 16,
        }
    }

    /// Event listener for terminal events.
    pub fn event_listener(&self) -> &T {
        &self.event_listener
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
    pub fn grid(&self) -> &Grid {
        if self.mode.contains(TermMode::ALT_SCREEN) {
            &self.alt_grid
        } else {
            &self.grid
        }
    }

    /// Mutable reference to the active grid.
    pub fn grid_mut(&mut self) -> &mut Grid {
        if self.mode.contains(TermMode::ALT_SCREEN) {
            &mut self.alt_grid
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
    pub fn image_cache(&self) -> &ImageCache {
        if self.mode.contains(TermMode::ALT_SCREEN) {
            &self.alt_image_cache
        } else {
            &self.image_cache
        }
    }

    /// Mutable reference to the active screen's image cache.
    pub fn image_cache_mut(&mut self) -> &mut ImageCache {
        if self.mode.contains(TermMode::ALT_SCREEN) {
            &mut self.alt_image_cache
        } else {
            &mut self.image_cache
        }
    }

    /// Set cell pixel dimensions (called by GUI after font metrics are known).
    pub fn set_cell_dimensions(&mut self, width: u16, height: u16) {
        self.cell_pixel_width = width;
        self.cell_pixel_height = height;
    }

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

    /// Resize the terminal to new dimensions.
    ///
    /// Resizes both primary and alternate grids. The primary grid uses text
    /// reflow (soft-wrapped lines re-wrap to fit the new width). The alternate
    /// grid does not reflow (full-screen apps manage their own layout).
    ///
    /// Marks all lines dirty so the renderer repaints. Also marks selection
    /// as dirty since content positions change.
    pub fn resize(&mut self, new_lines: usize, new_cols: usize) {
        if new_lines == 0 || new_cols == 0 {
            return;
        }

        // Primary grid: reflow enabled. Prune image placements if rows evicted.
        let prev_primary = self.grid.total_evicted();
        self.grid.resize(new_lines, new_cols, true);
        let new_primary = self.grid.total_evicted();
        if new_primary > prev_primary {
            self.image_cache
                .prune_scrollback(StableRowIndex(new_primary as u64));
        }

        // Alternate grid: no reflow (apps like vim handle their own layout).
        // Alt grid has 0 scrollback capacity, so every scroll evicts.
        let prev_alt = self.alt_grid.total_evicted();
        self.alt_grid.resize(new_lines, new_cols, false);
        let new_alt = self.alt_grid.total_evicted();
        if new_alt > prev_alt {
            self.alt_image_cache
                .prune_scrollback(StableRowIndex(new_alt as u64));
        }

        // Mark selection dirty since cell positions changed.
        // Note: both grids are already fully marked dirty by
        // `Grid::finalize_resize` → `dirty.resize()` → `mark_all()`.
        self.selection_dirty = true;
    }

    // Alt screen swap methods (swap_alt, swap_alt_no_cursor, swap_alt_clear)
    // are in `alt_screen.rs`.
}

/// Extract the last path component from a CWD path for tab display.
///
/// - `/home/user/projects` → `projects`
/// - `/` → `/`
/// - `~` passthrough (shouldn't occur from OSC 7, but handle gracefully).
pub fn cwd_short_path(cwd: &str) -> &str {
    if cwd == "/" {
        return cwd;
    }
    // Strip trailing slash then take last component.
    let trimmed = cwd.strip_suffix('/').unwrap_or(cwd);
    let component = trimmed.rsplit('/').next().unwrap_or(cwd);
    // Paths like `///` reduce to an empty component after stripping — return `/`.
    if component.is_empty() { "/" } else { component }
}

#[cfg(test)]
mod tests;
