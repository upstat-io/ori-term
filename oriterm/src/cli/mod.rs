//! CLI subcommands for headless diagnostics.
//!
//! Provides `ls-fonts`, `show-keys`, `list-themes`, `validate-config`,
//! `show-config`, and `completions` subcommands that run without opening a
//! window. Standard in modern terminals (`WezTerm` `ls-fonts`, Ghostty
//! `+list-fonts`).

use std::fmt::Write;
use std::process;

use clap::{CommandFactory, Parser, Subcommand};
use clap_complete::Shell;

use crate::config::Config;
use crate::font::discovery;
use crate::keybindings;

/// GPU-accelerated terminal emulator.
#[derive(Parser)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "CLI argument struct — boolean flags are the standard clap pattern"
)]
#[command(
    name = "oriterm",
    version = env!("ORITERM_VERSION"),
    long_version = env!("ORITERM_VERSION"),
    about
)]
pub(crate) struct Cli {
    /// Subcommand to run (omit to launch the terminal).
    #[command(subcommand)]
    pub command: Option<SubCommand>,

    /// Connect to a running mux daemon at this socket path.
    ///
    /// Instead of running an embedded mux, the terminal connects to
    /// an existing `oriterm-mux` daemon for multiplexer state. Used
    /// together with `--window` for cross-process tab migration.
    #[arg(long)]
    pub connect: Option<std::path::PathBuf>,

    /// Claim an existing mux window ID (used with `--connect`).
    ///
    /// When connecting to a daemon that already has a window allocated,
    /// pass its numeric ID here. The terminal will render that window
    /// instead of creating a new one.
    #[arg(long, requires = "connect")]
    pub window: Option<u64>,

    /// Serialized tab state for the claimed window (base64 JSON).
    ///
    /// Used during cross-process tab migration to transfer the session
    /// layout from the source process to the new window process.
    #[arg(long, requires = "window")]
    pub tabs_json: Option<String>,

    /// Initial window position as "x,y" (used for tear-off).
    ///
    /// Passed to the new window process to ensure it appears at the
    /// same coordinates as the tab drag/tear-off origin.
    #[arg(long)]
    pub position: Option<String>,

    /// Open a new window (default when daemon is running).
    #[arg(long)]
    pub new_window: bool,

    /// Open a new tab in an existing window (or new window if none exists).
    ///
    /// When the IPC daemon is running, sends a "new tab" request to the
    /// existing instance. Without a daemon, launches a new window with one
    /// tab (same as default behavior).
    #[arg(long)]
    pub new_tab: bool,

    /// Force embedded (single-process) mode, ignoring config.
    ///
    /// Bypasses daemon auto-start entirely. Useful for debugging,
    /// CI testing, or environments where daemon spawning isn't possible.
    #[arg(long)]
    pub embedded: bool,

    /// Enable profiling output (frame timing, allocation counts).
    ///
    /// Logs performance statistics at `info` level so they appear in
    /// `oriterm.log` without setting `RUST_LOG=debug`. Build with
    /// `--features profile` to also enable per-allocation counting.
    #[arg(long)]
    pub profile: bool,

    /// Write per-keypress latency to a CSV file next to the binary.
    ///
    /// Records `timestamp_ms, event_to_present_ms` for every keypress.
    /// Use for measuring input-to-display latency. The file is created
    /// at `<binary_dir>/oriterm-latency.csv`.
    #[arg(long)]
    pub latency_log: bool,
}

/// Diagnostic subcommands that run headlessly.
#[derive(Subcommand)]
pub(crate) enum SubCommand {
    /// List discovered fonts and fallback chain.
    LsFonts(LsFontsArgs),
    /// Dump current keybindings.
    ShowKeys(ShowKeysArgs),
    /// List available color themes.
    ListThemes(ListThemesArgs),
    /// Validate the config file without launching.
    ValidateConfig,
    /// Dump the resolved config (defaults + user overrides) as TOML.
    ShowConfig,
    /// Generate shell completion scripts.
    Completions(CompletionsArgs),
    /// Register `oriterm` as the default terminal on Windows.
    ///
    /// Writes the conhost delegation selectors and the COM
    /// `LocalServer32` registration so launching `cmd.exe` /
    /// `powershell.exe` from Explorer or the Run dialog opens in
    /// `oriterm`. Windows-only — prints "not supported" + exits 1
    /// on Linux/macOS.
    RegisterDefault,
    /// Remove `oriterm`'s default-terminal registration on Windows.
    ///
    /// Idempotent — runs to completion even if `oriterm` was never
    /// registered. Windows-only — prints "not supported" + exits 1
    /// on Linux/macOS.
    UnregisterDefault,
}

/// Arguments for the `ls-fonts` subcommand.
#[derive(Parser)]
pub(crate) struct LsFontsArgs {
    /// Show which font resolves a specific character.
    #[arg(long)]
    codepoint: Option<char>,
}

