//! App constructors for embedded and daemon modes.

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::AtomicU64;
use std::time::{Duration, Instant};

use winit::event_loop::EventLoopProxy;
use winit::keyboard::ModifiersState;

use oriterm_mux::backend::MuxBackend;

use super::event_loop_helpers::resolve_ui_theme;
use super::keyboard_input::ImeState;
use super::mouse_selection::MouseState;
use super::perf_stats::PerfStats;
use super::{App, EventSender};
use crate::clipboard::Clipboard;
use crate::config::Config;
use crate::config::monitor::ConfigMonitor;
use crate::event::TermEvent;
use crate::keybindings;
use crate::session::SessionRegistry;
use crate::window_manager::WindowManager;
use oriterm_ui::animation::CursorBlink;

/// Daemon-mode session startup parameters for [`App::new_daemon`].
///
/// Bundles the daemon socket and the claimed-window startup state (window id,
/// tabs JSON, initial position) — distinct from the event proxy, config, and
/// debug flags.
pub(crate) struct DaemonSession<'a> {
    /// Path to the running `oriterm-mux` daemon socket.
    pub socket_path: &'a std::path::Path,
    /// Existing mux window id to claim, if any.
    pub window_id: Option<u64>,
    /// Serialized tab state to restore on the claimed window.
    pub tabs_json: Option<String>,
    /// Initial window position spec (e.g. `"x,y"`).
    pub position: Option<&'a str>,
}

impl App {
    /// Create a new application instance in daemon mode.
    ///
    /// Instead of an embedded mux, connects to a running `oriterm-mux`
    /// daemon at `socket_path`. If `window_id` is provided, claims an
    /// existing mux window; otherwise creates a new one during init.
    pub(crate) fn new_daemon(
        event_proxy: EventLoopProxy<TermEvent>,
        config: Config,
        session: DaemonSession<'_>,
        profiling: bool,
        latency_log: bool,
    ) -> Self {
        let DaemonSession {
            socket_path,
            window_id,
            tabs_json,
            position,
        } = session;
        let mux_wakeup = make_mux_wakeup(&event_proxy);

        let mux: Option<Box<dyn MuxBackend>> =
            match oriterm_mux::MuxClient::connect(socket_path, mux_wakeup) {
                Ok(client) => {
                    log::info!("daemon mode: connected to {}", socket_path.display());
                    Some(Box::new(client))
                }
                Err(e) => {
                    log::error!(
                        "failed to connect to daemon at {}: {e}",
                        socket_path.display()
                    );
                    None
                }
            };

        let mut app = Self::build_common(event_proxy, config, mux, profiling, latency_log);

        // Store the claimed window ID so init can use it instead of creating one.
        if let Some(wid) = window_id {
            app.active_window = Some(crate::session::WindowId::from_raw(wid));
        }
        app.claimed_tabs = tabs_json;
        app.initial_position = position.and_then(parse_position);

        app
    }

    /// Create a new application instance.
    ///
    /// All GPU/window/tab state is `None` until [`resumed`] is called by
    /// the event loop (lazy initialization pattern from winit docs).
    pub(crate) fn new(
        event_proxy: EventLoopProxy<TermEvent>,
        config: Config,
        profiling: bool,
        latency_log: bool,
    ) -> Self {
        let (builtin_count, user_count) = crate::scheme::discover_count();
        log::info!(
            "themes: {} available ({} built-in, {} user)",
            builtin_count + user_count,
            builtin_count,
            user_count,
        );

        let mux_wakeup = make_mux_wakeup(&event_proxy);
        let mux = oriterm_mux::EmbeddedMux::new(mux_wakeup);

        Self::build_common(
            event_proxy,
            config,
            Some(Box::new(mux)),
            profiling,
            latency_log,
        )
    }

    /// Create a new application instance from a Windows console handoff
    /// payload (Section 03.9 Phase 4 — `-Embedding` startup path).
    ///
    /// Identical to [`App::new`] except the resulting `App` carries a
    /// pending [`HandoffData`](crate::platform::default_terminal::handoff::HandoffData)
    /// that [`try_init`](super::App::try_init) consumes by calling
    /// `EmbeddedMux::adopt_pane` for the initial pane instead of
    /// `spawn_pane`. The adopted pane uses the pre-existing pipe handles
    /// from `conhost.exe` rather than spawning a fresh shell.
    ///
    /// Windows-only — the entire COM handoff path has no analogue on
    /// other platforms.
    #[cfg(target_os = "windows")]
    pub(crate) fn new_handoff(
        event_proxy: EventLoopProxy<TermEvent>,
        config: Config,
        handoff: crate::platform::default_terminal::handoff::HandoffData,
        profiling: bool,
        latency_log: bool,
    ) -> Self {
        let mut app = Self::new(event_proxy, config, profiling, latency_log);
        app.handoff_pending = Some(handoff);
        app
    }

