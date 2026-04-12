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

use std::collections::BTreeSet;
use std::path::Path;

use super::super::tuple::{Category, Tuple};
use super::{
    DispatchExtractError, is_action_intermediates_scrutinee, parse_rust, pattern_byte_slice,
    pattern_char_literal, read_rust,
};

pub(super) fn extract_mod_arms(
    path: &Path,
    out: &mut BTreeSet<Tuple>,
) -> Result<(), DispatchExtractError> {
    let source = read_rust(path)?;
    let file = parse_rust(path, &source)?;

    // We walk this file looking for ESC-byte match patterns and the
    // DCS `hook` function. `Performer::execute` dispatches C0 bytes;
    // the catalog treats those as per-byte rows covered under
    // `ecma-48.md` directly, so we do not emit tuples for execute.
    let mut v = ModVisitor { out };
    syn::visit::Visit::visit_file(&mut v, &file);
    Ok(())
}

struct ModVisitor<'a> {
    out: &'a mut BTreeSet<Tuple>,
}

impl syn::visit::Visit<'_> for ModVisitor<'_> {
    fn visit_expr_match(&mut self, m: &syn::ExprMatch) {
        if is_byte_intermediates_scrutinee(&m.expr) {
            // `match (byte, intermediates) { (b'B', intermediates) => ... }`
            // — esc_dispatch arms.
            for arm in &m.arms {
                collect_esc_arm(&arm.pat, self.out);
            }
        } else if is_action_intermediates_scrutinee(&m.expr) {
            // DCS hook's `match action { ... }` uses (action, []/[$]).
            for arm in &m.arms {
                if let Some(tuple) = arm_to_dcs_tuple(&arm.pat) {
                    self.out.insert(tuple);
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

fn collect_esc_arm(pat: &syn::Pat, out: &mut BTreeSet<Tuple>) {
    let syn::Pat::Tuple(tup) = pat else {
        return;
    };
    if tup.elems.len() != 2 {
        return;
    }

    // First element — byte literal for the final byte.
    let final_byte = match &tup.elems[0] {
        syn::Pat::Lit(syn::PatLit {
            lit: syn::Lit::Byte(b),
            ..
        }) => b.value(),
        _ => return,
    };

    // Second element — slice pattern of byte literals, OR a bare
    // `intermediates` identifier (catch-all, e.g. `configure_charset!`).
    let intermediates: Vec<u8> = match &tup.elems[1] {
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
            bytes
        }
        syn::Pat::Ident(_) => {
            // The catch-all `intermediates` case — e.g. the charset
            // designation macro that accepts `(`, `)`, `*`, `+`. We
            // emit four tuples, one per supported intermediate.
            for intermediate in [b'(', b')', b'*', b'+'] {
                out.insert(Tuple::new(
                    Category::Da,
                    [intermediate],
                    "-",
                    (final_byte as char).to_string(),
                ));
            }
            return;
        }
        _ => return,
    };

    out.insert(Tuple::new(
        Category::Esc,
        intermediates,
        "-",
        (final_byte as char).to_string(),
    ));
}

fn arm_to_dcs_tuple(pat: &syn::Pat) -> Option<Tuple> {
    let syn::Pat::Tuple(tup) = pat else {
        return None;
    };
    if tup.elems.len() != 2 {
        return None;
    }

    let final_byte = pattern_char_literal(&tup.elems[0])?;
    let intermediates = pattern_byte_slice(&tup.elems[1])?;

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
