//! Dirty tracking with column-level damage bounds for incremental rendering.
//!
//! Tracks which visible rows have changed since the last drain, and which
//! column range within each row was affected. The GPU renderer calls
//! `drain()` each frame to discover dirty regions, rebuilds only those
//! regions' instance buffers, and the tracker resets to clean.

use std::ops::Range;

/// Per-line damage bounds tracking dirty state and affected column range.
///
/// When undamaged: `dirty == false`, column bounds are meaningless.
/// When damaged: `left <= right` defines the inclusive column range.
/// Full-line damage uses `left = 0, right = cols - 1`.
#[derive(Debug, Clone, Copy)]
pub struct LineDamageBounds {
    dirty: bool,
    /// Leftmost changed column (inclusive).
    left: usize,
    /// Rightmost changed column (inclusive).
    right: usize,
}

impl LineDamageBounds {
    /// Create clean (undamaged) bounds.
    fn clean() -> Self {
        Self {
            dirty: false,
            left: usize::MAX,
            right: 0,
        }
    }

    /// Expand the damage range to include `[left, right]` (inclusive).
    fn expand(&mut self, left: usize, right: usize) {
        self.dirty = true;
        self.left = self.left.min(left);
        self.right = self.right.max(right);
    }

    /// Mark the entire line dirty with full-width bounds.
    fn mark_full(&mut self, cols: usize) {
        self.dirty = true;
        self.left = 0;
        self.right = cols.saturating_sub(1);
    }

    /// Reset to clean state.
    fn reset(&mut self) {
        self.dirty = false;
        self.left = usize::MAX;
        self.right = 0;
    }
}

/// A dirty line yielded by [`DirtyIter`].
///
/// Contains the line index and the inclusive column range that changed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DirtyLine {
    /// Visible line index (0 = top).
    pub line: usize,
    /// Leftmost changed column (inclusive).
    pub left: usize,
    /// Rightmost changed column (inclusive).
    pub right: usize,
}

/// Tracks which rows have changed since last read.
///
/// Each visible line has damage bounds tracking both the dirty state and the
/// affected column range. `mark_all` provides a fast path for operations that
/// invalidate everything (scroll, resize, alternate screen swap). The `drain`
/// iterator yields dirty line info and resets the tracker to clean in a single
/// pass.
#[derive(Debug, Clone)]
pub struct DirtyTracker {
    /// Per-line damage bounds.
    lines: Vec<LineDamageBounds>,
    /// Number of columns (for full-line marks).
    cols: usize,
    /// Shortcut: everything changed (resize, scroll, alt screen swap).
    all_dirty: bool,
}

impl DirtyTracker {
    /// Create a new tracker with all lines clean.
    pub fn new(num_lines: usize, cols: usize) -> Self {
        Self {
            lines: vec![LineDamageBounds::clean(); num_lines],
            cols,
            all_dirty: false,
        }
    }

    /// Mark a single line fully dirty (all columns).
    ///
    /// Used for operations that affect the entire line: cursor movement,
    /// scroll, line-level erase.
    pub fn mark(&mut self, line: usize) {
        if let Some(bounds) = self.lines.get_mut(line) {
            bounds.mark_full(self.cols);
        }
    }

    /// Mark specific columns on a line as dirty.
    ///
    /// `left` and `right` are inclusive column indices. Used for cell writes
    /// and partial erases where only part of the line changed.
    pub fn mark_cols(&mut self, line: usize, left: usize, right: usize) {
        if let Some(bounds) = self.lines.get_mut(line) {
            bounds.expand(left, right);
        }
    }

    /// Mark a contiguous range of lines fully dirty.
    ///
    /// When the range covers all lines, sets `all_dirty` instead of
    /// individual entries — avoids O(n) updates and lets `collect_damage`
    /// take the fast path. Out-of-bounds indices are clamped silently.
    pub fn mark_range(&mut self, range: Range<usize>) {
        let len = self.lines.len();
        if range.start == 0 && range.end >= len {
            self.mark_all();
        } else {
            let start = range.start.min(len);
            let end = range.end.min(len);
            for bounds in &mut self.lines[start..end] {
                bounds.mark_full(self.cols);
            }
        }
    }

    /// Mark everything dirty.
    pub fn mark_all(&mut self) {
        self.all_dirty = true;
    }

    /// Check whether all lines are marked dirty.
    pub fn is_all_dirty(&self) -> bool {
        self.all_dirty
    }

    /// Check whether a specific line is dirty.
    pub fn is_dirty(&self, line: usize) -> bool {
        self.all_dirty || self.lines.get(line).is_some_and(|b| b.dirty)
    }

    /// Check whether any line is dirty.
    pub fn is_any_dirty(&self) -> bool {
        self.all_dirty || self.lines.iter().any(|b| b.dirty)
    }

    /// Get the column damage bounds for a specific line.
    ///
    /// Returns `(left, right)` inclusive. When `all_dirty` is set, returns
    /// full-line bounds. Returns `None` if the line is clean.
    pub fn col_bounds(&self, line: usize) -> Option<(usize, usize)> {
        if self.all_dirty {
            return Some((0, self.cols.saturating_sub(1)));
        }
        self.lines
            .get(line)
            .filter(|b| b.dirty)
            .map(|b| (b.left, b.right))
    }

    /// Yield dirty lines and reset all to clean.
    ///
    /// The returned iterator borrows the tracker mutably. Each yielded
    /// entry is immediately cleared, and any un-iterated dirty lines are
    /// cleared when the iterator is dropped.
    pub fn drain(&mut self) -> DirtyIter<'_> {
        let all = self.all_dirty;
        self.all_dirty = false;
        DirtyIter {
            lines: &mut self.lines,
            cols: self.cols,
            pos: 0,
            all,
        }
    }

    /// Resize the tracker to new dimensions, marking all dirty.
    pub fn resize(&mut self, num_lines: usize, cols: usize) {
        self.cols = cols;
        self.lines.resize(num_lines, LineDamageBounds::clean());
        self.mark_all();
    }
}

/// Iterator over dirty lines produced by [`DirtyTracker::drain`].
///
/// Yields [`DirtyLine`] entries with line index and column bounds.
/// Clears each entry as it yields. When dropped, clears any remaining
/// dirty entries that were not iterated.
pub struct DirtyIter<'a> {
    lines: &'a mut [LineDamageBounds],
    cols: usize,
    pos: usize,
    all: bool,
}

impl Iterator for DirtyIter<'_> {
    type Item = DirtyLine;

    fn next(&mut self) -> Option<DirtyLine> {
        while self.pos < self.lines.len() {
            let idx = self.pos;
            self.pos += 1;
            let bounds = &mut self.lines[idx];
            if self.all || bounds.dirty {
                let result = if self.all && !bounds.dirty {
                    // all_dirty but this line wasn't individually marked:
                    // report full-line damage.
                    DirtyLine {
                        line: idx,
                        left: 0,
                        right: self.cols.saturating_sub(1),
                    }
                } else {
                    DirtyLine {
                        line: idx,
                        left: bounds.left,
                        right: bounds.right,
                    }
                };
                bounds.reset();
                return Some(result);
            }
        }
        None
    }
}

impl Drop for DirtyIter<'_> {
    fn drop(&mut self) {
        // Clear any remaining dirty entries that were not iterated.
        for bounds in &mut self.lines[self.pos..] {
            bounds.reset();
        }
    }
}

#[cfg(test)]
mod tests;