    /// Shared constructor logic: build bindings, config monitor, UI theme,
    /// and the common struct fields.
    fn build_common(
        event_proxy: EventLoopProxy<TermEvent>,
        config: Config,
        mux: Option<Box<dyn MuxBackend>>,
        profiling: bool,
        latency_log: bool,
    ) -> Self {
        let bindings = keybindings::merge_bindings(&config.keybind);
        let config_proxy = event_proxy.clone();
        let monitor = ConfigMonitor::new(Arc::new(move || {
            let _ = config_proxy.send_event(TermEvent::ConfigReload);
        }));
        let blink_interval = Duration::from_millis(config.terminal.cursor_blink_interval_ms);
        let text_blink_interval = Duration::from_millis(config.terminal.text_blink_rate_ms);
        let ui_theme = resolve_ui_theme(&config);
        let event_sender = EventSender(Arc::new(move |ev| {
            let _ = event_proxy.send_event(ev);
        }));

        Self {
            gpu: None,
            pipelines: None,
            gpu_health: crate::gpu::recovery::GpuHealth::new(),
            device_lost_signal: Arc::new(AtomicU64::new(0)),
            last_seen_device_lost_signal: 0,
            font_set: None,
            user_fallback_map: Vec::new(),
            window_manager: WindowManager::new(),
            windows: HashMap::new(),
            dialogs: HashMap::new(),
            focused_window_id: None,
            session: SessionRegistry::new(),
            mux,
            active_window: None,
            notification_buf: Vec::new(),
            modifiers: ModifiersState::empty(),
            cursor_blink: CursorBlink::new(blink_interval),
            text_blink: CursorBlink::new(text_blink_interval),
            mouse_cursor_hidden: false,
            blinking_active: false,
            blink_wakeup_gen: Arc::new(AtomicU64::new(0)),
            next_blink_gen: 1,
            last_cursor_pos: (0, 0),
            font_catalog_prewarm_started: false,
            mouse: MouseState::new(),
            pane_selections: HashMap::new(),
            mark_cursors: HashMap::new(),
            clipboard: Clipboard::new(),
            event_proxy: event_sender,
            config,
            bindings,
            _config_monitor: monitor,
            ime: ImeState::new(),
            ui_theme,
            pending_dropdown_id: None,
            pending_focus_out: None,

            torn_off_pending: None,

            pending_destroy: Vec::new(),
            scratch_dirty_windows: Vec::new(),
            scratch_pane_sels: HashMap::new(),
            scratch_pane_mcs: HashMap::new(),
            last_render: Instant::now(),
            perf: PerfStats::new(profiling, latency_log),
            debug_overlay_enabled: false,
            debug_fps: 0.0,
            claimed_tabs: None,
            initial_position: None,
            #[cfg(target_os = "windows")]
            handoff_pending: None,
        }
    }
}

/// Parse a "x,y" string into (i32, i32).
fn parse_position(s: &str) -> Option<(i32, i32)> {
    let mut parts = s.split(',');
    let x = parts.next()?.parse().ok()?;
    let y = parts.next()?.parse().ok()?;
    Some((x, y))
}

/// Build the mux-wakeup callback used by both [`App::new`] (embedded mode)
/// and [`App::new_daemon`] (daemon mode).
///
/// Returns the `Arc<dyn Fn() + Send + Sync>` shape that
/// `EmbeddedMux::new` and `MuxClient::connect` accept. The closure
/// signals the winit event loop via `EventLoopProxy::send_event(TermEvent::MuxWakeup)`,
/// which the App's `about_to_wait` handler then drains via `pump_mux_events`.
///
/// `let _ = proxy.send_event(...)` is intentional: the wakeup is a
/// performance hint (skip `try_recv` on idle iterations), NOT a
/// correctness primitive — if the event loop is closed, the next
/// `pump_mux_events` cycle still drains the underlying channel via
/// `poll_events`. Byte-loss safety lives at the `MuxEvent` channel
/// layer, not at the wakeup signal.
fn make_mux_wakeup(proxy: &EventLoopProxy<TermEvent>) -> Arc<dyn Fn() + Send + Sync> {
    let proxy = proxy.clone();
    Arc::new(move || {
        let _ = proxy.send_event(TermEvent::MuxWakeup);
    })
}