/// Arguments for the `show-keys` subcommand.
#[derive(Parser)]
pub(crate) struct ShowKeysArgs {
    /// Show only built-in default bindings (ignore user config).
    #[arg(long)]
    default: bool,
}

/// Arguments for the `list-themes` subcommand.
#[derive(Parser)]
pub(crate) struct ListThemesArgs {
    /// Print a 16-color sample for each theme.
    #[arg(long)]
    preview: bool,
}

/// Arguments for the `completions` subcommand.
#[derive(Parser)]
pub(crate) struct CompletionsArgs {
    /// Shell to generate completions for.
    #[arg(value_enum)]
    pub shell: Shell,
}

/// Attach to the parent console on Windows so CLI output is visible.
///
/// The `#![windows_subsystem = "windows"]` attribute suppresses the console.
/// CLI subcommands need to write to the parent terminal.
pub(crate) fn attach_console() {
    #[cfg(windows)]
    {
        // SAFETY: `AttachConsole` is a standard Win32 API. Passing
        // `ATTACH_PARENT_PROCESS` attaches to the console of the parent
        // process (e.g. cmd.exe / PowerShell). Failure is harmless —
        // output just won't be visible.
        #[allow(unsafe_code)]
        unsafe {
            windows_sys::Win32::System::Console::AttachConsole(
                windows_sys::Win32::System::Console::ATTACH_PARENT_PROCESS,
            );
        }
    }
}

/// Dispatch a CLI subcommand. Prints to stdout and exits.
pub(crate) fn dispatch(cmd: SubCommand) -> ! {
    match cmd {
        SubCommand::LsFonts(args) => run_ls_fonts(&args),
        SubCommand::ShowKeys(args) => run_show_keys(&args),
        SubCommand::ListThemes(args) => run_list_themes(&args),
        SubCommand::ValidateConfig => run_validate_config(),
        SubCommand::ShowConfig => run_show_config(),
        SubCommand::Completions(args) => run_completions(&args),
        SubCommand::RegisterDefault => run_register_default(),
        SubCommand::UnregisterDefault => run_unregister_default(),
    }
}

/// `ls-fonts` — list discovered fonts with fallback chain.
fn run_ls_fonts(args: &LsFontsArgs) -> ! {
    let config = Config::load();
    let weight = config.font.effective_weight();
    let bold_weight = config.font.effective_bold_weight();
    let result = discovery::discover_fonts(config.font.family.as_deref(), weight, bold_weight);

    let mut out = String::new();
    let _ = writeln!(out, "Primary font family: {}", result.primary.family_name);
    let _ = writeln!(out, "  Origin: {:?}", result.primary.origin);

    let labels = ["Regular", "Bold", "Italic", "Bold Italic"];
    for (i, label) in labels.iter().enumerate() {
        if result.primary.has_variant[i] {
            let path_str = result.primary.paths[i]
                .as_ref()
                .map_or_else(|| "(embedded)".to_owned(), |p| p.display().to_string());
            let _ = writeln!(out, "  {label}: {path_str}");
        } else {
            let _ = writeln!(out, "  {label}: (synthesized)");
        }
    }

    if !result.fallbacks.is_empty() {
        let _ = writeln!(out, "\nFallback chain:");
        for (i, fb) in result.fallbacks.iter().enumerate() {
            let _ = writeln!(
                out,
                "  {}. {} (index {})",
                i + 1,
                fb.path.display(),
                fb.face_index
            );
        }
    }

    if let Some(ch) = args.codepoint {
        let _ = writeln!(out, "\nCodepoint U+{:04X} ({ch}):", ch as u32);
        let _ = writeln!(
            out,
            "  (font resolution requires loading — run the terminal to test)"
        );
    }

    print!("{out}");
    process::exit(0)
}

/// `show-keys` — dump keybindings in human-readable format.
fn run_show_keys(args: &ShowKeysArgs) -> ! {
    let bindings = if args.default {
        keybindings::default_bindings()
    } else {
        let config = Config::load();
        keybindings::merge_bindings(&config.keybind)
    };

    let mut out = String::new();
    let source = if args.default { "Default" } else { "Active" };
    let _ = writeln!(out, "{source} keybindings:\n");

    for b in &bindings {
        let _ = writeln!(out, "  {}", format_binding(b));
    }

    print!("{out}");
    process::exit(0)
}

/// `list-themes` — list available color schemes.
fn run_list_themes(args: &ListThemesArgs) -> ! {
    let mut out = String::new();
    let _ = writeln!(out, "Available themes:\n");
    let _ = writeln!(out, "  * Catppuccin Mocha (default)");

    if args.preview {
        let _ = writeln!(out);
        let _ = writeln!(out, "  16-color palette:");
        let _ = write!(out, "  ");
        // Standard ANSI colors 0-7.
        for i in 0..8u8 {
            let _ = write!(out, "\x1b[48;5;{i}m  ");
        }
        let _ = writeln!(out, "\x1b[0m");
        let _ = write!(out, "  ");
        // Bright colors 8-15.
        for i in 8..16u8 {
            let _ = write!(out, "\x1b[48;5;{i}m  ");
        }
        let _ = writeln!(out, "\x1b[0m");
    }

    print!("{out}");
    process::exit(0)
}

