//! Tack-driven terminfo conformance tests.
//!
//! Spawns the ncurses `tack` (Terminfo Action Checker) tool against
//! ori_term's pinned terminfo entry (`extra/ori_term.info`, compiled
//! at runtime via `oriterm_test_support::TerminfoEnv`), navigates
//! tack's menus from a PTY, and snapshots the rendered grid against
//! insta golden references.
//!
//! Requires `tack` and `tic` installed (`apt install ncurses-bin` on
//! Debian/Ubuntu, `brew install ncurses` on macOS). Tests gracefully
//! skip on systems where either tool is missing — including native
//! Windows where ncurses is not available without WSL/MSYS2.
//!
//! # Commands
//!
//! - Run: `cargo test -p oriterm_core --test tack`
//! - Update snapshots: `INSTA_UPDATE=1 cargo test -p oriterm_core --test tack`
//!
//! # Layout
//!
//! - `main.rs` — this file (smoke + cross-cutting tests)
//! - Sub-module declarations for Sections 04-06 land as those sections
//!   complete; they consume the framework that lives in
//!   `oriterm_test_support::tack_framework`.

use oriterm_test_support::{PtySession, TerminfoEnv, tack_available, tic_available};

// The framework lives in oriterm_test_support — no `mod framework;`
// here. Test files import via `use oriterm_test_support::tack_framework::*`.
mod test_menu;

/// Smoke test: spawn tack under the pinned terminfo, wait for the
/// main menu, capture as snapshot, quit cleanly.
///
/// This is the canary that proves the tack pipeline (PtySession +
/// TerminfoEnv + tack child) works end-to-end. If it fails, no
/// scenario test in Sections 04-06 of the tack-conformance plan can
/// possibly pass — fix it here.
#[test]
fn tack_smoke_main_menu_at_80x24() {
    if !tack_available() || !tic_available() {
        eprintln!("tack or tic not installed, skipping tack_smoke_main_menu_at_80x24");
        return;
    }

    let env = TerminfoEnv::compile();
    let mut session = PtySession::spawn_tack(&env, 80, 24);

    // Wait for the main menu prompt to appear. The exact prompt string
    // `tack [n] >` is documented in the tack man page and verified
    // live during plan creation.
    //
    // Race-condition note: `wait_for` uses `drain_blocking(100)` plus
    // a content scan; it does not race on DECRQSS/DA handshakes
    // because `PtySession::drain()` writes captured `PtyWrite`
    // responses back to the PTY before returning. If tack sends a DA
    // / DECRQSS query before drawing the main menu, the response is
    // written back inside the same drain call and tack's menu draw
    // follows naturally. No fixed sleeps needed.
    session.wait_for("tack [n] >", 5_000);

    let grid = session.grid_text();

    // Programmatic assertions against the captured grid catch silent
    // regressions where the snapshot updates but a critical menu item
    // disappears. The substrings come from the live tack capture
    // during plan creation.
    //
    // Verified against tack v1.08 (ncurses 6.4) on Linux x86_64 —
    // update this comment (and the snapshot via `INSTA_UPDATE=1`) if
    // a distro upgrade changes the main-menu wording.
    assert!(
        grid.contains("Main Menu"),
        "main menu header missing:\n{grid}"
    );
    assert!(
        grid.contains("begin testing"),
        "'begin testing' missing:\n{grid}"
    );
    assert!(grid.contains("tools"), "'tools' missing:\n{grid}");
    assert!(grid.contains("quit"), "'quit' missing:\n{grid}");
    assert!(grid.contains("tack [n] >"), "prompt missing:\n{grid}");

    // Capture as an insta snapshot. The first run creates the golden;
    // later runs compare against it byte-for-byte.
    insta::assert_snapshot!("tack_smoke_main_menu_80x24", grid);

    // Quit tack via the canonical state-aware helper. `quit_tack(5)`
    // is the strict superset of `wait_for_child_exit(2_000)` from
    // Section 03's handoff contract item 3: Phase 1 sends a bare
    // `q` per iteration via `send_raw` (raw mode, no newline) and
    // observes `try_wait()` between sends; Phase 2 IS exactly
    // `wait_for_child_exit(2_000)`. The previous
    // `send(b"q\n") + wait_for_child_exit(2_000)` antipattern is
    // banned project-wide — see plans/tack-conformance/section-04
    // -scenario-framework.md, runner/mod.rs, and quit_tack's
    // rustdoc for the rationale.
    //
    // Exit-status assertion: verified exit 0 across 10 consecutive
    // runs on ncurses 6.4 / tack v1.08 (Linux x86_64). The
    // supplementary `eprintln!` keeps the exact code in CI logs so
    // a future distro upgrade surfaces a breadcrumb before the
    // `assert!` flips the test red.
    let exit = session.quit_tack(5);
    eprintln!("tack_smoke_main_menu_at_80x24: tack exit status = {exit:?}");
    assert!(
        exit.success(),
        "tack exited non-zero: {exit:?}\nGrid:\n{}",
        session.grid_text()
    );
}
