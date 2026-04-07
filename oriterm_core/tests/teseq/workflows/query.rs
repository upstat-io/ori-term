//! Query-response workflow tests (DA handshake, cursor tracking).

use super::{assert_pty_writes, compute_da2_version, run_scenario};

#[test]
fn query_da_handshake() {
    let Some(outcome) = run_scenario("query_da_handshake") else {
        return;
    };
    let da2_version = compute_da2_version();
    assert_pty_writes(
        &outcome,
        &[
            "\x1b[?64;6;4c",
            &format!("\x1b[>0;{da2_version};1c"),
            "\x1bP!|00000000\x1b\\",
        ],
    );
}

#[test]
fn query_cursor_tracking() {
    let Some(outcome) = run_scenario("query_cursor_tracking") else {
        return;
    };
    // CUP 5;10 -> DSR -> CUU 3 -> DSR -> CUF 20 -> DSR
    // DSR reports 1-based coordinates.
    assert_pty_writes(&outcome, &["\x1b[5;10R", "\x1b[2;10R", "\x1b[2;30R"]);
}
