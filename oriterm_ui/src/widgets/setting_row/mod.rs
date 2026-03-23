//! Setting row widget — two-line label with right-side control.
//!
//! Displays a name and description on the left, a control widget on the right,
//! and a full-width hover background via `HoverController` + `VisualStateAnimator`.

use std::cell::RefCell;
use std::rc::Rc;

use crate::color::Color;
use crate::controllers::{EventController, HoverController};
use crate::draw::RectStyle;
use crate::geometry::{Insets, Point, Rect};
use crate::layout::{Align, Direction, LayoutBox, LayoutNode, SizeSpec, compute_layout};
use crate::sense::Sense;
use crate::text::TextStyle;
use crate::theme::UiTheme;
use crate::visual_state::common_states;
use crate::visual_state::transition::VisualStateAnimator;
use crate::widget_id::WidgetId;

use super::{DrawCtx, LayoutCtx, TextMeasurer, Widget, WidgetAction};

/// Minimum row height in logical pixels.
const MIN_HEIGHT: f32 = 44.0;

/// Name label font size.
const NAME_FONT_SIZE: f32 = 13.0;

/// Description label font size.
const DESC_FONT_SIZE: f32 = 11.5;

/// Corner radius for hover background.
const CORNER_RADIUS: f32 = 0.0;

/// Padding inside the row.
const ROW_PADDING: Insets = Insets::vh(10.0, 14.0);

/// Gap between label area and control.
const LABEL_CONTROL_GAP: f32 = 24.0;

/// Gap between name and description lines.
const NAME_DESC_GAP: f32 = 2.0;

/// A settings row with name + description labels and a right-side control.
///
/// Hover background transitions smoothly via `VisualStateAnimator`. The control
/// widget (dropdown, toggle, slider, etc.) handles its own input — the row only
/// tracks hover state.
pub struct SettingRowWidget {
    id: WidgetId,
    name: String,
    description: String,
    control: Box<dyn Widget>,

    // Interaction.
    controllers: Vec<Box<dyn EventController>>,
    animator: VisualStateAnimator,

    /// Cached layout result, keyed by bounds.
    cached_layout: RefCell<Option<(Rect, Rc<LayoutNode>)>>,
}

impl SettingRowWidget {
    /// Creates a setting row with name, description, and control widget.
    pub fn new(
        name: impl Into<String>,
        description: impl Into<String>,
        control: Box<dyn Widget>,
        theme: &UiTheme,
    ) -> Self {
        Self {
            id: WidgetId::next(),
            name: name.into(),
            description: description.into(),
            control,
            controllers: vec![Box::new(HoverController::new())],
            animator: VisualStateAnimator::new(vec![common_states(
                Color::TRANSPARENT,
                theme.bg_card,
                Color::TRANSPARENT,
                Color::TRANSPARENT,
            )]),
            cached_layout: RefCell::new(None),
        }
    }

    /// Returns the name label text.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the description label text.
    pub fn description(&self) -> &str {
        &self.description
    }
}

// Layout helpers.
impl SettingRowWidget {
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

    /// Builds a row layout: [name+desc column (fill)] [control (hug)].
    fn build_layout_box(&self, ctx: &LayoutCtx<'_>) -> LayoutBox {
        let name_style = TextStyle::new(NAME_FONT_SIZE, ctx.theme.fg_primary);
        let desc_style = TextStyle::new(DESC_FONT_SIZE, ctx.theme.fg_secondary);
        let name_m = ctx.measurer.measure(&self.name, &name_style, f32::INFINITY);
        let desc_m = ctx
            .measurer
            .measure(&self.description, &desc_style, f32::INFINITY);

        // Left: name + description stacked vertically.
        let label_box = LayoutBox::flex(
            Direction::Column,
            vec![
                LayoutBox::leaf(name_m.width, name_m.height),
                LayoutBox::leaf(desc_m.width, desc_m.height),
            ],
        )
        .with_gap(NAME_DESC_GAP)
        .with_width(SizeSpec::Fill);

        // Right: control widget.
        let control_box = self.control.layout(ctx);

        // Row with labels on left, control on right, center-aligned vertically.
        LayoutBox::flex(Direction::Row, vec![label_box, control_box])
            .with_align(Align::Center)
            .with_gap(LABEL_CONTROL_GAP)
            .with_padding(ROW_PADDING)
            .with_min_height(MIN_HEIGHT)
            .with_widget_id(self.id)
    }
}

