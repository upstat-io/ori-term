//! Application entry point.
//!
//! Contains the startup sequence for the terminal emulator. Called from
//! `main.rs` via the public [`run()`] function.

use clap::Parser;

use crate::config::{Config, ProcessModel};
use crate::event::TermEvent;

/// Run the terminal emulator.
///
/// Parses CLI arguments, initializes logging, sets up the event loop,
/// and runs the application. This is the main entry point called by the
/// binary.
pub fn run() {
    // Intercept `-Embedding` BEFORE clap — conhost.exe passes only that
    // flag for default-terminal activation and clap would reject it.
    #[cfg(target_os = "windows")]
    if has_embedding_arg() {
        run_default_terminal_handoff();
        return;
    }

    let args = crate::cli::Cli::parse();

    // CLI subcommands run headlessly — no window, no event loop.
    if let Some(cmd) = args.command {
        crate::cli::attach_console();
        crate::cli::dispatch(cmd);
    }

    init_logger();
    log::info!("oriterm {}", env!("ORITERM_VERSION"));
    install_panic_hook();

    // Ensure the config directory exists before any config I/O.
    if let Err(e) = crate::platform::config_paths::ensure_config_dir() {
        log::warn!("failed to create config directory: {e}");
    }

    if args.new_window {
        log::info!("--new-window requested");
    }
    if args.new_tab {
        log::info!("--new-tab requested");
        if try_relay_new_tab() {
            return;
        }
    }
    #[cfg(windows)]
    set_app_user_model_id();

    let event_loop = build_event_loop();

    #[cfg(windows)]
    submit_jump_list_on_startup();

    let proxy = event_loop.create_proxy();

    let config = Config::load();

    // CLI flag > config for process model decision.
    let embedded = args.embedded || config.process_model == ProcessModel::Embedded;

    let profiling = args.profile;
    if profiling {
        log::info!("profiling mode enabled (--profile)");
    }
    let latency_log = args.latency_log;
    if latency_log {
        log::info!("latency logging enabled (--latency-log)");
    }

    let mut app = if let Some(ref socket) = args.connect {
        // Explicit --connect always uses daemon mode (regardless of config).
        crate::app::App::new_daemon(
            proxy,
            config,
            socket,
            args.window,
            args.tabs_json.clone(),
            args.position.as_deref(),
            profiling,
            latency_log,
        )
    } else if embedded {
        log::info!("embedded mode (config or --embedded flag)");
        crate::app::App::new(proxy, config, profiling, latency_log)
    } else {
        // Daemon mode with retry + fallback.
        match ensure_daemon_with_retry() {
            Ok(socket_path) => crate::app::App::new_daemon(
                proxy,
                config,
                &socket_path,
                None,
                None,
                None,
                profiling,
                latency_log,
            ),
            Err(e) => {
                log::warn!("failed to ensure daemon: {e}, falling back to embedded mode");
                crate::app::App::new(proxy, config, profiling, latency_log)
            }
        }
    };

    if let Err(e) = event_loop.run_app(&mut app) {
        log::error!("event loop error: {e}");
    }
}

/// Detect the conhost-issued `-Embedding` flag.
///
/// Conhost passes `-Embedding` (case-insensitive) as the SOLE
/// command-line argument when activating a registered default terminal
/// via `CoCreateInstance`. We scan for it manually because clap would
/// reject the unknown flag.
#[cfg(target_os = "windows")]
fn has_embedding_arg() -> bool {
    std::env::args()
        .skip(1) // skip argv[0]
        .any(|arg| arg.eq_ignore_ascii_case("-Embedding"))
}

/// Run the COM server lifecycle and hand the resulting payload to a
/// fresh `App::new_handoff` event loop.
///
/// This replaces the entire normal startup path when conhost has
/// activated us as the default terminal. We skip jump list submission
/// and daemon discovery — the COM server runs as a `REGCLS_SINGLEUSE`
/// process that handles exactly one session.
#[cfg(target_os = "windows")]
fn run_default_terminal_handoff() {
    init_logger();
    log::info!(
        "oriterm {} — default-terminal handoff (-Embedding)",
        env!("ORITERM_VERSION")
    );
    install_panic_hook();

    let handoff = match crate::platform::default_terminal::run_com_server() {
        Ok(payload) => payload,
        Err(e) => {
            log::error!("default-terminal handoff failed: {e}");
            std::process::exit(1);
        }
    };

    // No jump list submission — APARTMENTTHREADED COM would conflict
    // with the MULTITHREADED init `run_com_server` already performed.
    let event_loop = build_event_loop();
    let proxy = event_loop.create_proxy();
    let config = Config::load();

    let mut app = crate::app::App::new_handoff(proxy, config, handoff, false, false);
    if let Err(e) = event_loop.run_app(&mut app) {
        log::error!("event loop error: {e}");
    }
}

