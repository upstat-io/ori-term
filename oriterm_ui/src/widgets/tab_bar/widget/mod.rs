//! Tab bar rendering widget.
//!
//! [`TabBarWidget`] draws the tab strip: tab backgrounds with titles, close
//! buttons, separators, new-tab (+) button, and dropdown button. All
//! coordinates are in logical pixels; the caller applies scale at the
//! rendering boundary.
//!
//! The widget implements [`Widget`] for draw integration. Event handling
//! dispatch is Section 16.3.

mod animation;
mod control_state;
mod controls_draw;
mod drag_draw;
mod draw;

use std::time::{Duration, Instant};

use crate::animation::{AnimBehavior, AnimProperty};
#[cfg(not(target_os = "macos"))]
use crate::color::Color;
use crate::theme::UiTheme;
use crate::widget_id::WidgetId;
#[cfg(not(target_os = "macos"))]
use crate::widgets::window_chrome::controls::{ControlButtonColors, WindowControlButton};
#[cfg(not(target_os = "macos"))]
use crate::widgets::window_chrome::layout::ControlKind;

use super::colors::TabBarColors;
use super::hit::TabBarHit;
use super::layout::TabBarLayout;

/// Duration for tab hover background animation.
const TAB_HOVER_DURATION: Duration = Duration::from_millis(100);

/// Duration for close button fade in/out animation.
const CLOSE_BTN_FADE_DURATION: Duration = Duration::from_millis(80);

/// Duration for tab open width animation.
const TAB_OPEN_DURATION: Duration = Duration::from_millis(200);

/// Duration for tab close width animation.
const TAB_CLOSE_DURATION: Duration = Duration::from_millis(150);

/// Icon type for tab entries.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TabIcon {
    /// Single emoji grapheme cluster.
    Emoji(String),
}

/// Per-tab visual state provided by the application layer.
#[derive(Debug, Clone)]
pub struct TabEntry {
    /// Tab title (empty string shows "Terminal" as fallback).
    pub title: String,
    /// Optional icon to show before the title.
    pub icon: Option<TabIcon>,
    /// When the bell last fired (for pulse animation). `None` if no bell.
    pub bell_start: Option<Instant>,
}

impl TabEntry {
    /// Creates a tab entry with the given title, no icon, and no bell.
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            icon: None,
            bell_start: None,
        }
    }

    /// Sets the tab icon.
    #[must_use]
    pub fn with_icon(mut self, icon: Option<TabIcon>) -> Self {
        self.icon = icon;
        self
    }
}

/// Tab bar rendering widget.
///
/// Holds all visual state needed to draw the tab strip. The application
/// layer updates state through setter methods; the widget's [`draw`]
/// implementation emits primitives into the [`Scene`](crate::draw::Scene).
pub struct TabBarWidget {
    id: WidgetId,

    // Tab data.
    tabs: Vec<TabEntry>,
    active_index: usize,

    // Style-driven geometry.
    metrics: super::constants::TabBarMetrics,

    // Computed layout.
    layout: TabBarLayout,
    colors: TabBarColors,
    window_width: f32,
    tab_width_lock: Option<f32>,

    // Interaction state.
    hover_hit: TabBarHit,

    // Drag state: (tab index, visual X position in logical pixels).
    drag_visual: Option<(usize, f32)>,

    // Per-tab animation offsets for smooth transitions (pixels).
    anim_offsets: Vec<f32>,

    // Per-tab hover animation progress (0.0 = inactive, 1.0 = hovered).
    hover_progress: Vec<AnimProperty<f32>>,

    // Per-tab close button fade (0.0 = hidden, 1.0 = visible).
    close_btn_opacity: Vec<AnimProperty<f32>>,

    // Per-tab width multiplier for open/close animation (0.0 = collapsed, 1.0 = full).
    width_multipliers: Vec<AnimProperty<f32>>,

    // Per-tab closing flag (true = tab is animating closed, skip interaction).
    closing_tabs: Vec<bool>,

    // Window control buttons: [minimize, maximize/restore, close].
    #[cfg(not(target_os = "macos"))]
    controls: [WindowControlButton; 3],
    /// Index of the currently pressed control button (for routing mouse-up).
    #[cfg(not(target_os = "macos"))]
    pressed_control: Option<usize>,

    /// Extra left margin for platform chrome (macOS traffic lights).
    left_inset: f32,
}

impl TabBarWidget {
    /// Creates a new tab bar widget with default dark theme colors.
    pub fn new(window_width: f32) -> Self {
        Self::with_theme(window_width, &UiTheme::dark())
    }

    /// Creates a new tab bar widget with colors from the given theme.
    pub fn with_theme(window_width: f32, theme: &UiTheme) -> Self {
        Self::with_theme_and_metrics(
            window_width,
            theme,
            super::constants::TabBarMetrics::DEFAULT,
        )
    }

    /// Creates a new tab bar widget with colors and style-driven metrics.
    pub fn with_theme_and_metrics(
        window_width: f32,
        theme: &UiTheme,
        metrics: super::constants::TabBarMetrics,
    ) -> Self {
        let layout = TabBarLayout::compute(0, window_width, None, 0.0, &metrics);

        Self {
            id: WidgetId::next(),
            tabs: Vec::new(),
            active_index: 0,
            metrics,
            layout,
            colors: TabBarColors::from_theme(theme),
            window_width,
            tab_width_lock: None,
            hover_hit: TabBarHit::None,
            drag_visual: None,
            anim_offsets: Vec::new(),
            hover_progress: Vec::new(),
            close_btn_opacity: Vec::new(),
            width_multipliers: Vec::new(),
            closing_tabs: Vec::new(),
            #[cfg(not(target_os = "macos"))]
            controls: create_controls(control_colors_from_theme(theme)),
            #[cfg(not(target_os = "macos"))]
            pressed_control: None,
            left_inset: 0.0,
        }
    }

