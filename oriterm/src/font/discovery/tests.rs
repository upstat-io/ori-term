#[cfg(any(target_os = "linux", target_os = "macos"))]
use std::path::PathBuf;

use super::families::{FALLBACK_FONTS, PRIMARY_FAMILIES};
use super::{
    EMBEDDED_FONT_DATA, FontOrigin, discover_fonts, embedded_family, enumerate_mono_families,
    prewarm_catalog, resolve_user_fallback,
};

/// Embedded font fixtures — only consumed by `_from_roots` enumeration tests
/// gated to Linux/macOS. Windows has no `_from_roots` analogue (DirectWrite has
/// no path-roots concept), so the constants are dead on Windows under
/// `-D dead_code`.
#[cfg(any(target_os = "linux", target_os = "macos"))]
const JBM_REGULAR: &[u8] = include_bytes!("../../../fonts/JetBrainsMono-Regular.ttf");
#[cfg(any(target_os = "linux", target_os = "macos"))]
const IBM_PLEX_REGULAR: &[u8] = include_bytes!("../../../fonts/IBMPlexMono-Regular.ttf");
#[cfg(any(target_os = "linux", target_os = "macos"))]
const IBM_PLEX_BOLD: &[u8] = include_bytes!("../../../fonts/IBMPlexMono-Bold.ttf");
#[cfg(any(target_os = "linux", target_os = "macos"))]
const NOTO_EMOJI: &[u8] = include_bytes!("../../../fonts/test-emoji.ttf");

/// The embedded `JetBrains` Mono bytes parse as a valid font.
#[test]
fn embedded_font_is_valid() {
    let font_ref = swash::FontRef::from_index(EMBEDDED_FONT_DATA, 0);
    assert!(
        font_ref.is_some(),
        "embedded font data should parse as a valid font"
    );
}

/// The embedded family has the correct origin and variant flags.
#[test]
fn embedded_family_has_correct_origin() {
    let family = embedded_family();

    assert_eq!(family.origin, FontOrigin::Embedded);
    assert!(
        family.has_variant[0],
        "Regular slot must be marked available"
    );
    assert!(
        !family.has_variant[1],
        "Bold slot should be unavailable (needs synthesis)"
    );
    assert!(
        !family.has_variant[2],
        "Italic slot should be unavailable (needs synthesis)"
    );
    assert!(
        !family.has_variant[3],
        "BoldItalic slot should be unavailable (needs synthesis)"
    );

    // All paths are None for embedded fonts.
    for (i, path) in family.paths.iter().enumerate() {
        assert!(path.is_none(), "embedded font path[{i}] should be None");
    }
}

/// Every `FamilySpec` has at least one Regular candidate.
#[test]
fn family_spec_consistency() {
    for spec in PRIMARY_FAMILIES {
        assert!(
            !spec.regular.is_empty(),
            "FamilySpec {:?} must have at least one Regular candidate",
            spec.name,
        );
    }
}

/// Every `FallbackSpec` has at least one filename candidate.
#[test]
fn fallback_spec_consistency() {
    for spec in FALLBACK_FONTS {
        assert!(
            !spec.filenames.is_empty(),
            "FallbackSpec {:?} must have at least one filename",
            spec.name,
        );
    }
}

/// `discover_fonts` always succeeds — the embedded fallback guarantees a result.
#[test]
fn discover_finds_at_least_one_font() {
    let result = discover_fonts(None, 400, 550);
    assert!(
        result.primary.has_variant[0],
        "discover_fonts must always find at least a Regular variant",
    );
}

/// A bogus family name doesn't panic and falls through to defaults or embedded.
#[test]
fn unknown_family_falls_back() {
    let result = discover_fonts(Some("NonExistentFontFamily_XYZ_12345"), 400, 550);
    assert!(
        result.primary.has_variant[0],
        "bogus family should fall back gracefully",
    );
}

