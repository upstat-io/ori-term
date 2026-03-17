//! Generic container widget that holds dynamic children.
//!
//! `ContainerWidget` owns `Vec<Box<dyn Widget>>` and delegates layout,
//! drawing, and event handling to its children. It is the composition
//! primitive that enables building complex UIs from standard controls.
//!
//! Replaces `FlexWidget` with additional capabilities: mouse capture
//! semantics, post-construction child management, padding, and explicit
//! sizing via `SizeSpec`.

use std::cell::RefCell;
use std::rc::Rc;

use crate::geometry::{Insets, Rect};
use crate::input::{EventResponse, HoverEvent, KeyEvent, MouseEvent};
use crate::invalidation::{DirtyKind, InvalidationTracker};
use crate::layout::{Align, Direction, GridColumns, Justify, LayoutBox, LayoutNode, SizeSpec};
use crate::sense::Sense;
use crate::widget_id::WidgetId;

use super::{DrawCtx, EventCtx, LayoutCtx, TextMeasurer, Widget, WidgetAction, WidgetResponse};

mod event_dispatch;
mod layout_build;

/// Layout mode for a container — flex or grid.
#[derive(Debug, Clone)]
enum LayoutMode {
    /// Flex layout along a direction with alignment and justification.
    Flex {
        direction: Direction,
        align: Align,
        justify: Justify,
        gap: f32,
    },
    /// Grid layout with column specification and gaps.
    Grid {
        columns: GridColumns,
        row_gap: f32,
        column_gap: f32,
    },
}

impl LayoutMode {
    /// Builds a `LayoutBox` from child boxes using this layout mode.
    fn build(&self, children: Vec<LayoutBox>) -> LayoutBox {
        match self {
            Self::Flex {
                direction,
                align,
                justify,
                gap,
            } => LayoutBox::flex(*direction, children)
                .with_gap(*gap)
                .with_align(*align)
                .with_justify(*justify),
            Self::Grid {
                columns,
                row_gap,
                column_gap,
            } => LayoutBox::grid(*columns, children)
                .with_row_gap(*row_gap)
                .with_column_gap(*column_gap),
        }
    }
}

/// A widget that composes other widgets in a flex or grid layout.
///
/// Arranges children along a main axis (horizontal for Row, vertical for
/// Column) with configurable gap, alignment, justification, padding, and
/// explicit sizing. Alternatively arranges children in a grid with
/// configurable column count and gaps. Supports mouse capture so drag
/// events stay on the pressed child.
pub struct ContainerWidget {
    id: WidgetId,
    children: Vec<Box<dyn Widget>>,

    // Layout configuration.
    layout_mode: LayoutMode,
    padding: Insets,
    width: SizeSpec,
    height: SizeSpec,

    // Whether to clip children to the container's bounds.
    clip_children: bool,

    // Per-container input state (hover/capture tracking among children).
    input_state: ContainerInputState,

    /// Whether this container's layout needs recomputation.
    needs_layout: bool,
    /// Whether this container's paint is dirty.
    needs_paint: bool,

    /// Computed layout cache. Uses `RefCell` because `draw(&self)` and
    /// `layout(&self)` take shared references but need to update the cache.
    cached_layout: RefCell<Option<(Rect, Rc<LayoutNode>)>>,
}

/// Tracks mouse interaction state within a container's children.
#[derive(Debug, Default)]
struct ContainerInputState {
    /// Child currently under the cursor (receives hover events).
    hovered_child: Option<usize>,
    /// Child that captured the mouse (receives all events until release).
    captured_child: Option<usize>,
}

// Constructors and child management.
impl ContainerWidget {
    /// Creates an empty container with the given layout direction.
    pub fn new(direction: Direction) -> Self {
        Self {
            id: WidgetId::next(),
            children: Vec::new(),
            layout_mode: LayoutMode::Flex {
                direction,
                align: Align::Start,
                justify: Justify::Start,
                gap: 0.0,
            },
            padding: Insets::ZERO,
            width: SizeSpec::Hug,
            height: SizeSpec::Hug,
            clip_children: false,
            input_state: ContainerInputState::default(),
            needs_layout: true,
            needs_paint: true,
            cached_layout: RefCell::new(None),
        }
    }

    /// Creates a grid container with the given column spec and gap.
    pub fn grid(columns: GridColumns, gap: f32) -> Self {
        Self {
            id: WidgetId::next(),
            children: Vec::new(),
            layout_mode: LayoutMode::Grid {
                columns,
                row_gap: gap,
                column_gap: gap,
            },
            padding: Insets::ZERO,
            width: SizeSpec::Hug,
            height: SizeSpec::Hug,
            clip_children: false,
            input_state: ContainerInputState::default(),
            needs_layout: true,
            needs_paint: true,
            cached_layout: RefCell::new(None),
        }
    }

