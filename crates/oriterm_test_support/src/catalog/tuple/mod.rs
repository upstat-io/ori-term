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

mod canonical;

use core::fmt::{self, Display, Write as _};

pub use canonical::canonical_tuple;

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

/// Parse a category string back to [`Category`].
///
/// Accepts the canonical display form: `C0`, `C1`, `ESC`, `CSI`,
/// `OSC`, `DCS`, `APC`, `PM`, `SOS`, `DA`.
impl Category {
    pub fn from_display_str(s: &str) -> Option<Self> {
        match s.trim() {
            "C0" => Some(Self::C0),
            "C1" => Some(Self::C1),
            "ESC" => Some(Self::Esc),
            "CSI" => Some(Self::Csi),
            "OSC" => Some(Self::Osc),
            "DCS" => Some(Self::Dcs),
            "APC" => Some(Self::Apc),
            "PM" => Some(Self::Pm),
            "SOS" => Some(Self::Sos),
            "DA" => Some(Self::Da),
            _ => None,
        }
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

    /// Parse a tuple from the canonical [`Display`] form.
    ///
    /// ```text
    /// (CSI, [?], Ps, h)
    /// (OSC, [], 4;index;rgb, BEL)
    /// (CSI, [0x20], Ps, q)
    /// ```
    ///
    /// Accepts both single-character intermediates (`?`, `$`) and
    /// hex-encoded bytes (`0x20`, `0x3f`). Whitespace around
    /// delimiters is tolerated.
    pub fn from_display_str(s: &str) -> Option<Self> {
        let s = s.trim();
        let inner = s.strip_prefix('(')?.strip_suffix(')')?;
        // Category: everything up to the first `, [`
        let (cat_str, rest) = inner.split_once(", [")?;
        // Intermediates: everything up to the `]`
        let (int_str, rest) = rest.split_once("], ")?;
        // Params and final_byte: split at the LAST `, `
        let (params, final_byte) = rest.rsplit_once(", ")?;

        let category = Category::from_display_str(cat_str)?;
        let intermediates = parse_display_intermediates(int_str)?;
        Some(Self::new(
            category,
            intermediates,
            params.trim(),
            final_byte.trim(),
        ))
    }
}

/// Parse the intermediates list from display form.
///
/// Accepts: empty string, comma-separated graphic chars (`?`, `$`),
/// hex bytes (`0x20`), or multi-char intermediates (`_G` → `[b'_', b'G']`).
fn parse_display_intermediates(s: &str) -> Option<Vec<u8>> {
    let s = s.trim();
    if s.is_empty() {
        return Some(Vec::new());
    }
    let mut out = Vec::new();
    for part in s.split(',') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        if let Some(hex) = part.strip_prefix("0x") {
            let b = u8::from_str_radix(hex, 16).ok()?;
            out.push(b);
        } else {
            // Each character in the part is a separate intermediate byte.
            for ch in part.chars() {
                if ch.is_ascii() {
                    out.push(ch as u8);
                } else {
                    return None;
                }
            }
        }
    }
    Some(out)
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
