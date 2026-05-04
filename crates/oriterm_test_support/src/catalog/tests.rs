//! Module-level integration tests for the `catalog` module.
//!
//! Per-submodule unit tests live in their respective sibling
//! `tests.rs` files:
//! - `tuple/tests.rs` — canonical tuple parsing and Tuple equality
//! - `check/tests.rs` — `--check` pass finding detection
//! - `parser/tests.rs` — catalog markdown table parsing
//!
//! This file covers cross-module integration tests, positive-pin
//! tests for extractors, and the `walk_catalog_files` function
//! from `mod.rs`.
//!
//! ## Cross-producer SSOT-alignment matrix — per-producer params shape contract
//!
//! Four producers feed `TupleSig` comparison consumed by
//! `spec-coverage-report --check`. Each has a deliberate, by-design
//! `params` shape; signatures (`category`, `intermediates`,
//! `final_byte`) MUST agree across all producers per category, but
//! `params` may diverge because each producer has different
//! information available at construction time:
//!
//! | Category | Catalog (`canonical_tuple`) | Dispatch walker | Capture (`extract_capture_tuples`) | Runtime (`perform_action_to_tuple`) |
//! |---|---|---|---|---|
//! | OSC | `osc_placeholder` payload | `""` | `osc_placeholder` payload | `""` |
//! | CSI | tokens (`Ps;Ps`/`Ps`/`-`) | `"Ps"` (arm pattern) | `csi_params_placeholder(arity)` | `""` |
//! | DCS | `Pid` (q+empty ints) / `Pt` | matches catalog | matches catalog | `""` |
//! | ESC | `"-"` | `"-"` | `"-"` | `""` |
//! | APC | `"Pt"` (generic) / `"key-value"` (\_G) | `"Pt"` | `"Pt"` | `None` (filtered) |
//!
//! `TupleSig` excludes `params` (see `tuple/mod.rs:18`), so signature
//! equality alone would hide intentional producer divergence. The
//! per-category sibling matrix tests below pin both signature alignment
//! AND per-producer params shape; together they prevent the
//! `("CAT", [], TERMINATOR)` collapse class of regression that
//! retrofit-fixed for OSC.

use super::tuple::{Category, Tuple};
use super::walk_catalog_files;
use super::{
    Classification, build_dispatch_map, canonical_tuple, classify_from_map, extract_capture_tuples,
    extract_dispatch_tuples, extract_namedprivatemode_tuples,
};

use vte::ansi::PerformAction;

use crate::spec_chain::uncataloged::{UncatalogedDetector, perform_action_to_tuple};

/// Resolve the term_repo workspace root via the canonical SSOT helper.
/// All call sites in this file use term-repo-relative paths (vte source
/// scanning); `paths::term_workspace_root()` is always available and has
/// no wrapper concern. See `bug-tracker/plans/completed//`.
fn workspace_root() -> &'static std::path::Path {
    crate::paths::term_workspace_root()
}

// -------- walk_catalog_files -----------------------------------------------

#[test]
fn walk_catalog_files_returns_sorted_md_paths() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let dir = tmp.path().join("catalog");
    std::fs::create_dir_all(&dir).expect("create dir");
    // Create files in reverse alphabetical order.
    for name in ["z.md", "a.md", "m.md", "README.md", "_mapping.md"] {
        std::fs::write(dir.join(name), "# stub").expect("write");
    }
    let paths = walk_catalog_files(&dir).expect("walk succeeds");
    let names: Vec<&str> = paths
        .iter()
        .filter_map(|p| p.file_name()?.to_str())
        .collect();
    // README.md and _mapping.md are excluded; remaining sorted.
    assert_eq!(names, vec!["a.md", "m.md", "z.md"]);
}

#[test]
fn walk_catalog_files_skips_non_md_files() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let dir = tmp.path().join("catalog");
    std::fs::create_dir_all(&dir).expect("create dir");
    std::fs::write(dir.join("data.csv"), "csv content").expect("write");
    std::fs::write(dir.join("ecma-48.md"), "# catalog").expect("write");
    let paths = walk_catalog_files(&dir).expect("walk succeeds");
    assert_eq!(paths.len(), 1);
}

// -------- Positive-pin: extract_dispatch_tuples ----------------------------

#[test]
fn extract_dispatch_tuples_includes_known_csi_tuples() {
    let root = workspace_root();
    let csi_path = root.join("crates/vte/src/ansi/dispatch/csi/mod.rs");
    if !csi_path.exists() {
        eprintln!(
            "SKIP: VTE dispatch source not found at {}",
            csi_path.display()
        );
        return;
    }
    let tuples = extract_dispatch_tuples(&root).expect("extraction succeeds");

    // CUP: CSI Ps ; Ps H — cursor position, universally implemented.
    let has_cup = tuples
        .iter()
        .any(|t| t.category == Category::Csi && t.intermediates.is_empty() && t.final_byte == "H");
    assert!(has_cup, "CUP (CSI H) must be present in dispatch tuples");

    // ED: CSI Ps J — erase in display.
    let has_ed = tuples
        .iter()
        .any(|t| t.category == Category::Csi && t.intermediates.is_empty() && t.final_byte == "J");
    assert!(has_ed, "ED (CSI J) must be present in dispatch tuples");

    // SGR: CSI Ps m — at least one SGR parameter tuple.
    let has_sgr = tuples
        .iter()
        .any(|t| t.category == Category::Csi && t.intermediates.is_empty() && t.final_byte == "m");
    assert!(has_sgr, "SGR (CSI m) must be present in dispatch tuples");

    // APC: unconditional dispatch.
    let has_apc = tuples
        .iter()
        .any(|t| t.category == Category::Apc && t.final_byte == "ST");
    assert!(has_apc, "APC must be present in dispatch tuples");
}

// -------- Positive-pin: extract_namedprivatemode_tuples --------------------

