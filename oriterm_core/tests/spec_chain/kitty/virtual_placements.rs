//! Per-placeholder rung (§13.4) — drives kitty `U=1` virtual placement and
//! `U+10EEEE` cell encoding through transmit → store → snapshot → render.
//!
//! Coverage matrix per the §13.4 plan body: encoding, snapshot, emit,
//! reflow, scroll, ED, EL, alt-screen.

use oriterm_core::image::ImageId;
use oriterm_test_support::spec_chain::SpecHarness;

use super::fixtures::{b64, kitty_apc, ok_reply_for, reply_bytes, reply_contains, rgba_4x4_red};

/// Drive a `U=1,a=t` transmit. Returns the harness after the command.
fn transmit_u1(h: &mut SpecHarness, id: u32) {
    let control = format!("a=t,U=1,i={id},f=32,s=4,v=4");
    h.feed(&kitty_apc(control.as_bytes(), &b64(&rgba_4x4_red())));
}

/// Drive a `U=1,a=p` place for an already-stored image.
fn place_u1(h: &mut SpecHarness, id: u32) {
    let control = format!("a=p,U=1,i={id},c=2,r=2");
    h.feed(&kitty_apc(control.as_bytes(), ""));
}

/// Write a placeholder cell at the current cursor position carrying the
/// supplied diacritic-encoded row/col + fg = `image_id_low`.
fn write_placeholder_cell(h: &mut SpecHarness, image_id_low: u32, row: char, col: char) {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(format!("\x1b[38;5;{image_id_low}m").as_bytes());
    let mut buf = [0u8; 4];
    bytes.extend_from_slice('\u{10EEEE}'.encode_utf8(&mut buf).as_bytes());
    bytes.extend_from_slice(row.encode_utf8(&mut buf).as_bytes());
    bytes.extend_from_slice(col.encode_utf8(&mut buf).as_bytes());
    bytes.extend_from_slice(b"\x1b[39m");
    h.feed(&bytes);
}

/// Catalog row: `KG-UNICODE-PLACEHOLDER-TRANSMIT-U1`.
#[test]
fn kitty_u1_transmit_records_placeholder_anchor() {
    let mut h = SpecHarness::new();
    transmit_u1(&mut h, 1);

    let anchors = h.term().image_cache().placeholder_anchors();
    assert!(
        anchors.contains(&ImageId::from_raw(1)),
        "a=t,U=1 MUST add an anchor for image_id=1 — got {anchors:?}",
    );
    assert!(
        reply_contains(&h, &ok_reply_for(1)),
        "a=t,U=1 emits OK — transcript: {:?}",
        String::from_utf8_lossy(&reply_bytes(&h)),
    );
}

/// Catalog row: `KG-UNICODE-PLACEHOLDER-PLACE-U1`.
#[test]
fn kitty_u1_place_records_placeholder_anchor() {
    let mut h = SpecHarness::new();
    // First store via `a=t` without U=1, then `a=p,U=1` to record the anchor.
    h.feed(&kitty_apc(b"a=t,i=2,f=32,s=4,v=4", &b64(&rgba_4x4_red())));
    place_u1(&mut h, 2);

    let anchors = h.term().image_cache().placeholder_anchors();
    assert!(
        anchors.contains(&ImageId::from_raw(2)),
        "a=p,U=1 MUST add an anchor for image_id=2 — got {anchors:?}",
    );
}

