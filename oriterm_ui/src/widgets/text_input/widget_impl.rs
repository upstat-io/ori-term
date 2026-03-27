//! `Widget` trait implementation for `TextInputWidget`.
//!
//! Separated from `mod.rs` to keep files under 500 lines.

use crate::controllers::EventController;
use crate::draw::RectStyle;
use crate::geometry::{Point, Rect};
use crate::input::{InputEvent, Key, Modifiers, MouseButton};
use crate::layout::LayoutBox;
use crate::sense::Sense;
use crate::visual_state::transition::VisualStateAnimator;
use crate::widget_id::WidgetId;

use super::super::{DrawCtx, LayoutCtx, OnInputResult, Widget, WidgetAction};
use super::TextInputWidget;

impl Widget for TextInputWidget {
    fn id(&self) -> WidgetId {
        self.id
    }

    fn is_focusable(&self) -> bool {
        !self.disabled
    }

    fn sense(&self) -> Sense {
        Sense::click_and_drag().union(Sense::focusable())
    }

    #[expect(
        clippy::string_slice,
        reason = "char_indices guarantees valid boundaries"
    )]
    fn layout(&self, ctx: &LayoutCtx<'_>) -> LayoutBox {
        let style = self.text_style();
        let text = self.editing.text();
        let metrics = ctx.measurer.measure(
            if text.is_empty() {
                &self.placeholder
            } else {
                text
            },
            &style,
            f32::INFINITY,
        );
        let w = (metrics.width + self.style.padding.width()).max(self.style.min_width);
        let h = metrics.height + self.style.padding.height();

        // Cache character X-offsets for click-to-cursor in on_input.
        let mut offsets = self.char_offsets.borrow_mut();
        offsets.clear();
        for (i, _) in text.char_indices() {
            let x = ctx
                .measurer
                .measure(&text[..i], &style, f32::INFINITY)
                .width;
            offsets.push((i, x));
        }
        // End-of-text position.
        let end_x = ctx.measurer.measure(text, &style, f32::INFINITY).width;
        offsets.push((text.len(), end_x));

        LayoutBox::leaf(w, h).with_widget_id(self.id)
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

    #[expect(
        clippy::string_slice,
        reason = "selection bounds always on char boundaries"
    )]
    fn paint(&self, ctx: &mut DrawCtx<'_>) {
        let focused = ctx.is_interaction_focused();
        let bounds = ctx.bounds;
        let s = &self.style;
        let text = self.editing.text();

        // Background + border.
        let bg = if self.disabled { s.disabled_bg } else { s.bg };
        let hovered = ctx.is_hot();
        let border_color = if focused {
            s.focus_border_color
        } else if hovered {
            s.hover_border_color
        } else {
            s.border_color
        };

        // Layer captures the input bg for subpixel text compositing.
        ctx.scene.push_layer_bg(bg);

        let bg_style = RectStyle::filled(bg)
            .with_border(s.border_width, border_color)
            .with_radius(s.corner_radius);
        ctx.scene.push_quad(bounds, bg_style);

        // Clip to inner area.
        let inner = bounds.inset(s.padding);
        ctx.scene.push_clip(inner);

        let style = self.text_style();

        if text.is_empty() {
            // Placeholder.
            if !self.placeholder.is_empty() {
                let shaped = ctx.measurer.shape(&self.placeholder, &style, inner.width());
                let y = inner.y() + (inner.height() - shaped.height) / 2.0;
                ctx.scene
                    .push_text(Point::new(inner.x(), y), shaped, s.placeholder_color);
            }
        } else {
            // Selection highlight.
            if let Some((sel_start, sel_end)) = self.editing.selection_range() {
                if sel_start != sel_end {
                    let prefix_w = ctx
                        .measurer
                        .measure(&text[..sel_start], &style, f32::INFINITY)
                        .width;
                    let sel_w = ctx
                        .measurer
                        .measure(&text[sel_start..sel_end], &style, f32::INFINITY)
                        .width;
                    let sel_rect =
                        Rect::new(inner.x() + prefix_w, inner.y(), sel_w, inner.height());
                    ctx.scene
                        .push_quad(sel_rect, RectStyle::filled(s.selection_color));
                }
            }

            // Text.
            let shaped = ctx.measurer.shape(text, &style, f32::INFINITY);
            let fg = if self.disabled { s.disabled_fg } else { s.fg };
            let y = inner.y() + (inner.height() - shaped.height) / 2.0;
            ctx.scene.push_text(Point::new(inner.x(), y), shaped, fg);
        }

        // Cursor (only when focused).
        if focused && !self.disabled {
            let cursor_x = inner.x() + self.cursor_x(ctx.measurer);
            let cursor_rect = Rect::new(cursor_x, inner.y(), s.cursor_width, inner.height());
            ctx.scene
                .push_quad(cursor_rect, RectStyle::filled(s.cursor_color));
        }

        ctx.scene.pop_clip();
        ctx.scene.pop_layer_bg();

        // Signal continued redraws while the animator is transitioning.
        if self.animator.is_animating() {
            ctx.request_anim_frame();
        }
    }

    fn on_input(&mut self, event: &InputEvent, bounds: Rect) -> OnInputResult {
        if self.disabled {
            return OnInputResult::ignored();
        }
        match event {
            InputEvent::MouseDown {
                pos,
                button: MouseButton::Left,
                ..
            } => self.handle_click(*pos, bounds),
            InputEvent::KeyDown { key, modifiers } => self.handle_key_input(*key, *modifiers),
            _ => OnInputResult::ignored(),
        }
    }
}

