//! Tab bar drawing implementation.
//!
//! Contains all visual rendering logic: tab backgrounds, title text, close
//! buttons, separators, new-tab/dropdown buttons, dragged tab overlay, and
//! the bell animation phase. The [`Widget`] trait impl routes into these
//! drawing helpers.

use std::time::Instant;

use crate::animation::{AnimProperty, Lerp};
use crate::color::Color;
use crate::draw::RectStyle;
use crate::geometry::{Point, Rect};
use crate::layout::LayoutBox;
use crate::sense::Sense;
use crate::text::{TextOverflow, TextStyle};

use super::super::constants::{
    CLOSE_BUTTON_RIGHT_PAD, CLOSE_BUTTON_WIDTH, DROPDOWN_BUTTON_WIDTH, ICON_TEXT_GAP,
    NEW_TAB_BUTTON_WIDTH,
};
use super::super::hit::TabBarHit;
use super::{TabBarWidget, TabEntry, TabIcon};

use crate::icons::IconId;
use crate::widgets::{DrawCtx, LayoutCtx, Widget};

// Drawing constants (logical pixels).

/// Corner radius for the active tab's top-left and top-right corners.
pub(super) const ACTIVE_TAB_RADIUS: f32 = 8.0;

/// Corner radius for hover backgrounds on buttons and close targets.
const BUTTON_HOVER_RADIUS: f32 = 4.0;

/// Inset from close button edges to the × icon area.
pub(super) const CLOSE_ICON_INSET: f32 = 7.0;

/// Half-arm length of the + icon in the new-tab button.
const PLUS_ARM: f32 = 5.0;

/// Half-extent of the ▾ chevron in the dropdown button.
const CHEVRON_HALF: f32 = 5.0;

/// Vertical inset for separators from tab top/bottom edges.
const SEPARATOR_INSET: f32 = 8.0;

/// Duration of the bell pulse animation in seconds.
const BELL_DURATION_SECS: f32 = 3.0;

