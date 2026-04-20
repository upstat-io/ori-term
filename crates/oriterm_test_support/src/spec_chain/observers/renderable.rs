//! Renderable rung observer (rung 4).
//!
//! Asserts that the terminal's `RenderableContent` snapshot matches the
//! expectation. Operates against the snapshot (the buffer the renderer
//! actually consumes), NOT the live state on `Term` — `palette_snapshot`,
//! `RenderableCursor`, `RenderableCell`, and `RenderableContent::damage`
//! are the canonical Rung 4 surfaces, and asserting against the live
//! accessors would bypass the path the renderer reads.

use oriterm_core::Term;
use oriterm_core::effect::sink::QueueingEffectSink;
use oriterm_core::term::renderable::RenderableContent;
use vte::ansi::Rgb;
use vte::ansi::cursor_icon::CursorIcon;

use crate::spec_chain::api::RungResult;
use crate::spec_chain::scenario::{RenderableExpectation, RungName};

/// Observe the renderable rung (Rung 4) against `term.renderable_content()`.
///
/// Each non-`None` field on `expected` produces one assertion against the
/// snapshot. The first failing assertion short-circuits with a diagnostic
/// reason string; otherwise the rung passes.
pub fn observe_renderable(
    term: &Term<QueueingEffectSink>,
    expected: RenderableExpectation,
) -> RungResult {
    let rc = term.renderable_content();

    if let Some(cells) = expected.cells {
        if let Err(reason) = check_cells(&rc, cells) {
            return RungResult::fail(RungName::Renderable, reason);
        }
    }
    if let Some(triple) = expected.hyperlink_at {
        if let Err(reason) = check_hyperlink(&rc, triple) {
            return RungResult::fail(RungName::Renderable, reason);
        }
    }
    if let Some(pos) = expected.cursor_position {
        if let Err(reason) = check_cursor_position(&rc, pos) {
            return RungResult::fail(RungName::Renderable, reason);
        }
    }
    if let Some(shape) = expected.cursor_shape {
        if rc.cursor.shape != shape {
            return RungResult::fail(
                RungName::Renderable,
                format!(
                    "cursor shape mismatch: expected {:?}, got {:?}",
                    shape, rc.cursor.shape
                ),
            );
        }
    }
    if let Some(entry) = expected.palette_index {
        if let Err(reason) = check_palette(&rc, entry) {
            return RungResult::fail(RungName::Renderable, reason);
        }
    }
    if let Some(icon) = expected.mouse_cursor_icon {
        if let Err(reason) = check_mouse_cursor_icon(&rc, icon) {
            return RungResult::fail(RungName::Renderable, reason);
        }
    }
    if let Some(lines) = expected.damaged_lines {
        if let Err(reason) = check_damaged_lines(&rc, lines) {
            return RungResult::fail(RungName::Renderable, reason);
        }
    }

    RungResult::pass(RungName::Renderable)
}

fn check_cells(rc: &RenderableContent, cells: &[(usize, usize, char)]) -> Result<(), String> {
    for &(line, col, ch) in cells {
        match rc
            .cells
            .iter()
            .find(|c| c.line == line && c.column.0 == col)
        {
            Some(cell) if cell.ch == ch => {}
            Some(cell) => {
                return Err(format!(
                    "cell at ({line},{col}) char mismatch: expected {:?}, got {:?}",
                    ch, cell.ch
                ));
            }
            None => {
                return Err(format!(
                    "cell at ({line},{col}) not present in renderable snapshot: \
                     expected char {ch:?}"
                ));
            }
        }
    }
    Ok(())
}

fn check_hyperlink(
    rc: &RenderableContent,
    (line, col, uri): (usize, usize, &str),
) -> Result<(), String> {
    let cell = rc
        .cells
        .iter()
        .find(|c| c.line == line && c.column.0 == col)
        .ok_or_else(|| {
            format!(
                "hyperlink at ({line},{col}): cell not present in renderable snapshot \
                 (expected URI {uri:?})"
            )
        })?;
    match &cell.hyperlink_uri {
        Some(actual) if actual == uri => Ok(()),
        Some(actual) => Err(format!(
            "hyperlink at ({line},{col}) URI mismatch: expected {uri:?}, got {actual:?}"
        )),
        None => Err(format!(
            "hyperlink at ({line},{col}) missing: expected {uri:?}, got None"
        )),
    }
}

fn check_cursor_position(
    rc: &RenderableContent,
    (line, col): (usize, usize),
) -> Result<(), String> {
    if rc.cursor.line == line && rc.cursor.column.0 == col {
        Ok(())
    } else {
        Err(format!(
            "cursor position mismatch: expected ({line},{col}), got ({},{})",
            rc.cursor.line, rc.cursor.column.0
        ))
    }
}

fn check_palette(rc: &RenderableContent, (index, rgb): (usize, Rgb)) -> Result<(), String> {
    let want = [rgb.r, rgb.g, rgb.b];
    match rc.palette_snapshot.get(index) {
        Some(actual) if *actual == want => Ok(()),
        Some(actual) => Err(format!(
            "palette[{index}] mismatch: expected {want:?}, got {actual:?}"
        )),
        None => Err(format!(
            "palette[{index}] out of range (snapshot has {} entries, expected {want:?})",
            rc.palette_snapshot.len()
        )),
    }
}

fn check_mouse_cursor_icon(rc: &RenderableContent, icon: CursorIcon) -> Result<(), String> {
    match rc.mouse_cursor_icon {
        Some(actual) if actual == icon => Ok(()),
        Some(actual) => Err(format!(
            "mouse_cursor_icon mismatch: expected {icon:?}, got {actual:?}"
        )),
        None => Err(format!(
            "mouse_cursor_icon missing: expected {icon:?}, got None"
        )),
    }
}

fn check_damaged_lines(rc: &RenderableContent, expected: &[usize]) -> Result<(), String> {
    let mut actual: Vec<usize> = if rc.all_dirty {
        (0..rc.lines).collect()
    } else {
        let mut v: Vec<usize> = rc.damage.iter().map(|d| d.line).collect();
        v.sort_unstable();
        v.dedup();
        v
    };
    let mut want: Vec<usize> = expected.to_vec();
    want.sort_unstable();
    want.dedup();
    actual.sort_unstable();
    if actual == want {
        Ok(())
    } else {
        Err(format!(
            "damaged_lines mismatch: expected {want:?}, got {actual:?} (all_dirty={})",
            rc.all_dirty
        ))
    }
}
