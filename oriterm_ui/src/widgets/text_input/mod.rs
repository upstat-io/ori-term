//! Text input widget — single-line text field with cursor and selection.
//!
//! Handles keyboard editing (character input, backspace, delete, arrow
//! navigation, Home/End, Ctrl+A select all). Emits `WidgetAction::TextChanged`
//! on content changes.
//!
//! Clipboard operations are deferred — the widget emits actions that the
//! app layer interprets for actual clipboard I/O.

mod widget_impl;

use std::cell::RefCell;

use crate::color::Color;
use crate::controllers::{ClickController, EventController, FocusController, HoverController};
use crate::geometry::Insets;
use crate::text::TextStyle;
use crate::text::editing::TextEditingState;
use crate::theme::UiTheme;
use crate::visual_state::focus_states;
use crate::visual_state::transition::VisualStateAnimator;
use crate::widget_id::WidgetId;

/// Visual style for a [`TextInputWidget`].
#[derive(Debug, Clone, PartialEq)]
pub struct TextInputStyle {
    /// Text color.
    pub fg: Color,
    /// Background color.
    pub bg: Color,
    /// Border color.
    pub border_color: Color,
    /// Border color on hover.
    pub hover_border_color: Color,
    /// Border color when focused.
    pub focus_border_color: Color,
    /// Border width.
    pub border_width: f32,
    /// Corner radius.
    pub corner_radius: f32,
    /// Inner padding.
    pub padding: Insets,
    /// Font size in logical pixels.
    pub font_size: f32,
    /// Placeholder text color.
    pub placeholder_color: Color,
    /// Cursor color.
    pub cursor_color: Color,
    /// Cursor width in pixels.
    pub cursor_width: f32,
    /// Selection highlight color.
    pub selection_color: Color,
    /// Minimum width.
    pub min_width: f32,
    /// Disabled text color.
    pub disabled_fg: Color,
    /// Disabled background.
    pub disabled_bg: Color,
}

impl TextInputStyle {
    /// Derives a text input style from the given theme.
    pub fn from_theme(theme: &UiTheme) -> Self {
        Self {
            fg: theme.fg_primary,
            bg: theme.bg_input,
            border_color: theme.border,
            hover_border_color: theme.fg_faint,
            focus_border_color: theme.accent,
            border_width: 1.0,
            corner_radius: theme.corner_radius,
            padding: Insets::vh(6.0, 8.0),
            font_size: theme.font_size,
            placeholder_color: theme.fg_disabled,
            cursor_color: theme.fg_primary,
            cursor_width: 1.5,
            selection_color: theme.accent.with_alpha(0.3),
            min_width: 120.0,
            disabled_fg: theme.fg_disabled,
            disabled_bg: theme.bg_secondary,
        }
    }

    /// Settings-panel text input style: 2px border, 12px font, 200px min-width.
    ///
    /// Matches mockup `input[type="text"]` in settings: `border: 2px`,
    /// `font-size: 12px`, `padding: 6px 10px`, `width: 200px`.
    pub fn settings(theme: &UiTheme) -> Self {
        Self {
            border_width: 2.0,
            font_size: 12.0,
            padding: Insets::vh(6.0, 10.0),
            min_width: 200.0,
            ..Self::from_theme(theme)
        }
    }
}

impl Default for TextInputStyle {
    fn default() -> Self {
        Self::from_theme(&UiTheme::dark())
    }
}

/// A single-line text input field.
///
/// Manages text content, cursor position, and selection. Keyboard
/// editing is handled internally; `WidgetAction::TextChanged` is
/// emitted when content changes. Border color transitions between
/// unfocused and focused states are handled by [`VisualStateAnimator`]
/// with `focus_states()`.
pub struct TextInputWidget {
    pub(super) id: WidgetId,
    pub(super) editing: TextEditingState,
    pub(super) placeholder: String,
    pub(super) disabled: bool,
    pub(super) style: TextInputStyle,
    pub(super) controllers: Vec<Box<dyn EventController>>,
    pub(super) animator: VisualStateAnimator,
    /// Cached character boundary X-offsets from last layout.
    ///
    /// Each entry is `(byte_position, x_offset)`. Populated during `layout()`
    /// (which has access to the text measurer) and read during `on_input()`
    /// for click-to-cursor mapping.
    pub(super) char_offsets: RefCell<Vec<(usize, f32)>>,
}

impl Default for TextInputWidget {
    fn default() -> Self {
        Self::new()
    }
}

impl TextInputWidget {
    /// Creates an empty text input.
    pub fn new() -> Self {
        let style = TextInputStyle::default();
        Self {
            id: WidgetId::next(),
            editing: TextEditingState::new(),
            placeholder: String::new(),
            disabled: false,
            controllers: vec![
                Box::new(HoverController::new()),
                Box::new(ClickController::new()),
                Box::new(FocusController::new()),
            ],
            animator: VisualStateAnimator::new(vec![focus_states(
                style.border_color,
                style.focus_border_color,
            )]),
            style,
            char_offsets: RefCell::new(Vec::new()),
        }
    }

    /// Returns the current text content.
    pub fn text(&self) -> &str {
        self.editing.text()
    }

    /// Sets the text content programmatically.
    pub fn set_text(&mut self, text: impl Into<String>) {
        self.editing.set_text(text);
    }

    /// Returns the cursor byte position.
    pub fn cursor(&self) -> usize {
        self.editing.cursor()
    }

    /// Returns the selection anchor (start of selection), if any.
    pub fn selection_anchor(&self) -> Option<usize> {
        self.editing.selection_anchor()
    }

    /// Returns the selected text range as `(start, end)`, if any.
    pub fn selection_range(&self) -> Option<(usize, usize)> {
        self.editing.selection_range()
    }

    /// Returns whether the input is disabled.
    pub fn is_disabled(&self) -> bool {
        self.disabled
    }

    /// Sets the disabled state.
    pub fn set_disabled(&mut self, disabled: bool) {
        self.disabled = disabled;
    }

    /// Sets placeholder text.
    #[must_use]
    pub fn with_placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = placeholder.into();
        self
    }

    /// Sets the disabled state via builder.
    #[must_use]
    pub fn with_disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Sets the style.
    #[must_use]
    pub fn with_style(mut self, style: TextInputStyle) -> Self {
        self.animator = VisualStateAnimator::new(vec![focus_states(
            style.border_color,
            style.focus_border_color,
        )]);
        self.style = style;
        self
    }

    /// Builds the `TextStyle` for measurement.
    pub(super) fn text_style(&self) -> TextStyle {
        let color = if self.disabled {
            self.style.disabled_fg
        } else {
            self.style.fg
        };
        TextStyle::new(self.style.font_size, color)
    }

    /// Computes cursor X position in pixels using the measurer.
    #[expect(clippy::string_slice, reason = "cursor always on char boundary")]
    pub(super) fn cursor_x(&self, measurer: &dyn super::TextMeasurer) -> f32 {
        let cursor = self.editing.cursor();
        let prefix = &self.editing.text()[..cursor];
        let style = self.text_style();
        let metrics = measurer.measure(prefix, &style, f32::INFINITY);
        metrics.width
    }
}

impl std::fmt::Debug for TextInputWidget {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TextInputWidget")
            .field("id", &self.id)
            .field("editing", &self.editing)
            .field("placeholder", &self.placeholder)
            .field("disabled", &self.disabled)
            .field("style", &self.style)
            .field("controller_count", &self.controllers.len())
            .field("animator", &self.animator)
            .field("char_offsets_len", &self.char_offsets.borrow().len())
            .finish()
    }
}

#[cfg(test)]
mod tests;
