//! Window chrome widget — title bar with minimize/maximize/close controls.
//!
//! [`WindowChromeWidget`] composes a title label area with three
//! [`WindowControlButton`]s in a horizontal row. It draws the caption
//! background, manages active/inactive state, and exposes
//! [`interactive_rects`](WindowChromeWidget::interactive_rects) for
//! platform hit testing.

pub mod constants;
pub mod controls;
pub mod dispatch;
pub mod layout;

use crate::animation::Lerp;
use crate::color::Color;
use crate::draw::RectStyle;
use crate::geometry::{Point, Rect};
use crate::layout::LayoutBox;
use crate::sense::Sense;
use crate::theme::UiTheme;
use crate::widget_id::WidgetId;

use self::controls::{ControlButtonColors, WindowControlButton};
use self::layout::{ChromeLayout, ChromeMode, ControlKind};
use super::{DrawCtx, LayoutCtx, Widget};

/// Window chrome widget: caption bar with title and window controls.
///
/// Draws a colored bar at the top of the window containing a title string
/// and control buttons. In [`ChromeMode::Full`] mode: minimize,
/// maximize/restore, and close. In [`ChromeMode::Dialog`] mode: close only.
/// The caption background is draggable (the platform layer uses
/// [`interactive_rects`] to exclude button areas from the drag zone).
pub struct WindowChromeWidget {
    id: WidgetId,
    title: String,
    /// Whether the window is currently active (focused).
    active: bool,
    /// Whether the window is currently maximized.
    is_maximized: bool,
    /// Whether the window is currently fullscreen.
    is_fullscreen: bool,
    /// Which controls to show (full vs dialog).
    mode: ChromeMode,
    /// Window width in logical pixels (updated on resize).
    window_width: f32,
    /// Cached layout (recomputed on state/size change).
    chrome_layout: ChromeLayout,
    /// Control buttons. Length depends on mode: 3 for Full, 1 for Dialog.
    controls: Vec<WindowControlButton>,
    /// Index of the currently pressed control button (for routing mouse-up
    /// in [`dispatch_input`](Self::dispatch_input)).
    pressed_control: Option<usize>,
    /// Caption background color (active).
    caption_bg: Color,
    /// Caption background color (inactive / unfocused).
    caption_bg_inactive: Color,
    /// Caption foreground (title text) color.
    caption_fg: Color,
    /// Left inset for the title area (reserves space for native controls).
    ///
    /// On macOS, this accounts for the traffic light buttons. On other
    /// platforms, this is 0.
    title_left_inset: f32,
}

impl WindowChromeWidget {
    /// Creates a new window chrome widget with default dark theme colors.
    pub fn new(title: impl Into<String>, window_width: f32) -> Self {
        Self::with_theme(title, window_width, &UiTheme::dark())
    }

    /// Creates a new window chrome widget with colors from the given theme.
    pub fn with_theme(title: impl Into<String>, window_width: f32, theme: &UiTheme) -> Self {
        Self::with_theme_and_mode(title, window_width, theme, ChromeMode::Full)
    }

    /// Creates a dialog chrome widget (close button only).
    pub fn dialog(title: impl Into<String>, window_width: f32, theme: &UiTheme) -> Self {
        Self::with_theme_and_mode(title, window_width, theme, ChromeMode::Dialog)
    }

    /// Creates a window chrome widget with the given mode and theme.
    fn with_theme_and_mode(
        title: impl Into<String>,
        window_width: f32,
        theme: &UiTheme,
        mode: ChromeMode,
    ) -> Self {
        let chrome_layout =
            ChromeLayout::compute_with_inset(window_width, false, false, mode, 0.0);

        let caption_bg = theme.bg_secondary;

        let colors = ControlButtonColors {
            fg: theme.fg_primary,
            bg: Color::TRANSPARENT,
            hover_bg: theme.bg_hover,
            close_hover_bg: theme.close_hover_bg,
            close_pressed_bg: theme.close_pressed_bg,
        };

        let controls = match mode {
            ChromeMode::Full => vec![
                WindowControlButton::new(ControlKind::Minimize, colors),
                WindowControlButton::new(ControlKind::MaximizeRestore, colors),
                WindowControlButton::new(ControlKind::Close, colors),
            ],
            // macOS dialogs use the native traffic light for close — no custom controls.
            #[cfg(target_os = "macos")]
            ChromeMode::Dialog => Vec::new(),
            #[cfg(not(target_os = "macos"))]
            ChromeMode::Dialog => vec![WindowControlButton::new(ControlKind::Close, colors)],
        };

        Self {
            id: WidgetId::next(),
            title: title.into(),
            active: true,
            is_maximized: false,
            is_fullscreen: false,
            mode,
            window_width,
            chrome_layout,
            controls,
            pressed_control: None,
            caption_bg,
            caption_bg_inactive: darken(caption_bg, 0.3),
            caption_fg: theme.fg_secondary,
            title_left_inset: 0.0,
        }
    }

    // Accessors

    /// Returns the caption height in logical pixels (0 if fullscreen).
    pub fn caption_height(&self) -> f32 {
        self.chrome_layout.caption_height
    }

    /// Returns the interactive rects for hit test exclusion.
    ///
    /// These are the button rects within the caption area. Points inside
    /// these rects should be treated as `Client` hits (clickable), not
    /// `Caption` (draggable).
    pub fn interactive_rects(&self) -> &[Rect] {
        &self.chrome_layout.interactive_rects
    }

    /// Whether the chrome is visible (false in fullscreen).
    pub fn is_visible(&self) -> bool {
        self.chrome_layout.visible
    }

    // State updates

