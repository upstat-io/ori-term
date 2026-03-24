//! Scroll container widget — clips content and supports scrolling.
//!
//! Wraps a single child widget that may be taller (or wider) than the
//! container's visible area. Provides mouse wheel scrolling, keyboard
//! navigation (PageUp/Down, Home/End), and an overlay scrollbar.

use std::cell::RefCell;
use std::rc::Rc;

use crate::geometry::Rect;
use crate::input::{InputEvent, Modifiers, MouseButton, ScrollDelta};
use crate::interaction::LifecycleEvent;
use crate::layout::{LayoutBox, LayoutNode, SizeSpec, compute_layout};
use crate::sense::Sense;
use crate::theme::UiTheme;
use crate::widget_id::WidgetId;

use super::scrollbar::ScrollbarVisualState;
use super::{DrawCtx, LayoutCtx, LifecycleCtx, OnInputResult, Widget, WidgetAction};

/// Re-export the shared scrollbar style.
pub use super::scrollbar::ScrollbarStyle;

mod input;
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

/// Per-axis scrollbar interaction state.
#[derive(Debug, Default)]
struct AxisBarState {
    dragging: bool,
    /// Pointer position along the axis at drag start.
    drag_start_pointer: f32,
    /// Scroll offset at drag start.
    drag_start_offset: f32,
    /// Cursor over the track/thumb area.
    track_hovered: bool,
    /// Cursor specifically over the thumb.
    thumb_hovered: bool,
}

impl AxisBarState {
    fn visual_state(&self) -> ScrollbarVisualState {
        if self.dragging {
            ScrollbarVisualState::Dragging
        } else if self.track_hovered || self.thumb_hovered {
            ScrollbarVisualState::Hovered
        } else {
            ScrollbarVisualState::Rest
        }
    }

