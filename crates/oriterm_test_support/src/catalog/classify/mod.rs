//! Tuple classification — match a capture-emitted tuple against
//! the dispatch table and return the handler method(s) it routes
//! to.
//!
//! This module powers the `catalog_coverage_check classify` and
//! `catalog_coverage_check capture-top10-covered` subcommands
//! (Section 01.5.c).
//!
//! The classifier delegates to the dispatch extractors in the
//! sibling `dispatch_extract` submodule which walk the dispatch
//! AST and capture both the arm pattern (→ tuple) and the arm
//! body (→ handler method calls). The result is a
//! `BTreeMap<Tuple, BTreeSet<String>>` mapping each dispatched
//! tuple to the set of `Handler::*` methods that arm invokes.
//!
//! `NamedPrivateMode` entries are added as
//! `set_private_mode` / `unset_private_mode`.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use super::dispatch_extract::{self, DispatchExtractError};
use super::tuple::{Category, Tuple};

/// Result of classifying a single tuple.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Classification {
    /// Tuple matches at least one dispatch arm. `handlers` lists
    /// every `Handler::*` method that arm(s) call.
    Dispatched { handlers: Vec<String> },
    /// Tuple's category is recognized but no dispatch arm matches.
    NoDispatch,
}

/// Build the full dispatch map: `Tuple → {Handler method names}`.
///
/// Delegates to [`dispatch_extract::extract_dispatch_map`] for
/// CSI / OSC / ESC / DCS / APC arms, then adds `NamedPrivateMode`
/// entries as `set_private_mode` / `unset_private_mode`.
///
/// # Errors
///
/// Returns [`DispatchExtractError`] on IO or `syn` parse failure.
pub fn build_dispatch_map(
    workspace_root: &Path,
) -> Result<BTreeMap<Tuple, BTreeSet<String>>, DispatchExtractError> {
    let mut map = dispatch_extract::extract_dispatch_map(workspace_root)?;

    // NamedPrivateMode entries — each mode number gets a
    // set_private_mode/unset_private_mode handler pair.
    let pm_tuples = dispatch_extract::extract_namedprivatemode_tuples(workspace_root)?;
    for tuple in pm_tuples {
        let method = if tuple.final_byte == "h" {
            "set_private_mode"
        } else {
            "unset_private_mode"
        };
        map.entry(tuple).or_default().insert(method.to_string());
    }

    Ok(map)
}

/// Classify a tuple using a pre-built map (avoids rebuilding per
/// tuple when classifying a batch).
///
/// CSI dispatch arms match on `(action, intermediates)` only —
/// the number of parameters is a runtime value. So a capture
/// tuple `(CSI, [], Ps;Ps, H)` must match the dispatch tuple
/// `(CSI, [], Ps, H)`. We normalize the lookup accordingly:
/// for CSI tuples, collapse all `Ps`-containing params to `"Ps"`.
/// For `NamedPrivateMode` tuples `(CSI, [?], <n>, h|l)`, we try
/// both the literal numeric param and the generic `Ps` fallback.
#[must_use]
pub fn classify_from_map(map: &BTreeMap<Tuple, BTreeSet<String>>, tuple: &Tuple) -> Classification {
    // Direct lookup first (exact match).
    if let Some(handlers) = map.get(tuple) {
        return Classification::Dispatched {
            handlers: handlers.iter().cloned().collect(),
        };
    }

    // CSI normalization: collapse Ps-based params to single "Ps".
    if tuple.category == Category::Csi {
        let normalized = Tuple::new(
            Category::Csi,
            tuple.intermediates.clone(),
            "Ps",
            tuple.final_byte.clone(),
        );
        if let Some(handlers) = map.get(&normalized) {
            return Classification::Dispatched {
                handlers: handlers.iter().cloned().collect(),
            };
        }
        // Also try "-" (zero params).
        let no_params = Tuple::new(
            Category::Csi,
            tuple.intermediates.clone(),
            "-",
            tuple.final_byte.clone(),
        );
        if let Some(handlers) = map.get(&no_params) {
            return Classification::Dispatched {
                handlers: handlers.iter().cloned().collect(),
            };
        }
        // Catch-all intermediates fallback: dispatch arms like
        // `('c', intermediates)` are extracted with the WILDCARD_INTERMEDIATES
        // sentinel `[0xFF]`. Try the sentinel form for ANY intermediates
        // (empty or non-empty) — a catch-all arm matches both. The sentinel
        // approach prevents false positives because only catch-all arms
        // produce `[0xFF]` entries; explicit `[]` arms keep `[]`.
        let catch_all = Tuple::new(
            Category::Csi,
            dispatch_extract::WILDCARD_INTERMEDIATES.to_vec(),
            "Ps",
            tuple.final_byte.clone(),
        );
        if let Some(handlers) = map.get(&catch_all) {
            return Classification::Dispatched {
                handlers: handlers.iter().cloned().collect(),
            };
        }
    }

    // OSC normalization: after the SSOT alignment, capture
    // tuples carry the payload in `params` while dispatch tuples have
    // empty `params` (the dispatch arm only knows the selector). The
    // lookup matches on `final_byte` (the selector) — drop `params` to
    // bridge the two shapes.
    if tuple.category == Category::Osc {
        let normalized = Tuple::new(
            Category::Osc,
            tuple.intermediates.clone(),
            "",
            tuple.final_byte.clone(),
        );
        if let Some(handlers) = map.get(&normalized) {
            return Classification::Dispatched {
                handlers: handlers.iter().cloned().collect(),
            };
        }
    }

    // APC normalization: the catalog uses `(APC, [_G], key-value, ST)`
    // for kitty graphics, but APC dispatch is unconditional — the
    // handler gets the raw payload and discriminates inside. The
    // dispatch map has the generic `(APC, [], Pt, ST)`. Normalize
    // any APC tuple to the generic form.
    if tuple.category == Category::Apc {
        let generic = Tuple::new(Category::Apc, Vec::<u8>::new(), "Pt", "ST");
        if let Some(handlers) = map.get(&generic) {
            return Classification::Dispatched {
                handlers: handlers.iter().cloned().collect(),
            };
        }
    }

    Classification::NoDispatch
}