/// `validate-config` — parse and validate the config file, exit 0 or 1.
fn run_validate_config() -> ! {
    let exit_code = match validate_config_inner() {
        Ok(()) => {
            println!("config: valid");
            0
        }
        Err(errors) => {
            for e in &errors {
                eprintln!("error: {e}");
            }
            1
        }
    };
    process::exit(exit_code)
}

/// Core validation logic, separated for testability.
///
/// Returns `Ok(())` when the config is valid, or a list of error messages.
fn validate_config_inner() -> Result<(), Vec<String>> {
    let config = match Config::try_load() {
        Ok(c) => c,
        Err(e) => return Err(vec![e]),
    };

    let mut errors = Vec::new();
    validate_colors(&config, &mut errors);
    validate_keybindings(&config, &mut errors);

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

/// `show-config` — dump the resolved config as TOML.
fn run_show_config() -> ! {
    let config = Config::load();
    match toml::to_string_pretty(&config) {
        Ok(toml) => {
            print!("{toml}");
            process::exit(0);
        }
        Err(e) => {
            eprintln!("error: failed to serialize config: {e}");
            process::exit(1);
        }
    }
}

/// `register-default` — register `oriterm` as the Windows default
/// terminal handler. Windows-only — prints "not supported" + exit 1
/// elsewhere so the cross-platform binary still parses + dispatches
/// without panicking.
fn run_register_default() -> ! {
    #[cfg(windows)]
    {
        let exit_code = match register_default_inner() {
            Ok(()) => {
                println!("oriterm: registered as the default Windows terminal");
                0
            }
            Err(e) => {
                eprintln!("oriterm: failed to register as default terminal: {e}");
                1
            }
        };
        process::exit(exit_code);
    }
    #[cfg(not(windows))]
    {
        eprintln!("oriterm: --register-default is only supported on Windows");
        process::exit(1);
    }
}

/// Resolve the current executable path and call into the registry
/// helpers. Extracted for testability — tests can call the helper
/// without running the dispatcher's `process::exit`.
#[cfg(windows)]
fn register_default_inner() -> std::io::Result<()> {
    let exe_path = std::env::current_exe()?;
    crate::platform::default_terminal::registry::register_all(&exe_path)
}

/// `unregister-default` — remove the Windows default-terminal
/// registration. Idempotent — succeeds even when no registration is
/// present.
fn run_unregister_default() -> ! {
    #[cfg(windows)]
    {
        let exit_code = match crate::platform::default_terminal::registry::unregister_all() {
            Ok(()) => {
                println!("oriterm: removed default-terminal registration");
                0
            }
            Err(e) => {
                eprintln!("oriterm: failed to unregister default terminal: {e}");
                1
            }
        };
        process::exit(exit_code);
    }
    #[cfg(not(windows))]
    {
        eprintln!("oriterm: --unregister-default is only supported on Windows");
        process::exit(1);
    }
}

/// `completions` — generate shell completion script for the given shell.
fn run_completions(args: &CompletionsArgs) -> ! {
    use std::io::IsTerminal;

    let mut cmd = Cli::command();
    clap_complete::generate(args.shell, &mut cmd, "oriterm", &mut std::io::stdout());

    // When output goes to a terminal (not redirected to a file), print
    // install instructions on stderr so the user knows what to do.
    if std::io::stdout().is_terminal() {
        eprintln!();
        match args.shell {
            Shell::Bash => {
                eprintln!("# To install, run:");
                eprintln!(
                    "#   oriterm completions bash > ~/.local/share/bash-completion/completions/oriterm"
                );
            }
            Shell::Zsh => {
                eprintln!("# To install, add to your fpath and run:");
                eprintln!("#   oriterm completions zsh > ~/.zfunc/_oriterm");
                eprintln!("#   echo 'fpath=(~/.zfunc $fpath)' >> ~/.zshrc");
            }
            Shell::Fish => {
                eprintln!("# To install, run:");
                eprintln!("#   oriterm completions fish > ~/.config/fish/completions/oriterm.fish");
            }
            Shell::PowerShell => {
                eprintln!("# To install, add to your PowerShell profile:");
                eprintln!("#   oriterm completions powershell >> $PROFILE");
            }
            _ => {}
        }
    }

    process::exit(0)
}

/// Generate completion script into a byte buffer (for testing).
#[cfg(test)]
fn generate_completions(shell: Shell) -> Vec<u8> {
    let mut cmd = Cli::command();
    let mut buf = Vec::new();
    clap_complete::generate(shell, &mut cmd, "oriterm", &mut buf);
    buf
}

mod format;
mod validate;
use format::format_binding;
use validate::{validate_colors, validate_keybindings};

#[cfg(test)]
mod tests;