impl TextInputWidget {
    /// Handles a left-click: positions cursor at the closest character boundary.
    fn handle_click(&mut self, pos: Point, bounds: Rect) -> OnInputResult {
        let inner = bounds.inset(self.style.padding);
        let rel_x = (pos.x - inner.x()).max(0.0);

        let offsets = self.char_offsets.borrow();
        let mut best_pos = 0;
        let mut best_dist = f32::MAX;
        for &(byte_pos, x_offset) in offsets.iter() {
            let dist = (x_offset - rel_x).abs();
            if dist < best_dist {
                best_dist = dist;
                best_pos = byte_pos;
            }
        }

        self.editing.set_cursor(best_pos);
        OnInputResult::handled()
    }

    /// Handles keyboard input: editing, navigation, and selection.
    fn handle_key_input(&mut self, key: Key, modifiers: Modifiers) -> OnInputResult {
        let shift = modifiers.shift();
        let ctrl = modifiers.ctrl();

        match key {
            Key::Character(ch) => self.handle_character(ch, ctrl),
            Key::Backspace => self.handle_backspace(),
            Key::Delete => self.handle_delete(),
            Key::ArrowLeft => {
                self.editing.move_left(shift);
                OnInputResult::handled()
            }
            Key::ArrowRight => {
                self.editing.move_right(shift);
                OnInputResult::handled()
            }
            Key::Home => {
                self.editing.home(shift);
                OnInputResult::handled()
            }
            Key::End => {
                self.editing.end(shift);
                OnInputResult::handled()
            }
            _ => OnInputResult::ignored(),
        }
    }

    /// Handles a character insertion (or Ctrl+A).
    fn handle_character(&mut self, ch: char, ctrl: bool) -> OnInputResult {
        if ctrl {
            if ch == 'a' {
                self.editing.select_all();
                return OnInputResult::handled();
            }
            return OnInputResult::ignored();
        }
        self.editing.insert_char(ch);
        OnInputResult::handled().with_action(WidgetAction::TextChanged {
            id: self.id,
            text: self.editing.text().to_owned(),
        })
    }

    /// Handles Backspace: delete selection or previous character.
    fn handle_backspace(&mut self) -> OnInputResult {
        if self.editing.backspace() {
            return self.text_changed_result();
        }
        OnInputResult::handled()
    }

    /// Handles Delete: delete selection or next character.
    fn handle_delete(&mut self) -> OnInputResult {
        if self.editing.delete() {
            return self.text_changed_result();
        }
        OnInputResult::handled()
    }

    /// Returns a handled result with `TextChanged` action.
    fn text_changed_result(&self) -> OnInputResult {
        OnInputResult::handled().with_action(WidgetAction::TextChanged {
            id: self.id,
            text: self.editing.text().to_owned(),
        })
    }
}
