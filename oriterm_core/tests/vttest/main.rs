//! vttest conformance tests.
//!
//! Spawns `vttest` in a real PTY, feeds its output through `Term`'s VTE
//! parser, sends scripted keystrokes to navigate menus, and captures grid
//! snapshots at each test screen. Snapshots are compared against insta
//! golden references. Structural assertions verify key screens (border
//! fills, DA/DSR responses, ICH/DCH/IL/DL character patterns).
//!
//! Tests run at multiple terminal sizes (80x24, 97x33, 120x40) to catch
//! size-dependent bugs in cursor positioning, origin mode, and border drawing.
//!
//! # Coverage
//!
//! - Menu 1: Cursor movement (border fill, DECCOLM 132-col).
//! - Menu 2: Screen features (scroll, SGR, origin mode, SAVE/RESTORE).
//! - Menu 3: Character sets (DEC Special Graphics, SI/SO).
//! - Menu 4: Double-size characters (snapshot baseline — DECDHL/DECDWL not implemented).
//! - Menu 5: Keyboard (LED, auto-repeat — hardware-dependent, VTE layer only).
//! - Menu 6: Terminal reports (DA1/DA2/DA3, DSR, DECRQM).
//! - Menu 7: VT52 mode (navigation only — VT52 not implemented).
//! - Menu 8: VT102 features (ICH/DCH/IL/DL with structural assertions).
//!
//! # Cross-platform
//!
//! These tests require a Unix PTY (`portable-pty` on Linux/macOS). On Windows
//! (cross-compile target), vttest is unavailable and the tests gracefully skip
//! via `vttest_available()` returning false. CI runs on Linux and macOS.
//!
//! # Commands
//!
//! - Text tests: `cargo test -p oriterm_core --test vttest`
//! - GPU golden tests: `cargo test -p oriterm --features gpu-tests -- vttest_golden`
//! - Update snapshots: `INSTA_UPDATE=1 cargo test -p oriterm_core --test vttest`
//!
//! Requires `vttest` installed (`sudo apt install vttest`).

mod menu1;
mod menu2;
mod menu3;
mod menu4;
mod menu5;
mod menu6;
mod menu7;
mod menu8;
mod pty_size;
mod session;
