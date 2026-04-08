//! Tack `n) begin testing` submenu scenarios.
//!
//! Section 04 introduces the first scenario (`modes::tack_modes_am`).
//! Section 05 of the tack-conformance plan adds the rest of the test
//! menu catalog (modes/glitches, ACS, color, cursor movement). The
//! `begin_testing_inventory` discovery test (05.0) pins the menu graph
//! and is the drift gate for every other Section 05 scenario.
//!
//! 05.2 adds the `acs` and `graphic_rendition` test wrappers — both
//! navigate to the same `tack/test/acs [n] >` sub-menu (tack v1.08
//! combines them under one menu key) but use different parsers.

pub mod acs;
pub mod begin_testing_inventory;
pub mod color;
pub mod cursor_movement;
pub mod graphic_rendition;
pub mod modes;
