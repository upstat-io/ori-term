//! Tab bar rendering widget.
//!
//! [`TabBarWidget`] draws the tab strip: tab backgrounds with titles, close
//! buttons, separators, new-tab (+) button, and dropdown button. All
//! coordinates are in logical pixels; the caller applies scale at the
//! rendering boundary.
//!
//! The widget implements [`Widget`] for draw integration. Event handling
//! stubs are provided here; full hit-test dispatch is Section 16.3.

mod controls_draw;
mod draw;

use std::time::Instant;

use crate::animation::Lerp;
use crate::color::Color;
use crate::geometry::{Point, Rect};
use crate::input::{HoverEvent, MouseButton, MouseEvent, MouseEventKind};
use crate::theme::UiTheme;
use crate::widget_id::WidgetId;
use crate::widgets::window_chrome::controls::{ControlButtonColors, WindowControlButton};
use crate::widgets::window_chrome::layout::ControlKind;
use crate::widgets::{EventCtx, Widget, WidgetResponse};

use super::colors::TabBarColors;
use super::constants::{DROPDOWN_BUTTON_WIDTH, NEW_TAB_BUTTON_WIDTH, TAB_BAR_HEIGHT};
use super::hit::TabBarHit;
use super::layout::TabBarLayout;

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
/// implementation emits [`DrawCommand`](crate::draw::DrawCommand)s into
/// the draw list.
pub struct TabBarWidget {
    id: WidgetId,

    // Tab data.
    tabs: Vec<TabEntry>,
    active_index: usize,

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

    // Window control buttons: [minimize, maximize/restore, close].
    controls: [WindowControlButton; 3],
    /// Index of the currently hovered control button (`None` if not hovering).
    hovered_control: Option<usize>,

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
        let layout = TabBarLayout::compute(0, window_width, None, 0.0);
        let caption_bg = theme.bg_secondary;
        let ctrl_colors = control_colors_from_theme(theme);
        let controls = create_controls(ctrl_colors, caption_bg);

