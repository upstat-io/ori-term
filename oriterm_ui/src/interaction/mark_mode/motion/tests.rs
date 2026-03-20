//! Tests for pure mark mode motion functions.

use super::{
    AbsCursor, GridBounds, WordContext, buffer_end, buffer_start, line_end, line_start, move_down,
    move_left, move_right, move_up, page_down, page_up, word_left, word_right,
};

// ---------------------------------------------------------------------------
// GridBounds helpers
// ---------------------------------------------------------------------------

/// Standard 80x24 grid with no scrollback.
fn bounds_80x24() -> GridBounds {
    GridBounds {
        total_rows: 24,
        cols: 80,
        visible_lines: 24,
    }
}

/// 80-column grid with 100 rows of scrollback + 24 visible.
fn bounds_with_scrollback() -> GridBounds {
    GridBounds {
        total_rows: 124,
        cols: 80,
        visible_lines: 24,
    }
}

// ---------------------------------------------------------------------------
// move_left
// ---------------------------------------------------------------------------

#[test]
fn move_left_decrements_col() {
    let c = AbsCursor { abs_row: 0, col: 5 };
    let r = move_left(c, bounds_80x24());
    assert_eq!(r, AbsCursor { abs_row: 0, col: 4 });
}

#[test]
fn move_left_wraps_to_prev_row() {
    let c = AbsCursor { abs_row: 1, col: 0 };
    let r = move_left(c, bounds_80x24());
    assert_eq!(
        r,
        AbsCursor {
            abs_row: 0,
            col: 79
        }
    );
}

#[test]
fn move_left_clamps_at_buffer_start() {
    let c = AbsCursor { abs_row: 0, col: 0 };
    let r = move_left(c, bounds_80x24());
    assert_eq!(r, AbsCursor { abs_row: 0, col: 0 });
}

// ---------------------------------------------------------------------------
// move_right
// ---------------------------------------------------------------------------

#[test]
fn move_right_increments_col() {
    let c = AbsCursor { abs_row: 0, col: 5 };
    let r = move_right(c, bounds_80x24());
    assert_eq!(r, AbsCursor { abs_row: 0, col: 6 });
}

#[test]
fn move_right_wraps_to_next_row() {
    let c = AbsCursor {
        abs_row: 0,
        col: 79,
    };
    let r = move_right(c, bounds_80x24());
    assert_eq!(r, AbsCursor { abs_row: 1, col: 0 });
}

#[test]
fn move_right_clamps_at_buffer_end() {
    let c = AbsCursor {
        abs_row: 23,
        col: 79,
    };
    let r = move_right(c, bounds_80x24());
    assert_eq!(
        r,
        AbsCursor {
            abs_row: 23,
            col: 79
        }
    );
}

// ---------------------------------------------------------------------------
// move_up / move_down
// ---------------------------------------------------------------------------

#[test]
fn move_up_decrements_row() {
    let c = AbsCursor {
        abs_row: 5,
        col: 10,
    };
    let r = move_up(c);
    assert_eq!(
        r,
        AbsCursor {
            abs_row: 4,
            col: 10
        }
    );
}

#[test]
fn move_up_clamps_at_top() {
    let c = AbsCursor {
        abs_row: 0,
        col: 10,
    };
    let r = move_up(c);
    assert_eq!(
        r,
        AbsCursor {
            abs_row: 0,
            col: 10
        }
    );
}

#[test]
fn move_down_increments_row() {
    let c = AbsCursor {
        abs_row: 5,
        col: 10,
    };
    let r = move_down(c, bounds_80x24());
    assert_eq!(
        r,
        AbsCursor {
            abs_row: 6,
            col: 10
        }
    );
}

#[test]
fn move_down_clamps_at_bottom() {
    let c = AbsCursor {
        abs_row: 23,
        col: 10,
    };
    let r = move_down(c, bounds_80x24());
    assert_eq!(
        r,
        AbsCursor {
            abs_row: 23,
            col: 10
        }
    );
}

#[test]
fn move_down_preserves_col() {
    let c = AbsCursor {
        abs_row: 5,
        col: 40,
    };
    let r = move_down(c, bounds_80x24());
    assert_eq!(r.col, 40);
}

// ---------------------------------------------------------------------------
// page_up / page_down
// ---------------------------------------------------------------------------

#[test]
fn page_up_moves_by_visible_lines() {
    let b = bounds_with_scrollback();
    let c = AbsCursor {
        abs_row: 50,
        col: 10,
    };
    let r = page_up(c, b);
    assert_eq!(
        r,
        AbsCursor {
            abs_row: 26,
            col: 10
        }
    );
}

#[test]
fn page_up_clamps_at_top() {
    let b = bounds_with_scrollback();
    let c = AbsCursor {
        abs_row: 5,
        col: 10,
    };
    let r = page_up(c, b);
    assert_eq!(
        r,
        AbsCursor {
            abs_row: 0,
            col: 10
        }
    );
}