    /// Returns the current tab bar metrics.
    pub fn metrics(&self) -> &super::constants::TabBarMetrics {
        &self.metrics
    }

    /// Sets the tab bar metrics (style change) and recomputes layout.
    pub fn set_metrics(&mut self, metrics: super::constants::TabBarMetrics) {
        self.metrics = metrics;
        self.recompute_layout();
    }

    // Theme

    /// Updates all theme-derived colors from a new [`UiTheme`].
    pub fn apply_theme(&mut self, theme: &UiTheme) {
        self.colors = TabBarColors::from_theme(theme);
        #[cfg(not(target_os = "macos"))]
        {
            let ctrl_colors = control_colors_from_theme(theme);
            for ctrl in &mut self.controls {
                ctrl.set_colors(ctrl_colors);
            }
        }
    }

    // State setters

    /// Updates the tab list and recomputes layout.
    ///
    /// Resets per-tab animation state (hover progress, close button opacity)
    /// since tab indices may have changed due to add/remove/reorder.
    pub fn set_tabs(&mut self, tabs: Vec<TabEntry>) {
        let n = tabs.len();
        self.tabs = tabs;
        self.hover_progress.clear();
        self.hover_progress.resize_with(n, || {
            AnimProperty::with_behavior(
                0.0,
                AnimBehavior::ease_out(TAB_HOVER_DURATION.as_millis() as u64),
            )
        });
        self.close_btn_opacity.clear();
        self.close_btn_opacity.resize_with(n, || {
            AnimProperty::with_behavior(
                0.0,
                AnimBehavior::ease_out(CLOSE_BTN_FADE_DURATION.as_millis() as u64),
            )
        });
        self.width_multipliers.clear();
        self.width_multipliers.resize_with(n, || {
            AnimProperty::with_behavior(
                1.0,
                AnimBehavior::ease_out(TAB_OPEN_DURATION.as_millis() as u64),
            )
        });
        self.closing_tabs.clear();
        self.closing_tabs.resize(n, false);
        self.recompute_layout();
    }

    /// Sets the active tab index.
    pub fn set_active_index(&mut self, index: usize) {
        self.active_index = index;
    }

    /// Updates the window width and recomputes layout.
    pub fn set_window_width(&mut self, width: f32) {
        self.window_width = width;
        self.recompute_layout();
    }

    /// Sets the tab width lock (freezes widths during hover).
    pub fn set_tab_width_lock(&mut self, lock: Option<f32>) {
        self.tab_width_lock = lock;
        self.recompute_layout();
    }

    /// Sets the left inset for platform chrome (macOS traffic lights).
    ///
    /// On macOS: `MACOS_TRAFFIC_LIGHT_WIDTH` (76px). On Windows/Linux: 0.
    pub fn set_left_inset(&mut self, inset: f32) {
        self.left_inset = inset;
        self.recompute_layout();
    }
}

// Free functions

/// Builds [`ControlButtonColors`] from a [`UiTheme`].
#[cfg(not(target_os = "macos"))]
fn control_colors_from_theme(theme: &UiTheme) -> ControlButtonColors {
    ControlButtonColors {
        fg: theme.fg_primary,
        bg: Color::TRANSPARENT,
        hover_bg: theme.bg_hover,
        close_hover_bg: theme.close_hover_bg,
        close_pressed_bg: theme.close_pressed_bg,
    }
}

/// Creates the three control buttons with initial colors.
#[cfg(not(target_os = "macos"))]
fn create_controls(colors: ControlButtonColors) -> [WindowControlButton; 3] {
    let min_btn = WindowControlButton::new(ControlKind::Minimize, colors);
    let max_btn = WindowControlButton::new(ControlKind::MaximizeRestore, colors);
    let close_btn = WindowControlButton::new(ControlKind::Close, colors);
    [min_btn, max_btn, close_btn]
}

// Test helpers

#[cfg(test)]
impl TabBarWidget {
    /// Test-only access to bell phase computation.
    pub fn bell_phase_for_test(tab: &TabEntry, now: Instant) -> f32 {
        draw::bell_phase(tab, now)
    }

    /// Test-only access to drag-adjusted new-tab button X.
    pub fn test_new_tab_button_x(&self) -> f32 {
        draw::new_tab_button_x(self)
    }

    /// Test-only access to drag-adjusted dropdown button X.
    pub fn test_dropdown_button_x(&self) -> f32 {
        draw::dropdown_button_x(self)
    }

    /// Test-only access to hover progress for a tab.
    pub fn test_hover_progress(&self, index: usize, now: Instant) -> f32 {
        self.hover_progress.get(index).map_or(0.0, |p| p.get(now))
    }

    /// Test-only access to close button opacity for a tab.
    pub fn test_close_btn_opacity(&self, index: usize, now: Instant) -> f32 {
        self.close_btn_opacity
            .get(index)
            .map_or(0.0, |o| o.get(now))
    }
}
