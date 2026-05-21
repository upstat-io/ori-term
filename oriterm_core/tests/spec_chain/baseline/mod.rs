//! Baseline ECMA-48 + DEC private mode spec_chain coverage.
//!
//! Subsections of 
//! land their spec_chain conversions here:
//!
//! - `tack_section_05` — converts the `n) begin testing` menu scenarios
//! into protocol-level verification chain tests. Each tack scenario
//! family has its own submodule documenting which catalog rows it
//! exercises (and which it does not, against tack v1.08).
//! - `tack_section_06` — converts the `t) tools` sub-menu scenarios
//! (status reports, SGR modes, character sets, ENQ/ACK) into
//! protocol-level verification chain tests.
//! - `declrmm` — DECLRMM (mode 69) spec_chain coverage, driving the
//! mode flag + margin set + CUF movement through parser → dispatch
//! → state to verify cursor clamping at `right_margin`.

mod declrmm;
mod tack_section_05;
mod tack_section_06;
