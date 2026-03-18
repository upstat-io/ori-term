//! Scroll container widget — clips content and supports scrolling.
//!
//! Wraps a single child widget that may be taller (or wider) than the
//! container's visible area. Provides mouse wheel scrolling, keyboard
//! navigation (PageUp/Down, Home/End), and an overlay scrollbar.

use std::cell::RefCell;
use std::rc::Rc;

use crate::color::Color;
use crate::geometry::{Point, Rect};
use crate::input::{InputEvent, Key, Modifiers, MouseButton, ScrollDelta};
use crate::interaction::LifecycleEvent;
use crate::layout::{LayoutBox, LayoutNode, SizeSpec, compute_layout};
use crate::sense::Sense;
use crate::widget_id::WidgetId;

use crate::theme::UiTheme;

use super::{DrawCtx, LayoutCtx, LifecycleCtx, OnInputResult, Widget, WidgetAction};

mod rendering;
mod scrollbar;

/// Scroll direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScrollDirection {
    /// Vertical scrolling only (most common).
    Vertical,
    /// Horizontal scrolling only.
    Horizontal,
    /// Both axes scroll independently.
    Both,
}

/// Style for the overlay scrollbar.
#[derive(Debug, Clone, PartialEq)]
pub struct ScrollbarStyle {
    /// Scrollbar width (logical pixels).
    pub width: f32,
    /// Scrollbar thumb color.
    pub thumb_color: Color,
    /// Scrollbar track color (behind the thumb).
    pub track_color: Color,
    /// Corner radius of the thumb.
    pub thumb_radius: f32,
    /// Minimum thumb height (logical pixels).
    pub min_thumb_height: f32,
}

impl Default for ScrollbarStyle {
    fn default() -> Self {
        Self {
            width: 6.0,
            thumb_color: Color::WHITE.with_alpha(0.25),
            track_color: Color::TRANSPARENT,
            thumb_radius: 3.0,
            min_thumb_height: 20.0,
        }
    }
}

/// When to show the scrollbar.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ScrollbarPolicy {
    /// Show scrollbar only when content overflows (default).
    #[default]
    Auto,
    /// Always show scrollbar.
    Always,
    /// Never show scrollbar (content still scrollable via wheel).
    Hidden,
}

/// Tracks scrollbar drag and hover state.
#[derive(Debug, Default)]
struct ScrollbarState {
    /// Whether the scrollbar thumb is being dragged.
    dragging: bool,
    /// Y offset of drag start.
    drag_start_y: f32,
    /// Scroll offset at drag start.
    drag_start_offset: f32,
    /// Whether the cursor is over the scrollbar track area.
    track_hovered: bool,
}

/// A scrollable container that clips its child to visible bounds.
///
/// Supports vertical, horizontal, or dual-axis scrolling. Renders a
/// thin overlay scrollbar when content overflows. Supports scrollbar
/// thumb drag interaction.
pub struct ScrollWidget {
    id: WidgetId,
    child: Box<dyn Widget>,
    direction: ScrollDirection,
    /// Current scroll offset (pixels scrolled from top/left).
    scroll_offset: f32,
    /// Horizontal scroll offset (only used with `Both` direction).
    scroll_offset_x: f32,
    scrollbar_style: ScrollbarStyle,
    scrollbar_policy: ScrollbarPolicy,
    scrollbar: ScrollbarState,
    /// Pixels per mouse wheel line.
    line_height: f32,
    /// Optional height override for the layout box.
    ///
    /// When set to `Fill`, the scroll container expands to fill the
    /// remaining space in a column — creating a sticky footer effect
    /// where siblings below it stay pinned at the bottom.
    ///
    /// **Constraint:** `Fill` should only be used when the scroll is in a
    /// column with known remaining height. Internally, scroll still caches
    /// the child's natural height for content-space calculations.
    height_override: Option<SizeSpec>,
    /// Cached child natural size, keyed by viewport bounds.
    cached_child_layout: RefCell<Option<(Rect, Rc<LayoutNode>)>>,
}

impl ScrollWidget {
    /// Creates a vertical scroll container wrapping the given child.
    pub fn vertical(child: Box<dyn Widget>) -> Self {
        Self {
            id: WidgetId::next(),
            child,
            direction: ScrollDirection::Vertical,
            scroll_offset: 0.0,
            scroll_offset_x: 0.0,
            scrollbar_style: ScrollbarStyle::default(),
            scrollbar_policy: ScrollbarPolicy::default(),
            scrollbar: ScrollbarState::default(),
            line_height: 20.0,
            height_override: None,
            cached_child_layout: RefCell::new(None),
        }
    }

    /// Creates a scroll container with a specific direction.
    pub fn new(child: Box<dyn Widget>, direction: ScrollDirection) -> Self {
        Self {
            id: WidgetId::next(),
            child,
            direction,
            scroll_offset: 0.0,
            scroll_offset_x: 0.0,
            scrollbar_style: ScrollbarStyle::default(),
            scrollbar_policy: ScrollbarPolicy::default(),
            scrollbar: ScrollbarState::default(),
            line_height: 20.0,
            height_override: None,
            cached_child_layout: RefCell::new(None),
        }
    }

