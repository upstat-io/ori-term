//! ESC + DCS walker, driven off `crates/vte/src/ansi/dispatch/mod.rs`.
//!
//! The `Performer` impl in `dispatch/mod.rs` owns `esc_dispatch`
//! (the ESC byte table — IND, RI, DECALN, charset designation,
//! keypad application mode, single shifts) AND the `hook` entry
//! that starts a DCS sequence. Both paths have (`final_byte`,
//! `intermediates`) scrutinees the extractor understands:
//!
//! - `esc_dispatch`: `match (byte, intermediates) { (b'D', []) => ... }`
//! - DCS hook:       `match (action, intermediates) { ('q', []) => ... }`
//!
//! ESC arms emit `(ESC, intermediates, -, final)` tuples; charset-
//! designation arms with a catch-all `intermediates` ident bind
//! emit four DA tuples, one per supported intermediate. DCS hook
//! arms emit `(DCS, intermediates, Pid|Pt, final)` tuples.
//!
//! Entry point:
//! - [`extract_mod_arms_with_handlers`] — tuples + handler methods

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use super::super::tuple::{Category, Tuple};
use super::{DispatchExtractError, extract_handler_methods, parse_rust, read_rust};

pub(super) fn extract_mod_arms_with_handlers(
    path: &Path,
    map: &mut BTreeMap<Tuple, BTreeSet<String>>,
) -> Result<(), DispatchExtractError> {
    let source = read_rust(path)?;
    let file = parse_rust(path, &source)?;

    let mut v = ModVisitorWithHandlers {
        map,
        in_hook: false,
    };
    syn::visit::Visit::visit_file(&mut v, &file);
    Ok(())
}

struct ModVisitorWithHandlers<'a> {
    map: &'a mut BTreeMap<Tuple, BTreeSet<String>>,
    in_hook: bool,
}

impl syn::visit::Visit<'_> for ModVisitorWithHandlers<'_> {
    fn visit_impl_item_fn(&mut self, method: &syn::ImplItemFn) {
        let name = method.sig.ident.to_string();
        let was_in_hook = self.in_hook;
        self.in_hook = name == "hook";
        syn::visit::visit_impl_item_fn(self, method);
        self.in_hook = was_in_hook;
    }

    fn visit_expr_match(&mut self, m: &syn::ExprMatch) {
        if is_byte_intermediates_scrutinee(&m.expr) {
            for arm in &m.arms {
                if let Some(tuple) = esc_arm_to_tuple(&arm.pat) {
                    let methods = extract_handler_methods(&arm.body);
                    self.map.entry(tuple).or_default().extend(methods);
                } else {
                    // Check for charset designation catch-all arms
                    // that produce DA tuples.
                    collect_esc_da_arms(&arm.pat, &arm.body, self.map);
                }
            }
        } else if self.in_hook && is_action_scrutinee(&m.expr) {
            for arm in &m.arms {
                if let Some(tuple) = dcs_hook_arm_to_tuple(&arm.pat, arm.guard.as_ref()) {
                    let methods = extract_handler_methods(&arm.body);
                    self.map.entry(tuple).or_default().extend(methods);
                }
            }
        } else {
            syn::visit::visit_expr_match(self, m);
        }
    }
}

fn is_byte_intermediates_scrutinee(expr: &syn::Expr) -> bool {
    let syn::Expr::Tuple(t) = expr else {
        return false;
    };
    if t.elems.len() != 2 {
        return false;
    }
    let has_byte = matches!(&t.elems[0], syn::Expr::Path(p) if p.path.is_ident("byte"));
    let has_inter = matches!(&t.elems[1], syn::Expr::Path(p) if p.path.is_ident("intermediates"));
    has_byte && has_inter
}

fn is_action_scrutinee(expr: &syn::Expr) -> bool {
    matches!(expr, syn::Expr::Path(p) if p.path.is_ident("action"))
}

/// Extract an ESC tuple from a match arm pattern with a byte-slice
/// intermediates element.
fn esc_arm_to_tuple(pat: &syn::Pat) -> Option<Tuple> {
    let syn::Pat::Tuple(tup) = pat else {
        return None;
    };
    if tup.elems.len() != 2 {
        return None;
    }
    let final_byte = match &tup.elems[0] {
        syn::Pat::Lit(syn::PatLit {
            lit: syn::Lit::Byte(b),
            ..
        }) => b.value(),
        _ => return None,
    };
    match &tup.elems[1] {
        syn::Pat::Slice(slice) => {
            let mut bytes = Vec::new();
            for elem in &slice.elems {
                if let syn::Pat::Lit(syn::PatLit {
                    lit: syn::Lit::Byte(b),
                    ..
                }) = elem
                {
                    bytes.push(b.value());
                }
            }
            Some(Tuple::new(
                Category::Esc,
                bytes,
                "-",
                (final_byte as char).to_string(),
            ))
        }
        _ => None,
    }
}

/// Handle the charset designation catch-all arms that produce DA
/// tuples (one per supported intermediate: `(`, `)`, `*`, `+`).
fn collect_esc_da_arms(
    pat: &syn::Pat,
    body: &syn::Expr,
    map: &mut BTreeMap<Tuple, BTreeSet<String>>,
) {
    let syn::Pat::Tuple(tup) = pat else { return };
    if tup.elems.len() != 2 {
        return;
    }
    let final_byte = match &tup.elems[0] {
        syn::Pat::Lit(syn::PatLit {
            lit: syn::Lit::Byte(b),
            ..
        }) => b.value(),
        _ => return,
    };
    // Second element is a bare `intermediates` identifier (catch-all).
    let syn::Pat::Ident(_) = &tup.elems[1] else {
        return;
    };
    let methods = extract_handler_methods(body);
    for intermediate in [b'(', b')', b'*', b'+'] {
        let tuple = Tuple::new(
            Category::Da,
            [intermediate],
            "-",
            (final_byte as char).to_string(),
        );
        map.entry(tuple).or_default().extend(methods.clone());
    }
}

fn dcs_hook_arm_to_tuple(
    pat: &syn::Pat,
    guard: Option<&(syn::token::If, Box<syn::Expr>)>,
) -> Option<Tuple> {
    let final_byte = match pat {
        syn::Pat::Lit(syn::PatLit {
            lit: syn::Lit::Char(c),
            ..
        }) => c.value(),
        _ => return None,
    };
    let intermediates = if let Some((_if_token, guard_expr)) = guard {
        infer_intermediates_from_guard(guard_expr)
    } else {
        Vec::new()
    };
    let params = if final_byte == 'q' && intermediates.is_empty() {
        "Pid"
    } else {
        "Pt"
    };
    Some(Tuple::new(
        Category::Dcs,
        intermediates,
        params,
        final_byte.to_string(),
    ))
}

fn infer_intermediates_from_guard(expr: &syn::Expr) -> Vec<u8> {
    if let syn::Expr::MethodCall(mc) = expr {
        if mc.method == "is_empty" {
            return Vec::new();
        }
    }
    if let syn::Expr::Binary(syn::ExprBinary {
        op: syn::BinOp::Eq(_),
        right,
        ..
    }) = expr
    {
        if let syn::Expr::Reference(r) = &**right {
            if let syn::Expr::Array(arr) = &*r.expr {
                let mut bytes = Vec::new();
                for elem in &arr.elems {
                    if let syn::Expr::Lit(syn::ExprLit {
                        lit: syn::Lit::Byte(b),
                        ..
                    }) = elem
                    {
                        bytes.push(b.value());
                    }
                }
                return bytes;
            }
        }
    }
    Vec::new()
}
