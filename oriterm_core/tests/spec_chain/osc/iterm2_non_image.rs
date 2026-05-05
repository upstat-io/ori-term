//! OSC 1337 non-image sub-ops spec_chain coverage (Section 10.7).
//!
//! Drives the iTerm2 OSC 1337 non-image variants — SetMark, RemoteHost=,
//! CurrentDir=, Copy=, ReportCellSize, SetUserVar=, ShellIntegrationVersion=
//! — through the high-level VTE `Processor` path and asserts the matching
//! `Term` state mutation or `Effect::*` emission. The `File=` image path
//! is out of scope; Section 14 owns its verification. A single regression
//! test feeds a minimal `File=name=test.png;:<png>` payload to pin that
//! the sub-dispatcher still routes it to `Handler::iterm2_file`.
//!
//! Dispatch: `crates/vte/src/ansi/dispatch/osc.rs:249-324` `b"1337"` arm
//! → `dispatch_iterm2_osc1337` → `Handler::iterm2_*` (per sub-op).
//! Handler: `oriterm_core/src/term/handler/osc.rs::osc_iterm2_*`.
//!
//! Catalog rows: ITERM2-1337-SETMARK, ITERM2-1337-REMOTEHOST,
//! ITERM2-1337-CURRENTDIR, ITERM2-1337-COPY, ITERM2-1337-REPORTCELLSIZE,
//! ITERM2-1337-SETUSERVAR, ITERM2-1337-SHELLINTVERSION,
//! SHINT-OSC-1337-REMOTEHOST, SHINT-OSC-1337-CURRENTDIR,
//! SHINT-OSC-1337-SETMARK, SHINT-OSC-1337-SETUSERVAR,
//! SHINT-OSC-1337-REPORTCELLSIZE.
//! Cross-refs: SHINT-OSC-1337-* rows in catalog/shell-integration.md.

use oriterm_core::effect::{ClipboardSelection, Effect, HostEffect};
use oriterm_test_support::spec_chain::{DispatchArgs, SpecHarness, expect_single_pty_write};

/// Assert that `method` appears at least once in the recorded dispatch
/// calls. Used to pin Handler-trait coverage when the state side-effect
/// is not uniquely identifying.
fn assert_dispatched(harness: &SpecHarness, method: &str) {
    let present = harness
        .outcome()
        .dispatched_calls
        .iter()
        .any(|call| match &call.args {
            DispatchArgs::Other { method: m } => *m == method,
            _ => call.method == method,
        });
    assert!(
        present,
        "expected a dispatch to `{method}`; got {:?}",
        harness.outcome().dispatched_calls
    );
}

/// Locate the single `ClipboardStore` effect and return `(selection, data)`.
fn expect_clipboard_store(harness: &SpecHarness) -> (ClipboardSelection, String) {
    let effects = &harness.outcome().effects_emitted;
    let mut found = None;
    for eff in effects {
        if let Effect::Host(HostEffect::ClipboardStore { selection, data }) = eff {
            assert!(found.is_none(), "more than one ClipboardStore emitted");
            found = Some((*selection, data.clone()));
        }
    }
    found.unwrap_or_else(|| panic!("expected one ClipboardStore; got {effects:?}"))
}

/// Assert no `ClipboardStore` effect is present on the transcript.
fn assert_no_clipboard_store(harness: &SpecHarness) {
    for eff in &harness.outcome().effects_emitted {
        if let Effect::Host(HostEffect::ClipboardStore { .. }) = eff {
            panic!("unexpected ClipboardStore on transcript: {eff:?}");
        }
    }
}

// ── SetMark ─────────────────────────────────────────────────────────

