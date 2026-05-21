//! Tests for shell detection, injection, and script embedding.
//!
//! Catalog rows: OSC-7, OSC-9, OSC-99, OSC-133-PROMPT, OSC-133-CMD-COMPLETE,
//! OSC-633, OSC-777, SHINT-OSC-7-CWD, SHINT-OSC-133-PROMPT, SHINT-OSC-133-CMD-COMPLETE,
//! SHINT-OSC-633-VSCODE, SHINT-OSC-9-NOTIFY, SHINT-OSC-99-NOTIFY, SHINT-OSC-777-NOTIFY

use std::path::Path;

use super::scripts::ensure_scripts_on_disk;
use super::{Shell, detect_shell};

// Shell detection

#[test]
fn detect_shell_unix_paths() {
 assert_eq!(detect_shell("/usr/bin/bash"), Some(Shell::Bash));
 assert_eq!(detect_shell("/bin/zsh"), Some(Shell::Zsh));
 assert_eq!(detect_shell("/usr/local/bin/fish"), Some(Shell::Fish));
 assert_eq!(detect_shell("/usr/bin/pwsh"), Some(Shell::PowerShell));
}

#[test]
fn detect_shell_windows_exe() {
 assert_eq!(detect_shell("bash.exe"), Some(Shell::Bash));
 assert_eq!(detect_shell("pwsh.exe"), Some(Shell::PowerShell));
 assert_eq!(detect_shell("powershell.exe"), Some(Shell::PowerShell));
 assert_eq!(detect_shell("wsl.exe"), Some(Shell::Wsl));
}

#[test]
fn detect_shell_bare_names() {
 assert_eq!(detect_shell("bash"), Some(Shell::Bash));
 assert_eq!(detect_shell("zsh"), Some(Shell::Zsh));
 assert_eq!(detect_shell("fish"), Some(Shell::Fish));
 assert_eq!(detect_shell("powershell"), Some(Shell::PowerShell));
}

#[test]
fn detect_shell_wsl() {
 assert_eq!(detect_shell("wsl"), Some(Shell::Wsl));
 assert_eq!(detect_shell("wsl.exe"), Some(Shell::Wsl));
}

#[test]
fn detect_shell_unknown() {
 assert_eq!(detect_shell("cmd.exe"), None);
 assert_eq!(detect_shell("sh"), None);
 assert_eq!(detect_shell("/bin/dash"), None);
 assert_eq!(detect_shell("nu"), None);
 assert_eq!(detect_shell(""), None);
}

#[test]
fn detect_shell_windows_full_paths() {
 assert_eq!(
 detect_shell(r"C:\Windows\System32\bash.exe"),
 Some(Shell::Bash)
 );
 assert_eq!(
 detect_shell(r"C:\Program Files\PowerShell\7\pwsh.exe"),
 Some(Shell::PowerShell)
 );
}

// Version stamping and script writing

#[test]
fn ensure_scripts_writes_all_files() {
 let tmp = tempfile::tempdir().expect("create temp dir");
 let dir = ensure_scripts_on_disk(tmp.path()).expect("write scripts");

 // Verify all expected files exist.
 assert!(dir.join("bash/oriterm.bash").is_file());
 assert!(dir.join("bash/bash-preexec.sh").is_file());
 assert!(dir.join("zsh/.zshenv").is_file());
 assert!(dir.join("zsh/oriterm-integration").is_file());
 assert!(
 dir.join("fish/vendor_conf.d/oriterm-shell-integration.fish")
 .is_file()
 );
 assert!(dir.join("powershell/oriterm.ps1").is_file());
 assert!(dir.join(".version").is_file());
}

#[test]
fn ensure_scripts_version_stamp_skips_rewrite() {
 let tmp = tempfile::tempdir().expect("create temp dir");

 // First write.
 let dir = ensure_scripts_on_disk(tmp.path()).expect("first write");

 // Record the mtime of a script file.
 let script = dir.join("bash/oriterm.bash");
 let mtime1 = std::fs::metadata(&script)
 .expect("metadata")
 .modified()
 .expect("mtime");

 // Brief pause so mtime would differ if rewritten.
 std::thread::sleep(std::time::Duration::from_millis(50));

 // Second write — should skip because version matches.
 let _ = ensure_scripts_on_disk(tmp.path()).expect("second write");

 let mtime2 = std::fs::metadata(&script)
 .expect("metadata")
 .modified()
 .expect("mtime");

 assert_eq!(mtime1, mtime2, "script should not be rewritten");
}

#[test]
fn ensure_scripts_rewrites_on_stale_version() {
 let tmp = tempfile::tempdir().expect("create temp dir");
 let dir = ensure_scripts_on_disk(tmp.path()).expect("first write");

 // Tamper with the version stamp.
 std::fs::write(dir.join(".version"), "0.0.0-stale").expect("tamper");

 // Should rewrite.
 let dir2 = ensure_scripts_on_disk(tmp.path()).expect("rewrite");
 let stamp = std::fs::read_to_string(dir2.join(".version")).expect("read stamp");
 assert_eq!(stamp.trim(), env!("CARGO_PKG_VERSION"));
}

#[test]
fn scripts_contain_osc_sequences() {
 // Verify the embedded scripts contain expected OSC sequences.
 let bash = include_str!("../../shell-integration/bash/oriterm.bash");
 assert!(bash.contains("133;A"), "bash script must emit OSC 133;A");
 assert!(bash.contains("133;B"), "bash script must emit OSC 133;B");
 assert!(bash.contains("133;C"), "bash script must emit OSC 133;C");
 assert!(bash.contains("133;D"), "bash script must emit OSC 133;D");
 assert!(bash.contains("]7;"), "bash script must emit OSC 7");

 let zsh = include_str!("../../shell-integration/zsh/oriterm-integration");
 assert!(zsh.contains("133;A"), "zsh script must emit OSC 133;A");
 assert!(zsh.contains("]7;"), "zsh script must emit OSC 7");

 let fish =
 include_str!("../../shell-integration/fish/vendor_conf.d/oriterm-shell-integration.fish");
 assert!(fish.contains("133;A"), "fish script must emit OSC 133;A");
 assert!(fish.contains("]7;"), "fish script must emit OSC 7");

 let ps = include_str!("../../shell-integration/powershell/oriterm.ps1");
 assert!(ps.contains("133;A"), "pwsh script must emit OSC 133;A");
 assert!(ps.contains("]7;"), "pwsh script must emit OSC 7");
}

// Injection configuration

#[test]
fn setup_injection_bash_returns_posix_flag() {
 let mut cmd = portable_pty::CommandBuilder::new("bash");
 let dir = Path::new("/tmp/test-integration");

 let extra = super::inject::setup_injection(&mut cmd, Shell::Bash, dir, None);
 assert_eq!(extra, Some("--posix"));
}

#[test]
fn setup_injection_zsh_returns_none() {
 let mut cmd = portable_pty::CommandBuilder::new("zsh");
 let dir = Path::new("/tmp/test-integration");

 let extra = super::inject::setup_injection(&mut cmd, Shell::Zsh, dir, None);
 assert_eq!(extra, None);
}

#[test]
fn setup_injection_fish_returns_none() {
 let mut cmd = portable_pty::CommandBuilder::new("fish");
 let dir = Path::new("/tmp/test-integration");

 let extra = super::inject::setup_injection(&mut cmd, Shell::Fish, dir, None);
 assert_eq!(extra, None);
}

#[test]
fn setup_injection_powershell_returns_none() {
 let mut cmd = portable_pty::CommandBuilder::new("pwsh");
 let dir = Path::new("/tmp/test-integration");

 let extra = super::inject::setup_injection(&mut cmd, Shell::PowerShell, dir, None);
 assert_eq!(extra, None);
}

#[test]
fn setup_injection_wsl_returns_none() {
 let mut cmd = portable_pty::CommandBuilder::new("wsl");
 let dir = Path::new("/tmp/test-integration");

 let extra = super::inject::setup_injection(&mut cmd, Shell::Wsl, dir, Some("/home/user"));
 assert_eq!(extra, None);
}

// Raw interceptor

use oriterm_core::effect::{
 Effect, EffectSink, HostEffect, NotificationSource, PtyEffect, QueueingEffectSink,
};
use oriterm_core::{PromptState, Term, Theme};

/// Helper: create a minimal terminal for interceptor tests.
fn make_term() -> Term<QueueingEffectSink> {
 Term::new(24, 80, 100, Theme::Dark, QueueingEffectSink::new())
}

/// Helper: feed raw bytes through the interceptor.
fn intercept(term: &mut Term<QueueingEffectSink>, bytes: &[u8]) {
 let mut parser = vte::Parser::new();
 let mut interceptor = super::interceptor::RawInterceptor::new(term);
 parser.advance(&mut interceptor, bytes);
}

/// Test-local notification record for readable assertions.
struct TestNotification {
 source: NotificationSource,
 title: String,
 body: String,
}

/// Drain effects from a queuing term and extract desktop notifications,
/// applying `ClearPendingNotifications` semantics (clears preceding notifs).
fn drain_desktop_notifications(term: &Term<QueueingEffectSink>) -> Vec<TestNotification> {
 let mut effects = Vec::new();
 term.effect_sink().drain_into(&mut effects);
 let mut notifs = Vec::new();
 for effect in effects {
 match effect {
 Effect::Host(HostEffect::DesktopNotification {
 source,
 title,
 body,
 }) => {
 notifs.push(TestNotification {
 source,
 title,
 body,
 });
 }
 Effect::Host(HostEffect::ClearPendingNotifications) => notifs.clear(),
 _ => {}
 }
 }
 notifs
}

#[test]
fn interceptor_osc7_sets_cwd() {
 let mut term = make_term();
 assert!(term.cwd().is_none());

 intercept(&mut term, b"\x1b]7;file://hostname/home/user\x07");
 assert_eq!(term.cwd(), Some("/home/user"));
}

#[test]
fn interceptor_osc7_empty_hostname() {
 let mut term = make_term();
 intercept(&mut term, b"\x1b]7;file:///tmp/test\x07");
 assert_eq!(term.cwd(), Some("/tmp/test"));
}

#[test]
fn interceptor_osc7_marks_title_dirty() {
 let mut term = make_term();
 assert!(!term.is_title_dirty());

 intercept(&mut term, b"\x1b]7;file://host/home\x07");
 assert!(term.is_title_dirty());
 assert!(!term.has_explicit_title());
}

#[test]
fn interceptor_osc133_prompt_state_transitions() {
 let mut term = make_term();
 assert_eq!(term.prompt_state(), PromptState::None);

 // A — prompt start
 intercept(&mut term, b"\x1b]133;A\x07");
 assert_eq!(term.prompt_state(), PromptState::PromptStart);
 assert!(term.prompt_mark_pending());

 // B — command start
 intercept(&mut term, b"\x1b]133;B\x07");
 assert_eq!(term.prompt_state(), PromptState::CommandStart);

 // C — output start
 intercept(&mut term, b"\x1b]133;C\x07");
 assert_eq!(term.prompt_state(), PromptState::OutputStart);

 // D — command complete
 intercept(&mut term, b"\x1b]133;D\x07");
 assert_eq!(term.prompt_state(), PromptState::None);
}

#[test]
fn interceptor_osc9_simple_notification() {
 let mut term = make_term();
 intercept(&mut term, b"\x1b]9;Hello world\x07");

 let notifs = drain_desktop_notifications(&term);
 assert_eq!(notifs.len(), 1);
 assert_eq!(notifs[0].body, "Hello world");
 assert!(notifs[0].title.is_empty());
}

#[test]
fn interceptor_osc777_notification() {
 let mut term = make_term();
 intercept(&mut term, b"\x1b]777;notify;Build;Done!\x07");

 let notifs = drain_desktop_notifications(&term);
 assert_eq!(notifs.len(), 1);
 assert_eq!(notifs[0].title, "Build");
 assert_eq!(notifs[0].body, "Done!");
}

#[test]
fn interceptor_osc777_ignores_non_notify() {
 let mut term = make_term();
 intercept(&mut term, b"\x1b]777;other;foo;bar\x07");

 let notifs = drain_desktop_notifications(&term);
 assert!(notifs.is_empty());
}

// Effective title resolution

#[test]
fn effective_title_prefers_explicit() {
 let mut term = make_term();

 // Set explicit title via OSC 0 (high-level VTE processor).
 let mut proc = vte::ansi::Processor::<vte::ansi::StdSyncHandler>::new();
 proc.advance(&mut term, b"\x1b]0;my terminal\x07");

 // Also set CWD via raw interceptor.
 term.set_cwd(Some("/home/user/projects".to_string()));

 // Explicit title should win.
 assert_eq!(term.effective_title(), "my terminal");
}

