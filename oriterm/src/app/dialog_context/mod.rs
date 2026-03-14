//! Per-dialog-window state container.
//!
//! Dialog windows (settings, confirmations, about) are real OS windows with
//! their own GPU surface and UI-only renderer. Unlike [`WindowContext`] they
//! have no terminal grid, tab bar, or pane cache — just UI widgets.

mod content_actions;
mod event_handling;

use std::sync::Arc;

use winit::window::Window;

use oriterm_ui::compositor::layer_animator::LayerAnimator;
use oriterm_ui::compositor::layer_tree::LayerTree;
use oriterm_ui::draw::{DrawList, SceneCache};
use oriterm_ui::geometry::Rect;
use oriterm_ui::invalidation::InvalidationTracker;
use oriterm_ui::overlay::OverlayManager;
use oriterm_ui::scale::ScaleFactor;
use oriterm_ui::widgets::Widget;
use oriterm_ui::widgets::dialog::DialogWidget;
use oriterm_ui::widgets::settings_panel::SettingsPanel;
use oriterm_ui::widgets::window_chrome::WindowChromeWidget;

use crate::app::settings_overlay::SettingsIds;
use crate::config::Config;
use crate::event::ConfirmationKind;
use crate::font::TextShapeCache;
use crate::gpu::WindowRenderer;
use crate::window_manager::types::DialogKind;

/// Per-dialog-window state. Lighter than `WindowContext` — no terminal grid,
/// no tab bar, no pane cache. Dialogs are not part of the session model.
pub(crate) struct DialogWindowContext {
    /// The native window handle.
    pub(super) window: Arc<Window>,
    /// wgpu rendering surface bound to this dialog window.
    pub(super) surface: wgpu::Surface<'static>,
    /// Surface configuration (format, size, present mode).
    pub(super) surface_config: wgpu::SurfaceConfiguration,
    /// Per-window GPU renderer in `UiOnly` mode — no grid pipelines.
    pub(super) renderer: Option<WindowRenderer>,
    /// What kind of dialog this is.
    pub(super) kind: DialogKind,
    /// The dialog content (widget tree + associated state).
    pub(super) content: DialogContent,
    /// Dialog title bar with close button only (dialog chrome mode).
    pub(super) chrome: WindowChromeWidget,
    /// Overlay manager for popups within the dialog (e.g. dropdowns).
    pub(super) overlays: OverlayManager,
    /// Compositor layer tree for overlay animations.
    pub(super) layer_tree: LayerTree,
    /// Compositor layer animator for overlay fade transitions.
    pub(super) layer_animator: LayerAnimator,
    /// Text shaping cache (persists across frames for cached UI text measurer).
    pub(super) text_cache: TextShapeCache,
    /// Scratch draw list for rendering the dialog frame.
    pub(super) draw_list: DrawList,
    /// DPI scale factor for this window's display.
    pub(super) scale_factor: ScaleFactor,
    /// Last cursor position in logical pixels (for mouse click handlers).
    pub(super) last_cursor_pos: oriterm_ui::geometry::Point,
    /// Per-widget scene cache for retained UI rendering.
    pub(super) scene_cache: SceneCache,
    /// Per-widget invalidation tracker.
    pub(super) invalidation: InvalidationTracker,
    /// Whether this dialog needs a redraw.
    pub(super) dirty: bool,
    /// Whether this dialog should bypass the normal frame budget once.
    pub(super) urgent_redraw: bool,
}

/// Dialog-specific content and associated state.
///
/// Each variant carries the widget tree and any pending state for that
/// dialog type. The variant determines how events are dispatched and
/// how the dialog renders.
pub(crate) enum DialogContent {
    /// Settings / preferences dialog.
    ///
    /// `panel` and configs are boxed to keep the enum's stack size down
    /// (`SettingsPanel` alone is ~800 bytes, each `Config` ~500 bytes).
    Settings {
        /// The settings form panel widget.
        panel: Box<SettingsPanel>,
        /// Widget IDs for matching actions to config fields.
        ids: SettingsIds,
        /// Working copy of the config being edited. Applied on Save.
        pending_config: Box<Config>,
        /// Original config snapshot for Cancel / diff detection.
        #[expect(
            dead_code,
            reason = "reserved for dirty detection in future cancel-guard UX"
        )]
        original_config: Box<Config>,
    },
    /// Confirmation prompt (e.g. paste with newlines, close with running processes).
    Confirmation {
        /// The confirmation dialog widget (title, message, OK/Cancel buttons).
        /// Boxed to avoid large enum variant size difference.
        dialog: Box<DialogWidget>,
        /// What action to take when the user clicks OK.
        kind: ConfirmationKind,
    },
}

