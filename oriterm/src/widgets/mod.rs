//! Terminal-specific widgets that bridge `oriterm_ui` and the terminal.
//!
//! These widgets live in the binary crate because they depend on terminal
//! types (`CellMetrics`, `Tab`) that `oriterm_ui` doesn't know about.

pub(crate) mod terminal_grid;
pub(crate) mod terminal_preview;
