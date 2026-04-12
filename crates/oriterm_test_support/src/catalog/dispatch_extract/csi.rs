//! CSI dispatch-arm walker.
//!
//! Walks `crates/vte/src/ansi/dispatch/csi.rs` — specifically the
//! top-level `match (action, intermediates) { ... }` in the
//! `dispatch` function — and emits one [`Tuple`] per arm pattern.
//!
//! Two entry points:
//! - [`extract_csi_arms`] — tuples only (no handler method names)
//! - [`extract_csi_arms_with_handlers`] — tuples + handler methods

use std::collections::{BTreeMap, BTreeSet};

use super::super::tuple::{Category, Tuple};
use super::{
    extract_handler_methods, is_action_intermediates_scrutinee, pattern_byte_slice,
    pattern_char_literal,
};

pub(super) fn extract_csi_arms(file: &syn::File, out: &mut BTreeSet<Tuple>) {
    let mut map: BTreeMap<Tuple, BTreeSet<String>> = BTreeMap::new();
    extract_csi_arms_with_handlers(file, &mut map);
    out.extend(map.into_keys());
}

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
    struct CsiVisitor<'a> {
        map: &'a mut BTreeMap<Tuple, BTreeSet<String>>,
    }

    impl syn::visit::Visit<'_> for CsiVisitor<'_> {
        fn visit_expr_match(&mut self, m: &syn::ExprMatch) {
            if is_action_intermediates_scrutinee(&m.expr) {
                for arm in &m.arms {
                    let tuples = collect_csi_tuples_from_pat(&arm.pat);
                    if !tuples.is_empty() {
                        let methods = extract_handler_methods(&arm.body);
                        for tuple in tuples {
                            self.map.entry(tuple).or_default().extend(methods.clone());
                        }
                    }
                }
            } else {
                syn::visit::visit_expr_match(self, m);
            }
        }
    }

    let mut v = CsiVisitor { map };
    syn::visit::Visit::visit_block(&mut v, block);
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
            if let (Some(final_byte), Some(intermediates)) = (
                pattern_char_literal(&tup.elems[0]),
                pattern_byte_slice(&tup.elems[1]),
            ) {
                out.push(Tuple::new(
                    Category::Csi,
                    intermediates,
                    "Ps",
                    final_byte.to_string(),
                ));
            }
        }
        syn::Pat::Or(or) => {
            for sub in &or.cases {
                collect_csi_recursive(sub, out);
            }
        }
        _ => {}
    }
}