/// `OSC 1337 ; SetMark ST` records a navigation mark at the current
/// cursor row, sharing the `prompt_markers` SSOT with OSC 133;A.
/// Anchor: catalog row `ITERM2-1337-SETMARK`.
#[test]
fn osc1337_set_mark() {
    let mut harness = SpecHarness::new();
    // Move cursor so the recorded row is deterministic and non-zero.
    harness.feed(b"\x1b[5;1H");
    assert!(harness.term().prompt_markers().is_empty());

    harness.feed(b"\x1b]1337;SetMark\x1b\\");

    let markers = harness.term().prompt_markers();
    assert_eq!(markers.len(), 1, "SetMark must push exactly one marker");
    let m = &markers[0];
    assert_eq!(
        m.prompt, 4,
        "marker row must be cursor line (row 5 → abs 4)"
    );
    assert!(m.command.is_none());
    assert!(m.output.is_none());
}

// ── RemoteHost ──────────────────────────────────────────────────────

/// Pins: `OSC 1337 ; RemoteHost=<host> ST` stores `<host>` as observable
/// `Term::remote_host` state — the iTerm2 shell-integration hook that
/// lets hosts track the current SSH destination.
/// Anchor: catalog row `ITERM2-1337-REMOTEHOST`.
#[test]
fn osc1337_remote_host() {
    let mut harness = SpecHarness::new();
    harness.feed(b"\x1b]1337;RemoteHost=user@host.example.com\x1b\\");
    assert_eq!(harness.term().remote_host(), Some("user@host.example.com"));
}

// ── CurrentDir ──────────────────────────────────────────────────────

/// Pins: `OSC 1337 ; CurrentDir=<path> ST` stores `<path>` as observable
/// `Term::cwd` state — the iTerm2-flavoured CWD hook that shares its
/// backing field with OSC 7 (see the SSOT direction-A / direction-B
/// tests below). Anchor: catalog row `ITERM2-1337-CURRENTDIR`.
#[test]
fn osc1337_current_dir() {
    let mut harness = SpecHarness::new();
    harness.feed(b"\x1b]1337;CurrentDir=/path/to/dir\x1b\\");
    assert_eq!(harness.term().cwd(), Some("/path/to/dir"));
}

// ── Copy ────────────────────────────────────────────────────────────

/// Pins: `OSC 1337 ; Copy=:<b64> ST` with empty selection stores the
/// decoded UTF-8 in the default clipboard. Base64("Hello") = `SGVsbG8=`.
/// Anchor: catalog row `ITERM2-1337-COPY` (empty-selection default path).
#[test]
fn osc1337_copy() {
    let mut harness = SpecHarness::new();
    harness.feed(b"\x1b]1337;Copy=:SGVsbG8=\x1b\\");
    let (selection, data) = expect_clipboard_store(&harness);
    assert_eq!(selection, ClipboardSelection::Clipboard);
    assert_eq!(data, "Hello");
}

// ── ReportCellSize ──────────────────────────────────────────────────

/// `OSC 1337 ; ReportCellSize ST` emits a PTY reply of the form
/// `OSC 1337 ; ReportCellSize=<H>;<W> ST` with H and W as floats
/// (one decimal place) per the iTerm2 proprietary-escape-codes
/// specification. Cross-reference WezTerm's emit shape at
/// `wezterm-escape-parser/src/osc.rs:1351`:
/// `ReportCellSize={height_pixels:.1};{width_pixels:.1}`.
/// Harness defaults are 16×8 → "16.0;8.0".
/// Anchor: catalog row `ITERM2-1337-REPORTCELLSIZE` (cross-reference WezTerm shape at wezterm-escape-parser/src/osc.rs:1351).
#[test]
fn osc1337_report_cell_size() {
    let mut harness = SpecHarness::new();
    let h = f32::from(harness.term().cell_pixel_height());
    let w = f32::from(harness.term().cell_pixel_width());

    harness.feed(b"\x1b]1337;ReportCellSize\x1b\\");

    let bytes = expect_single_pty_write(&harness);
    let expected = format!("\x1b]1337;ReportCellSize={h:.1};{w:.1}\x1b\\");
    assert_eq!(String::from_utf8_lossy(&bytes), expected);
}

// ── SetUserVar ──────────────────────────────────────────────────────

