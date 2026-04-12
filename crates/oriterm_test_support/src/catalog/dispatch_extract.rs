//! Dispatch arm + `NamedPrivateMode` extractors.
//!
//! Walks `crates/vte/src/ansi/dispatch/{csi,osc,mod}.rs` and
//! `crates/vte/src/ansi/types.rs` via the `syn` AST parser and
//! emits one [`Tuple`] per output sequence (not per match arm —
//! 1-to-many dispatch arms expand mechanically, per Phase 2
//! Finding G).
//!
//! Two extractors live here:
//!
//! - [`extract_dispatch_tuples`] — walks CSI / OSC / DCS / APC /
//!   PM / SOS dispatch arms.
//! - [`extract_namedprivatemode_tuples`] — walks
//!   `PrivateMode::new()`'s match arms to enumerate every
//!   supported DEC private mode number, emitting paired
//!   `(CSI, [?], Ps, h)` / `(CSI, [?], Ps, l)` tuples per
//!   variant.
//!
//! Disjointness invariant: `extract_dispatch_tuples` filters out
//! every `(CSI, [?], Ps, h|l)` tuple before emitting, so the
//! intersection of its output with `extract_namedprivatemode_tuples`
//! is empty. `--check` unions both sets.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use super::tuple::{Category, Tuple};

/// Error returned by the dispatch extractors.
#[derive(Debug)]
pub enum DispatchExtractError {
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
    Parse {
        path: PathBuf,
        source: syn::Error,
    },
}

impl core::fmt::Display for DispatchExtractError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Io { path, source } => write!(f, "io error reading {}: {source}", path.display()),
            Self::Parse { path, source } => {
                write!(f, "syn parse error in {}: {source}", path.display())
            }
        }
    }
}

impl std::error::Error for DispatchExtractError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            Self::Parse { source, .. } => Some(source),
        }
    }
}

/// Walk the VTE dispatch tree and emit the canonical dispatch tuple set.
///
/// Scans `crates/vte/src/ansi/dispatch/{csi,osc,mod}.rs` and
/// `crates/vte/src/lib.rs` relative to `workspace_root`. Filters
/// out DECSET/DECRST pairs — those come exclusively from
/// [`extract_namedprivatemode_tuples`] so the two extractors are
/// disjoint.
///
/// # Errors
///
/// Returns [`DispatchExtractError`] on io failures or `syn` parse
/// errors.
pub fn extract_dispatch_tuples(workspace_root: &Path) -> Result<Vec<Tuple>, DispatchExtractError> {
    let mut tuples: BTreeSet<Tuple> = BTreeSet::new();

    let csi_path = workspace_root.join("crates/vte/src/ansi/dispatch/csi.rs");
    extract_csi_arms(&csi_path, &mut tuples)?;

    let osc_path = workspace_root.join("crates/vte/src/ansi/dispatch/osc.rs");
    extract_osc_arms(&osc_path, &mut tuples)?;

    let mod_path = workspace_root.join("crates/vte/src/ansi/dispatch/mod.rs");
    extract_mod_arms(&mod_path, &mut tuples)?;

    // SGR numeric universe — every numeric param handled by
    // `attrs_from_sgr_parameters`. We reparse csi.rs since the
    // function lives there.
    extract_sgr_params(&csi_path, &mut tuples)?;

    // Filter out DECSET/DECRST tuples — those come exclusively from
    // `extract_namedprivatemode_tuples`. The disjointness invariant
    // is tested in sibling tests.
    Ok(tuples
        .into_iter()
        .filter(|t| {
            !(t.category == Category::Csi
                && t.intermediates == [b'?']
                && (t.final_byte == "h" || t.final_byte == "l"))
        })
        .collect())
}