#[test]
fn extract_namedprivatemode_tuples_includes_known_modes() {
    let root = workspace_root();
    let types_path = root.join("crates/vte/src/ansi/types.rs");
    if !types_path.exists() {
        eprintln!("SKIP: types.rs not found at {}", types_path.display());
        return;
    }
    let tuples = extract_namedprivatemode_tuples(&root).expect("extraction succeeds");

    // DECCKM (mode 1) — cursor key mode.
    let has_decckm_set = tuples.iter().any(|t| {
        t.category == Category::Csi
            && t.intermediates == [b'?']
            && t.params == "1"
            && t.final_byte == "h"
    });
    assert!(has_decckm_set, "DECCKM set (CSI ? 1 h) must be present");

    // DECTCEM (mode 25) — show/hide cursor.
    let has_dectcem_reset = tuples.iter().any(|t| {
        t.category == Category::Csi
            && t.intermediates == [b'?']
            && t.params == "25"
            && t.final_byte == "l"
    });
    assert!(
        has_dectcem_reset,
        "DECTCEM reset (CSI ? 25 l) must be present"
    );

    // Every mode number should produce exactly 2 tuples (h + l).
    assert_eq!(
        tuples.len() % 2,
        0,
        "NamedPrivateMode tuples must come in h/l pairs"
    );
}

// -------- Positive-pin: build_dispatch_map ---------------------------------

#[test]
fn build_dispatch_map_includes_known_handler_names() {
    let root = workspace_root();
    let csi_path = root.join("crates/vte/src/ansi/dispatch/csi/mod.rs");
    if !csi_path.exists() {
        eprintln!(
            "SKIP: VTE dispatch source not found at {}",
            csi_path.display()
        );
        return;
    }
    let map = build_dispatch_map(&root).expect("dispatch map builds");

    // The map should contain at least CSI, OSC, ESC, and APC entries.
    let categories: std::collections::BTreeSet<_> = map.keys().map(|t| t.category).collect();
    assert!(
        categories.contains(&Category::Csi),
        "dispatch map must contain CSI entries"
    );
    assert!(
        categories.contains(&Category::Osc),
        "dispatch map must contain OSC entries"
    );

    // Verify a known handler: CUP (CSI H) should call `goto`.
    let cup_key = map
        .keys()
        .find(|t| t.category == Category::Csi && t.intermediates.is_empty() && t.final_byte == "H");
    assert!(cup_key.is_some(), "CUP must be in dispatch map");
    let cup_handlers = &map[cup_key.unwrap()];
    assert!(
        cup_handlers.contains("goto"),
        "CUP handler must include `goto`, got: {cup_handlers:?}"
    );

    // NamedPrivateMode entries should have set/unset handlers.
    let has_set_pm = map.values().any(|hs| hs.contains("set_private_mode"));
    assert!(
        has_set_pm,
        "dispatch map must contain set_private_mode handler"
    );
    let has_unset_pm = map.values().any(|hs| hs.contains("unset_private_mode"));
    assert!(
        has_unset_pm,
        "dispatch map must contain unset_private_mode handler"
    );

    // APC handler.
    let has_apc_handler = map.values().any(|hs| hs.contains("apc_dispatch"));
    assert!(
        has_apc_handler,
        "dispatch map must contain apc_dispatch handler"
    );
}

// -------- Cross-producer SSOT alignment matrix ----------------
//
// Four producers construct OSC tuples that MUST yield identical
// `TupleSig` for the same OSC sequence:
//
//   1. catalog `parse_osc` (`canonical_tuple`)            — `tuple/canonical.rs`
//   2. dispatch `extract_dispatch_tuples`                  — `dispatch_extract/osc.rs`
//   3. capture `extract_capture_tuples`                    — `capture_extract.rs`
//   4. runtime `UncatalogedDetector::feed_actions`         — `spec_chain/uncataloged/mod.rs`
//
// Pre-fix, producer 4 alone placed the OSC selector in `final_byte`
// and producers 1+2+3 placed it in `params` with the terminator in
// `final_byte` (collapsing all OSCs to `("OSC", [], "BEL")`). After
// the SSOT alignment, all four put the selector in `final_byte`.
//
// OSC 7/9/99/133/633/777 are owned by `oriterm_mux::shell_integration::
// RawInterceptor` and have NO arm in `crates/vte/src/ansi/dispatch/osc.rs`.
// The matrix excludes them — including them would assert against a
// producer-2 source that does not exist.

/// Synthesize an OSC byte stream for the given selector + payload args.
/// Used to drive producers 3 (capture) and 4 (runtime).
fn osc_bytes(selector: &str, payload: &[&str]) -> Vec<u8> {
    let mut s = String::new();
    s.push_str("\x1b]"); // ESC ]
    s.push_str(selector);
    for p in payload {
        s.push(';');
        s.push_str(p);
    }
    s.push('\x07'); // BEL
    s.into_bytes()
}

/// Build a `Sequence`-column markdown string for the given selector +
/// payload placeholders. Used to drive producer 1 (catalog parse_osc).
fn catalog_sequence_string(selector: &str, payload_placeholders: &[&str]) -> String {
    let mut s = format!("`OSC {selector}");
    for p in payload_placeholders {
        s.push_str(" ; ");
        s.push_str(p);
    }
    s.push_str(" BEL|ST`");
    s
}

/// Producer 4: feed a synthesized `PerformAction::OscDispatch` to
/// `perform_action_to_tuple` and return the producer's actual `Tuple`
/// (NOT a reconstructed proxy). Also exercise `UncatalogedDetector`
/// with the same action and assert its signature matches the producer
/// tuple's signature, so the observer-level invariant is verified
/// alongside the producer-level shape.
fn runtime_observer_tuple(selector: &str, payload: &[&str]) -> Tuple {
    let mut params: Vec<Vec<u8>> = vec![selector.as_bytes().to_vec()];
    for p in payload {
        params.push(p.as_bytes().to_vec());
    }
    let action = PerformAction::OscDispatch {
        params,
        bell_terminated: true,
    };

    // Observe the producer's actual Tuple — `params` shape is what the
    // producer constructs, not what the test reconstructs from a sig.
    let producer_tuple = perform_action_to_tuple(&action).expect("OscDispatch must yield a Tuple");

    // Also verify the UncatalogedDetector path produces the same signature.
    let mut detector = UncatalogedDetector::new();
    detector.feed_actions(&[action]);
    let detector_sig = detector
        .seen()
        .iter()
        .next()
        .cloned()
        .expect("UncatalogedDetector must produce one signature for OscDispatch");
    assert_eq!(
        producer_tuple.signature(),
        detector_sig,
        "perform_action_to_tuple and UncatalogedDetector must agree on the signature"
    );

    producer_tuple
}

