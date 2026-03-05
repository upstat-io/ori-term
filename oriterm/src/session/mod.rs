//! GUI session model: windows, tabs, and pane layouts.
//!
//! This module owns all presentation state — how panes are grouped into
//! tabs, how tabs are grouped into windows, how panes are arranged
//! within a tab. The mux layer knows nothing about this; it just
//! provides panes.

// Section 02 will wire these types into App; until then they're unused.
#![allow(
    dead_code,
    unused_imports,
    reason = "consumed by App in mux-flatten section 02"
)]

pub mod id;
mod registry;
mod tab;
mod window;

// Layout submodules (populated by section 04):
// pub mod split_tree;
// pub mod floating;
// pub mod rect;
// pub mod compute;
// pub mod nav;

pub use id::{IdAllocator, SessionId, TabId, WindowId};
pub use registry::SessionRegistry;
pub use tab::Tab;
pub use window::Window;