    /// Overrides the height sizing spec for the scroll container's layout box.
    ///
    /// By default, the scroll widget reports its child's natural height. Set
    /// to `SizeSpec::Fill` to make it fill remaining space in a column layout,
    /// creating a sticky footer effect where siblings below stay pinned.
    pub fn set_height(&mut self, spec: SizeSpec) {
        self.height_override = Some(spec);
    }

    /// Sets the scrollbar style.
    #[must_use]
    pub fn with_scrollbar_style(mut self, style: ScrollbarStyle) -> Self {
        self.scrollbar_style = style;
        self
    }

    /// Sets the scrollbar visibility policy.
    #[must_use]
    pub fn with_scrollbar_policy(mut self, policy: ScrollbarPolicy) -> Self {
        self.scrollbar_policy = policy;
        self
    }

    /// Returns the current vertical scroll offset.
    pub fn scroll_offset(&self) -> f32 {
        self.scroll_offset
    }

    /// Sets the vertical scroll offset, clamping to valid range.
    pub fn set_scroll_offset(&mut self, offset: f32, content_height: f32, view_height: f32) {
        let max = (content_height - view_height).max(0.0);
        self.scroll_offset = offset.clamp(0.0, max);
    }

    /// Returns cached child natural size if viewport matches, otherwise recomputes.
    fn child_natural_size(
        &self,
        measurer: &dyn super::TextMeasurer,
        theme: &UiTheme,
        viewport: Rect,
    ) -> (f32, f32) {
        {
            let cached = self.cached_child_layout.borrow();
            if let Some((ref cv, ref node)) = *cached {
                if *cv == viewport {
                    return (node.rect.width(), node.rect.height());
                }
            }
        }
        let ctx = LayoutCtx { measurer, theme };
        let child_box = self.child.layout(&ctx);
        let (w, h) = match self.direction {
            ScrollDirection::Vertical => (viewport.width(), f32::INFINITY),
            ScrollDirection::Horizontal => (f32::INFINITY, viewport.height()),
            ScrollDirection::Both => (f32::INFINITY, f32::INFINITY),
        };
        let unconstrained = Rect::new(0.0, 0.0, w, h);
        let node = compute_layout(&child_box, unconstrained);
        let size = (node.rect.width(), node.rect.height());
        *self.cached_child_layout.borrow_mut() = Some((viewport, Rc::new(node)));
        size
    }

    /// Scrolls by a delta, clamping to valid range. Returns true if offset changed.
    ///
    /// Positive `delta_y` scrolls down (increases offset, reveals content
    /// below). The mouse event handler negates the raw wheel delta so
    /// wheel-up (positive y from winit) produces a negative `delta_y` here,
    /// decreasing the offset — scrolling the view up.
    fn scroll_by(&mut self, delta_y: f32, content_height: f32, view_height: f32) -> bool {
        let max = (content_height - view_height).max(0.0);
        let old = self.scroll_offset;
        self.scroll_offset = (self.scroll_offset + delta_y).clamp(0.0, max);
        (self.scroll_offset - old).abs() > f32::EPSILON
    }

    /// Returns the cached child content height, falling back to viewport height.
    fn cached_content_height(&self, viewport: Rect) -> f32 {
        self.cached_child_layout
            .borrow()
            .as_ref()
            .map_or_else(|| viewport.height(), |(_, node)| node.rect.height())
    }

    /// Handles scroll-related key presses (arrows, PageUp/Down, Home/End).
    fn handle_scroll_key(&mut self, key: Key, content_h: f32, view_h: f32) -> bool {
        match key {
            Key::ArrowUp => {
                self.scroll_by(-self.line_height, content_h, view_h);
                true
            }
            Key::ArrowDown => {
                self.scroll_by(self.line_height, content_h, view_h);
                true
            }
            Key::PageUp => {
                self.scroll_by(-view_h, content_h, view_h);
                true
            }
            Key::PageDown => {
                self.scroll_by(view_h, content_h, view_h);
                true
            }
            Key::Home => {
                self.scroll_offset = 0.0;
                true
            }
            Key::End => {
                self.scroll_offset = (content_h - view_h).max(0.0);
                true
            }
            _ => false,
        }
    }

    /// Handles mouse-down on the scrollbar (thumb drag start or track click).
    fn handle_scrollbar_down(
        &mut self,
        pos: Point,
        viewport: Rect,
        content_h: f32,
        view_h: f32,
    ) -> bool {
        if self.scrollbar_policy == ScrollbarPolicy::Hidden {
            return false;
        }
        let thumb = self.scrollbar_thumb_rect(viewport, content_h, view_h);
        if thumb.contains(pos) {
            self.scrollbar.dragging = true;
            self.scrollbar.drag_start_y = pos.y;
            self.scrollbar.drag_start_offset = self.scroll_offset;
            return true;
        }
        let track = self.scrollbar_track_rect(viewport);
        if track.contains(pos) {
            let ratio = (pos.y - track.y()) / track.height();
            let max = (content_h - view_h).max(0.0);
            self.scroll_offset = (ratio * max).clamp(0.0, max);
            return true;
        }
        false
    }

