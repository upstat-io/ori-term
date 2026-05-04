use super::walk_vttest_screens;
use crate::session::{PtySession, vttest_available};

/// Regression: — helper walks menu 1 to completion, returning
/// the number of screens captured.
///
/// Pin: closure invoked at least once, terminates on `"Enter choice number"`.
#[test]
fn walk_vttest_screens_walks_menu1_to_completion() {
    if !vttest_available() {
        eprintln!("SKIP: vttest not installed");
        return;
    }

    let mut s = PtySession::spawn_vttest(80, 24);
    s.wait_for("Enter choice number", 5000);
    s.send(b"1\r");

    let mut visited: Vec<usize> = Vec::new();
    let count = walk_vttest_screens(&mut s, 20, &[], |_session, _text, screen| {
        visited.push(screen);
    });

    assert!(
        count > 0,
        "walker should capture at least one menu 1 screen"
    );
    assert_eq!(
        visited.len(),
        count,
        "returned count must match the number of closure invocations",
    );
    assert_eq!(
        visited.first().copied(),
        Some(1),
        "first visited screen must be 1-indexed",
    );
}

/// Regression: — helper makes ZERO closure calls when the
/// sentinel is already present in the initial grid (vttest at the
/// main menu before any selection).
///
/// Pin: walker must not invoke the closure for the sentinel screen
/// itself; returned count is 0.
#[test]
fn walk_vttest_screens_zero_calls_when_sentinel_already_present() {
    if !vttest_available() {
        eprintln!("SKIP: vttest not installed");
        return;
    }

    let mut s = PtySession::spawn_vttest(80, 24);
    s.wait_for("Enter choice number", 5000);

    // Main menu shows "Enter choice number" — walker should bail immediately.
    let mut invoked = 0usize;
    let count = walk_vttest_screens(&mut s, 20, &[], |_session, _text, _screen| {
        invoked += 1;
    });

    assert_eq!(
        count, 0,
        "walker must return 0 when sentinel present at start"
    );
    assert_eq!(
        invoked, 0,
        "closure must not be invoked when sentinel present"
    );
}

/// Regression: — helper terminates after `max_screens`
/// closure invocations.
///
/// Pin: with `max_screens = 2`, closure invoked exactly 2 times even if
/// vttest has more screens before returning to the menu.
#[test]
fn walk_vttest_screens_max_screens_cap_terminates_loop() {
    if !vttest_available() {
        eprintln!("SKIP: vttest not installed");
        return;
    }

    let mut s = PtySession::spawn_vttest(80, 24);
    s.wait_for("Enter choice number", 5000);
    s.send(b"2\r"); // menu 2 has 16+ screens

    let mut invoked = 0usize;
    let count = walk_vttest_screens(&mut s, 2, &[], |_session, _text, _screen| {
        invoked += 1;
    });

    assert_eq!(count, 2, "walker must respect max_screens cap");
    assert_eq!(
        invoked, 2,
        "closure must be invoked exactly max_screens times"
    );
}

/// Regression: — helper passes the SAME `text` value to the
/// closure that it used for the sentinel check.
///
/// Pin: closure receives non-empty text on every invocation; the text
/// does NOT contain the sentinel (proves the helper checked first).
#[test]
fn walk_vttest_screens_passes_captured_text_to_closure() {
    if !vttest_available() {
        eprintln!("SKIP: vttest not installed");
        return;
    }

    let mut s = PtySession::spawn_vttest(80, 24);
    s.wait_for("Enter choice number", 5000);
    s.send(b"1\r");

    let mut texts: Vec<String> = Vec::new();
    walk_vttest_screens(&mut s, 5, &[], |_session, text, _screen| {
        texts.push(text.to_string());
    });

    assert!(
        !texts.is_empty(),
        "should have captured at least one screen"
    );
    for (idx, text) in texts.iter().enumerate() {
        assert!(
            !text.is_empty(),
            "captured text at idx {idx} must not be empty",
        );
        assert!(
            !text.contains("Enter choice number"),
            "captured text at idx {idx} must NOT contain the sentinel \
             (helper must check sentinel BEFORE invoking closure); got: {text:?}",
        );
    }
}