/// Producer 3: write the OSC byte stream to a tempfile, run
/// `extract_capture_tuples`, return the OSC tuple verbatim (full shape
/// including `params` so callers can pin canonicalization).
fn capture_tuple(selector: &str, payload: &[&str]) -> Tuple {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("osc.cap");
    std::fs::write(&path, osc_bytes(selector, payload)).expect("write cap");
    let tuples = extract_capture_tuples(&path).expect("extract");
    tuples
        .into_iter()
        .find(|(t, _)| t.category == Category::Osc)
        .map(|(t, _)| t)
        .expect("capture must yield one OSC tuple")
}

/// Producer 2: walk the VTE dispatch tree and find the OSC tuple
/// whose `final_byte` matches `selector`. Returns `None` when the VTE
/// dispatch source file is absent (cross-compiled / packaged builds).
/// The dispatch walker leaves `params` empty by contract — the arm
/// only knows the selector, not the payload shape.
fn dispatch_tuple(selector: &str) -> Option<Tuple> {
    let root = workspace_root();
    let osc_path = root.join("crates/vte/src/ansi/dispatch/osc.rs");
    if !osc_path.exists() {
        return None;
    }
    let tuples = extract_dispatch_tuples(&root).expect("dispatch extraction succeeds");
    tuples
        .into_iter()
        .find(|t| t.category == Category::Osc && t.final_byte == selector)
}

/// Producer 1: canonicalize the catalog Sequence-column markdown
/// for the given selector + payload placeholders. Returns the full
/// `Tuple` so callers can pin the canonical `params` shape.
fn catalog_tuple(selector: &str, payload_placeholders: &[&str]) -> Tuple {
    let seq = catalog_sequence_string(selector, payload_placeholders);
    canonical_tuple(&seq).expect("catalog parse_osc must canonicalize OSC sequence")
}

/// The SSOT-alignment matrix. Each row is
/// `(selector, raw_payload_args, catalog_payload_placeholders)`.
/// Selectors are dispatched in `crates/vte/src/ansi/dispatch/osc.rs`
/// and exercise the four producers per §2 test matrix.
fn osc_ssot_matrix() -> Vec<(&'static str, Vec<&'static str>, Vec<&'static str>)> {
    vec![
        // (selector, raw payload args for runtime/capture, catalog payload placeholders)
        ("0", vec!["title"], vec!["Pt"]),
        ("4", vec!["1", "rgb:ff/00/00"], vec!["index", "rgb"]),
        ("10", vec!["rgb:ff/ff/ff"], vec!["spec"]),
        ("52", vec!["c", "aGVsbG8="], vec!["mode", "b64"]),
        ("104", vec!["1"], vec!["index"]),
        // OSC 110 is a zero-payload reset arm (`osc.rs:297`).
        ("110", vec![], vec![]),
        ("1337", vec!["File=name=...:base64bytes"], vec!["key=value"]),
        // Sun console aliases (`osc.rs:317-330`) — nonnumeric selectors.
        ("L", vec!["icon-name"], vec!["Pt"]),
        ("l", vec!["window-title"], vec!["Pt"]),
    ]
}

/// Regression: all four OSC tuple producers MUST yield
/// identical `TupleSig` for the same OSC sequence (selector lives in
/// `final_byte` after the SSOT alignment) AND each producer MUST
/// emit the per-producer `params` shape its contract promises:
/// catalog/capture carry the canonical payload placeholder string;
/// dispatch/runtime leave `params` empty (arm only knows the selector).
#[test]
fn osc_tuple_sig_aligns_across_all_four_producers() {
    let osc_path = workspace_root().join("crates/vte/src/ansi/dispatch/osc.rs");
    let dispatch_source_present = osc_path.exists();

    // Self-verifying matrix completeness counts each PRODUCER CELL
    // exercised, not each selector. With dispatch source present:
    // 9 selectors × 4 producers = 36 cells. Without it (cross-compiled
    // / packaged builds): 9 × 3 = 27 cells (catalog, capture, runtime).
    let mut producer_cells_exercised = 0_usize;

    for (selector, raw_payload, catalog_payload) in osc_ssot_matrix() {
        let t1 = catalog_tuple(selector, &catalog_payload);
        let t3 = capture_tuple(selector, &raw_payload);
        let t4 = runtime_observer_tuple(selector, &raw_payload);
        producer_cells_exercised += 3; // t1 + t3 + t4 always run

        // Selector lands in `final_byte` for every producer (SSOT).
        assert_eq!(
            t1.final_byte, selector,
            "selector {selector}: catalog must place selector in final_byte"
        );
        assert_eq!(
            t3.final_byte, selector,
            "selector {selector}: capture must place selector in final_byte"
        );
        assert_eq!(
            t4.final_byte, selector,
            "selector {selector}: runtime must place selector in final_byte"
        );

        // Per-producer `params` shape contract:
        // - catalog and capture canonicalize the payload via `osc_placeholder`,
        //   joined by `;`. They MUST agree byte-for-byte (both call the
        //   same SSOT canonicalization).
        // - runtime leaves `params` empty (the observer fast-path).
        assert_eq!(
            t1.params, t3.params,
            "selector {selector}: catalog and capture must agree on canonical params shape"
        );
        assert_eq!(
            t4.params, "",
            "selector {selector}: runtime observer params must be empty by contract"
        );

        // SSOT alignment: signatures (cat, ints, final) must match
        // across all producers regardless of `params` divergence —
        // this is what `spec-coverage-report --check` consumes.
        assert_eq!(
            t1.signature(),
            t3.signature(),
            "selector {selector}: catalog and capture signatures must match"
        );
        assert_eq!(
            t1.signature(),
            t4.signature(),
            "selector {selector}: catalog and runtime signatures must match"
        );

        // Producer 2 (dispatch) — every selector in this matrix has
        // an arm in `crates/vte/src/ansi/dispatch/osc.rs`, so the
        // lookup MUST succeed when the source file is present.
        if dispatch_source_present {
            let t2 = dispatch_tuple(selector).unwrap_or_else(|| {
                panic!(
                    "selector {selector}: dispatch_extract must yield a tuple with \
                     selector in final_byte (got dispatch tuples: {:?})",
                    extract_dispatch_tuples(&workspace_root())
                        .expect("extract")
                        .into_iter()
                        .filter(|t| t.category == Category::Osc)
                        .collect::<Vec<_>>()
                )
            });
            assert_eq!(
                t2.params, "",
                "selector {selector}: dispatch arm params must be empty by contract"
            );
            assert_eq!(
                t1.signature(),
                t2.signature(),
                "selector {selector}: catalog and dispatch signatures must match"
            );
            producer_cells_exercised += 1;
        }
    }

    let expected_cells = if dispatch_source_present { 36 } else { 27 };
    let producer_count = if dispatch_source_present { 4 } else { 3 };
    assert_eq!(
        producer_cells_exercised, expected_cells,
        "self-verifying matrix completeness — expected {expected_cells} producer cells \
         (9 selectors × {producer_count} producers), got {producer_cells_exercised}"
    );
}

