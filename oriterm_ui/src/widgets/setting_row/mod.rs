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
use crate::text::{FontWeight, TextStyle, TextTransform};
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

/// Tag font size (CSS: `font-size: 9px`).
const TAG_FONT_SIZE: f32 = 9.0;

/// Gap between name text and first tag chip (CSS: `gap: 6px`).
const NAME_TAG_GAP: f32 = 6.0;

/// Tag horizontal padding (CSS: `padding: 2px 5px`).
const TAG_PAD_H: f32 = 5.0;

/// Tag vertical padding (CSS: `padding: 2px 5px`).
const TAG_PAD_V: f32 = 2.0;

/// Tag border width (CSS: `border: 1px solid currentColor`).
const TAG_BORDER: f32 = 1.0;

/// Tag letter spacing (CSS: `letter-spacing: 0.06em` at 9px = 0.54px).
const TAG_LETTER_SPACING: f32 = 0.54;

/// Inline status tag variant.
///
/// Each variant maps to a `(text_color, bg_color)` pair from the theme.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingTagKind {
    /// New feature indicator.
    New,
    /// Requires restart to take effect.
    Restart,
    /// Advanced/power-user setting.
    Advanced,
    /// Experimental/unstable feature.
    Experimental,
}

impl SettingTagKind {
    /// Returns `(text_color, bg_color)` for this tag variant.
    pub fn colors(self, theme: &UiTheme) -> (Color, Color) {
        match self {
            Self::New => (theme.accent, theme.accent_bg_strong),
            Self::Restart => (theme.warning, theme.warning_bg),
            Self::Advanced => (theme.fg_secondary, theme.bg_secondary),
            Self::Experimental => (theme.danger, theme.danger_bg),
        }
    }
}

/// An inline status tag displayed after the setting name.
///
/// Tags render as small uppercase chips with a 1px border in the variant's
/// theme color, placed in a row after the setting name text.
#[derive(Debug, Clone)]
pub struct SettingTag {
    /// Tag variant determining colors.
    pub kind: SettingTagKind,
    /// Display text (e.g., "Restart", "Advanced").
    pub text: String,
}

impl SettingTag {
    /// Creates a new setting tag.
    pub fn new(kind: SettingTagKind, text: impl Into<String>) -> Self {
        Self {
            kind,
            text: text.into(),
        }
    }
}

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
    tags: Vec<SettingTag>,

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
            tags: Vec::new(),
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

    /// Adds an inline status tag after the setting name.
    #[must_use]
    pub fn with_tag(mut self, tag: SettingTag) -> Self {
        self.tags.push(tag);
        self
    }

    /// Returns the name label text.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the description label text.
    pub fn description(&self) -> &str {
        &self.description
    }

    /// Returns the inline status tags.
    pub fn tags(&self) -> &[SettingTag] {
        &self.tags
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

        // Name line: plain leaf when no tags, row with tag chips when tags exist.
        let name_line = if self.tags.is_empty() {
            LayoutBox::leaf(name_m.width, name_m.height)
        } else {
            let tag_style = tag_text_style(ctx.theme);
            let mut children = vec![LayoutBox::leaf(name_m.width, name_m.height)];
            for tag in &self.tags {
                let m = ctx.measurer.measure(&tag.text, &tag_style, f32::INFINITY);
                let w = m.width + 2.0 * (TAG_PAD_H + TAG_BORDER);
                let h = m.height + 2.0 * (TAG_PAD_V + TAG_BORDER);
                children.push(LayoutBox::leaf(w, h));
            }
            LayoutBox::flex(Direction::Row, children)
                .with_gap(NAME_TAG_GAP)
                .with_align(Align::Center)
        };

        // Left: name line + description stacked vertically.
        let label_box = LayoutBox::flex(
            Direction::Column,
            vec![name_line, LayoutBox::leaf(desc_m.width, desc_m.height)],
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
        let bg = self.animator.get_bg_color();
        if bg.a > 0.001 {
            let rect_style = RectStyle::filled(bg).with_radius(CORNER_RADIUS);
            ctx.scene.push_quad(ctx.bounds, rect_style);
        }

        let layout = self.get_or_compute_layout(ctx.measurer, ctx.theme, ctx.bounds);

        // Draw label area (first child = the column with name + desc).
        if let Some(label_col) = layout.children.first() {
            if let Some(first_child) = label_col.children.first() {
                if self.tags.is_empty() {
                    // Name label (first child of the column) — direct leaf.
                    paint_name(&self.name, ctx, first_child);
                } else {
                    // Name text (first child of the nested row).
                    if let Some(n) = first_child.children.first() {
                        paint_name(&self.name, ctx, n);
                    }
                    // Tag chips (remaining children of the row).
                    for (i, tag) in self.tags.iter().enumerate() {
                        if let Some(tag_node) = first_child.children.get(i + 1) {
                            paint_tag_chip(tag, ctx, tag_node);
                        }
                    }
                }
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
        if self.animator.is_animating() {
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

/// Builds the `TextStyle` used for tag chip text.
fn tag_text_style(theme: &UiTheme) -> TextStyle {
    TextStyle {
        size: TAG_FONT_SIZE,
        weight: FontWeight::BOLD,
        letter_spacing: TAG_LETTER_SPACING,
        text_transform: TextTransform::Uppercase,
        color: theme.fg_primary,
        line_height: Some(1.3),
        ..TextStyle::default()
    }
}

/// Paints the setting name text at the given layout node position.
fn paint_name(name: &str, ctx: &mut DrawCtx<'_>, node: &LayoutNode) {
    let style = TextStyle::new(NAME_FONT_SIZE, ctx.theme.fg_primary);
    let shaped = ctx.measurer.shape(name, &style, node.content_rect.width());
    let pos = Point::new(node.content_rect.x(), node.content_rect.y());
    ctx.scene.push_text(pos, shaped, ctx.theme.fg_primary);
}

/// Paints a single tag chip: background quad with border, then uppercase text.
fn paint_tag_chip(tag: &SettingTag, ctx: &mut DrawCtx<'_>, node: &LayoutNode) {
    let (text_color, bg_color) = tag.kind.colors(ctx.theme);

    // Background + border.
    let rect_style = RectStyle::filled(bg_color).with_border(TAG_BORDER, text_color);
    ctx.scene.push_quad(node.content_rect, rect_style);

    // Text centered inside the chip.
    let style = tag_text_style(ctx.theme);
    let shaped = ctx.measurer.shape(&tag.text, &style, f32::INFINITY);
    let x = node.content_rect.x() + TAG_PAD_H + TAG_BORDER;
    let y = node.content_rect.y() + TAG_PAD_V + TAG_BORDER;
    ctx.scene.push_text(Point::new(x, y), shaped, text_color);
}

#[cfg(test)]
mod tests;
