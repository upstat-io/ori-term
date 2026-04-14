//! Baseline ECMA-48 + DEC private mode spec_chain coverage.
//!
//! Subsections of `plans/spec-conformance/section-08-ecma-48-baseline.md`
//! land their spec_chain conversions here:
//!
//! - `tack_section_05` — converts the `n) begin testing` menu scenarios
//!   from the legacy `plans/tack-conformance/section-05-test-menu-scenarios.md`
//!   into protocol-level verification chain tests. Each tack scenario
//!   family has its own submodule documenting which catalog rows it
//!   exercises (and which it does not, against tack v1.08).
//! - `tack_section_06` — converts the `t) tools` sub-menu scenarios
//!   (status reports, SGR modes, character sets, ENQ/ACK) from the
//!   legacy `plans/tack-conformance/section-06-tools-menu-scenarios.md`
//!   into protocol-level verification chain tests.

mod tack_section_05;
mod tack_section_06;
