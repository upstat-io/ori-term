//! Top-level form layout widget.
//!
//! Composes `FormSection`s into a vertical stack with aligned label
//! columns across all rows. The widest label determines the label
//! column width for uniform alignment.

use std::cell::RefCell;
use std::rc::Rc;

use crate::geometry::{Insets, Point, Rect};
use crate::input::{HoverEvent, KeyEvent, MouseEvent, MouseEventKind};
use crate::layout::{Align, Direction, LayoutBox, LayoutNode, SizeSpec, compute_layout};
use crate::sense::Sense;
use crate::theme::UiTheme;
use crate::widget_id::WidgetId;

use super::form_section::FormSection;
use super::{
    CaptureRequest, DrawCtx, EventCtx, LayoutCtx, TextMeasurer, Widget, WidgetAction,
    WidgetResponse,
};

/// Padding added to the computed label width for breathing room.
const LABEL_PADDING: f32 = 12.0;

/// Gap between sections.
const SECTION_GAP: f32 = 24.0;

/// Inner padding around the form content.
const FORM_PADDING: Insets = Insets::tlbr(16.0, 24.0, 16.0, 24.0);

/// Vertical layout of form sections with aligned label columns.
///
/// Measures all labels across all rows and aligns label columns to
/// the widest label. Sections are separated by `SECTION_GAP`.
pub struct FormLayout {
    id: WidgetId,
    sections: Vec<FormSection>,

    /// Computed label column width (widest label + padding).
    label_column_width: f32,
    /// Index of the section with active mouse capture (drag in progress).
    captured_section: Option<usize>,
    /// Index of the section currently under the cursor (for hover tracking).
    hovered_section: Option<usize>,

    /// Cached layout result, keyed by bounds.
    cached_layout: RefCell<Option<(Rect, Rc<LayoutNode>)>>,
}

impl FormLayout {
    /// Creates an empty form layout.
    pub fn new() -> Self {
        Self {
            id: WidgetId::next(),
            sections: Vec::new(),
            label_column_width: 100.0,
            captured_section: None,
            hovered_section: None,
            cached_layout: RefCell::new(None),
        }
    }

    /// Adds a section to the form.
    #[must_use]
    pub fn with_section(mut self, section: FormSection) -> Self {
        self.sections.push(section);
        self
    }

    /// Read access to sections.
    pub fn sections(&self) -> &[FormSection] {
        &self.sections
    }

    /// Mutable access to sections.
    ///
    /// Must only be called during setup phase (before the first layout pass).
    /// Mutating sections after layout may cause stale cached layout data.
    pub fn sections_mut(&mut self) -> &mut [FormSection] {
        &mut self.sections
    }

    /// Measures all labels and computes the aligned label column width.
    ///
    /// Call this after building the form and before the first draw/layout.
    /// Sets the label width on every `FormRow` for uniform alignment.
    pub fn compute_label_widths(&mut self, measurer: &dyn TextMeasurer, theme: &UiTheme) {
        let mut max_width: f32 = 0.0;
        for section in &self.sections {
            for row in section.rows() {
                let w = row.measure_label_width(measurer, theme);
                max_width = max_width.max(w);
            }
        }
        self.label_column_width = max_width + LABEL_PADDING;

        // Propagate to all rows.
        for section in &mut self.sections {
            for row in section.rows_mut() {
                row.set_label_width(self.label_column_width);
            }
        }
    }
}

// Layout helpers.
impl FormLayout {
    /// Returns cached layout if bounds match, otherwise recomputes.
    fn get_or_compute_layout(
        &self,
        measurer: &dyn TextMeasurer,
        theme: &UiTheme,
        bounds: Rect,
    ) -> Rc<LayoutNode> {
        {
            let cached = self.cached_layout.borrow();
            if let Some((ref cb, ref node)) = *cached {
                if *cb == bounds {
                    return Rc::clone(node);
                }
            }
        }
        let ctx = LayoutCtx { measurer, theme };
        let layout_box = self.build_layout_box(&ctx);
        let node = Rc::new(compute_layout(&layout_box, bounds));
        *self.cached_layout.borrow_mut() = Some((bounds, Rc::clone(&node)));
        node
    }

