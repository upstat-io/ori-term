//! Const `ScenarioSpec` catalog consumed by both text tests
//! (`oriterm_core/tests/tack/`) and GPU goldens
//! (`oriterm/src/gpu/visual_regression/tack/`).
//!
//! Section 04 introduces the first submodule (`modes`) in 04.4 which
//! contains `TACK_MODES_AM` and `parse_modes_screen`. Sections 05-08
//! add:
//!   - 05.0: `begin_testing_inventory` (drift gate / SSOT for the
//!     begin-testing menu graph)
//!   - 05.2: `acs`, `graphic_rendition` (combined screen on tack
//!     v1.08; see each module's rustdoc)
//!   - 05: `color`, `cursor_movement`, `padding`
//!   - 06: `menu_inventory` (shared drift-gate helper for Section 06),
//!     `tools_menu_inventory` (06.0), `status_reports_inventory`
//!     (06.0.b), `status_reports` (06.1), `sgr_modes` (06.2),
//!     `character_sets` (06.3), `enq_ack` (06.4)
//!   - 08: keyboard / function key consts
//!
//! Each submodule defines `pub const SCENARIO_*: ScenarioSpec` values
//! and a `pub fn parse_*_screen(grid: &str) -> ScreenFacts` function
//! pointer. `ScenarioSpec` is `const`-constructible so the catalog
//! forms `pub const ALL_*: &[&ScenarioSpec]` arrays for
//! exhaustiveness tests.

pub mod acs;
pub mod begin_testing_inventory;
pub mod color;
pub mod cursor_movement;
pub mod graphic_rendition;
pub mod menu_inventory;
pub mod modes;
pub mod padding;
pub mod status_reports;
pub mod status_reports_inventory;
pub mod tools_menu_inventory;
