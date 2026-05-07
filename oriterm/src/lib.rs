//! GPU-accelerated terminal emulator library.
//!
//! This module provides the core application logic for the oriterm terminal
//! emulator. The binary entry point lives in `main.rs` and calls [`run()`].

#[cfg(feature = "profile")]
pub mod alloc;

pub(crate) mod app;
pub(crate) mod cli;
pub(crate) mod clipboard;
pub(crate) mod config;
mod entry;
pub(crate) mod event;
pub(crate) mod font;
pub mod gpu;
pub(crate) mod key_encoding;
pub(crate) mod keybindings;
pub(crate) mod log_filter;
pub(crate) mod platform;
pub(crate) mod scheme;
pub(crate) mod session;
pub(crate) mod url_detect;
pub(crate) mod widgets;
pub(crate) mod window;
pub(crate) mod window_manager;

pub use entry::run;

/// Re-exports for benches and integration tests. Not part of the stable public
/// API; gated `#[doc(hidden)]` so they don't surface in rustdoc.
#[doc(hidden)]
pub mod bench_seam {
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    pub use crate::font::discovery::enumerate_mono_families_from_roots;
    pub use crate::font::discovery::{FamilyEntry, enumerate_mono_families};
}