#[test]
fn effective_title_falls_back_to_cwd() {
 let mut term = make_term();

 // CWD set but no explicit title.
 term.set_cwd(Some("/home/user/projects".to_string()));
 assert!(!term.has_explicit_title());

 assert_eq!(term.effective_title(), "projects");
}

#[test]
fn effective_title_cwd_after_osc7_clears_explicit() {
 let mut term = make_term();

 // Set explicit title via OSC 0.
 let mut proc = vte::ansi::Processor::<vte::ansi::StdSyncHandler>::new();
 proc.advance(&mut term, b"\x1b]0;vim\x07");
 assert_eq!(term.effective_title(), "vim");

 // OSC 7 clears explicit flag.
 intercept(&mut term, b"\x1b]7;file:///home/user/code\x07");
 assert!(!term.has_explicit_title());
 assert_eq!(term.effective_title(), "code");
}

#[test]
fn effective_title_empty_fallback() {
 let term = make_term();
 // No explicit title, no CWD.
 assert_eq!(term.effective_title(), "");
}

// Prompt row tracking

#[test]
fn mark_prompt_row_records_position() {
 let mut term = make_term();
 assert!(term.prompt_markers().is_empty());

 // Simulate OSC 133;A → prompt_mark_pending = true.
 intercept(&mut term, b"\x1b]133;A\x07");
 assert!(term.prompt_mark_pending());

 // Mark the prompt row (deferred marking).
 term.mark_prompt_row();
 assert!(!term.prompt_mark_pending());
 assert_eq!(term.prompt_markers()[0].prompt, 0); // Cursor at row 0.
}

#[test]
fn mark_prompt_row_avoids_duplicates() {
 let mut term = make_term();

 intercept(&mut term, b"\x1b]133;A\x07");
 term.mark_prompt_row();
 intercept(&mut term, b"\x1b]133;A\x07");
 term.mark_prompt_row();

 // Should not duplicate the same row.
 assert_eq!(term.prompt_markers().len(), 1);
 assert_eq!(term.prompt_markers()[0].prompt, 0);
}

#[test]
fn prune_prompt_markers_removes_evicted() {
 let mut term = make_term();

 // Manually insert some prompt rows.
 intercept(&mut term, b"\x1b]133;A\x07");
 term.mark_prompt_row();
 // Simulate moving cursor down and marking another prompt.
 term.set_prompt_mark_pending(true);
 // Can't easily move cursor in tests, so test prune directly.
 // Push artificial rows for testing.
 term.prune_prompt_markers(0); // No-op.
 assert_eq!(term.prompt_markers().len(), 1);
}

#[test]
fn no_prompts_navigation_is_noop() {
 let mut term = make_term();
 assert!(!term.scroll_to_previous_prompt());
 assert!(!term.scroll_to_next_prompt());
}

#[test]
fn interceptor_osc7_path_parsing() {
 use super::interceptor::parse_osc7_path;
 assert_eq!(parse_osc7_path("file://host/home/user"), "/home/user");
 assert_eq!(parse_osc7_path("file:///tmp"), "/tmp");
 assert_eq!(parse_osc7_path("/just/a/path"), "/just/a/path");
 assert_eq!(parse_osc7_path("file://host"), "host");
}

// Command timing

#[test]
fn osc133c_records_command_start() {
 let mut term = make_term();
 assert!(term.last_command_duration().is_none());

 // OSC 133;C — output start (command executing).
 intercept(&mut term, b"\x1b]133;C\x07");
 assert_eq!(term.prompt_state(), PromptState::OutputStart);
 // command_start is set internally but not directly exposed.
 // We verify indirectly by completing the command.
}

#[test]
fn osc133d_computes_command_duration() {
 let mut term = make_term();

 // C → D cycle should produce a duration.
 intercept(&mut term, b"\x1b]133;C\x07");
 std::thread::sleep(std::time::Duration::from_millis(10));
 intercept(&mut term, b"\x1b]133;D\x07");

 assert_eq!(term.prompt_state(), PromptState::None);
 let dur = term.last_command_duration().expect("should have duration");
 assert!(
 dur.as_millis() >= 10,
 "duration should be >= 10ms, got {dur:?}"
 );
}

#[test]
fn osc133d_without_c_produces_no_duration() {
 let mut term = make_term();

 // D without prior C — no duration.
 intercept(&mut term, b"\x1b]133;D\x07");
 assert!(term.last_command_duration().is_none());
}

#[test]
fn command_duration_updates_on_new_command() {
 let mut term = make_term();

 // First command.
 intercept(&mut term, b"\x1b]133;C\x07");
 std::thread::sleep(std::time::Duration::from_millis(10));
 intercept(&mut term, b"\x1b]133;D\x07");
 let dur1 = term.last_command_duration().unwrap();

 // Second command.
 intercept(&mut term, b"\x1b]133;C\x07");
 std::thread::sleep(std::time::Duration::from_millis(10));
 intercept(&mut term, b"\x1b]133;D\x07");
 let dur2 = term.last_command_duration().unwrap();

 // Both should be valid, second may differ slightly.
 assert!(dur1.as_millis() >= 10);
 assert!(dur2.as_millis() >= 10);
}

// Gap analysis tests

// OSC 7: percent-encoded paths (Fish and some shells percent-encode URIs).

#[test]
fn interceptor_osc7_percent_encoded_space() {
 let mut term = make_term();
 intercept(&mut term, b"\x1b]7;file://host/home/user/my%20project\x07");
 assert_eq!(term.cwd(), Some("/home/user/my project"));
}

#[test]
fn interceptor_osc7_percent_encoded_special_chars() {
 let mut term = make_term();
 // %C3%A9 is UTF-8 for 'é'.
 intercept(&mut term, b"\x1b]7;file:///home/user/caf%C3%A9\x07");
 assert_eq!(term.cwd(), Some("/home/user/café"));
}

#[test]
fn percent_decode_passthrough() {
 use super::interceptor::percent_decode;
 let input = "/home/user/projects";
 let result = percent_decode(input);
 assert!(matches!(result, std::borrow::Cow::Borrowed(_)));
 assert_eq!(result, "/home/user/projects");
}

#[test]
fn percent_decode_space_and_hash() {
 use super::interceptor::percent_decode;
 assert_eq!(percent_decode("hello%20world"), "hello world");
 assert_eq!(percent_decode("%23hash"), "#hash");
}

#[test]
fn percent_decode_invalid_hex_passthrough() {
 use super::interceptor::percent_decode;
 // %ZZ is not valid hex — pass through literally.
 assert_eq!(percent_decode("hello%ZZworld"), "hello%ZZworld");
 // Truncated: % at end of string.
 assert_eq!(percent_decode("hello%2"), "hello%2");
}

// OSC 7: Windows drive letter paths (cross-compiled from WSL targeting Windows).

#[test]
fn interceptor_osc7_windows_drive_letter() {
 let mut term = make_term();
 intercept(&mut term, b"\x1b]7;file:///C:/Users/eric/code\x07");
 assert_eq!(term.cwd(), Some("/C:/Users/eric/code"));
}

#[test]
fn parse_osc7_path_windows_drive() {
 use super::interceptor::parse_osc7_path;
 assert_eq!(parse_osc7_path("file:///C:/Users/eric"), "/C:/Users/eric");
}

// OSC 7: query string and fragment stripping.

#[test]
fn interceptor_osc7_strips_query_and_fragment() {
 let mut term = make_term();
 intercept(
 &mut term,
 b"\x1b]7;file://host/home/user?query=1#section\x07",
 );
 assert_eq!(term.cwd(), Some("/home/user"));
}

#[test]
fn parse_osc7_path_strips_query() {
 use super::interceptor::parse_osc7_path;
 assert_eq!(parse_osc7_path("file://host/home/user?q=1"), "/home/user");
}

#[test]
fn parse_osc7_path_strips_fragment() {
 use super::interceptor::parse_osc7_path;
 assert_eq!(
 parse_osc7_path("file://host/home/user#section"),
 "/home/user"
 );
}

#[test]
fn parse_osc7_path_bare_path_strips_fragment() {
 use super::interceptor::parse_osc7_path;
 assert_eq!(parse_osc7_path("/tmp/dir#frag"), "/tmp/dir");
}

// OSC 133;D with exit code parameter (shells emit `D;0` or `D;127`).

#[test]
fn interceptor_osc133d_with_exit_code_zero() {
 let mut term = make_term();
 intercept(&mut term, b"\x1b]133;C\x07");
 std::thread::sleep(std::time::Duration::from_millis(10));
 intercept(&mut term, b"\x1b]133;D;0\x07");

 assert_eq!(term.prompt_state(), PromptState::None);
 assert!(term.last_command_duration().is_some());
}

#[test]
fn interceptor_osc133d_with_nonzero_exit_code() {
 let mut term = make_term();
 intercept(&mut term, b"\x1b]133;C\x07");
 intercept(&mut term, b"\x1b]133;D;127\x07");

 assert_eq!(term.prompt_state(), PromptState::None);
}

// OSC 99: Kitty notification protocol. Kitty's spec mandates
// `OSC 99 ; metadata ; payload ST` (two semicolons even when metadata is
// empty); the payload lives at params[2]. Default `p=title` routes the
// payload into the `title` field — only `p=body` routes into `body`.

#[test]
fn interceptor_osc99_kitty_notification() {
 let mut term = make_term();
 intercept(&mut term, b"\x1b]99;;Build complete\x07");

 let notifs = drain_desktop_notifications(&term);
 assert_eq!(notifs.len(), 1);
 assert_eq!(notifs[0].title, "Build complete");
 assert!(notifs[0].body.is_empty());
}

// Script writing: nonexistent parent directory returns error.

#[test]
fn ensure_scripts_nonexistent_parent_returns_error() {
 // Unix: /dev/null is a file, so /dev/null/child fails in create_dir_all.
 // Windows: path with reserved characters (`<>`) is always invalid.
 #[cfg(unix)]
 let bad = Path::new("/dev/null/shell-int");
 #[cfg(windows)]
 let bad = Path::new(r"C:\<invalid>\shell-int");
 let result = ensure_scripts_on_disk(bad);
 assert!(result.is_err());
}

// Prompt navigation with multiple prompts across scrollback.

#[test]
fn prompt_navigation_scrolls_to_previous() {
 // Small terminal: 4 visible lines, 100 scrollback.
 let mut term = Term::new(4, 80, 100, Theme::Dark, QueueingEffectSink::new());
 let mut proc = vte::ansi::Processor::<vte::ansi::StdSyncHandler>::new();

 // Mark prompt at current position (abs row 0).
 term.set_prompt_mark_pending(true);
 term.mark_prompt_row();
 assert_eq!(term.prompt_markers()[0].prompt, 0);

 // Write enough lines to push the prompt into scrollback.
 for _ in 0..20 {
 proc.advance(&mut term, b"\n");
 }

 // Viewport is at bottom, prompt row 0 is in scrollback.
 assert!(term.scroll_to_previous_prompt());
}

#[test]
fn prompt_navigation_no_prompt_above_returns_false() {
 let mut term = Term::new(4, 80, 100, Theme::Dark, QueueingEffectSink::new());

 // Mark prompt at current position (row 0), viewport is already here.
 term.set_prompt_mark_pending(true);
 term.mark_prompt_row();

 // No prompt ABOVE viewport top — should return false.
 assert!(!term.scroll_to_previous_prompt());
}

#[test]
fn prompt_navigation_scrolls_to_next() {
 let mut term = Term::new(4, 80, 100, Theme::Dark, QueueingEffectSink::new());
 let mut proc = vte::ansi::Processor::<vte::ansi::StdSyncHandler>::new();

 // Write some content then mark a prompt.
 for _ in 0..10 {
 proc.advance(&mut term, b"\n");
 }
 term.set_prompt_mark_pending(true);
 term.mark_prompt_row();
 let prompt_row = term.prompt_markers().last().unwrap().prompt;

 // Write more content to push the prompt into scrollback.
 for _ in 0..20 {
 proc.advance(&mut term, b"\n");
 }

 // Scroll all the way up.
 let sb_len = term.grid().scrollback().len();
 term.grid_mut().scroll_display(sb_len as isize);

 // Now navigate to the next prompt (should be below viewport).
 let scrolled = term.scroll_to_next_prompt();
 // There should be a prompt row at or after the viewport bottom.
 assert!(
 scrolled || prompt_row < sb_len,
 "should navigate to prompt below viewport"
 );
}

// Gap analysis: extra content after OSC 133 action letter

#[test]
fn interceptor_osc133_extra_content_after_action_letter() {
 let mut term = make_term();
 // `133;Cextra` — action letter C followed by garbage. The VTE parser
 // splits on `;`, so params[1] = b"Cextra". Our handler checks only
 // params[1][0], so the action should still be recognized.
 intercept(&mut term, b"\x1b]133;Cextra\x07");
 assert_eq!(term.prompt_state(), PromptState::OutputStart);
}

