//! Text editing controller — cursor movement, selection, character input.
//!
//! Owns the editable text state (content, cursor, selection) and handles all
//! keyboard editing operations. Emits `WidgetAction::TextChanged` on content
//! modifications. Designed for `TextInputWidget` (Section 08.3 migration).

use crate::action::WidgetAction;
use crate::input::{InputEvent, Key};

use super::{ControllerCtx, ControllerRequests, EventController};

/// Single-line text editing controller.
///
/// Handles cursor movement (Left/Right/Home/End), text selection
/// (Shift+arrow), character input, Backspace/Delete, and Ctrl+A select all.
/// Clipboard operations (Ctrl+C/V/X) are deferred to the app layer.
#[derive(Debug, Clone)]
pub struct TextEditController {
    /// The text content being edited.
    text: String,
    /// Byte offset of the cursor within `text`.
    cursor: usize,
    /// Byte offset of the selection anchor, if a selection is active.
    selection_anchor: Option<usize>,
}

impl TextEditController {
    /// Creates a new text editing controller with empty content.
    pub fn new() -> Self {
        Self {
            text: String::new(),
            cursor: 0,
            selection_anchor: None,
        }
    }

    /// Returns the current text content.
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Sets the text content, moving the cursor to the end.
    pub fn set_text(&mut self, text: impl Into<String>) {
        self.text = text.into();
        self.cursor = self.text.len();
        self.selection_anchor = None;
    }

    /// Returns the cursor byte position.
    pub fn cursor(&self) -> usize {
        self.cursor
    }

    /// Returns the selection range as `(start, end)`, if any.
    pub fn selection_range(&self) -> Option<(usize, usize)> {
        self.selection_anchor.map(|anchor| {
            let start = anchor.min(self.cursor);
            let end = anchor.max(self.cursor);
            (start, end)
        })
    }

    /// Returns the byte offset of the next char boundary after `pos`.
    fn next_char_boundary(&self, pos: usize) -> usize {
        let mut idx = pos + 1;
        while idx < self.text.len() && !self.text.is_char_boundary(idx) {
            idx += 1;
        }
        idx.min(self.text.len())
    }

    /// Returns the byte offset of the previous char boundary before `pos`.
    fn prev_char_boundary(&self, pos: usize) -> usize {
        if pos == 0 {
            return 0;
        }
        let mut idx = pos - 1;
        while idx > 0 && !self.text.is_char_boundary(idx) {
            idx -= 1;
        }
        idx
    }

    /// Deletes the selected text. Returns `true` if text was deleted.
    fn delete_selection(&mut self) -> bool {
        if let Some((start, end)) = self.selection_range() {
            if start != end {
                self.text.drain(start..end);
                self.cursor = start;
                self.selection_anchor = None;
                return true;
            }
        }
        self.selection_anchor = None;
        false
    }

    /// Moves cursor left, extending selection if `shift` is held.
    fn move_left(&mut self, shift: bool) {
        if shift {
            if self.selection_anchor.is_none() {
                self.selection_anchor = Some(self.cursor);
            }
            self.cursor = self.prev_char_boundary(self.cursor);
        } else {
            match self.selection_range() {
                Some((start, end)) if start != end => self.cursor = start,
                _ => self.cursor = self.prev_char_boundary(self.cursor),
            }
            self.selection_anchor = None;
        }
    }

    /// Moves cursor right, extending selection if `shift` is held.
    fn move_right(&mut self, shift: bool) {
        if shift {
            if self.selection_anchor.is_none() {
                self.selection_anchor = Some(self.cursor);
            }
            if self.cursor < self.text.len() {
                self.cursor = self.next_char_boundary(self.cursor);
            }
        } else {
            match self.selection_range() {
                Some((start, end)) if start != end => self.cursor = end,
                _ => {
                    if self.cursor < self.text.len() {
                        self.cursor = self.next_char_boundary(self.cursor);
                    }
                }
            }
            self.selection_anchor = None;
        }
    }

