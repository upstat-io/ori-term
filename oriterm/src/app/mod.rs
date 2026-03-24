//! Application struct and winit event loop handler.
//!
//! [`App`] implements winit's [`ApplicationHandler`] to drive the terminal.
//! It wires together the three-phase rendering pipeline (Extract → Prepare →
//! Render), handles window events, and dispatches terminal events from the
//! PTY reader thread.

mod chrome;
mod clipboard_ops;
pub(crate) mod config_reload;
mod constructors;
mod context_menu;
mod cursor_hover;
pub(crate) mod dialog_context;
mod dialog_management;
mod dialog_rendering;
mod divider_drag;
mod event_loop;
mod event_loop_helpers;
mod floating_drag;
mod init;
mod keyboard_input;
mod mark_mode;
mod mouse_input;
mod mouse_report;
mod mouse_selection;
mod mux_pump;
mod pane_accessors;
mod pane_ops;
mod perf_stats;
mod redraw;
mod render_dispatch;
mod search_ui;
mod settings_overlay;
pub(crate) mod snapshot_grid;
mod tab_bar_input;
mod tab_drag;
mod tab_management;
#[allow(
    dead_code,
    reason = "incremental pipeline — delivery loop wired in OverlayManager migration"
)]
mod widget_pipeline;
pub(crate) mod window_context;
mod window_management;

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use winit::keyboard::ModifiersState;
use winit::window::WindowId;

use oriterm_core::{Selection, TermMode};
use oriterm_mux::{MarkCursor, PaneId};

use crate::session::{SessionRegistry, WindowId as SessionWindowId};
use crate::window_manager::WindowManager;

use self::dialog_context::DialogWindowContext;
use self::event_loop_helpers::{resolve_ui_theme, resolve_ui_theme_with, winit_mods_to_ui};
use self::keyboard_input::ImeState;
use self::mouse_selection::MouseState;
use self::perf_stats::PerfStats;
use self::window_context::WindowContext;
use crate::clipboard::Clipboard;
use crate::config::Config;
use crate::config::monitor::ConfigMonitor;
use crate::event::TermEvent;
use crate::font::FontSet;
use crate::gpu::{GpuPipelines, GpuState, WindowRenderer};
use crate::keybindings::KeyBinding;
use oriterm_mux::MuxNotification;
use oriterm_mux::backend::MuxBackend;
use oriterm_ui::animation::CursorBlink;

use oriterm_ui::theme::UiTheme;

/// Event sender for deferred actions through the event loop.
///
/// Wraps the concrete `EventLoopProxy` behind a callback so logic layers
/// don't depend on winit's concrete type. The concrete binding is set up
/// in the constructors from `EventLoopProxy::send_event`.
pub(crate) struct EventSender(Arc<dyn Fn(TermEvent) + Send + Sync>);

impl EventSender {
    /// Send an event through the event loop.
    pub fn send(&self, event: TermEvent) {
        (self.0)(event);
    }
}

/// Default DPI for font rasterization.
const DEFAULT_DPI: f32 = 96.0;

/// Minimum time between renders (~60 FPS cap).
///
/// Prevents burning CPU when PTY output is continuous. The event loop
/// defers rendering until this budget has elapsed since the last frame.
/// 16ms matches the typical 60 Hz display refresh — sufficient for a
/// terminal and leaves ample time for event processing between frames.
const FRAME_BUDGET: Duration = Duration::from_millis(16);

/// Deferred focus-out state.
///
/// When a terminal window receives `Focused(false)`, the focus-out escape
/// sequence is deferred until `about_to_wait`. If the new focused window
/// turns out to be a child dialog, the focus-out is suppressed — the
/// terminal is still "active" from the user's perspective.
struct PendingFocusOut {
    /// The winit window that lost focus.
    window_id: WindowId,
}

/// Terminal application state and event loop handler.
///
/// Owns all top-level resources: GPU state, renderer, windows, and mux.
/// Implements winit's `ApplicationHandler<TermEvent>` to receive both
/// window events and terminal events from the PTY reader thread.
///
/// Per-window state (widgets, caches, interaction) lives in [`WindowContext`]
/// inside the `windows` map.
pub(crate) struct App {
    // GPU + rendering (lazy init on Resumed).
    gpu: Option<GpuState>,
    /// Shared stateless GPU pipelines and bind group layouts.
    pipelines: Option<GpuPipelines>,
    /// Cached font set with user fallbacks pre-applied (cloned per new window).
    font_set: Option<FontSet>,
    /// Maps loaded fallback index → config index (for `apply_font_config`).
    user_fallback_map: Vec<usize>,

    // Window manager: tracks window kinds, parent-child hierarchy, and focus.
    // Parallels `windows` HashMap — both keyed by winit WindowId.
    window_manager: WindowManager,