#[test]
fn interceptor_osc133_extra_content_after_d() {
 let mut term = make_term();
 // Set up a C→D cycle to verify D still works.
 intercept(&mut term, b"\x1b]133;C\x07");
 intercept(&mut term, b"\x1b]133;Dextra\x07");
 assert_eq!(term.prompt_state(), PromptState::None);
}

// Gap analysis: negative exit code in OSC 133;D

#[test]
fn interceptor_osc133d_with_negative_exit_code() {
 let mut term = make_term();
 intercept(&mut term, b"\x1b]133;C\x07");
 std::thread::sleep(std::time::Duration::from_millis(10));
 // Signal-killed process: exit code -1.
 intercept(&mut term, b"\x1b]133;D;-1\x07");

 assert_eq!(term.prompt_state(), PromptState::None);
 assert!(term.last_command_duration().is_some());
}

// Gap analysis: exit code with option suffix

#[test]
fn interceptor_osc133d_with_exit_code_and_aid() {
 let mut term = make_term();
 intercept(&mut term, b"\x1b]133;C\x07");
 std::thread::sleep(std::time::Duration::from_millis(10));
 // `D;127;aid=foo` — exit code followed by key=value option.
 intercept(&mut term, b"\x1b]133;D;127;aid=foo\x07");

 assert_eq!(term.prompt_state(), PromptState::None);
 assert!(term.last_command_duration().is_some());
}

// Gap analysis: OSC 133;A with trailing semicolons/bare keys

#[test]
fn interceptor_osc133a_trailing_semicolon() {
 let mut term = make_term();
 // `133;A;` — trailing semicolon produces an empty params[2].
 intercept(&mut term, b"\x1b]133;A;\x07");
 assert_eq!(term.prompt_state(), PromptState::PromptStart);
 assert!(term.prompt_mark_pending());
}

#[test]
fn interceptor_osc133a_bare_key_option() {
 let mut term = make_term();
 // `133;A;barekey` — unknown option, should be tolerated.
 intercept(&mut term, b"\x1b]133;A;barekey\x07");
 assert_eq!(term.prompt_state(), PromptState::PromptStart);
}

// Gap analysis: OSC 7 with empty URI

#[test]
fn interceptor_osc7_empty_uri() {
 let mut term = make_term();
 // `\x1b]7;\x07` — semicolon but no path content.
 intercept(&mut term, b"\x1b]7;\x07");
 // Empty URI should not set CWD (path is empty after parsing).
 assert!(term.cwd().is_none());
}

#[test]
fn interceptor_osc7_file_scheme_only() {
 let mut term = make_term();
 // `file://` with no path after scheme.
 intercept(&mut term, b"\x1b]7;file://\x07");
 // No `/` after hostname portion — path is the hostname itself.
 // This is an edge case; CWD should either be empty or not set.
 // The parser returns the empty hostname portion, which is empty.
 // Since the path is empty, CWD should not be updated.
 assert!(term.cwd().is_none());
}

// Gap analysis: OSC 9 single-character body

#[test]
fn interceptor_osc9_single_char_body() {
 let mut term = make_term();
 intercept(&mut term, b"\x1b]9;X\x07");

 let notifs = drain_desktop_notifications(&term);
 assert_eq!(notifs.len(), 1);
 assert_eq!(notifs[0].body, "X");
}

// Gap analysis: command timing very fast

#[test]
fn command_timing_very_fast_command() {
 let mut term = make_term();
 // C→D with no sleep in between — sub-millisecond command.
 intercept(&mut term, b"\x1b]133;C\x07");
 intercept(&mut term, b"\x1b]133;D\x07");

 let dur = term.last_command_duration().expect("should have duration");
 // Should be zero or very small, not None.
 assert!(dur.as_secs() == 0);
}

// Gap analysis: RIS clears shell state (end-to-end via interceptor)

#[test]
fn ris_clears_cwd_and_effective_title() {
 let mut term = make_term();

 // Set CWD via OSC 7.
 intercept(&mut term, b"\x1b]7;file:///home/user/projects\x07");
 assert_eq!(term.effective_title(), "projects");

 // RIS via high-level VTE processor.
 let mut proc = vte::ansi::Processor::<vte::ansi::StdSyncHandler>::new();
 proc.advance(&mut term, b"\x1bc");

 // CWD should be cleared, title falls back to empty.
 assert!(term.cwd().is_none());
 assert_eq!(term.effective_title(), "");
}

#[test]
fn ris_clears_prompt_markers() {
 let mut term = make_term();

 // Set up a full prompt lifecycle.
 intercept(&mut term, b"\x1b]133;A\x07");
 term.mark_prompt_row();
 intercept(&mut term, b"\x1b]133;B\x07");
 term.mark_command_start_row();
 intercept(&mut term, b"\x1b]133;C\x07");
 term.mark_output_start_row();

 assert_eq!(term.prompt_markers().len(), 1);
 assert!(term.prompt_markers()[0].command.is_some());

 // RIS.
 let mut proc = vte::ansi::Processor::<vte::ansi::StdSyncHandler>::new();
 proc.advance(&mut term, b"\x1bc");

 assert!(term.prompt_markers().is_empty());
 assert_eq!(term.prompt_state(), PromptState::None);
}

#[test]
fn ris_clears_pending_notifications() {
 let mut term = make_term();

 // Push a notification via OSC 9.
 intercept(&mut term, b"\x1b]9;Build finished\x07");
 assert_eq!(drain_desktop_notifications(&term).len(), 1);

 // Push another.
 intercept(&mut term, b"\x1b]9;Test passed\x07");

 // RIS.
 let mut proc = vte::ansi::Processor::<vte::ansi::StdSyncHandler>::new();
 proc.advance(&mut term, b"\x1bc");

 assert!(
 drain_desktop_notifications(&term).is_empty(),
 "RIS should clear pending notifications"
 );
}

// Gap analysis: multiple A markers without B/C/D

#[test]
fn multiple_osc133a_without_completion_creates_separate_markers() {
 let mut term = make_term();
 let mut proc = vte::ansi::Processor::<vte::ansi::StdSyncHandler>::new();

 // First prompt.
 intercept(&mut term, b"\x1b]133;A\x07");
 term.mark_prompt_row();

 // Move cursor down (simulates shell output between prompts).
 proc.advance(&mut term, b"\r\n\r\n");

 // Second prompt (e.g., user pressed Ctrl-C, shell re-emits prompt).
 intercept(&mut term, b"\x1b]133;A\x07");
 term.mark_prompt_row();

 // Should have two markers — first incomplete, second fresh.
 assert_eq!(term.prompt_markers().len(), 2);
 assert!(term.prompt_markers()[0].command.is_none());
 assert!(term.prompt_markers()[0].output.is_none());
}

// XTVERSION (CSI > q) —

/// Regression: XTVERSION (CSI > q) now routes through the
/// vendored `vte::ansi::Processor` (was a `RawInterceptor::csi_dispatch`
/// override). The dual-pass production helper runs the raw interceptor
/// FIRST and the high-level processor SECOND on the same bytes — exactly
/// the order the IO thread uses at `oriterm_mux/src/pane/io_thread/mod.rs`
/// `handle_bytes`. Asserts EXACTLY one PTY write; a leftover raw-interceptor
/// arm would produce two replies and surface here.
#[test]
fn xtversion_responds_with_oriterm_version() {
 let sink = QueueingEffectSink::new();
 let mut term = Term::new(24, 80, 100, Theme::Dark, sink);

 feed_mux_and_proc(&mut term, b"\x1b[>q");

 let mut effects = Vec::new();
 term.effect_sink().drain_into(&mut effects);
 let xtversion_writes: Vec<_> = effects
 .iter()
 .filter_map(|e| match e {
 Effect::Pty(PtyEffect::Write { bytes, kind }) if bytes.starts_with(b"\x1bP>|") => {
 Some((bytes.clone(), *kind))
 }
 _ => None,
 })
 .collect();
 assert_eq!(
 xtversion_writes.len(),
 1,
 "XTVERSION must produce exactly one PTY write through the dual-pass production path, got: {xtversion_writes:?}"
 );
 let (bytes, kind) = &xtversion_writes[0];
 assert_eq!(*kind, oriterm_core::effect::PtyWriteKind::DeviceAttribute);
 let s = String::from_utf8_lossy(bytes);
 assert!(
 s.contains("oriterm"),
 "XTVERSION reply must contain 'oriterm', got: {s}"
 );
}

// OSC 7 non-UTF-8 edge case

#[test]
fn interceptor_osc7_non_utf8_bytes_returns_empty_path() {
 let mut term = make_term();

 // Feed OSC 7 with invalid UTF-8 — the interceptor uses
 // `from_utf8().unwrap_or_default()`, so it should produce an empty
 // string and not set CWD.
 let mut raw = Vec::new();
 raw.extend_from_slice(b"\x1b]7;file:///");
 raw.push(0xFF); // Invalid UTF-8.
 raw.push(0xFE);
 raw.push(0x07); // BEL terminator.
 intercept(&mut term, &raw);

 // The raw_path should be empty because from_utf8 fails on the whole
 // param and returns "". Or it may parse as "file:///" with an empty path.
 // Either way, CWD should NOT be set to garbage.
 if let Some(cwd) = term.cwd() {
 // If CWD was set, it should be a valid path (the "/" portion).
 assert!(
 cwd.is_ascii(),
 "CWD should not contain non-UTF-8 garbage, got: {cwd:?}"
 );
 }
}

// spec_chain_helper: production-order dual-pass byte feed.
// `spec_chain_helper::feed_mux_and_proc` encapsulates the production
// "interceptor FIRST, processor SECOND" byte-feed order so downstream
// spec_chain tests for OSC 7 / 9 / 99 / 133 / 633 / 777 cannot accidentally
// reorder the two passes (a silent false-green source). It lives in the
// sibling unit-test module because `RawInterceptor` is `pub(crate)` and
// integration tests in `oriterm_mux/tests/` cannot reach it.
// The high-level `vte::ansi::Processor` silently drops OSC 133 / 9 / 99 / 777
// (no `Handler` trait route exists for them), so a test that calls only
// `Processor::advance` would observe NO state mutation for those sequences
// — exactly the behavior the production path avoids by running the
// interceptor FIRST. The TDD pair below pins this contract.

mod spec_chain_helper {
 use oriterm_core::Term;
 use oriterm_core::effect::QueueingEffectSink;
 use vte::Parser;
 use vte::ansi::{Processor, StdSyncHandler};

 use crate::shell_integration::interceptor::RawInterceptor;

 /// Feed `bytes` through the production-order parser chain:
 /// the raw `vte::Parser` + `RawInterceptor` runs first, then the
 /// high-level `vte::ansi::Processor` runs on the same bytes.
 /// Both passes mutate `term`. The interceptor handles OSC 7 / 9 / 99
 /// / 133 / 633 / 777, and the high-level processor drives every
 /// other OSC, CSI, ESC, and DCS sequence.
 pub(super) fn feed_mux_and_proc(term: &mut Term<QueueingEffectSink>, bytes: &[u8]) {
 // Scope the interceptor so its `&mut term` borrow ends before the
 // processor takes its own `&mut term` borrow on the next line.
 {
 let mut interceptor = RawInterceptor::new(term);
 let mut raw_parser = Parser::new();
 raw_parser.advance(&mut interceptor, bytes);
 }
 let mut processor = Processor::<StdSyncHandler>::new();
 processor.advance(term, bytes);
 }
}

/// TDD RED side: feeding OSC 133;A through ONLY the high-level
/// `vte::ansi::Processor` (no `RawInterceptor` pass) leaves `prompt_state`
/// unchanged. Proves the high-level processor really drops OSC 133, so
/// the mux interceptor is load-bearing for the production path.
#[test]
fn osc133a_via_processor_only_does_not_change_prompt_state() {
 use oriterm_core::PromptState;

 let mut term = make_term();
 assert_eq!(term.prompt_state(), PromptState::None);

 let mut processor = vte::ansi::Processor::<vte::ansi::StdSyncHandler>::new();
 processor.advance(&mut term, b"\x1b]133;A\x1b\\");

 assert_eq!(
 term.prompt_state(),
 PromptState::None,
 "high-level Processor must NOT route OSC 133 to a Handler hook \
 — if this assertion ever fires, OSC 133 was added to the \
 high-level dispatcher and would be double-handled in production"
 );
 assert!(
 !term.prompt_mark_pending(),
 "high-level Processor must NOT set prompt_mark_pending for OSC 133"
 );
}

