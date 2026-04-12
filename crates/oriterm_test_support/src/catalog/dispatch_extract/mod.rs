//! Dispatch-arm + `NamedPrivateMode` tuple extractors.
//!
//! Walks `crates/vte/src/ansi/dispatch/{csi,osc,mod}.rs` and
//! `crates/vte/src/ansi/types.rs` via the `syn` AST parser and
//! emits one [`Tuple`] per output sequence (not per match arm —
//! 1-to-many dispatch arms expand mechanically, per Phase 2
//! Finding G).
//!
//! Two public extractors live here:
//!
//! - [`extract_dispatch_tuples`] — walks CSI / OSC / DCS / APC /
//!   PM / SOS dispatch arms.
//! - [`extract_namedprivatemode_tuples`] — walks
//!   `PrivateMode::new()`'s match arms to enumerate every
//!   supported DEC private mode number, emitting paired
//!   `(CSI, [?], Ps, h)` / `(CSI, [?], Ps, l)` tuples per
//!   variant.
//!
//! Disjointness invariant: `extract_dispatch_tuples` filters out
//! every `(CSI, [?], Ps, h|l)` tuple before emitting, so the
//! intersection of its output with `extract_namedprivatemode_tuples`
//! is empty. `--check` unions both sets.
//!
//! Organization (per `.claude/rules/code-hygiene.md` §File Size):
//! the walkers live in per-category submodules so no single file
//! approaches the 500-line limit:
//!
//! - `csi.rs` — CSI dispatch walker
//! - `osc.rs` — OSC dispatch walker
//! - `esc_dcs.rs` — ESC + DCS walker (driven off `dispatch/mod.rs`)
//! - `sgr.rs` — SGR numeric-parameter walker
//! - `private_mode.rs` — `NamedPrivateMode` match-arm walker
//!
//! Shared AST / IO helpers (`read_rust`, `parse_rust`,
//! `type_path_ends_with`, `pattern_char_literal`,
//! `pattern_byte_slice`, `is_action_intermediates_scrutinee`) live
//! in this file and are `pub(super)` so submodules can reuse them
//! without going through a wider public API.

mod csi;
mod esc_dcs;
mod osc;
mod private_mode;
mod sgr;

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use super::tuple::{Category, Tuple};

/// Error returned by the dispatch extractors.
#[derive(Debug)]
pub enum DispatchExtractError {
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
    Parse {
        path: PathBuf,
        source: syn::Error,
    },
}

impl core::fmt::Display for DispatchExtractError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Io { path, source } => write!(f, "io error reading {}: {source}", path.display()),
            Self::Parse { path, source } => {
                write!(f, "syn parse error in {}: {source}", path.display())
            }
        }
    }
}

impl std::error::Error for DispatchExtractError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            Self::Parse { source, .. } => Some(source),
        }
    }
}

/// Walk the VTE dispatch tree and emit the canonical dispatch tuple set.
///
/// Scans `crates/vte/src/ansi/dispatch/{csi,osc,mod}.rs` and
/// `crates/vte/src/lib.rs` relative to `workspace_root`. Filters
/// out DECSET/DECRST pairs — those come exclusively from
/// [`extract_namedprivatemode_tuples`] so the two extractors are
/// disjoint.
///
/// # Errors
///
/// Returns [`DispatchExtractError`] on io failures or `syn` parse
/// errors.
pub fn extract_dispatch_tuples(workspace_root: &Path) -> Result<Vec<Tuple>, DispatchExtractError> {
    let mut tuples: BTreeSet<Tuple> = BTreeSet::new();

    // Parse csi.rs once and pass the AST to both CSI and SGR walkers.
    let csi_path = workspace_root.join("crates/vte/src/ansi/dispatch/csi.rs");
    let csi_source = read_rust(&csi_path)?;
    let csi_file = parse_rust(&csi_path, &csi_source)?;
    csi::extract_csi_arms(&csi_file, &mut tuples);
    sgr::extract_sgr_params(&csi_file, &mut tuples);

    let osc_path = workspace_root.join("crates/vte/src/ansi/dispatch/osc.rs");
    osc::extract_osc_arms(&osc_path, &mut tuples)?;

    let mod_path = workspace_root.join("crates/vte/src/ansi/dispatch/mod.rs");
    esc_dcs::extract_mod_arms(&mod_path, &mut tuples)?;

    // APC dispatch is unconditional — `Performer::apc_end` calls
    // `handler.apc_dispatch(&payload)` for every APC sequence.
    // This isn't a match arm, so the visitors don't catch it.
    tuples.insert(Tuple::new(Category::Apc, Vec::<u8>::new(), "Pt", "ST"));

    // Filter out DECSET/DECRST tuples — those come exclusively from
    // `extract_namedprivatemode_tuples`. The disjointness invariant
    // is tested in sibling tests.
    Ok(tuples
        .into_iter()
        .filter(|t| {
            !(t.category == Category::Csi
                && t.intermediates == [b'?']
                && (t.final_byte == "h" || t.final_byte == "l"))
        })
        .collect())
}