/// Pins: `OSC 1337 ; SetUserVar=<key>=<b64> ST` decodes the value and
/// records it on `Term::user_vars`. Base64("Hello") = `SGVsbG8=`.
/// Anchor: catalog row `ITERM2-1337-SETUSERVAR`.
#[test]
fn osc1337_set_user_var() {
    let mut harness = SpecHarness::new();
    harness.feed(b"\x1b]1337;SetUserVar=MY_VAR=SGVsbG8=\x1b\\");
    assert_eq!(harness.term().user_var("MY_VAR"), Some("Hello"));
}

// ── ShellIntegrationVersion ─────────────────────────────────────────

/// Pins: `OSC 1337 ; ShellIntegrationVersion=<n> ST` stores `<n>` as
/// observable `Term::shell_integration_version` state — the iTerm2
/// version handshake the shell sends on startup.
/// Anchor: catalog row `ITERM2-1337-SHELLINTVERSION`.
#[test]
fn osc1337_shell_integration_version() {
    let mut harness = SpecHarness::new();
    harness.feed(b"\x1b]1337;ShellIntegrationVersion=5\x1b\\");
    assert_eq!(harness.term().shell_integration_version(), Some("5"));
}

// ── File= regression (Section 14 boundary pin) ──────────────────────

/// Minimal 1×1 PNG as base64 ("SGVsbG8=" is too short — use a real PNG
/// header so the image parser does not reject the payload shape before
/// the dispatch assertion runs). We only assert that `iterm2_file` was
/// dispatched, NOT that the image ends up in the cache (Section 14
/// covers that).
const MINIMAL_PNG_B64: &str =
    "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNgAAIAAAUAAeImBZsAAAAASUVORK5CYII=";

/// Pins: the `File=` sub-op still routes to the `iterm2_file` handler —
/// this is a Section 14 boundary pin. We only assert the dispatch reaches
/// the handler, not that the image lands in the cache (Section 14 owns
/// image-pipeline verification). Anchor: catalog row
/// `ITERM2-1337` (File= dispatch boundary).
#[test]
fn osc1337_file_still_routes_to_iterm2_file() {
    let mut harness = SpecHarness::new();
    let payload = format!("\x1b]1337;File=name=dGVzdC5wbmc=;size=68:{MINIMAL_PNG_B64}\x1b\\");
    harness.feed(payload.as_bytes());
    assert_dispatched(&harness, "iterm2_file");
}

// ── Unknown keys drop silently ──────────────────────────────────────

/// Regression guard: an unrecognized `OSC 1337 ; NotARealKey=value ST`
/// sub-op is silently dropped — baseline CWD / remote-host / prompt
/// markers / user_vars state is untouched. Guards against the
/// dispatcher accidentally mutating state for keys it does not
/// recognize. Anchor: catalog row `ITERM2-1337` (unknown-key drop).
#[test]
fn osc1337_unknown_key_dropped() {
    let mut harness = SpecHarness::new();
    let before_cwd = harness.term().cwd().map(str::to_owned);
    let before_remote = harness.term().remote_host().map(str::to_owned);
    let before_markers = harness.term().prompt_markers().len();
    harness.feed(b"\x1b]1337;NotARealKey=value\x1b\\");
    assert_eq!(harness.term().cwd().map(str::to_owned), before_cwd);
    assert_eq!(
        harness.term().remote_host().map(str::to_owned),
        before_remote
    );
    assert_eq!(harness.term().prompt_markers().len(), before_markers);
    assert_eq!(harness.term().user_vars_len(), 0);
}

/// REGRESSION GUARD (blind-spot #9 Section 14 de-risk) — a `File=` payload
/// that includes an unknown sub-key still routes to `iterm2_file`; the
/// sub-dispatcher must not drop the entire payload because a newer
/// iTerm2 attribute is present alongside the recognized ones.
/// Anchor: catalog row `ITERM2-1337-FILE` (unknown-sub-key tolerance).
#[test]
fn osc1337_unknown_file_subop_safely_ignored() {
    let mut harness = SpecHarness::new();
    let payload =
        format!("\x1b]1337;File=name=dGVzdC5wbmc=;UnknownFileAttr=bar:{MINIMAL_PNG_B64}\x1b\\");
    harness.feed(payload.as_bytes());
    assert_dispatched(&harness, "iterm2_file");
}

