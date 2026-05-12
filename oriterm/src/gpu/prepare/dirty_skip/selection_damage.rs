//! Selection damage detection — marks rows dirty when selection changes.
//!
//! Extracted from `dirty_skip/mod.rs` to keep each file under 500 lines.

use log::trace;
use oriterm_core::RenderableCursor;

use crate::gpu::frame_input::{FrameInput, SelectionDamageSnapshot};
use crate::gpu::prepared_frame::PreparedFrame;

/// Snapshot of the previous frame's row-state inputs that drive the
/// dirty-set computation. Sourced from `PreparedFrame::prev_*` fields;
/// derived once at the call site and passed by value so `build_dirty_set`
/// doesn't need to borrow the whole frame (which conflicts with `&mut
/// frame.scratch_dirty`).
#[derive(Debug, Clone, Copy)]
pub struct PrevFrameState {
    pub selection_snapshot: Option<SelectionDamageSnapshot>,
    pub resolved_cursor: Option<RenderableCursor>,
    pub hovered_cell: Option<(usize, usize)>,
}

impl PrevFrameState {
    pub fn from_frame(frame: &PreparedFrame) -> Self {
        Self {
            selection_snapshot: frame.prev_selection_snapshot,
            resolved_cursor: frame.prev_resolved_cursor,
            hovered_cell: frame.prev_hovered_cell,
        }
    }

    /// Previous-cursor line for dirty-set row computation. The
    /// visibility-canonicalized storage rule guarantees `Some(...)`
    /// already implies visible, so no extra filter needed.
    fn cursor_line(&self) -> Option<usize> {
        self.resolved_cursor.as_ref().map(|c| c.line)
    }
}

/// Build a fast dirty-row lookup from `RenderableContent` damage info.
///
/// See: `PrevFrameState` for the prev-frame snapshot fields consumed.
///
/// Returns a `Vec<bool>` indexed by viewport line — `true` = regenerate.
/// When `all_dirty` is set, every row is dirty.
///
/// `resolved_cursor` is the cursor that will actually render this frame
/// (raw `content.cursor` overlaid with `mark_cursor` when mark mode is
/// active). Using `resolved_cursor.line` instead of raw `content.cursor.line`
/// avoids dirtying the wrong row when mark mode relocates the visible cursor.
pub fn build_dirty_set(
    input: &FrameInput,
    num_rows: usize,
    resolved_cursor: &RenderableCursor,
    prev_state: PrevFrameState,
    dirty: &mut Vec<bool>,
) {
    let prev_selection = prev_state.selection_snapshot;
    let prev_cursor_line = prev_state.cursor_line();
    let prev_hovered_cell = prev_state.hovered_cell;
    dirty.clear();
    if input.content.all_dirty {
        dirty.resize(num_rows, true);
        if log::log_enabled!(log::Level::Trace) {
            trace!("build_dirty_set all_dirty=true rows={num_rows}");
        }
        return;
    }

    dirty.resize(num_rows, false);
    for d in &input.content.damage {
        if d.line < num_rows {
            dirty[d.line] = true;
        }
    }

    // The cursor row is always dirty (cursor blink state may have changed).
    if resolved_cursor.visible && resolved_cursor.line < num_rows {
        dirty[resolved_cursor.line] = true;
    }

    // Why: cursor-move must dirty the previous cursor row too — the cursor
    // cell can carry inverted FG/BG colors that bake per-cell at emit time,
    // so clean-row replay would leave stale "with cursor" colors on the row
    // the cursor just left.
    let mut previous_cursor_dirtied = false;
    if let Some(prev_line) = prev_cursor_line
        && prev_line < num_rows
    {
        let cursor_moved = !resolved_cursor.visible || resolved_cursor.line != prev_line;
        if cursor_moved {
            dirty[prev_line] = true;
            previous_cursor_dirtied = true;
        }
    }

    // Why: when the hovered cell moves, the rows containing the previous
    // and current hover positions must be dirtied. The hyperlink hover
    // path bakes a solid underline into the terminal-tier `backgrounds`
    // buffer for the hovered cell, so a hover delta requires those rows
    // to regenerate.
    let mut hover_rows_dirtied = 0u32;
    if prev_hovered_cell != input.hovered_cell {
        if let Some((prev_row, _)) = prev_hovered_cell
            && prev_row < num_rows
            && !dirty[prev_row]
        {
            dirty[prev_row] = true;
            hover_rows_dirtied += 1;
        }
        if let Some((curr_row, _)) = input.hovered_cell
            && curr_row < num_rows
            && !dirty[curr_row]
        {
            dirty[curr_row] = true;
            hover_rows_dirtied += 1;
        }
    }

    // Selection damage: mark rows that changed selection state.
    let new_selection = input
        .selection
        .as_ref()
        .and_then(|s| s.damage_snapshot(num_rows));

    if log::log_enabled!(log::Level::Trace) {
        // Trace-only counting path — the per-source counts and total
        // never run when tracing is disabled.
        let damage_count = input
            .content
            .damage
            .iter()
            .filter(|d| d.line < num_rows)
            .count();
        // Trace the resolved cursor (mark-mode override applied) so the
        // counter matches the actual dirty-set's row source — downstream
        // logic dirties `resolved_cursor.line`, not raw terminal cursor.
        let cursor_contributed = resolved_cursor.visible && resolved_cursor.line < num_rows;
        let pre = dirty.iter().filter(|d| **d).count();
        mark_selection_damage(dirty, prev_selection, new_selection);
        let post = dirty.iter().filter(|d| **d).count();
        let selection_added = post.saturating_sub(pre);
        trace!(
            "build_dirty_set all_dirty=false rows={num_rows} damage={damage_count} cursor={cursor_contributed} previous_cursor_dirtied={previous_cursor_dirtied} hover_rows_dirtied={hover_rows_dirtied} selection_added={selection_added} total={post}"
        );
    } else {
        mark_selection_damage(dirty, prev_selection, new_selection);
    }
}

