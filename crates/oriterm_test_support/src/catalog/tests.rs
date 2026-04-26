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

use super::tuple::{Category, Tuple};
use super::walk_catalog_files;
use super::{
    Classification, build_dispatch_map, canonical_tuple, classify_from_map, extract_capture_tuples,
    extract_dispatch_tuples, extract_namedprivatemode_tuples,
};

use vte::ansi::PerformAction;

use crate::spec_chain::uncataloged::{UncatalogedDetector, perform_action_to_tuple};

/// Resolve the workspace root from `CARGO_MANIFEST_DIR`.
fn workspace_root() -> std::path::PathBuf {
    std::env::var("CARGO_MANIFEST_DIR")
        .ok()
        .and_then(|mpd| {
            std::path::PathBuf::from(mpd)
                .parent()?
                .parent()
                .map(std::path::Path::to_path_buf)
        })
        .unwrap_or_else(|| std::path::PathBuf::from("."))
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

// -------- Cross-producer SSOT alignment matrix (BUG-07-019) ----------------
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
/// and exercise the four producers per BUG-07-019 §2 TDD matrix.
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

/// Regression: BUG-07-019 — all four OSC tuple producers MUST yield
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

/// Regression: BUG-07-019 — distinct selectors yield distinct
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

/// Regression: BUG-07-019 — no OSC TupleSig should carry "BEL" or
/// "ST" in `final_byte` after the SSOT alignment (selector took the
/// slot). Negative pin against the broken pre-fix shape.
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

/// Regression: BUG-07-019 — after the SSOT alignment, the OSC
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
