//! CSI dispatch-arm walker.
//!
//! Walks `crates/vte/src/ansi/dispatch/csi.rs` — specifically the
//! top-level `match (action, intermediates) { ... }` in the
//! `dispatch` function — and emits one [`Tuple`] per arm pattern.
//! The arm body (`handler.foo(...)`) is NOT consumed here;
//! per-handler classification lives in the sibling `classify`
//! module when it lands.

use std::collections::BTreeSet;
use std::path::Path;

use super::super::tuple::{Category, Tuple};
use super::{
    DispatchExtractError, is_action_intermediates_scrutinee, parse_rust, pattern_byte_slice,
    pattern_char_literal, read_rust,
};

pub(super) fn extract_csi_arms(
    path: &Path,
    out: &mut BTreeSet<Tuple>,
) -> Result<(), DispatchExtractError> {
    let source = read_rust(path)?;
    let file = parse_rust(path, &source)?;

    for item in &file.items {
        let syn::Item::Fn(func) = item else { continue };
        if func.sig.ident != "dispatch" {
            continue;
        }
        scan_csi_dispatch_fn(&func.block, out);
    }
    Ok(())
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
                    if let Some(tuple) = arm_to_csi_tuple(&arm.pat) {
                        self.out.insert(tuple);
                    }
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

fn arm_to_csi_tuple(pat: &syn::Pat) -> Option<Tuple> {
    // Arm patterns look like `('H', [])`, `('h', [b'?'])`, etc.
    let syn::Pat::Tuple(tup) = pat else {
        return None;
    };
    if tup.elems.len() != 2 {
        return None;
    }

    // First element — character literal for the final byte.
    let final_byte = pattern_char_literal(&tup.elems[0])?;
    // Second element — slice / array pattern of byte literals.
    let intermediates = pattern_byte_slice(&tup.elems[1])?;

    Some(Tuple::new(
        Category::Csi,
        intermediates,
        "Ps",
        final_byte.to_string(),
    ))
}