/// Frequency of the bell pulse sine wave in Hz.
const BELL_FREQUENCY_HZ: f32 = 2.0;

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

        // Active tab gets rounded top corners.
        let style = if strip.active {
            RectStyle::filled(bg).with_per_corner_radius(
                ACTIVE_TAB_RADIUS,
                ACTIVE_TAB_RADIUS,
                0.0,
                0.0,
            )
        } else {
            RectStyle::filled(bg)
        };

        // Width multiplier — content fades in faster than width expands.
        let width_t = self
            .width_multipliers
            .get(index)
            .map_or(1.0, AnimProperty::get);
        let content_opacity = (width_t * 2.0).min(1.0);

        // Clip tab content to its bounds — prevents overflow into adjacent tabs.
        ctx.scene.push_clip(tab_rect);
        ctx.scene.push_layer_bg(bg);
        ctx.scene.push_quad(tab_rect, style);

        // Only draw content when somewhat visible.
        if content_opacity > 0.01 {
            self.draw_tab_label(ctx, tab, x, strip, self.editing_index == Some(index));

            // Close button: always visible on active, animated fade on inactive.
            if strip.active {
                self.draw_close_button(ctx, index, x, strip, content_opacity);
            } else {
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
        let text_offset = if let Some(TabIcon::Emoji(ref emoji)) = tab.icon {
            let icon_style = TextStyle::new(ctx.theme.font_size_small, color);
            let icon_shaped = ctx.measurer.shape(emoji, &icon_style, f32::INFINITY);
            let icon_x = x + self.metrics.tab_padding;
            let icon_y = strip.y + (strip.h - icon_shaped.height) / 2.0;
            let icon_w = icon_shaped.width;
            ctx.scene
                .push_text(Point::new(icon_x, icon_y), icon_shaped, color);
            icon_w + ICON_TEXT_GAP
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

        // Hover highlight on the close button.
        if self.hover_hit == TabBarHit::CloseTab(index) {
            let style = RectStyle::filled(self.colors.button_hover_bg.with_alpha(opacity))
                .with_radius(BUTTON_HOVER_RADIUS);
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

    /// Draws separators between tabs with suppression rules.
    fn draw_separators(&self, ctx: &mut DrawCtx<'_>, strip: &TabStrip) {
        for i in 1..self.tabs.len() {
            // Suppress adjacent to active tab.
            if i == self.active_index || i == self.active_index + 1 {
                continue;
            }
            // Suppress adjacent to hovered tab.
            if let TabBarHit::Tab(h) | TabBarHit::CloseTab(h) = self.hover_hit {
                if i == h || i == h + 1 {
                    continue;
                }
            }
            // Suppress adjacent to dragged tab.
            if let Some((d, _)) = self.drag_visual {
                if i == d || i == d + 1 {
                    continue;
                }
            }

            let x = self.layout.tab_x(i);
            let y1 = strip.y + SEPARATOR_INSET;
            let y2 = strip.y + strip.h - SEPARATOR_INSET;
            ctx.scene.push_line(
                Point::new(x, y1),
                Point::new(x, y2),
                1.0,
                self.colors.separator,
            );
        }
    }

    /// Draws the new-tab (+) button.
    fn draw_new_tab_button(&self, ctx: &mut DrawCtx<'_>, strip: &TabStrip) {
        let bx = new_tab_button_x(self);
        let btn = Rect::new(bx, strip.y, NEW_TAB_BUTTON_WIDTH, strip.h);

        if self.hover_hit == TabBarHit::NewTab {
            let style =
                RectStyle::filled(self.colors.button_hover_bg).with_radius(BUTTON_HOVER_RADIUS);
            ctx.scene.push_quad(btn, style);
        }

        // + icon.
        let cx = bx + NEW_TAB_BUTTON_WIDTH / 2.0;
        let cy = strip.y + strip.h / 2.0;
        let fg = self.colors.close_fg;
        let icon_size = (PLUS_ARM * 2.0).round() as u32;
        let icon_rect = Rect::new(cx - PLUS_ARM, cy - PLUS_ARM, PLUS_ARM * 2.0, PLUS_ARM * 2.0);
        draw_icon(ctx, IconId::Plus, icon_rect, icon_size, fg);
    }

    /// Draws the dropdown (▾) button.
    fn draw_dropdown_button(&self, ctx: &mut DrawCtx<'_>, strip: &TabStrip) {
        let bx = dropdown_button_x(self);
        let btn = Rect::new(bx, strip.y, DROPDOWN_BUTTON_WIDTH, strip.h);

        if self.hover_hit == TabBarHit::Dropdown {
            let style =
                RectStyle::filled(self.colors.button_hover_bg).with_radius(BUTTON_HOVER_RADIUS);
            ctx.scene.push_quad(btn, style);
        }

        // ▾ chevron.
        let cx = bx + DROPDOWN_BUTTON_WIDTH / 2.0;
        let cy = strip.y + strip.h / 2.0;
        let fg = self.colors.close_fg;
        let icon_size = (CHEVRON_HALF * 2.0).round() as u32;
        let icon_rect = Rect::new(
            cx - CHEVRON_HALF,
            cy - CHEVRON_HALF,
            CHEVRON_HALF * 2.0,
            CHEVRON_HALF * 2.0,
        );
        draw_icon(ctx, IconId::ChevronDown, icon_rect, icon_size, fg);
    }
}

/// Emit a vector icon from the pre-resolved icon atlas.
///
/// No-ops when `ctx.icons` is `None` (tests) or the icon wasn't resolved.
fn draw_icon(ctx: &mut DrawCtx<'_>, id: IconId, rect: Rect, size_px: u32, color: Color) {
    if let Some(resolved) = ctx.icons.and_then(|ic| ic.get(id, size_px)) {
        ctx.scene
            .push_icon(rect, resolved.atlas_page, resolved.uv, color);
    }
}

// Free functions used by both drawing and tests

/// Computes the bell animation phase for a tab.
///
/// Returns 0.0–1.0 for an active bell animation, 0.0 otherwise.
/// The phase follows a decaying sine wave that pulses for
/// [`BELL_DURATION_SECS`] seconds after the bell fires.
pub(super) fn bell_phase(tab: &TabEntry, now: Instant) -> f32 {
    let Some(start) = tab.bell_start else {
        return 0.0;
    };
    let elapsed = now.duration_since(start).as_secs_f32();
    if elapsed >= BELL_DURATION_SECS {
        return 0.0;
    }
    let fade = 1.0 - (elapsed / BELL_DURATION_SECS);
    let wave = (elapsed * BELL_FREQUENCY_HZ * std::f32::consts::TAU)
        .sin()
        .abs();
    wave * fade
}

/// X position of the new-tab button, adjusted for drag.
///
/// When dragging a tab past the end of the strip, the button moves
/// right to stay visible: `max(default_x, drag_x + tab_width)`.
pub(super) fn new_tab_button_x(widget: &TabBarWidget) -> f32 {
    let default_x = widget.layout.new_tab_x();
    if let Some((idx, drag_x)) = widget.drag_visual {
        default_x.max(drag_x + widget.layout.tab_width_at(idx))
    } else {
        default_x
    }
}

/// X position of the dropdown button, adjusted for drag.
pub(super) fn dropdown_button_x(widget: &TabBarWidget) -> f32 {
    let default_x = widget.layout.dropdown_x();
    if let Some((idx, drag_x)) = widget.drag_visual {
        default_x.max(drag_x + widget.layout.tab_width_at(idx) + NEW_TAB_BUTTON_WIDTH)
    } else {
        default_x
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
