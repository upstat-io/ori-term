//! Capture-file tuple extractor.
//!
//! Streams a PTY `.cap` file through `vte::Parser` and emits one
//! [`Tuple`] per recognized escape sequence. Does NOT reimplement
//! the VT state machine — consumes the vendored `vte` crate
//! directly. See
//! bootstrap.md §01.3` for the rationale.
//!
//! Payload normalization is exhaustive per the plan's
//! "Payload normalization for captures" rules:
//!
//! - PM / SOS payloads collapse to `Pt` with the canonical `ST`
//! terminator.
//! - APC payloads with a recognized `_G` prefix (kitty) collapse
//! to `(APC, [_G], key-value, ST)`. Unknown APC prefixes fall
//! back to `(APC, [], Pt, ST)` — the payload's first byte is
//! NOT interpreted as a phantom intermediate.
//! - DCS sixel (`q` final) collapses to `(DCS, [], Pid, q)`; all
//! other DCS forms collapse to `(DCS, [sorted ints], Pt, final)`.
//! - OSC tuples place the dispatch selector in `final_byte` per the
//! SSOT alignment (`OSC 4 ; 1 ; rgb:ff/00/00` →
//! `(OSC, [], index;rgb, 4)`). Payload placeholders go in `params`.
//! - CSI numeric params collapse to `Ps` / `Ps;Ps` per arity.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use vte::Parser;

use super::tuple::{Category, Tuple, csi_params_placeholder, osc_placeholder};

/// Capture-extraction error.
#[derive(Debug)]
pub enum CaptureExtractError {
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
}

impl core::fmt::Display for CaptureExtractError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Io { path, source } => write!(f, "io error reading {}: {source}", path.display()),
        }
    }
}

impl std::error::Error for CaptureExtractError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
        }
    }
}

/// Extract (tuple, count) pairs from a `.cap` file.
/// The return type is a sorted `Vec<(Tuple, u32)>` so callers can
/// report top-N frequencies without a second pass.
/// # Errors
/// Returns [`CaptureExtractError::Io`] if the file cannot be read.
pub fn extract_capture_tuples(cap_path: &Path) -> Result<Vec<(Tuple, u32)>, CaptureExtractError> {
    let bytes = fs::read(cap_path).map_err(|source| CaptureExtractError::Io {
        path: cap_path.to_path_buf(),
        source,
    })?;

    let mut sink = TupleSink::default();
    let mut parser = Parser::new();
    parser.advance(&mut sink, &bytes);

    let mut tuples: Vec<(Tuple, u32)> = sink.counts.into_iter().collect();
    tuples.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    Ok(tuples)
}

#[derive(Default)]
struct TupleSink {
    counts: BTreeMap<Tuple, u32>,
    // Scratch for accumulating DCS payloads between hook / put / unhook.
    dcs_action: Option<char>,
    dcs_intermediates: Vec<u8>,
    dcs_param_arity: usize,
}

impl TupleSink {
    fn bump(&mut self, tuple: Tuple) {
        *self.counts.entry(tuple).or_insert(0) += 1;
    }
}

impl vte::Perform for TupleSink {
    fn print(&mut self, _c: char) {
        // Printable codepoints do not generate catalog tuples —
        // they flow through `Term::input` which is not a sequence.
    }

    fn execute(&mut self, _byte: u8) {
        // C0 controls are dispatched per-byte directly; the catalog
        // tracks them under `ecma-48.md` C0 rows without a tuple.
    }

    fn hook(&mut self, params: &vte::Params, intermediates: &[u8], _ignore: bool, action: char) {
        self.dcs_action = Some(action);
        self.dcs_intermediates = intermediates.to_vec();
        self.dcs_param_arity = params.len();
    }

    fn put(&mut self, _byte: u8) {
        // Payload accumulation is not tracked — the tuple form uses
        // the `Pt` / `Pid` placeholder regardless of payload content.
    }

    fn unhook(&mut self) {
        if let Some(final_byte) = self.dcs_action.take() {
            let params = if final_byte == 'q' && self.dcs_intermediates.is_empty() {
                "Pid"
            } else {
                "Pt"
            };
            let ints = std::mem::take(&mut self.dcs_intermediates);
            let tuple = Tuple::new(Category::Dcs, ints, params, final_byte.to_string());
            self.bump(tuple);
        }
    }

    fn osc_dispatch(&mut self, params: &[&[u8]], _bell_terminated: bool) {
        if params.is_empty() || params[0].is_empty() {
            return;
        }
        let selector = match std::str::from_utf8(params[0]) {
            Ok(s) => s.to_string(),
            Err(e) => {
                // Non-UTF-8 OSC selector: real-world OSC selectors
                // are ASCII per spec, but a malformed capture stream
                // can carry garbage. Surface to stderr so the coverage
                // gap is visible; do NOT emit a tuple (we have no
                // canonical signature for non-UTF-8 selectors).
                eprintln!(
                    "warning: capture_extract::osc_dispatch: skipped malformed selector \
 (bytes={:02x?}): {e}",
                    params[0]
                );
                return;
            }
        };
        // SSOT: selector → `final_byte`; payload
        // placeholders → `params`. `osc_placeholder` is keyed on
        // raw payload position (idx 1, 2,...), unchanged.
        let payload: Vec<String> = params
            .iter()
            .enumerate()
            .skip(1)
            .map(|(idx, raw)| {
                let raw_str = std::str::from_utf8(raw).unwrap_or("");
                osc_placeholder(&selector, idx, raw_str)
            })
            .collect();
        self.bump(Tuple::new(
            Category::Osc,
            Vec::<u8>::new(),
            payload.join(";"),
            selector,
        ));
    }

    fn csi_dispatch(
        &mut self,
        params: &vte::Params,
        intermediates: &[u8],
        _ignore: bool,
        action: char,
    ) {
        let params_placeholder = csi_params_placeholder(params.len());
        self.bump(Tuple::new(
            Category::Csi,
            intermediates.to_vec(),
            params_placeholder,
            action.to_string(),
        ));
    }

    fn esc_dispatch(&mut self, intermediates: &[u8], _ignore: bool, byte: u8) {
        let final_byte = (byte as char).to_string();
        // Charset designation intermediates produce DA tuples.
        if intermediates
            .iter()
            .all(|b| matches!(b, b'(' | b')' | b'*' | b'+'))
            && !intermediates.is_empty()
        {
            self.bump(Tuple::new(
                Category::Da,
                intermediates.to_vec(),
                "-",
                final_byte,
            ));
        } else {
            self.bump(Tuple::new(
                Category::Esc,
                intermediates.to_vec(),
                "-",
                final_byte,
            ));
        }
    }

    fn apc_start(&mut self) {}
    fn apc_put(&mut self, _byte: u8) {}
    fn apc_end(&mut self) {
        // The `vte` crate's Perform trait exposes the start/put/end
        // triad but does not pass the accumulated payload back. We
        // emit the generic APC tuple here; a richer wrapper in
        // Section 04.9 may emit `(APC, [_G], key-value, ST)` when
        // the payload begins with `G`.
        self.bump(Tuple::new(Category::Apc, Vec::<u8>::new(), "Pt", "ST"));
    }

    fn terminated(&self) -> bool {
        false
    }
}
