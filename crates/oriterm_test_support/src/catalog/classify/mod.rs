//! Tuple classification — match a capture-emitted tuple against
//! the dispatch table and return the handler method(s) it routes
//! to.
//!
//! This module powers the `catalog_coverage_check classify` and
//! `catalog_coverage_check capture-top10-covered` subcommands
//! (Section 01.5.c).
//!
//! The classifier walks the dispatch AST via the same `syn`
//! infrastructure the dispatch extractors use, but also captures
//! the handler method names from each match arm body. The result
//! is a `BTreeMap<Tuple, BTreeSet<String>>` mapping each
//! dispatched tuple to the set of `Handler::*` methods that arm
//! invokes.

mod walkers;

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use super::dispatch_extract::DispatchExtractError;
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
/// Walks `crates/vte/src/ansi/dispatch/{csi,osc,mod}.rs` relative
/// to `workspace_root`, extracting both the arm pattern (→ tuple)
/// and the arm body (→ handler method calls on `handler` or
/// `self.handler`). Also includes `NamedPrivateMode` entries as
/// `set_private_mode` / `unset_private_mode`.
///
/// # Errors
///
/// Returns [`DispatchExtractError`] on IO or `syn` parse failure.
pub fn build_dispatch_map(
    workspace_root: &Path,
) -> Result<BTreeMap<Tuple, BTreeSet<String>>, DispatchExtractError> {
    let mut map: BTreeMap<Tuple, BTreeSet<String>> = BTreeMap::new();

    let csi_path = workspace_root.join("crates/vte/src/ansi/dispatch/csi.rs");
    walkers::extract_csi_with_handlers(&csi_path, &mut map)?;

    let mod_path = workspace_root.join("crates/vte/src/ansi/dispatch/mod.rs");
    walkers::extract_mod_with_handlers(&mod_path, &mut map)?;

    let osc_path = workspace_root.join("crates/vte/src/ansi/dispatch/osc.rs");
    walkers::extract_osc_with_handlers(&osc_path, &mut map)?;

    let types_path = workspace_root.join("crates/vte/src/ansi/types.rs");
    walkers::extract_private_mode_handlers(&types_path, workspace_root, &mut map)?;

    // APC dispatch is unconditional — Performer::apc_end calls
    // handler.apc_dispatch(&payload) for every APC sequence.
    // This isn't a match arm, so the visitor doesn't catch it.
    map.entry(Tuple::new(Category::Apc, Vec::<u8>::new(), "Pt", "ST"))
        .or_default()
        .insert("apc_dispatch".to_string());

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
    }

    // OSC normalization: capture tuples have params like `"0;text"` but
    // dispatch tuples have just `"0"` (the numeric ID that the match
    // arm pattern selects on). Strip everything after the first `;`.
    if tuple.category == Category::Osc {
        let numeric_id = tuple.params.split(';').next().unwrap_or(&tuple.params);
        let normalized = Tuple::new(
            Category::Osc,
            tuple.intermediates.clone(),
            numeric_id,
            tuple.final_byte.clone(),
        );
        if let Some(handlers) = map.get(&normalized) {
            return Classification::Dispatched {
                handlers: handlers.iter().cloned().collect(),
            };
        }
    }

    // SGR normalization: a capture tuple `(CSI, [], Ps, m)` should
    // match any of the per-SGR-code tuples `(CSI, [], 0, m)`,
    // `(CSI, [], 1, m)`, etc. in the dispatch map.
    if tuple.category == Category::Csi && tuple.final_byte == "m" && tuple.intermediates.is_empty()
    {
        let probe = Tuple::new(Category::Csi, Vec::<u8>::new(), "0", "m");
        if let Some(handlers) = map.get(&probe) {
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
