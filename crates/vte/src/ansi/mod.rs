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
mod processor;
mod types;

// Re-export cursor_icon (was `pub use` in original).
#[doc(inline)]
pub use cursor_icon;

// Re-export all public items to preserve the crate's public API.
pub use colors::{Hyperlink, Rgb};
pub use handler::Handler;
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

/// Maximum number of bytes read in one synchronized update (2MiB).
const SYNC_BUFFER_SIZE: usize = 0x20_0000;

/// Number of bytes in the BSU/ESU CSI sequences.
const SYNC_ESCAPE_LEN: usize = 8;

/// BSU CSI sequence for beginning or extending synchronized updates.
const BSU_CSI: [u8; SYNC_ESCAPE_LEN] = *b"\x1b[?2026h";

/// ESU CSI sequence for terminating synchronized updates.
const ESU_CSI: [u8; SYNC_ESCAPE_LEN] = *b"\x1b[?2026l";

// Tests for parsing escape sequences.
//
// Byte sequences used in these tests are recording of pty stdout.
#[cfg(test)]
mod tests;