// ── user_vars cap / RSS regression ──────────────────────────────────

/// Pins: 257 distinct `SetUserVar` keys must not blow through the default
/// 256-entry cap; the oldest (KEY_0) is evicted. RSS regression pin for
/// the unbounded-growth vector. Anchor: catalog row `ITERM2-1337-SETUSERVAR`
/// (cap-enforcement arm) / `oriterm_core/tests/rss_regression.rs` invariant.
#[test]
fn osc1337_user_vars_cap_evicts_oldest() {
    let mut harness = SpecHarness::new();
    // Base64 of the single byte 'x' is "eA==".
    for i in 0..=256_u32 {
        let osc = format!("\x1b]1337;SetUserVar=KEY_{i}=eA==\x1b\\");
        harness.feed(osc.as_bytes());
    }
    assert_eq!(
        harness.term().user_vars_len(),
        256,
        "cap must hold user_vars at 256 entries"
    );
    assert_eq!(
        harness.term().user_var("KEY_0"),
        None,
        "oldest (KEY_0) must be evicted"
    );
    assert_eq!(
        harness.term().user_var("KEY_256"),
        Some("x"),
        "newest (KEY_256) must be retained"
    );
}

/// Pins: re-inserting an existing key does NOT refresh its eviction
/// position (non-LRU; matches `IndexMap::insert` semantics). After A/B/C
/// fill the cap, re-inserting A keeps A at index 0 — so when D is inserted
/// the FIFO evicts A (the oldest insertion), NOT B. A future implementer
/// who changes `insert` to `shift_remove + insert` (refreshing position)
/// would break this pin. Anchor: catalog row `ITERM2-1337-SETUSERVAR`
/// (FIFO eviction semantics arm).
#[test]
fn osc1337_user_vars_reinsert_does_not_refresh_position() {
    let mut harness = SpecHarness::new();
    harness.term_mut().set_user_vars_max(3);

    // Base64("a") = "YQ==", "b" = "Yg==", "c" = "Yw==", "d" = "ZA==",
    // "updated" = "dXBkYXRlZA==".
    harness.feed(b"\x1b]1337;SetUserVar=A=YQ==\x1b\\");
    harness.feed(b"\x1b]1337;SetUserVar=B=Yg==\x1b\\");
    harness.feed(b"\x1b]1337;SetUserVar=C=Yw==\x1b\\");
    // Re-insert A with a new value. Position must NOT refresh — A stays
    // at index 0 (front of the FIFO).
    harness.feed(b"\x1b]1337;SetUserVar=A=dXBkYXRlZA==\x1b\\");
    // Push D — this evicts index 0 = A, NOT B.
    harness.feed(b"\x1b]1337;SetUserVar=D=ZA==\x1b\\");

    assert_eq!(harness.term().user_vars_len(), 3);
    assert_eq!(harness.term().user_var("A"), None, "A must be evicted");
    assert_eq!(harness.term().user_var("B"), Some("b"));
    assert_eq!(harness.term().user_var("C"), Some("c"));
    assert_eq!(harness.term().user_var("D"), Some("d"));
}

// ── Regression guards (invalid base64) ──────────────────────────────────

/// Regression guard: an `OSC 1337 ; Copy=:<garbage> ST` with invalid base64
/// must NOT emit a `ClipboardStore` effect — the decoder rejects before
/// reaching the clipboard handler. Anchor: catalog row
/// `ITERM2-1337-COPY` (invalid-base64 drop).
#[test]
fn osc1337_copy_invalid_base64_dropped() {
    let mut harness = SpecHarness::new();
    harness.feed(b"\x1b]1337;Copy=:!!!not-valid-base64!!!\x1b\\");
    assert_no_clipboard_store(&harness);
}

