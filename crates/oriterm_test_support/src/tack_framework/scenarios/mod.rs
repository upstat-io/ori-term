//! Const `ScenarioSpec` catalog consumed by both text tests
//! (`oriterm_core/tests/tack/`) and GPU goldens
//! (`oriterm/src/gpu/visual_regression/tack/`).
//!
//! Section 04 introduces the first submodule (`modes`) in 04.4 which
//! contains `TACK_MODES_AM` and `parse_modes_screen`. Sections 05-08
//! add:
//!   - 05: `acs`, `graphic_rendition`, `color`, `cursor_movement`
//!   - 06: `tools_menu` submodules
//!   - 08: keyboard / function key consts
//!
//! Each submodule defines `pub const SCENARIO_*: ScenarioSpec` values
//! and a `pub fn parse_*_screen(grid: &str) -> ScreenFacts` function
//! pointer. `ScenarioSpec` is `const`-constructible so the catalog
//! forms `pub const ALL_*: &[&ScenarioSpec]` arrays for
//! exhaustiveness tests.

pub mod modes;
