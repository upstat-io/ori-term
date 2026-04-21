//! Behaviour pins for the DEC presentation handlers.
//!
//! CSI-path presentation queries (§09A.8) exercise real dispatches
//! through the full VTE → `Term` pipeline, verifying either an
//! effect (PTY write) or a state mutation. Column ops (DECIC/DECDC)
//! and ESC-path back/forward index ops (DECBI/DECFI) — §09A.7 —
//! mutate grid cells on every row of the active scroll region, with
//! DECLRMM-aware horizontal clamping. Deeper spec_chain coverage
//! lives in `oriterm_core/tests/spec_chain/dec_presentation/`.

use crate::cell::CellFlags;
use crate::effect::sink::{EffectSink, QueueingEffectSink};
use crate::effect::{Effect, PtyEffect};
use crate::index::Column;
use crate::term::Term;
use crate::theme::Theme;

use super::super::test_helpers::feed;

fn term_24x80() -> Term<QueueingEffectSink> {
    Term::new(24, 80, 0, Theme::default(), QueueingEffectSink::new())
}

fn seed_abc(t: &mut Term<QueueingEffectSink>) {
    feed(t, b"ABC");
    feed(t, b"\x1b[1;1H"); // home cursor so state is deterministic
}

/// Drain every `PtyEffect::Write` the Term has queued and concatenate
/// the byte payloads. Reply bytes for a single CSI query land in a
/// single write, so callers can assert the full reply verbatim.
fn drain_pty_writes(t: &Term<QueueingEffectSink>) -> Vec<u8> {
    let mut effects = Vec::new();
    t.effect_sink().drain_into(&mut effects);
    let mut bytes = Vec::new();
    for eff in effects {
        if let Effect::Pty(PtyEffect::Write { bytes: b, .. }) = eff {
            bytes.extend_from_slice(&b);
        }
    }
    bytes
}

// ─── CSI-path presentation queries (§09A.8) ──────────────────────────

#[test]
fn decrqde_reply_encodes_grid_dimensions() {
    let mut t = term_24x80();
    feed(&mut t, b"\x1b[\"v"); // DECRQDE
    let reply = drain_pty_writes(&t);
    assert_eq!(reply, b"\x1b[24;80;1;1;1\"w");
}

#[test]
fn decrqde_reply_reflects_current_grid_shape() {
    let mut t = Term::new(10, 40, 0, Theme::default(), QueueingEffectSink::new());
    feed(&mut t, b"\x1b[\"v");
    let reply = drain_pty_writes(&t);
    assert_eq!(reply, b"\x1b[10;40;1;1;1\"w");
}

#[test]
fn decrqupss_reply_is_iso_latin1_constant() {
    let mut t = term_24x80();
    feed(&mut t, b"\x1b[&u"); // DECRQUPSS
    let reply = drain_pty_writes(&t);
    assert_eq!(reply, b"\x1bP1!u%5\x1b\\");
}

#[test]
fn decrqpsr_mode2_reply_lists_all_default_tab_stops() {
    let mut t = term_24x80();
    feed(&mut t, b"\x1b[2$w");
    let reply = drain_pty_writes(&t);
    // Default tab stops at every 8th column: 1-based → 1, 9, 17, 25,
    // 33, 41, 49, 57, 65, 73.
    assert_eq!(reply, b"\x1bP2$u1/9/17/25/33/41/49/57/65/73\x1b\\");
}

#[test]
fn decrqpsr_mode2_reply_picks_up_custom_tab_stops() {
    let mut t = Term::new(5, 10, 0, Theme::default(), QueueingEffectSink::new());
    // Clear all stops then set a single stop at column 3 (0-based col 2).
    feed(&mut t, b"\x1b[3g"); // TBC 3 — clear all tab stops
    feed(&mut t, b"\x1b[1;3H\x1bH"); // move to col 3, HTS
    feed(&mut t, b"\x1b[2$w"); // DECRQPSR mode 2
    let reply = drain_pty_writes(&t);
    assert_eq!(reply, b"\x1bP2$u3\x1b\\");
}

#[test]
fn decrqpsr_mode1_emits_dcs_reply() {
    let mut t = term_24x80();
    seed_abc(&mut t);
    feed(&mut t, b"\x1b[1;1H"); // home
    feed(&mut t, b"\x1b[1$w"); // DECRQPSR mode 1
    let reply = drain_pty_writes(&t);
    // Stub payload: cursor at 1;1;1, filler slots for state we don't
    // yet serialize. See `decrqpsr_impl` for the deviation note.
    assert!(
        reply.starts_with(b"\x1bP1$u"),
        "DECRQPSR mode 1 must emit DCS 1 $ u header, got {:?}",
        String::from_utf8_lossy(&reply),
    );
    assert!(reply.ends_with(b"\x1b\\"));
}

