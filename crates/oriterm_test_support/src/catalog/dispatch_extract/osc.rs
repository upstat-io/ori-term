//! OSC dispatch-arm walker.
//!
//! Walks `crates/vte/src/ansi/dispatch/osc.rs` — specifically the
//! `match params[0] { ... }` in the `dispatch` function — and
//! emits one [`Tuple`] per recognized numeric-id byte string
//! literal.

use std::collections::BTreeSet;
use std::path::Path;

use super::super::tuple::{Category, Tuple};
use super::{DispatchExtractError, parse_rust, read_rust};

pub(super) fn extract_osc_arms(
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
        scan_osc_dispatch_fn(&func.block, out);
    }
    Ok(())
}

fn scan_osc_dispatch_fn(block: &syn::Block, out: &mut BTreeSet<Tuple>) {
    // OSC dispatch matches `params[0]` against byte-string literals.
    struct OscVisitor<'a> {
        out: &'a mut BTreeSet<Tuple>,
    }

    impl syn::visit::Visit<'_> for OscVisitor<'_> {
        fn visit_expr_match(&mut self, m: &syn::ExprMatch) {
            if is_params_zero_scrutinee(&m.expr) {
                for arm in &m.arms {
                    collect_osc_arm(&arm.pat, self.out);
                }
            } else {
                syn::visit::visit_expr_match(self, m);
            }
        }
    }

    let mut v = OscVisitor { out };
    syn::visit::Visit::visit_block(&mut v, block);
}

fn is_params_zero_scrutinee(expr: &syn::Expr) -> bool {
    // Matches `params[0]` indexing.
    let syn::Expr::Index(idx) = expr else {
        return false;
    };
    matches!(&*idx.expr, syn::Expr::Path(p) if p.path.is_ident("params"))
}

fn collect_osc_arm(pat: &syn::Pat, out: &mut BTreeSet<Tuple>) {
    match pat {
        syn::Pat::Lit(syn::PatLit {
            lit: syn::Lit::ByteStr(b),
            ..
        }) => {
            let value = b.value();
            if let Ok(id) = std::str::from_utf8(&value) {
                if !id.is_empty() {
                    out.insert(Tuple::new(
                        Category::Osc,
                        Vec::<u8>::new(),
                        id.to_string(),
                        "BEL",
                    ));
                }
            }
        }
        syn::Pat::Or(or) => {
            for sub in &or.cases {
                collect_osc_arm(sub, out);
            }
        }
        _ => {}
    }
}