    /// Sets the window title.
    pub fn set_title(&mut self, title: String) {
        self.title = title;
    }

    /// Sets the active/focused state.
    pub fn set_active(&mut self, active: bool) {
        self.active = active;
    }

    /// Sets the maximized state and recomputes layout.
    pub fn set_maximized(&mut self, maximized: bool) {
        self.is_maximized = maximized;
        for ctrl in &mut self.controls {
            ctrl.set_maximized(maximized);
        }
        self.recompute_layout();
    }

    /// Sets the fullscreen state and recomputes layout.
    pub fn set_fullscreen(&mut self, fullscreen: bool) {
        self.is_fullscreen = fullscreen;
        self.recompute_layout();
    }

    /// Updates the window width and recomputes layout.
    pub fn set_window_width(&mut self, width: f32) {
        self.window_width = width;
        self.recompute_layout();
    }

    /// Sets a left inset for the title area to reserve space for native
    /// window controls (e.g. macOS traffic lights).
    pub fn set_title_left_inset(&mut self, inset: f32) {
        self.title_left_inset = inset;
        self.recompute_layout();
    }

    /// Updates all theme-derived colors from a new [`UiTheme`].
    pub fn apply_theme(&mut self, theme: &UiTheme) {
        self.caption_bg = theme.bg_secondary;
        self.caption_bg_inactive = darken(theme.bg_secondary, 0.3);
        self.caption_fg = theme.fg_secondary;
        let colors = ControlButtonColors {
            fg: theme.fg_primary,
            bg: Color::TRANSPARENT,
            hover_bg: theme.bg_hover,
            close_hover_bg: theme.close_hover_bg,
            close_pressed_bg: theme.close_pressed_bg,
        };
        for ctrl in &mut self.controls {
            ctrl.set_colors(colors);
        }
    }

    /// Recomputes the chrome layout from current state.
    fn recompute_layout(&mut self) {
        self.chrome_layout = ChromeLayout::compute_with_inset(
            self.window_width,
            self.is_maximized,
            self.is_fullscreen,
            self.mode,
            self.title_left_inset,
        );
    }

    /// Returns the current caption background color based on active state.
    fn current_caption_bg(&self) -> Color {
        if self.active {
            self.caption_bg
        } else {
            self.caption_bg_inactive
        }
    }

    /// Finds which control button (if any) contains the given point.
    fn control_at_point(&self, point: Point) -> Option<usize> {
        self.chrome_layout
            .controls
            .iter()
            .position(|c| c.rect.contains(point))
    }
}

impl Widget for WindowChromeWidget {
    fn id(&self) -> WidgetId {
        self.id
    }

    fn is_focusable(&self) -> bool {
        false
    }

    fn sense(&self) -> Sense {
        Sense::none()
    }

    fn layout(&self, _ctx: &LayoutCtx<'_>) -> LayoutBox {
        LayoutBox::leaf(self.window_width, self.chrome_layout.caption_height)
            .with_widget_id(self.id)
    }

    fn paint(&self, ctx: &mut DrawCtx<'_>) {
        if !self.chrome_layout.visible {
            return;
        }

        // Layer captures the caption bg for subpixel title text compositing.
        let bg = self.current_caption_bg();
        ctx.scene.push_layer_bg(bg);

        // Caption background bar.
        let caption_rect = Rect::new(0.0, 0.0, ctx.bounds.width(), self.caption_height());
        ctx.scene.push_quad(caption_rect, RectStyle::filled(bg));

        // Title text (centered vertically in the title area).
        if !self.title.is_empty() {
            let title_rect = self.chrome_layout.title_rect;
            let style = crate::text::TextStyle::new(ctx.theme.font_size_small, self.caption_fg);
            let shaped = ctx.measurer.shape(&self.title, &style, title_rect.width());
            let x = title_rect.x() + 8.0;
            let y = title_rect.y() + (title_rect.height() - shaped.height) / 2.0;
            ctx.scene
                .push_text(Point::new(x, y), shaped, self.caption_fg);
        }

        ctx.scene.pop_layer_bg();

        // Control buttons (outside the caption layer — each button has its own bg).
        for (i, ctrl) in self.controls.iter().enumerate() {
            let ctrl_rect = self.chrome_layout.controls[i].rect;
            let mut child_ctx = DrawCtx {
                measurer: ctx.measurer,
                scene: ctx.scene,
                bounds: ctrl_rect,
                now: ctx.now,
                theme: ctx.theme,
                icons: ctx.icons,
                interaction: None,
                widget_id: None,
                frame_requests: ctx.frame_requests,
            };
            ctrl.paint(&mut child_ctx);
        }
    }

    fn for_each_child_mut(&mut self, visitor: &mut dyn FnMut(&mut dyn Widget)) {
        for ctrl in &mut self.controls {
            visitor(ctrl);
        }
    }
}

// Test helpers

#[cfg(test)]
impl WindowChromeWidget {
    /// Test-only access to the active caption background.
    pub fn test_caption_bg(&self) -> Color {
        self.caption_bg
    }

    /// Test-only access to the inactive caption background.
    pub fn test_caption_bg_inactive(&self) -> Color {
        self.caption_bg_inactive
    }

    /// Test-only access to the caption foreground (title text).
    pub fn test_caption_fg(&self) -> Color {
        self.caption_fg
    }

    /// Test-only access to the maximized flag.
    pub fn test_is_maximized(&self) -> bool {
        self.is_maximized
    }
}

/// Darken a color by blending toward black.
fn darken(color: Color, amount: f32) -> Color {
    Color::lerp(color, Color::BLACK, amount)
}

#[cfg(test)]
mod tests;
