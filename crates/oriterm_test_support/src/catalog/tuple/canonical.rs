//! Sequence-column → [`Tuple`] canonicalizer.
//!
//! [`canonical_tuple`] parses the catalog's `Sequence` column
//! string (e.g., `` `CSI Ps;Ps H` ``) into a canonical [`Tuple`].
//! The function handles backtick wrapping, mnemonic suffixes like
//! `(CUP)`, and the `h / l` pair form used by DECSET/DECRST rows.

use super::{Category, Tuple};

/// Canonicalize a `Sequence`-column string into a [`Tuple`].
///
/// Accepted shapes (non-exhaustive — see `§01.3.a`):
///
/// - `CSI Ps;Ps H` → `(CSI, [], Ps;Ps, H)`
/// - `CSI ? Ps h` → `(CSI, [?], Ps, h)`
/// - `CSI Ps SP q` → `(CSI, [SP], Ps, q)`
/// - `ESC ^ Pt ST` → `(PM, [], Pt, ST)`
/// - `ESC X Pt ST` → `(SOS, [], Pt, ST)`
/// - `DCS $ q Pt ST` → `(DCS, [$], Pt, q)`
/// - `OSC 4 ; index ; spec BEL|ST` → `(OSC, [], 4;index;rgb, BEL)`
///
/// Returns `None` for sequences the canonicalizer does not recognize.
/// The `None` path is never taken by catalog rows that pass
/// [`parser::parse_catalog_markdown`] because the parser rejects rows
/// whose `Sequence` column is empty or malformed.
pub fn canonical_tuple(sequence: &str) -> Option<Tuple> {
    // Extract the content of the innermost code span. The catalog's
    // Sequence column wraps the actual sequence notation in backticks
    // (sometimes double-backtick `` `` `…` `` ``) and may append a
    // mnemonic suffix like `(CUP)` outside the code span. We want
    // ONLY what's inside the backticks.
    let s = extract_code_span_content(sequence);

    // Handle `CSI … h` / `CSI … l` pair form by taking the first alternative
    // — the catalog mechanically expands to paired `h`/`l` tuples at check
    // time via `parser::row_to_tuples`.
    let s = s.split(" / ").next().unwrap_or(&s);
    let s = s.split('/').next().unwrap_or(s).trim();
    let s = s.trim_matches('`').trim();

    if let Some(rest) = s.strip_prefix("CSI ") {
        return parse_csi(rest);
    }
    if let Some(rest) = s.strip_prefix("OSC ") {
        return parse_osc(rest);
    }
    if let Some(rest) = s.strip_prefix("DCS ") {
        return parse_dcs(rest);
    }
    if let Some(rest) = s.strip_prefix("APC ") {
        return Some(parse_apc_sequence(rest));
    }
    if let Some(rest) = s.strip_prefix("ESC ") {
        return parse_esc(rest);
    }
    if matches!(s, "BEL") || s.starts_with("C0") {
        // C0 controls are not canonicalized as tuples — they go through
        // `Performer::execute` directly and are not match-arm-driven
        // from the catalog's point of view.
        return None;
    }

    None
}

/// Extract the content from a backtick-delimited code span.
///
/// Handles:
/// - `` `CSI Ps H` `` → `CSI Ps H`
/// - `` `` `CSI Ps H` `` (CUP) `` → `CSI Ps H`
/// - `CSI Ps H` (no backticks) → `CSI Ps H`
///
/// Returns the innermost code-span content, or the trimmed input
/// if no backticks are present.
fn extract_code_span_content(s: &str) -> String {
    let s = s.trim();
    // Walk forward to the first backtick, then backward from the
    // end to the last backtick. Content between those two positions
    // is the code-span body. Byte-index slicing is safe because
    // backticks are ASCII (single-byte, never part of a multi-byte
    // UTF-8 sequence).
    let first_tick = s.find('`');
    let last_tick = s.rfind('`');
    match (first_tick, last_tick) {
        (Some(f), Some(l)) if f < l => {
            let inner = s.get(f + 1..l).unwrap_or(s);
            let inner = inner.trim().trim_matches('`').trim();
            inner.to_string()
        }
        _ => s.to_string(),
    }
}

fn parse_csi(rest: &str) -> Option<Tuple> {
    let tokens: Vec<&str> = rest.split_whitespace().collect();
    if tokens.is_empty() {
        return None;
    }
    let (final_token, body) = tokens.split_last()?;
    if final_token.len() != 1 {
        return None;
    }
    let final_byte = final_token.to_string();

    let mut intermediates: Vec<u8> = Vec::new();
    let mut params_tokens: Vec<&str> = Vec::new();
    for tok in body {
        match *tok {
            "?" | ">" | "=" | "!" | "\"" | "#" | "$" | "%" | "&" | "(" | ")" | "*" | "+" => {
                intermediates.push(tok.as_bytes()[0]);
            }
            "SP" => {
                intermediates.push(b' ');
            }
            _ => params_tokens.push(tok),
        }
    }

    let params = normalize_csi_params(&params_tokens);
    Some(Tuple::new(Category::Csi, intermediates, params, final_byte))
}