#[test]
fn kitty_u1_image_survives_unrelated_lru_pressure_via_anchor() {
    let mut h = SpecHarness::new();
    transmit_u1(&mut h, 100);
    assert_eq!(h.term().image_cache().image_count(), 1);

    // Tight cache cap: exactly 4×4 RGBA = 64 bytes. Anything beyond would
    // normally trigger LRU eviction; the anchor must protect i=100.
    h.term_mut().image_cache_mut().set_memory_limit(64);

    for next_id in 200..210 {
        h.feed(&kitty_apc(
            format!("a=t,i={next_id},f=32,s=4,v=4").as_bytes(),
            &b64(&rgba_4x4_red()),
        ));
    }

    assert!(
        h.term()
            .image_cache()
            .get_no_touch(ImageId::from_raw(100))
            .is_some(),
        "U=1 anchored image must survive LRU pressure — anchors={:?}, image_count={}",
        h.term().image_cache().placeholder_anchors(),
        h.term().image_cache().image_count(),
    );
}

#[test]
fn kitty_u1_anchor_does_not_intersect_viewport_placements() {
    let mut h = SpecHarness::new();
    transmit_u1(&mut h, 1);

    // U=1 must NOT synthesize a real ImagePlacement — the viewport
    // intersection query returns zero entries (per the §13.4 layer-bleed
    // contract: ImageCache.placements remains the real-placement SSOT).
    let snap = h.term().renderable_content();
    assert_eq!(
        snap.images.len(),
        0,
        "U=1 transmit MUST NOT add a RenderablePlacement — images={:?}",
        snap.images,
    );
}

/// Catalog row: `KG-UNICODE-PLACEHOLDER-CELL-RESOLVE`.
#[test]
fn placeholder_decode_with_fg_color_and_diacritics() {
    let mut h = SpecHarness::new();
    // Image id will be 42; transmit then write one placeholder cell.
    transmit_u1(&mut h, 42);
    write_placeholder_cell(&mut h, 42, '\u{0305}', '\u{0305}');

    let snap = h.term().renderable_content();
    assert_eq!(
        snap.placeholder_cells.len(),
        1,
        "MUST emit one RenderablePlaceholderCell — got {:?}",
        snap.placeholder_cells,
    );
    let p = snap.placeholder_cells[0];
    assert_eq!(p.image_id, ImageId::from_raw(42));
    assert_eq!(p.image_row, 0);
    assert_eq!(p.image_col, 0);
    assert_eq!(p.line, 0);
}

#[test]
fn placeholder_decode_continuation_row_only() {
    let mut h = SpecHarness::new();
    transmit_u1(&mut h, 7);
    // Two adjacent cells: first has row+col, second has row only → second
    // inherits col = prev.col + 1 = 1.
    write_placeholder_cell(&mut h, 7, '\u{0305}', '\u{0305}');
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"\x1b[38;5;7m");
    let mut buf = [0u8; 4];
    bytes.extend_from_slice('\u{10EEEE}'.encode_utf8(&mut buf).as_bytes());
    bytes.extend_from_slice('\u{0305}'.encode_utf8(&mut buf).as_bytes());
    bytes.extend_from_slice(b"\x1b[39m");
    h.feed(&bytes);

    let snap = h.term().renderable_content();
    assert_eq!(
        snap.placeholder_cells.len(),
        2,
        "two placeholder cells expected — got {:?}",
        snap.placeholder_cells,
    );
    assert_eq!(snap.placeholder_cells[0].image_col, 0);
    assert_eq!(snap.placeholder_cells[1].image_col, 1);
}

#[test]
fn placeholder_cell_without_image_renders_as_glyph_not_quad() {
    // Bare U+10EEEE with no fg color (image_id_low=0) MUST NOT emit a
    // RenderablePlaceholderCell, even though the glyph is in the grid.
    let mut h = SpecHarness::new();
    let mut bytes = Vec::new();
    let mut buf = [0u8; 4];
    bytes.extend_from_slice('\u{10EEEE}'.encode_utf8(&mut buf).as_bytes());
    h.feed(&bytes);

    let snap = h.term().renderable_content();
    assert_eq!(
        snap.placeholder_cells.len(),
        0,
        "bare U+10EEEE without fg color must NOT emit an image quad — got {:?}",
        snap.placeholder_cells,
    );
    // The glyph itself still occupies a cell in the snapshot.
    let cell0 = &snap.cells[0];
    assert_eq!(cell0.ch, '\u{10EEEE}');
}

