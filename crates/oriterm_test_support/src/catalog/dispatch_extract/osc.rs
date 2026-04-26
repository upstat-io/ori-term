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
use super::{DispatchExtractError, extract_handler_methods, parse_rust, read_rust};

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
    struct OscVisitor<'a> {
        map: &'a mut BTreeMap<Tuple, BTreeSet<String>>,
    }

    impl syn::visit::Visit<'_> for OscVisitor<'_> {
        fn visit_expr_match(&mut self, m: &syn::ExprMatch) {
            if is_params_zero_scrutinee(&m.expr) {
                for arm in &m.arms {
                    collect_osc_arm_with_handlers(&arm.pat, &arm.body, self.map);
                }
            } else {
                syn::visit::visit_expr_match(self, m);
            }
        }
    }

    let mut v = OscVisitor { map };
    syn::visit::Visit::visit_block(&mut v, block);
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
            if let Ok(id) = std::str::from_utf8(&value) {
                if !id.is_empty() {
                    // SSOT (BUG-07-019): OSC selector in `final_byte`.
                    // The dispatch arm only knows the selector — payload
                    // shape lives in catalog/capture, so `params` is empty.
                    let tuple = Tuple::new(Category::Osc, Vec::<u8>::new(), "", id);
                    let methods = extract_handler_methods(body);
                    map.entry(tuple).or_default().extend(methods);
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