    fn is_interacting(&self) -> bool {
        self.dragging
    }
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
    /// Current vertical scroll offset (pixels scrolled from top).
    scroll_offset: f32,
    /// Horizontal scroll offset (only used with Horizontal/Both direction).
    scroll_offset_x: f32,
    scrollbar_style: ScrollbarStyle,
    scrollbar_policy: ScrollbarPolicy,
    v_bar: AxisBarState,
    h_bar: AxisBarState,
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
        Self::new(child, ScrollDirection::Vertical)
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
            v_bar: AxisBarState::default(),
            h_bar: AxisBarState::default(),
            line_height: 20.0,
            height_override: None,
            cached_child_layout: RefCell::new(None),
        }
    }

    /// Overrides the height sizing spec for the scroll container's layout box.
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

    // Axis helpers

    /// Whether this direction has a vertical scrollbar.
    fn has_vertical(&self) -> bool {
        matches!(
            self.direction,
            ScrollDirection::Vertical | ScrollDirection::Both
        )
    }

    /// Whether this direction has a horizontal scrollbar.
    fn has_horizontal(&self) -> bool {
        matches!(
            self.direction,
            ScrollDirection::Horizontal | ScrollDirection::Both
        )
    }

    /// Space to reserve at the far end of each scrollbar track when both axes
    /// are active (avoids corner overlap).
    fn reserve_far_edge(&self, both_visible: bool) -> f32 {
        if both_visible {
            self.scrollbar_style.thickness + self.scrollbar_style.edge_inset
        } else {
            0.0
        }
    }

    // Content size helpers

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

    /// Scrolls vertical axis by a delta, clamping to valid range.
    fn scroll_by(&mut self, delta_y: f32, content_height: f32, view_height: f32) -> bool {
        let max = (content_height - view_height).max(0.0);
        let old = self.scroll_offset;
        self.scroll_offset = (self.scroll_offset + delta_y).clamp(0.0, max);
        (self.scroll_offset - old).abs() > f32::EPSILON
    }

    /// Scrolls horizontal axis by a delta, clamping to valid range.
    fn scroll_by_x(&mut self, delta_x: f32, content_width: f32, view_width: f32) -> bool {
        let max = (content_width - view_width).max(0.0);
        let old = self.scroll_offset_x;
        self.scroll_offset_x = (self.scroll_offset_x + delta_x).clamp(0.0, max);
        (self.scroll_offset_x - old).abs() > f32::EPSILON
    }

    /// Returns the cached child content height, falling back to viewport height.
    fn cached_content_height(&self, viewport: Rect) -> f32 {
        self.cached_child_layout
            .borrow()
            .as_ref()
            .map_or_else(|| viewport.height(), |(_, node)| node.rect.height())
    }

    /// Returns the cached child content width, falling back to viewport width.
    fn cached_content_width(&self, viewport: Rect) -> f32 {
        self.cached_child_layout
            .borrow()
            .as_ref()
            .map_or_else(|| viewport.width(), |(_, node)| node.rect.width())
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
        let child_box = self.child.layout(ctx);
        let mut lb = LayoutBox::flex(crate::layout::Direction::Column, vec![child_box])
            .with_widget_id(self.id)
            .with_clip(true)
            .with_overflow()
            .with_content_offset(-self.scroll_offset_x, -self.scroll_offset);

        match self.direction {
            ScrollDirection::Vertical => lb = lb.with_width(SizeSpec::Fill),
            ScrollDirection::Horizontal => lb = lb.with_height(SizeSpec::Fill),
            ScrollDirection::Both => {}
        }

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
        if let LifecycleEvent::HotChanged { is_hot: false, .. } = event {
            // Clear hover state. Do NOT clear an active drag — the drag
            // owns the pointer until MouseUp regardless of where the
            // cursor wanders (standard scrollbar behavior on all platforms).
            self.v_bar.track_hovered = false;
            self.v_bar.thumb_hovered = false;
            self.h_bar.track_hovered = false;
            self.h_bar.thumb_hovered = false;
        }
    }

    fn on_input(&mut self, event: &InputEvent, bounds: Rect) -> OnInputResult {
        let view_h = bounds.height();
        let view_w = bounds.width();
        let content_h = self.cached_content_height(bounds);
        let content_w = self.cached_content_width(bounds);

        match event {
            InputEvent::Scroll { delta, .. } => {
                let (dx, dy) = match delta {
                    ScrollDelta::Pixels { x, y } => (-*x, -*y),
                    ScrollDelta::Lines { x, y } => (-*x * self.line_height, -*y * self.line_height),
                };
                let mut scrolled = false;
                if self.has_vertical() {
                    scrolled |= self.scroll_by(dy, content_h, view_h);
                }
                if self.has_horizontal() {
                    scrolled |= self.scroll_by_x(dx, content_w, view_w);
                }
                if scrolled {
                    OnInputResult::handled()
                } else {
                    OnInputResult::ignored()
                }
            }
            InputEvent::KeyDown { key, modifiers } if *modifiers == Modifiers::NONE => {
                if self.handle_scroll_key(*key, content_h, view_h) {
                    OnInputResult::handled()
                } else {
                    OnInputResult::ignored()
                }
            }
            InputEvent::MouseDown {
                pos,
                button: MouseButton::Left,
                ..
            } => {
                let was_dragging = self.v_bar.dragging || self.h_bar.dragging;
                if self.handle_scrollbar_down(*pos, bounds, content_w, content_h) {
                    let now_dragging = self.v_bar.dragging || self.h_bar.dragging;
                    if now_dragging && !was_dragging {
                        // Drag just started — capture pointer so MouseMove
                        // and MouseUp arrive even outside the widget bounds.
                        OnInputResult::handled().with_capture()
                    } else {
                        OnInputResult::handled()
                    }
                } else {
                    OnInputResult::ignored()
                }
            }
            InputEvent::MouseMove { pos, .. } => {
                if self.handle_scrollbar_move(*pos, bounds, content_w, content_h) {
                    OnInputResult::handled()
                } else {
                    OnInputResult::ignored()
                }
            }
            InputEvent::MouseUp { .. }
                if self.v_bar.is_interacting() || self.h_bar.is_interacting() =>
            {
                self.v_bar.dragging = false;
                self.h_bar.dragging = false;
                // Release pointer capture.
                OnInputResult::handled().with_release()
            }
            _ => OnInputResult::ignored(),
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
        *self.cached_child_layout.borrow_mut() = None;
    }
}

#[cfg(test)]
mod tests;