    /// Creates a horizontal (Row) container.
    pub fn row() -> Self {
        Self::new(Direction::Row)
    }

    /// Creates a vertical (Column) container.
    pub fn column() -> Self {
        Self::new(Direction::Column)
    }

    /// Vertical stack with standard spacing.
    pub fn vstack(gap: f32) -> Self {
        Self::column().with_gap(gap)
    }

    /// Horizontal stack with standard spacing.
    pub fn hstack(gap: f32) -> Self {
        Self::row().with_gap(gap)
    }

    /// Centered container that centers its single child.
    pub fn centered() -> Self {
        Self::column()
            .with_align(Align::Center)
            .with_justify(Justify::Center)
    }

    /// Adds a child widget, transferring ownership.
    pub fn add_child(&mut self, child: Box<dyn Widget>) {
        self.children.push(child);
    }

    /// Removes a child by index, returning ownership.
    ///
    /// Resets hover/capture state since child indices shift.
    pub fn remove_child(&mut self, index: usize) -> Box<dyn Widget> {
        self.input_state = ContainerInputState::default();
        self.children.remove(index)
    }

    /// Number of children.
    pub fn child_count(&self) -> usize {
        self.children.len()
    }

    /// Read access to children.
    pub fn children(&self) -> &[Box<dyn Widget>] {
        &self.children
    }

    /// Mutable access to children.
    pub fn children_mut(&mut self) -> &mut [Box<dyn Widget>] {
        &mut self.children
    }

    /// Access a specific child by index.
    pub fn child(&self, index: usize) -> Option<&dyn Widget> {
        self.children.get(index).map(AsRef::as_ref)
    }

    /// Mutable access to a specific child by index.
    pub fn child_mut(&mut self, index: usize) -> Option<&mut Box<dyn Widget>> {
        self.children.get_mut(index)
    }
}

// Dirty tracking.
impl ContainerWidget {
    /// Updates dirty flags based on a child's event response.
    ///
    /// Sets the container's boolean flags and, if a tracker is provided,
    /// marks the source widget in the tracker for scoped invalidation.
    pub fn update_dirty(
        &mut self,
        response: &WidgetResponse,
        tracker: Option<&mut InvalidationTracker>,
    ) {
        match response.response {
            EventResponse::RequestLayout => {
                self.needs_layout = true;
                self.needs_paint = true;
            }
            EventResponse::RequestPaint => {
                self.needs_paint = true;
            }
            _ => {}
        }
        if let (Some(tracker), Some(source)) = (tracker, response.source) {
            tracker.mark(source, DirtyKind::from(response.response));
        }
    }

    /// Whether this container needs layout recomputation.
    pub fn needs_layout(&self) -> bool {
        self.needs_layout
    }

    /// Whether this container needs repaint.
    pub fn needs_paint(&self) -> bool {
        self.needs_paint
    }

    /// Clears dirty flags after layout/paint passes complete.
    pub fn clear_dirty(&mut self) {
        self.needs_layout = false;
        self.needs_paint = false;
    }
}

// Builder methods.
impl ContainerWidget {
    /// Adds a child widget via builder pattern.
    #[must_use]
    pub fn with_child(mut self, child: Box<dyn Widget>) -> Self {
        self.add_child(child);
        self
    }

    /// Adds multiple children at once.
    #[must_use]
    pub fn with_children(mut self, children: Vec<Box<dyn Widget>>) -> Self {
        self.children = children;
        self
    }

    /// Sets the gap between children along the main axis.
    #[must_use]
    pub fn with_gap(mut self, gap: f32) -> Self {
        if let LayoutMode::Flex { gap: ref mut g, .. } = self.layout_mode {
            *g = gap;
        }
        self
    }

    /// Sets padding inside the container edges.
    #[must_use]
    pub fn with_padding(mut self, padding: Insets) -> Self {
        self.padding = padding;
        self
    }

    /// Sets cross-axis alignment (only meaningful for flex containers).
    #[must_use]
    pub fn with_align(mut self, align: Align) -> Self {
        if let LayoutMode::Flex {
            align: ref mut a, ..
        } = self.layout_mode
        {
            *a = align;
        }
        self
    }

    /// Sets main-axis justification (only meaningful for flex containers).
    #[must_use]
    pub fn with_justify(mut self, justify: Justify) -> Self {
        if let LayoutMode::Flex {
            justify: ref mut j, ..
        } = self.layout_mode
        {
            *j = justify;
        }
        self
    }