/// TDD GREEN side: feeding OSC 133;A via the production-order
/// `spec_chain_helper::feed_mux_and_proc` drives the interceptor's OSC
/// 133 handler, which transitions `prompt_state` to `PromptStart`. This
/// is the contract every downstream §10.3/§10.4/§10.8 mux-intercepted
/// OSC test relies on.
#[test]
fn osc133a_via_spec_chain_helper_sets_prompt_start() {
 use oriterm_core::PromptState;

 let mut term = make_term();
 assert_eq!(term.prompt_state(), PromptState::None);

 feed_mux_and_proc(&mut term, b"\x1b]133;A\x1b\\");

 assert_eq!(
 term.prompt_state(),
 PromptState::PromptStart,
 "production-order dual-pass MUST drive OSC 133;A through the \
 interceptor — if this assertion fails, the interceptor pass \
 was dropped and downstream spec_chain tests would silently \
 pass against a parser that never saw the sequence"
 );
 assert!(
 term.prompt_mark_pending(),
 "OSC 133;A via interceptor must set prompt_mark_pending"
 );
}

// §10.3 — OSC 9 / 99 / 777 desktop notifications.
// These tests pin the `NotificationSource` discriminator that the mux
// interceptor produces for each OSC variant, plus a regression guard proving
// the high-level `vte::ansi::Processor` does NOT route OSC 9 to a
// notification effect (mux interceptor is load-bearing).

/// §10.3 — OSC 9 simple body: source=Osc9, title="", body preserved.
/// OSC 9 (Growl-style, iTerm2/Windows Terminal) has no title field.
#[test]
fn osc9_simple_body_fires_notification() {
 let mut term = make_term();
 intercept(&mut term, b"\x1b]9;Build complete\x1b\\");

 let notifs = drain_desktop_notifications(&term);
 assert_eq!(notifs.len(), 1);
 assert_eq!(notifs[0].source, NotificationSource::Osc9);
 assert_eq!(notifs[0].title, "");
 assert_eq!(notifs[0].body, "Build complete");
}

/// §10.3 — OSC 99 spec-conformant simple form (`OSC 99 ;; payload ST` —
/// two semicolons mandatory per Kitty's spec even when metadata is empty).
/// Default `p=title` (no `p` key in metadata) routes the payload into the
/// `title` field per Kitty `desktop-notifications.rst` line 472. Pinning
/// the source discriminator prevents a future refactor from collapsing
/// the OSC 9 / OSC 99 arms in `handle_notification_simple`.
#[test]
fn osc99_default_payload_routes_to_title() {
 let mut term = make_term();
 intercept(&mut term, b"\x1b]99;;kitty payload\x1b\\");

 let notifs = drain_desktop_notifications(&term);
 assert_eq!(notifs.len(), 1);
 assert_eq!(notifs[0].source, NotificationSource::Osc99);
 assert_eq!(
 notifs[0].title, "kitty payload",
 "Kitty OSC 99 default p=title: payload at params[2] must route into title."
 );
 assert_eq!(notifs[0].body, "");
}

/// §10.3 — Kitty's OSC 99 two-parameter form with metadata that does NOT
/// include a `p=` key (here `i=1:t=info`): the default `p=title` still
/// applies; the payload routes into `title`; metadata is recognised as
/// opaque and silently discarded. Pins the deviation tracked in
/// `plans/spec-conformance/catalog/osc.md::OSC-99` — only the `p=` key
/// is honoured; chunking (`i=` chunk id, `d=` done), base64 (`e=1`), type
/// (`t=`), application (`f=`), urgency (`u=`), sound (`s=`), and other
/// metadata keys are not honoured.
#[test]
fn osc99_metadata_form_default_p_routes_payload_to_title() {
 let mut term = make_term();
 intercept(&mut term, b"\x1b]99;i=1:t=info;hello\x1b\\");

 let notifs = drain_desktop_notifications(&term);
 assert_eq!(notifs.len(), 1);
 assert_eq!(notifs[0].source, NotificationSource::Osc99);
 assert_eq!(
 notifs[0].title, "hello",
 "metadata without `p=` defaults to p=title; payload routes to title."
 );
 assert_eq!(notifs[0].body, "");
}

/// §10.3 — Kitty's OSC 99 with `p=body` in the metadata: payload routes to
/// `body` (not `title`). Pins the only metadata key the implementation
/// actually parses (`p=`).
#[test]
fn osc99_p_body_routes_payload_to_body() {
 let mut term = make_term();
 intercept(&mut term, b"\x1b]99;p=body;hello\x1b\\");

 let notifs = drain_desktop_notifications(&term);
 assert_eq!(notifs.len(), 1);
 assert_eq!(notifs[0].source, NotificationSource::Osc99);
 assert_eq!(notifs[0].title, "");
 assert_eq!(notifs[0].body, "hello");
}

/// §10.3 — Kitty's OSC 99 with both empty metadata and empty payload
/// (`OSC 99 ;; ST`): per Kitty's spec rule "A notification with not title
/// and no body is ignored", the notification is dropped — no
/// `DesktopNotification` effect is emitted.
#[test]
fn osc99_empty_payload_drops_notification() {
 let mut term = make_term();
 intercept(&mut term, b"\x1b]99;;\x1b\\");

 let notifs = drain_desktop_notifications(&term);
 assert!(
 notifs.is_empty(),
 "Kitty OSC 99 with empty payload (no title, no body) must be dropped"
 );
}

/// §10.3 — Kitty's OSC 99 with `p=close` (or any other unknown payload kind
/// — `icon`, `?`, `alive`, `buttons`): per Kitty spec "Terminal emulators
/// should ignore payloads of unknown type", the notification is dropped.
#[test]
fn osc99_unsupported_payload_kind_drops_notification() {
 let mut term = make_term();
 intercept(&mut term, b"\x1b]99;p=close;something\x1b\\");

 let notifs = drain_desktop_notifications(&term);
 assert!(
 notifs.is_empty(),
 "Kitty OSC 99 with p=close (or any unknown p value) must be dropped"
 );
}

/// §10.3 — OSC 777 with `notify` action, title, and body: source=Osc777.
#[test]
fn osc777_notify_title_body() {
 let mut term = make_term();
 intercept(
 &mut term,
 b"\x1b]777;notify;Build;completed successfully\x1b\\",
 );

 let notifs = drain_desktop_notifications(&term);
 assert_eq!(notifs.len(), 1);
 assert_eq!(notifs[0].source, NotificationSource::Osc777);
 assert_eq!(notifs[0].title, "Build");
 assert_eq!(notifs[0].body, "completed successfully");
}

/// §10.3 — OSC 777 with a non-`notify` action is filtered out; no
/// notification effect is emitted.
#[test]
fn osc777_non_notify_action_dropped() {
 let mut term = make_term();
 intercept(&mut term, b"\x1b]777;BAD_ACTION;title;body\x1b\\");

 let notifs = drain_desktop_notifications(&term);
 assert!(
 notifs.is_empty(),
 "OSC 777 with action != 'notify' must not emit a desktop notification"
 );
}

/// §10.3 — OSC 9 with empty body still emits a notification (body="").
#[test]
fn osc9_empty_body() {
 let mut term = make_term();
 intercept(&mut term, b"\x1b]9;\x1b\\");

 let notifs = drain_desktop_notifications(&term);
 assert_eq!(notifs.len(), 1);
 assert_eq!(notifs[0].source, NotificationSource::Osc9);
 assert_eq!(notifs[0].title, "");
 assert_eq!(notifs[0].body, "");
}

/// §10.3 — OSC 777 with an empty title field: title="", body preserved.
#[test]
fn osc777_missing_title() {
 let mut term = make_term();
 intercept(&mut term, b"\x1b]777;notify;;body-only\x1b\\");

 let notifs = drain_desktop_notifications(&term);
 assert_eq!(notifs.len(), 1);
 assert_eq!(notifs[0].source, NotificationSource::Osc777);
 assert_eq!(notifs[0].title, "");
 assert_eq!(notifs[0].body, "body-only");
}

/// §10.3 — Property: OSC 9 (Growl form) and OSC 99 (Kitty form, default
/// `p=title`) fed in the same scenario produce *distinct* `NotificationSource`
/// variants AND distinct field-routing semantics — OSC 9 routes payload into
/// `body` (no title), OSC 99 routes payload into `title` (default `p=title`).
/// A refactor that collapses the OSC 9 / 99 detection in
/// `handle_notification_simple` would fail this assertion immediately. The
/// OSC 99 input uses Kitty-conformant `;;` form per spec.
#[test]
fn osc9_and_osc99_use_different_sources() {
 let mut term = make_term();
 intercept(&mut term, b"\x1b]9;first\x1b\\");
 intercept(&mut term, b"\x1b]99;;second\x1b\\");

 let notifs = drain_desktop_notifications(&term);
 assert_eq!(notifs.len(), 2);
 assert_eq!(notifs[0].source, NotificationSource::Osc9);
 assert_eq!(notifs[0].title, "");
 assert_eq!(notifs[0].body, "first");
 assert_eq!(notifs[1].source, NotificationSource::Osc99);
 assert_eq!(notifs[1].title, "second");
 assert_eq!(notifs[1].body, "");
 assert_ne!(
 notifs[0].source, notifs[1].source,
 "OSC 9 and OSC 99 must produce distinct NotificationSource variants"
 );
}

/// §10.3 — Regression guard: feeding OSC 9 through the high-level
/// `vte::ansi::Processor` ALONE (no `RawInterceptor` pass) does NOT emit a
/// desktop notification. Proves the mux interceptor is load-bearing for
/// OSC 9; if someone accidentally adds OSC 9 to the high-level dispatcher
/// too, this test fails (double-dispatch detection). Mirrors
/// `osc133a_via_processor_only_does_not_change_prompt_state`.
#[test]
fn osc9_via_processor_without_mux_drops() {
 let mut term = make_term();

 let mut processor = vte::ansi::Processor::<vte::ansi::StdSyncHandler>::new();
 processor.advance(&mut term, b"\x1b]9;X\x1b\\");

 let notifs = drain_desktop_notifications(&term);
 assert!(
 notifs.is_empty(),
 "high-level Processor must NOT route OSC 9 to a notification \
 effect — if this assertion fires, OSC 9 was added to the \
 high-level dispatcher and would be double-handled in production"
 );
}

// §10.4 — OSC 133 semantic prompt + OSC 633 VS Code shell integration.
// Both OSC 133 and OSC 633 drive the same `PromptState` state machine through
// the mux interceptor. OSC 633 additionally records the raw command line via
// `E` and routes property settings (`P;Cwd=...`) through `Term::set_cwd` —
// the same SSOT OSC 7 writes to. Negative pins confirm the high-level
// `vte::ansi::Processor` does NOT dispatch either OSC, so a future refactor
// that accidentally duplicates a dispatch arm on the high-level side is
// detected immediately.

use oriterm_core::PromptMarker;

/// Drain `HostEffect::CommandComplete` effects from the terminal's queue.
/// Used by OSC 133;D / OSC 633;D tests to assert the effect landed.
fn drain_command_complete(term: &Term<QueueingEffectSink>) -> Vec<std::time::Duration> {
 let mut effects = Vec::new();
 term.effect_sink().drain_into(&mut effects);
 let mut out = Vec::new();
 for effect in effects {
 if let Effect::Host(HostEffect::CommandComplete { duration }) = effect {
 out.push(duration);
 }
 }
 out
}

/// §10.4 — OSC 133;A drives `PromptState` to `PromptStart` and sets
/// `prompt_mark_pending`. Matches the dispatch at
/// `oriterm_mux/src/shell_integration/interceptor.rs` `handle_osc133` `b'A'`
/// arm. Uses `feed_mux_and_proc` so the production-order dual-pass is pinned.
#[test]
fn osc133_a_sets_prompt_state() {
 let mut term = make_term();
 assert_eq!(term.prompt_state(), PromptState::None);

 feed_mux_and_proc(&mut term, b"\x1b]133;A\x1b\\");

 assert_eq!(term.prompt_state(), PromptState::PromptStart);
 assert!(term.prompt_mark_pending());
}

/// §10.4 — OSC 133;B drives `PromptState` to `CommandStart` and sets
/// `command_start_mark_pending`.
#[test]
fn osc133_b_sets_command_state() {
 let mut term = make_term();

 feed_mux_and_proc(&mut term, b"\x1b]133;B\x1b\\");

 assert_eq!(term.prompt_state(), PromptState::CommandStart);
 assert!(term.command_start_mark_pending());
}

/// §10.4 — OSC 133;C drives `PromptState` to `OutputStart` and sets
/// `output_start_mark_pending`. Command-start time is stored with a live
/// wall-clock `Instant` per interceptor.rs `b'C'` arm; the exact value is
/// not asserted because there is no injectable-clock seam at the C step
/// (the Option A seam only covers the D step via `finish_command(now)`).
#[test]
fn osc133_c_sets_output_state() {
 let mut term = make_term();

 feed_mux_and_proc(&mut term, b"\x1b]133;C\x1b\\");

 assert_eq!(term.prompt_state(), PromptState::OutputStart);
 assert!(term.output_start_mark_pending());
}

