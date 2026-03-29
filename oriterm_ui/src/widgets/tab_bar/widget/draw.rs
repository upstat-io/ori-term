//! Tab bar drawing implementation.
//!
//! Contains tab backgrounds, title text, close buttons, dragged tab overlay,
//! and the [`Widget`] trait impl. Separators, action buttons, and free
//! functions live in [`draw_helpers`](super::draw_helpers).

use crate::animation::{AnimProperty, Lerp};
use crate::color::Color;
use crate::draw::RectStyle;
use crate::geometry::{Point, Rect};
use crate::icons::IconId;
use crate::layout::LayoutBox;
use crate::sense::Sense;
use crate::text::{TextOverflow, TextStyle};

use super::super::constants::{
    CLOSE_BUTTON_RIGHT_PAD, CLOSE_BUTTON_WIDTH, ICON_TEXT_GAP, TAB_BAR_BORDER_BOTTOM,
};
use super::super::hit::TabBarHit;
use super::draw_helpers::{bell_phase, draw_icon};
use super::{TabBarWidget, TabEntry, TabIcon};

use crate::widgets::{DrawCtx, LayoutCtx, Widget};

// Drawing constants (logical pixels).

/// Inset from close button edges to the × icon area.
pub(super) const CLOSE_ICON_INSET: f32 = 7.0;

/// Tab strip geometry and per-tab draw state passed to drawing helpers.
pub(super) struct TabStrip {
    /// Y coordinate of the tab tops (after top margin).
    pub(super) y: f32,
    /// Height of each tab (bar height minus top margin).
    pub(super) h: f32,
    /// Whether the current tab being drawn is the active tab.
    pub(super) active: bool,
    /// Bell animation phase for the current tab (0.0 if none).
    pub(super) bell: f32,
    /// Title/icon text color for the current tab.
    pub(super) text_color: Color,
}

// Drawing helpers

impl TabBarWidget {
    /// Draws a single tab (background, title text, close button).
    ///
    /// Per-tab state (`active`, `bell` phase) is carried in `strip` to keep
    /// the argument count within clippy's limit.
    fn draw_tab(&self, ctx: &mut DrawCtx<'_>, index: usize, strip: &TabStrip) {
        let tab = &self.tabs[index];
        let x = self.layout.tab_x(index) + self.anim_offset(index);
        let tab_rect = Rect::new(x, strip.y, self.layout.tab_width_at(index), strip.h);
        let bg = self.tab_background_color(index, strip);

        // Flat tab background (active distinction comes from accent bar in 01.3).
        let style = RectStyle::filled(bg);

        // Width multiplier — content fades in faster than width expands.
        let width_t = self
            .width_multipliers
            .get(index)
            .map_or(1.0, AnimProperty::get);
        let content_opacity = (width_t * 2.0).min(1.0);

        // Clip tab content horizontally (prevents overflow into adjacent tabs).
        // No vertical clip — emoji may exceed font metrics and extend below
        // the tab strip. The tab bar background quad provides the visual bound.
        let clip_rect = Rect::new(x, 0.0, self.layout.tab_width_at(index), f32::MAX);
        ctx.scene.push_clip(clip_rect);
        ctx.scene.push_layer_bg(bg);
        ctx.scene.push_quad(tab_rect, style);

        // Active tab decorations: accent bar (top) and bottom bleed.
        if strip.active {
            let y0 = ctx.bounds.y();
            // 2px accent bar at the very top of the tab bar.
            let accent = Rect::new(tab_rect.x(), y0, tab_rect.width(), TAB_BAR_BORDER_BOTTOM);
            ctx.scene
                .push_quad(accent, RectStyle::filled(self.colors.accent_bar));
            // 2px bleed that erases the bottom border under the active tab.
            let bleed_y = y0 + self.metrics.height - TAB_BAR_BORDER_BOTTOM;
            let bleed = Rect::new(
                tab_rect.x(),
                bleed_y,
                tab_rect.width(),
                TAB_BAR_BORDER_BOTTOM,
            );
            ctx.scene
                .push_quad(bleed, RectStyle::filled(self.colors.active_bg));
        }

        // Only draw content when somewhat visible.
        if content_opacity > 0.01 {
            self.draw_tab_label(ctx, tab, x, strip, self.editing_index == Some(index));

            let hovered = self.is_tab_hovered(index);

            // Modified dot: shown on non-active, non-hovered tabs.
            if tab.modified && !strip.active && !hovered {
                self.draw_modified_dot(ctx, index, x, strip);
            } else if strip.active {
                // Active tab: close at 0.6 opacity (1.0 when hovered).
                let base = if hovered { 1.0 } else { 0.6 };
                self.draw_close_button(ctx, index, x, strip, base * content_opacity);
            } else {
                // Inactive tab: animated fade (modified dot replaced by close on hover).
                let opacity = self
                    .close_btn_opacity
                    .get(index)
                    .map_or(0.0, AnimProperty::get);
                let opacity = opacity * content_opacity;
                if opacity > 0.01 {
                    self.draw_close_button(ctx, index, x, strip, opacity);
                }
            }
        }

        ctx.scene.pop_layer_bg();
        ctx.scene.pop_clip();
    }