/// Regression: distinct selectors yield distinct
/// signatures from each producer. Pre-fix all OSC TupleSig collapsed
/// to `("OSC", [], "BEL")` regardless of selector — this pin would
/// fail against the broken code.
#[test]
fn osc_tuple_sig_distinct_per_selector() {
    let s52 = runtime_observer_tuple("52", &["c", "aGVsbG8="]).signature();
    let s1337 = runtime_observer_tuple("1337", &["File=..."]).signature();
    let s4 = runtime_observer_tuple("4", &["1", "rgb:ff/00/00"]).signature();
    assert_ne!(s52, s1337, "OSC 52 and OSC 1337 must have distinct sigs");
    assert_ne!(s52, s4, "OSC 52 and OSC 4 must have distinct sigs");
    assert_ne!(s1337, s4, "OSC 1337 and OSC 4 must have distinct sigs");
}

/// Regression: no OSC TupleSig should carry "BEL" or
/// "ST" in `final_byte` after the SSOT alignment (selector took the
/// slot). Regression guard against the broken pre-fix shape.
#[test]
fn osc_tuple_sig_does_not_collapse_to_terminator() {
    for (selector, raw_payload, catalog_payload) in osc_ssot_matrix() {
        let p1 = catalog_tuple(selector, &catalog_payload).signature();
        let p3 = capture_tuple(selector, &raw_payload).signature();
        let p4 = runtime_observer_tuple(selector, &raw_payload).signature();
        for sig in [&p1, &p3, &p4] {
            assert_ne!(
                sig.2, "BEL",
                "selector {selector}: signature must not collapse to BEL terminator"
            );
            assert_ne!(
                sig.2, "ST",
                "selector {selector}: signature must not collapse to ST terminator"
            );
        }
    }
}

/// Regression: after the SSOT alignment, the OSC
/// normalization in `classify_from_map` (`classify/mod.rs:127-143`)
/// drops `params` and matches on `(category, intermediates,
/// final_byte)` — the simplest possible bridge between capture
/// shape (`params = "<payload>"`) and dispatch shape (`params = ""`).
/// The pre-fix `params.split(';')` extraction is gone.
#[test]
fn classify_from_map_osc_normalizes_via_final_byte_only() {
    let root = workspace_root();
    let osc_path = root.join("crates/vte/src/ansi/dispatch/osc.rs");
    if !osc_path.exists() {
        eprintln!("SKIP: VTE OSC dispatch source not found");
        return;
    }
    let map = build_dispatch_map(&root).expect("dispatch map builds");

    // Capture-shaped tuple for OSC 0 — selector in final_byte, payload
    // placeholder in params. The classifier's OSC normalization must
    // bridge this to the dispatch-shape tuple (empty params).
    let capture_shape = Tuple::new(Category::Osc, Vec::<u8>::new(), "text", "0");
    match classify_from_map(&map, &capture_shape) {
        Classification::Dispatched { .. } => {}
        Classification::NoDispatch => panic!(
            "classify_from_map must dispatch capture-shape OSC 0; \
             dispatch map has: {:?}",
            map.keys()
                .filter(|k| k.category == Category::Osc && k.final_byte == "0")
                .collect::<Vec<_>>()
        ),
    }

    // Capture-shape OSC 4 with multi-arg payload also bridges to dispatch.
    let osc4_capture = Tuple::new(Category::Osc, Vec::<u8>::new(), "index;rgb", "4");
    assert!(
        matches!(
            classify_from_map(&map, &osc4_capture),
            Classification::Dispatched { .. }
        ),
        "OSC 4 capture-shape must bridge to dispatch via classify_from_map"
    );

    // Interceptor-owned selector (OSC 7) has no dispatch arm —
    // classify_from_map correctly returns NoDispatch.
    let osc7_capture = Tuple::new(Category::Osc, Vec::<u8>::new(), "file:///cwd", "7");
    assert!(
        matches!(
            classify_from_map(&map, &osc7_capture),
            Classification::NoDispatch
        ),
        "OSC 7 (interceptor-owned) must return NoDispatch from classify_from_map"
    );
}

// =========================================================================
// CSI cross-producer SSOT-alignment matrix
// =========================================================================
//
// Mirrors the OSC matrix shape for CSI selectors. Per-producer params
// contract is TIERED (catalog/capture share arity-driven shape; dispatch
// is arity-agnostic `"Ps"`; runtime is empty by contract):
//
//   catalog.params == capture.params (Ps;Ps for arity 2, Ps for arity 1, - for arity 0)
//   dispatch.params == "Ps" (single placeholder regardless of arity)
//   runtime.params == "" (empty by contract — runtime observer fast-path)

/// Producer 4: feed a synthetic `PerformAction::CsiDispatch` to
/// `perform_action_to_tuple`. `args` carries the numeric parameter
/// arity (each entry becomes a single-element u16 vec).
fn runtime_csi_tuple(action_byte: char, args: &[u16]) -> Tuple {
    let params: Vec<Vec<u16>> = args.iter().map(|v| vec![*v]).collect();
    let action = PerformAction::CsiDispatch {
        params,
        intermediates: Vec::new(),
        ignore: false,
        action: action_byte,
    };
    perform_action_to_tuple(&action).expect("CsiDispatch must yield a Tuple")
}