    /// Sets width sizing.
    #[must_use]
    pub fn with_width(mut self, width: SizeSpec) -> Self {
        self.width = width;
        self
    }

    /// Sets height sizing.
    #[must_use]
    pub fn with_height(mut self, height: SizeSpec) -> Self {
        self.height = height;
        self
    }

    /// Enables clipping children to the container's bounds.
    #[must_use]
    pub fn with_clip(mut self, clip: bool) -> Self {
        self.clip_children = clip;
        self
    }
}

// Layout helpers live in `layout_build.rs`.

impl Widget for ContainerWidget {
    fn id(&self) -> WidgetId {
        self.id
    }

    fn is_focusable(&self) -> bool {
        false
    }

    fn layout(&self, ctx: &LayoutCtx<'_>) -> LayoutBox {
        self.build_layout_box(ctx)
    }

    fn sense(&self) -> Sense {
        Sense::none()
    }

    fn paint(&self, ctx: &mut DrawCtx<'_>) {
        let layout = self.get_or_compute_layout(ctx.measurer, ctx.theme, ctx.bounds);
        // Use content-space clip rect so visibility culling works inside
        // scroll transforms (where clip is in viewport space but child
        // layout rects are in content space).
        let visible_bounds = ctx
            .draw_list
            .current_clip_rect_in_content_space()
            .map_or(ctx.bounds, |clip| clip.intersection(ctx.bounds));

        if self.clip_children {
            ctx.draw_list.push_clip(ctx.bounds);
        }

        for (idx, child) in self.children.iter().enumerate() {
            if let Some(child_node) = layout.children.get(idx) {
                if !child_node.rect.intersects(visible_bounds) {
                    continue;
                }

                let child_id = child.id();
                let bounds = child_node.content_rect;

                // Scene cache hit — replay cached commands.
                if Self::try_replay_cached(ctx, child_id, bounds) {
                    continue;
                }

                // Cache miss — draw and capture for future reuse.
                let start = ctx.draw_list.len();
                let log_start = ctx.scene_cache.as_ref().map_or(0, |c| c.log_position());
                let mut child_ctx = DrawCtx {
                    measurer: ctx.measurer,
                    draw_list: ctx.draw_list,
                    bounds,
                    focused_widget: ctx.focused_widget,
                    now: ctx.now,
                    animations_running: ctx.animations_running,
                    theme: ctx.theme,
                    icons: ctx.icons,
                    scene_cache: ctx.scene_cache.as_deref_mut(),
                    interaction: None,
                    widget_id: None,
                    frame_requests: None,
                };
                child.paint(&mut child_ctx);
                Self::store_in_cache(ctx, child_id, bounds, start, log_start);
            }
        }

        if self.clip_children {
            ctx.draw_list.pop_clip();
        }
    }

    fn handle_mouse(&mut self, event: &MouseEvent, ctx: &EventCtx<'_>) -> WidgetResponse {
        self.dispatch_mouse(event, ctx)
    }

    fn handle_hover(&mut self, event: HoverEvent, ctx: &EventCtx<'_>) -> WidgetResponse {
        match event {
            HoverEvent::Enter => {
                // Position unknown — defer to next mouse Move for targeting.
                WidgetResponse::handled()
            }
            HoverEvent::Leave => {
                if let Some(idx) = self.input_state.hovered_child.take() {
                    let layout = self.get_or_compute_layout(ctx.measurer, ctx.theme, ctx.bounds);
                    if let (Some(child), Some(child_node)) =
                        (self.children.get_mut(idx), layout.children.get(idx))
                    {
                        let child_ctx = EventCtx {
                            measurer: ctx.measurer,
                            bounds: child_node.content_rect,
                            is_focused: ctx.focused_widget == Some(child.id()),
                            focused_widget: ctx.focused_widget,
                            theme: ctx.theme,
                            interaction: None,
                            widget_id: None,
                            frame_requests: None,
                        };
                        child.handle_hover(HoverEvent::Leave, &child_ctx);
                    }
                }
                WidgetResponse::handled()
            }
        }
    }

    fn handle_key(&mut self, event: KeyEvent, ctx: &EventCtx<'_>) -> WidgetResponse {
        self.dispatch_key(event, ctx)
    }

    fn accept_action(&mut self, action: &WidgetAction) -> bool {
        self.children.iter_mut().any(|c| c.accept_action(action))
    }

    fn focusable_children(&self) -> Vec<WidgetId> {
        self.children
            .iter()
            .flat_map(|c| c.focusable_children())
            .collect()
    }
}

#[cfg(test)]
mod tests;
