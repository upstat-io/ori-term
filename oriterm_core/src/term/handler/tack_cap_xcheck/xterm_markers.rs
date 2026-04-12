//! Section 06.5 direct-VTE cap xcheck — xterm extension markers.
//!
//! `AX` and `XT` are pure bool advertisements with no escape
//! sequence. They are validated entirely by terminfo declaration
//! presence — there is no VTE round-trip to exercise.

use super::assert_cap_declared;

pub(super) const REGISTERED: &[&str] = &["AX", "XT"];

#[test]
fn tack_cap_xcheck_ax_bool_declared() {
    assert_cap_declared("AX");
}

#[test]
fn tack_cap_xcheck_xt_bool_declared() {
    assert_cap_declared("XT");
}
