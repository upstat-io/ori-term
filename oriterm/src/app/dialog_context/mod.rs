//! Per-dialog-window state container.
//!
//! Dialog windows (settings, confirmations, about) are real OS windows with
//! their own GPU surface and UI-only renderer. Unlike [`WindowContext`] they
//! have no terminal grid, tab bar, or pane cache — just UI widgets.

mod content_actions;
mod content_key_dispatch;
mod event_handling;
mod focus_setup;
pub(in crate::app) mod key_conversion;
mod keymap_dispatch;
mod overlay_actions;

use std::sync::Arc;

use winit::window::Window;

use oriterm_ui::draw::Scene;
use oriterm_ui::geometry::Rect;
use oriterm_ui::scale::ScaleFactor;
use oriterm_ui::surface::{DamageSet, RenderStrategy, SurfaceLifecycle};
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
    /// Pure UI framework state (interaction, focus, overlays, compositor, keymap, animation).
    pub(super) root: oriterm_ui::window_root::WindowRoot,
    /// Text shaping cache (persists across frames for cached UI text measurer).
    pub(super) text_cache: TextShapeCache,
    /// Scratch scene for rendering the dialog frame.
    pub(super) scene: Scene,
    /// DPI scale factor for this window's display.
    pub(super) scale_factor: ScaleFactor,
    /// Last cursor position in logical pixels (for mouse click handlers).
    pub(super) last_cursor_pos: oriterm_ui::geometry::Point,
    // Surface strategy, damage tracking, and lifecycle.
    #[expect(
        dead_code,
        reason = "vocabulary for retained-ui plan; consumed by future render paths"
    )]
    pub(super) render_strategy: RenderStrategy,
    #[expect(
        dead_code,
        reason = "vocabulary for retained-ui plan; consumed by future render paths"
    )]
    pub(super) damage: DamageSet,
    /// Lifecycle state for framework-managed visibility transitions.
    pub(super) lifecycle: SurfaceLifecycle,

    /// Whether the previous frame had widget animations in progress.
    /// Used for phase gating: when true, prepare + prepaint must run.
    pub(super) ui_stale: bool,
    /// Cached content layout node, keyed by viewport bounds.
    ///
    /// Avoids recomputing the full layout tree on every scroll/mouse event.
    /// Invalidated on resize, widget structural changes, and `content_offset`
    /// changes. The cache dramatically reduces per-scroll-tick cost from
    /// 3-4 tree walks to 0-1.
    pub(super) cached_layout: Option<(Rect, std::rc::Rc<oriterm_ui::layout::LayoutNode>)>,
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
        /// Boxed to keep the Settings variant small (`SettingsIds` contains a `Vec`).
        ids: Box<SettingsIds>,
        /// Working copy of the config being edited. Applied on Save.
        pending_config: Box<Config>,
        /// Original config snapshot for dirty detection (pending != original).
        original_config: Box<Config>,
        /// Current sidebar page index, preserved across rebuilds (e.g., reset).
        active_page: usize,
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
            root: oriterm_ui::window_root::WindowRoot::with_viewport(
                oriterm_ui::widgets::label::LabelWidget::new(""),
                viewport,
            ),
            text_cache: TextShapeCache::new(),
            scene: Scene::new(),
            scale_factor,
            last_cursor_pos: oriterm_ui::geometry::Point::new(0.0, 0.0),
            render_strategy: RenderStrategy::UiRetained,
            damage: DamageSet::default(),
            lifecycle: SurfaceLifecycle::CreatedHidden,
            ui_stale: true,
            cached_layout: None,
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
        self.root
            .set_viewport(Rect::new(0.0, 0.0, logical_w, logical_h));
        self.root.invalidation_mut().invalidate_all();
        self.root.damage_mut().reset();
        self.cached_layout = None;
        self.root.mark_dirty();
    }

    /// Schedule an immediate redraw for latency-sensitive UI feedback.
    ///
    /// Also calls `window.request_redraw()` so that winit generates a
    /// `RedrawRequested` event, which ensures the dialog is rendered
    /// promptly instead of waiting for `about_to_wait` — important for
    /// responsive scrolling and hover feedback.
    pub(super) fn request_urgent_redraw(&mut self) {
        self.root.mark_dirty();
        self.root.set_urgent_redraw(true);
        self.window.request_redraw();
    }

    /// Whether this dialog has a non-zero surface area for rendering.
    pub(super) fn has_surface_area(&self) -> bool {
        self.surface_config.width > 0 && self.surface_config.height > 0
    }
}