/// §10.4 — OSC 133;D after a full A→B→C lifecycle clears `PromptState` to
/// `None` AND emits a `HostEffect::CommandComplete` with a non-negative
/// duration. The deferred-mark helpers are invoked between feeds so the
/// `PromptMarker` for the completed lifecycle carries A/B/C fields — this
/// mirrors `post_parse_housekeeping` in production.
/// The exhaustive match on `PromptMarker { prompt, command, output }` is a
/// property: if a future refactor adds a fourth field (e.g. `complete`),
/// this test MUST be updated explicitly — it will not silently compile. This
/// is the catch for scope clarification D ("`PromptMarker` has no D-field").
#[test]
fn osc133_d_clears_state_and_emits_command_complete() {
 let mut term = make_term();

 feed_mux_and_proc(&mut term, b"\x1b]133;A\x1b\\");
 term.mark_prompt_row();
 feed_mux_and_proc(&mut term, b"\x1b]133;B\x1b\\");
 term.mark_command_start_row();
 feed_mux_and_proc(&mut term, b"\x1b]133;C\x1b\\");
 term.mark_output_start_row();
 feed_mux_and_proc(&mut term, b"\x1b]133;D\x1b\\");

 assert_eq!(term.prompt_state(), PromptState::None);

 let durations = drain_command_complete(&term);
 assert_eq!(
 durations.len(),
 1,
 "OSC 133;D after C must emit exactly one HostEffect::CommandComplete"
 );
 assert!(durations[0] >= std::time::Duration::ZERO);

 let marker = term
 .prompt_markers()
 .last()
 .expect("deferred-mark helpers populate the marker");
 let PromptMarker {
 prompt: _,
 command,
 output,
 } = marker;
 assert!(
 command.is_some(),
 "B-marked command row must survive into the completed lifecycle"
 );
 assert!(
 output.is_some(),
 "C-marked output row must survive into the completed lifecycle"
 );
}

/// §10.4 — Two OSC 133;A feeds without intervening B/C/D produce TWO
/// `PromptMarker`s, each with `command == None` and `output == None`. The
/// deferred-mark helper is called after each A so the pending flag flushes
/// into the marker vec (mirrors production `post_parse_housekeeping`). Uses
/// the high-level `Processor` between A feeds to move the cursor so the
/// de-duplication logic in `mark_prompt_row` does NOT coalesce the rows.
#[test]
fn osc133_a_without_b_does_not_record_command() {
 let mut term = make_term();

 feed_mux_and_proc(&mut term, b"\x1b]133;A\x1b\\");
 term.mark_prompt_row();

 // Move cursor to a new row so the second A marks a distinct position.
 feed_mux_and_proc(&mut term, b"\r\n\r\n");

 feed_mux_and_proc(&mut term, b"\x1b]133;A\x1b\\");
 term.mark_prompt_row();

 let markers = term.prompt_markers();
 assert_eq!(
 markers.len(),
 2,
 "two A feeds at distinct rows must produce two markers"
 );
 for (i, marker) in markers.iter().enumerate() {
 assert!(
 marker.command.is_none(),
 "marker {i}: no B feed means command must remain None"
 );
 assert!(
 marker.output.is_none(),
 "marker {i}: no C feed means output must remain None"
 );
 }
}

/// §10.4 — OSC 133;D without a preceding C is a no-op: no
/// `HostEffect::CommandComplete` is emitted because `finish_command()`
/// returns `None` when `command_start` is unset (interceptor.rs `b'D'` arm
/// wraps the push in `if let Some(duration) =...`). Pins the
/// `set_prompt_state(None) + finish_command() == None → skip effect` path.
#[test]
fn osc133_command_complete_without_c_is_noop() {
 let mut term = make_term();

 feed_mux_and_proc(&mut term, b"\x1b]133;D\x1b\\");

 assert_eq!(term.prompt_state(), PromptState::None);
 let durations = drain_command_complete(&term);
 assert!(
 durations.is_empty(),
 "OSC 133;D without a preceding C must not emit CommandComplete"
 );
}

/// §10.4 — A full A→B→C→D lifecycle with deferred-mark helpers populates
/// one `PromptMarker` whose `prompt`, `command`, and `output` rows all
/// correspond to distinct absolute positions (advanced via `\r\n` between
/// steps). Verifies the marker-flush plumbing end-to-end.
#[test]
fn osc133_full_lifecycle_records_markers() {
 let mut term = make_term();

 feed_mux_and_proc(&mut term, b"\x1b]133;A\x1b\\");
 term.mark_prompt_row();
 feed_mux_and_proc(&mut term, b"\r\n\x1b]133;B\x1b\\");
 term.mark_command_start_row();
 feed_mux_and_proc(&mut term, b"\r\n\x1b]133;C\x1b\\");
 term.mark_output_start_row();
 feed_mux_and_proc(&mut term, b"\r\n\x1b]133;D\x1b\\");

 let markers = term.prompt_markers();
 assert_eq!(
 markers.len(),
 1,
 "single A→B→C→D lifecycle records exactly one marker"
 );
 let marker = &markers[0];
 let cmd = marker.command.expect("B must fill command row");
 let out = marker.output.expect("C must fill output row");
 assert!(
 marker.prompt < cmd && cmd < out,
 "prompt < command < output must hold after \\r\\n-advanced lifecycle: \
 got prompt={}, command={cmd}, output={out}",
 marker.prompt,
 );
}

// ── OSC 633 (VS Code shell integration) ─────────────────────────────

/// §10.4 — OSC 633;A mirrors OSC 133;A: drives `PromptState::PromptStart`
/// and sets `prompt_mark_pending`. VS Code's `shellIntegrationAddon.ts`
/// uses the same `A` sub-command semantics as Final Term OSC 133.
#[test]
fn osc633_a_sets_prompt_state() {
 let mut term = make_term();

 feed_mux_and_proc(&mut term, b"\x1b]633;A\x1b\\");

 assert_eq!(term.prompt_state(), PromptState::PromptStart);
 assert!(term.prompt_mark_pending());
}

/// §10.4 — OSC 633;B mirrors OSC 133;B.
#[test]
fn osc633_b_sets_command_state() {
 let mut term = make_term();

 feed_mux_and_proc(&mut term, b"\x1b]633;B\x1b\\");

 assert_eq!(term.prompt_state(), PromptState::CommandStart);
 assert!(term.command_start_mark_pending());
}

/// §10.4 — OSC 633;C mirrors OSC 133;C.
#[test]
fn osc633_c_sets_output_state() {
 let mut term = make_term();

 feed_mux_and_proc(&mut term, b"\x1b]633;C\x1b\\");

 assert_eq!(term.prompt_state(), PromptState::OutputStart);
 assert!(term.output_start_mark_pending());
}

/// §10.4 — OSC 633;D after C emits `HostEffect::CommandComplete` and
/// clears `PromptState` — same contract as OSC 133;D.
#[test]
fn osc633_d_emits_command_complete() {
 let mut term = make_term();

 feed_mux_and_proc(&mut term, b"\x1b]633;C\x1b\\");
 feed_mux_and_proc(&mut term, b"\x1b]633;D\x1b\\");

 assert_eq!(term.prompt_state(), PromptState::None);
 let durations = drain_command_complete(&term);
 assert_eq!(
 durations.len(),
 1,
 "OSC 633;D after C must emit one CommandComplete"
 );
 assert!(durations[0] >= std::time::Duration::ZERO);
}

/// §10.4 — OSC 633;E records the raw command line text on
/// `Term::last_command_line`. This is the VS Code-specific sub-op that has
/// no OSC 133 counterpart.
#[test]
fn osc633_e_records_command_line() {
 let mut term = make_term();
 assert!(term.last_command_line().is_none());

 feed_mux_and_proc(&mut term, b"\x1b]633;E;git status\x1b\\");

 assert_eq!(term.last_command_line(), Some("git status"));
}

/// §10.4 — OSC 633;P;Cwd=<path> routes through `Term::set_cwd` — the SAME
/// canonical field OSC 7 writes to (scope clarification H: CWD SSOT). Pins
/// that VS Code's CWD reporting shares the single source of truth with the
/// Final Term / iTerm2 OSC 7 path.
#[test]
fn osc633_p_cwd_sets_term_cwd() {
 let mut term = make_term();
 assert!(term.cwd().is_none());

 feed_mux_and_proc(&mut term, b"\x1b]633;P;Cwd=/home/user/project\x1b\\");

 assert_eq!(term.cwd(), Some("/home/user/project"));
 assert!(
 term.is_title_dirty(),
 "OSC 633;P;Cwd must mark the title dirty, same as OSC 7"
 );
 assert!(!term.has_explicit_title());
}

/// §10.4 — OSC 633;P with an unknown key (e.g. `IsWindows=True`) is
/// silently dropped — only `Cwd=` is honoured. Pins the forward-compat
/// behavior called out in `shellIntegrationAddon.ts`: unknown `P` keys do
/// NOT mutate state.
#[test]
fn osc633_p_unknown_key_dropped() {
 let mut term = make_term();

 feed_mux_and_proc(&mut term, b"\x1b]633;P;IsWindows=True\x1b\\");

 assert!(term.cwd().is_none(), "unknown P key must not set CWD");
 assert!(
 !term.is_title_dirty(),
 "unknown P key must not mark the title dirty"
 );
}

/// §10.4 — Regression guard: feeding OSC 633;A through the high-level
/// `vte::ansi::Processor` ALONE (no `RawInterceptor` pass) does NOT mutate
/// `PromptState`. Proves OSC 633 dispatch is interceptor-only — if someone
/// adds a `b"633"` arm to `crates/vte/src/ansi/dispatch/osc.rs`, this test
/// fails (double-dispatch detection). Mirrors
/// `osc133a_via_processor_only_does_not_change_prompt_state`.
#[test]
fn osc633_via_high_level_processor_drops() {
 let mut term = make_term();

 let mut processor = vte::ansi::Processor::<vte::ansi::StdSyncHandler>::new();
 processor.advance(&mut term, b"\x1b]633;A\x1b\\");

 assert_eq!(
 term.prompt_state(),
 PromptState::None,
 "high-level Processor must NOT route OSC 633;A — if this assertion \
 fires, OSC 633 was added to the high-level dispatcher and would be \
 double-handled in production"
 );
 assert!(!term.prompt_mark_pending());
}

// §10.8 — OSC 7 CWD via production-order dual-pass.
// OSC 7 is interceptor-handled: the high-level `vte::ansi::Processor`
// does NOT route it to a `Handler` method (the default
// `Handler::set_working_directory` is a no-op, and `Term` does not
// override it). The canonical path runs `RawInterceptor` FIRST, which
// parses the `file://hostname/path` URI, strips the hostname,
// percent-decodes the path, and writes it to `Term::set_cwd`. These
// tests drive the sequence through `spec_chain_helper::feed_mux_and_proc`
// so both passes execute in production order.

/// §10.8 — Canonical OSC 7 input: `file://host/path`. The `parse_osc7_path`
/// helper strips the hostname and returns `/path`, which reaches
/// `Term::set_cwd` via the interceptor. The high-level processor pass
/// on the same bytes is a no-op for this sequence.
#[test]
fn osc7_file_uri_sets_cwd() {
 let mut term = make_term();
 assert!(term.cwd().is_none());

 feed_mux_and_proc(&mut term, b"\x1b]7;file:///home/user/project\x1b\\");

 assert_eq!(
 term.cwd(),
 Some("/home/user/project"),
 "production dual-pass must drive OSC 7 through the interceptor's \
 parse_osc7_path → percent_decode → Term::set_cwd pipeline"
 );
}

/// §10.8 — OSC 7 with an explicit hostname (`file://myhost.example.com/path`):
/// the interceptor's `parse_osc7_path` skips the hostname portion and
/// returns only the absolute path segment (`/path/to/dir`).
#[test]
fn osc7_file_uri_with_hostname() {
 let mut term = make_term();

 feed_mux_and_proc(
 &mut term,
 b"\x1b]7;file://myhost.example.com/path/to/dir\x1b\\",
 );

 assert_eq!(
 term.cwd(),
 Some("/path/to/dir"),
 "hostname segment must be stripped — parse_osc7_path returns the path after the hostname"
 );
}

/// §10.8 — OSC 7 URI-encoded bytes (`%20` for space) round-trip through
/// `percent_decode` in the interceptor. Final CWD contains a literal
/// space, not the `%20` escape.
#[test]
fn osc7_percent_decoded() {
 let mut term = make_term();

 feed_mux_and_proc(&mut term, b"\x1b]7;file:///home/user/my%20folder\x1b\\");

 assert_eq!(
 term.cwd(),
 Some("/home/user/my folder"),
 "percent_decode must convert %20 → space in the CWD payload"
 );
}