    // Per-window state, keyed by winit WindowId for event routing.
    windows: HashMap<WindowId, WindowContext>,
    // Dialog window state, keyed by winit WindowId.
    // Separate from `windows` because dialogs have no terminal grid, tab bar,
    // or session model — they only render UI widgets.
    dialogs: HashMap<WindowId, DialogWindowContext>,
    // Winit ID of the currently focused window (set on Focused(true)).
    focused_window_id: Option<WindowId>,

    // GUI-side session registry: tabs, windows, and ID allocators.
    // Owns the session model — the mux only provides panes.
    session: SessionRegistry,

    // Mux backend (Section 44.3): abstracts in-process vs daemon mux access.
    // Owns pane structs (embedded) or proxies IPC (client).
    mux: Option<Box<dyn MuxBackend>>,
    // Active session window ID (maps to the focused TermWindow).
    active_window: Option<SessionWindowId>,
    // Double-buffer for mux notifications (avoids per-frame allocation).
    notification_buf: Vec<MuxNotification>,

    // Keyboard modifier state (updated on ModifiersChanged).
    modifiers: ModifiersState,

    // Cursor blink state (application-level, not terminal-level).
    cursor_blink: CursorBlink,

    // Whether the OS mouse cursor is currently hidden (typing auto-hide).
    mouse_cursor_hidden: bool,

    // Whether the terminal's CURSOR_BLINKING mode is active.
    // Cached from the last extracted frame to gate blink timer in about_to_wait.
    blinking_active: bool,

    // Last cursor position (line, column) for blink-reset-on-move detection.
    // Compared per frame; reset blink when the cursor moves due to PTY output.
    last_cursor_pos: (usize, usize),

    // Mouse selection state (click detection, drag tracking).
    mouse: MouseState,

    // Per-pane selection state (Section 07: client-side selection).
    // Selection lives on App (not Pane) so daemon mode can operate on
    // snapshot data without locking the terminal.
    pane_selections: HashMap<PaneId, Selection>,

    // Per-pane mark cursor state (Section 08: client-side mark mode).
    // Mark cursor lives on App (not Pane) so daemon mode works.
    mark_cursors: HashMap<PaneId, MarkCursor>,

    // System clipboard for copy/paste.
    clipboard: Clipboard,

    // Event sender for deferred actions through the event loop.
    event_proxy: EventSender,

    // User configuration (loaded from TOML, hot-reloaded on file change).
    config: Config,

    // Merged keybinding table (defaults + user overrides).
    bindings: Vec<KeyBinding>,

    // Config file watcher (kept alive for the lifetime of the app).
    _config_monitor: Option<ConfigMonitor>,

    // IME composition state machine.
    ime: ImeState,

    // Active UI theme. Centralized here so all widget creation and event
    // contexts use a single source of truth. When dynamic theming arrives,
    // only this field and the theme-change handler need updating.
    ui_theme: UiTheme,

    // Widget IDs for the currently-open settings overlay. Set when the
    // overlay opens, cleared on dismiss. Used by overlay dispatch to
    // match widget actions to config fields.
    settings_ids: Option<settings_overlay::SettingsIds>,

    // Working copy of the config being edited in the settings panel.
    // Created when the panel opens, mutated by control changes, applied
    // on Save, discarded on Cancel. `self.config` stays untouched until Save.
    settings_pending: Option<Config>,

    // The dropdown widget ID whose popup is currently open. Set when
    // `OpenDropdown` creates a popup overlay, cleared on selection or
    // dismiss. Used to route `Selected` events to the correct dropdown.
    pending_dropdown_id: Option<oriterm_ui::widget_id::WidgetId>,

    // Deferred focus-out: set in Focused(false), consumed in about_to_wait.
    // If focus moved to a child dialog, the focus-out escape sequence is
    // suppressed (the terminal is still "active" from the user's perspective).
    pending_focus_out: Option<PendingFocusOut>,

    // Pending tear-off state. Set by `tear_off_tab()`, consumed by
    // `check_torn_off_merge()` in `about_to_wait`.
    torn_off_pending: Option<tab_drag::TornOffPending>,

    // Dialog windows pending destruction (Closing → Destroyed).
    // Populated by close_dialog(), drained by drain_pending_destroy() in about_to_wait.
    pending_destroy: Vec<WindowId>,

    // Scratch buffers reused per frame to avoid per-frame allocations.
    scratch_dirty_windows: Vec<WindowId>,
    scratch_pane_sels: HashMap<PaneId, Selection>,
    scratch_pane_mcs: HashMap<PaneId, MarkCursor>,