/// Producer 3: synthesize a CSI byte stream and run it through
/// `extract_capture_tuples`, returning the CSI tuple verbatim.
fn capture_csi_tuple(action_byte: char, args: &[u16]) -> Tuple {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"\x1b[");
    for (i, v) in args.iter().enumerate() {
        if i > 0 {
            bytes.push(b';');
        }
        bytes.extend_from_slice(v.to_string().as_bytes());
    }
    bytes.push(action_byte as u8);
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("csi.cap");
    std::fs::write(&path, bytes).expect("write cap");
    let tuples = extract_capture_tuples(&path).expect("extract");
    tuples
        .into_iter()
        .find(|(t, _)| t.category == Category::Csi)
        .map(|(t, _)| t)
        .expect("capture must yield one CSI tuple")
}

/// Producer 2: walk the VTE dispatch tree and find the CSI tuple
/// `(CSI, [], <action_byte>)`. Returns `None` when the VTE dispatch
/// source file is absent (cross-compiled / packaged builds).
fn dispatch_csi_tuple(action_byte: char) -> Option<Tuple> {
    let root = workspace_root();
    let csi_path = root.join("crates/vte/src/ansi/dispatch/csi/mod.rs");
    if !csi_path.exists() {
        return None;
    }
    let tuples = extract_dispatch_tuples(root).expect("dispatch extraction succeeds");
    tuples.into_iter().find(|t| {
        t.category == Category::Csi
            && t.intermediates.is_empty()
            && t.final_byte == action_byte.to_string()
    })
}

/// Producer 1: build a Sequence-column markdown like `` `CSI Ps;Ps H` ``
/// (arity Ps tokens joined by `;` + final_byte) and canonicalize.
/// `;`-joining matches the catalog's actual `Sequence`-column form for
/// multi-parameter CSI rows (e.g., CUP: `CSI Ps;Ps H`); space-joined
/// `Ps Ps` would canonicalize to `PsPs` because `normalize_csi_params`
/// strips whitespace from the joined token sequence.
fn catalog_csi_tuple(action_byte: char, arity: usize) -> Tuple {
    let mut s = String::from("`CSI");
    if arity > 0 {
        s.push(' ');
        let placeholders: Vec<&str> = vec!["Ps"; arity];
        s.push_str(&placeholders.join(";"));
    }
    s.push(' ');
    s.push(action_byte);
    s.push('`');
    canonical_tuple(&s).expect("catalog parse_csi must canonicalize CSI sequence")
}

/// The CSI SSOT-alignment matrix. Each row is `(action_byte, args)`.
/// Selectors are real dispatch-backed entries (CUP `H`, ED `J`).
fn csi_ssot_matrix() -> Vec<(char, Vec<u16>)> {
    vec![('H', vec![1, 1]), ('J', vec![0])]
}

/// Regression: all four CSI tuple producers MUST yield identical
/// `TupleSig` for the same CSI sequence (selector in `final_byte`,
/// empty intermediates) AND each producer MUST emit the per-producer
/// `params` shape its contract promises (catalog/capture arity-driven,
/// dispatch arity-agnostic `Ps`, runtime empty).
#[test]
fn csi_tuple_sig_aligns_across_all_four_producers() {
    let csi_path = workspace_root().join("crates/vte/src/ansi/dispatch/csi/mod.rs");
    let dispatch_source_present = csi_path.exists();

    let mut producer_cells_exercised = 0_usize;

    for (action_byte, args) in csi_ssot_matrix() {
        let arity = args.len();
        let t1 = catalog_csi_tuple(action_byte, arity);
        let t3 = capture_csi_tuple(action_byte, &args);
        let t4 = runtime_csi_tuple(action_byte, &args);
        producer_cells_exercised += 3;

        // Selector lands in `final_byte`, intermediates empty.
        assert_eq!(t1.final_byte, action_byte.to_string());
        assert!(t1.intermediates.is_empty());
        assert_eq!(t3.final_byte, action_byte.to_string());
        assert!(t3.intermediates.is_empty());
        assert_eq!(t4.final_byte, action_byte.to_string());
        assert!(t4.intermediates.is_empty());

        // Per-producer params shape contract:
        // catalog/capture share arity-driven shape; runtime empty.
        let expected_arity_shape = if arity == 0 {
            "-".to_string()
        } else {
            vec!["Ps"; arity].join(";")
        };
        assert_eq!(
            t1.params, expected_arity_shape,
            "selector {action_byte}: catalog params must be arity-driven Ps shape"
        );
        assert_eq!(
            t3.params, expected_arity_shape,
            "selector {action_byte}: capture params must match catalog (arity-driven Ps shape)"
        );
        assert_eq!(
            t4.params, "",
            "selector {action_byte}: runtime observer params must be empty by contract"
        );

        // Signature alignment.
        assert_eq!(
            t1.signature(),
            t3.signature(),
            "selector {action_byte}: catalog and capture signatures must match"
        );
        assert_eq!(
            t1.signature(),
            t4.signature(),
            "selector {action_byte}: catalog and runtime signatures must match"
        );

        if dispatch_source_present {
            let t2 = dispatch_csi_tuple(action_byte).unwrap_or_else(|| {
                panic!(
                    "selector {action_byte}: dispatch_extract must yield a CSI tuple with \
                     selector {action_byte} and empty intermediates"
                )
            });
            // Dispatch always emits "Ps" (arity-agnostic arm pattern).
            assert_eq!(
                t2.params, "Ps",
                "selector {action_byte}: dispatch arm params must be arity-agnostic 'Ps'"
            );
            assert_eq!(
                t1.signature(),
                t2.signature(),
                "selector {action_byte}: catalog and dispatch signatures must match"
            );
            producer_cells_exercised += 1;
        }
    }

    let expected_cells = if dispatch_source_present { 8 } else { 6 };
    let producer_count = if dispatch_source_present { 4 } else { 3 };
    assert_eq!(
        producer_cells_exercised, expected_cells,
        "CSI matrix completeness — expected {expected_cells} producer cells \
         (2 selectors × {producer_count} producers), got {producer_cells_exercised}"
    );
}

/// Regression: distinct CSI selectors yield distinct signatures.
#[test]
fn csi_tuple_sig_distinct_per_selector() {
    let cup = runtime_csi_tuple('H', &[1, 1]).signature();
    let ed = runtime_csi_tuple('J', &[0]).signature();
    assert_ne!(
        cup, ed,
        "CSI H (CUP) and CSI J (ED) must have distinct sigs"
    );
}