    /// Moves cursor to `target`, extending selection if `shift` is held.
    fn move_to(&mut self, target: usize, shift: bool) {
        if shift && self.selection_anchor.is_none() {
            self.selection_anchor = Some(self.cursor);
        }
        self.cursor = target;
        if !shift {
            self.selection_anchor = None;
        }
    }

    /// Inserts a character at the cursor, deleting any selection first.
    fn insert_char(&mut self, ch: char) {
        self.delete_selection();
        self.text.insert(self.cursor, ch);
        self.cursor += ch.len_utf8();
    }

    /// Handles Backspace: deletes selection or previous character.
    /// Returns `true` if text was modified.
    fn handle_backspace(&mut self) -> bool {
        if self.delete_selection() {
            return true;
        }
        if self.cursor > 0 {
            let prev = self.prev_char_boundary(self.cursor);
            self.text.drain(prev..self.cursor);
            self.cursor = prev;
            return true;
        }
        false
    }

    /// Handles Delete: deletes selection or next character.
    /// Returns `true` if text was modified.
    fn handle_delete(&mut self) -> bool {
        if self.delete_selection() {
            return true;
        }
        if self.cursor < self.text.len() {
            let next = self.next_char_boundary(self.cursor);
            self.text.drain(self.cursor..next);
            return true;
        }
        false
    }

    /// Emits a `TextChanged` action via the controller context.
    fn emit_text_changed(&self, ctx: &mut ControllerCtx<'_>) {
        ctx.emit_action(WidgetAction::TextChanged {
            id: ctx.widget_id,
            text: self.text.clone(),
        });
    }
}

impl Default for TextEditController {
    fn default() -> Self {
        Self::new()
    }
}

impl EventController for TextEditController {
    fn handle_event(&mut self, event: &InputEvent, ctx: &mut ControllerCtx<'_>) -> bool {
        match event {
            InputEvent::KeyDown { key, modifiers } => {
                let shift = modifiers.shift();
                let ctrl = modifiers.ctrl();

                match key {
                    Key::Character(ch) => {
                        if ctrl {
                            if *ch == 'a' {
                                // Select all.
                                self.selection_anchor = Some(0);
                                self.cursor = self.text.len();
                                ctx.requests.insert(ControllerRequests::PAINT);
                                return true;
                            }
                            // Ctrl+C/V/X deferred to app layer.
                            return false;
                        }
                        self.insert_char(*ch);
                        self.emit_text_changed(ctx);
                        ctx.requests.insert(ControllerRequests::PAINT);
                        true
                    }
                    Key::Backspace => {
                        if self.handle_backspace() {
                            self.emit_text_changed(ctx);
                        }
                        ctx.requests.insert(ControllerRequests::PAINT);
                        true
                    }
                    Key::Delete => {
                        if self.handle_delete() {
                            self.emit_text_changed(ctx);
                        }
                        ctx.requests.insert(ControllerRequests::PAINT);
                        true
                    }
                    Key::ArrowLeft => {
                        self.move_left(shift);
                        ctx.requests.insert(ControllerRequests::PAINT);
                        true
                    }
                    Key::ArrowRight => {
                        self.move_right(shift);
                        ctx.requests.insert(ControllerRequests::PAINT);
                        true
                    }
                    Key::Home => {
                        self.move_to(0, shift);
                        ctx.requests.insert(ControllerRequests::PAINT);
                        true
                    }
                    Key::End => {
                        self.move_to(self.text.len(), shift);
                        ctx.requests.insert(ControllerRequests::PAINT);
                        true
                    }
                    _ => false,
                }
            }
            // Consume KeyUp for handled keys to prevent leaking.
            InputEvent::KeyUp { key, .. } => matches!(
                key,
                Key::Backspace
                    | Key::Delete
                    | Key::ArrowLeft
                    | Key::ArrowRight
                    | Key::Home
                    | Key::End
            ),
            _ => false,
        }
    }

    fn reset(&mut self) {
        self.selection_anchor = None;
    }
}

#[cfg(test)]
mod tests;
