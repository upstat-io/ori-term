//! `Widget` trait implementation for `SliderWidget`.
//!
//! Separated from `mod.rs` to keep files under 500 lines.

use crate::color::Color;
use crate::controllers::EventController;
use crate::draw::RectStyle;
use crate::geometry::{Point, Rect};
use crate::layout::{LayoutBox, SizeSpec};
use crate::sense::Sense;
use crate::text::TextStyle;
use crate::visual_state::transition::VisualStateAnimator;
use crate::widget_id::WidgetId;

use super::super::{DrawCtx, LayoutCtx, Widget, WidgetAction};
use super::{SliderWidget, VALUE_GAP, VALUE_LABEL_WIDTH};

impl Widget for SliderWidget {
    fn id(&self) -> WidgetId {
        self.id
    }

    fn is_focusable(&self) -> bool {
        !self.disabled
    }

    fn sense(&self) -> Sense {
        Sense::click_and_drag()
    }

    fn layout(&self, _ctx: &LayoutCtx<'_>) -> LayoutBox {
        let height = self.style.thumb_size.max(self.style.track_height);
        LayoutBox::leaf(self.style.width, height)
            .with_width(SizeSpec::Fill)
            .with_widget_id(self.id)
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
        let focused = ctx.is_interaction_focused() || ctx.focused_widget == Some(self.id);
        let s = &self.style;
        let tb = self.track_bounds(ctx.bounds);

        // Focus ring around track area.
        if focused {
            let ring = tb.inset(crate::geometry::Insets::all(-2.0));
            let ring_style = RectStyle::filled(Color::TRANSPARENT)
                .with_border(2.0, s.focus_ring_color)
                .with_radius(s.track_radius + 2.0);
            ctx.draw_list.push_rect(ring, ring_style);
        }

        // Track background.
        let track_y = tb.y() + (tb.height() - s.track_height) / 2.0;
        let track_rect = Rect::new(tb.x(), track_y, tb.width(), s.track_height);
        let bg_color = if self.disabled {
            s.disabled_bg
        } else {
            s.track_bg
        };
        let track_style = RectStyle::filled(bg_color).with_radius(s.track_radius);
        ctx.draw_list.push_rect(track_rect, track_style);

        // Filled portion (left of thumb).
        let norm = self.normalized();
        let fill_width = norm * tb.width();
        if fill_width > 0.0 {
            let fill_rect = Rect::new(tb.x(), track_y, fill_width, s.track_height);
            let fill_color = if self.disabled {
                s.disabled_fill
            } else {
                s.fill_color
            };
            let fill_style = RectStyle::filled(fill_color).with_radius(s.track_radius);
            ctx.draw_list.push_rect(fill_rect, fill_style);
        }

        // Thumb.
        let half_thumb = s.thumb_size / 2.0;
        let travel = tb.width() - s.thumb_size;
        let thumb_x = tb.x() + travel * norm;
        let thumb_y = tb.y() + (tb.height() - s.thumb_size) / 2.0;
        let thumb_rect = Rect::new(thumb_x, thumb_y, s.thumb_size, s.thumb_size);
        let thumb_bg = if self.disabled {
            s.disabled_bg
        } else {
            self.animator.get_bg_color(ctx.now)
        };
        let thumb_style = RectStyle::filled(thumb_bg)
            .with_border(s.thumb_border_width, s.thumb_border_color)
            .with_radius(half_thumb);
        ctx.draw_list.push_rect(thumb_rect, thumb_style);

        // Value label to the right of the track.
        let value_text = self.format_value();
        let text_style = TextStyle::new(ctx.theme.font_size, ctx.theme.fg_secondary);
        let shaped = ctx
            .measurer
            .shape(&value_text, &text_style, VALUE_LABEL_WIDTH);
        let label_x = tb.right() + VALUE_GAP;
        // Right-align within the label area.
        let text_x = label_x + VALUE_LABEL_WIDTH - shaped.width;
        let text_y = ctx.bounds.y() + (ctx.bounds.height() - shaped.height) / 2.0;
        ctx.draw_list
            .push_text(Point::new(text_x, text_y), shaped, ctx.theme.fg_secondary);

        // Signal continued redraws while the animator is transitioning.
        if self.animator.is_animating(ctx.now) {
            ctx.request_anim_frame();
        }
    }

    fn on_action(&mut self, action: WidgetAction, bounds: Rect) -> Option<WidgetAction> {
        let tb = self.track_bounds(bounds);
        match action {
            WidgetAction::DragStart { pos, .. } => {
                self.drag_origin = Some(pos);
                self.set_value_action(self.value_from_x(pos.x, tb))
            }
            WidgetAction::DragUpdate { total_delta, .. } => {
                if let Some(origin) = self.drag_origin {
                    let x = origin.x + total_delta.x;
                    self.set_value_action(self.value_from_x(x, tb))
                } else {
                    None
                }
            }
            WidgetAction::DragEnd { .. } => {
                self.drag_origin = None;
                None
            }
            // SliderKeyController emits ValueChanged — sync widget value.
            WidgetAction::ValueChanged { value, .. } => {
                self.value = value.clamp(self.min, self.max);
                Some(WidgetAction::ValueChanged {
                    id: self.id,
                    value: self.value,
                })
            }
            other => Some(other),
        }
    }
}