/// Catalog row: `KG-UNICODE-PLACEHOLDER-REFLOW`.
#[test]
fn placeholder_cells_resolve_to_stored_image_after_reflow() {
    let mut h = SpecHarness::new();
    transmit_u1(&mut h, 33);
    write_placeholder_cell(&mut h, 33, '\u{0305}', '\u{0305}');

    let pre = h.term().renderable_content();
    assert_eq!(pre.placeholder_cells.len(), 1);
    assert_eq!(pre.placeholder_cells[0].image_id, ImageId::from_raw(33));

    // Resize narrower — drives column reflow. The placeholder cell should
    // still resolve to image 33 because the cell carries the encoding,
    // not the cache.
    let (lines, _cols) = (h.term().grid().lines(), h.term().grid().cols());
    h.term_mut().resize(lines, 40, true);

    let post = h.term().renderable_content();
    assert_eq!(
        post.placeholder_cells.len(),
        1,
        "reflow must preserve the placeholder cell — got {:?}",
        post.placeholder_cells,
    );
    assert_eq!(post.placeholder_cells[0].image_id, ImageId::from_raw(33));
}

#[test]
fn placeholder_anchored_cell_does_not_also_emit_text_glyph_quad() {
    // Double-exposure clamp: a cell carrying a valid U+10EEEE + diacritic
    // anchor pattern must produce ONLY the image quad. The corresponding
    // `RenderableCell` entry MUST NOT carry `U+10EEEE` as its glyph — if
    // it did, the GPU would draw the placeholder codepoint as a fallback
    // glyph ON TOP of the image quad at the same (line, column) position.
    //
    // Snapshot-side suppression substitutes a space so the cell still
    // exists for bg / flags / selection coverage; only the glyph render
    // is suppressed.
    let mut h = SpecHarness::new();
    transmit_u1(&mut h, 200);
    write_placeholder_cell(&mut h, 200, '\u{0305}', '\u{0305}');

    let snap = h.term().renderable_content();
    assert_eq!(
        snap.placeholder_cells.len(),
        1,
        "precondition: one placeholder cell pushed",
    );
    let pc = &snap.placeholder_cells[0];

    let cell_at_pos = snap
        .cells
        .iter()
        .find(|c| c.line == pc.line && c.column == pc.column)
        .expect("a RenderableCell exists at the placeholder position");
    assert_ne!(
        cell_at_pos.ch, '\u{10EEEE}',
        "placeholder-anchored cell must NOT keep U+10EEEE in the cells snapshot — that would emit a text-glyph quad on top of the image quad. Got cells entry: {cell_at_pos:?}",
    );
}

