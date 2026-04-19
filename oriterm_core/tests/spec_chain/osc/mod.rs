//! OSC suite spec_chain coverage (high-level `Processor` path).
//!
//! Each submodule covers one OSC family that routes through the
//! high-level `Processor::advance_with_observer` path. Mux-intercepted
//! OSCs (7, 9, 99, 133, 633, 777) live in `oriterm_mux/src/shell_integration/tests.rs`.

mod clipboard;
mod color_reset;
mod cursor;
mod hyperlinks;
mod iterm2_non_image;