/// Regression guard: an `OSC 1337 ; SetUserVar=KEY=<garbage> ST` with
/// invalid base64 must NOT record the key in `user_vars` — the decoder
/// rejects before reaching the set-user-var handler, keeping the map
/// empty. Anchor: catalog row `ITERM2-1337-SETUSERVAR`
/// (invalid-base64 drop).
#[test]
fn osc1337_set_user_var_invalid_base64_dropped() {
    let mut harness = SpecHarness::new();
    harness.feed(b"\x1b]1337;SetUserVar=MY_KEY=!!!invalid!!!\x1b\\");
    assert_eq!(harness.term().user_var("MY_KEY"), None);
    assert_eq!(harness.term().user_vars_len(), 0);
}

// ── Matrix pins (Copy selection variants, missing separator) ────────
//
// Each regression guard asserts BOTH that the dispatch reached the handler
// AND that the handler dropped without side effects. Without the
// dispatch assertion a "no effect" check could also pass if the OSC
// was silently rejected at the parser — which would hide a real
// dispatch regression (the regression guard passes for the wrong reason).

/// Pins: `OSC 1337 ; Copy=<payload-without-colon> ST` drops — the handler
/// dispatches but short-circuits before base64 decode because the
/// selection-char separator is missing. Regression guard paired with
/// `osc1337_copy`. Anchor: catalog row `ITERM2-1337-COPY` (malformed-payload
/// drop path).
#[test]
fn osc1337_copy_missing_colon_dropped() {
    let mut harness = SpecHarness::new();
    harness.feed(b"\x1b]1337;Copy=noconvsep\x1b\\");
    assert_dispatched(&harness, "iterm2_copy");
    assert_no_clipboard_store(&harness);
}

/// Each recognized selection prefix (c/p/s) routes to the matching
/// `ClipboardSelection` variant. Mirrors the OSC 52 store matrix.
/// Anchor: catalog row `ITERM2-1337-COPY` (selection-prefix matrix).
#[test]
fn osc1337_copy_explicit_selection_chars() {
    for (ch, expected) in [
        (b'c', ClipboardSelection::Clipboard),
        (b'p', ClipboardSelection::Primary),
        (b's', ClipboardSelection::Select),
    ] {
        let mut harness = SpecHarness::new();
        let payload = format!("\x1b]1337;Copy={}:SGVsbG8=\x1b\\", ch as char);
        harness.feed(payload.as_bytes());
        let (selection, data) = expect_clipboard_store(&harness);
        assert_eq!(
            selection, expected,
            "selection char `{}` must route to {expected:?}",
            ch as char,
        );
        assert_eq!(data, "Hello");
    }
}

/// Pins: unknown selection characters dispatch but drop — the handler
/// reaches the selection match but falls through to `_ => return` rather
/// than routing to a default variant. Regression guard paired with
/// `osc1337_copy_explicit_selection_chars`. Anchor: catalog row
/// `ITERM2-1337-COPY` (unknown-selection-char drop path).
#[test]
fn osc1337_copy_unknown_selection_char_dropped() {
    let mut harness = SpecHarness::new();
    harness.feed(b"\x1b]1337;Copy=x:SGVsbG8=\x1b\\");
    assert_dispatched(&harness, "iterm2_copy");
    assert_no_clipboard_store(&harness);
}