#[test]
fn placeholder_category_matrix_completeness() {
    // §13.4 mandates 8 coverage categories: encoding, snapshot, emit,
    // reflow, scroll, ED, EL, alt-screen. This pin extracts `#[test] fn`
    // declarations from sibling test files (NOT raw string occurrences
    // that could match this file's own probe table) and asserts each
    // category has at least one declared probe test.
    let category_probes: &[(&str, &[&str])] = &[
        (
            "encoding",
            &[
                "placeholder_decode_with_fg_color_and_diacritics",
                "placeholder_decode_continuation_row_only",
                "placeholder_cell_without_image_renders_as_glyph_not_quad",
            ],
        ),
        (
            "snapshot",
            &[
                "renderable_content_surfaces_placeholder_cells_alongside_image_placements",
                "placeholder_anchored_cell_does_not_also_emit_text_glyph_quad",
            ],
        ),
        // `emit` exercises the GPU emit path's snapshot side — proving
        // placeholder cells reach the image-quad pipeline (and do NOT
        // double-emit a text-glyph quad).
        (
            "emit",
            &["placeholder_anchored_cell_does_not_also_emit_text_glyph_quad"],
        ),
        (
            "reflow",
            &["placeholder_cells_resolve_to_stored_image_after_reflow"],
        ),
        (
            "scroll",
            &[
                "prune_scrollback_retains_anchor_when_placeholder_cells_remain_in_viewport",
                "prune_scrollback_clears_orphaned_placeholder_anchors_for_images_with_no_surviving_cells",
            ],
        ),
        (
            "ed",
            &["ed_full_screen_clears_orphaned_placeholder_anchors"],
        ),
        // `el` = CSI K (Erase in Line); the DECERA rect-erase test
        // covers a different sequence ($z), so we need a CSI K probe.
        (
            "el",
            &["line_erase_csi_k_with_placeholder_cell_reconciles_anchor"],
        ),
        (
            "alt-screen",
            &[
                "inactive_alt_screen_u1_anchors_reconciled_on_resize",
                "inactive_alt_screen_u1_anchors_retained_when_cells_survive_resize",
                "inactive_primary_screen_u1_anchors_reconciled_on_resize",
            ],
        ),
    ];

    let test_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/spec_chain/kitty");
    let mut declared_fn_names: std::collections::HashSet<String> = std::collections::HashSet::new();
    for entry in std::fs::read_dir(&test_dir).expect("kitty test dir") {
        let entry = entry.expect("entry");
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("rs") {
            continue;
        }
        let src = std::fs::read_to_string(&path).expect("read");
        // Two-line state: previous line was `#[test]` → current line's
        // `fn NAME(` is a test declaration. String-literal mentions of
        // names elsewhere (e.g. inside this file's `category_probes`
        // table) are ignored because they are not preceded by `#[test]`.
        let mut prev_was_test_attr = false;
        for line in src.lines() {
            let trimmed = line.trim_start();
            if prev_was_test_attr {
                if let Some(rest) = trimmed.strip_prefix("fn ") {
                    if let Some(end) = rest.find('(') {
                        declared_fn_names.insert(rest[..end].trim().to_string());
                    }
                }
                prev_was_test_attr = false;
            }
            if trimmed == "#[test]" {
                prev_was_test_attr = true;
            }
        }
    }

    let mut missing: Vec<&str> = Vec::new();
    for (category, probes) in category_probes {
        if !probes.iter().any(|name| declared_fn_names.contains(*name)) {
            missing.push(category);
        }
    }
    assert!(
        missing.is_empty(),
        "matrix completeness — categories with no `#[test] fn` probe declaration: {missing:?}. Discovered {} test functions across sibling files.",
        declared_fn_names.len(),
    );
}

/// Inventory of grid-mutation call sites in the four owning files that
/// CAN affect placeholder-bearing cells. Each entry is
/// `(file_relative_to_crate, mutation_api_substring)`. Adding a new
/// mutation site to one of these files without updating this list is
/// caught by `placeholder_anchor_mutation_sites_registered_via_inventory_grep`.
const PLACEHOLDER_MUTATION_SITES: &[(&str, &str)] = &[
    // helpers.rs — `input_char` ASCII fast path + non-ASCII branch.
    ("src/term/handler/helpers.rs", "put_char_ascii(c)"),
    ("src/term/handler/helpers.rs", "grid.put_char(c)"),
    // presentation/mod.rs — DECIC / DECDC insert/delete columns at
    // VPA-bounded positions plus VT220-style insert/delete column
    // operators that fall through `insert_columns(1, ...)` /
    // `delete_columns(1, ...)`.
    (
        "src/term/handler/presentation/mod.rs",
        "insert_columns(count, at_col)",
    ),
    (
        "src/term/handler/presentation/mod.rs",
        "delete_columns(count, at_col)",
    ),
    (
        "src/term/handler/presentation/mod.rs",
        "insert_columns(1, left_bound)",
    ),
    (
        "src/term/handler/presentation/mod.rs",
        "delete_columns(1, left_bound)",
    ),
    // rect_ops/mod.rs — DECCRA copy, DECFRA fill, DECERA erase,
    // DECSERA selective erase. DECCRA was added 2026-05-20 after the
    // inventory test surfaced the missing reconcile.
    ("src/term/handler/rect_ops/mod.rs", ".copy_rect(src.top"),
    ("src/term/handler/rect_ops/mod.rs", "grid.fill_rect("),
    ("src/term/handler/rect_ops/mod.rs", ".erase_rect_all("),
    (
        "src/term/handler/rect_ops/mod.rs",
        ".erase_rect_unprotected(",
    ),
    // resize/mod.rs — primary + alt grid resize, reconciled symmetrically
    // via the wrapper `reconcile_both_placeholder_anchors` at function end.
    (
        "src/term/resize/mod.rs",
        "self.grid.resize(new_lines, new_cols, reflow)",
    ),
    (
        "src/term/resize/mod.rs",
        "alt.resize(new_lines, new_cols, false)",
    ),
];