#[test]
fn decrqpsr_unknown_mode_emits_no_reply() {
    let mut t = term_24x80();
    feed(&mut t, b"\x1b[7$w"); // DECRQPSR mode 7 — unknown
    let reply = drain_pty_writes(&t);
    assert!(reply.is_empty(), "unknown mode must not reply");
}

#[test]
fn decscl_sets_conformance_level_and_c1_mode() {
    let mut t = term_24x80();
    feed(&mut t, b"\x1b[61;1\"p"); // DECSCL level=61 (VT100) c1=1 (7-bit)
    assert_eq!(t.conformance_level(), 61);
    assert!(t.c1_7bit());
}

#[test]
fn decscl_accepts_8bit_c1_mode() {
    let mut t = term_24x80();
    feed(&mut t, b"\x1b[63;0\"p"); // DECSCL level=63 c1=0 (8-bit)
    assert_eq!(t.conformance_level(), 63);
    assert!(!t.c1_7bit());
    // Pc=2 is spec-equivalent to Pc=0.
    feed(&mut t, b"\x1b[63;2\"p");
    assert!(!t.c1_7bit());
}

#[test]
fn decscl_triggers_soft_reset_side_effects() {
    let mut t = term_24x80();
    // Set INSERT mode and a non-default scroll region.
    feed(&mut t, b"\x1b[4h"); // SM: insert
    feed(&mut t, b"\x1b[2;10r"); // DECSTBM: margins
    // DECSCL level=64 — must soft-reset state below.
    feed(&mut t, b"\x1b[64;1\"p");
    // After soft reset, INSERT is cleared and scroll region is full grid.
    assert!(
        !t.mode().contains(crate::term::TermMode::INSERT),
        "DECSCL must clear INSERT via soft reset"
    );
    let region = t.grid().scroll_region();
    assert_eq!(region.start, 0);
    assert_eq!(region.end, t.grid().lines());
}

#[test]
fn decsca_toggles_char_protection_flag() {
    let mut t = term_24x80();
    assert!(!t.char_protection());
    feed(&mut t, b"\x1b[1\"q"); // DECSCA Ps=1 → protected
    assert!(t.char_protection());
    feed(&mut t, b"\x1b[0\"q"); // DECSCA Ps=0 → unprotected
    assert!(!t.char_protection());
    feed(&mut t, b"\x1b[2\"q"); // DECSCA Ps=2 → unprotected
    assert!(!t.char_protection());
    feed(&mut t, b"\x1b[1\"q");
    assert!(t.char_protection());
}

#[test]
fn decsca_protected_flag_propagates_to_written_cells() {
    let mut t = term_24x80();
    feed(&mut t, b"\x1b[1;1H"); // home
    feed(&mut t, b"\x1b[1\"q"); // DECSCA Ps=1 → protect
    feed(&mut t, b"PQ");
    feed(&mut t, b"\x1b[0\"q"); // DECSCA Ps=0 → unprotect
    feed(&mut t, b"R");

    let row = &t.grid()[crate::index::Line(0)];
    assert!(row[Column(0)].flags.contains(CellFlags::PROTECTED));
    assert!(row[Column(1)].flags.contains(CellFlags::PROTECTED));
    assert!(!row[Column(2)].flags.contains(CellFlags::PROTECTED));
    // Sanity: chars land where we expect.
    assert_eq!(row[Column(0)].ch, 'P');
    assert_eq!(row[Column(1)].ch, 'Q');
    assert_eq!(row[Column(2)].ch, 'R');
}

#[test]
fn sgr_reset_preserves_decsca_protection() {
    // DECSCA is NOT an SGR attribute; `SGR 0` must not clear it.
    let mut t = term_24x80();
    feed(&mut t, b"\x1b[1;1H");
    feed(&mut t, b"\x1b[1\"q"); // protect
    feed(&mut t, b"\x1b[0m"); // SGR reset
    feed(&mut t, b"Z");
    let row = &t.grid()[crate::index::Line(0)];
    assert!(
        row[Column(0)].flags.contains(CellFlags::PROTECTED),
        "SGR 0 must preserve DECSCA PROTECTED bit"
    );
}

#[test]
fn decstr_clears_decsca_protection() {
    let mut t = term_24x80();
    feed(&mut t, b"\x1b[1\"q"); // protect
    assert!(t.char_protection());
    feed(&mut t, b"\x1b[!p"); // DECSTR
    assert!(
        !t.char_protection(),
        "DECSTR must clear DECSCA protection state"
    );
}

