//! `NamedPrivateMode` → `(CSI, [?], <n>, h|l)` extractor.
//!
//! Walks `crates/vte/src/ansi/types.rs::PrivateMode::new` and
//! emits a paired set/reset tuple per recognized numeric variant.
//! The extractor scans `match self { 1 => NamedPrivateMode::...,
//! 25 => ..., ... }` arms and collects every literal `u16`. Each
//! number becomes two tuples: `(CSI, [?], "<n>", h)` and
//! `(CSI, [?], "<n>", l)`.
//!
//! Note: `params` carries the literal decimal string (`"1"`,
//! `"25"`, `"2026"`, …), NOT the placeholder `Ps`. This lets
//! `--check` match a catalog row whose `Sequence` column is
//! written as `CSI ? 2026 h` against the actual dispatch tuple.

use std::collections::BTreeSet;
use std::path::Path;

use super::super::tuple::{Category, Tuple};
use super::{DispatchExtractError, parse_rust, read_rust, type_path_ends_with};

pub(super) fn extract(workspace_root: &Path) -> Result<Vec<Tuple>, DispatchExtractError> {
    let types_path = workspace_root.join("crates/vte/src/ansi/types.rs");
    let source = read_rust(&types_path)?;
    let file = parse_rust(&types_path, &source)?;

    let mut numbers: BTreeSet<u16> = BTreeSet::new();
    for item in &file.items {
        let syn::Item::Impl(imp) = item else { continue };
        // Look for `impl PrivateMode` and walk its `new` function.
        if !type_path_ends_with(&imp.self_ty, "PrivateMode") {
            continue;
        }
        for impl_item in &imp.items {
            let syn::ImplItem::Fn(method) = impl_item else {
                continue;
            };
            if method.sig.ident != "new" {
                continue;
            }
            scan_match_numeric_arms(&method.block, &mut numbers);
        }
    }

    let mut tuples = Vec::with_capacity(numbers.len() * 2);
    for n in numbers {
        let ps = n.to_string();
        tuples.push(Tuple::new(Category::Csi, [b'?'], ps.clone(), "h"));
        tuples.push(Tuple::new(Category::Csi, [b'?'], ps, "l"));
    }
    Ok(tuples)
}

fn scan_match_numeric_arms(block: &syn::Block, out: &mut BTreeSet<u16>) {
    for stmt in &block.stmts {
        let syn::Stmt::Expr(expr, _) = stmt else {
            continue;
        };
        let syn::Expr::Match(m) = expr else { continue };
        for arm in &m.arms {
            // Patterns are either `N => ...` or `N | M => ...` or `_`.
            collect_numeric_pats(&arm.pat, out);
        }
    }
}

fn collect_numeric_pats(pat: &syn::Pat, out: &mut BTreeSet<u16>) {
    match pat {
        syn::Pat::Lit(lit) => {
            if let syn::Lit::Int(int) = &lit.lit {
                if let Ok(n) = int.base10_parse::<u16>() {
                    out.insert(n);
                }
            }
        }
        syn::Pat::Or(or) => {
            for sub in &or.cases {
                collect_numeric_pats(sub, out);
            }
        }
        _ => {}
    }
}
