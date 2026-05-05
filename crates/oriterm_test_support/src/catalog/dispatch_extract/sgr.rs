//! SGR numeric-parameter walker.
//!
//! Walks `attrs_from_sgr_parameters` in
//! `crates/vte/src/ansi/dispatch/csi/sgr.rs` and emits one
//! `(CSI, [], <n>, m)` tuple per distinct SGR parameter code. The
//! match is `match param { [0] =>..., [38] =>..., [38, params @..] =>...,... }`
//! — we take the FIRST numeric literal of each slice pattern as
//! the SGR code.

use std::collections::BTreeSet;

use super::super::tuple::{Category, Tuple};
use super::walk_match_exprs;

pub(super) fn extract_sgr_params(file: &syn::File, out: &mut BTreeSet<Tuple>) {
    // Find the `attrs_from_sgr_parameters` function and walk its
    // `match param { [0] =>...,... }` to extract every supported
    // numeric SGR parameter.
    for item in &file.items {
        let syn::Item::Fn(func) = item else { continue };
        if func.sig.ident != "attrs_from_sgr_parameters" {
            continue;
        }
        collect_sgr_numeric_from_block(&func.block, out);
    }
}

fn collect_sgr_numeric_from_block(block: &syn::Block, out: &mut BTreeSet<Tuple>) {
    // The SGR `match param {... }` is the single top-level match
    // we care about — return false from the closure so the walker
    // does not descend into nested matches.
    walk_match_exprs(block, |m| {
        for arm in &m.arms {
            collect_sgr_arm(&arm.pat, out);
        }
        false
    });
}

fn collect_sgr_arm(pat: &syn::Pat, out: &mut BTreeSet<Tuple>) {
    // Arm is a slice pattern: `[0]`, `[4, 0]`, `[38]`, `[38, params @..]`.
    // We extract the FIRST numeric literal as the SGR parameter code,
    // since `[38,...]` variants all belong to SGR 38.
    let slice = match pat {
        syn::Pat::Slice(s) => s,
        syn::Pat::Or(or) => {
            for sub in &or.cases {
                collect_sgr_arm(sub, out);
            }
            return;
        }
        _ => return,
    };

    let first = slice.elems.first();
    let Some(syn::Pat::Lit(syn::PatLit {
        lit: syn::Lit::Int(int),
        ..
    })) = first
    else {
        return;
    };
    let Ok(n) = int.base10_parse::<u16>() else {
        return;
    };
    // One tuple per SGR code. `params` carries the numeric code as a
    // literal string so the canonical catalog row `CSI <n> m` matches.
    out.insert(Tuple::new(
        Category::Csi,
        Vec::<u8>::new(),
        n.to_string(),
        "m",
    ));
}