/// Walk `crates/vte/src/ansi/types.rs::PrivateMode::new` relative to
/// `workspace_root` and emit a paired `(CSI, [?], Ps, h)` /
/// `(CSI, [?], Ps, l)` tuple per `NamedPrivateMode` variant.
///
/// `Ps` in the tuple `params` field is literally the decimal
/// string of the numeric value (`"1"`, `"25"`, `"2026"`, …) — NOT
/// the placeholder `Ps`. This lets `--check` match a catalog row
/// for mode 2026 against the `(CSI, [?], 2026, h)` tuple.
///
/// # Errors
///
/// Returns [`DispatchExtractError`] on io or parse failure.
pub fn extract_namedprivatemode_tuples(
    workspace_root: &Path,
) -> Result<Vec<Tuple>, DispatchExtractError> {
    private_mode::extract(workspace_root)
}

// -------- Shared AST / IO helpers used across walker submodules -----------

pub(super) fn read_rust(path: &Path) -> Result<String, DispatchExtractError> {
    fs::read_to_string(path).map_err(|source| DispatchExtractError::Io {
        path: path.to_path_buf(),
        source,
    })
}

pub(super) fn parse_rust(path: &Path, source: &str) -> Result<syn::File, DispatchExtractError> {
    syn::parse_file(source).map_err(|source| DispatchExtractError::Parse {
        path: path.to_path_buf(),
        source,
    })
}

pub(super) fn type_path_ends_with(ty: &syn::Type, want: &str) -> bool {
    let syn::Type::Path(tp) = ty else {
        return false;
    };
    tp.path.segments.last().is_some_and(|seg| seg.ident == want)
}

/// The scrutinee shape used by CSI dispatch AND DCS hook arms:
/// `(action, intermediates)`.
pub(super) fn is_action_intermediates_scrutinee(expr: &syn::Expr) -> bool {
    let syn::Expr::Tuple(t) = expr else {
        return false;
    };
    if t.elems.len() != 2 {
        return false;
    }
    let has_action = matches!(&t.elems[0], syn::Expr::Path(p) if p.path.is_ident("action"));
    let has_inter = matches!(&t.elems[1], syn::Expr::Path(p) if p.path.is_ident("intermediates"));
    has_action && has_inter
}

/// Extract a `char` literal from a pattern element. Used by CSI and
/// DCS arm → tuple conversion.
pub(super) fn pattern_char_literal(pat: &syn::Pat) -> Option<char> {
    match pat {
        syn::Pat::Lit(syn::PatLit {
            lit: syn::Lit::Char(c),
            ..
        }) => Some(c.value()),
        _ => None,
    }
}

/// Extract a slice of byte literals from a pattern element. Used
/// by CSI and DCS arm → tuple conversion.
pub(super) fn pattern_byte_slice(pat: &syn::Pat) -> Option<Vec<u8>> {
    let syn::Pat::Slice(slice) = pat else {
        return None;
    };
    let mut bytes = Vec::new();
    for elem in &slice.elems {
        let syn::Pat::Lit(syn::PatLit {
            lit: syn::Lit::Byte(b),
            ..
        }) = elem
        else {
            return None;
        };
        bytes.push(b.value());
    }
    Some(bytes)
}
