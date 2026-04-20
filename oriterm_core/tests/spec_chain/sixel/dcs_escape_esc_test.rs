use oriterm_test_support::spec_chain::{DispatchArgs, SpecHarness};

#[test]
fn test_esc_esc_backslash() {
    let mut harness = SpecHarness::new();
    harness.feed(b"\x1bPq#0;2;100;0;0~\x1b\x1b\\");

    let aborted = harness
        .outcome()
        .dispatched_calls
        .iter()
        .rev()
        .find_map(|c| match &c.args {
            DispatchArgs::SixelEnd { aborted } => Some(*aborted),
            _ => None,
        })
        .expect("sixel_end must have been dispatched");

    assert!(aborted, "ESC ESC \\ should abort the DCS");
}
