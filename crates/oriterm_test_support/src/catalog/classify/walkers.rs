//! AST walkers that extract (Tuple, handler-method) pairs from
//! `crates/vte/src/ansi/dispatch/{csi,osc,mod}.rs`.
//!
//! Each walker mirrors the structure of the corresponding
//! `dispatch_extract` submodule but additionally captures the
//! `Handler::*` method names from the match arm body.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use super::super::dispatch_extract::{
    DispatchExtractError, is_action_intermediates_scrutinee, parse_rust, pattern_byte_slice,
    pattern_char_literal, read_rust,
};
use super::super::tuple::{Category, Tuple};

// -------- CSI dispatch ---------------------------------------------------

pub(super) fn extract_csi_with_handlers(
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
        walk_csi_dispatch(&func.block, map);
    }
    Ok(())
}

fn walk_csi_dispatch(block: &syn::Block, map: &mut BTreeMap<Tuple, BTreeSet<String>>) {
    struct Visitor<'a> {
        map: &'a mut BTreeMap<Tuple, BTreeSet<String>>,
    }

    impl syn::visit::Visit<'_> for Visitor<'_> {
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

    let mut v = Visitor { map };
    syn::visit::Visit::visit_block(&mut v, block);
}

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

// -------- mod.rs dispatch (ESC + DCS hook) --------------------------------

pub(super) fn extract_mod_with_handlers(
    path: &Path,
    map: &mut BTreeMap<Tuple, BTreeSet<String>>,
) -> Result<(), DispatchExtractError> {
    struct Visitor<'a> {
        map: &'a mut BTreeMap<Tuple, BTreeSet<String>>,
        in_hook: bool,
    }

    impl syn::visit::Visit<'_> for Visitor<'_> {
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

    let source = read_rust(path)?;
    let file = parse_rust(path, &source)?;
    let mut v = Visitor {
        map,
        in_hook: false,
    };
    syn::visit::Visit::visit_file(&mut v, &file);
    Ok(())
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
        syn::Pat::Ident(_) | _ => None,
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

// -------- OSC dispatch ---------------------------------------------------

pub(super) fn extract_osc_with_handlers(
    path: &Path,
    map: &mut BTreeMap<Tuple, BTreeSet<String>>,
) -> Result<(), DispatchExtractError> {
    struct Visitor<'a> {
        map: &'a mut BTreeMap<Tuple, BTreeSet<String>>,
    }

    impl syn::visit::Visit<'_> for Visitor<'_> {
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

    let source = read_rust(path)?;
    let file = parse_rust(path, &source)?;
    let mut v = Visitor { map };
    syn::visit::Visit::visit_file(&mut v, &file);
    Ok(())
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
                    let tuple = Tuple::new(Category::Osc, Vec::<u8>::new(), id.to_string(), "BEL");
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

// -------- NamedPrivateMode -----------------------------------------------

pub(super) fn extract_private_mode_handlers(
    _types_path: &Path,
    workspace_root: &Path,
    map: &mut BTreeMap<Tuple, BTreeSet<String>>,
) -> Result<(), DispatchExtractError> {
    let tuples = super::super::dispatch_extract::extract_namedprivatemode_tuples(workspace_root)?;
    for tuple in tuples {
        let method = if tuple.final_byte == "h" {
            "set_private_mode"
        } else {
            "unset_private_mode"
        };
        map.entry(tuple).or_default().insert(method.to_string());
    }
    Ok(())
}

// -------- Handler method extraction from arm body AST --------------------

fn extract_handler_methods(expr: &syn::Expr) -> BTreeSet<String> {
    struct MethodVisitor<'a> {
        methods: &'a mut BTreeSet<String>,
    }

    impl syn::visit::Visit<'_> for MethodVisitor<'_> {
        fn visit_expr_method_call(&mut self, mc: &syn::ExprMethodCall) {
            if is_handler_receiver(&mc.receiver) {
                self.methods.insert(mc.method.to_string());
            }
            syn::visit::visit_expr_method_call(self, mc);
        }
    }

    let mut methods = BTreeSet::new();
    let mut v = MethodVisitor {
        methods: &mut methods,
    };
    syn::visit::Visit::visit_expr(&mut v, expr);
    methods
}

/// Check if a receiver is `handler` (direct ident) or
/// `self.handler` (field access on self).
fn is_handler_receiver(expr: &syn::Expr) -> bool {
    match expr {
        syn::Expr::Path(p) => p.path.is_ident("handler"),
        syn::Expr::Field(f) => {
            matches!(&*f.base, syn::Expr::Path(p) if p.path.is_ident("self"))
                && matches!(&f.member, syn::Member::Named(ident) if ident == "handler")
        }
        syn::Expr::Reference(r) => is_handler_receiver(&r.expr),
        syn::Expr::Unary(syn::ExprUnary {
            op: syn::UnOp::Deref(_),
            expr,
            ..
        }) => is_handler_receiver(expr),
        _ => false,
    }
}