    /// Builds a vertical column of sections.
    fn build_layout_box(&self, ctx: &LayoutCtx<'_>) -> LayoutBox {
        let section_boxes: Vec<LayoutBox> = self
            .sections
            .iter()
            .map(|s| s.layout(ctx).with_width(SizeSpec::Fill))
            .collect();
        LayoutBox::flex(Direction::Column, section_boxes)
            .with_gap(SECTION_GAP)
            .with_padding(FORM_PADDING)
            .with_align(Align::Stretch)
            .with_width(SizeSpec::Fill)
            .with_widget_id(self.id)
    }
}

impl Widget for FormLayout {
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

        for (idx, section) in self.sections.iter().enumerate() {
            if let Some(section_node) = layout.children.get(idx) {
                if !section_node.rect.intersects(visible_bounds) {
                    continue;
                }
                let mut child_ctx = DrawCtx {
                    measurer: ctx.measurer,
                    draw_list: ctx.draw_list,
                    bounds: section_node.content_rect,
                    focused_widget: ctx.focused_widget,
                    now: ctx.now,
                    theme: ctx.theme,
                    icons: ctx.icons,
                    scene_cache: ctx.scene_cache.as_deref_mut(),
                    interaction: None,
                    widget_id: None,
                    frame_requests: None,
                };
                section.paint(&mut child_ctx);
            }
        }
    }

    fn for_each_child_mut(&mut self, visitor: &mut dyn FnMut(&mut dyn Widget)) {
        for section in &mut self.sections {
            visitor(section);
        }
    }

    fn handle_mouse(&mut self, event: &MouseEvent, ctx: &EventCtx<'_>) -> WidgetResponse {
        let layout = self.get_or_compute_layout(ctx.measurer, ctx.theme, ctx.bounds);

        // During capture, route all events to the captured section.
        if let Some(cap_idx) = self.captured_section {
            if let (Some(section), Some(node)) =
                (self.sections.get_mut(cap_idx), layout.children.get(cap_idx))
            {
                let child_ctx = ctx.for_child(node.content_rect, None);
                let resp = section.handle_mouse(event, &child_ctx);
                if resp.capture.should_release(&event.kind) {
                    self.captured_section = None;
                }
                if resp.response.needs_layout() {
                    *self.cached_layout.borrow_mut() = None;
                }
                return resp;
            }
        }

        // Move events: position-based hover tracking.
        if event.kind == MouseEventKind::Move {
            return self.update_section_hover(&layout, event.pos, ctx);
        }

        for (idx, section) in self.sections.iter_mut().enumerate() {
            if let Some(section_node) = layout.children.get(idx) {
                if section_node.rect.contains(event.pos) {
                    let child_ctx = ctx.for_child(section_node.content_rect, None);
                    let resp = section.handle_mouse(event, &child_ctx);
                    if resp.capture == CaptureRequest::Acquire {
                        self.captured_section = Some(idx);
                    }
                    if resp.response.needs_layout() {
                        *self.cached_layout.borrow_mut() = None;
                    }
                    return resp;
                }
            }
        }
        WidgetResponse::ignored()
    }

    fn handle_hover(&mut self, event: HoverEvent, ctx: &EventCtx<'_>) -> WidgetResponse {
        if event == HoverEvent::Leave {
            self.captured_section = None;
            // Send Leave to the currently hovered section.
            if let Some(old_idx) = self.hovered_section.take() {
                let layout = self.get_or_compute_layout(ctx.measurer, ctx.theme, ctx.bounds);
                if let (Some(section), Some(node)) =
                    (self.sections.get_mut(old_idx), layout.children.get(old_idx))
                {
                    let child_ctx = EventCtx {
                        measurer: ctx.measurer,
                        bounds: node.content_rect,
                        is_focused: false,
                        focused_widget: ctx.focused_widget,
                        theme: ctx.theme,
                        interaction: None,
                        widget_id: None,
                        frame_requests: None,
                    };
                    section.handle_hover(HoverEvent::Leave, &child_ctx);
                }
                return WidgetResponse::paint();
            }
        }
        // Enter is handled by Move-based hover tracking.
        WidgetResponse::ignored()
    }

    fn handle_key(&mut self, event: KeyEvent, ctx: &EventCtx<'_>) -> WidgetResponse {
        let layout = self.get_or_compute_layout(ctx.measurer, ctx.theme, ctx.bounds);
        for (idx, section) in self.sections.iter_mut().enumerate() {
            if let Some(section_node) = layout.children.get(idx) {
                let child_ctx = EventCtx {
                    measurer: ctx.measurer,
                    bounds: section_node.content_rect,
                    is_focused: false,
                    focused_widget: ctx.focused_widget,
                    theme: ctx.theme,
                    interaction: None,
                    widget_id: None,
                    frame_requests: None,
                };
                let resp = section.handle_key(event, &child_ctx);
                if resp.response.is_handled() {
                    if resp.response.needs_layout() {
                        *self.cached_layout.borrow_mut() = None;
                    }
                    return resp;
                }
            }
        }
        WidgetResponse::ignored()
    }

    fn accept_action(&mut self, action: &WidgetAction) -> bool {
        self.sections.iter_mut().any(|s| s.accept_action(action))
    }

    fn focusable_children(&self) -> Vec<WidgetId> {
        self.sections
            .iter()
            .flat_map(Widget::focusable_children)
            .collect()
    }
}

