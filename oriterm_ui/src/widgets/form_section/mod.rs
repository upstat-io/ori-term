//! Collapsible form section widget.
//!
//! Groups related `FormRow`s under a header. Clicking the header
//! toggles expand/collapse. When collapsed, child rows are hidden
//! and the section occupies only the header height.

use std::cell::RefCell;
use std::rc::Rc;

use crate::draw::RectStyle;
use crate::geometry::{Point, Rect};
use crate::input::{InputEvent, MouseButton};

/// Map a logical row index to the child node index.
///
/// The header occupies index 0 in the layout children, so row `i` is at `i + 1`.
fn row_node_index(row_idx: usize) -> usize {
    row_idx + 1
}
use crate::layout::{Align, Direction, LayoutBox, LayoutNode, SizeSpec, compute_layout};
use crate::sense::Sense;
use crate::text::{FontWeight, TextStyle};
use crate::theme::UiTheme;
use crate::widget_id::WidgetId;

use super::form_row::FormRow;
use super::{DrawCtx, LayoutCtx, OnInputResult, TextMeasurer, Widget, WidgetAction};

/// Height of the section header in pixels.
const HEADER_HEIGHT: f32 = 28.0;

/// Gap between rows within a section (includes header-to-first-row gap).
const ROW_GAP: f32 = 12.0;

/// A collapsible section with a header label and child form rows.
///
/// When expanded, the header and all rows are visible. When collapsed,
/// only the header is shown. Clicking the header toggles state.
pub struct FormSection {
    id: WidgetId,
    title: String,
    rows: Vec<FormRow>,
    expanded: bool,

    /// Cached layout result, keyed by bounds.
    cached_layout: RefCell<Option<(Rect, Rc<LayoutNode>)>>,
}

impl FormSection {
    /// Creates an expanded section with the given title.
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            id: WidgetId::next(),
            title: title.into(),
            rows: Vec::new(),
            expanded: true,
            cached_layout: RefCell::new(None),
        }
    }

    /// Adds a form row to this section.
    #[must_use]
    pub fn with_row(mut self, row: FormRow) -> Self {
        self.rows.push(row);
        self
    }

    /// Sets initial expanded state.
    #[must_use]
    pub fn expanded(mut self, expanded: bool) -> Self {
        self.expanded = expanded;
        self
    }

    /// Returns the section title.
    pub fn title(&self) -> &str {
        &self.title
    }

    /// Returns whether this section is expanded.
    pub fn is_expanded(&self) -> bool {
        self.expanded
    }

    /// Read access to rows (for label width measurement).
    pub fn rows(&self) -> &[FormRow] {
        &self.rows
    }

    /// Mutable access to rows (for setting label widths).
    ///
    /// Must only be called during setup phase (before the first layout pass).
    /// Mutating rows after layout may cause stale cached layout data.
    pub fn rows_mut(&mut self) -> &mut [FormRow] {
        &mut self.rows
    }

    /// Returns the header text style for the given theme.
    fn header_style(theme: &UiTheme) -> TextStyle {
        TextStyle::new(theme.font_size, theme.fg_primary).with_weight(FontWeight::Bold)
    }

    /// Returns the expand/collapse indicator.
    fn indicator(&self) -> &str {
        if self.expanded { "▾" } else { "▸" }
    }
}

// Layout helpers.
impl FormSection {
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

    /// Builds a column: header + rows (if expanded).
    fn build_layout_box(&self, ctx: &LayoutCtx<'_>) -> LayoutBox {
        let header_box = LayoutBox::leaf(0.0, HEADER_HEIGHT).with_width(SizeSpec::Fill);

        let mut children = vec![header_box];

        if self.expanded {
            for row in &self.rows {
                children.push(row.layout(ctx).with_width(SizeSpec::Fill));
            }
        }

        LayoutBox::flex(Direction::Column, children)
            .with_gap(if self.expanded { ROW_GAP } else { 0.0 })
            .with_align(Align::Stretch)
            .with_width(SizeSpec::Fill)
            .with_widget_id(self.id)
    }
}

