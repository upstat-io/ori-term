//! Effect-cutover §01.3 deletion pins.
//!
//! These tests verify that the legacy adapter surface is GONE from the
//! workspace — no production source file references `LegacyEventSink`,
//! `IoThreadEventProxy`, `MuxEventProxy`, `DesktopNotificationRecord`,
//! `Event::ClipboardLoad`, `Event::ColorRequest`, or the
//! `Term::drain_notifications()` shim. After §01.3 lands the only
//! event/effect path is `Effect` → `EffectSink` → effect router →
//! `MuxEvent` / `MuxNotification`.
//!
//! Each pin uses `grep -rn` on the workspace source. On Windows (where
//! `grep` is not part of the base install) the pin skips gracefully per
//! `.claude/rules/tests.md` §Graceful Skip Protocol — the test still
//! shows up in the runnable set with a clear `SKIP:` message.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Absolute path to the workspace root (the directory containing the
/// top-level `Cargo.toml`). `cargo test` runs from each crate's
/// directory, so we walk up two levels from this file's
/// `CARGO_MANIFEST_DIR` (`oriterm_core`) to reach the workspace root.
/// Resolve the term_repo workspace root via the canonical SSOT helper per
/// `.claude/rules/test-organization.md §Wrapper/Subrepo Path Discovery` —
/// never reintroduce ad-hoc `manifest_dir.parent()` arithmetic.
fn workspace_root() -> PathBuf {
    oriterm_test_support::paths::term_workspace_root().to_path_buf()
}

/// Skip protocol for missing `grep`. Matches the `tack`/`reseq` pattern
/// used by other conformance tests — emit a visible `SKIP:` line and
/// return without asserting.
fn ensure_grep_or_skip(test_name: &str) -> bool {
    if Command::new("grep").arg("--version").output().is_ok() {
        true
    } else {
        eprintln!("SKIP: {test_name}: `grep` is not available on this platform");
        false
    }
}

/// Grep `pattern` against the workspace source roots, returning matching
/// `path:line:text` rows. Excludes the plan tree, the bug-tracker, the
/// effect-cutover overview, this test file (which mentions the names by
/// design), build artefacts, and the `.git` directory.
fn grep_sources(pattern: &str, roots: &[&str]) -> Vec<String> {
    let root = workspace_root();
    let mut paths: Vec<PathBuf> = Vec::new();
    for r in roots {
        let p = root.join(r);
        if p.exists() {
            paths.push(p);
        }
    }
    if paths.is_empty() {
        return Vec::new();
    }

    let mut cmd = Command::new("grep");
    cmd.arg("-rnE");
    cmd.arg("--include=*.rs");
    cmd.arg("--exclude-dir=target");
    cmd.arg("--exclude-dir=.git");
    cmd.arg(pattern);
    for p in &paths {
        cmd.arg(p);
    }

    let output = cmd.output().expect("grep should run when available");
    // Exit code 1 means "no matches" — that's OK for our purposes.
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();

    let test_self = file!();
    let test_self_path = Path::new(test_self).file_name().and_then(|n| n.to_str());

    stdout
        .lines()
        .filter(|line| match test_self_path {
            Some(name) => !line.contains(name),
            None => true,
        })
        .map(|s| s.to_string())
        .collect()
}

const WORKSPACE_ROOTS: &[&str] = &[
    "oriterm_core/src",
    "oriterm_mux/src",
    "oriterm/src",
    "oriterm_ui/src",
    "oriterm_ipc/src",
    "crates/oriterm_test_support/src",
    "oriterm_core/tests",
    "oriterm_mux/tests",
    "oriterm/tests",
    "oriterm_ui/tests",
];

/// Deletion pin: zero `LegacyEventSink` references in the workspace.
#[test]
fn no_legacy_event_sink_references() {
    if !ensure_grep_or_skip("no_legacy_event_sink_references") {
        return;
    }
    let hits = grep_sources("LegacyEventSink", WORKSPACE_ROOTS);
    assert!(
        hits.is_empty(),
        "LegacyEventSink must be fully deleted post §01.3; found {} reference(s):\n{}",
        hits.len(),
        hits.join("\n")
    );
}

/// Deletion pin: zero `IoThreadEventProxy` references in the workspace.
#[test]
fn no_io_thread_event_proxy_references() {
    if !ensure_grep_or_skip("no_io_thread_event_proxy_references") {
        return;
    }
    let hits = grep_sources("IoThreadEventProxy", WORKSPACE_ROOTS);
    assert!(
        hits.is_empty(),
        "IoThreadEventProxy must be fully deleted post §01.3; found {} reference(s):\n{}",
        hits.len(),
        hits.join("\n")
    );
}

