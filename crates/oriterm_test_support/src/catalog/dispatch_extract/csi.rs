//! CSI dispatch-arm walker.
//!
//! Walks `crates/vte/src/ansi/dispatch/csi.rs` — specifically the
//! top-level `match (action, intermediates) { ... }` in the
//! `dispatch` function — and emits one [`Tuple`] per arm pattern.
//! The arm body (`handler.foo(...)`) is NOT consumed here;
//! per-handler classification lives in the sibling `classify`
//! module when it lands.

use std::collections::BTreeSet;

use super::super::tuple::{Category, Tuple};
use super::{is_action_intermediates_scrutinee, pattern_byte_slice, pattern_char_literal};

pub(super) fn extract_csi_arms(file: &syn::File, out: &mut BTreeSet<Tuple>) {
    for item in &file.items {
        let syn::Item::Fn(func) = item else { continue };
        if func.sig.ident != "dispatch" {
            continue;
        }
        scan_csi_dispatch_fn(&func.block, out);
    }
}

fn scan_csi_dispatch_fn(block: &syn::Block, out: &mut BTreeSet<Tuple>) {
    // Walk every expression looking for `match (action, intermediates)`.
    struct CsiVisitor<'a> {
        out: &'a mut BTreeSet<Tuple>,
    }

    impl syn::visit::Visit<'_> for CsiVisitor<'_> {
        fn visit_expr_match(&mut self, m: &syn::ExprMatch) {
            // Heuristic: the scrutinee for `csi::dispatch`'s main
            // match is a tuple `(action, intermediates)`. We match
            // on the shape of the expression.
            if is_action_intermediates_scrutinee(&m.expr) {
                for arm in &m.arms {
                    collect_csi_arm(&arm.pat, self.out);
                }
            } else {
                // Descend further — some nested matches exist.
                syn::visit::visit_expr_match(self, m);
            }
        }
    }

    let mut v = CsiVisitor { out };
    syn::visit::Visit::visit_block(&mut v, block);
}

/// Collect CSI tuples from an arm pattern, handling OR patterns
/// like `('H', []) | ('f', [])`.
fn collect_csi_arm(pat: &syn::Pat, out: &mut BTreeSet<Tuple>) {
    match pat {
        syn::Pat::Tuple(tup) if tup.elems.len() == 2 => {
            if let (Some(final_byte), Some(intermediates)) = (
                pattern_char_literal(&tup.elems[0]),
                pattern_byte_slice(&tup.elems[1]),
            ) {
                out.insert(Tuple::new(
                    Category::Csi,
                    intermediates,
                    "Ps",
                    final_byte.to_string(),
                ));
            }
        }
        syn::Pat::Or(or) => {
            for sub in &or.cases {
                collect_csi_arm(sub, out);
            }
        }
        _ => {}
    }
}