/// Strip a single naive Rust string literal pass — replaces `"..."`
/// regions with spaces so brace/quote counters skip them. Sufficient
/// for the rect_ops / helpers / presentation / resize sources, which
/// don't use raw strings (`r#"..."#`) or escaped-quote-bearing
/// string literals on the same line as significant braces.
fn strip_strings(line: &str) -> String {
    let mut out = String::with_capacity(line.len());
    let mut in_str = false;
    let mut prev = ' ';
    for c in line.chars() {
        if !in_str && c == '"' {
            in_str = true;
            out.push(' ');
        } else if in_str && c == '"' && prev != '\\' {
            in_str = false;
            out.push(' ');
        } else if in_str {
            out.push(' ');
        } else {
            out.push(c);
        }
        prev = c;
    }
    out
}

/// Walk back through `lines` to find the most recent function
/// declaration line. Returns the index of that line. Panics if no
/// `fn ` declaration is found above — callers MUST ensure the
/// mutation site is inside a function.
fn find_enclosing_fn_decl(lines: &[&str]) -> usize {
    for (i, line) in lines.iter().enumerate().rev() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("fn ")
            || trimmed.starts_with("pub fn ")
            || trimmed.starts_with("pub(crate) fn ")
            || trimmed.starts_with("pub(super) fn ")
            || trimmed.starts_with("pub(in ")
        {
            return i;
        }
    }
    panic!(
        "no enclosing fn declaration found above mutation line at end of slice ({} lines)",
        lines.len()
    );
}

fn count_open_braces(line: &str) -> usize {
    strip_strings(line).chars().filter(|&c| c == '{').count()
}

fn count_close_braces(line: &str) -> usize {
    strip_strings(line).chars().filter(|&c| c == '}').count()
}

/// Cumulative brace depth at the end of `lines` (i.e., after the LAST
/// line is fully consumed). Comment lines are skipped to avoid
/// confusing comment-only braces with code-bearing ones.
fn brace_depth_at(lines: &[&str]) -> usize {
    let mut depth: isize = 0;
    for line in lines {
        if line.trim().starts_with("//") {
            continue;
        }
        let stripped = strip_strings(line);
        for c in stripped.chars() {
            match c {
                '{' => depth += 1,
                '}' => depth -= 1,
                _ => {}
            }
        }
    }
    depth.max(0) as usize
}