impl Widget for FormSection {
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
        // The header area is clickable for expand/collapse; sense must be
        // non-none so the propagation pipeline includes this widget in the
        // hit path and delivers `on_input` calls for header clicks.
        Sense::click()
    }

    fn on_input(&mut self, event: &InputEvent, bounds: Rect) -> OnInputResult {
        // Toggle expand/collapse on left-click in the header zone.
        if let InputEvent::MouseDown {
            pos,
            button: MouseButton::Left,
            ..
        } = event
        {
            let header_bottom = bounds.y() + HEADER_HEIGHT;
            if pos.y >= bounds.y() && pos.y < header_bottom {
                self.expanded = !self.expanded;
                *self.cached_layout.borrow_mut() = None;
                return OnInputResult::handled();
            }
        }
        OnInputResult::ignored()
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

        // Draw header.
        if let Some(header_node) = layout.children.first() {
            if header_node.rect.intersects(visible_bounds) {
                self.draw_header(ctx, &header_node.content_rect);
            }
        }

        // Draw rows (if expanded).
        if self.expanded {
            for (i, row) in self.rows.iter().enumerate() {
                // Row nodes start at index 1 (index 0 is the header).
                if let Some(row_node) = layout.children.get(row_node_index(i)) {
                    if !row_node.rect.intersects(visible_bounds) {
                        continue;
                    }
                    let mut child_ctx = DrawCtx {
                        measurer: ctx.measurer,
                        draw_list: ctx.draw_list,
                        bounds: row_node.content_rect,
                        now: ctx.now,
                        theme: ctx.theme,
                        icons: ctx.icons,
                        scene_cache: ctx.scene_cache.as_deref_mut(),
                        interaction: None,
                        widget_id: None,
                        frame_requests: None,
                    };
                    row.paint(&mut child_ctx);
                }
            }
        }
    }

    fn for_each_child_mut(&mut self, visitor: &mut dyn FnMut(&mut dyn Widget)) {
        for row in &mut self.rows {
            visitor(row);
        }
    }

    fn accept_action(&mut self, action: &WidgetAction) -> bool {
        self.rows.iter_mut().any(|r| r.accept_action(action))
    }

    fn focusable_children(&self) -> Vec<WidgetId> {
        if !self.expanded {
            return Vec::new();
        }
        self.rows
            .iter()
            .flat_map(Widget::focusable_children)
            .collect()
    }
}

impl FormSection {
    /// Draws the section header with indicator, title, and subtle separator.
    fn draw_header(&self, ctx: &mut DrawCtx<'_>, bounds: &Rect) {
        let indicator = self.indicator();
        let style = Self::header_style(ctx.theme);

        // Measure for vertical centering.
        let ind_metrics = ctx.measurer.measure(indicator, &style, f32::INFINITY);
        let text_y = bounds.y() + (bounds.height() - ind_metrics.height) / 2.0;

        // Draw indicator.
        let ind_shaped = ctx.measurer.shape(indicator, &style, f32::INFINITY);
        ctx.draw_list.push_text(
            Point::new(bounds.x(), text_y),
            ind_shaped,
            ctx.theme.fg_primary,
        );

        // Draw title after indicator with a small gap.
        let title_x = bounds.x() + ind_metrics.width + 6.0;
        let title_shaped = ctx.measurer.shape(
            &self.title,
            &style,
            bounds.width() - ind_metrics.width - 6.0,
        );
        ctx.draw_list.push_text(
            Point::new(title_x, text_y),
            title_shaped,
            ctx.theme.fg_primary,
        );

        // Subtle separator line below header.
        let line_y = bounds.bottom() - 1.0;
        let line_rect = Rect::new(bounds.x(), line_y, bounds.width(), 1.0);
        let line_color = ctx.theme.border.with_alpha(0.3);
        ctx.draw_list
            .push_rect(line_rect, RectStyle::filled(line_color));
    }
}

#[cfg(test)]
mod tests;