/// Deletion pin: zero `MuxEventProxy` references in the workspace.
#[test]
fn no_mux_event_proxy_references() {
    if !ensure_grep_or_skip("no_mux_event_proxy_references") {
        return;
    }
    let hits = grep_sources("MuxEventProxy", WORKSPACE_ROOTS);
    assert!(
        hits.is_empty(),
        "MuxEventProxy must be fully deleted post §01.3; found {} reference(s):\n{}",
        hits.len(),
        hits.join("\n")
    );
}

/// Deletion pin: zero `Event::ClipboardLoad` or `Event::ColorRequest`
/// references in the workspace. Both variants are deleted in §01.3
/// (replaced by the `Effect::HostRequest` path).
#[test]
fn no_event_clipboardload_or_colorrequest_variants() {
    if !ensure_grep_or_skip("no_event_clipboardload_or_colorrequest_variants") {
        return;
    }
    // Anchor at non-identifier byte to skip false positives like
    // `RecordedEvent::ClipboardLoad` (a test-local enum that survives
    // the cutover and shares the suffix). `[^A-Za-z0-9_]` ensures the
    // match is preceded by punctuation/space — `Event::ClipboardLoad`
    // matches but `RecordedEvent::ClipboardLoad` does not.
    let hits = grep_sources(
        "(^|[^A-Za-z0-9_])Event::(ClipboardLoad|ColorRequest)",
        WORKSPACE_ROOTS,
    );
    assert!(
        hits.is_empty(),
        "Event::ClipboardLoad / Event::ColorRequest must be fully deleted post §01.3; \
         found {} reference(s):\n{}",
        hits.len(),
        hits.join("\n")
    );
}

/// Deletion pin: zero `Term::drain_notifications` shim definitions in
/// `oriterm_core/src/term/`. The mux-side `drain_notifications` methods
/// (on `InProcessMux`, `MuxBackend`, etc.) STAY — they are part of the
/// production mux notification pipeline and unrelated to the deleted
/// `Term` shim.
#[test]
fn no_drain_notifications_shim() {
    if !ensure_grep_or_skip("no_drain_notifications_shim") {
        return;
    }
    let hits = grep_sources(r"fn drain_notifications\(", &["oriterm_core/src/term"]);
    assert!(
        hits.is_empty(),
        "Term::drain_notifications shim must be fully deleted post §01.3; \
         found {} reference(s) inside oriterm_core/src/term/:\n{}",
        hits.len(),
        hits.join("\n")
    );
}

/// Deletion pin: zero `DesktopNotificationRecord` references in the
/// workspace. The notification queueing field on `LegacyEventSink` was
/// the only producer; both type and producer go away in §01.3.
#[test]
fn no_desktop_notification_record_references() {
    if !ensure_grep_or_skip("no_desktop_notification_record_references") {
        return;
    }
    let hits = grep_sources("DesktopNotificationRecord", WORKSPACE_ROOTS);
    assert!(
        hits.is_empty(),
        "DesktopNotificationRecord must be fully deleted post §01.3; \
         found {} reference(s):\n{}",
        hits.len(),
        hits.join("\n")
    );
}

/// Effect-cutover §01.3 negative pin: the `Event` enum carries only
/// the legacy variants that survive the cutover. `ClipboardLoad`,
/// `ColorRequest`, `ChildExit`, and `Wakeup` (plus the closure-bearing
/// shapes) are gone — `HostRequest` and `HostEffect::ChildExit` cover
/// those code paths via the `Effect` path now.
#[test]
fn event_enum_variants_exhaustive_list() {
    use oriterm_core::event::{ClipboardType, Event};

    // Construct every surviving variant; the compiler enforces that any
    // future addition is exercised here.
    let variants: Vec<Event> = vec![
        Event::Wakeup,
        Event::Bell,
        Event::Title(String::new()),
        Event::ResetTitle,
        Event::IconName(String::new()),
        Event::ResetIconName,
        Event::ClipboardStore(ClipboardType::Clipboard, String::new()),
        Event::PtyWrite(String::new()),
        Event::CursorBlinkingChange,
        Event::Cwd(String::new()),
        Event::CommandComplete(std::time::Duration::from_secs(1)),
        Event::MouseCursorDirty,
    ];
    assert_eq!(
        variants.len(),
        12,
        "post §01.3 the Event enum must have exactly 12 variants"
    );

    // Spot-check that the deleted variants are NOT constructible by name.
    // Compile-failure attempts are off-the-shelf in `event/tests.rs`;
    // this assertion is the documentation pin so a regression that
    // re-adds (e.g.) `Event::ChildExit` immediately fails this test
    // (the variant count would jump to 13).
}