#[test]
fn page_down_moves_by_visible_lines() {
    let b = bounds_with_scrollback();
    let c = AbsCursor {
        abs_row: 50,
        col: 10,
    };
    let r = page_down(c, b);
    assert_eq!(
        r,
        AbsCursor {
            abs_row: 74,
            col: 10
        }
    );
}

#[test]
fn page_down_clamps_at_bottom() {
    let b = bounds_with_scrollback();
    let c = AbsCursor {
        abs_row: 120,
        col: 10,
    };
    let r = page_down(c, b);
    assert_eq!(
        r,
        AbsCursor {
            abs_row: 123,
            col: 10
        }
    );
}

// ---------------------------------------------------------------------------
// line_start / line_end
// ---------------------------------------------------------------------------

#[test]
fn line_start_moves_to_col_zero() {
    let c = AbsCursor {
        abs_row: 5,
        col: 40,
    };
    let r = line_start(c);
    assert_eq!(r, AbsCursor { abs_row: 5, col: 0 });
}

#[test]
fn line_end_moves_to_last_col() {
    let c = AbsCursor { abs_row: 5, col: 0 };
    let r = line_end(c, bounds_80x24());
    assert_eq!(
        r,
        AbsCursor {
            abs_row: 5,
            col: 79
        }
    );
}

// ---------------------------------------------------------------------------
// buffer_start / buffer_end
// ---------------------------------------------------------------------------

#[test]
fn buffer_start_goes_to_origin() {
    let r = buffer_start();
    assert_eq!(r, AbsCursor { abs_row: 0, col: 0 });
}

#[test]
fn buffer_end_goes_to_last_cell() {
    let r = buffer_end(bounds_80x24());
    assert_eq!(
        r,
        AbsCursor {
            abs_row: 23,
            col: 79
        }
    );
}

#[test]
fn buffer_end_with_scrollback() {
    let r = buffer_end(bounds_with_scrollback());
    assert_eq!(
        r,
        AbsCursor {
            abs_row: 123,
            col: 79
        }
    );
}

// ---------------------------------------------------------------------------
// Degenerate grid bounds
// ---------------------------------------------------------------------------

#[test]
fn single_row_single_col_grid_all_motions_clamp() {
    let b = GridBounds {
        total_rows: 1,
        cols: 1,
        visible_lines: 1,
    };
    let origin = AbsCursor { abs_row: 0, col: 0 };

    assert_eq!(move_left(origin, b), origin);
    assert_eq!(move_right(origin, b), origin);
    assert_eq!(move_up(origin), origin);
    assert_eq!(move_down(origin, b), origin);
    assert_eq!(page_up(origin, b), origin);
    assert_eq!(page_down(origin, b), origin);
    assert_eq!(line_start(origin), origin);
    assert_eq!(line_end(origin, b), origin);
    assert_eq!(buffer_start(), origin);
    assert_eq!(buffer_end(b), origin);
}

#[test]
fn zero_column_grid_does_not_panic() {
    let b = GridBounds {
        total_rows: 10,
        cols: 0,
        visible_lines: 10,
    };
    let c = AbsCursor { abs_row: 0, col: 0 };

    // These should not panic — saturating_sub handles cols=0.
    let _ = move_left(c, b);
    let _ = move_right(c, b);
    let _ = line_end(c, b);
    let _ = buffer_end(b);
}

#[test]
fn zero_row_grid_does_not_panic() {
    let b = GridBounds {
        total_rows: 0,
        cols: 80,
        visible_lines: 0,
    };
    let c = AbsCursor { abs_row: 0, col: 0 };

    let _ = move_down(c, b);
    let _ = page_down(c, b);
    let _ = buffer_end(b);
}

// ---------------------------------------------------------------------------
// Sequential motions accumulate
// ---------------------------------------------------------------------------

#[test]
fn sequential_right_motions_accumulate() {
    let b = bounds_80x24();
    let mut c = AbsCursor { abs_row: 0, col: 0 };
    for _ in 0..5 {
        c = move_right(c, b);
    }
    assert_eq!(c, AbsCursor { abs_row: 0, col: 5 });
}

#[test]
fn sequential_motions_wrap_across_rows() {
    let b = GridBounds {
        total_rows: 10,
        cols: 3,
        visible_lines: 10,
    };
    let mut c = AbsCursor { abs_row: 0, col: 0 };
    // 3 cols per row: move right 7 times -> row 2 col 1.
    for _ in 0..7 {
        c = move_right(c, b);
    }
    assert_eq!(c, AbsCursor { abs_row: 2, col: 1 });

    // Move left 7 times -> back to origin.
    for _ in 0..7 {
        c = move_left(c, b);
    }
    assert_eq!(c, AbsCursor { abs_row: 0, col: 0 });
}