    /// Handles mouse-move for scrollbar drag and track hover detection.
    fn handle_scrollbar_move(
        &mut self,
        pos: Point,
        viewport: Rect,
        content_h: f32,
        view_h: f32,
    ) -> bool {
        if self.scrollbar.dragging {
            let track = self.scrollbar_track_rect(viewport);
            let delta_y = pos.y - self.scrollbar.drag_start_y;
            let track_h = track.height();
            let max = (content_h - view_h).max(0.0);
            let scroll_delta = if track_h > 0.0 {
                delta_y * max / track_h
            } else {
                0.0
            };
            self.scroll_offset = (self.scrollbar.drag_start_offset + scroll_delta).clamp(0.0, max);
            return true;
        }
        // Track hover detection.
        let track = self.scrollbar_track_rect(viewport);
        let was_hovered = self.scrollbar.track_hovered;
        self.scrollbar.track_hovered = track.contains(pos);
        was_hovered != self.scrollbar.track_hovered
    }
}

impl Widget for ScrollWidget {
    fn id(&self) -> WidgetId {
        self.id
    }

    fn is_focusable(&self) -> bool {
        true
    }

    fn layout(&self, ctx: &LayoutCtx<'_>) -> LayoutBox {
        // Include the child's layout so hit testing can reach widgets inside
        // the scroll area. The child's natural size determines the scroll
        // extent, and clip=true ensures hit testing is clipped to the viewport.
        // content_offset matches the translate applied during rendering.
        let child_box = self.child.layout(ctx);
        let mut lb = LayoutBox::flex(crate::layout::Direction::Column, vec![child_box])
            .with_widget_id(self.id)
            .with_clip(true)
            .with_content_offset(-self.scroll_offset_x, -self.scroll_offset);

        // For vertical scrolling, use Fill width so the scroll container
        // expands to the parent's available width. The child's natural width
        // in infinite space may be narrow when it contains Fill-width children
        // (Fill resolves to 0 in unbounded contexts). Only height scrolls.
        // Symmetrically, horizontal scroll uses Fill height.
        match self.direction {
            ScrollDirection::Vertical => lb = lb.with_width(SizeSpec::Fill),
            ScrollDirection::Horizontal => lb = lb.with_height(SizeSpec::Fill),
            ScrollDirection::Both => {}
        }

        // Apply height override (e.g. Fill for sticky footer layouts).
        if let Some(h_spec) = self.height_override {
            lb = lb.with_height(h_spec);
        }
        lb
    }

    fn sense(&self) -> Sense {
        Sense::hover()
    }

    fn paint(&self, ctx: &mut DrawCtx<'_>) {
        self.draw_impl(ctx);
    }

    fn for_each_child_mut(&mut self, visitor: &mut dyn FnMut(&mut dyn Widget)) {
        visitor(self.child.as_mut());
    }

    fn lifecycle(&mut self, event: &LifecycleEvent, _ctx: &mut LifecycleCtx<'_>) {
        // Reset scrollbar hover when cursor leaves the scroll container.
        if let LifecycleEvent::HotChanged { is_hot: false, .. } = event {
            self.scrollbar.track_hovered = false;
        }
    }

    fn on_input(&mut self, event: &InputEvent, bounds: Rect) -> OnInputResult {
        let view_h = bounds.height();
        let content_h = self.cached_content_height(bounds);

        let handled = match event {
            InputEvent::Scroll { delta, .. } => {
                let delta_y = match delta {
                    ScrollDelta::Pixels { y, .. } => -*y,
                    ScrollDelta::Lines { y, .. } => -*y * self.line_height,
                };
                self.scroll_by(delta_y, content_h, view_h);
                true
            }
            InputEvent::KeyDown { key, modifiers } if *modifiers == Modifiers::NONE => {
                self.handle_scroll_key(*key, content_h, view_h)
            }
            InputEvent::MouseDown {
                pos,
                button: MouseButton::Left,
                ..
            } => self.handle_scrollbar_down(*pos, bounds, content_h, view_h),
            InputEvent::MouseMove { pos, .. } => {
                self.handle_scrollbar_move(*pos, bounds, content_h, view_h)
            }
            InputEvent::MouseUp { .. } if self.scrollbar.dragging => {
                self.scrollbar.dragging = false;
                true
            }
            _ => false,
        };
        if handled {
            OnInputResult::handled()
        } else {
            OnInputResult::ignored()
        }
    }

    fn accept_action(&mut self, action: &WidgetAction) -> bool {
        self.child.accept_action(action)
    }

    fn focusable_children(&self) -> Vec<WidgetId> {
        let mut ids = Vec::new();
        if self.is_focusable() {
            ids.push(self.id());
        }
        ids.extend(self.child.focusable_children());
        ids
    }

    fn reset_scroll(&mut self) {
        self.scroll_offset = 0.0;
        self.scroll_offset_x = 0.0;
    }
}

#[cfg(test)]
mod tests;