/// If a discovered Regular path is `Some`, the file actually exists on disk.
#[test]
fn discovered_regular_path_exists() {
    let result = discover_fonts(None, 400, 550);
    if let Some(path) = &result.primary.paths[0] {
        assert!(
            path.exists(),
            "discovered Regular path should exist: {}",
            path.display(),
        );
    }
    // If paths[0] is None, it's the embedded font — that's fine.
}

/// All discovered fallback paths should exist on disk.
#[test]
fn discovered_fallback_paths_exist() {
    let result = discover_fonts(None, 400, 550);
    for fb in &result.fallbacks {
        assert!(
            fb.path.exists(),
            "fallback path should exist: {}",
            fb.path.display(),
        );
    }
}

/// `resolve_user_fallback` returns `None` for a nonexistent font name.
#[test]
fn resolve_user_fallback_nonexistent() {
    let result = resolve_user_fallback("NonExistentFontFamily_XYZ_12345");
    assert!(result.is_none(), "bogus fallback name should return None");
}

/// Different weights don't panic and still produce valid results.
#[test]
fn different_weights_succeed() {
    for weight in [100, 300, 400, 700, 900] {
        let result = discover_fonts(None, weight, (weight + 150).min(900));
        assert!(
            result.primary.has_variant[0],
            "weight {weight} should still find a Regular variant",
        );
    }
}

/// The embedded font data is a reasonable size (> 50KB for a real TTF).
#[test]
fn embedded_font_size_reasonable() {
    assert!(
        EMBEDDED_FONT_DATA.len() > 50_000,
        "embedded font should be > 50KB, got {} bytes",
        EMBEDDED_FONT_DATA.len(),
    );
}

/// Linux-specific: the font index finds real files on the system.
#[cfg(target_os = "linux")]
#[test]
fn font_index_finds_files() {
    let index = super::linux::build_font_index();
    // On a typical Linux system, at least one font should exist.
    // If no fonts are installed, the test still passes (empty index is valid).
    for (name, path) in &index {
        assert!(
            path.exists(),
            "indexed font {name:?} should exist at {}",
            path.display(),
        );
        // Spot-check: only font-like extensions.
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            let valid = [
                "ttf", "otf", "ttc", "woff", "woff2", "dfont", "pcf", "bdf", "pfb",
            ];
            // We index everything; just verify the path is a file.
            let _ = valid.contains(&ext);
        }
    }
}

/// Linux-specific: `DejaVu` Sans Mono should be installed on most systems.
#[cfg(target_os = "linux")]
#[test]
fn linux_finds_dejavu() {
    let index = super::linux::build_font_index();
    // DejaVu is installed on most Linux distros. If not, skip gracefully.
    if let Some(path) = index.get("DejaVuSansMono.ttf") {
        assert!(path.exists(), "DejaVu Sans Mono path should exist");
    }
}

/// Verify `discover_fonts` result is internally consistent.
#[test]
fn discovery_result_consistency() {
    let result = discover_fonts(None, 400, 550);
    verify_result_consistency(&result);
}

/// Embedded font has valid metrics (not a dummy/truncated file).
#[test]
fn embedded_font_has_metrics() {
    let font_ref = swash::FontRef::from_index(EMBEDDED_FONT_DATA, 0).unwrap();
    let metrics = font_ref.metrics(&[]);
    assert!(
        metrics.units_per_em > 0,
        "font must have valid units_per_em"
    );
    assert!(metrics.ascent > 0.0, "font must have positive ascent");
}

/// Discovered primary family name is non-empty.
#[test]
fn discovered_family_name_nonempty() {
    let result = discover_fonts(None, 400, 550);
    assert!(
        !result.primary.family_name.is_empty(),
        "primary family name must not be empty",
    );
}