    // Frame budget: time of last render to enforce FRAME_BUDGET spacing.
    last_render: Instant,

    // Performance counters logged periodically.
    perf: PerfStats,
}

impl App {
    // -- Window context accessors --

    /// The focused window's context, if any.
    fn focused_ctx(&self) -> Option<&WindowContext> {
        self.focused_window_id.and_then(|id| self.windows.get(&id))
    }

    /// The focused window's context (mutable), if any.
    fn focused_ctx_mut(&mut self) -> Option<&mut WindowContext> {
        self.focused_window_id
            .and_then(|id| self.windows.get_mut(&id))
    }

    /// The focused window's renderer, if any.
    fn focused_renderer(&self) -> Option<&WindowRenderer> {
        self.focused_window_id
            .and_then(|id| self.windows.get(&id))
            .and_then(|ctx| ctx.renderer.as_ref())
    }

    /// Mark all windows as needing a redraw.
    ///
    /// Used when mux notifications (PTY output, layout changes) may affect
    /// any window — not just the focused one. In multi-window setups, pane
    /// output in the unfocused window must still trigger a render.
    fn mark_all_windows_dirty(&mut self) {
        for ctx in self.windows.values_mut() {
            ctx.root.mark_dirty();
        }
    }

    /// Mark only the window containing `pane_id` as dirty.
    ///
    /// Falls back to [`mark_all_windows_dirty`] if the pane's window cannot
    /// be resolved (orphan pane during close, or session out of sync).
    fn mark_pane_window_dirty(&mut self, pane_id: PaneId) {
        if let Some(session_wid) = self.session.window_for_pane(pane_id) {
            for ctx in self.windows.values_mut() {
                if ctx.window.session_window_id() == session_wid {
                    ctx.root.mark_dirty();
                    return;
                }
            }
        }
        // Fallback: pane not found in session → mark all dirty.
        self.mark_all_windows_dirty();
    }

    /// Re-rasterize fonts and update rendering settings for a new DPI scale.
    ///
    /// Called when the window moves between monitors with different scale
    /// factors. Recalculates font size at physical DPI, updates hinting
    /// and subpixel mode, and clears/recaches glyph atlases.
    ///
    /// `winit_id` identifies the window whose DPI changed. Only that
    /// window's renderer is affected — other windows keep their DPI.
    fn handle_dpi_change(&mut self, winit_id: WindowId, scale_factor: f64) {
        let Some(gpu) = &self.gpu else { return };
        let Some(ctx) = self.windows.get_mut(&winit_id) else {
            return;
        };
        let Some(renderer) = ctx.renderer.as_mut() else {
            return;
        };
        let scale = scale_factor as f32;
        let physical_dpi = DEFAULT_DPI * scale;

        // Re-rasterize at new physical DPI. This recomputes cell metrics
        // and clears the glyph cache + GPU atlases.
        renderer.set_font_size(self.config.font.size, physical_dpi, gpu);

        // Update hinting and subpixel mode for the new scale factor.
        let hinting = config_reload::resolve_hinting(&self.config.font, scale_factor);
        let opacity = f64::from(self.config.window.effective_opacity());
        let format = config_reload::resolve_subpixel_mode(&self.config.font, scale_factor, opacity)
            .glyph_format();
        renderer.set_hinting_and_format(hinting, format, gpu);

        ctx.pane_cache.invalidate_all();
        ctx.text_cache.clear();
        ctx.root.invalidation_mut().invalidate_all();
        ctx.root.damage_mut().reset();
        ctx.root.mark_dirty();

        // Mark all grid lines dirty so the frame extraction re-reads every
        // cell with the new cell metrics. Without this, the terminal content
        // appears stale until PTY output marks individual lines dirty.
        if let Some(pane_id) = self.active_pane_id_for_window(winit_id) {
            if let Some(mux) = self.mux.as_mut() {
                mux.mark_all_dirty(pane_id);
            }
        }
    }

    /// Handle system dark/light theme change.
    ///
    /// Updates the terminal palette and UI chrome colors. Respects
    /// [`ThemeOverride`]: if the user forced dark/light, the system
    /// notification is ignored — only `Auto` delegates to the system.
    fn handle_theme_changed(&mut self, winit_theme: winit::window::Theme) {
        let system_theme = match winit_theme {
            winit::window::Theme::Dark => oriterm_core::Theme::Dark,
            winit::window::Theme::Light => oriterm_core::Theme::Light,
        };
        let theme = self.config.colors.resolve_theme(|| system_theme);
        let palette = config_reload::build_palette_from_config(&self.config.colors, theme);

        // Apply to all panes via MuxBackend.
        if let Some(mux) = self.mux.as_mut() {
            for pane_id in mux.pane_ids() {
                mux.set_pane_theme(pane_id, theme, palette.clone());
            }
        }

        // Update UI chrome theme (tab bar, window controls).
        self.ui_theme = resolve_ui_theme_with(&self.config, system_theme);
        for ctx in self.windows.values_mut() {
            ctx.tab_bar.apply_theme(&self.ui_theme);
            ctx.pane_cache.invalidate_all();
            ctx.text_cache.clear();
            ctx.root.invalidation_mut().invalidate_all();
            ctx.root.damage_mut().reset();
            ctx.root.mark_dirty();
        }
    }