// Hover tracking.
impl FormLayout {
    /// Updates section hover state based on cursor position.
    fn update_section_hover(
        &mut self,
        layout: &LayoutNode,
        pos: Point,
        ctx: &EventCtx<'_>,
    ) -> WidgetResponse {
        let new_hover = self.sections.iter().enumerate().find_map(|(i, _)| {
            layout
                .children
                .get(i)
                .filter(|n| n.rect.contains(pos))
                .map(|_| i)
        });

        if new_hover == self.hovered_section {
            // Same section — forward Move so it can update row hover.
            if let Some(idx) = new_hover {
                if let (Some(section), Some(node)) =
                    (self.sections.get_mut(idx), layout.children.get(idx))
                {
                    let child_ctx = EventCtx {
                        measurer: ctx.measurer,
                        bounds: node.content_rect,
                        is_focused: false,
                        focused_widget: ctx.focused_widget,
                        theme: ctx.theme,
                        interaction: None,
                        widget_id: None,
                        frame_requests: None,
                    };
                    let move_event = MouseEvent {
                        kind: MouseEventKind::Move,
                        pos,
                        modifiers: crate::input::Modifiers::NONE,
                    };
                    return section.handle_mouse(&move_event, &child_ctx);
                }
            }
            return WidgetResponse::ignored();
        }

        // Leave old section.
        if let Some(old_idx) = self.hovered_section {
            if let (Some(section), Some(node)) =
                (self.sections.get_mut(old_idx), layout.children.get(old_idx))
            {
                let child_ctx = ctx.for_child(node.content_rect, None);
                section.handle_hover(HoverEvent::Leave, &child_ctx);
            }
        }

        // Enter new section (forward Move so it sets up row hover).
        if let Some(new_idx) = new_hover {
            if let (Some(section), Some(node)) =
                (self.sections.get_mut(new_idx), layout.children.get(new_idx))
            {
                let child_ctx = ctx.for_child(node.content_rect, None);
                let move_event = MouseEvent {
                    kind: MouseEventKind::Move,
                    pos,
                    modifiers: crate::input::Modifiers::NONE,
                };
                section.handle_mouse(&move_event, &child_ctx);
            }
        }

        self.hovered_section = new_hover;
        WidgetResponse::paint()
    }
}

impl Default for FormLayout {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests;
