//! Selection damage detection — marks rows dirty when selection changes.
//!
//! Extracted from `dirty_skip/mod.rs` to keep each file under 500 lines.

use log::trace;

use crate::gpu::frame_input::{FrameInput, SelectionDamageSnapshot};

/// Build a fast dirty-row lookup from `RenderableContent` damage info.
///
/// Returns a `Vec<bool>` indexed by viewport line, where `true` means
/// the row needs regeneration. When `all_dirty` is set, all rows are dirty.
///
/// `prev_selection` is the selection snapshot from the previous frame.
/// When the selection changes between frames, the affected rows are marked
/// dirty so their instances are regenerated with correct selection colors.
///
/// `prev_cursor_line` is the cursor row from the previous frame (when
/// visible). When the cursor moves between frames, the previous cursor
/// row is dirtied so its cells regenerate without inheriting the "with
/// cursor" per-cell colors baked into `saved_tier`.
pub fn build_dirty_set(
    input: &FrameInput,
    num_rows: usize,
    prev_selection: Option<SelectionDamageSnapshot>,
    prev_cursor_line: Option<usize>,
    dirty: &mut Vec<bool>,
) {
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
    if input.content.cursor.visible && input.content.cursor.line < num_rows {
        dirty[input.content.cursor.line] = true;
    }

    // Cursor-move dirties the previous cursor row too — the cursor cell
    // can carry inverted FG/BG colors that bake per-cell at emit time.
    // Without this, clean-row replay leaves stale "with cursor" colors
    // on the row the cursor just left.
    let mut previous_cursor_dirtied = false;
    if let Some(prev_line) = prev_cursor_line
        && prev_line < num_rows
    {
        let cursor_moved = !input.content.cursor.visible || input.content.cursor.line != prev_line;
        if cursor_moved {
            dirty[prev_line] = true;
            previous_cursor_dirtied = true;
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
        let cursor_contributed =
            input.content.cursor.visible && input.content.cursor.line < num_rows;
        let pre = dirty.iter().filter(|d| **d).count();
        mark_selection_damage(dirty, prev_selection, new_selection);
        let post = dirty.iter().filter(|d| **d).count();
        let selection_added = post.saturating_sub(pre);
        trace!(
            "build_dirty_set all_dirty=false rows={num_rows} damage={damage_count} cursor={cursor_contributed} previous_cursor_dirtied={previous_cursor_dirtied} selection_added={selection_added} total={post}"
        );
    } else {
        mark_selection_damage(dirty, prev_selection, new_selection);
    }
}

/// Mark rows dirty that changed selection state between frames.
///
/// Compares full selection snapshots (line range, column extents, mode) to
/// detect any visual change. A row needs instance regeneration if:
/// - It was selected before but not now (or vice versa).
/// - It is a boundary line (first/last of either selection) and column
///   extents, side, or mode changed — these determine which cells within
///   the row are highlighted.
/// - For block selections or mode changes, every overlapping row may need
///   dirtying since the column range applies uniformly to all rows.
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
