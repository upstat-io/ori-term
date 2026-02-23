//! Pure geometry computation for window chrome layout.
//!
//! [`ChromeLayout`] takes window dimensions and state, producing caption
//! height, button rectangles, title area, and interactive rects for hit
//! testing. No rendering, no side effects — fully testable in isolation.
//!
//! Follows Chrome's `OpaqueBrowserFrameViewLayout` pattern: a single pure
//! function from inputs → geometry outputs.

use crate::geometry::Rect;

use super::constants::{
    CAPTION_HEIGHT, CAPTION_HEIGHT_MAXIMIZED, CONTROL_BUTTON_WIDTH, RESIZE_BORDER_WIDTH,
};

/// The three window control button positions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControlKind {
    /// Minimize (─) button.
    Minimize,
    /// Maximize (□) or Restore (⧉) button.
    MaximizeRestore,
    /// Close (×) button.
    Close,
}

/// A positioned control button in the chrome layout.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ControlRect {
    /// Which control this is.
    pub kind: ControlKind,
    /// Bounding rectangle in logical pixels.
    pub rect: Rect,
}

/// Computed window chrome geometry.
///
/// Produced by [`ChromeLayout::compute`]. All coordinates are in logical
/// pixels relative to the window's top-left corner.
#[derive(Debug, Clone, PartialEq)]
pub struct ChromeLayout {
    /// Caption bar height in logical pixels.
    pub caption_height: f32,
    /// Title text area (left of the control buttons).
    pub title_rect: Rect,
    /// The three control button rects, ordered: minimize, maximize, close.
    pub controls: [ControlRect; 3],
    /// All interactive rects within the caption (for hit test exclusion).
    ///
    /// Points inside these rects are `Client`, not `Caption`, so clicks
    /// reach the buttons instead of initiating a window drag.
    pub interactive_rects: Vec<Rect>,
    /// Whether chrome is visible (false in fullscreen).
    pub visible: bool,
}

impl ChromeLayout {
    /// Compute chrome layout from current window state.
    ///
    /// Returns a layout with `visible = false` when fullscreen (chrome
    /// hidden). The caller should skip drawing and use 0 caption offset.
    pub fn compute(window_width: f32, is_maximized: bool, is_fullscreen: bool) -> Self {
        if is_fullscreen {
            return Self::hidden();
        }

        let caption_h = if is_maximized {
            CAPTION_HEIGHT_MAXIMIZED
        } else {
            CAPTION_HEIGHT
        };

        // Control buttons are right-aligned: [minimize] [maximize] [close].
        // Each button spans the full caption height.
        let btn_h = caption_h;
        let btn_w = CONTROL_BUTTON_WIDTH;

        // Buttons sit flush with caption top. Resize border overlap is
        // handled by the hit test layer, not by vertical button offset.
        let btn_y = 0.0;

        let close_x = window_width - btn_w;
        let maximize_x = close_x - btn_w;
        let minimize_x = maximize_x - btn_w;

        let controls = [
            ControlRect {
                kind: ControlKind::Minimize,
                rect: Rect::new(minimize_x, btn_y, btn_w, btn_h),
            },
            ControlRect {
                kind: ControlKind::MaximizeRestore,
                rect: Rect::new(maximize_x, btn_y, btn_w, btn_h),
            },
            ControlRect {
                kind: ControlKind::Close,
                rect: Rect::new(close_x, btn_y, btn_w, btn_h),
            },
        ];

        // Title area: left edge to the first button, full caption height.
        let title_x = if is_maximized {
            0.0
        } else {
            RESIZE_BORDER_WIDTH
        };
        let title_width = (minimize_x - title_x).max(0.0);
        let title_rect = Rect::new(title_x, btn_y, title_width, caption_h);

        let interactive_rects = controls.iter().map(|c| c.rect).collect();

        Self {
            caption_height: caption_h,
            title_rect,
            controls,
            interactive_rects,
            visible: true,
        }
    }

    /// Returns a hidden layout (fullscreen mode).
    fn hidden() -> Self {
        Self {
            caption_height: 0.0,
            title_rect: Rect::default(),
            controls: [
                ControlRect {
                    kind: ControlKind::Minimize,
                    rect: Rect::default(),
                },
                ControlRect {
                    kind: ControlKind::MaximizeRestore,
                    rect: Rect::default(),
                },
                ControlRect {
                    kind: ControlKind::Close,
                    rect: Rect::default(),
                },
            ],
            interactive_rects: Vec::new(),
            visible: false,
        }
    }
}