        Self {
            id: WidgetId::next(),
            tabs: Vec::new(),
            active_index: 0,
            layout,
            colors: TabBarColors::from_theme(theme),
            window_width,
            tab_width_lock: None,
            hover_hit: TabBarHit::None,
            drag_visual: None,
            anim_offsets: Vec::new(),
            controls,
            hovered_control: None,
            left_inset: 0.0,
        }
    }

    // --- Theme ---

    /// Updates all theme-derived colors from a new [`UiTheme`].
    pub fn apply_theme(&mut self, theme: &UiTheme) {
        self.colors = TabBarColors::from_theme(theme);
        let ctrl_colors = control_colors_from_theme(theme);
        let caption_bg = theme.bg_secondary;
        for ctrl in &mut self.controls {
            ctrl.set_colors(ctrl_colors);
            ctrl.set_caption_bg(caption_bg);
        }
    }

    // --- State setters ---

    /// Updates the tab list and recomputes layout.
    pub fn set_tabs(&mut self, tabs: Vec<TabEntry>) {
        self.tabs = tabs;
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

    /// Updates which element the cursor is hovering.
    pub fn set_hover_hit(&mut self, hit: TabBarHit) {
        self.hover_hit = hit;
    }

    /// Sets the dragged tab visual state.
    ///
    /// `Some((index, x))` means tab `index` is being dragged and its visual
    /// position is at `x` logical pixels. `None` means no drag in progress.
    pub fn set_drag_visual(&mut self, drag: Option<(usize, f32)>) {
        self.drag_visual = drag;
    }

    // --- Accessors ---

    /// Current computed layout.
    pub fn layout(&self) -> &TabBarLayout {
        &self.layout
    }

    /// Number of tabs.
    pub fn tab_count(&self) -> usize {
        self.tabs.len()
    }

    /// Current hover hit state.
    pub fn hover_hit(&self) -> TabBarHit {
        self.hover_hit
    }

    /// Current tab width lock value, if active.
    pub fn tab_width_lock(&self) -> Option<f32> {
        self.tab_width_lock
    }

    /// Update the title of the tab at `index`.
    ///
    /// No-op if `index` is out of bounds.
    pub fn update_tab_title(&mut self, index: usize, title: String) {
        if let Some(entry) = self.tabs.get_mut(index) {
            entry.title = title;
        }
    }

    /// Start a bell animation on the tab at `index`.
    ///
    /// Records the current instant as the bell start time. No-op if
    /// `index` is out of bounds.
    pub fn ring_bell(&mut self, index: usize) {
        if let Some(entry) = self.tabs.get_mut(index) {
            entry.bell_start = Some(Instant::now());
        }
    }

    // --- Window control state ---

    /// Sets the maximized state on all control buttons.
    ///
    /// The maximize/restore button changes symbol (□ vs ⧉).
    pub fn set_maximized(&mut self, maximized: bool) {
        for ctrl in &mut self.controls {
            ctrl.set_maximized(maximized);
        }
    }

    /// Sets the active/focused state.
    ///
    /// Adjusts the caption background color on control buttons: active
    /// windows use `bar_bg`, inactive windows use a darkened variant.
    pub fn set_active(&mut self, active: bool) {
        let caption_bg = if active {
            self.colors.bar_bg
        } else {
            Color::lerp(self.colors.bar_bg, Color::BLACK, 0.3)
        };
        for ctrl in &mut self.controls {
            ctrl.set_caption_bg(caption_bg);
        }
    }

    /// Returns all interactive rects in logical pixels.
    ///
    /// Includes tab rects, new-tab button, dropdown button, and the three
    /// control buttons. The platform layer scales these to physical pixels
    /// and uses them for `WM_NCHITTEST` — points inside are `HTCLIENT`
    /// (clickable), everything else is `HTCAPTION` (draggable).
    pub fn interactive_rects(&self) -> Vec<Rect> {
        let mut rects = Vec::with_capacity(self.tabs.len() + 5);
        // Tab rects.
        for i in 0..self.tabs.len() {
            let x = self.layout.tab_x(i);
            rects.push(Rect::new(x, 0.0, self.layout.tab_width, TAB_BAR_HEIGHT));
        }
        // New-tab button.
        let ntx = self.layout.new_tab_x();
        rects.push(Rect::new(ntx, 0.0, NEW_TAB_BUTTON_WIDTH, TAB_BAR_HEIGHT));
        // Dropdown button.
        let ddx = self.layout.dropdown_x();
        rects.push(Rect::new(ddx, 0.0, DROPDOWN_BUTTON_WIDTH, TAB_BAR_HEIGHT));
        // Control buttons.
        for i in 0..3 {
            rects.push(self.control_rect(i));
        }
        rects
    }

    /// Updates hover state for control buttons based on cursor position.
    ///
    /// Routes `HoverEvent::Enter`/`Leave` to the appropriate
    /// [`WindowControlButton`] so animation transitions play correctly.
    pub fn update_control_hover(&mut self, pos: Point, ctx: &EventCtx<'_>) -> WidgetResponse {
        let new_idx = (0..3).find(|&i| self.control_rect(i).contains(pos));

        if new_idx == self.hovered_control {
            return WidgetResponse::ignored();
        }

        // Leave old control.
        let left = if let Some(old) = self.hovered_control {
            let child_ctx = EventCtx {
                measurer: ctx.measurer,
                bounds: self.control_rect(old),
                is_focused: false,
                focused_widget: ctx.focused_widget,
                theme: ctx.theme,
            };
            self.controls[old].handle_hover(HoverEvent::Leave, &child_ctx);
            true
        } else {
            false
        };

        // Enter new control.
        let entered = if let Some(new) = new_idx {
            let child_ctx = EventCtx {
                measurer: ctx.measurer,
                bounds: self.control_rect(new),
                is_focused: false,
                focused_widget: ctx.focused_widget,
                theme: ctx.theme,
            };
            self.controls[new].handle_hover(HoverEvent::Enter, &child_ctx);
            true
        } else {
            false
        };

        self.hovered_control = new_idx;

        if left || entered {
            WidgetResponse::redraw()
        } else {
            WidgetResponse::ignored()
        }
    }

    /// Clears control button hover state (e.g. when cursor leaves the tab bar).
    pub fn clear_control_hover(&mut self, ctx: &EventCtx<'_>) {
        if let Some(old) = self.hovered_control.take() {
            let child_ctx = EventCtx {
                measurer: ctx.measurer,
                bounds: self.control_rect(old),
                is_focused: false,
                focused_widget: ctx.focused_widget,
                theme: ctx.theme,
            };
            self.controls[old].handle_hover(HoverEvent::Leave, &child_ctx);
        }
    }

    /// Routes a mouse event to the appropriate control button.
    ///
    /// On button down: sets pressed state on the hovered control.
    /// On button up: releases the pressed control and emits the action.
    pub fn handle_control_mouse(
        &mut self,
        event: &MouseEvent,
        ctx: &EventCtx<'_>,
    ) -> WidgetResponse {
        match event.kind {
            MouseEventKind::Down(MouseButton::Left) => {
                let idx = (0..3).find(|&i| self.control_rect(i).contains(event.pos));
                if let Some(i) = idx {
                    let child_ctx = EventCtx {
                        measurer: ctx.measurer,
                        bounds: self.control_rect(i),
                        is_focused: false,
                        focused_widget: ctx.focused_widget,
                        theme: ctx.theme,
                    };
                    return self.controls[i].handle_mouse(event, &child_ctx);
                }
                WidgetResponse::ignored()
            }
            MouseEventKind::Up(MouseButton::Left) => {
                for i in 0..3 {
                    if self.controls[i].is_pressed() {
                        let child_ctx = EventCtx {
                            measurer: ctx.measurer,
                            bounds: self.control_rect(i),
                            is_focused: false,
                            focused_widget: ctx.focused_widget,
                            theme: ctx.theme,
                        };
                        return self.controls[i].handle_mouse(event, &child_ctx);
                    }
                }
                WidgetResponse::ignored()
            }
            _ => WidgetResponse::ignored(),
        }
    }

    // --- Private helpers ---

    /// Recomputes layout from current state.
    fn recompute_layout(&mut self) {
        self.layout = TabBarLayout::compute(
            self.tabs.len(),
            self.window_width,
            self.tab_width_lock,
            self.left_inset,
        );
    }

    /// Returns the animation offset for a tab, or 0.0 if none.
    fn anim_offset(&self, index: usize) -> f32 {
        self.anim_offsets.get(index).copied().unwrap_or(0.0)
    }

    /// Whether the given tab index is the one being dragged.
    fn is_dragged(&self, index: usize) -> bool {
        self.drag_visual.is_some_and(|(i, _)| i == index)
    }

    /// Swaps the internal animation offset buffer with an external one.
    ///
    /// Used by [`TabSlideState`](super::slide::TabSlideState) to populate
    /// per-tab offsets from compositor transforms without allocating. The
    /// caller fills `buf` with compositor-driven offsets, swaps in, and
    /// gets the old buffer back for reuse next frame.
    pub(crate) fn swap_anim_offsets(&mut self, buf: &mut Vec<f32>) {
        std::mem::swap(&mut self.anim_offsets, buf);
    }
}

// --- Free functions ---

/// Builds [`ControlButtonColors`] from a [`UiTheme`].
fn control_colors_from_theme(theme: &UiTheme) -> ControlButtonColors {
    ControlButtonColors {
        fg: theme.fg_primary,
        bg: Color::TRANSPARENT,
        hover_bg: theme.bg_hover,
        close_hover_bg: theme.close_hover_bg,
        close_pressed_bg: theme.close_pressed_bg,
    }
}

/// Creates the three control buttons with initial colors and caption bg.
fn create_controls(colors: ControlButtonColors, caption_bg: Color) -> [WindowControlButton; 3] {
    let mut min_btn = WindowControlButton::new(ControlKind::Minimize, colors);
    min_btn.set_caption_bg(caption_bg);
    let mut max_btn = WindowControlButton::new(ControlKind::MaximizeRestore, colors);
    max_btn.set_caption_bg(caption_bg);
    let mut close_btn = WindowControlButton::new(ControlKind::Close, colors);
    close_btn.set_caption_bg(caption_bg);
    [min_btn, max_btn, close_btn]
}

// --- Test helpers ---

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
}
