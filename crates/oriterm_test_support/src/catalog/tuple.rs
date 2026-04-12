//! Canonical tuple form for escape sequences.
//!
//! The tuple `(category, intermediates, params, final_byte)` is the
//! common currency across dispatch extraction, catalog parsing, and
//! capture extraction. The same sequence is written identically
//! regardless of which extractor produced it, so set-equality on
//! `Vec<Tuple>` answers "does this row cover this dispatch arm?".
//!
//! See `plans/spec-conformance/section-01-catalog-bootstrap.md §01.3.a`
//! for the full rules.

use core::fmt::{self, Display, Write as _};

/// Escape sequence category — the top-level discriminator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Category {
    /// C0 control code (`0x00`–`0x1F`).
    C0,
    /// C1 control code (`0x80`–`0x9F`).
    C1,
    /// 7-bit ESC sequence, no CSI (`ESC <final>` or `ESC <intermediate> <final>`).
    Esc,
    /// Control Sequence Introducer (`CSI … <final>`).
    Csi,
    /// Operating System Command (`OSC <numeric>; <payload> BEL|ST`).
    Osc,
    /// Device Control String (`DCS … <final> … ST`).
    Dcs,
    /// Application Program Command (`APC <key=value>; <payload> ST`).
    Apc,
    /// Privacy Message (`PM … ST`).
    Pm,
    /// Start Of String (`SOS … ST`).
    Sos,
    /// Charset designation (`ESC ( <id>`, etc.).
    Da,
}

impl Display for Category {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::C0 => "C0",
            Self::C1 => "C1",
            Self::Esc => "ESC",
            Self::Csi => "CSI",
            Self::Osc => "OSC",
            Self::Dcs => "DCS",
            Self::Apc => "APC",
            Self::Pm => "PM",
            Self::Sos => "SOS",
            Self::Da => "DA",
        };
        f.write_str(s)
    }
}

/// Canonical tuple form for a single escape sequence.
///
/// - `category` — see [`Category`].
/// - `intermediates` — sorted byte sequence (`?`, `>`, `$`, ...).
/// - `params` — normalized parameter placeholder (`Ps`, `Ps;Ps`, `text`, …).
/// - `final_byte` — the dispatch-triggering byte, or the canonical
///   terminator for string-family sequences (`ST` for PM/SOS/APC).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Tuple {
    pub category: Category,
    pub intermediates: Vec<u8>,
    pub params: String,
    pub final_byte: String,
}

impl Tuple {
    /// Construct a tuple with already-canonicalized components.
    ///
    /// Intermediates are sorted in-place so that `[b'?', b'$']` and
    /// `[b'$', b'?']` produce the same tuple. Callers passing a
    /// pre-sorted slice pay only the comparison cost.
    pub fn new(
        category: Category,
        intermediates: impl Into<Vec<u8>>,
        params: impl Into<String>,
        final_byte: impl Into<String>,
    ) -> Self {
        let mut intermediates = intermediates.into();
        intermediates.sort_unstable();
        Self {
            category,
            intermediates,
            params: params.into(),
            final_byte: final_byte.into(),
        }
    }
}

impl Display for Tuple {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({}, [", self.category)?;
        for (i, b) in self.intermediates.iter().enumerate() {
            if i > 0 {
                f.write_str(", ")?;
            }
            if b.is_ascii_graphic() {
                f.write_char(*b as char)?;
            } else {
                write!(f, "0x{b:02x}")?;
            }
        }
        write!(f, "], {}, {})", self.params, self.final_byte)
    }
}

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
    // Strip backticks, then trim whitespace.
    let s = sequence.trim().trim_matches('`').trim();

