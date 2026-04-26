//! OSC dispatch-arm walker.
//!
//! Walks `crates/vte/src/ansi/dispatch/osc.rs` — specifically the
//! `match params[0] { ... }` in the `dispatch` function — and
//! emits one [`Tuple`] per recognized numeric-id byte string
//! literal.
//!
//! Entry point:
//! - [`extract_osc_arms_with_handlers`] — tuples + handler methods

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use super::super::tuple::{Category, Tuple};
use super::{
    DispatchExtractError, extract_handler_methods, parse_rust, read_rust, walk_match_exprs,
};

pub(super) fn extract_osc_arms_with_handlers(
    path: &Path,
    map: &mut BTreeMap<Tuple, BTreeSet<String>>,
) -> Result<(), DispatchExtractError> {
    let source = read_rust(path)?;
    let file = parse_rust(path, &source)?;

    for item in &file.items {
        let syn::Item::Fn(func) = item else { continue };
        if func.sig.ident != "dispatch" {
            continue;
        }
        scan_osc_dispatch_fn(&func.block, map);
    }
    Ok(())
}

fn scan_osc_dispatch_fn(block: &syn::Block, map: &mut BTreeMap<Tuple, BTreeSet<String>>) {
    walk_match_exprs(block, |m| {
        if !is_params_zero_scrutinee(&m.expr) {
            return true; // not the OSC dispatch match — recurse into nested ones
        }
        for arm in &m.arms {
            collect_osc_arm_with_handlers(&arm.pat, &arm.body, map);
        }
        false // handled — don't descend
    });
}

fn is_params_zero_scrutinee(expr: &syn::Expr) -> bool {
    let syn::Expr::Index(idx) = expr else {
        return false;
    };
    matches!(&*idx.expr, syn::Expr::Path(p) if p.path.is_ident("params"))
}

fn collect_osc_arm_with_handlers(
    pat: &syn::Pat,
    body: &syn::Expr,
    map: &mut BTreeMap<Tuple, BTreeSet<String>>,
) {
    match pat {
        syn::Pat::Lit(syn::PatLit {
            lit: syn::Lit::ByteStr(b),
            ..
        }) => {
            let value = b.value();
            match std::str::from_utf8(&value) {
                Ok(id) if !id.is_empty() => {
                    // SSOT: OSC selector in `final_byte`.
                    // The dispatch arm only knows the selector — payload
                    // shape lives in catalog/capture, so `params` is empty.
                    let tuple = Tuple::new(Category::Osc, Vec::<u8>::new(), "", id);
                    let methods = extract_handler_methods(body);
                    map.entry(tuple).or_default().extend(methods);
                }
                Ok(_) => {
                    // Empty selector — skip silently; an empty arm pattern
                    // can never dispatch.
                }
                Err(e) => {
                    // Non-UTF-8 selector in a ByteStr arm: the dispatch
                    // arm exists in the AST but the catalog cannot map
                    // it back. Surface to stderr so coverage gaps are
                    // visible, not silently dropped.
                    eprintln!(
                        "warning: dispatch_extract::osc: skipped malformed arm selector \
                         (bytes={:02x?}): {e}",
                        value
                    );
                }
            }
        }
        syn::Pat::Or(or) => {
            for sub in &or.cases {
                collect_osc_arm_with_handlers(sub, body, map);
            }
        }
        _ => {}
    }
}
