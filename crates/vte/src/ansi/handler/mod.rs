//! Handler trait for terminal emulator actions.
//!
//! The `Handler` trait is the single semantic contract between the VTE parser
//! and its consumer (in oriterm, `oriterm_core::term::Term`). The trait body
//! is assembled from three method-group macros so each source file stays
//! under the 500-line hygiene cap:
//!
//! - [`core_methods`] — upstream/core terminal methods (cursor, modes,
//!   charsets, sixel/DCS/APC/iTerm2-file entry points). Exports
//!   `handler_core_methods!`.
//! - [`vendored_osc_methods`] — oriterm vendored-patch OSC trait methods
//!   (Section 10.0 OSC 1337 iTerm2 sub-ops + Section 10.9 color extensions).
//!   Exports `handler_vendored_osc_methods!`.
//! - [`dec_private_methods`] — Section 09A DEC private CSI extensions
//!   (rectangular-area ops + presentation ops). Exports
//!   `handler_dec_private_methods!`.
//!
//! Each sibling file defines exactly one `macro_rules!` macro whose body is
//! the method declarations for that section. `mod.rs` declares the trait
//! and invokes all three macros inside the trait body; each macro expands
//! to a sequence of trait items (items-level expansion is supported by
//! Rust for `macro_rules!`, unlike the expression-level-only `include!`).
//!
//! See `crates/vte/README.md` for the oriterm vendored-patch policy.

extern crate alloc;

use alloc::string::String;

use cursor_icon::CursorIcon;

use super::colors::{Hyperlink, Rgb};
use super::types::{
    Attr, CharsetIndex, ClearMode, CursorShape, CursorStyle, KeyboardModes,
    KeyboardModesApplyBehavior, LineClearMode, Mode, ModifyOtherKeys, PrivateMode, ScpCharPath,
    ScpUpdateMode, StandardCharset, TabulationClearMode,
};

mod core_methods;
mod dec_private_methods;
mod vendored_osc_methods;

use core_methods::handler_core_methods;
use dec_private_methods::handler_dec_private_methods;
use vendored_osc_methods::handler_vendored_osc_methods;

/// Type that handles actions from the parser.
///
/// XXX Should probably not provide default impls for everything, but it makes
/// writing specific handler impls for tests far easier.
pub trait Handler {
    handler_core_methods!();
    handler_vendored_osc_methods!();
    handler_dec_private_methods!();
}
