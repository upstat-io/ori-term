//! Parser helpers for tack's `s) ANSI status reports` sub-submenu
//! walker (Section 06.1).
//!
//! Unlike the stable-screen scenarios in Sections 04/05, the
//! status-reports walker batches sub-tests per `n` press (see
//! `super::status_reports_inventory` for the empirical context).
//! The walker test in `oriterm_core/tests/tack/tools_menu/status_reports.rs`
//! captures every grid snapshot and hands the history to the helpers
//! below, which locate each curated sub-test's label and extract the
//! response region on the SAME line as the label (tack's format:
//! `(PROTO) label (QUERY) RESPONSE`, with the response padded
//! to a fixed column and separated from the label by ≥2 spaces).

/// Shape of a curated response-check row.
///
/// Each entry wires a sub-test label from
/// `super::status_reports_inventory` to a response-content predicate
/// that the walker asserts against the extracted response region.
pub struct HasResponseExpectation {
    /// The sub-test label (unique substring of tack's header row,
    /// matching one of the entries in `STATUS_REPORTS_INVENTORY`).
    pub label: &'static str,
    /// Short mnemonic mirroring the inventory entry — used in
    /// failure diagnostics to locate the sub-test by its friendly
    /// name rather than the substring label.
    pub mnemonic: &'static str,
    /// Predicate that inspects the extracted response region and
    /// returns `true` if the content is recognizable as a valid
    /// response for this sub-test (e.g. DA1 starts with `^[[?`
    /// which is how the grid renders ESC `[` `?`).
    pub response_check: fn(&str) -> bool,
}

/// Tack renders `\x1b` as the two-character visible form `^[` in
/// its status-reports response column — it's NOT a real escape
/// byte in the captured grid, it's tack's literal rendering of
/// the control char for human inspection. Predicates check for
/// this literal form.
const ESC_PREFIX: &str = "^[";

/// Curated response-content expectations for Section 06.1.
///
/// Each predicate asserts a content-specific marker unique to the
/// sub-test's response format under `ori_term` terminfo, verified
/// against the Section 06.1 diagnostic walker snapshot captured
/// during implementation (2026-04-08):
///
/// - DA1 (Primary device attributes) → `^[[?64;6;4c`
/// - DSR Terminal status → `^[[0n`
/// - DSR Cursor position → `^[[7;50R` (row;col R)
/// - DA2 (Secondary device attrs) → `^[[>0;200;1c`
/// - DA3 (Tertiary device attrs) → `^[P!|00000000^[\` (DCS P ! | … ST)
pub const STATUS_REPORTS_RESPONSE_EXPECTATIONS: &[HasResponseExpectation] = &[
    HasResponseExpectation {
        label: "Primary device attributes",
        mnemonic: "da1",
        response_check: is_primary_da_response,
    },
    HasResponseExpectation {
        label: "Terminal status",
        mnemonic: "dsr_status",
        response_check: is_dsr_terminal_status_response,
    },
    HasResponseExpectation {
        label: "Cursor position",
        mnemonic: "dsr_cpr",
        response_check: is_dsr_cursor_position_response,
    },
    HasResponseExpectation {
        label: "Secondary device attributes",
        mnemonic: "da2",
        response_check: is_secondary_da_response,
    },
    HasResponseExpectation {
        label: "Tertiary device attributes",
        mnemonic: "da3",
        response_check: is_tertiary_da_response,
    },
];

/// DA1 canonical response: `^[[?<params>c`. `ori_term` advertises
/// `^[[?64;6;4c` (VT420 level with selective erase + REGIS + sixel).
#[must_use]
pub fn is_primary_da_response(response: &str) -> bool {
    let trimmed = response.trim();
    trimmed.starts_with(&format!("{ESC_PREFIX}[?")) && trimmed.ends_with('c')
}

/// DSR terminal status canonical response: `^[[0n` (OK) or
/// `^[[3n` (not OK).
#[must_use]
pub fn is_dsr_terminal_status_response(response: &str) -> bool {
    let trimmed = response.trim();
    trimmed.starts_with(&format!("{ESC_PREFIX}[")) && trimmed.ends_with('n')
}

/// DSR cursor position canonical response: `^[[<row>;<col>R`.
#[must_use]
pub fn is_dsr_cursor_position_response(response: &str) -> bool {
    let trimmed = response.trim();
    trimmed.starts_with(&format!("{ESC_PREFIX}["))
        && trimmed.contains(';')
        && trimmed.ends_with('R')
}

/// DA2 canonical response: `^[[><type>;<fw>;<cart>c`.
#[must_use]
pub fn is_secondary_da_response(response: &str) -> bool {
    let trimmed = response.trim();
    trimmed.starts_with(&format!("{ESC_PREFIX}[>")) && trimmed.ends_with('c')
}

/// DA3 canonical response: DCS `!|` <unit-id-hex> ST. Rendered by
/// the grid as `^[P!|<hex>^[\` (DCS P ! | … ESC \).
#[must_use]
pub fn is_tertiary_da_response(response: &str) -> bool {
    let trimmed = response.trim();
    trimmed.starts_with(&format!("{ESC_PREFIX}P!|"))
        && trimmed.ends_with(&format!("{ESC_PREFIX}\\"))
}

/// Scan the walker `history` for the first snapshot containing `label`.
///
/// Returns the response column that follows the label on the same line.
/// Returns `None` if the label was never observed OR if the response
/// column is empty (tack's "illegal response" marker leaves the column
/// blank).
#[must_use]
pub fn extract_response_for_label(history: &[String], label: &str) -> Option<String> {
    for snapshot in history {
        if let Some(response) = extract_response_from_snapshot(snapshot, label) {
            return Some(response);
        }
    }
    None
}

/// Scan `snapshot` for the first line containing `label`, split the
/// line on runs of ≥2 spaces, and return the last non-empty segment
/// as the sub-test's response. Tack's status-reports walker emits
/// each sub-test on one line of the form
/// `(PROTO) label (QUERY)<spaces><RESPONSE>` under `ori_term`
/// terminfo — the last segment after the gap is the response value
/// tack captured from the VTE.
pub(crate) fn extract_response_from_snapshot(snapshot: &str, label: &str) -> Option<String> {
    let line = snapshot.lines().find(|line| line.contains(label))?;
    let response = last_segment_after_gap(line)?;
    if response.is_empty() {
        None
    } else {
        Some(response.to_string())
    }
}

/// Return the substring of `line` that follows the rightmost run
/// of ≥2 consecutive spaces. Trailing padding is stripped first,
/// so a label row with only a padding gap (tack's "illegal
/// response" rows, which leave the response column blank) returns
/// `None`. Used by [`extract_response_from_snapshot`] to isolate
/// tack's response column from its `(PROTO) label (QUERY)` prefix.
pub(crate) fn last_segment_after_gap(line: &str) -> Option<&str> {
    let trimmed = line.trim_end();
    if trimmed.len() < 2 {
        return None;
    }
    let bytes = trimmed.as_bytes();
    let mut rightmost_gap_end: Option<usize> = None;
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b' ' {
            let start = i;
            while i < bytes.len() && bytes[i] == b' ' {
                i += 1;
            }
            if i - start >= 2 {
                rightmost_gap_end = Some(i);
            }
        } else {
            i += 1;
        }
    }
    let gap_end = rightmost_gap_end?;
    // `gap_end` is byte-aligned to ASCII space boundaries by the
    // scan above (we only advance past b' ' when rebuilding the
    // run), so this slice is always UTF-8 safe.
    trimmed.get(gap_end..)
}

#[cfg(test)]
mod tests;