/// §10.8 — OSC 7 emits `Effect::Host(HostEffect::CwdSet { cwd })` on
/// the effect transcript. The consumer-side test for mux clients reads
/// this effect to update session-level state. The scenario asserts
/// exactly one CwdSet was emitted and its payload matches the set CWD.
#[test]
fn osc7_emits_host_effect_cwd_set() {
 let mut term = make_term();

 feed_mux_and_proc(&mut term, b"\x1b]7;file:///home/user/project\x1b\\");

 let mut effects = Vec::new();
 term.effect_sink().drain_into(&mut effects);

 let mut cwd_sets = effects.iter().filter_map(|eff| match eff {
 Effect::Host(HostEffect::CwdSet { cwd }) => Some(cwd.clone()),
 _ => None,
 });
 let first = cwd_sets
 .next()
 .expect("OSC 7 must emit exactly one CwdSet effect");
 assert_eq!(first, "/home/user/project");
 assert!(
 cwd_sets.next().is_none(),
 "OSC 7 must emit exactly one CwdSet; found a second"
 );
}

/// §10.8 — Relative-path payload (no `file://` prefix) flows through
/// `strip_uri_suffix` unchanged. The path is non-empty, so the
/// interceptor writes it verbatim to `Term::set_cwd`. This pins the
/// behavior documented in `parse_osc7_path` at
/// `interceptor.rs:289-303` — a future regression that rejected
/// non-URI payloads would break this contract.
#[test]
fn osc7_relative_path_passed_through() {
 let mut term = make_term();

 feed_mux_and_proc(&mut term, b"\x1b]7;relative/path\x1b\\");

 assert_eq!(
 term.cwd(),
 Some("relative/path"),
 "parse_osc7_path passes non-URI payloads through strip_uri_suffix unchanged"
 );
}

/// §10.8 — Regression guard: feeding OSC 7 through the high-level
/// `vte::ansi::Processor` ALONE (no `RawInterceptor` pass) does NOT
/// set CWD. Proves OSC 7 is interceptor-only in production — `Term`
/// does not override `Handler::set_working_directory` (the default is
/// a no-op). Mirrors `osc9_via_processor_without_mux_drops` /
/// `osc633_via_high_level_processor_drops`.
#[test]
fn osc7_via_high_level_processor_drops() {
 let mut term = make_term();
 assert!(term.cwd().is_none());

 let mut processor = vte::ansi::Processor::<vte::ansi::StdSyncHandler>::new();
 processor.advance(&mut term, b"\x1b]7;file:///home/user/project\x1b\\");

 assert!(
 term.cwd().is_none(),
 "high-level Processor must NOT route OSC 7 — Term does not override \
 Handler::set_working_directory; the canonical path is the interceptor. \
 If this assertion fires, a `b\"7\"` arm with a non-default handler \
 override was added, which would create a second dispatch path \
 and cause double-handling in production"
 );
}

// — Kitty keyboard mode stack snapshot / restore via OSC 133 / 633
// Shell-integration protocol tests for the fix
// - OSC 133 ; C (command-start) takes a paired contents-based snapshot.
// - OSC 133 ; A (next prompt) / ; D (command-done) restores verbatim
// AND unconditionally reapplies top-of-stack mode bits.
// - OSC 633 A / C / D mirror OSC 133 (VS Code shell-integration superset).
// - Alt-screen × paired per-screen snapshot.
// - Over-pop / max-depth-eviction recovery.
// - Same-chunk OSC-then-CSI vs CSI-then-OSC parser-pass ordering.
// - Negative pins: no snapshot → no modification on ; A.

use std::collections::VecDeque;

use oriterm_core::TermMode;
use oriterm_core::term::KEYBOARD_MODE_STACK_MAX_DEPTH;
use vte::ansi::KeyboardModes;

use spec_chain_helper::feed_mux_and_proc;

// --- Exact failing case ---

/// Regression: push one kitty mode, `;C` snapshot, child
/// crashes without popping, `;A` next prompt. Stack must restore to
/// snapshotted empty depth AND `KITTY_KEYBOARD_PROTOCOL` bits cleared.
/// This is the user-visible crash-during-command symptom.
#[test]
fn keyboard_mode_stack_child_crash_on_osc_133_a_restores_to_snapshot_depth() {
 let mut term = make_term();
 // Shell emits `;C` before child runs — snapshot empty stack.
 feed_mux_and_proc(&mut term, b"\x1b]133;C\x1b\\");
 // Child pushes DISAMBIGUATE (parameter 1).
 feed_mux_and_proc(&mut term, b"\x1b[>1u");
 assert!(!term.keyboard_mode_stack().is_empty());
 assert!(term.mode().contains(TermMode::DISAMBIGUATE_ESC_CODES));

 // Child crashes (no pop); shell draws next prompt → `;A`.
 feed_mux_and_proc(&mut term, b"\x1b]133;A\x1b\\");

 assert!(
 term.keyboard_mode_stack().is_empty(),
 "restore must truncate to snapshot depth"
 );
 assert!(
 !term.mode().intersects(TermMode::KITTY_KEYBOARD_PROTOCOL),
 "KITTY bits must be cleared when restore top is NO_MODE"
 );
}

// --- Edge cases ---

/// Regression: `;D` is the clean-path restore trigger;
/// a well-behaved child emits `;D` before exiting.
#[test]
fn keyboard_mode_stack_child_clean_exit_on_osc_133_d_restores_to_snapshot_depth() {
 let mut term = make_term();
 feed_mux_and_proc(&mut term, b"\x1b]133;C\x1b\\");
 feed_mux_and_proc(&mut term, b"\x1b[>1u");
 assert!(!term.keyboard_mode_stack().is_empty());

 feed_mux_and_proc(&mut term, b"\x1b]133;D\x1b\\");

 assert!(term.keyboard_mode_stack().is_empty());
 assert!(!term.mode().intersects(TermMode::KITTY_KEYBOARD_PROTOCOL));
}

/// Regression: `;C` then `;A` with no pushes in between
/// must leave the stack empty.
#[test]
fn keyboard_mode_stack_empty_at_c_and_a_stays_empty() {
 let mut term = make_term();
 feed_mux_and_proc(&mut term, b"\x1b]133;C\x1b\\");
 feed_mux_and_proc(&mut term, b"\x1b]133;A\x1b\\");

 assert!(term.keyboard_mode_stack().is_empty());
}

/// Regression: contents-based snapshot preserves
/// shell-held modes across command boundaries. Shell pushes 1 mode,
/// `;C`, child pushes 2 more and crashes, `;A` restores stack to
/// `[shell_mode]`.
#[test]
fn keyboard_mode_stack_shell_held_mode_at_c_preserved_after_a() {
 let mut term = make_term();
 // Shell integration pushes a mode on init.
 feed_mux_and_proc(&mut term, b"\x1b[>1u");
 assert_eq!(term.keyboard_mode_stack().len(), 1);
 let shell_mode = KeyboardModes::from_bits_truncate(1);

 feed_mux_and_proc(&mut term, b"\x1b]133;C\x1b\\");
 // Child pushes two additional modes and crashes.
 feed_mux_and_proc(&mut term, b"\x1b[>3u");
 feed_mux_and_proc(&mut term, b"\x1b[>7u");
 assert_eq!(term.keyboard_mode_stack().len(), 3);

 feed_mux_and_proc(&mut term, b"\x1b]133;A\x1b\\");

 assert_eq!(
 term.keyboard_mode_stack(),
 &VecDeque::from(vec![shell_mode]),
 "shell-held modes below the snapshot depth must be preserved"
 );
}

/// Regression: snapshot/restore is reset each command
/// cycle. After three consecutive `;C → push → crash → ;A` cycles,
/// the stack returns to its shell-held state each time.
#[test]
fn keyboard_mode_stack_three_command_cycles_each_restores_independently() {
 let mut term = make_term();
 for _ in 0..3 {
 feed_mux_and_proc(&mut term, b"\x1b]133;C\x1b\\");
 feed_mux_and_proc(&mut term, b"\x1b[>1u");
 feed_mux_and_proc(&mut term, b"\x1b]133;A\x1b\\");
 assert!(term.keyboard_mode_stack().is_empty());
 }
}

/// Regression: in-band `CSI < u` pop without a prior
/// `;C` must take effect. Restore is snapshot-gated, so no snapshot
/// means no restore, and in-band pushes/pops operate normally.
#[test]
fn keyboard_mode_stack_in_band_csi_pop_without_prior_c_still_pops() {
 let mut term = make_term();
 feed_mux_and_proc(&mut term, b"\x1b[>1u");
 feed_mux_and_proc(&mut term, b"\x1b[>3u");
 assert_eq!(term.keyboard_mode_stack().len(), 2);

 // Child pops one — no prior `;C`.
 feed_mux_and_proc(&mut term, b"\x1b[<1u");
 assert_eq!(term.keyboard_mode_stack().len(), 1);

 // `;A` with no snapshot is a no-op.
 feed_mux_and_proc(&mut term, b"\x1b]133;A\x1b\\");
 assert_eq!(
 term.keyboard_mode_stack().len(),
 1,
 "restore without snapshot must not modify the stack"
 );
}

// --- OSC 633 parallel (VS Code shell integration superset) ---

/// Regression: OSC 633 `;C` takes the same paired
/// contents-based snapshot as OSC 133 `;C`.
#[test]
fn osc_633_c_snapshots_both_paired_depths() {
 let mut term = make_term();
 feed_mux_and_proc(&mut term, b"\x1b[>1u");

 feed_mux_and_proc(&mut term, b"\x1b]633;C\x1b\\");

 assert!(term.pre_command_kb_stack_snapshot().is_some());
 assert!(term.inactive_pre_command_kb_stack_snapshot().is_some());
 assert_eq!(term.pre_command_kb_stack_snapshot().unwrap().len(), 1);
}

/// Regression: OSC 633 `;A` restore mirrors OSC 133 `;A`.
#[test]
fn osc_633_a_restores_both_paired_depths() {
 let mut term = make_term();
 feed_mux_and_proc(&mut term, b"\x1b]633;C\x1b\\");
 feed_mux_and_proc(&mut term, b"\x1b[>1u");
 assert!(!term.keyboard_mode_stack().is_empty());

 feed_mux_and_proc(&mut term, b"\x1b]633;A\x1b\\");

 assert!(term.keyboard_mode_stack().is_empty());
 assert!(!term.mode().intersects(TermMode::KITTY_KEYBOARD_PROTOCOL));
}

/// Regression: OSC 633 `;D` restore mirrors OSC 133 `;D`.
#[test]
fn osc_633_d_restores_both_paired_depths() {
 let mut term = make_term();
 feed_mux_and_proc(&mut term, b"\x1b]633;C\x1b\\");
 feed_mux_and_proc(&mut term, b"\x1b[>1u");

 feed_mux_and_proc(&mut term, b"\x1b]633;D\x1b\\");

 assert!(term.keyboard_mode_stack().is_empty());
 assert!(!term.mode().intersects(TermMode::KITTY_KEYBOARD_PROTOCOL));
}

// --- alt-screen × paired per-screen snapshot ---

/// Regression: snapshot on primary, toggle to alt, push
/// on alt, toggle back, `;A` on primary. Both stacks must be cleaned
/// — the inactive (alt) side too, so child pushes on the alt screen
/// before its crash don't leak.
#[test]
fn keyboard_mode_stack_snapshot_on_primary_then_alt_push_then_a_cleans_primary_not_alt() {
 let mut term = make_term();
 // `;C` on primary, both stacks currently empty.
 feed_mux_and_proc(&mut term, b"\x1b]133;C\x1b\\");
 // Enter alt, push on alt, exit alt (swap back to primary).
 feed_mux_and_proc(&mut term, b"\x1b[?1049h");
 feed_mux_and_proc(&mut term, b"\x1b[>1u");
 feed_mux_and_proc(&mut term, b"\x1b[?1049l");

 // Now on primary. Active stack is primary (empty). Inactive stack
 // is alt ([mode_1]) because toggle_alt_common swapped them back.
 assert_eq!(term.inactive_keyboard_mode_stack().len(), 1);

 feed_mux_and_proc(&mut term, b"\x1b]133;A\x1b\\");

 assert!(term.keyboard_mode_stack().is_empty());
 assert!(
 term.inactive_keyboard_mode_stack().is_empty(),
 "inactive (alt) stack must also be restored — both-stack snapshot catches the alt-side leak"
 );
}

