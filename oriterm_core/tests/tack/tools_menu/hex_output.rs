//! Excluded: tack's `h) enable hex output on echo tool` is a modal
//! toggle on the echo tool. Since `e) echo tool` is excluded (see
//! `echo_tool.rs`), toggling its hex mode is moot. If `ori_term` ever
//! needs to validate hex-output rendering of received bytes, add
//! a direct-VTE test in
//! `oriterm_core/src/term/handler/tack_cap_xcheck/` (hex output is
//! a display convention, not a terminfo cap).
