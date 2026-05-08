//! Application struct and winit event loop handler.
//!
//! [`App`] implements winit's [`ApplicationHandler`] to drive the terminal.
//! It wires together the three-phase rendering pipeline (Extract → Prepare →
//! Render), handles window events, and dispatches terminal events from the
//! PTY reader thread.

mod chrome;
#[cfg(all(test, feature = "gpu-tests"))]
pub(crate) use chrome::compute_window_layout;
mod cell_metrics;
mod clipboard_ops;
pub(crate) mod config_reload;
mod constructors;
mod context_menu;
mod cursor_hover;
pub(crate) mod dialog_context;
pub(crate) mod dialog_management;
mod dialog_rendering;
mod divider_drag;
mod dpi_change;
mod dropdown_popup;
mod event_loop;
mod event_loop_helpers;
mod floating_drag;
mod focus_accessors;
mod gpu_recovery;
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
mod post_spawn;
mod redraw;
mod render_dispatch;
mod search_ui;
mod settings_overlay;
pub(crate) mod snapshot_grid;
mod tab_bar_input;
mod tab_drag;
mod tab_management;
#[cfg(test)]
pub(crate) mod test_support;
mod widget_pipeline;
pub(crate) mod window_context;
mod window_management;

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::AtomicU64;
use std::time::{Duration, Instant};

use winit::keyboard::ModifiersState;
use winit::window::WindowId;

use oriterm_core::Selection;
use oriterm_mux::{MarkCursor, PaneId};

use crate::session::{SessionRegistry, WindowId as SessionWindowId};
use crate::window_manager::WindowManager;

use self::dialog_context::DialogWindowContext;
use self::event_loop_helpers::{resolve_ui_theme, winit_mods_to_ui};
use self::keyboard_input::ImeState;
use self::mouse_selection::MouseState;
use self::perf_stats::PerfStats;
use self::window_context::WindowContext;
use crate::clipboard::Clipboard;
use crate::config::Config;
use crate::config::monitor::ConfigMonitor;
use crate::event::TermEvent;
use crate::font::FontSet;
use crate::gpu::{GpuPipelines, GpuState};
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
#[derive(Clone)]
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
#[expect(
    clippy::struct_excessive_bools,
    reason = "App carries 4 orthogonal one-shot/cached flags (mouse_cursor_hidden, \
              blinking_active, font_catalog_prewarm_started, debug_overlay_enabled); \
              they have unrelated lifecycles and no natural grouping into a state-machine enum"
)]
pub(crate) struct App {
    // GPU + rendering (lazy init on Resumed).
    gpu: Option<GpuState>,
    /// Shared stateless GPU pipelines and bind group layouts.
    pipelines: Option<GpuPipelines>,
    /// Global GPU device health.
    ///
    /// The render gate (5.16.2) consults this before submitting any draw
    /// work — `Recovering` and `Unavailable` block rendering. The
    /// `App::recover_gpu()` state machine (5.16.2) is the only mutator;
    /// 5.16.1 only adds the field with the default `Healthy { epoch: 0 }`
    /// and routes detection events into a logging stub.
    gpu_health: crate::gpu::recovery::GpuHealth,
    /// Cross-thread "device-lost callback fired" counter.
    ///
    /// Bumped by the closure registered with
    /// `wgpu::Device::set_device_lost_callback` whenever the underlying
    /// device dies. The render path samples this in `finish_render` and
    /// compares against `last_seen_device_lost_signal` to detect a loss
    /// that occurred *between* submit and present — the case where the
    /// callback ran on a wgpu thread mid-frame and the event has not yet
    /// reached `user_event`. 5.16.2's render gate uses the same signal to
    /// short-circuit before re-entering submit.
    device_lost_signal: Arc<AtomicU64>,
    /// Last value of `device_lost_signal` observed on the main thread.
    ///
    /// Updated by `finish_render` after each frame so the next frame can
    /// detect any new increments. A delta means the device-lost callback
    /// fired since the last frame.
    last_seen_device_lost_signal: u64,
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

    // Text blink timer for SGR 5/6 blinking text.
    // Runs continuously (not gated by per-cell flags — the timer is cheap).
    text_blink: CursorBlink,

    // Whether the OS mouse cursor is currently hidden (typing auto-hide).
    mouse_cursor_hidden: bool,

    // Whether the terminal's CURSOR_BLINKING mode is active.
    // Cached from the last extracted frame to gate blink timer in about_to_wait.
    blinking_active: bool,

    // Generation counter for blink wakeup thread deduplication.
    // 0 = no pending thread. Nonzero = generation of the pending thread.
    // CAS ensures stale threads don't clear the flag after a reset+respawn.
    blink_wakeup_gen: Arc<AtomicU64>,
    // Monotonic generation counter (main thread only, never zero).
    next_blink_gen: u64,

    // Last cursor position (line, column) for blink-reset-on-move detection.
    // Compared per frame; reset blink when the cursor moves due to PTY output.
    last_cursor_pos: (usize, usize),

    // Whether the post-first-render font-catalog prewarm thread has been
    // spawned. One-shot — flipped on the first Ok render result so monospace
    // family enumeration is amortized off the UI thread before the Settings
    // dialog can demand it. Decoupled from startup intentionally: spawning
    // during init competes with first-frame rendering for CPU/IO bandwidth.
    font_catalog_prewarm_started: bool,

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

    // Debug performance overlay toggle (Ctrl+Shift+F12).
    debug_overlay_enabled: bool,
    // EWMA-smoothed FPS for the debug overlay display.
    debug_fps: f32,

    // Pending Windows console handoff payload (Section 03.9 Phase 3).
    //
    // Set by `App::new_handoff` when entered via the `-Embedding` COM
    // server path. Consumed by `try_init` to construct the initial pane
    // via `EmbeddedMux::adopt_pane` instead of `spawn_pane`. Always
    // `None` on Linux/macOS and on the normal Windows startup path.
    #[cfg(target_os = "windows")]
    handoff_pending: Option<crate::platform::default_terminal::handoff::HandoffData>,
}

#[cfg(test)]
mod tests;
