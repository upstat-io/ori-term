//! Cross-platform PTY abstraction.
//!
//! Provides PTY creation, shell spawning, and a background reader thread.
//! Uses `portable-pty` for platform abstraction: `ConPTY` on Windows,
//! `openpty`/`forkpty` on Linux, POSIX PTY on macOS.

mod reader;
mod spawn;

#[cfg(unix)]
pub mod signal;

pub use reader::{PtyEvent, PtyReader};
pub use spawn::{PtyConfig, spawn_pty};

#[cfg(test)]
mod tests;