#[test]
fn decsasd_stores_target() {
    let mut t = term_24x80();
    feed(&mut t, b"\x1b[0$}");
    assert_eq!(t.active_status_display(), 0);
    feed(&mut t, b"\x1b[1$}");
    assert_eq!(t.active_status_display(), 1);
}

#[test]
fn decssdt_stores_line_type() {
    let mut t = term_24x80();
    feed(&mut t, b"\x1b[0$~");
    assert_eq!(t.status_line_type(), 0);
    feed(&mut t, b"\x1b[1$~");
    assert_eq!(t.status_line_type(), 1);
    feed(&mut t, b"\x1b[2$~");
    assert_eq!(t.status_line_type(), 2);
}

// ─── Column + index ops (§09A.7) ─────────────────────────────────────

#[test]
fn decic_shifts_content_right_at_cursor() {
    let mut t = term_24x80();
    seed_abc(&mut t);
    feed(&mut t, b"\x1b[1;1H"); // home
    feed(&mut t, b"\x1b[2'}"); // DECIC count=2
    let row = &t.grid()[crate::index::Line(0)];
    assert_eq!(row[Column(0)].ch, ' ');
    assert_eq!(row[Column(1)].ch, ' ');
    assert_eq!(row[Column(2)].ch, 'A');
    assert_eq!(row[Column(3)].ch, 'B');
    assert_eq!(row[Column(4)].ch, 'C');
}

#[test]
fn decdc_shifts_content_left_at_cursor() {
    let mut t = term_24x80();
    seed_abc(&mut t);
    feed(&mut t, b"\x1b[1;1H");
    feed(&mut t, b"\x1b[1'~"); // DECDC count=1
    let row = &t.grid()[crate::index::Line(0)];
    assert_eq!(row[Column(0)].ch, 'B');
    assert_eq!(row[Column(1)].ch, 'C');
}

#[test]
fn decbi_at_column_zero_inserts_blank_column() {
    let mut t = term_24x80();
    seed_abc(&mut t);
    feed(&mut t, b"\x1b[1;1H"); // home (col == left bound)
    feed(&mut t, b"\x1b6"); // DECBI
    let row = &t.grid()[crate::index::Line(0)];
    assert_eq!(row[Column(0)].ch, ' ');
    assert_eq!(row[Column(1)].ch, 'A');
    assert_eq!(row[Column(2)].ch, 'B');
    assert_eq!(row[Column(3)].ch, 'C');
}

#[test]
fn decbi_not_at_margin_moves_cursor_left() {
    let mut t = term_24x80();
    seed_abc(&mut t);
    feed(&mut t, b"\x1b[1;3H"); // col 3 (0-based col 2)
    feed(&mut t, b"\x1b6"); // DECBI
    // Grid content unchanged; cursor moved back by one column.
    let row = &t.grid()[crate::index::Line(0)];
    assert_eq!(row[Column(0)].ch, 'A');
    assert_eq!(row[Column(1)].ch, 'B');
    assert_eq!(row[Column(2)].ch, 'C');
    assert_eq!(t.grid().cursor().col().0, 1);
}

#[test]
fn decfi_at_last_column_deletes_at_left_margin() {
    let mut t = term_24x80();
    seed_abc(&mut t);
    // Move to the last column (80 cols → col index 79 == right bound).
    feed(&mut t, b"\x1b[1;80H");
    feed(&mut t, b"\x1b9"); // DECFI
    // DECFI at right margin deletes a column at the LEFT bound on
    // every row — with no DECLRMM, left bound is 0. So 'A' at col 0
    // is removed; 'B', 'C' shift left.
    let row = &t.grid()[crate::index::Line(0)];
    assert_eq!(row[Column(0)].ch, 'B');
    assert_eq!(row[Column(1)].ch, 'C');
}

#[test]
fn decfi_not_at_margin_moves_cursor_right() {
    let mut t = term_24x80();
    seed_abc(&mut t);
    feed(&mut t, b"\x1b[1;1H"); // home
    feed(&mut t, b"\x1b9"); // DECFI
    // Grid content unchanged; cursor moved forward by one column.
    let row = &t.grid()[crate::index::Line(0)];
    assert_eq!(row[Column(0)].ch, 'A');
    assert_eq!(row[Column(1)].ch, 'B');
    assert_eq!(row[Column(2)].ch, 'C');
    assert_eq!(t.grid().cursor().col().0, 1);
}
