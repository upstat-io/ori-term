//! Whitespace-bounded grid match helpers.
//!
//! Blind `grid.contains("am")` is dangerous: any 2-letter substring
//! will collide with longer words and column-aligned table cells.
//! [`grid_has_token`] matches `am` only when it appears as a whole
//! word — surrounded by ASCII whitespace, line edges, or grid edges.
//! [`grid_line_starts_with`] is for prompt markers that always start
//! a line. [`grid_find_field`] locates a labeled field and returns
//! the trailing value (the text after the label up to the next
//! whitespace run).
//!
//! Every per-scenario parser MUST use these helpers instead of
//! `str::contains` for capability labels, control-response markers,
//! or any short literal that could collide with a substring.

/// True iff `token` appears in `grid` as a whitespace-bounded word.
///
/// "Whitespace-bounded" means: each side of the match is either
/// ASCII whitespace, the start of `grid`, or the end of `grid`.
/// Punctuation does NOT count as a boundary — e.g., `grid_has_token`
/// for `"am"` against `"am,bce"` returns FALSE (the comma is not
/// whitespace). Tack draws cap names with surrounding spaces, so the
/// whitespace-only rule is correct for the modes/SGR screens.
///
/// O(`grid_len` * `token_len`) worst case, single-pass scan.
#[must_use]
pub fn grid_has_token(grid: &str, token: &str) -> bool {
    if token.is_empty() {
        return false;
    }
    let bytes = grid.as_bytes();
    let needle = token.as_bytes();
    let mut i = 0;
    while i + needle.len() <= bytes.len() {
        if &bytes[i..i + needle.len()] == needle {
            let left_ok = i == 0 || bytes[i - 1].is_ascii_whitespace();
            let right_end = i + needle.len();
            let right_ok = right_end == bytes.len() || bytes[right_end].is_ascii_whitespace();
            if left_ok && right_ok {
                return true;
            }
        }
        i += 1;
    }
    false
}

/// True iff any line in `grid` starts with `prefix` (after trimming
/// leading whitespace from the line). Used for prompt markers like
/// `tack [m] >` that always begin their line.
#[must_use]
pub fn grid_line_starts_with(grid: &str, prefix: &str) -> bool {
    grid.lines()
        .any(|line| line.trim_start().starts_with(prefix))
}

/// Locate a labeled field on the grid and return the trailing value.
///
/// Searches each line for `label` as a whitespace-bounded token,
/// then returns the text after `label` (trimmed, up to the next
/// whitespace run or end of line). Returns `None` if the label is
/// not present as a token. Used for cap-value extraction like
/// `grid_find_field(grid, "setaf")` returning `\E[3%dm`.
#[must_use]
pub fn grid_find_field<'a>(grid: &'a str, label: &str) -> Option<&'a str> {
    for line in grid.lines() {
        if let Some(idx) = line.find(label) {
            // Confirm it's a token, not a substring.
            let left_ok = idx == 0 || line.as_bytes()[idx - 1].is_ascii_whitespace();
            if !left_ok {
                continue;
            }
            let after = idx + label.len();
            let rest = line.get(after..)?.trim_start();
            if rest.is_empty() {
                return None;
            }
            let value = rest.split_whitespace().next().unwrap_or("");
            return Some(value);
        }
    }
    None
}