/// Initialize a minimal file logger next to the executable.
///
/// See: `crate::log_filter::LogFilter` for `RUST_LOG` directive parsing.
///
/// Writes to `oriterm.log` in the same directory as the binary, avoiding an
/// external logging crate while still capturing errors from the GUI-subsystem
/// binary (which has no console). `RUST_LOG` accepts either a bare level
/// (`trace`, `debug`, `info`, `warn`, `error`) or an `env_logger`-style
/// directive list. Non-`oriterm*` targets are silently dropped at parse
/// time to preserve the wgpu/naga noise gate.
fn init_logger() {
    use std::io::Write;
    use std::sync::Mutex;

    struct FileLogger {
        file: Mutex<std::fs::File>,
        filter: crate::log_filter::LogFilter,
    }

    impl log::Log for FileLogger {
        fn enabled(&self, metadata: &log::Metadata<'_>) -> bool {
            self.filter.enabled(metadata.target(), metadata.level())
        }

        fn log(&self, record: &log::Record<'_>) {
            if !self.enabled(record.metadata()) {
                return;
            }
            if let Ok(mut f) = self.file.lock() {
                let _ = writeln!(f, "[{}] {}", record.level(), record.args());
            }
        }

        fn flush(&self) {
            if let Ok(f) = self.file.lock() {
                let _ = Write::flush(&mut &*f);
            }
        }
    }

    let path = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.join("oriterm.log")))
        .unwrap_or_else(|| std::path::PathBuf::from("oriterm.log"));

    if let Ok(file) = std::fs::File::create(&path) {
        let rust_log = std::env::var("RUST_LOG").ok();
        let filter = crate::log_filter::LogFilter::parse(rust_log.as_deref().unwrap_or(""));
        let max_level = filter.max_level();
        let logger = Box::new(FileLogger {
            file: Mutex::new(file),
            filter,
        });
        if log::set_logger(Box::leak(logger)).is_ok() {
            log::set_max_level(max_level);
            log::info!("log max_level: {max_level} (RUST_LOG={rust_log:?})");
        }
    }
}

/// Install a panic hook that writes to the log file before aborting.
///
/// GUI-subsystem binaries on Windows have no console, so panics vanish
/// silently. This hook ensures the backtrace is captured in `oriterm.log`.
fn install_panic_hook() {
    std::panic::set_hook(Box::new(|info| {
        log::error!("PANIC: {info}");
        if let Some(bt) = std::backtrace::Backtrace::force_capture()
            .to_string()
            .lines()
            .take(30)
            .collect::<Vec<_>>()
            .first()
        {
            log::error!("backtrace (first line): {bt}");
        }
    }));
}

/// Try to start the daemon up to 3 times before giving up.
fn ensure_daemon_with_retry() -> std::io::Result<std::path::PathBuf> {
    const MAX_ATTEMPTS: u32 = 3;
    let mut last_err = None;
    for attempt in 1..=MAX_ATTEMPTS {
        match oriterm_mux::discovery::ensure_daemon() {
            Ok(path) => return Ok(path),
            Err(e) => {
                log::warn!("daemon attempt {attempt}/{MAX_ATTEMPTS} failed: {e}");
                last_err = Some(e);
            }
        }
    }
    Err(last_err.unwrap_or_else(|| std::io::Error::other("daemon start failed")))
}

/// Try to relay a new-tab request to an already-running daemon.
///
/// If a daemon is running, sends `RequestNewTab` and returns `true` — the
/// caller should exit. If no daemon is available, returns `false` — the
/// caller should fall through to normal startup.
fn try_relay_new_tab() -> bool {
    let socket_path = oriterm_mux::server::socket_path();
    if !socket_path.exists() {
        log::info!("no daemon socket found, starting new window");
        return false;
    }
    match oriterm_mux::one_shot::request_new_tab(&socket_path) {
        Ok(()) => {
            log::info!("new-tab request relayed to daemon, exiting");
            true
        }
        Err(e) => {
            log::warn!("failed to relay new-tab to daemon: {e}, starting new window");
            false
        }
    }
}

/// Set the Windows App User Model ID for taskbar grouping and Jump Lists.
///
/// Must be called before window creation. Uses the existing `windows-sys`
/// crate since this is a flat Win32 API, not a COM interface.
#[cfg(windows)]
fn set_app_user_model_id() {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;

    let id: Vec<u16> = OsStr::new("Ori.Terminal")
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();

    // SAFETY: `SetCurrentProcessExplicitAppUserModelID` is a standard
    // Win32 API. The wide string is valid and null-terminated.
    #[allow(unsafe_code)]
    let hr = unsafe {
        windows_sys::Win32::UI::Shell::SetCurrentProcessExplicitAppUserModelID(id.as_ptr())
    };
    if hr != 0 {
        log::warn!("SetCurrentProcessExplicitAppUserModelID failed: HRESULT 0x{hr:08x}");
    }
}

/// Initialize COM and submit the Jump List on startup.
///
/// Explicit `CoInitializeEx` is required because winit does not
/// initialize COM until window creation (inside `App::resumed`).
#[cfg(windows)]
fn submit_jump_list_on_startup() {
    use windows::Win32::System::Com::{COINIT_APARTMENTTHREADED, CoInitializeEx};

    // SAFETY: `CoInitializeEx` is a standard Win32 API for COM
    // initialization. The subsequent winit `OleInitialize` call will
    // harmlessly return `S_FALSE` (already initialized).
    #[allow(unsafe_code)]
    let hr = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) };
    if hr.is_err() {
        log::warn!("CoInitializeEx failed: {hr:?}");
        return;
    }

    let tasks = crate::platform::jump_list::build_jump_list_tasks();
    if let Err(e) = crate::platform::jump_list::submit_jump_list(&tasks) {
        log::warn!("jump list submission failed: {e}");
    }
}

/// Build a winit event loop usable from the main thread.
fn build_event_loop() -> winit::event_loop::EventLoop<TermEvent> {
    #[cfg(windows)]
    {
        winit::event_loop::EventLoop::<TermEvent>::with_user_event()
            .build()
            .expect("failed to create event loop")
    }
    #[cfg(target_os = "linux")]
    {
        use winit::platform::x11::EventLoopBuilderExtX11;
        winit::event_loop::EventLoop::<TermEvent>::with_user_event()
            .with_any_thread(true)
            .build()
            .expect("failed to create event loop")
    }
    #[cfg(target_os = "macos")]
    {
        winit::event_loop::EventLoop::<TermEvent>::with_user_event()
            .build()
            .expect("failed to create event loop")
    }
}