impl Widget for SettingRowWidget {
    fn id(&self) -> WidgetId {
        self.id
    }

    fn is_focusable(&self) -> bool {
        false
    }

    fn sense(&self) -> Sense {
        Sense::hover()
    }

    fn layout(&self, ctx: &LayoutCtx<'_>) -> LayoutBox {
        self.build_layout_box(ctx)
    }

    fn controllers(&self) -> &[Box<dyn EventController>] {
        &self.controllers
    }

    fn controllers_mut(&mut self) -> &mut [Box<dyn EventController>] {
        &mut self.controllers
    }

    fn visual_states(&self) -> Option<&VisualStateAnimator> {
        Some(&self.animator)
    }

    fn visual_states_mut(&mut self) -> Option<&mut VisualStateAnimator> {
        Some(&mut self.animator)
    }

    fn paint(&self, ctx: &mut DrawCtx<'_>) {
        // Invalidate cache each frame so children with changed sizes get fresh layout.
        *self.cached_layout.borrow_mut() = None;

        // Hover background.
        let bg = self.animator.get_bg_color(ctx.now);
        if bg.a > 0.001 {
            let rect_style = RectStyle::filled(bg).with_radius(CORNER_RADIUS);
            ctx.scene.push_quad(ctx.bounds, rect_style);
        }

        let layout = self.get_or_compute_layout(ctx.measurer, ctx.theme, ctx.bounds);

        // Draw label area (first child = the column with name + desc).
        if let Some(label_col) = layout.children.first() {
            // Name label (first child of the column).
            if let Some(name_node) = label_col.children.first() {
                let style = TextStyle::new(NAME_FONT_SIZE, ctx.theme.fg_primary);
                let shaped = ctx
                    .measurer
                    .shape(&self.name, &style, name_node.content_rect.width());
                let pos = Point::new(name_node.content_rect.x(), name_node.content_rect.y());
                ctx.scene.push_text(pos, shaped, ctx.theme.fg_primary);
            }
            // Description label (second child of the column).
            if let Some(desc_node) = label_col.children.get(1) {
                let style = TextStyle::new(DESC_FONT_SIZE, ctx.theme.fg_secondary);
                let shaped =
                    ctx.measurer
                        .shape(&self.description, &style, desc_node.content_rect.width());
                let pos = Point::new(desc_node.content_rect.x(), desc_node.content_rect.y());
                ctx.scene.push_text(pos, shaped, ctx.theme.fg_secondary);
            }
        }

        // Draw control (second child of the row).
        if let Some(control_node) = layout.children.get(1) {
            let mut child_ctx = DrawCtx {
                measurer: ctx.measurer,
                scene: ctx.scene,
                bounds: control_node.content_rect,
                now: ctx.now,
                theme: ctx.theme,
                icons: ctx.icons,
                interaction: None,
                widget_id: None,
                frame_requests: ctx.frame_requests,
            };
            self.control.paint(&mut child_ctx);
        }

        // Keep animating while transitioning.
        if self.animator.is_animating(ctx.now) {
            ctx.request_anim_frame();
        }
    }

    fn for_each_child_mut(&mut self, visitor: &mut dyn FnMut(&mut dyn Widget)) {
        visitor(self.control.as_mut());
    }

    fn accept_action(&mut self, action: &WidgetAction) -> bool {
        self.control.accept_action(action)
    }

    fn focusable_children(&self) -> Vec<WidgetId> {
        self.control.focusable_children()
    }
}

#[cfg(test)]
mod tests;