/// Helper: scan a source file for grid-mutation API calls that
/// COULD touch placeholder-bearing cells. Returns `(line_number,
/// trimmed_line)` for each match. Used by both inventory tests.
fn scan_mutation_lines(path: &std::path::Path) -> Vec<(usize, String)> {
    let src =
        std::fs::read_to_string(path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    // Surface patterns: grid-mutation API calls that overwrite or
    // replace cells. `push_zerowidth` is excluded — it APPENDS a
    // combining mark to an existing cell rather than overwriting.
    // `cursor_mut`, `set_col`, `move_*` adjust cursor state without
    // touching cells, so they are excluded too.
    // Patterns must be PREFIXED by a non-identifier character so
    // `put_char(` does not match the substring inside `input_char(`,
    // `cursor_mut().put_char(...)` does match but `fn input_char` does
    // not.
    const PATTERNS: &[&str] = &[
        ".put_char_ascii(",
        ".put_char(",
        ".insert_columns(",
        ".delete_columns(",
        ".copy_rect(",
        ".fill_rect(",
        ".erase_rect_all(",
        ".erase_rect_unprotected(",
        ".resize(",
    ];
    let mut hits: Vec<(usize, String)> = Vec::new();
    for (idx, line) in src.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.starts_with("//") {
            continue;
        }
        // Skip pure method-definition lines (`fn foo(` / `pub fn foo(`)
        // — the test cares about call sites, not declarations.
        if trimmed.starts_with("fn ")
            || trimmed.starts_with("pub fn ")
            || trimmed.starts_with("pub(crate) fn ")
            || trimmed.starts_with("pub(super) fn ")
            || trimmed.starts_with("pub(in ")
        {
            continue;
        }
        for pat in PATTERNS {
            if line.contains(pat) {
                hits.push((idx + 1, line.to_string()));
                break;
            }
        }
    }
    hits
}

/// Each `(file, api)` pair in `PLACEHOLDER_MUTATION_SITES` MUST resolve
/// to at least one mutation-line in the corresponding source file.
/// Conversely: every mutation-line found in the four owning files MUST
/// be covered by at least one inventory entry. Adding a new mutation
/// site without registering it here is the silent-drift regression
/// this test is designed to catch.
#[test]
fn placeholder_anchor_mutation_sites_registered_via_inventory_grep() {
    let crate_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut files: std::collections::BTreeSet<&str> = std::collections::BTreeSet::new();
    for (file, _) in PLACEHOLDER_MUTATION_SITES {
        files.insert(*file);
    }

    // Coverage direction 1: every inventory entry resolves to a real
    // mutation line.
    let mut unresolved_inventory: Vec<String> = Vec::new();
    for (file, needle) in PLACEHOLDER_MUTATION_SITES {
        let path = crate_root.join(file);
        let src = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("read inventory file {file}: {e}"));
        if !src
            .lines()
            .any(|l| !l.trim().starts_with("//") && l.contains(needle))
        {
            unresolved_inventory.push(format!("{file}: needle {needle:?} not found"));
        }
    }
    assert!(
        unresolved_inventory.is_empty(),
        "inventory entries with no matching mutation line: {unresolved_inventory:?}"
    );

    // Coverage direction 2: every mutation line in the owning files is
    // covered by at least one inventory entry. A new mutation site must
    // be ADDED to PLACEHOLDER_MUTATION_SITES or this test fails.
    let mut uncovered_mutations: Vec<String> = Vec::new();
    for file in &files {
        let path = crate_root.join(file);
        for (lineno, line) in scan_mutation_lines(&path) {
            let covered = PLACEHOLDER_MUTATION_SITES
                .iter()
                .any(|(invfile, needle)| invfile == file && line.contains(needle));
            if !covered {
                uncovered_mutations.push(format!("{file}:{lineno} — {line}"));
            }
        }
    }
    assert!(
        uncovered_mutations.is_empty(),
        "mutation-site drift — these lines are NOT covered by PLACEHOLDER_MUTATION_SITES (add to the inventory if they can touch placeholder-bearing cells, or refine the scan PATTERNS in `scan_mutation_lines` if they cannot):\n{}",
        uncovered_mutations.join("\n"),
    );
}

