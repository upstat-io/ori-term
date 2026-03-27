//! Icon atlas resolution for the current frame.
//!
//! Pre-resolves all icon atlas entries so widget `DrawCtx` can look up
//! icons by `(IconId, logical_size)` without touching the atlas directly.

use oriterm_ui::icons::{IconId, ResolvedIcon, SIDEBAR_NAV_ICON_SIZE};

use super::WindowRenderer;
use crate::gpu::state::GpuState;

impl WindowRenderer {
    /// Pre-resolve all icon atlas entries for the current frame.
    ///
    /// Icons are rasterized at **physical pixel size** (`logical × scale`)
    /// so each texel maps 1:1 to a screen pixel. The `ResolvedIcons` map
    /// is keyed by logical size (what widgets pass) so the scaling is
    /// transparent to widget code.
    ///
    /// Call once per frame before constructing `DrawCtx`.
    pub fn resolve_icons(&mut self, gpu: &GpuState, scale: f32) {
        self.resolved_icons.clear();
        for &(id, logical_size) in &Self::ICON_SIZES {
            let physical_size = (logical_size as f32 * scale).round() as u32;
            if physical_size == 0 {
                continue;
            }
            if let Some(entry) = self.icon_cache.get_or_insert(
                id,
                physical_size,
                scale,
                &mut self.atlas,
                &gpu.device,
                &gpu.queue,
            ) {
                self.resolved_icons.insert(
                    id,
                    logical_size,
                    ResolvedIcon {
                        atlas_page: entry.page,
                        uv: [entry.uv_x, entry.uv_y, entry.uv_w, entry.uv_h],
                    },
                );
            }
        }
    }

    /// Pre-resolved icon atlas entries for the current frame.
    ///
    /// Valid after [`resolve_icons`](Self::resolve_icons) has been called.
    pub fn resolved_icons(&self) -> &oriterm_ui::icons::ResolvedIcons {
        &self.resolved_icons
    }

    /// All `(IconId, logical_size)` pairs used by widgets.
    ///
    /// Sizes are in **logical pixels** — [`resolve_icons`](Self::resolve_icons)
    /// multiplies by the display scale factor to get the physical rasterization
    /// size. Derived from widget constants:
    /// - `Close` (tab): `(CLOSE_BUTTON_WIDTH - 2 * CLOSE_ICON_INSET).round()` = 10
    /// - `Plus`: `(PLUS_ARM * 2).round()` = 10
    /// - `ChevronDown`: `(CHEVRON_HALF * 2).round()` = 10
    /// - `Minimize`/`Maximize`/`Restore`/`WindowClose`: `SYMBOL_SIZE.round()` = 10
    pub(super) const ICON_SIZES: [(IconId, u32); 20] = [
        // Window chrome (10px logical).
        (IconId::Close, 10),
        (IconId::Plus, 10),
        (IconId::ChevronDown, 10),
        (IconId::Minimize, 10),
        (IconId::Maximize, 10),
        (IconId::Restore, 10),
        (IconId::WindowClose, 10),
        // Settings sidebar nav (SIDEBAR_NAV_ICON_SIZE logical).
        (IconId::Sun, SIDEBAR_NAV_ICON_SIZE),
        (IconId::Palette, SIDEBAR_NAV_ICON_SIZE),
        (IconId::Type, SIDEBAR_NAV_ICON_SIZE),
        (IconId::Terminal, SIDEBAR_NAV_ICON_SIZE),
        (IconId::Keyboard, SIDEBAR_NAV_ICON_SIZE),
        (IconId::Window, SIDEBAR_NAV_ICON_SIZE),
        (IconId::Bell, SIDEBAR_NAV_ICON_SIZE),
        (IconId::Activity, SIDEBAR_NAV_ICON_SIZE),
        // Sidebar search field (12px logical).
        (IconId::Search, 12),
        // Settings footer unsaved indicator (14px logical).
        (IconId::AlertCircle, 14),
        // Dropdown trigger filled triangle (10px logical).
        (IconId::DropdownArrow, 10),
        // Number input stepper arrows (8px logical, matching the 8px font-size).
        (IconId::StepperUp, 8),
        (IconId::StepperDown, 8),
    ];
}
