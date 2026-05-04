// SPDX-License-Identifier: Apache-2.0
//
// This module was originally part of the `alacritty_terminal` crate, which is
// licensed under the Apache License, Version 2.0 and is part of the Alacritty
// project (https://github.com/alacritty/alacritty).

//! ANSI Terminal Stream Parsing.

use core::time::Duration;

mod attr;
mod colors;
mod dispatch;
mod handler;
pub mod observer;
mod processor;
mod types;

// Re-export cursor_icon (was `pub use` in original).
#[doc(inline)]
pub use cursor_icon;

// Re-export all public items to preserve the crate's public API.
pub use colors::{Hyperlink, Rgb};
pub use handler::Handler;
pub use observer::{PerformAction, PerformActionCollector, PerformObserver};
#[cfg(feature = "std")]
pub use processor::StdSyncHandler;
pub use processor::{Processor, Timeout};
pub use types::{
    Attr, CharsetIndex, ClearMode, Color, CursorShape, CursorStyle, KeyboardModes,
    KeyboardModesApplyBehavior, LineClearMode, Mode, ModifyOtherKeys, NamedColor, NamedMode,
    NamedPrivateMode, PrivateMode, ScpCharPath, ScpUpdateMode, StandardCharset,
    TabulationClearMode, C0,
};

/// Maximum time before a synchronized update is aborted.
const SYNC_UPDATE_TIMEOUT: Duration = Duration::from_millis(150);

// Tests for parsing escape sequences.
//
// Byte sequences used in these tests are recording of pty stdout.
#[cfg(test)]
mod tests;