/// Every mutation site in `PLACEHOLDER_MUTATION_SITES` MUST be followed
/// (within the same function body) by a call to
/// `reconcile_placeholder_anchors_from_grid()` OR
/// `reconcile_both_placeholder_anchors()`. Silent regressions that
/// drop reconcile from a mutation path are the canonical defect the
/// §13.4 cluster prevents — this pin lifts the manual-audit burden
/// into a mechanical check.
///
/// **Heuristic scope (PRESENCE-only, not REACHABILITY):** the scan
/// only verifies the reconcile-call TEXT appears in the enclosing
/// function body — it does NOT prove the reconcile is on the same
/// branch as the mutation. A future regression that gates reconcile
/// behind an unrelated `if`/`match` arm would still satisfy this pin.
/// The companion runtime pins
/// (`printable_overwrite_of_placeholder_cell_reconciles_anchor`,
/// `rectangular_copy_replacing_placeholder_cells_reconciles_anchor`,
/// etc. at `placeholder_anchors.rs`) drive the actual mutation under
/// each gate variant and assert the anchor cleared — that's the
/// reachability layer. This static scan is the FILE-LAYOUT defense
/// (catches "new mutation site added without ANY reconcile call");
/// the runtime pins are the BEHAVIOR defense. Both layers required.
#[test]
fn every_mutation_path_in_helpers_resize_presentation_rect_ops_calls_reconcile_placeholder_anchors()
{
    let crate_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));

    let mut violations: Vec<String> = Vec::new();
    for (file, needle) in PLACEHOLDER_MUTATION_SITES {
        let path = crate_root.join(file);
        let src = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {file}: {e}"));
        let lines: Vec<&str> = src.lines().collect();

        // Find every occurrence of the mutation needle, then scan
        // forward up to the next `}` at the top indent for either
        // reconcile API. A reconcile within the same function body
        // satisfies the pin. The scan stops at function boundary.
        for (idx, line) in lines.iter().enumerate() {
            if line.trim().starts_with("//") {
                continue;
            }
            if !line.contains(needle) {
                continue;
            }
            // Function-boundary heuristic: walk BACK from the mutation
            // line to find the enclosing function's declaration line,
            // capture depth THERE (= the depth INSIDE the enclosing
            // function's body, NOT the depth at the mutation site
            // which may be nested in an `if let` / `match` arm), then
            // scan forward and exit when depth drops below that
            // function-body depth. Robust to last-function-in-file
            // case because the depth-drop happens at the function's
            // closing `}` regardless of trailing module items, AND to
            // mutations nested in inner blocks because we don't
            // confuse inner-scope close with function-scope close.
            let fn_decl_idx = find_enclosing_fn_decl(&lines[..=idx]);
            // The fn-decl line ends with its body's opening `{` (true
            // for all owning files), so brace_depth_at INCLUDES that
            // brace — function-body depth equals depth-at-end-of-decl.
            let body_depth = brace_depth_at(&lines[..=fn_decl_idx]);
            let mut depth = brace_depth_at(&lines[..=idx]);
            let mut found = false;
            for follow in &lines[idx + 1..] {
                let follow_trim = follow.trim();
                if follow_trim.starts_with("//") {
                    continue;
                }
                if follow_trim.contains("reconcile_placeholder_anchors_from_grid")
                    || follow_trim.contains("reconcile_both_placeholder_anchors")
                {
                    found = true;
                    break;
                }
                depth += count_open_braces(follow);
                depth = depth.saturating_sub(count_close_braces(follow));
                if depth < body_depth {
                    // Walked out of the enclosing function body.
                    break;
                }
            }
            if !found {
                violations.push(format!(
                    "{file}:{} — {} → no reconcile_placeholder_anchors_from_grid/reconcile_both_placeholder_anchors before enclosing function body closes",
                    idx + 1,
                    line.trim(),
                ));
            }
        }
    }
    assert!(
        violations.is_empty(),
        "mutation-site → reconcile drift:\n{}",
        violations.join("\n"),
    );
}
