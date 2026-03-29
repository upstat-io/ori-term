//! Per-window state container.
//!
//! Groups all state that is specific to a single OS window: widgets, render
//! caches, interaction state, and the dirty flag. Extracted from [`App`] to
//! enable multi-window support (Section 32.3).

use std::time::Instant;

use oriterm_mux::id::PaneId;

use crate::session::DividerLayout;

use oriterm_ui::draw::Scene;
use oriterm_ui::surface::{DamageSet, RenderStrategy};
use oriterm_ui::widgets::status_bar::StatusBarWidget;
use oriterm_ui::widgets::tab_bar::{TabBarWidget, TabSlideState};

use super::context_menu::ContextMenuState;
use super::divider_drag::DividerDragState;
use super::floating_drag::FloatingDragState;
use super::tab_drag::TabDragState;
use crate::font::TextShapeCache;
use crate::gpu::{FrameInput, PaneRenderCache, WindowRenderer};
use crate::url_detect::{DetectedUrl, UrlDetectCache};
use crate::widgets::terminal_grid::TerminalGridWidget;
use crate::window::TermWindow;

/// Per-window state: widgets, caches, interaction state, and dirty flag.
///
/// Each OS window gets its own `WindowContext`. The [`App`](super::App) stores
/// these in a `HashMap<WindowId, WindowContext>` keyed by winit window ID.
pub(crate) struct WindowContext {
    // Core window handle.
    pub(super) window: TermWindow,

    // Per-window GPU renderer (owns fonts, atlases, shaping, instance buffers).
    pub(super) renderer: Option<WindowRenderer>,

    // Widget layer.
    pub(super) tab_bar: TabBarWidget,
    pub(super) status_bar: StatusBarWidget,
    pub(super) terminal_grid: TerminalGridWidget,

    // Render caches.
    pub(super) pane_cache: PaneRenderCache,
    pub(super) frame: Option<FrameInput>,
    pub(super) chrome_scene: Scene,
    /// Pane rendered in the previous single-pane frame. Used to detect
    /// tab switches / tear-off so `renderable_cache` contamination from
    /// `swap_renderable_content` is flushed with a forced refresh.
    pub(super) last_rendered_pane: Option<PaneId>,

    // Layout caches.
    pub(super) tab_bar_phys_rect: oriterm_ui::geometry::Rect,
    pub(super) status_bar_phys_rect: oriterm_ui::geometry::Rect,
    pub(super) cached_dividers: Option<Vec<DividerLayout>>,

    // Tab slide animation.
    pub(super) tab_slide: TabSlideState,

    /// Pure UI framework state (interaction, focus, overlays, compositor, animation).
    pub(super) root: oriterm_ui::window_root::WindowRoot,

    // Terminal-specific interaction state.
    pub(super) hovering_divider: Option<DividerLayout>,
    pub(super) divider_drag: Option<DividerDragState>,
    pub(super) floating_drag: Option<FloatingDragState>,
    pub(super) tab_drag: Option<TabDragState>,
    pub(super) context_menu: Option<ContextMenuState>,
    pub(super) hovered_url: Option<DetectedUrl>,
    pub(super) url_cache: UrlDetectCache,
    pub(super) last_drag_area_press: Option<Instant>,
    /// Last tab body press: (tab index, timestamp) for double-click detection.
    pub(super) last_tab_press: Option<(usize, Instant)>,

    // Text shaping cache (persists across frames for cached UI text measurer).
    pub(super) text_cache: TextShapeCache,

    // Reusable buffers.
    pub(super) search_bar_buf: String,

    // Surface strategy and damage tracking.
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
    /// Chrome/overlay content has changed since the last full content render.
    ///
    /// When `true`, the GPU content cache texture is stale and
    /// `render_to_surface` must do a full render even if terminal content
    /// hasn't changed. Set by chrome hover, overlay animations, and other
    /// UI state changes. Cleared after a full content render.
    pub(super) ui_stale: bool,
}

impl WindowContext {
    /// Create a new window context from its constituent parts.
    ///
    /// The `window`, `tab_bar`, and `terminal_grid` are created during init;
    /// all other fields start at their defaults.
    pub fn new(
        window: TermWindow,
        tab_bar: TabBarWidget,
        status_bar: StatusBarWidget,
        terminal_grid: TerminalGridWidget,
        renderer: Option<WindowRenderer>,
    ) -> Self {
        Self {
            window,
            renderer,
            tab_bar,
            status_bar,
            terminal_grid,
            pane_cache: PaneRenderCache::new(),
            frame: None,
            chrome_scene: Scene::new(),
            last_rendered_pane: None,
            tab_slide: TabSlideState::new(),
            tab_bar_phys_rect: oriterm_ui::geometry::Rect::new(0.0, 0.0, 0.0, 0.0),
            status_bar_phys_rect: oriterm_ui::geometry::Rect::new(0.0, 0.0, 0.0, 0.0),
            cached_dividers: None,
            root: oriterm_ui::window_root::WindowRoot::new(
                oriterm_ui::widgets::label::LabelWidget::new(""),
            ),
            hovering_divider: None,
            divider_drag: None,
            floating_drag: None,
            tab_drag: None,
            context_menu: None,
            hovered_url: None,
            url_cache: UrlDetectCache::default(),
            last_drag_area_press: None,
            last_tab_press: None,
            text_cache: TextShapeCache::new(),
            search_bar_buf: String::new(),
            render_strategy: RenderStrategy::TerminalCached,
            damage: DamageSet::default(),
            ui_stale: true,
        }
    }
}
