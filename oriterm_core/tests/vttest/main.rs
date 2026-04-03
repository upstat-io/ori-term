//! vttest golden snapshot tests.
//!
//! Spawns `vttest` in a real PTY, feeds its output through `Term`'s VTE
//! parser, sends scripted keystrokes to navigate menus, and captures grid
//! snapshots at each test screen. Snapshots are compared against insta
//! golden references.
//!
//! Tests run at multiple terminal sizes (80x24, 97x33, 120x40) to catch
//! size-dependent bugs in cursor positioning, origin mode, and border drawing.
//!
//! Requires `vttest` installed (`sudo apt install vttest`).
//!
//! Run: `cargo test -p oriterm_core --test vttest`

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