#[test]
fn sequential_down_then_up_returns_to_start() {
    let b = bounds_80x24();
    let start = AbsCursor {
        abs_row: 10,
        col: 40,
    };
    let mut c = start;
    for _ in 0..5 {
        c = move_down(c, b);
    }
    assert_eq!(c.abs_row, 15);
    assert_eq!(c.col, 40);
    for _ in 0..5 {
        c = move_up(c);
    }
    assert_eq!(c, start);
}

// ---------------------------------------------------------------------------
// Page up preserves column
// ---------------------------------------------------------------------------

#[test]
fn page_up_preserves_col() {
    let b = bounds_with_scrollback();
    let c = AbsCursor {
        abs_row: 50,
        col: 42,
    };
    let r = page_up(c, b);
    assert_eq!(r.col, 42);
}

// ---------------------------------------------------------------------------
// Word navigation (pure motion functions)
// ---------------------------------------------------------------------------

#[test]
fn word_left_jumps_to_word_start() {
    // Cursor inside a word (col 7, word starts at 5).
    let c = AbsCursor { abs_row: 2, col: 7 };
    let ctx = WordContext {
        ws: 5,
        we: 9,
        prev_same_row_ws: None,
        prev_row_ws: None,
        next_same_row_we: None,
        next_row_we: None,
    };
    assert_eq!(word_left(c, &ctx), AbsCursor { abs_row: 2, col: 5 });
}

#[test]
fn word_left_jumps_to_prev_word_on_same_row() {
    // Cursor at word start (col 5, ws=5), prev word starts at 0.
    let c = AbsCursor { abs_row: 2, col: 5 };
    let ctx = WordContext {
        ws: 5,
        we: 9,
        prev_same_row_ws: Some(0),
        prev_row_ws: None,
        next_same_row_we: None,
        next_row_we: None,
    };
    assert_eq!(word_left(c, &ctx), AbsCursor { abs_row: 2, col: 0 });
}

#[test]
fn word_left_wraps_to_prev_row() {
    // Cursor at col 0, ws=0, no prev word on same row, prev row available.
    let c = AbsCursor { abs_row: 3, col: 0 };
    let ctx = WordContext {
        ws: 0,
        we: 4,
        prev_same_row_ws: None,
        prev_row_ws: Some(70),
        next_same_row_we: None,
        next_row_we: None,
    };
    assert_eq!(
        word_left(c, &ctx),
        AbsCursor {
            abs_row: 2,
            col: 70
        }
    );
}

#[test]
fn word_left_at_origin_clamps() {
    let c = AbsCursor { abs_row: 0, col: 0 };
    let ctx = WordContext {
        ws: 0,
        we: 0,
        prev_same_row_ws: None,
        prev_row_ws: None,
        next_same_row_we: None,
        next_row_we: None,
    };
    assert_eq!(word_left(c, &ctx), AbsCursor { abs_row: 0, col: 0 });
}

#[test]
fn word_right_jumps_to_word_end() {
    // Cursor inside a word (col 2, word ends at 4).
    let c = AbsCursor { abs_row: 1, col: 2 };
    let ctx = WordContext {
        ws: 0,
        we: 4,
        prev_same_row_ws: None,
        prev_row_ws: None,
        next_same_row_we: None,
        next_row_we: None,
    };
    assert_eq!(
        word_right(c, &ctx, bounds_80x24()),
        AbsCursor { abs_row: 1, col: 4 }
    );
}

#[test]
fn word_right_jumps_to_next_word_on_same_row() {
    // Cursor at word end (col 4, we=4), next word ends at 9.
    let c = AbsCursor { abs_row: 1, col: 4 };
    let ctx = WordContext {
        ws: 0,
        we: 4,
        prev_same_row_ws: None,
        prev_row_ws: None,
        next_same_row_we: Some(9),
        next_row_we: None,
    };
    assert_eq!(
        word_right(c, &ctx, bounds_80x24()),
        AbsCursor { abs_row: 1, col: 9 }
    );
}

#[test]
fn word_right_wraps_to_next_row() {
    // Cursor at word end, no next word on same row, next row available.
    let c = AbsCursor {
        abs_row: 1,
        col: 75,
    };
    let ctx = WordContext {
        ws: 70,
        we: 75,
        prev_same_row_ws: None,
        prev_row_ws: None,
        next_same_row_we: None,
        next_row_we: Some(5),
    };
    assert_eq!(
        word_right(c, &ctx, bounds_80x24()),
        AbsCursor { abs_row: 2, col: 5 }
    );
}

#[test]
fn word_right_clamps_at_end_of_buffer() {
    // Last row, at word end, no next word, no next row.
    let c = AbsCursor {
        abs_row: 23,
        col: 75,
    };
    let ctx = WordContext {
        ws: 70,
        we: 75,
        prev_same_row_ws: None,
        prev_row_ws: None,
        next_same_row_we: None,
        next_row_we: None,
    };
    assert_eq!(
        word_right(c, &ctx, bounds_80x24()),
        AbsCursor {
            abs_row: 23,
            col: 79
        }
    );
}
