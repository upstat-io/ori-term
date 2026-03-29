//! Shared text editing state for single-line text fields.
//!
//! Owns the text content, cursor position, and selection anchor.
//! Used by both [`TextInputWidget`](crate::widgets::text_input::TextInputWidget)
//! and sidebar search to avoid duplicating editing logic.

/// Editing state for a single-line text field.
///
/// Manages text content, cursor byte position, and optional selection.
/// All cursor/anchor values are byte offsets on character boundaries.
#[derive(Debug, Clone)]
pub struct TextEditingState {
    text: String,
    cursor: usize,
    selection_anchor: Option<usize>,
}

impl Default for TextEditingState {
    fn default() -> Self {
        Self::new()
    }
}

impl TextEditingState {
    /// Creates an empty editing state with cursor at position 0.
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

    /// Returns the cursor byte position.
    pub fn cursor(&self) -> usize {
        self.cursor
    }

    /// Returns the selection anchor, if any.
    pub fn selection_anchor(&self) -> Option<usize> {
        self.selection_anchor
    }

    /// Sets the text content, moving cursor to end and clearing selection.
    pub fn set_text(&mut self, text: impl Into<String>) {
        self.text = text.into();
        self.cursor = self.text.len();
        self.selection_anchor = None;
    }

    /// Returns the selected byte range as `(start, end)`, if any.
    pub fn selection_range(&self) -> Option<(usize, usize)> {
        self.selection_anchor.map(|anchor| {
            let start = anchor.min(self.cursor);
            let end = anchor.max(self.cursor);
            (start, end)
        })
    }

    /// Inserts a character at the cursor, replacing any selection.
    ///
    /// Returns `true` (content always changes on insert).
    pub fn insert_char(&mut self, ch: char) -> bool {
        self.delete_selection();
        self.text.insert(self.cursor, ch);
        self.cursor += ch.len_utf8();
        true
    }

    /// Deletes the character before the cursor (or the selection).
    ///
    /// Returns `true` if content changed.
    pub fn backspace(&mut self) -> bool {
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

    /// Deletes the character after the cursor (or the selection).
    ///
    /// Returns `true` if content changed.
    pub fn delete(&mut self) -> bool {
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

    /// Moves cursor to position 0, with optional shift-selection.
    pub fn home(&mut self, shift: bool) {
        if shift && self.selection_anchor.is_none() {
            self.selection_anchor = Some(self.cursor);
        }
        self.cursor = 0;
        if !shift {
            self.selection_anchor = None;
        }
    }

    /// Moves cursor to end of text, with optional shift-selection.
    pub fn end(&mut self, shift: bool) {
        if shift && self.selection_anchor.is_none() {
            self.selection_anchor = Some(self.cursor);
        }
        self.cursor = self.text.len();
        if !shift {
            self.selection_anchor = None;
        }
    }

    /// Selects all text (anchor at 0, cursor at end).
    pub fn select_all(&mut self) {
        if self.text.is_empty() {
            return;
        }
        self.selection_anchor = Some(0);
        self.cursor = self.text.len();
    }

    /// Deletes the currently selected text, placing cursor at selection start.
    ///
    /// Returns `true` if text was deleted.
    pub fn delete_selection(&mut self) -> bool {
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

    /// Returns the byte offset of the next char boundary after `pos`.
    pub fn next_char_boundary(&self, pos: usize) -> usize {
        let mut idx = pos + 1;
        while idx < self.text.len() && !self.text.is_char_boundary(idx) {
            idx += 1;
        }
        idx.min(self.text.len())
    }

    /// Returns the byte offset of the previous char boundary before `pos`.
    pub fn prev_char_boundary(&self, pos: usize) -> usize {
        if pos == 0 {
            return 0;
        }
        let mut idx = pos - 1;
        while idx > 0 && !self.text.is_char_boundary(idx) {
            idx -= 1;
        }
        idx
    }

    /// Moves cursor left, handling shift for selection.
    pub fn move_left(&mut self, shift: bool) {
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

    /// Moves cursor right, handling shift for selection.
    pub fn move_right(&mut self, shift: bool) {
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

    /// Sets cursor position directly (clamped to text length), clearing selection.
    pub fn set_cursor(&mut self, pos: usize) {
        self.cursor = pos.min(self.text.len());
        self.selection_anchor = None;
    }

    /// Sets cursor and selection anchor directly for programmatic selection.
    pub fn set_selection(&mut self, anchor: usize, cursor: usize) {
        self.selection_anchor = Some(anchor.min(self.text.len()));
        self.cursor = cursor.min(self.text.len());
    }
}

#[cfg(test)]
mod tests;