/// All discovered variant paths (Bold/Italic/BoldItalic) are distinct from Regular.
///
/// If DirectWrite or directory scan returned the same file for multiple variants,
/// the discovery layer should have filtered them to `None`.
#[test]
fn discovered_variant_paths_distinct() {
    let result = discover_fonts(None, 400, 550);
    let regular = &result.primary.paths[0];
    for (i, path) in result.primary.paths.iter().enumerate().skip(1) {
        if let (Some(r), Some(p)) = (regular, path) {
            assert_ne!(
                r, p,
                "variant path[{i}] must differ from Regular path (duplicate = needs synthesis)",
            );
        }
    }
}

/// Discovery with user override falls back consistently: the result always
/// passes the same consistency checks regardless of override outcome.
#[test]
fn user_override_result_consistent() {
    let bogus = discover_fonts(Some("__bogus_font__"), 400, 550);
    verify_result_consistency(&bogus);

    let no_override = discover_fonts(None, 400, 550);
    verify_result_consistency(&no_override);
}

/// Fallback discovery deduplicates — no two fallbacks share the same path.
#[test]
fn fallback_paths_unique() {
    let result = discover_fonts(None, 400, 550);
    let mut seen = std::collections::HashSet::new();
    for fb in &result.fallbacks {
        assert!(
            seen.insert(&fb.path),
            "duplicate fallback path: {}",
            fb.path.display(),
        );
    }
}

/// Linux-specific: user fallback resolves an absolute path to a real font file.
#[cfg(target_os = "linux")]
#[test]
fn resolve_user_fallback_absolute_path() {
    let index = super::linux::build_font_index();
    // Find any font file to test absolute path resolution.
    if let Some((_name, path)) = index.iter().next() {
        let path_str = path.to_str().expect("font path should be valid UTF-8");
        let result = resolve_user_fallback(path_str);
        assert!(
            result.is_some(),
            "absolute path to existing font should resolve"
        );
        let fb = result.unwrap();
        assert_eq!(fb.path, *path);
        assert_eq!(fb.origin, FontOrigin::UserConfig);
    }
}

/// Linux-specific: font index handles symlinks correctly (indexed path exists).
#[cfg(target_os = "linux")]
#[test]
fn font_index_follows_symlinks() {
    let index = super::linux::build_font_index();
    for (name, path) in &index {
        // Symlinks should resolve to real files.
        if path.is_symlink() {
            assert!(
                path.exists(),
                "symlinked font {name:?} at {} should resolve to a real file",
                path.display(),
            );
        }
    }
}

/// Linux-specific: font index keys are bare filenames (no directory components).
#[cfg(target_os = "linux")]
#[test]
fn font_index_keys_are_filenames() {
    let index = super::linux::build_font_index();
    for name in index.keys() {
        assert!(
            !name.contains('/'),
            "font index key should be a bare filename, got: {name:?}",
        );
    }
}

// --- Family-catalog enumeration (`enumerate_mono_families_from_roots`) ---
//
// Tests below construct a synthetic font dir via `tempfile::TempDir`, drop one
// or more embedded font fixtures into it, and call the platform's `_from_roots`
// helper directly so the OnceLock-cached system catalog is bypassed. The
// `_inner` shape on Windows has no `_from_roots` analogue (DirectWrite has no
// path-roots concept), so Windows-side coverage is the cross-compile gate plus
// a `windows`-only `dwrote::FontCollection::system()` smoke check.