    /// Read the terminal mode, locking briefly.
    ///
    /// Returns `None` if no active pane is present.
    fn terminal_mode(&self) -> Option<TermMode> {
        let id = self.active_pane_id()?;
        self.pane_mode(id)
    }

    // -- Mux pane accessors --

    /// The active pane's ID for a specific winit window.
    ///
    /// Resolves the session window from the winit window context, then walks
    /// the local session model (window → active tab → active pane) to find
    /// the `PaneId`. Used by window-specific operations (resize, DPI change).
    fn active_pane_id_for_window(&self, winit_id: WindowId) -> Option<PaneId> {
        let ctx = self.windows.get(&winit_id)?;
        let session_wid = ctx.window.session_window_id();
        let win = self.session.get_window(session_wid)?;
        let tab_id = win.active_tab()?;
        let tab = self.session.get_tab(tab_id)?;
        Some(tab.active_pane())
    }

    /// The active pane's ID, derived from the local session model.
    fn active_pane_id(&self) -> Option<PaneId> {
        let win_id = self.active_window?;
        let win = self.session.get_window(win_id)?;
        let tab_id = win.active_tab()?;
        let tab = self.session.get_tab(tab_id)?;
        Some(tab.active_pane())
    }

    /// Terminal mode flags for a pane.
    ///
    /// Delegates to [`MuxBackend::pane_mode`] — embedded mode reads the
    /// lock-free atomic cache, daemon mode reads the cached snapshot.
    fn pane_mode(&self, pane_id: PaneId) -> Option<TermMode> {
        self.mux
            .as_ref()?
            .pane_mode(pane_id)
            .map(TermMode::from_bits_truncate)
    }

    /// Drain the notification buffer and invoke `handler` on each notification.
    ///
    /// Takes the buffer from `self` to avoid borrow conflicts (the handler
    /// gets `&mut Self` without conflicting with the buffer), then restores
    /// it afterward to preserve `Vec` capacity across frames.
    fn with_drained_notifications(&mut self, mut handler: impl FnMut(&mut Self, MuxNotification)) {
        let mut buf = std::mem::take(&mut self.notification_buf);
        let count = buf.len();
        #[allow(
            clippy::iter_with_drain,
            reason = "drain preserves Vec capacity; into_iter drops it"
        )]
        for n in buf.drain(..) {
            handler(self, n);
        }
        // Shrink if capacity vastly exceeds typical usage.
        let cap = buf.capacity();
        if cap > 4 * count && cap > 4096 {
            buf.shrink_to(count * 2);
        }
        self.notification_buf = buf;
    }

    /// Current tab width lock value, if active.
    ///
    /// Delegates to the tab bar widget — the widget is the single source
    /// of truth for this value.
    pub(super) fn tab_width_lock(&self) -> Option<f32> {
        self.focused_ctx()
            .and_then(|ctx| ctx.tab_bar.tab_width_lock())
    }

    /// Freeze tab widths at `width` to prevent layout jitter.
    pub(super) fn acquire_tab_width_lock(&mut self, width: f32) {
        if let Some(ctx) = self.focused_ctx_mut() {
            ctx.tab_bar.set_tab_width_lock(Some(width));
        }
    }

    /// Restore the mouse cursor if it was hidden by typing auto-hide.
    ///
    /// Called on mouse move, cursor leave, and focus loss to ensure the
    /// OS cursor is visible again. Skips the winit call when the cursor
    /// is already visible to avoid redundant system calls.
    fn restore_mouse_cursor(&mut self, winit_id: WindowId) {
        if self.mouse_cursor_hidden {
            self.mouse_cursor_hidden = false;
            if let Some(ctx) = self.windows.get(&winit_id) {
                ctx.window.window().set_cursor_visible(true);
            }
        }
    }

    /// Release the tab width lock, allowing tabs to recompute widths.
    pub(super) fn release_tab_width_lock(&mut self) {
        if self.tab_width_lock().is_some() {
            if let Some(ctx) = self.focused_ctx_mut() {
                ctx.tab_bar.set_tab_width_lock(None);
                ctx.root.mark_dirty();
            }
        }
    }
}

#[cfg(test)]
mod tests;