/// Walk `crates/vte/src/ansi/types.rs::PrivateMode::new` relative to
/// `workspace_root` and emit a paired `(CSI, [?], Ps, h)` /
/// `(CSI, [?], Ps, l)` tuple per `NamedPrivateMode` variant.
///
/// `Ps` in the tuple `params` field is literally the decimal
/// string of the numeric value (`"1"`, `"25"`, `"2026"`, …) — NOT
/// the placeholder `Ps`. This lets `--check` match a catalog row
/// for mode 2026 against the `(CSI, [?], 2026, h)` tuple.
///
/// # Errors
///
/// Returns [`DispatchExtractError`] on io or parse failure.
pub fn extract_namedprivatemode_tuples(
    workspace_root: &Path,
) -> Result<Vec<Tuple>, DispatchExtractError> {
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

fn extract_csi_arms(path: &Path, out: &mut BTreeSet<Tuple>) -> Result<(), DispatchExtractError> {
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

fn is_action_intermediates_scrutinee(expr: &syn::Expr) -> bool {
    // The scrutinee is `(action, intermediates)`. We accept any tuple
    // with exactly two path-expression elements where the identifiers
    // are "action" and "intermediates".
    let syn::Expr::Tuple(t) = expr else {
        return false;
    };
    if t.elems.len() != 2 {
        return false;
    }
    let has_action = matches!(&t.elems[0], syn::Expr::Path(p) if p.path.is_ident("action"));
    let has_inter = matches!(&t.elems[1], syn::Expr::Path(p) if p.path.is_ident("intermediates"));
    has_action && has_inter
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

fn pattern_char_literal(pat: &syn::Pat) -> Option<char> {
    match pat {
        syn::Pat::Lit(syn::PatLit {
            lit: syn::Lit::Char(c),
            ..
        }) => Some(c.value()),
        _ => None,
    }
}

fn pattern_byte_slice(pat: &syn::Pat) -> Option<Vec<u8>> {
    let syn::Pat::Slice(slice) = pat else {
        return None;
    };
    let mut bytes = Vec::new();
    for elem in &slice.elems {
        let syn::Pat::Lit(syn::PatLit {
            lit: syn::Lit::Byte(b),
            ..
        }) = elem
        else {
            return None;
        };
        bytes.push(b.value());
    }
    Some(bytes)
}

fn extract_osc_arms(path: &Path, out: &mut BTreeSet<Tuple>) -> Result<(), DispatchExtractError> {
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

fn extract_mod_arms(path: &Path, out: &mut BTreeSet<Tuple>) -> Result<(), DispatchExtractError> {
    let source = read_rust(path)?;
    let file = parse_rust(path, &source)?;

    // We walk this file looking for ESC-byte match patterns and the
    // DCS `hook` function. `Performer::execute` dispatches C0 bytes;
    // the catalog treats those as per-byte rows covered under
    // ecma-48.md directly, so we do not emit tuples for execute.
    let mut v = ModVisitor { out };
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

fn extract_sgr_params(path: &Path, out: &mut BTreeSet<Tuple>) -> Result<(), DispatchExtractError> {
    let source = read_rust(path)?;
    let file = parse_rust(path, &source)?;

    // Find the `attrs_from_sgr_parameters` function and walk its
    // `match param { [0] => ..., ... }` to extract every supported
    // numeric SGR parameter.
    for item in &file.items {
        let syn::Item::Fn(func) = item else { continue };
        if func.sig.ident != "attrs_from_sgr_parameters" {
            continue;
        }
        collect_sgr_numeric_from_block(&func.block, out);
    }

    Ok(())
}

fn collect_sgr_numeric_from_block(block: &syn::Block, out: &mut BTreeSet<Tuple>) {
    struct SgrVisitor<'a> {
        out: &'a mut BTreeSet<Tuple>,
    }

    impl syn::visit::Visit<'_> for SgrVisitor<'_> {
        fn visit_expr_match(&mut self, m: &syn::ExprMatch) {
            for arm in &m.arms {
                collect_sgr_arm(&arm.pat, self.out);
            }
            // Don't descend further — the SGR match is the single
            // top-level match we care about.
        }
    }

    let mut v = SgrVisitor { out };
    syn::visit::Visit::visit_block(&mut v, block);
}

fn collect_sgr_arm(pat: &syn::Pat, out: &mut BTreeSet<Tuple>) {
    // Arm is a slice pattern: `[0]`, `[4, 0]`, `[38]`, `[38, params @ ..]`.
    // We extract the FIRST numeric literal as the SGR parameter code,
    // since `[38, ...]` variants all belong to SGR 38.
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

fn type_path_ends_with(ty: &syn::Type, want: &str) -> bool {
    let syn::Type::Path(tp) = ty else {
        return false;
    };
    tp.path.segments.last().is_some_and(|seg| seg.ident == want)
}

fn read_rust(path: &Path) -> Result<String, DispatchExtractError> {
    fs::read_to_string(path).map_err(|source| DispatchExtractError::Io {
        path: path.to_path_buf(),
        source,
    })
}

fn parse_rust(path: &Path, source: &str) -> Result<syn::File, DispatchExtractError> {
    syn::parse_file(source).map_err(|source| DispatchExtractError::Parse {
        path: path.to_path_buf(),
        source,
    })
}