    /// Resolves the background color for a tab.
    ///
    /// Priority: active bg > bell pulse > animated hover blend over inactive bg.
    fn tab_background_color(&self, index: usize, strip: &TabStrip) -> Color {
        if strip.active {
            self.colors.active_bg
        } else if strip.bell > 0.0 {
            self.colors.bell_pulse(strip.bell)
        } else {
            let hover_t = self
                .hover_progress
                .get(index)
                .map_or(0.0, AnimProperty::get);
            Color::lerp(self.colors.inactive_bg, self.colors.tab_hover_bg, hover_t)
        }
    }

    /// Draws the icon (if any) and title text for a tab at the given X position.
    ///
    /// When `editing` is true, uses the editing buffer text instead of the
    /// tab title and draws cursor + selection highlight. Icon rendering is
    /// always the same regardless of editing state.
    #[expect(
        clippy::too_many_arguments,
        reason = "tab label draw: self + ctx + tab + x + strip + editing"
    )]
    pub(super) fn draw_tab_label(
        &self,
        ctx: &mut DrawCtx<'_>,
        tab: &TabEntry,
        x: f32,
        strip: &TabStrip,
        editing: bool,
    ) {
        let color = strip.text_color;

        // Icon rendering: shape and draw emoji before the title.
        // Emoji fallback is injected into UI font collections from the terminal
        // font at renderer init, so emoji renders at the correct UI text size.
        // Emoji sized ~30% larger than text for visual prominence.
        let text_offset = if let Some(TabIcon::Emoji(ref emoji)) = tab.icon {
            let icon_size = ctx.theme.font_size_small * 1.5;
            let icon_style = TextStyle::new(icon_size, color);
            let icon_shaped = ctx.measurer.shape(emoji, &icon_style, f32::INFINITY);
            // Use the icon height as width — color emoji are square bitmaps
            // but the font advance is often narrower than the actual bitmap.
            let icon_extent = icon_shaped.height;
            let icon_x = x + self.metrics.tab_padding;
            let icon_y = strip.y + (strip.h - icon_extent) / 2.0;
            ctx.scene
                .push_text(Point::new(icon_x, icon_y), icon_shaped, color);
            icon_extent + ICON_TEXT_GAP
        } else {
            0.0
        };

        // Title text: use editing buffer when actively editing.
        let edit_text;
        let title = if editing {
            edit_text = self.editing.text().to_owned();
            if edit_text.is_empty() {
                "Terminal"
            } else {
                &edit_text
            }
        } else if tab.title.is_empty() {
            "Terminal"
        } else {
            &tab.title
        };
        let max_w = (self.layout.max_text_width() - text_offset).max(0.0);

        // Measure for cursor/selection before consuming text_style with overflow.
        let text_style = TextStyle::new(ctx.theme.font_size_small, color);
        let text_x = x + self.metrics.tab_padding + text_offset;

        // Measure full text for height (before consuming text_style).
        let full_shaped = ctx.measurer.shape(title, &text_style, f32::INFINITY);
        let text_y = strip.y + (strip.h - full_shaped.height) / 2.0;

        // Draw editing overlay (selection + cursor) behind text.
        if editing {
            super::edit_draw::draw_editing_overlay(
                ctx,
                &super::edit_draw::EditOverlayParams {
                    editing: &self.editing,
                    text_style: &text_style,
                    text_x,
                    text_y,
                    max_w,
                    line_h: full_shaped.height,
                },
            );
        }

        // Draw clamped text.
        let clamped = ctx.measurer.shape(
            title,
            &text_style.with_overflow(TextOverflow::Ellipsis),
            max_w,
        );
        ctx.scene
            .push_text(Point::new(text_x, text_y), clamped, color);
    }

    /// Draws a 6px accent-colored square dot indicating a modified tab.
    ///
    /// Positioned in the close button zone, vertically centered.
    fn draw_modified_dot(&self, ctx: &mut DrawCtx<'_>, index: usize, tab_x: f32, strip: &TabStrip) {
        let dot_size = 6.0_f32;
        let cx = tab_x + self.layout.tab_width_at(index)
            - CLOSE_BUTTON_RIGHT_PAD
            - CLOSE_BUTTON_WIDTH / 2.0
            - dot_size / 2.0;
        let cy = strip.y + (strip.h - dot_size) / 2.0;
        let dot = Rect::new(cx, cy, dot_size, dot_size);
        ctx.scene
            .push_quad(dot, RectStyle::filled(self.colors.accent_bar));
    }

    /// Draws the close (×) button for a tab with the given opacity.
    #[expect(
        clippy::too_many_arguments,
        reason = "close button draw: self + ctx + index + tab_x + strip + opacity"
    )]
    fn draw_close_button(
        &self,
        ctx: &mut DrawCtx<'_>,
        index: usize,
        tab_x: f32,
        strip: &TabStrip,
        opacity: f32,
    ) {
        let cx =
            tab_x + self.layout.tab_width_at(index) - CLOSE_BUTTON_RIGHT_PAD - CLOSE_BUTTON_WIDTH;
        let cy = strip.y + (strip.h - CLOSE_BUTTON_WIDTH) / 2.0;
        let btn = Rect::new(cx, cy, CLOSE_BUTTON_WIDTH, CLOSE_BUTTON_WIDTH);

        // Flat hover highlight on the close button.
        if self.hover_hit == TabBarHit::CloseTab(index) {
            let style = RectStyle::filled(self.colors.button_hover_bg.with_alpha(opacity));
            ctx.scene.push_quad(btn, style);
        }

        // × icon.
        let fg = self.colors.close_fg.with_alpha(opacity);
        let inset = CLOSE_ICON_INSET;
        let icon_size = (CLOSE_BUTTON_WIDTH - 2.0 * inset).round() as u32;
        let icon_rect = Rect::new(
            cx + inset,
            cy + inset,
            CLOSE_BUTTON_WIDTH - 2.0 * inset,
            CLOSE_BUTTON_WIDTH - 2.0 * inset,
        );
        draw_icon(ctx, IconId::Close, icon_rect, icon_size, fg);
    }

    /// Draws inactive tabs, active tab, and checks for running animations.
    ///
    /// Inactive tabs draw first (behind active). After all tabs, any running
    /// hover/close/width/bell animations request continued redraws.
    fn draw_all_tabs(&self, ctx: &mut DrawCtx<'_>, strip: &mut TabStrip) {
        // Inactive tabs (drawn first, behind active tab).
        for i in 0..self.tabs.len() {
            if i == self.active_index || self.is_dragged(i) {
                continue;
            }
            strip.active = false;
            strip.bell = bell_phase(&self.tabs[i], ctx.now);
            strip.text_color = self.colors.inactive_text;
            self.draw_tab(ctx, i, strip);

            if strip.bell > 0.0 {
                ctx.request_anim_frame();
            }
        }

        // Active tab (drawn on top of inactive tabs).
        if self.active_index < self.tabs.len() && !self.is_dragged(self.active_index) {
            strip.active = true;
            strip.bell = 0.0;
            strip.text_color = self.colors.text_fg;
            self.draw_tab(ctx, self.active_index, strip);
        }

        // Request continued redraws if any animation is running.
        let hover_animating = self.hover_progress.iter().any(AnimProperty::is_animating);
        let close_animating = self
            .close_btn_opacity
            .iter()
            .any(AnimProperty::is_animating);
        let width_animating = self.has_width_animation();
        if hover_animating || close_animating || width_animating {
            ctx.request_anim_frame();
        }
    }
}