/// Invalid UTF-8 payloads drop silently inside each handler (per
/// `std::str::from_utf8` check). Uses a lone 0xFF byte — valid OSC
/// parameter, invalid UTF-8. Pre-populates state so an accidental
/// mutation via the invalid path would be observable as a change
/// from the baseline, not as first-set-to-bad-value.
/// Anchor: catalog row `ITERM2-1337-COPY` + `ITERM2-1337-SETUSERVAR` (invalid-UTF-8 drop path).
#[test]
fn osc1337_invalid_utf8_dropped() {
    let mut harness = SpecHarness::new();

    // Pre-populate CWD and remote_host with known-good values so any
    // accidental overwrite on the invalid path is detectable as a drift
    // from this baseline — stronger than asserting absence on a virgin
    // harness (which would also pass if dispatch never reached the
    // handler).
    harness.term_mut().set_cwd(Some("/good-cwd".to_owned()));
    harness.feed(b"\x1b]1337;RemoteHost=user@ok\x1b\\");
    harness.feed(b"\x1b]1337;SetUserVar=GOOD=eA==\x1b\\");
    let cwd0 = harness.term().cwd().map(str::to_owned);
    let host0 = harness.term().remote_host().map(str::to_owned);
    let vars0 = harness.term().user_vars_len();

    harness.feed(b"\x1b]1337;CurrentDir=/\xff\x1b\\");
    assert_dispatched(&harness, "iterm2_current_dir");

    harness.feed(b"\x1b]1337;RemoteHost=\xff\x1b\\");
    assert_dispatched(&harness, "iterm2_remote_host");

    harness.feed(b"\x1b]1337;SetUserVar=\xff=eA==\x1b\\");
    assert_dispatched(&harness, "iterm2_set_user_var");

    assert_eq!(
        harness.term().cwd().map(str::to_owned),
        cwd0,
        "invalid-UTF-8 CurrentDir must NOT mutate cwd"
    );
    assert_eq!(
        harness.term().remote_host().map(str::to_owned),
        host0,
        "invalid-UTF-8 RemoteHost must NOT mutate remote_host"
    );
    assert_eq!(
        harness.term().user_vars_len(),
        vars0,
        "invalid-UTF-8 SetUserVar name must NOT add an entry"
    );
}

// ── CWD SSOT direction-A / direction-B ──────────────────────────────
//
// OSC 7 is interceptor-only (scope clarification §B) — the URI parse
// (`file://host/path` → `/path`) lives in `oriterm_mux::shell_integration`
// and cannot be imported into this crate without a boundary violation.
// The SSOT invariant under test is: both OSC 7 (via interceptor calling
// `Term::set_cwd`) and OSC 1337 CurrentDir (via `Handler::iterm2_current_dir`
// → `Term::set_cwd`) write the SAME `Term::cwd` field — any second CWD
// field would break either test. We simulate the OSC 7 side by calling
// `set_cwd` directly (exactly what the interceptor does post-URI-parse).

/// DIRECTION A: OSC 7 sets cwd first (via `set_cwd`, as the interceptor
/// would); OSC 1337 CurrentDir overwrites it via the shared field.
/// Anchor: catalog rows `ITERM2-1337-CURRENTDIR` + `OSC-7` (SSOT shared-field contract).
#[test]
fn osc1337_ssot_cwd_direction_a() {
    let mut harness = SpecHarness::new();
    harness.term_mut().set_cwd(Some("/start".to_owned()));
    assert_eq!(harness.term().cwd(), Some("/start"));

    harness.feed(b"\x1b]1337;CurrentDir=/other-path\x1b\\");
    assert_eq!(
        harness.term().cwd(),
        Some("/other-path"),
        "OSC 1337 CurrentDir must overwrite the OSC 7 cwd via the shared field"
    );
}

/// DIRECTION B: OSC 1337 CurrentDir sets cwd first (via the Handler
/// path); OSC 7's `set_cwd` overwrites it. Pins that BOTH directions
/// flow through the SAME field — a future regression where either OSC
/// writes a second field is caught here.
/// Anchor: catalog rows `ITERM2-1337-CURRENTDIR` + `OSC-7` (SSOT shared-field contract).
#[test]
fn osc1337_ssot_cwd_direction_b() {
    let mut harness = SpecHarness::new();
    harness.feed(b"\x1b]1337;CurrentDir=/from-iterm2\x1b\\");
    assert_eq!(harness.term().cwd(), Some("/from-iterm2"));

    harness.term_mut().set_cwd(Some("/from-osc7".to_owned()));
    assert_eq!(
        harness.term().cwd(),
        Some("/from-osc7"),
        "OSC 7's set_cwd must overwrite OSC 1337's cwd via the shared field"
    );
}
