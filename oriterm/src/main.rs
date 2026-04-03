//! Binary entry point for the oriterm terminal emulator.
//!
//! Builds a winit event loop and runs the [`App`] as the application handler.
//! All initialization (GPU, window, fonts, tab) happens lazily inside
//! [`App::resumed`] when the event loop first becomes active.

// GUI application — no console window on Windows.
#![windows_subsystem = "windows"]

#[cfg(feature = "profile")]
mod alloc;

#[cfg(feature = "profile")]
#[global_allocator]
#[allow(unsafe_code)]
static GLOBAL: alloc::CountingAlloc = alloc::CountingAlloc;

mod app;
mod cli;
mod clipboard;
mod config;
mod event;
mod font;
mod gpu;
mod key_encoding;
mod keybindings;
mod platform;
mod scheme;
mod session;
mod url_detect;
mod widgets;
mod window;
mod window_manager;

use clap::Parser;

use crate::config::{Config, ProcessModel};
use crate::event::TermEvent;

fn main() {
    let args = cli::Cli::parse();

    // CLI subcommands run headlessly — no window, no event loop.
    if let Some(cmd) = args.command {
        cli::attach_console();
        cli::dispatch(cmd);
    }

    init_logger();
    log::info!("oriterm {}", env!("ORITERM_VERSION"));
    install_panic_hook();

    // Ensure the config directory exists before any config I/O.
    if let Err(e) = platform::config_paths::ensure_config_dir() {
        log::warn!("failed to create config directory: {e}");
    }

    if args.new_window {
        log::info!("--new-window requested");
    }
    if args.new_tab {
        log::info!("--new-tab requested");
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

    let mut app = if let Some(ref socket) = args.connect {
        // Explicit --connect always uses daemon mode (regardless of config).
        app::App::new_daemon(proxy, config, socket, args.window, profiling)
    } else if embedded {
        log::info!("embedded mode (config or --embedded flag)");
        app::App::new(proxy, config, profiling)
    } else {
        // Daemon mode with retry + fallback.
        match ensure_daemon_with_retry() {
            Ok(socket_path) => app::App::new_daemon(proxy, config, &socket_path, None, profiling),
            Err(e) => {
                log::warn!("daemon auto-start failed after retries, using embedded mode: {e}");
                app::App::new(proxy, config, profiling)
            }
        }
    };

    if let Err(e) = event_loop.run_app(&mut app) {
        log::error!("event loop error: {e}");
    }
}

/// Initialize a minimal file logger next to the executable.
///
/// Writes to `oriterm.log` in the same directory as the binary.
/// This avoids needing an external logging crate while still capturing
/// errors from the GUI-subsystem binary (which has no console).
fn init_logger() {
    use std::io::Write;
    use std::sync::Mutex;

    struct FileLogger(Mutex<std::fs::File>);

    impl log::Log for FileLogger {
        fn enabled(&self, metadata: &log::Metadata<'_>) -> bool {
            // Only log our crate's messages, not wgpu/naga noise.
            metadata.target().starts_with("oriterm")
        }

        fn log(&self, record: &log::Record<'_>) {
            if !self.enabled(record.metadata()) {
                return;
            }
            if let Ok(mut f) = self.0.lock() {
                let _ = writeln!(f, "[{}] {}", record.level(), record.args());
            }
        }

        fn flush(&self) {
            if let Ok(f) = self.0.lock() {
                let _ = Write::flush(&mut &*f);
            }
        }
    }

    let path = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.join("oriterm.log")))
        .unwrap_or_else(|| std::path::PathBuf::from("oriterm.log"));

    if let Ok(file) = std::fs::File::create(&path) {
        let logger = Box::new(FileLogger(Mutex::new(file)));
        if log::set_logger(Box::leak(logger)).is_ok() {
            let rust_log = std::env::var("RUST_LOG").ok();
            let level = rust_log
                .as_deref()
                .and_then(|s| s.parse().ok())
                .unwrap_or(log::LevelFilter::Info);
            log::set_max_level(level);
            log::info!("log level: {level} (RUST_LOG={rust_log:?})");
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
            // Log just the first line to confirm backtrace is present;
            // the full backtrace is too noisy for the log. The important
            // info is in the panic message itself.
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

    let tasks = platform::jump_list::build_jump_list_tasks();
    if let Err(e) = platform::jump_list::submit_jump_list(&tasks) {
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