// Public drag overlay drawing (overlay tier — on top of all chrome text)

impl TabBarWidget {
    /// Draw the dragged tab overlay into the given draw context.
    ///
    /// Called separately from [`Widget::draw`] so the overlay can be rendered
    /// in the overlay tier (draws 10–13), ON TOP of all chrome text. Without
    /// this separation, regular tab text from the chrome tier (draw 7) would
    /// paint over the dragged tab's background (draw 6).
    pub fn draw_drag_overlay(&self, ctx: &mut DrawCtx<'_>) {
        let y0 = ctx.bounds.y();
        let strip = TabStrip {
            y: y0 + self.metrics.top_margin,
            h: self.metrics.height - self.metrics.top_margin,
            active: false,
            bell: 0.0,
            text_color: self.colors.text_fg,
        };
        self.draw_dragged_tab_overlay(ctx, &strip);
    }
}

// Widget impl

impl Widget for TabBarWidget {
    fn id(&self) -> crate::widget_id::WidgetId {
        self.id
    }

    fn is_focusable(&self) -> bool {
        false
    }

    fn layout(&self, _ctx: &LayoutCtx<'_>) -> LayoutBox {
        LayoutBox::leaf(self.window_width, self.metrics.height).with_widget_id(self.id)
    }

    fn sense(&self) -> Sense {
        Sense::click()
    }

