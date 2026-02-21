//! Paste text processing for terminal input.
//!
//! Pure functions that transform clipboard text before sending to the PTY:
//! character filtering, line ending normalization, ESC stripping, and
//! bracketed paste wrapping.

use std::path::Path;

/// Bracketed paste mode start sequence (XTERM DECSET 2004).
const BRACKET_START: &[u8] = b"\x1b[200~";

/// Bracketed paste mode end sequence.
const BRACKET_END: &[u8] = b"\x1b[201~";

/// Filter special characters from pasted text (Windows Terminal `FilterOnPaste`).
///
/// Applies the following transformations:
/// - Tab (`\t`) → stripped
/// - Non-breaking space (U+00A0, U+202F) → regular space
/// - Smart double quotes (U+201C, U+201D) → straight double quote (`"`)
/// - Smart single quotes (U+2018, U+2019) → straight single quote (`'`)
/// - Em-dash (U+2014) → double hyphen (`--`)
/// - En-dash (U+2013) → single hyphen (`-`)
pub fn filter_paste(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for ch in text.chars() {
        match ch {
            '\t' => {}
            '\u{00A0}' | '\u{202F}' => out.push(' '),
            '\u{201C}' | '\u{201D}' => out.push('"'),
            '\u{2018}' | '\u{2019}' => out.push('\''),
            '\u{2014}' => out.push_str("--"),
            '\u{2013}' => out.push('-'),
            _ => out.push(ch),
        }
    }
    out
}

/// Normalize line endings for terminal input.
///
/// Converts Windows CRLF (`\r\n`) to CR (`\r`). Standalone `\n` is also
/// converted to `\r` (terminals expect CR for newline input). Standalone
/// `\r` passes through unchanged.
pub fn normalize_line_endings(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(ch) = chars.next() {
        match ch {
            '\r' => {
                out.push('\r');
                // Consume the LF in a CRLF pair.
                if chars.peek() == Some(&'\n') {
                    chars.next();
                }
            }
            '\n' => {
                // Standalone LF → CR.
                out.push('\r');
            }
            _ => out.push(ch),
        }
    }
    out
}

/// Strip ESC (`\x1b`) characters from text.
///
/// Used inside bracketed paste to prevent applications from receiving
/// escape sequences that could break out of the paste context.
pub fn strip_escape_chars(text: &str) -> String {
    text.chars().filter(|&ch| ch != '\x1b').collect()
}

/// Count the number of newlines in pasted text.
///
/// Counts both `\n` and `\r` (but CRLF counts as one newline).
/// Returns the number of lines minus one (i.e. a single-line paste returns 0).
pub fn count_newlines(text: &str) -> usize {
    let bytes = text.as_bytes();
    let mut count = 0;
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'\r' && i + 1 < bytes.len() && bytes[i + 1] == b'\n' {
            count += 1;
            i += 2;
        } else if bytes[i] == b'\r' || bytes[i] == b'\n' {
            count += 1;
            i += 1;
        } else {
            i += 1;
        }
    }
    count
}

/// Prepare text for pasting into the terminal.
///
/// Applies the full paste processing pipeline:
/// 1. Character filtering (if `filter` is true)
/// 2. Line ending normalization (CRLF → CR, LF → CR)
/// 3. ESC stripping (if `bracketed` is true)
/// 4. Bracketed paste wrapping (if `bracketed` is true)
///
/// Returns the raw bytes to send to the PTY.
pub fn prepare_paste(text: &str, bracketed: bool, filter: bool) -> Vec<u8> {
    // Step 1: Character filtering.
    let filtered = if filter {
        filter_paste(text)
    } else {
        text.to_owned()
    };

    // Step 2: Line ending normalization.
    let normalized = normalize_line_endings(&filtered);

    // Step 3+4: ESC stripping and bracketed paste wrapping.
    if bracketed {
        let stripped = strip_escape_chars(&normalized);
        let mut buf = Vec::with_capacity(BRACKET_START.len() + stripped.len() + BRACKET_END.len());
        buf.extend_from_slice(BRACKET_START);
        buf.extend_from_slice(stripped.as_bytes());
        buf.extend_from_slice(BRACKET_END);
        buf
    } else {
        normalized.into_bytes()
    }
}

/// Format file paths for pasting into the terminal.
///
/// Paths containing spaces are wrapped in double quotes. Multiple paths
/// are space-separated.
pub fn format_dropped_paths(paths: &[&Path]) -> String {
    let mut out = String::new();
    for (i, path) in paths.iter().enumerate() {
        if i > 0 {
            out.push(' ');
        }
        let s = path.display().to_string();
        if s.contains(' ') {
            out.push('"');
            out.push_str(&s);
            out.push('"');
        } else {
            out.push_str(&s);
        }
    }
    out
}

#[cfg(test)]
mod tests;
