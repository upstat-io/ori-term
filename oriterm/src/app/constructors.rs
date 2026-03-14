//! App constructors for embedded and daemon modes.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use winit::event_loop::EventLoopProxy;
use winit::keyboard::ModifiersState;

use oriterm_mux::backend::MuxBackend;

use super::cursor_blink::CursorBlink;
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

impl App {
    /// Create a new application instance in daemon mode.
    ///
    /// Instead of an embedded mux, connects to a running `oriterm-mux`
    /// daemon at `socket_path`. If `window_id` is provided, claims an
    /// existing mux window; otherwise creates a new one during init.
    pub(crate) fn new_daemon(
        event_proxy: EventLoopProxy<TermEvent>,
        config: Config,
        socket_path: &std::path::Path,
        window_id: Option<u64>,
        profiling: bool,
    ) -> Self {
        let proxy_for_mux = event_proxy.clone();
        let mux_wakeup: Arc<dyn Fn() + Send + Sync> = Arc::new(move || {
            let _ = proxy_for_mux.send_event(TermEvent::MuxWakeup);
        });

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

        let mut app = Self::build_common(event_proxy, config, mux, profiling);

        // Store the claimed window ID so init can use it instead of creating one.
        if let Some(wid) = window_id {
            app.active_window = Some(crate::session::WindowId::from_raw(wid));
        }

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
    ) -> Self {
        let (builtin_count, user_count) = crate::scheme::discover_count();
        log::info!(
            "themes: {} available ({} built-in, {} user)",
            builtin_count + user_count,
            builtin_count,
            user_count,
        );

        let proxy_for_mux = event_proxy.clone();
        let mux_wakeup: Arc<dyn Fn() + Send + Sync> = Arc::new(move || {
            let _ = proxy_for_mux.send_event(TermEvent::MuxWakeup);
        });
        let mux = oriterm_mux::EmbeddedMux::new(mux_wakeup);

        Self::build_common(event_proxy, config, Some(Box::new(mux)), profiling)
    }

    /// Shared constructor logic: build bindings, config monitor, UI theme,
    /// and the common struct fields.
    fn build_common(
        event_proxy: EventLoopProxy<TermEvent>,
        config: Config,
        mux: Option<Box<dyn MuxBackend>>,
        profiling: bool,
    ) -> Self {
        let bindings = keybindings::merge_bindings(&config.keybind);
        let config_proxy = event_proxy.clone();
        let monitor = ConfigMonitor::new(Arc::new(move || {
            let _ = config_proxy.send_event(TermEvent::ConfigReload);
        }));
        let blink_interval = Duration::from_millis(config.terminal.cursor_blink_interval_ms);
        let ui_theme = resolve_ui_theme(&config);
        let event_sender = EventSender(Arc::new(move |ev| {
            let _ = event_proxy.send_event(ev);
        }));

        Self {
            gpu: None,
            pipelines: None,
            font_set: None,
            ui_font_set: None,
            user_fb_count: 0,
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
            mouse_cursor_hidden: false,
            blinking_active: false,
            last_cursor_pos: (0, 0),
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
            settings_ids: None,
            settings_pending: None,
            pending_dropdown_id: None,
            pending_focus_out: None,

            torn_off_pending: None,

            pending_destroy: Vec::new(),
            scratch_dirty_windows: Vec::new(),
            scratch_pane_sels: HashMap::new(),
            scratch_pane_mcs: HashMap::new(),
            last_render: Instant::now(),
            perf: PerfStats::new(profiling),
        }
    }
}