/// Mark rows dirty that changed selection state between frames.
///
/// See: `SelectionDamageSnapshot` for the per-frame snapshot shape.
///
/// Compares full selection snapshots (line range, column extents, mode) to
/// detect visual change. A row needs regeneration if its selection
/// membership flips, it is a boundary line whose column extents/side/mode
/// changed, or it lies inside a block selection / mode change where the
/// column range applies uniformly to all rows.
pub(crate) fn mark_selection_damage(
    dirty: &mut [bool],
    old: Option<SelectionDamageSnapshot>,
    new: Option<SelectionDamageSnapshot>,
) {
    if old == new {
        return;
    }
    let num_rows = dirty.len();
    if num_rows == 0 {
        return;
    }
    let max_line = num_rows - 1;

    let old_range = old.map(|s| (s.start_line, s.end_line));
    let new_range = new.map(|s| (s.start_line, s.end_line));

    match (old_range, new_range) {
        (None, None) => {}
        (Some((s, e)), None) => {
            // Selection cleared: damage all previously-selected lines.
            for d in &mut dirty[s..=e.min(max_line)] {
                *d = true;
            }
        }
        (None, Some((s, e))) => {
            // New selection: damage all newly-selected lines.
            for d in &mut dirty[s..=e.min(max_line)] {
                *d = true;
            }
        }
        (Some((os, oe)), Some((ns, ne))) => {
            // When either selection is block mode or the mode changed,
            // column extent changes affect every interior row — not just
            // boundary rows. Dirty the entire union of both ranges.
            let block_or_mode_changed = old
                .zip(new)
                .is_some_and(|(o, n)| o.mode != n.mode || is_block_mode(o, n));
            if block_or_mode_changed {
                let min_s = os.min(ns);
                let max_e = oe.max(ne).min(max_line);
                for d in &mut dirty[min_s..=max_e] {
                    *d = true;
                }
                return;
            }

            // Linear mode: only symmetric-difference and boundary rows
            // need regeneration. Interior rows are fully selected in both
            // the old and new selections, so their highlight is unchanged.
            let min_s = os.min(ns);
            let max_e = oe.max(ne).min(max_line);
            for (line, d) in dirty.iter_mut().enumerate().take(max_e + 1).skip(min_s) {
                let was = line >= os && line <= oe;
                let is = line >= ns && line <= ne;
                if was != is {
                    *d = true;
                }
            }
            // Boundary lines always dirty — column extent or side
            // may differ even when the line range overlaps.
            if os <= max_line {
                dirty[os] = true;
            }
            if oe <= max_line {
                dirty[oe] = true;
            }
            if ns <= max_line {
                dirty[ns] = true;
            }
            if ne <= max_line {
                dirty[ne] = true;
            }
        }
    }
}

/// Check if either snapshot uses block selection mode.
fn is_block_mode(a: SelectionDamageSnapshot, b: SelectionDamageSnapshot) -> bool {
    use oriterm_core::selection::SelectionMode;
    a.mode == SelectionMode::Block || b.mode == SelectionMode::Block
}
