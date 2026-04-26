//! CSI dispatch-arm walker.
//!
//! Walks `crates/vte/src/ansi/dispatch/csi/mod.rs` — specifically the
//! top-level `match (action, intermediates) { ... }` in the
//! `dispatch` function — and emits one [`Tuple`] per arm pattern.
//!
//! Entry point:
//! - [`extract_csi_arms_with_handlers`] — tuples + handler methods

use std::collections::{BTreeMap, BTreeSet};

use super::super::tuple::{Category, Tuple};
use super::{
    extract_handler_methods, is_action_intermediates_scrutinee, pattern_byte_slice,
    pattern_char_literal, walk_match_exprs,
};

pub(super) fn extract_csi_arms_with_handlers(
    file: &syn::File,
    map: &mut BTreeMap<Tuple, BTreeSet<String>>,
) {
    for item in &file.items {
        let syn::Item::Fn(func) = item else { continue };
        if func.sig.ident != "dispatch" {
            continue;
        }
        scan_csi_dispatch_fn(&func.block, map);
    }
}

fn scan_csi_dispatch_fn(block: &syn::Block, map: &mut BTreeMap<Tuple, BTreeSet<String>>) {
    walk_match_exprs(block, |m| {
        if !is_action_intermediates_scrutinee(&m.expr) {
            return true; // not the CSI dispatch match — recurse into nested ones
        }
        for arm in &m.arms {
            let tuples = collect_csi_tuples_from_pat(&arm.pat);
            if tuples.is_empty() {
                continue;
            }
            let methods = extract_handler_methods(&arm.body);
            for tuple in tuples {
                map.entry(tuple).or_default().extend(methods.clone());
            }
        }
        false // handled — don't descend
    });
}

/// Collect CSI tuples from an arm pattern, handling OR patterns
/// like `('H', []) | ('f', [])`.
fn collect_csi_tuples_from_pat(pat: &syn::Pat) -> Vec<Tuple> {
    let mut out = Vec::new();
    collect_csi_recursive(pat, &mut out);
    out
}

fn collect_csi_recursive(pat: &syn::Pat, out: &mut Vec<Tuple>) {
    match pat {
        syn::Pat::Tuple(tup) if tup.elems.len() == 2 => {
            let Some(final_byte) = pattern_char_literal(&tup.elems[0]) else {
                return;
            };
            // Resolve intermediates: explicit byte-slice or catch-all ident.
            let intermediates = match pattern_byte_slice(&tup.elems[1]) {
                Some(bytes) => bytes,
                None if matches!(&tup.elems[1], syn::Pat::Ident(_)) => {
                    // Catch-all identifier pattern: `('c', intermediates)`.
                    // Use WILDCARD_INTERMEDIATES sentinel so classify can
                    // distinguish this from explicit-empty `[]` patterns.
                    super::WILDCARD_INTERMEDIATES.to_vec()
                }
                None => return,
            };
            out.push(Tuple::new(
                Category::Csi,
                intermediates,
                "Ps",
                final_byte.to_string(),
            ));
        }
        syn::Pat::Or(or) => {
            for sub in &or.cases {
                collect_csi_recursive(sub, out);
            }
        }
        _ => {}
    }
}
