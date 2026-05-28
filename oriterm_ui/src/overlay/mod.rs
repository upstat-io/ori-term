//! Overlay and modal system for floating UI layers.
//!
//! Provides [`OverlayManager`] for managing a stack of overlays above the
//! main widget tree. Used by context menus, dropdown popups, command palette,
//! tooltips, and modal dialogs.

mod flash_widget;
mod manager;
mod overlay_id;
mod placement;

pub use manager::{
    CompositorHandles, FlashSpec, OverlayEventResult, OverlayManager, OverlayResponse,
};
pub use overlay_id::OverlayId;
pub use placement::Placement;

#[cfg(test)]
mod tests;