    // Handle `CSI … h` / `CSI … l` pair form by taking the first alternative
    // — the catalog mechanically expands to paired `h`/`l` tuples at check
    // time via `parser::row_to_tuples`.
    let s = s.split(" / ").next().unwrap_or(s);
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

fn parse_csi(rest: &str) -> Option<Tuple> {
    // CSI forms:
    //   "? Ps h"            → (CSI, [?], Ps, h)
    //   "Ps ; Ps H"         → (CSI, [], Ps;Ps, H)
    //   "Ps SP q"           → (CSI, [SP], Ps, q)  -- SP is literal space intermediate
    //   "> 4 ; Ps m"        → (CSI, [>], Ps, m)
    //   "? Ps $ p"          → (CSI, [$?], Ps, p)
    //   "c" / "0 c"         → (CSI, [], -, c)
    //
    // Strategy: tokenize into (tokens: Vec<&str>, last), then classify.
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
                // SCP / DECSCUSR use a literal space intermediate.
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
    // Join tokens, then collapse Ps-like placeholders on `;`.
    let joined = tokens.join(" ");
    // Canonicalize `Ps ; Ps` → `Ps;Ps`.
    let cleaned: String = joined.chars().filter(|c| !c.is_whitespace()).collect();
    if cleaned.is_empty() {
        "-".to_string()
    } else {
        cleaned
    }
}

fn parse_osc(rest: &str) -> Option<Tuple> {
    // OSC forms:
    //   "0 ; Pt BEL|ST"       → (OSC, [], 0;text, BEL)
    //   "4 ; index ; rgb BEL|ST" → (OSC, [], 4;index;rgb, BEL)
    //   "52 ; Pc ; <b64> BEL|ST" → (OSC, [], 52;mode;b64, BEL)
    //
    // Take the segment before the terminator as the payload.
    let (payload, terminator) = split_terminator(rest);
    let payload = payload.trim();
    // Split on `;` preserving the numeric id.
    let parts: Vec<&str> = payload.split(';').map(str::trim).collect();
    if parts.is_empty() {
        return None;
    }
    let mut canonical_parts: Vec<String> = Vec::with_capacity(parts.len());
    for (i, p) in parts.iter().enumerate() {
        if i == 0 {
            // Numeric id is preserved literally (see OSC numeric-id rule).
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
    // Dispatch placeholder on the numeric id + position.
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
    // DCS forms the catalog uses:
    //   "Ps1 ; Ps2 ; Ps3 q <data> ST"  → (DCS, [], Pid, q)  -- sixel
    //   "$ q Pt ST"                    → (DCS, [$], Pt, q)  -- DECRQSS
    //   "! | Pt ST"                    → (DCS, [!], Pt, |)  -- DECUDK
    //
    // Strategy: scan tokens for (a) single-char intermediate markers
    // `$`, `!`, `"`, `#` and (b) the first dispatch final byte
    // (`q`, `|`, `p`, `r`). Everything else is parameter text.
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
    // APC forms:
    //   "G key=value ; <data> ST"  → (APC, [_G], key-value, ST)
    //   "_G key=value ; <data> ST" → (APC, [_G], key-value, ST)
    let rest = rest.trim_start_matches('_').trim_start();
    let (_payload, terminator) = split_terminator(rest);
    if rest.starts_with('G') {
        Tuple::new(Category::Apc, [b'_', b'G'], "key-value", terminator)
    } else {
        Tuple::new(Category::Apc, Vec::<u8>::new(), "Pt", terminator)
    }
}

fn parse_esc(rest: &str) -> Option<Tuple> {
    // Forms:
    //   "^ Pt ST"  → (PM, [], Pt, ST)
    //   "X Pt ST"  → (SOS, [], Pt, ST)
    //   "_ G ... ST" → (APC, [_G], key-value, ST)
    //   "( B"      → (DA, [(], -, B)
    //   "D"        → (ESC, [], -, D)   -- IND
    //   "# 8"      → (ESC, [#], -, 8)  -- DECALN
    let tokens: Vec<&str> = rest.split_whitespace().collect();
    if tokens.is_empty() {
        return None;
    }
    match tokens[0] {
        "^" => Some(Tuple::new(Category::Pm, Vec::<u8>::new(), "Pt", "ST")),
        "X" => Some(Tuple::new(Category::Sos, Vec::<u8>::new(), "Pt", "ST")),
        "_" => Some(Tuple::new(Category::Apc, [b'_', b'G'], "key-value", "ST")),
        "(" | ")" | "*" | "+" => {
            // Charset designation.
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
    // Find the last ST / BEL / BEL|ST / BEL\|ST token in the sequence.
    // Markdown escapes `\|` inside a table cell but by the time the
    // parser has unescaped the cell we see a literal pipe.
    //
    // `split_once` from the right gives us a borrow pair without
    // string-index slicing (which clippy flags as potentially
    // non-UTF-8-safe even though our terminators are ASCII).
    for term in ["BEL|ST", "BEL", "ST", "0x9C"] {
        if let Some((payload, _after)) = rest.rsplit_once(term) {
            // Prefer BEL as canonical for OSC alternatives — it's
            // the form the catalog uses when both are valid.
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
