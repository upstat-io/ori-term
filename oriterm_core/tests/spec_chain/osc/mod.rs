//! OSC suite spec_chain coverage (high-level `Processor` path).
//!
//! Each submodule covers one OSC family that routes through the
//! high-level `Processor::advance_with_observer` path. Mux-intercepted
//! OSCs (7, 9, 99, 133, 633, 777) live in `oriterm_mux/src/shell_integration/tests.rs`.

mod basic;
mod clipboard;
mod color_reset;
mod cursor;
mod default_colors;
mod hyperlinks;
mod iterm2_non_image;
mod missing_rows;
mod palette;