/// Regression: CSI `final_byte` slot MUST hold the dispatch action
/// character, never an empty string and never a CSI intermediate
/// byte (`?`/`>`/`=`/`!`/`"`/`#`/`$`). Regression guard against the
/// pre-fix-style collapse where catalog incorrectly absorbed an
/// intermediate byte into `final_byte`.
#[test]
fn csi_tuple_sig_does_not_collapse_to_intermediate() {
    for (action_byte, args) in csi_ssot_matrix() {
        let t = runtime_csi_tuple(action_byte, &args);
        assert!(
            !t.final_byte.is_empty(),
            "selector {action_byte}: final_byte must not be empty"
        );
        for forbidden in ['?', '>', '=', '!', '"', '#', '$'] {
            assert_ne!(
                t.final_byte.chars().next(),
                Some(forbidden),
                "selector {action_byte}: final_byte must not be intermediate '{forbidden}'"
            );
        }
    }
}

// =========================================================================
// DCS cross-producer SSOT-alignment matrix
// =========================================================================
//
// Per-producer params contract: catalog == capture == dispatch (all
// emit `Pid` for q+empty-intermediates, `Pt` otherwise); runtime
// is empty by contract.

/// Producer 4: feed a synthetic `PerformAction::Hook` to
/// `perform_action_to_tuple`.
fn runtime_dcs_tuple(intermediates: &[u8], action_byte: char) -> Tuple {
    let action = PerformAction::Hook {
        params: Vec::new(),
        intermediates: intermediates.to_vec(),
        ignore: false,
        action: action_byte,
    };
    perform_action_to_tuple(&action).expect("Hook must yield a Tuple")
}

/// Producer 3: synthesize a DCS byte stream, run through
/// `extract_capture_tuples`, return the DCS tuple verbatim.
fn capture_dcs_tuple(intermediates: &[u8], action_byte: char) -> Tuple {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"\x1bP");
    bytes.extend_from_slice(intermediates);
    bytes.push(action_byte as u8);
    bytes.extend_from_slice(b"\x1b\\");
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("dcs.cap");
    std::fs::write(&path, bytes).expect("write cap");
    let tuples = extract_capture_tuples(&path).expect("extract");
    tuples
        .into_iter()
        .find(|(t, _)| t.category == Category::Dcs)
        .map(|(t, _)| t)
        .expect("capture must yield one DCS tuple")
}

/// Producer 2: walk the VTE dispatch tree, find the DCS tuple.
fn dispatch_dcs_tuple(intermediates: &[u8], action_byte: char) -> Option<Tuple> {
    let root = workspace_root();
    let mod_path = root.join("crates/vte/src/ansi/dispatch/mod.rs");
    if !mod_path.exists() {
        return None;
    }
    let tuples = extract_dispatch_tuples(root).expect("dispatch extraction succeeds");
    tuples.into_iter().find(|t| {
        t.category == Category::Dcs
            && t.intermediates == intermediates
            && t.final_byte == action_byte.to_string()
    })
}

/// Producer 1: build `` `DCS [intermediate ]<final> Pt ST` `` and
/// canonicalize.
fn catalog_dcs_tuple(intermediates: &[u8], action_byte: char) -> Tuple {
    let mut s = String::from("`DCS");
    for b in intermediates {
        s.push(' ');
        s.push(*b as char);
    }
    if intermediates.is_empty() && action_byte == 'q' {
        // Sixel form: emit Ps params before final to drive parse_dcs's
        // Pid arm (non-empty body so the Pid heuristic fires).
        s.push_str(" Ps");
    }
    s.push(' ');
    s.push(action_byte);
    s.push_str(" Pt ST`");
    canonical_tuple(&s).expect("catalog parse_dcs must canonicalize DCS sequence")
}

/// The DCS SSOT-alignment matrix. Each row is `(intermediates, final_byte)`.
/// `(b"", 'q')` = sixel (Pid). `(b"$", 'q')` = DECRQM-q (Pt).
fn dcs_ssot_matrix() -> Vec<(Vec<u8>, char)> {
    vec![(vec![], 'q'), (vec![b'$'], 'q')]
}

/// Regression: all four DCS tuple producers MUST yield identical
/// `TupleSig` AND identical `params` shape (Pid for q+empty-ints, Pt
/// otherwise); runtime is empty by contract.
#[test]
fn dcs_tuple_sig_aligns_across_all_four_producers() {
    let mod_path = workspace_root().join("crates/vte/src/ansi/dispatch/mod.rs");
    let dispatch_source_present = mod_path.exists();

    let mut producer_cells_exercised = 0_usize;

    for (intermediates, action_byte) in dcs_ssot_matrix() {
        let t1 = catalog_dcs_tuple(&intermediates, action_byte);
        let t3 = capture_dcs_tuple(&intermediates, action_byte);
        let t4 = runtime_dcs_tuple(&intermediates, action_byte);
        producer_cells_exercised += 3;

        // Signature alignment.
        assert_eq!(
            t1.signature(),
            t3.signature(),
            "DCS ({intermediates:?}, {action_byte}): catalog and capture signatures must match"
        );
        assert_eq!(
            t1.signature(),
            t4.signature(),
            "DCS ({intermediates:?}, {action_byte}): catalog and runtime signatures must match"
        );

        // Per-producer params contract: catalog == capture (Pid for
        // q+empty-ints, Pt otherwise); runtime empty.
        let expected_params = if action_byte == 'q' && intermediates.is_empty() {
            "Pid"
        } else {
            "Pt"
        };
        assert_eq!(
            t1.params, expected_params,
            "DCS ({intermediates:?}, {action_byte}): catalog params must be {expected_params}"
        );
        assert_eq!(
            t3.params, expected_params,
            "DCS ({intermediates:?}, {action_byte}): capture params must match catalog"
        );
        assert_eq!(
            t4.params, "",
            "DCS ({intermediates:?}, {action_byte}): runtime params must be empty by contract"
        );

        if dispatch_source_present {
            let t2 = dispatch_dcs_tuple(&intermediates, action_byte).unwrap_or_else(|| {
                panic!(
                    "DCS ({intermediates:?}, {action_byte}): dispatch_extract must yield a tuple"
                )
            });
            assert_eq!(
                t2.params, expected_params,
                "DCS ({intermediates:?}, {action_byte}): dispatch params must match catalog"
            );
            assert_eq!(
                t1.signature(),
                t2.signature(),
                "DCS ({intermediates:?}, {action_byte}): catalog and dispatch signatures must match"
            );
            producer_cells_exercised += 1;
        }
    }

    let expected_cells = if dispatch_source_present { 8 } else { 6 };
    let producer_count = if dispatch_source_present { 4 } else { 3 };
    assert_eq!(
        producer_cells_exercised, expected_cells,
        "DCS matrix completeness — expected {expected_cells} producer cells \
         (2 inputs × {producer_count} producers), got {producer_cells_exercised}"
    );
}