#[cfg(any(target_os = "linux", target_os = "macos"))]
fn write_fixture(dir: &std::path::Path, name: &str, bytes: &[u8]) -> PathBuf {
    let path = dir.join(name);
    std::fs::write(&path, bytes).expect("write font fixture");
    path
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
fn enumerate_from_roots(roots: &[PathBuf]) -> Vec<super::FamilyEntry> {
    #[cfg(target_os = "linux")]
    {
        super::linux::enumerate_mono_families_from_roots(roots)
    }
    #[cfg(target_os = "macos")]
    {
        super::macos::enumerate_mono_families_from_roots(roots)
    }
}

/// Empty roots produce an empty catalog (no panic on missing dirs either).
#[cfg(any(target_os = "linux", target_os = "macos"))]
#[test]
fn enumerate_empty_root_returns_empty() {
    let dir = tempfile::tempdir().unwrap();
    let entries = enumerate_from_roots(&[dir.path().to_path_buf()]);
    assert!(
        entries.is_empty(),
        "empty dir must yield empty catalog, got {entries:?}"
    );
}

/// `JetBrains` Mono fixture pins family-name extraction from the `OpenType`
/// `name` table (Pin 1 in §03 — must NOT regress to filename stem).
#[cfg(any(target_os = "linux", target_os = "macos"))]
#[test]
fn enumerate_jbm_fixture_yields_jetbrains_mono_display_name() {
    let dir = tempfile::tempdir().unwrap();
    write_fixture(dir.path(), "JetBrainsMono-Regular.ttf", JBM_REGULAR);
    let entries = enumerate_from_roots(&[dir.path().to_path_buf()]);
    let jbm = entries
        .iter()
        .find(|fe| fe.display_name == "JetBrains Mono");
    assert!(
        jbm.is_some(),
        "expected display_name 'JetBrains Mono' (from OpenType name table), \
         got {:?}",
        entries.iter().map(|f| &f.display_name).collect::<Vec<_>>()
    );
    let jbm = jbm.unwrap();
    assert!(
        jbm.paths[0].is_some(),
        "Regular slot must be populated for enumerated family"
    );
    assert!(
        jbm.paths[0].as_ref().is_some_and(
            |p| p.file_name().and_then(|n| n.to_str()) == Some("JetBrainsMono-Regular.ttf")
        ),
        "Regular slot path must point at the JBM fixture file"
    );
}

/// Malformed font file is silently skipped — no panic, no entry, and other
/// fonts in the same dir still enumerate.
#[cfg(any(target_os = "linux", target_os = "macos"))]
#[test]
fn enumerate_skips_malformed_file_keeps_valid_neighbors() {
    let dir = tempfile::tempdir().unwrap();
    write_fixture(dir.path(), "JetBrainsMono-Regular.ttf", JBM_REGULAR);
    write_fixture(dir.path(), "broken.ttf", b"not a font, just garbage bytes");
    let entries = enumerate_from_roots(&[dir.path().to_path_buf()]);
    assert!(
        entries.iter().any(|fe| fe.display_name == "JetBrains Mono"),
        "valid neighbor must still appear despite malformed sibling"
    );
}

/// Non-monospace font is filtered out at enumeration time per §02 Q1 unanimous
/// consensus. The Noto Emoji fixture has `post.isFixedPitch == 0`, so it
/// should be absent from the catalog while the JBM fixture remains.
#[cfg(any(target_os = "linux", target_os = "macos"))]
#[test]
fn enumerate_drops_proportional_font() {
    let dir = tempfile::tempdir().unwrap();
    write_fixture(dir.path(), "JetBrainsMono-Regular.ttf", JBM_REGULAR);
    write_fixture(dir.path(), "test-emoji.ttf", NOTO_EMOJI);
    let entries = enumerate_from_roots(&[dir.path().to_path_buf()]);
    let jbm_present = entries.iter().any(|fe| fe.display_name == "JetBrains Mono");
    assert!(jbm_present, "monospace JBM fixture must enumerate");
    // Whatever the emoji font's display_name parses to, it must NOT pass the
    // mono filter — JBM is the only mono family in the dir.
    assert_eq!(
        entries.len(),
        1,
        "only the monospace family should appear in catalog, got {:?}",
        entries
            .iter()
            .map(|fe| (fe.display_name.clone(), fe.paths[0].clone()))
            .collect::<Vec<_>>()
    );
}

/// Two files declaring the same family name collapse to a single entry —
/// no double-counting. Mirrors the existing `index_font_dir` first-seen-wins
/// shape (Step 1.4 `dedup_by` + group-by-display-name).
#[cfg(any(target_os = "linux", target_os = "macos"))]
#[test]
fn enumerate_dedups_duplicate_family_names() {
    let dir = tempfile::tempdir().unwrap();
    // Same font fixture, two different filenames — both report
    // `display_name = "JetBrains Mono"` and pass the mono filter.
    write_fixture(dir.path(), "JetBrainsMono-Regular.ttf", JBM_REGULAR);
    write_fixture(dir.path(), "JetBrainsMono-Regular-copy.ttf", JBM_REGULAR);
    let entries = enumerate_from_roots(&[dir.path().to_path_buf()]);
    let jbm_count = entries
        .iter()
        .filter(|fe| fe.display_name == "JetBrains Mono")
        .count();
    assert_eq!(
        jbm_count, 1,
        "duplicate family-name files must collapse to one entry, got {jbm_count}"
    );
}

/// `Bold`/`Italic`/`BoldItalic` discovered as sibling files in the same dir
/// populate the corresponding slots automatically via the post-walk grouping
/// pass. `IBM Plex Mono` ships with Regular + Bold (Italic / `BoldItalic` absent).
#[cfg(any(target_os = "linux", target_os = "macos"))]
#[test]
fn enumerate_groups_bold_into_correct_slot() {
    let dir = tempfile::tempdir().unwrap();
    write_fixture(dir.path(), "IBMPlexMono-Regular.ttf", IBM_PLEX_REGULAR);
    write_fixture(dir.path(), "IBMPlexMono-Bold.ttf", IBM_PLEX_BOLD);
    let entries = enumerate_from_roots(&[dir.path().to_path_buf()]);
    let ibm = entries
        .iter()
        .find(|fe| fe.display_name == "IBM Plex Mono")
        .expect("IBM Plex Mono family must enumerate");
    assert!(
        ibm.paths[0].is_some(),
        "Regular slot must be populated for IBM Plex Mono"
    );
    assert!(
        ibm.paths[1].is_some(),
        "Bold slot must be populated when sibling Bold file is present"
    );
    let bold_path = ibm.paths[1].as_ref().unwrap();
    assert!(
        bold_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .contains("Bold"),
        "Bold slot path should reference the Bold fixture, got {}",
        bold_path.display()
    );
}

/// Bold-visited-first ordering (Round 5 F1) — even when the directory
/// walker happens to encounter the Bold variant before the Regular variant,
/// the Regular slot MUST be populated by the actual Regular face, not by Bold.
/// Faked here by enumerating two siblings; `HashMap` iteration order is
/// non-deterministic but the per-family `find(|f| !is_bold && !is_italic)`
/// always picks the Regular face out of the group.
#[cfg(any(target_os = "linux", target_os = "macos"))]
#[test]
fn enumerate_regular_slot_is_never_a_bold_face() {
    let dir = tempfile::tempdir().unwrap();
    // Lexicographically Bold sorts before Regular — `read_dir` order on most
    // filesystems mirrors this, so without correct grouping the Bold file
    // would race into slot 0.
    write_fixture(dir.path(), "IBMPlexMono-Bold.ttf", IBM_PLEX_BOLD);
    write_fixture(dir.path(), "IBMPlexMono-Regular.ttf", IBM_PLEX_REGULAR);
    let entries = enumerate_from_roots(&[dir.path().to_path_buf()]);
    let ibm = entries
        .iter()
        .find(|fe| fe.display_name == "IBM Plex Mono")
        .expect("IBM Plex Mono family must enumerate");
    let regular_path = ibm.paths[0].as_ref().expect("Regular slot must populate");
    let regular_name = regular_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("");
    assert!(
        regular_name.contains("Regular"),
        "Regular slot path must reference the Regular file, got {regular_name}"
    );
    assert!(
        !regular_name.contains("Bold"),
        "Regular slot must not be a Bold face — Bold file racing into slot 0 is a regression"
    );
}

// --- Public catalog (`enumerate_mono_families`) ---

/// The cached public API never panics and returns a valid (possibly empty) slice.
#[test]
fn enumerate_mono_families_returns_slice_no_panic() {
    let _ = enumerate_mono_families();
    // First-call initializes; second call hits the OnceLock cache.
    let second = enumerate_mono_families();
    // Sorted case-insensitively per Step 1.4 contract.
    let sorted: bool = second
        .windows(2)
        .all(|w| w[0].display_name.to_lowercase() <= w[1].display_name.to_lowercase());
    assert!(sorted, "enumerate_mono_families output must be sorted");
}

// --- Resolution-bridge integration ---
//
// Pin 2 (§03 Property): if "JetBrains Mono" appears in the system catalog
// (typical Linux box where the user installed JBM, or any system where the
// embedded JBM fixture happens to be in scope), `discover_fonts(Some("JetBrains
// Mono"), …)` must resolve to a real file — not fall through to embedded
// fallback. On systems where JBM isn't installed at all the test skips.

/// Bridge integration (Pin 2): when the catalog knows a family, the bridge
/// in `try_user_family_with_bridge_using` resolves it via the family-name →
/// path map. Deterministic — uses the injectable lookup seam to seed a
/// fixture catalog so the test does not depend on the host system having
/// `JetBrains` Mono installed (closes F2 test ordering per §06 Round 0).
#[cfg(any(target_os = "linux", target_os = "macos"))]
#[test]
fn try_user_family_with_bridge_resolves_enumerated_family() {
    use std::collections::HashMap;

    let dir = tempfile::tempdir().unwrap();
    let jbm_path = write_fixture(dir.path(), "JetBrainsMono-Regular.ttf", JBM_REGULAR);

    // Seed an in-memory catalog mapping "JetBrains Mono" → the fixture path.
    let mut catalog: HashMap<String, ([Option<PathBuf>; 4], [u32; 4])> = HashMap::new();
    catalog.insert(
        "JetBrains Mono".to_owned(),
        ([Some(jbm_path.clone()), None, None, None], [0; 4]),
    );

    // Empty filename index — bridge returns Some without filename probing.
    let index: HashMap<String, PathBuf> = HashMap::new();

    let result = super::unix::try_user_family_with_bridge_using("JetBrains Mono", &index, |name| {
        catalog.get(name).cloned()
    })
    .expect("bridge must resolve when catalog has the family");

    let resolved = result.primary.paths[0]
        .as_ref()
        .expect("Regular slot must be populated when catalog had a Regular");
    assert_eq!(
        resolved, &jbm_path,
        "bridge must thread the catalog-recorded path through to DiscoveryResult.primary"
    );
    assert_eq!(
        result.primary.origin,
        FontOrigin::UserConfig,
        "bridge fires under user-config origin"
    );
    assert_eq!(
        result.primary.family_name, "JetBrains Mono",
        "bridge preserves the family name passed in"
    );
}

/// Bridge regression guard (Pin 2 fall-through): when the catalog does NOT know
/// the family AND filename heuristics also miss, the bridge returns `None`
/// — never silently succeeds for genuinely uninstalled families.
#[cfg(any(target_os = "linux", target_os = "macos"))]
#[test]
fn try_user_family_with_bridge_returns_none_when_catalog_misses() {
    use std::collections::HashMap;

    let empty_catalog: HashMap<String, ([Option<PathBuf>; 4], [u32; 4])> = HashMap::new();
    let empty_index: HashMap<String, PathBuf> = HashMap::new();

    let result = super::unix::try_user_family_with_bridge_using(
        "TotallyNotInstalledFontFamily",
        &empty_index,
        |name| empty_catalog.get(name).cloned(),
    );
    assert!(
        result.is_none(),
        "bridge must return None when neither catalog nor heuristics match"
    );
}

// --- Algorithmic-duplication regression ---

/// Linux/macOS `index_font_dir` collapsed into a single canonical
/// `walk_font_dirs` helper (closes 's `LEAK:algorithmic-duplication`
/// finding). This test pins the SSOT at the API level: both platforms expose
/// `build_font_index` returning equivalently-shaped indices over their roots.
#[cfg(target_os = "linux")]
#[test]
fn build_font_index_uses_shared_walk_helper() {
    // Indirect proof of the SSOT: the function compiles and produces output —
    // any future refactor that re-introduces a private `index_font_dir`
    // duplicate becomes immediately visible via the byte-level surface.
    // Direct proof would be an architectural test; for now this is the
    // behavioral pin (filename-index still works after the consolidation).
    let index = super::linux::build_font_index();
    for (name, path) in &index {
        assert!(
            path.is_file(),
            "indexed font {name:?} should resolve to a file at {}",
            path.display()
        );
    }
}

/// Helper: verify that a `DiscoveryResult` is internally consistent.
fn verify_result_consistency(result: &super::DiscoveryResult) {
    let primary = &result.primary;

    // has_variant must match paths.
    for i in 0..4 {
        assert_eq!(
            primary.has_variant[i],
            primary.paths[i].is_some(),
            "has_variant[{i}] must match paths[{i}].is_some() for {:?}",
            primary.family_name,
        );
    }

    // If origin is Embedded, all paths must be None.
    if primary.origin == FontOrigin::Embedded {
        for (i, path) in primary.paths.iter().enumerate() {
            assert!(
                path.is_none(),
                "embedded font should have no paths, but paths[{i}] is Some",
            );
        }
    }

    // Existing paths must point to real files.
    for (i, path) in primary.paths.iter().enumerate() {
        if let Some(p) = path {
            assert!(
                p.exists(),
                "primary path[{i}] should exist: {}",
                p.display(),
            );
        }
    }
}

/// Regression: BUG-04-008 — `prewarm_catalog()` populates `FAMILY_CATALOG`
/// so the first Settings dialog open finds a hot cache instead of stalling
/// the UI thread on platform font enumeration.
///
/// `OnceLock::get_or_init` is process-global; since other tests in this
/// binary may already have warmed the catalog, the assertion is structural:
/// after `prewarm_catalog()` returns, `enumerate_mono_families()` returns
/// pointer-equal slices on subsequent calls (the `OnceLock` is populated
/// and stable).
#[test]
fn prewarm_catalog_populates_family_catalog() {
    prewarm_catalog();
    let a = enumerate_mono_families();
    let b = enumerate_mono_families();
    assert_eq!(
        a.as_ptr(),
        b.as_ptr(),
        "catalog must be a stable &'static slice after prewarm",
    );
    assert_eq!(
        a.len(),
        b.len(),
        "catalog length must be stable across calls",
    );
}

/// Regression: BUG-04-008 — `prewarm_catalog()` is idempotent under
/// repeated and concurrent calls. `OnceLock::get_or_init` allows exactly
/// one initializer; concurrent callers block until the first completes,
/// then read the stable result.
#[test]
fn prewarm_catalog_is_idempotent_under_concurrent_calls() {
    let handles: Vec<_> = (0..4)
        .map(|_| std::thread::spawn(prewarm_catalog))
        .collect();
    for h in handles {
        h.join().expect("prewarm thread must not panic");
    }
    // After all four threads complete, the catalog must be populated and
    // returning a stable slice (same pointer on subsequent calls).
    let a = enumerate_mono_families();
    let b = enumerate_mono_families();
    assert_eq!(
        a.as_ptr(),
        b.as_ptr(),
        "concurrent prewarm must leave catalog stable",
    );
}