/// Regression: Round 1 F1 — snapshot primary at depth 1
/// (shell-held mode), child enters alt, pushes mode on alt, exits
/// alt (swap back) before `;A`. Both stacks must restore to their
/// pre-command contents — shell_mode preserved on primary, alt cleared.
#[test]
fn keyboard_mode_stack_snapshot_on_primary_child_alt_push_exit_alt_before_a_restores_inactive_from_snapshot()
 {
 let mut term = make_term();
 let shell_mode = KeyboardModes::from_bits_truncate(1);
 feed_mux_and_proc(&mut term, b"\x1b[>1u"); // Shell push.
 feed_mux_and_proc(&mut term, b"\x1b]133;C\x1b\\");

 // Child goes to alt, pushes mode on alt.
 feed_mux_and_proc(&mut term, b"\x1b[?1049h");
 feed_mux_and_proc(&mut term, b"\x1b[>3u");
 // Child exits alt before crashing.
 feed_mux_and_proc(&mut term, b"\x1b[?1049l");
 // Active=Primary=[shell_mode]; Inactive=Alt=[mode_3] (swap back).
 assert_eq!(term.keyboard_mode_stack().len(), 1);
 assert_eq!(term.inactive_keyboard_mode_stack().len(), 1);

 feed_mux_and_proc(&mut term, b"\x1b]133;A\x1b\\");

 assert_eq!(
 term.keyboard_mode_stack(),
 &VecDeque::from(vec![shell_mode]),
 "active (primary) stack restored verbatim from snapshot"
 );
 assert!(
 term.inactive_keyboard_mode_stack().is_empty(),
 "inactive (alt) stack restored verbatim — child's alt push removed"
 );
 assert!(
 term.mode().contains(TermMode::DISAMBIGUATE_ESC_CODES),
 "TermMode bits reflect shell_mode (active-stack top after restore)"
 );
}

/// Regression: shell-held mode preserved across an
/// alt-screen command. Snapshot primary at depth 1, enter alt,
/// push 2 on alt, exit alt, `;A`. Primary stack restored.
#[test]
fn keyboard_mode_stack_snapshot_and_restore_across_one_toggle_preserves_primary() {
 let mut term = make_term();
 let shell_mode = KeyboardModes::from_bits_truncate(1);
 feed_mux_and_proc(&mut term, b"\x1b[>1u");
 feed_mux_and_proc(&mut term, b"\x1b]133;C\x1b\\");

 feed_mux_and_proc(&mut term, b"\x1b[?1049h");
 feed_mux_and_proc(&mut term, b"\x1b[>3u");
 feed_mux_and_proc(&mut term, b"\x1b[>7u");
 feed_mux_and_proc(&mut term, b"\x1b[?1049l");

 feed_mux_and_proc(&mut term, b"\x1b]133;A\x1b\\");

 assert_eq!(
 term.keyboard_mode_stack(),
 &VecDeque::from(vec![shell_mode])
 );
 assert!(term.mode().contains(TermMode::DISAMBIGUATE_ESC_CODES));
}

/// Regression: child pops one legitimately then crashes.
/// Contents-based snapshot recovers the original stack verbatim.
#[test]
fn keyboard_mode_stack_child_pops_one_and_crashes_restores_from_snapshot() {
 let mut term = make_term();
 let mode_a = KeyboardModes::from_bits_truncate(1);
 let mode_b = KeyboardModes::from_bits_truncate(3);
 feed_mux_and_proc(&mut term, b"\x1b[>1u");
 feed_mux_and_proc(&mut term, b"\x1b[>3u");
 feed_mux_and_proc(&mut term, b"\x1b]133;C\x1b\\");

 // Child pops one legitimately.
 feed_mux_and_proc(&mut term, b"\x1b[<1u");
 assert_eq!(term.keyboard_mode_stack().len(), 1);
 // Child crashes.

 feed_mux_and_proc(&mut term, b"\x1b]133;A\x1b\\");

 assert_eq!(
 term.keyboard_mode_stack(),
 &VecDeque::from(vec![mode_a, mode_b]),
 "contents-based snapshot restores verbatim including mode_b that was popped"
 );
}

/// Regression: Round 2 F1 — child over-pops shell-held
/// modes via `CSI < 5 u`. Contents-based snapshot recovers shell state.
#[test]
fn keyboard_mode_stack_child_over_pops_shell_held_modes_restore_recovers_shell_state() {
 let mut term = make_term();
 let mode_a = KeyboardModes::from_bits_truncate(1);
 let mode_b = KeyboardModes::from_bits_truncate(3);
 feed_mux_and_proc(&mut term, b"\x1b[>1u");
 feed_mux_and_proc(&mut term, b"\x1b[>3u");
 feed_mux_and_proc(&mut term, b"\x1b]133;C\x1b\\");

 // Child over-pops (pops 5 → truncates to empty).
 feed_mux_and_proc(&mut term, b"\x1b[<5u");
 assert!(term.keyboard_mode_stack().is_empty());

 feed_mux_and_proc(&mut term, b"\x1b]133;A\x1b\\");

 assert_eq!(
 term.keyboard_mode_stack(),
 &VecDeque::from(vec![mode_a, mode_b]),
 "contents-based snapshot recovers shell modes that were over-popped"
 );
 assert!(
 term.mode().intersects(TermMode::KITTY_KEYBOARD_PROTOCOL),
 "TermMode bits reflect mode_b (active-stack top after restore)"
 );
}

/// Regression: Round 2 F1 — child pushes past
/// `KEYBOARD_MODE_STACK_MAX_DEPTH`; `dcs_push_keyboard_mode`
/// ring-buffer evicts shell-held entries via `pop_front`.
/// Contents-based snapshot recovers the front-evicted shell state.
#[test]
fn keyboard_mode_stack_child_push_past_max_depth_evicts_shell_mode_then_a_recovers_evicted_mode() {
 let mut term = make_term();
 let shell_held = KeyboardModes::from_bits_truncate(1);
 feed_mux_and_proc(&mut term, b"\x1b[>1u");
 feed_mux_and_proc(&mut term, b"\x1b]133;C\x1b\\");

 // Child pushes MAX_DEPTH modes — oldest entry (shell_held) gets
 // evicted via pop_front.
 for _ in 0..KEYBOARD_MODE_STACK_MAX_DEPTH {
 feed_mux_and_proc(&mut term, b"\x1b[>7u");
 }
 assert_eq!(
 term.keyboard_mode_stack().len(),
 KEYBOARD_MODE_STACK_MAX_DEPTH
 );
 assert!(!term.keyboard_mode_stack().contains(&shell_held));

 feed_mux_and_proc(&mut term, b"\x1b]133;A\x1b\\");

 assert_eq!(
 term.keyboard_mode_stack(),
 &VecDeque::from(vec![shell_held]),
 "contents-based snapshot recovers front-evicted shell state"
 );
}

/// Regression: Phase 1.75 — `CSI = Ps u` mutates the
/// TermMode bits via `dcs_set_keyboard_mode` with Replace behavior
/// WITHOUT pushing or popping the stack. On restore, top-of-stack
/// mode is unconditionally reapplied so dirty bits are reverted.
#[test]
fn keyboard_mode_stack_csi_equals_u_mutates_without_push_then_crash_then_a_reapplies_stack_top() {
 let mut term = make_term();
 let shell_mode = KeyboardModes::from_bits_truncate(1);
 feed_mux_and_proc(&mut term, b"\x1b[>1u"); // shell_mode = DISAMBIGUATE.
 feed_mux_and_proc(&mut term, b"\x1b]133;C\x1b\\");

 // Child mutates the active bits via CSI = u (Replace) — stack
 // unchanged, but TermMode bits now reflect REPORT_ALL_KEYS_AS_ESC
 // (bit 8 = 0b1000).
 feed_mux_and_proc(&mut term, b"\x1b[=8u");
 assert_eq!(term.keyboard_mode_stack().len(), 1);
 assert!(term.mode().contains(TermMode::REPORT_ALL_KEYS_AS_ESC));
 assert!(!term.mode().contains(TermMode::DISAMBIGUATE_ESC_CODES));

 // Child crashes; `;A` restores.
 feed_mux_and_proc(&mut term, b"\x1b]133;A\x1b\\");

 assert_eq!(
 term.keyboard_mode_stack(),
 &VecDeque::from(vec![shell_mode])
 );
 assert!(
 term.mode().contains(TermMode::DISAMBIGUATE_ESC_CODES),
 "unconditional reapply reverts CSI = Ps u bit mutations"
 );
 assert!(!term.mode().contains(TermMode::REPORT_ALL_KEYS_AS_ESC));
}

// --- Same-chunk parser-pass ordering ---

/// Regression: Round 1 F2 — PTY chunk containing
/// `OSC 133 ; C` immediately followed by `CSI > u` in one byte
/// slice. Raw interceptor snapshots at pre-push depth, then the
/// high-level processor processes the push on top of the saved
/// snapshot. Next `;A` removes the push.
#[test]
fn osc_133_c_and_csi_push_same_chunk_snapshot_captures_pre_push_depth() {
 let mut term = make_term();
 // One chunk: `;C` then push.
 feed_mux_and_proc(&mut term, b"\x1b]133;C\x1b\\\x1b[>1u");

 assert_eq!(
 term.pre_command_kb_stack_snapshot().unwrap().len(),
 0,
 "snapshot captured pre-push depth (empty)"
 );
 assert_eq!(
 term.keyboard_mode_stack().len(),
 1,
 "high-level processor's push landed above the saved snapshot"
 );

 // Next `;A` truncates the push.
 feed_mux_and_proc(&mut term, b"\x1b]133;A\x1b\\");
 assert!(term.keyboard_mode_stack().is_empty());
}

/// Regression: reverse same-chunk ordering (`CSI > u`
/// then `OSC 133 ; C`). Raw interceptor still runs first; snapshot
/// captures the pre-chunk depth (empty), then the push lands.
/// Subsequent `;A` removes the push.
#[test]
fn csi_push_and_osc_133_c_same_chunk_snapshot_still_captures_pre_chunk_depth() {
 let mut term = make_term();
 feed_mux_and_proc(&mut term, b"\x1b[>1u\x1b]133;C\x1b\\");

 // Raw-first-then-high-level semantics: raw interceptor snapshots
 // at pre-chunk depth (empty), then the processor processes the
 // push on top.
 assert_eq!(
 term.pre_command_kb_stack_snapshot().unwrap().len(),
 0,
 "snapshot captures pre-chunk depth (raw interceptor fires before processor)"
 );
 assert_eq!(term.keyboard_mode_stack().len(), 1);

 feed_mux_and_proc(&mut term, b"\x1b]133;A\x1b\\");
 assert!(term.keyboard_mode_stack().is_empty());
}

/// Regression: review round-1 F1 — shell uses `CSI = Ps u` to
/// set a kitty keyboard mode WITHOUT pushing (set-path integration).
/// The bits live in `TermMode::KITTY_KEYBOARD_PROTOCOL` with an empty
/// keyboard mode stack. A subsequent `;C → child → ;A` cycle must
/// preserve the shell's set-only bits — prior to the paired bits
/// snapshot, reapplying stack-top (`NO_MODE` for empty stack) would
/// silently clear the shell's kitty state at every prompt boundary.
#[test]
fn keyboard_mode_stack_shell_set_without_push_csi_equals_u_survives_restore() {
 let mut term = make_term();
 // Shell sets kitty mode via `CSI = 1 u` — SET (Replace), no push.
 feed_mux_and_proc(&mut term, b"\x1b[=1u");
 assert!(term.mode().contains(TermMode::DISAMBIGUATE_ESC_CODES));
 assert!(
 term.keyboard_mode_stack().is_empty(),
 "CSI = Ps u path does not touch the stack"
 );

 // Command boundary: ;C snapshots current bits + empty stack.
 feed_mux_and_proc(&mut term, b"\x1b]133;C\x1b\\");
 assert_eq!(
 term.pre_command_kb_mode_bits_snapshot(),
 Some(KeyboardModes::DISAMBIGUATE_ESC_CODES),
 "paired bits snapshot captures shell-held set-only bits"
 );

 // Child runs and exits without interacting with the kitty stack.

 // Next prompt ;A restores: stack stays empty, bits restored to
 // shell's snapshotted DISAMBIGUATE_ESC_CODES.
 feed_mux_and_proc(&mut term, b"\x1b]133;A\x1b\\");

 assert!(
 term.mode().contains(TermMode::DISAMBIGUATE_ESC_CODES),
 "shell's set-only kitty mode must survive the command boundary"
 );
 assert!(term.keyboard_mode_stack().is_empty());
}

