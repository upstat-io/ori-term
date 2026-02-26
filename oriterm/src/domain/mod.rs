//! Concrete domain implementations for shell spawning.
//!
//! [`LocalDomain`] spawns shells on the local machine via `portable-pty`.
//! [`WslDomain`] is a stub for future WSL support.

mod local;
mod wsl;

#[allow(
    unused_imports,
    reason = "consumed by InProcessMux, wired to App in Section 31.2"
)]
pub(crate) use local::LocalDomain;
#[allow(
    unused_imports,
    reason = "consumed by InProcessMux, wired to App in Section 31.2"
)]
pub(crate) use wsl::WslDomain;
