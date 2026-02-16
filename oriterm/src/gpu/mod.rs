//! GPU rendering: wgpu state management and platform transparency.

pub(crate) mod state;
pub(crate) mod transparency;

pub(crate) use state::validate_gpu;