/// Regression: distinct DCS inputs yield distinct signatures
/// (different intermediates).
#[test]
fn dcs_tuple_sig_distinct_per_intermediates() {
    let sixel = runtime_dcs_tuple(&[], 'q').signature();
    let decrqm = runtime_dcs_tuple(&[b'$'], 'q').signature();
    assert_ne!(
        sixel, decrqm,
        "DCS sixel (q, []) and DECRQM-q (q, [$]) must have distinct sigs"
    );
}

/// Regression: `Pid` params is reachable ONLY when `final == 'q'` AND
/// intermediates are empty. Regression guard: any other shape produces `Pt`.
#[test]
fn dcs_tuple_sig_pid_pt_split_pinned() {
    // Sixel: empty intermediates + q → Pid.
    let sixel = capture_dcs_tuple(&[], 'q');
    assert_eq!(sixel.params, "Pid");

    // DECRQM-q: $ intermediate + q → Pt.
    let decrqm = capture_dcs_tuple(&[b'$'], 'q');
    assert_eq!(decrqm.params, "Pt");
    assert_ne!(decrqm.params, "Pid");
}

// =========================================================================
// ESC cross-producer SSOT-alignment matrix
// =========================================================================
//
// Per-producer params contract: catalog == capture == dispatch (all
// emit `"-"`); runtime is empty by contract.

/// Producer 4: feed a synthetic `PerformAction::EscDispatch`.
fn runtime_esc_tuple(byte: u8) -> Tuple {
    let action = PerformAction::EscDispatch {
        intermediates: Vec::new(),
        ignore: false,
        byte,
    };
    perform_action_to_tuple(&action).expect("EscDispatch must yield a Tuple")
}

/// Producer 3: synthesize an ESC byte stream, run through capture.
fn capture_esc_tuple(byte: u8) -> Tuple {
    let bytes = vec![0x1b, byte];
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("esc.cap");
    std::fs::write(&path, bytes).expect("write cap");
    let tuples = extract_capture_tuples(&path).expect("extract");
    tuples
        .into_iter()
        .find(|(t, _)| t.category == Category::Esc)
        .map(|(t, _)| t)
        .expect("capture must yield one ESC tuple")
}

/// Producer 2: walk the VTE dispatch tree, find the ESC tuple.
fn dispatch_esc_tuple(byte: u8) -> Option<Tuple> {
    let root = workspace_root();
    let mod_path = root.join("crates/vte/src/ansi/dispatch/mod.rs");
    if !mod_path.exists() {
        return None;
    }
    let tuples = extract_dispatch_tuples(root).expect("dispatch extraction succeeds");
    tuples.into_iter().find(|t| {
        t.category == Category::Esc
            && t.intermediates.is_empty()
            && t.final_byte == (byte as char).to_string()
    })
}

/// Producer 1: build `` `ESC <byte>` `` and canonicalize.
fn catalog_esc_tuple(byte: u8) -> Tuple {
    let s = format!("`ESC {}`", byte as char);
    canonical_tuple(&s).expect("catalog parse_esc must canonicalize ESC sequence")
}

/// The ESC SSOT-alignment matrix. `D` = IND, `M` = RI.
fn esc_ssot_matrix() -> Vec<u8> {
    vec![b'D', b'M']
}

/// Regression: all four ESC tuple producers MUST yield identical
/// `TupleSig` AND identical `params == "-"` shape; runtime is empty.
#[test]
fn esc_tuple_sig_aligns_across_all_four_producers() {
    let mod_path = workspace_root().join("crates/vte/src/ansi/dispatch/mod.rs");
    let dispatch_source_present = mod_path.exists();

    let mut producer_cells_exercised = 0_usize;

    for byte in esc_ssot_matrix() {
        let t1 = catalog_esc_tuple(byte);
        let t3 = capture_esc_tuple(byte);
        let t4 = runtime_esc_tuple(byte);
        producer_cells_exercised += 3;

        let final_str = (byte as char).to_string();

        // Signature alignment.
        assert_eq!(
            t1.signature(),
            t3.signature(),
            "ESC {final_str}: catalog and capture signatures must match"
        );
        assert_eq!(
            t1.signature(),
            t4.signature(),
            "ESC {final_str}: catalog and runtime signatures must match"
        );

        // Per-producer params contract: catalog == capture == "-",
        // runtime empty.
        assert_eq!(
            t1.params, "-",
            "ESC {final_str}: catalog params must be \"-\""
        );
        assert_eq!(
            t3.params, "-",
            "ESC {final_str}: capture params must be \"-\""
        );
        assert_eq!(
            t4.params, "",
            "ESC {final_str}: runtime params must be empty by contract"
        );

        if dispatch_source_present {
            let t2 = dispatch_esc_tuple(byte)
                .unwrap_or_else(|| panic!("ESC {final_str}: dispatch_extract must yield a tuple"));
            assert_eq!(
                t2.params, "-",
                "ESC {final_str}: dispatch params must be \"-\""
            );
            assert_eq!(
                t1.signature(),
                t2.signature(),
                "ESC {final_str}: catalog and dispatch signatures must match"
            );
            producer_cells_exercised += 1;
        }
    }

    let expected_cells = if dispatch_source_present { 8 } else { 6 };
    let producer_count = if dispatch_source_present { 4 } else { 3 };
    assert_eq!(
        producer_cells_exercised, expected_cells,
        "ESC matrix completeness — expected {expected_cells} producer cells \
         (2 selectors × {producer_count} producers), got {producer_cells_exercised}"
    );
}

/// Regression: distinct ESC selectors yield distinct signatures.
#[test]
fn esc_tuple_sig_distinct_per_selector() {
    let ind = runtime_esc_tuple(b'D').signature();
    let ri = runtime_esc_tuple(b'M').signature();
    assert_ne!(
        ind, ri,
        "ESC D (IND) and ESC M (RI) must have distinct sigs"
    );
}