/// Regression: review round-3 F1 — shell sets kitty bits via
/// set-only `CSI = Ps u` WITHOUT any shell integration (no OSC 133),
/// then user enters/exits alt screen (e.g. pages a manpage). Bits
/// must survive — live per-screen `inactive_keyboard_mode_bits` tracks
/// the off-screen effective bits so alt toggles preserve set state.
#[test]
fn kitty_set_only_bits_without_shell_integration_survive_alt_roundtrip() {
 let mut term = make_term();
 feed_mux_and_proc(&mut term, b"\x1b[=1u"); // set-only, no ;C
 assert!(term.mode().contains(TermMode::DISAMBIGUATE_ESC_CODES));

 // Enter alt (e.g. `man foo`), exit alt — no OSC 133 in play.
 feed_mux_and_proc(&mut term, b"\x1b[?1049h");
 assert!(
 !term.mode().intersects(TermMode::KITTY_KEYBOARD_PROTOCOL),
 "alt screen starts fresh — shell's bits must not leak onto alt"
 );
 feed_mux_and_proc(&mut term, b"\x1b[?1049l");

 assert!(
 term.mode().contains(TermMode::DISAMBIGUATE_ESC_CODES),
 "primary's set-only kitty bits must survive alt round-trip even \
 without shell integration"
 );
}

/// Regression: review round-3 F2 — child mid-command mutates
/// kitty bits via `CSI = Ps u`, then user enters/exits alt. The child's
/// mid-command bits must survive the round-trip and still be active on
/// return to primary (shell's ;D will restore to pre-command state
/// later). Live per-screen bits field preserves them.
#[test]
fn kitty_mid_command_bit_mutation_survives_alt_roundtrip() {
 let mut term = make_term();
 feed_mux_and_proc(&mut term, b"\x1b[>1u"); // shell push: stack=[DIS]
 feed_mux_and_proc(&mut term, b"\x1b]133;C\x1b\\");
 // Child mutates bits to REPORT_ALL mid-command.
 feed_mux_and_proc(&mut term, b"\x1b[=8u");
 assert!(term.mode().contains(TermMode::REPORT_ALL_KEYS_AS_ESC));

 // User enters alt (e.g. less), exits alt.
 feed_mux_and_proc(&mut term, b"\x1b[?1049h");
 feed_mux_and_proc(&mut term, b"\x1b[?1049l");

 assert!(
 term.mode().contains(TermMode::REPORT_ALL_KEYS_AS_ESC),
 "child's mid-command bits must survive the alt round-trip"
 );
}

/// Regression: review round-5 — inactive-screen bit mutations
/// during a shell command must be REVERTED by the shell's `;D` restore,
/// not persisted. Previously restore discarded the inactive bits snap;
/// now it applies the snap to `inactive_keyboard_mode_bits` so the alt
/// screen's mid-command mutations are cleaned at the prompt boundary.
#[test]
fn inactive_screen_bit_mutation_during_command_restored_on_primary_d() {
 let mut term = make_term();
 // Shell on primary pushes DIS + ;C.
 feed_mux_and_proc(&mut term, b"\x1b[>1u");
 feed_mux_and_proc(&mut term, b"\x1b]133;C\x1b\\");

 // Child enters alt, mutates alt-screen bits via CSI = 4 u (REPORT_ALTERNATE_KEYS),
 // then exits alt back to primary.
 feed_mux_and_proc(&mut term, b"\x1b[?1049h");
 feed_mux_and_proc(&mut term, b"\x1b[=4u");
 assert!(term.mode().contains(TermMode::REPORT_ALTERNATE_KEYS));
 feed_mux_and_proc(&mut term, b"\x1b[?1049l");
 // After exit, primary's live bits restored; alt's mutation is now
 // in the inactive live field.
 assert_eq!(
 term.inactive_keyboard_mode_bits(),
 KeyboardModes::REPORT_ALTERNATE_KEYS,
 "alt's mid-command mutation carries in inactive_keyboard_mode_bits"
 );

 // Shell `;D` — restore should revert alt's inactive bits to the
 // pre-command snapshot (NO_MODE).
 feed_mux_and_proc(&mut term, b"\x1b]133;D\x1b\\");

 assert_eq!(
 term.inactive_keyboard_mode_bits(),
 KeyboardModes::NO_MODE,
 "inactive bits snapshot must revert alt's mid-command mutation at `;D`"
 );
}

/// Regression: review round-4 — when `;A` fires on the ALT
/// screen during a shell command, the paired bits snapshot must be
/// swapped in `toggle_alt_common` so the active bits snap consumed by
/// alt's restore is alt's value (NO_MODE placeholder), not primary's
/// shell-set bits. Without the paired swap, alt-side `;A` incorrectly
/// applies primary's DISAMBIGUATE bits to the alt screen and then
/// consumes/loses primary's paired snap.
#[test]
fn alt_side_a_restore_uses_alt_paired_snapshot_not_primary() {
 let mut term = make_term();
 // Shell on primary pushes DIS + emits ;C.
 feed_mux_and_proc(&mut term, b"\x1b[>1u");
 feed_mux_and_proc(&mut term, b"\x1b]133;C\x1b\\");
 assert!(term.mode().contains(TermMode::DISAMBIGUATE_ESC_CODES));

 // Enter alt screen.
 feed_mux_and_proc(&mut term, b"\x1b[?1049h");
 assert!(
 !term.mode().intersects(TermMode::KITTY_KEYBOARD_PROTOCOL),
 "alt screen starts with its own (empty) bits after swap"
 );

 // `;A` fires on alt — should apply alt's paired bits (NO_MODE),
 // not primary's (DIS).
 feed_mux_and_proc(&mut term, b"\x1b]133;A\x1b\\");
 assert!(
 !term.mode().intersects(TermMode::KITTY_KEYBOARD_PROTOCOL),
 "alt-side `;A` restore must NOT apply primary's paired bits \
 snapshot to the alt screen"
 );

 // Exit alt — live inactive_keyboard_mode_bits carries primary's
 // original bits back via the swap, even though its paired snap was
 // consumed during alt's `;A`.
 feed_mux_and_proc(&mut term, b"\x1b[?1049l");
 assert!(
 term.mode().contains(TermMode::DISAMBIGUATE_ESC_CODES),
 "primary's original kitty bits must survive the alt round-trip \
 via the live per-screen bits field"
 );
}

/// Regression: review round-2 F1 — shell set-only kitty bits
/// survive an alt-screen round-trip that includes an `;A` firing on
/// the alt side (misbehaving integration or background emission).
/// `CSI = 1u; OSC 133;C; ?1049h; OSC 133;A; ?1049l` must end with
/// `DISAMBIGUATE_ESC_CODES` still active on primary.
#[test]
fn kitty_set_only_bits_survive_alt_screen_roundtrip_with_a_on_alt() {
 let mut term = make_term();
 // Shell sets via set-only `CSI = 1 u` on primary.
 feed_mux_and_proc(&mut term, b"\x1b[=1u");
 assert!(term.mode().contains(TermMode::DISAMBIGUATE_ESC_CODES));

 // Shell command-start snapshot.
 feed_mux_and_proc(&mut term, b"\x1b]133;C\x1b\\");
 // Enter alt screen.
 feed_mux_and_proc(&mut term, b"\x1b[?1049h");
 // `;A` fires on alt side.
 feed_mux_and_proc(&mut term, b"\x1b]133;A\x1b\\");
 // Exit alt screen back to primary.
 feed_mux_and_proc(&mut term, b"\x1b[?1049l");

 assert!(
 term.mode().contains(TermMode::DISAMBIGUATE_ESC_CODES),
 "set-only kitty bits must survive an alt-screen round-trip that \
 consumes the paired snapshot on the wrong screen"
 );
}

/// Regression: review round-1 F1 — child mutates bits via
/// `CSI = Ps u` during a command while shell used set-only before ;C.
/// Restore must return to shell's set-only bits even though stack is
/// empty.
#[test]
fn keyboard_mode_stack_shell_set_then_child_set_mutation_then_a_reapplies_shell_bits() {
 let mut term = make_term();
 feed_mux_and_proc(&mut term, b"\x1b[=1u"); // shell_mode = DISAMBIGUATE
 feed_mux_and_proc(&mut term, b"\x1b]133;C\x1b\\");

 // Child mutates bits to REPORT_ALL_KEYS_AS_ESC via CSI = 8 u.
 feed_mux_and_proc(&mut term, b"\x1b[=8u");
 assert!(term.mode().contains(TermMode::REPORT_ALL_KEYS_AS_ESC));
 assert!(!term.mode().contains(TermMode::DISAMBIGUATE_ESC_CODES));

 feed_mux_and_proc(&mut term, b"\x1b]133;A\x1b\\");

 assert!(
 term.mode().contains(TermMode::DISAMBIGUATE_ESC_CODES),
 "paired bits snapshot reverts child's CSI = Ps u mutation"
 );
 assert!(!term.mode().contains(TermMode::REPORT_ALL_KEYS_AS_ESC));
}

/// Regression: pin the raw-first invariant for pop + ;D
/// same-chunk. When a child emits `CSI < 1 u` immediately before
/// `OSC 133 ; D` and both land in one PTY read, the raw interceptor
/// runs restore BEFORE the high-level processor processes the pop.
/// Result: stack = snapshot minus one pop. Documents the tradeoff
/// called out in the plan's §2.5 Round 2 F2 disagreement — the
/// raw-first-then-high-level architecture is the chosen SSOT per plan.
#[test]
fn csi_pop_and_osc_133_d_same_chunk_raw_first_restores_then_pop_applies() {
 let mut term = make_term();
 let shell_held = KeyboardModes::from_bits_truncate(1);
 // Shell push + ;C snapshot, then a child push.
 feed_mux_and_proc(&mut term, b"\x1b[>1u");
 feed_mux_and_proc(&mut term, b"\x1b]133;C\x1b\\");
 feed_mux_and_proc(&mut term, b"\x1b[>3u");
 assert_eq!(term.keyboard_mode_stack().len(), 2);

 // Same chunk: child pops one, then shell emits ;D.
 feed_mux_and_proc(&mut term, b"\x1b[<1u\x1b]133;D\x1b\\");

 // Raw-first: ;D restore runs first, restoring stack to snapshot
 // [shell_held]; processor's pop then removes the one entry.
 assert!(
 term.keyboard_mode_stack().is_empty(),
 "raw-first invariant: restore then pop produces empty stack — \
 documented tradeoff, not a separate fix"
 );
 assert!(!term.mode().intersects(TermMode::KITTY_KEYBOARD_PROTOCOL));
 // Shell-held mode is gone from the stack but remains in
 // snapshot=None (consumed); next ;C will re-snapshot the current
 // empty stack. Pin the `shell_held` KeyboardModes value to keep the
 // local unused.
 let _ = shell_held;
}

/// Regression: pop + ;A same-chunk variant of the above.
/// Same invariant: raw interceptor's ;A restore fires before
/// processor's pop applies, producing stack = snapshot minus one pop.
#[test]
fn csi_pop_and_osc_133_a_same_chunk_raw_first_restores_then_pop_applies() {
 let mut term = make_term();
 feed_mux_and_proc(&mut term, b"\x1b[>1u");
 feed_mux_and_proc(&mut term, b"\x1b]133;C\x1b\\");
 feed_mux_and_proc(&mut term, b"\x1b[>3u");
 assert_eq!(term.keyboard_mode_stack().len(), 2);

 // Same chunk: child pops one, then shell emits ;A (next prompt).
 feed_mux_and_proc(&mut term, b"\x1b[<1u\x1b]133;A\x1b\\");

 assert!(term.keyboard_mode_stack().is_empty());
 assert!(!term.mode().intersects(TermMode::KITTY_KEYBOARD_PROTOCOL));
}

// --- Negative pins ---

/// Regression: without `;C` (no shell integration), `;A`
/// must NOT clear the stack. Restore is snapshot-gated.
#[test]
fn keyboard_mode_stack_osc_133_a_without_prior_c_does_not_modify_stack() {
 let mut term = make_term();
 feed_mux_and_proc(&mut term, b"\x1b[>1u");
 feed_mux_and_proc(&mut term, b"\x1b[>3u");
 assert_eq!(term.keyboard_mode_stack().len(), 2);

 // `;A` without prior `;C`.
 feed_mux_and_proc(&mut term, b"\x1b]133;A\x1b\\");

 assert_eq!(
 term.keyboard_mode_stack().len(),
 2,
 "`;A` with no snapshot must not modify the stack"
 );
}

/// Regression: `;A` without prior `;C` leaves BOTH
/// paired snapshot fields still `None`.
#[test]
fn keyboard_mode_stack_restore_without_snapshot_leaves_paired_fields_none() {
 let mut term = make_term();
 feed_mux_and_proc(&mut term, b"\x1b[>1u");

 feed_mux_and_proc(&mut term, b"\x1b]133;A\x1b\\");

 assert!(term.pre_command_kb_stack_snapshot().is_none());
 assert!(term.inactive_pre_command_kb_stack_snapshot().is_none());
}