    fn prepaint(&mut self, _ctx: &mut crate::widgets::PrepaintCtx<'_>) {
        self.tick_animations();
    }

    fn paint(&self, ctx: &mut DrawCtx<'_>) {
        if self.tabs.is_empty() {
            return;
        }

        let y0 = ctx.bounds.y();
        let w = ctx.bounds.width();

        // 1. Tab bar background.
        let bar = Rect::new(0.0, y0, w, self.metrics.height);
        ctx.scene
            .push_quad(bar, RectStyle::filled(self.colors.bar_bg));

        // 1.5. Bottom border (drawn behind tabs; active tab bleed erases it).
        let border_y = y0 + self.metrics.height - TAB_BAR_BORDER_BOTTOM;
        let border_rect = Rect::new(0.0, border_y, w, TAB_BAR_BORDER_BOTTOM);
        ctx.scene
            .push_quad(border_rect, RectStyle::filled(self.colors.bar_border));

        let mut strip = TabStrip {
            y: y0 + self.metrics.top_margin,
            h: self.metrics.height - self.metrics.top_margin,
            active: false,
            bell: 0.0,
            text_color: self.colors.inactive_text,
        };

        // 2. Inactive tabs, 3. Active tab, 3.5. Animation checks.
        self.draw_all_tabs(ctx, &mut strip);

        // 4. Separators, 5. New tab, 6. Dropdown, 6.5. Window controls.
        self.draw_separators(ctx, &strip);
        self.draw_new_tab_button(ctx, &strip);
        self.draw_dropdown_button(ctx, &strip);
        #[cfg(not(target_os = "macos"))]
        self.draw_window_controls(ctx);
    }
}