/// Regression: ESC `( B` (charset designation) routes to
/// `Category::Da`, NOT `Category::Esc`. Pinned in `capture_extract.rs`
/// (charset intermediates `(`/`)`/`*`/`+`) and in the dispatch walker's
/// charset DA arms.
#[test]
fn esc_charset_designation_routes_to_da_not_esc() {
    let bytes = vec![0x1b, b'(', b'B'];
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("charset.cap");
    std::fs::write(&path, bytes).expect("write cap");
    let tuples = extract_capture_tuples(&path).expect("extract");

    // The capture pipeline routes `ESC ( B` to Category::Da.
    let da = tuples.iter().any(|(t, _)| {
        t.category == Category::Da && t.intermediates == vec![b'('] && t.final_byte == "B"
    });
    assert!(da, "ESC ( B must route to Category::Da");

 // Regression guard: it MUST NOT appear under Category::Esc.
    let esc_b = tuples
        .iter()
        .any(|(t, _)| t.category == Category::Esc && t.final_byte == "B");
    assert!(
        !esc_b,
        "ESC ( B must NOT be captured as Category::Esc — charset designations are DA"
    );
}

// =========================================================================
// APC cross-producer SSOT-alignment matrix (3 producers + runtime-None pin)
// =========================================================================
//
// APC has 3 tuple producers: catalog, dispatch, capture all emit the
// generic `(APC, [], "Pt", "ST")`. Runtime is FILTERED — `ApcEnd`
// returns `None` per `spec_chain/uncataloged/mod.rs:152-158` because
// `vte::Perform::apc_end()` does not surface the accumulated payload.
// `_G` (kitty graphics) lives in classification normalization, not
// the cross-producer matrix.

/// Producer 1: canonicalize generic APC.
fn catalog_apc_tuple() -> Tuple {
    canonical_tuple("`APC Pt ST`").expect("catalog must canonicalize generic APC")
}

/// Producer 2: walk the VTE dispatch tree, find the generic APC tuple.
fn dispatch_apc_tuple() -> Option<Tuple> {
    let root = workspace_root();
    let mod_path = root.join("crates/vte/src/ansi/dispatch/mod.rs");
    if !mod_path.exists() {
        return None;
    }
    let tuples = extract_dispatch_tuples(root).expect("dispatch extraction succeeds");
    tuples
        .into_iter()
        .find(|t| t.category == Category::Apc && t.intermediates.is_empty())
}

/// Producer 3: synthesize APC byte stream, run through capture.
fn capture_apc_tuple() -> Tuple {
    // `\x1b_<payload>\x1b\\` — APC start, payload, ST terminator.
    let bytes: Vec<u8> = b"\x1b_payload\x1b\\".to_vec();
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("apc.cap");
    std::fs::write(&path, bytes).expect("write cap");
    let tuples = extract_capture_tuples(&path).expect("extract");
    tuples
        .into_iter()
        .find(|(t, _)| t.category == Category::Apc)
        .map(|(t, _)| t)
        .expect("capture must yield one APC tuple")
}

/// Regression: 3 APC tuple producers (catalog, dispatch, capture)
/// MUST yield identical `TupleSig` and identical `params == "Pt"`.
/// Runtime is filtered — see `apc_runtime_returns_none_for_apc_end`.
#[test]
fn apc_tuple_sig_aligns_across_three_producers() {
    let mod_path = workspace_root().join("crates/vte/src/ansi/dispatch/mod.rs");
    let dispatch_source_present = mod_path.exists();

    let mut producer_cells_exercised = 0_usize;

    let t1 = catalog_apc_tuple();
    let t3 = capture_apc_tuple();
    producer_cells_exercised += 2;

    // Signature alignment: `(APC, [], ST)`.
    assert_eq!(
        t1.signature(),
        t3.signature(),
        "APC: catalog and capture signatures must match"
    );

    // Per-producer params contract: all 3 emit "Pt".
    assert_eq!(t1.params, "Pt", "APC: catalog params must be \"Pt\"");
    assert_eq!(t3.params, "Pt", "APC: capture params must be \"Pt\"");
    assert_eq!(
        t1.final_byte, "ST",
        "APC: catalog final_byte must be \"ST\""
    );
    assert_eq!(
        t3.final_byte, "ST",
        "APC: capture final_byte must be \"ST\""
    );

    if dispatch_source_present {
        let t2 =
            dispatch_apc_tuple().expect("APC: dispatch_extract must yield a generic APC tuple");
        assert_eq!(t2.params, "Pt", "APC: dispatch params must be \"Pt\"");
        assert_eq!(
            t1.signature(),
            t2.signature(),
            "APC: catalog and dispatch signatures must match"
        );
        producer_cells_exercised += 1;
    }

    let expected_cells = if dispatch_source_present { 3 } else { 2 };
    let producer_count = if dispatch_source_present { 3 } else { 2 };
    assert_eq!(
        producer_cells_exercised, expected_cells,
        "APC matrix completeness — expected {expected_cells} producer cells \
         ({producer_count} producers), got {producer_cells_exercised}"
    );
}

/// Regression: APC has only 3 tuple producers because runtime
/// observer filters `PerformAction::ApcEnd` — the `vte` crate's
/// `apc_end()` callback does not surface the accumulated APC payload,
/// so `perform_action_to_tuple` returns `None` by design (see
/// `spec_chain/uncataloged/mod.rs:152-158`). This test pins the
/// absence as an explicit machine-checkable contract: any future
/// change that maps `ApcEnd` to a tuple would cause this assertion
/// to fail and force a deliberate decision about the runtime/APC
/// contract.
#[test]
fn apc_runtime_returns_none_for_apc_end() {
    let action = PerformAction::ApcEnd;
    assert!(
        perform_action_to_tuple(&action).is_none(),
        "APC runtime contract: PerformAction::ApcEnd must filter to None"
    );
}

/// Regression: `UncatalogedDetector` MUST observe the same filter
/// — `feed_actions(&[ApcEnd])` adds nothing to the seen set.
#[test]
fn apc_uncataloged_detector_does_not_record_apc_end() {
    let mut detector = UncatalogedDetector::new();
    detector.feed_actions(&[PerformAction::ApcEnd]);
    assert!(
        detector.seen().is_empty(),
        "UncatalogedDetector must not record ApcEnd (filtered to None)"
    );
}
