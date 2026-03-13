//! Platform-specific abstractions for cross-platform operations.
//!
//! Each submodule provides a unified API with `#[cfg]`-gated platform
//! implementations. Follows Chromium's pattern of thin platform glue
//! behind a shared interface.

pub(crate) mod config_paths;
pub(crate) mod jump_list;
pub(crate) mod memory;
pub(crate) mod notify;
pub(crate) mod scroll;
pub(crate) mod shutdown;
pub(crate) mod theme;
pub(crate) mod url;