/// Returns `true` when a dialog content event requires an immediate redraw.
///
/// This decision combines three signals from the event dispatch result:
/// - `handled`: a widget consumed the event and mutated local state
///   (e.g. sidebar search text changed)
/// - `state_changed`: interaction state changed (focus cycling, active toggle)
/// - `paint_requested`: a controller explicitly requested repaint
///
/// Extracted as a testable function because this condition was the root
/// cause of TPR-10-014 (missing `handled` check caused silent stale frames).
pub(super) fn needs_content_redraw(
    handled: bool,
    state_changed: bool,
    requests: oriterm_ui::controllers::ControllerRequests,
) -> bool {
    handled
        || state_changed
        || requests.contains(oriterm_ui::controllers::ControllerRequests::PAINT)
}

/// Computes the parent map from a content widget's current layout (TPR-04-002).
#[expect(
    clippy::too_many_arguments,
    reason = "borrow-split fields from DialogWindowContext"
)]
pub(super) fn content_parent_map(
    panel: &dyn Widget,
    chrome_h: f32,
    renderer: &WindowRenderer,
    scale: f32,
    text_cache: &TextShapeCache,
    surface_config: &wgpu::SurfaceConfiguration,
    ui_theme: &oriterm_ui::theme::UiTheme,
) -> std::collections::HashMap<oriterm_ui::widget_id::WidgetId, oriterm_ui::widget_id::WidgetId> {
    use crate::font::CachedTextMeasurer;
    use oriterm_ui::interaction::build_parent_map;
    use oriterm_ui::layout::compute_layout;
    use oriterm_ui::widgets::LayoutCtx;

    let m = CachedTextMeasurer::new(renderer.ui_measurer(scale), text_cache, scale);
    let lctx = LayoutCtx {
        measurer: &m,
        theme: ui_theme,
    };
    let (w, h) = (
        surface_config.width as f32 / scale,
        surface_config.height as f32 / scale,
    );
    build_parent_map(&compute_layout(
        &panel.layout(&lctx),
        Rect::new(0.0, 0.0, w, h - chrome_h),
    ))
}

/// Recomputes the hot path from the stored cursor position after a rebuild.
///
/// Unlike `WindowRoot::clear_hot_path()` which unconditionally drops all
/// hover, this hit-tests the current widget tree against `last_cursor_pos`
/// to preserve hover on widgets that survive the rebuild (TPR-04-007).
///
/// Takes decomposed fields because call sites destructure `ctx.content`
/// (for panel/config access), preventing `&mut DialogWindowContext`.
#[expect(
    clippy::too_many_arguments,
    reason = "borrow-split fields from DialogWindowContext"
)]
pub(super) fn recompute_dialog_hot_path(
    root: &mut oriterm_ui::window_root::WindowRoot,
    chrome: &WindowChromeWidget,
    content_widget: &dyn Widget,
    cursor_pos: oriterm_ui::geometry::Point,
    renderer: Option<&WindowRenderer>,
    scale: f32,
    text_cache: &TextShapeCache,
    surface_config: &wgpu::SurfaceConfiguration,
    ui_theme: &oriterm_ui::theme::UiTheme,
) {
    use crate::font::CachedTextMeasurer;
    use oriterm_ui::input::layout_hit_test_path;
    use oriterm_ui::layout::compute_layout;
    use oriterm_ui::widgets::LayoutCtx;

    let chrome_h = chrome.caption_height();
    let mut hot_path = Vec::new();

    if cursor_pos.y >= chrome_h {
        let Some(renderer) = renderer else {
            root.clear_hot_path();
            return;
        };
        let m = CachedTextMeasurer::new(renderer.ui_measurer(scale), text_cache, scale);
        let w = surface_config.width as f32 / scale;
        let h = surface_config.height as f32 / scale;
        let vp = Rect::new(0.0, 0.0, w, h - chrome_h);
        let lctx = LayoutCtx {
            measurer: &m,
            theme: ui_theme,
        };
        let node = compute_layout(&content_widget.layout(&lctx), vp);
        let local = oriterm_ui::geometry::Point::new(cursor_pos.x, cursor_pos.y - chrome_h);
        let hit = layout_hit_test_path(&node, local);
        for entry in &hit.path {
            hot_path.push(entry.widget_id);
        }
    } else if let Some(btn_id) = chrome.widget_at_point(cursor_pos) {
        hot_path.push(chrome.id());
        hot_path.push(btn_id);
    } else {
        // Cursor is in the chrome area but not on any interactive control.
        // hot_path stays empty — all widgets become un-hot.
    }

    let changed = root.interaction_mut().update_hot_path(&hot_path);
    root.mark_widgets_prepaint_dirty(&changed);
}

#[cfg(test)]
mod tests;