impl DialogContent {
    /// Returns a shared reference to the content's root widget.
    pub(super) fn content_widget(&self) -> &dyn Widget {
        match self {
            Self::Settings { panel, .. } => &**panel,
            Self::Confirmation { dialog, .. } => &**dialog,
        }
    }

    /// Returns an exclusive reference to the content's root widget.
    pub(super) fn content_widget_mut(&mut self) -> &mut dyn Widget {
        match self {
            Self::Settings { panel, .. } => &mut **panel,
            Self::Confirmation { dialog, .. } => &mut **dialog,
        }
    }

    /// Invalidate any cached layout state.
    ///
    /// Called when external state that affects layout changes (e.g. DPI)
    /// but the bounds remain the same, so the cache wouldn't auto-invalidate.
    pub(super) fn invalidate_cache(&self) {
        match self {
            Self::Settings { panel, .. } => panel.invalidate_cache(),
            Self::Confirmation { .. } => {}
        }
    }
}

impl DialogWindowContext {
    /// Create a new dialog window context.
    #[expect(
        clippy::too_many_arguments,
        reason = "constructor wiring: window + surface + renderer + content + scale + theme"
    )]
    pub(crate) fn new(
        window: Arc<Window>,
        surface: wgpu::Surface<'static>,
        surface_config: wgpu::SurfaceConfiguration,
        renderer: Option<WindowRenderer>,
        kind: DialogKind,
        content: DialogContent,
        scale_factor: ScaleFactor,
        theme: &oriterm_ui::theme::UiTheme,
    ) -> Self {
        let (w, h) = (surface_config.width, surface_config.height);
        let scale = scale_factor.factor() as f32;
        let logical_w = w as f32 / scale;
        let logical_h = h as f32 / scale;
        let viewport = Rect::new(0.0, 0.0, logical_w, logical_h);
        let chrome = WindowChromeWidget::dialog(kind.title(), logical_w, theme);
        Self {
            window,
            surface,
            surface_config,
            renderer,
            kind,
            content,
            chrome,
            overlays: OverlayManager::new(viewport),
            layer_tree: LayerTree::new(viewport),
            layer_animator: LayerAnimator::new(),
            text_cache: TextShapeCache::new(),
            draw_list: DrawList::new(),
            scene_cache: SceneCache::new(),
            invalidation: InvalidationTracker::new(),
            scale_factor,
            last_cursor_pos: oriterm_ui::geometry::Point::new(0.0, 0.0),
            dirty: true,
            urgent_redraw: false,
        }
    }

    /// Reconfigure the surface after a resize.
    pub(super) fn resize_surface(&mut self, width: u32, height: u32, gpu: &crate::gpu::GpuState) {
        let w = width.max(1);
        let h = height.max(1);
        self.surface_config.width = w;
        self.surface_config.height = h;
        gpu.configure_surface(&self.surface, &self.surface_config);
        let scale = self.scale_factor.factor() as f32;
        let logical_w = w as f32 / scale;
        self.chrome.set_window_width(logical_w);
        let logical_w = w as f32 / scale;
        let logical_h = h as f32 / scale;
        self.overlays
            .set_viewport(Rect::new(0.0, 0.0, logical_w, logical_h));
        self.scene_cache.clear();
        self.invalidation.invalidate_all();
        self.dirty = true;
    }

    /// Schedule an immediate redraw for latency-sensitive UI feedback.
    pub(super) fn request_urgent_redraw(&mut self) {
        self.dirty = true;
        self.urgent_redraw = true;
    }

    /// Whether this dialog has a non-zero surface area for rendering.
    pub(super) fn has_surface_area(&self) -> bool {
        self.surface_config.width > 0 && self.surface_config.height > 0
    }
}