fn normalize_csi_params(tokens: &[&str]) -> String {
    if tokens.is_empty() {
        return "-".to_string();
    }
    let joined = tokens.join(" ");
    let cleaned: String = joined.chars().filter(|c| !c.is_whitespace()).collect();
    if cleaned.is_empty() {
        "-".to_string()
    } else {
        cleaned
    }
}

fn parse_osc(rest: &str) -> Option<Tuple> {
    let (payload, terminator) = split_terminator(rest);
    let payload = payload.trim();
    let parts: Vec<&str> = payload.split(';').map(str::trim).collect();
    if parts.is_empty() {
        return None;
    }
    let mut canonical_parts: Vec<String> = Vec::with_capacity(parts.len());
    for (i, p) in parts.iter().enumerate() {
        if i == 0 {
            canonical_parts.push(p.to_string());
        } else {
            canonical_parts.push(placeholder_for_osc_part(parts[0], i, p));
        }
    }
    Some(Tuple::new(
        Category::Osc,
        Vec::<u8>::new(),
        canonical_parts.join(";"),
        terminator,
    ))
}

fn placeholder_for_osc_part(numeric_id: &str, idx: usize, raw: &str) -> String {
    match numeric_id {
        "0" | "1" | "2" | "7" | "22" | "50" | "l" | "L" => "text".to_string(),
        "4" => match idx {
            1 => "index".to_string(),
            2 => {
                if raw.contains('?') {
                    "?".to_string()
                } else {
                    "rgb".to_string()
                }
            }
            _ => raw.to_string(),
        },
        "10" | "11" | "12" => {
            if raw.contains('?') {
                "?".to_string()
            } else {
                "rgb".to_string()
            }
        }
        "8" => {
            if idx == 1 {
                "params".to_string()
            } else {
                "uri".to_string()
            }
        }
        "52" => match idx {
            1 => "mode".to_string(),
            2 => {
                if raw.contains('?') {
                    "?".to_string()
                } else {
                    "b64".to_string()
                }
            }
            _ => raw.to_string(),
        },
        "104" => "index".to_string(),
        "1337" => "key=value".to_string(),
        _ => raw.to_string(),
    }
}

fn parse_dcs(rest: &str) -> Option<Tuple> {
    let (payload, _terminator) = split_terminator(rest);
    let tokens: Vec<&str> = payload.split_whitespace().collect();
    if tokens.is_empty() {
        return None;
    }

    let mut intermediates: Vec<u8> = Vec::new();
    let mut final_byte: Option<char> = None;
    for tok in &tokens {
        if tok.len() == 1 {
            let b = tok.as_bytes()[0];
            if matches!(b, b'$' | b'!' | b'"' | b'#') {
                intermediates.push(b);
                continue;
            }
            if matches!(b, b'q' | b'|' | b'p' | b'r') {
                final_byte = Some(b as char);
                break;
            }
        }
    }

    let final_byte = final_byte?.to_string();
    let params = if final_byte == "q" && intermediates.is_empty() {
        "Pid".to_string()
    } else {
        "Pt".to_string()
    };

    Some(Tuple::new(Category::Dcs, intermediates, params, final_byte))
}

fn parse_apc_sequence(rest: &str) -> Tuple {
    let rest = rest.trim_start_matches('_').trim_start();
    let (_payload, terminator) = split_terminator(rest);
    if rest.starts_with('G') {
        Tuple::new(Category::Apc, [b'_', b'G'], "key-value", terminator)
    } else {
        Tuple::new(Category::Apc, Vec::<u8>::new(), "Pt", terminator)
    }
}

fn parse_esc(rest: &str) -> Option<Tuple> {
    let tokens: Vec<&str> = rest.split_whitespace().collect();
    if tokens.is_empty() {
        return None;
    }
    match tokens[0] {
        "^" => Some(Tuple::new(Category::Pm, Vec::<u8>::new(), "Pt", "ST")),
        "X" => Some(Tuple::new(Category::Sos, Vec::<u8>::new(), "Pt", "ST")),
        "_" => Some(Tuple::new(Category::Apc, [b'_', b'G'], "key-value", "ST")),
        "(" | ")" | "*" | "+" => {
            if let Some(final_token) = tokens.get(1) {
                if final_token.len() == 1 {
                    return Some(Tuple::new(
                        Category::Da,
                        [tokens[0].as_bytes()[0]],
                        "-",
                        final_token.to_string(),
                    ));
                }
            }
            None
        }
        "#" => {
            if let Some(final_token) = tokens.get(1) {
                if final_token.len() == 1 {
                    return Some(Tuple::new(
                        Category::Esc,
                        [b'#'],
                        "-",
                        final_token.to_string(),
                    ));
                }
            }
            None
        }
        other => {
            if other.len() == 1 {
                Some(Tuple::new(
                    Category::Esc,
                    Vec::<u8>::new(),
                    "-",
                    other.to_string(),
                ))
            } else {
                None
            }
        }
    }
}

fn split_terminator(rest: &str) -> (&str, String) {
    for term in ["BEL|ST", "BEL", "ST", "0x9C"] {
        if let Some((payload, _after)) = rest.rsplit_once(term) {
            let canonical = if term == "ST" && !rest.contains("BEL") {
                "ST".to_string()
            } else {
                "BEL".to_string()
            };
            return (payload, canonical);
        }
    }
    (rest, "ST".to_string())
}
