//! Spec_chain conversions for tack section 05 (`n) begin testing`) scenarios.
//!
//! The original tack-conformance plan
//! exercised the begin-testing submenu through real PTY navigation against
//! tack v1.08. Each scenario family is converted here into a standalone
//! protocol-level spec_chain test driving the catalog row(s) the scenario
//! exercises directly through `SpecHarness` — no PTY, no menu navigation,
//! no parser stubs.
//!
//! # Per-family conversion map
//!
//! | Scenario family    | Catalog rows exercised                         | File                  |
//! |---                 |---                                             |---                    |
//! | `acs`              | `ECMA48-C0-BEL`                                | `acs.rs`              |
//! | `graphic_rendition`| (combined with `acs` on tack v1.08)            | `graphic_rendition.rs`|
//! | `cursor_movement`  | `ECMA48-CSI-CUP`, `ECMA48-CSI-ED`              | `cursor_movement.rs`  |
//! | `modes`            | `DEC-DECAWM` (mode 7), `DEC-DECREVWRAP` (45)   | `modes.rs`            |
//! | `color`            | (none in ECMA-48; numeric terminfo caps only)  | `color.rs`            |
//! | `padding`          | `ECMA48-CSI-DA1` (already covered by pilot)    | `padding.rs`          |
//!
//! Empirical reality (tack v1.08, captured 2026-04-08): each tack test
//! screen surfaces ONE parenthesized cap label plus a `Done` terminator.
//! The conversion targets the protocol bytes the cap labels imply, not
//! the surface text — that is the whole point of the absorption strategy.

mod acs;
mod color;
mod cursor_movement;
mod graphic_rendition;
mod modes;
mod padding;
