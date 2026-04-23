//! Golden image comparison functions.
//!
//! Two comparison paths:
//! - [`compare_with_reference`]: legacy, uses fixed `PIXEL_TOLERANCE` and
//!   `MAX_MISMATCH_PERCENT` constants. Retained for existing visual regression tests.
//! - [`compare_with_reference_strict`]: spec-conformance, reads tolerance
//!   from [`GoldenLaneConfig`]. Two-gate: per-channel max + mismatch percentage.

use image::{ImageBuffer, Rgba, RgbaImage};

use super::{GoldenLaneConfig, MAX_MISMATCH_PERCENT, PIXEL_TOLERANCE, reference_dir};

/// Compare rendered pixels against a reference PNG.
///
/// - `ORITERM_UPDATE_GOLDEN=1`: overwrites the reference and returns Ok.
/// - If reference doesn't exist: saves `pixels` as the reference and passes.
/// - If reference exists: compares with `PIXEL_TOLERANCE` and
///   `MAX_MISMATCH_PERCENT`. On failure, saves `*_actual.png` and
///   `*_diff.png` alongside the reference.
///
/// Returns `Ok(())` on match, `Err(message)` on mismatch.
pub(super) fn compare_with_reference(
    name: &str,
    pixels: &[u8],
    width: u32,
    height: u32,
) -> Result<(), String> {
    let ref_dir = reference_dir();
    let ref_path = ref_dir.join(format!("{name}.png"));
    let actual_path = ref_dir.join(format!("{name}_actual.png"));
    let diff_path = ref_dir.join(format!("{name}_diff.png"));

    let actual: RgbaImage =
        ImageBuffer::from_raw(width, height, pixels.to_vec()).expect("pixel buffer size mismatch");

    check_ci_guard();

    // Regeneration mode: overwrite reference with current output.
    if std::env::var("ORITERM_UPDATE_GOLDEN").as_deref() == Ok("1") {
        std::fs::create_dir_all(&ref_dir).expect("failed to create reference dir");
        actual
            .save(&ref_path)
            .expect("failed to save reference PNG");
        eprintln!(
            "golden updated: {} ({}×{})",
            ref_path.display(),
            width,
            height,
        );
        let _ = std::fs::remove_file(&actual_path);
        let _ = std::fs::remove_file(&diff_path);
        return Ok(());
    }

    if !ref_path.exists() {
        std::fs::create_dir_all(&ref_dir).expect("failed to create reference dir");
        actual
            .save(&ref_path)
            .expect("failed to save reference PNG");
        eprintln!(
            "reference saved: {} ({}×{}). Re-run to compare.",
            ref_path.display(),
            width,
            height,
        );
        return Ok(());
    }

    let reference = image::open(&ref_path)
        .expect("failed to open reference PNG")
        .to_rgba8();

    if reference.width() != width || reference.height() != height {
        actual
            .save(&actual_path)
            .expect("failed to save actual PNG");
        return Err(format!(
            "size mismatch: reference is {}×{}, actual is {width}×{height}. Actual saved to {}",
            reference.width(),
            reference.height(),
            actual_path.display(),
        ));
    }

    let (mismatches, diff_img) = pixel_diff(&reference, &actual, PIXEL_TOLERANCE);

    if mismatches > 0 {
        let total = (width * height) as usize;
        let pct = mismatches as f64 / total as f64 * 100.0;

        if pct > MAX_MISMATCH_PERCENT {
            actual
                .save(&actual_path)
                .expect("failed to save actual PNG");
            diff_img.save(&diff_path).expect("failed to save diff PNG");

            Err(format!(
                "{mismatches}/{total} pixels differ ({pct:.2}%, threshold {MAX_MISMATCH_PERCENT}%). \
                 tolerance=±{PIXEL_TOLERANCE}\n\
                 actual: {}\n\
                 diff:   {}\n\
                 to accept the new golden: \
                 ORITERM_UPDATE_GOLDEN=1 cargo test -p oriterm --features gpu-tests --lib \
                 (reviews the diff PNG + re-runs the test against the updated reference)",
                actual_path.display(),
                diff_path.display(),
            ))
        } else {
            let _ = std::fs::remove_file(&actual_path);
            let _ = std::fs::remove_file(&diff_path);
            Ok(())
        }
    } else {
        let _ = std::fs::remove_file(&actual_path);
        let _ = std::fs::remove_file(&diff_path);
        Ok(())
    }
}

/// Compare rendered pixels against a reference PNG with configurable
/// strict tolerance from [`GoldenLaneConfig`].
///
/// Two-gate comparison:
/// 1. **Per-channel max**: each channel difference must be ≤
///    `config.pixel_tolerance` (default 0 = exact match).
/// 2. **Mismatch percentage**: the fraction of differing pixels must be
///    ≤ `config.max_diff_percent` (default 0.0 = zero mismatches).
///
/// Failure diagnostics include per-channel max difference, total mismatch
/// count, mismatch percentage, and saves `_actual.png` + `_diff.png` for
/// visual inspection.
pub(crate) fn compare_with_reference_strict(
    name: &str,
    pixels: &[u8],
    width: u32,
    height: u32,
    config: &GoldenLaneConfig,
) -> Result<(), String> {
    let ref_dir = reference_dir();
    let ref_path = ref_dir.join(format!("{name}.png"));
    let actual_path = ref_dir.join(format!("{name}_actual.png"));
    let diff_path = ref_dir.join(format!("{name}_diff.png"));

    let actual: RgbaImage =
        ImageBuffer::from_raw(width, height, pixels.to_vec()).expect("pixel buffer size mismatch");

    check_ci_guard();

    // Regeneration mode.
    if std::env::var("ORITERM_UPDATE_GOLDEN").as_deref() == Ok("1") {
        std::fs::create_dir_all(&ref_dir).expect("failed to create reference dir");
        actual
            .save(&ref_path)
            .expect("failed to save reference PNG");
        eprintln!(
            "golden updated (strict): {} ({}×{})",
            ref_path.display(),
            width,
            height,
        );
        let _ = std::fs::remove_file(&actual_path);
        let _ = std::fs::remove_file(&diff_path);
        return Ok(());
    }

    if !ref_path.exists() {
        std::fs::create_dir_all(&ref_dir).expect("failed to create reference dir");
        actual
            .save(&ref_path)
            .expect("failed to save reference PNG");
        eprintln!(
            "strict reference saved: {} ({}×{}). Re-run to compare.",
            ref_path.display(),
            width,
            height,
        );
        return Ok(());
    }

    let reference = image::open(&ref_path)
        .expect("failed to open reference PNG")
        .to_rgba8();

    if reference.width() != width || reference.height() != height {
        actual
            .save(&actual_path)
            .expect("failed to save actual PNG");
        return Err(format!(
            "size mismatch: reference is {}×{}, actual is {width}×{height}. Actual saved to {}",
            reference.width(),
            reference.height(),
            actual_path.display(),
        ));
    }

    let stats = pixel_diff_stats(&reference, &actual, config.pixel_tolerance);
    let total = (width * height) as usize;
    let pct = if total > 0 {
        stats.mismatch_count as f64 / total as f64 * 100.0
    } else {
        0.0
    };

    let channel_max = stats
        .max_diff_r
        .max(stats.max_diff_g)
        .max(stats.max_diff_b)
        .max(stats.max_diff_a);
    let pct_ok = pct <= config.max_diff_percent;

    if stats.mismatch_count == 0 && channel_max == 0 {
        let _ = std::fs::remove_file(&actual_path);
        let _ = std::fs::remove_file(&diff_path);
        return Ok(());
    }

    if channel_max <= config.pixel_tolerance && pct_ok {
        let _ = std::fs::remove_file(&actual_path);
        let _ = std::fs::remove_file(&diff_path);
        return Ok(());
    }

    actual
        .save(&actual_path)
        .expect("failed to save actual PNG");
    stats
        .diff_image
        .save(&diff_path)
        .expect("failed to save diff PNG");

    Err(format!(
        "strict comparison failed: {}/{total} pixels differ ({pct:.4}%)\n\
         per-channel max: R={} G={} B={} A={} (tolerance={})\n\
         mismatch %: {pct:.4}% (threshold={:.4}%)\n\
         actual: {}\n\
         diff:   {}\n\
         to accept the new golden: \
         ORITERM_UPDATE_GOLDEN=1 cargo test -p oriterm --features gpu-tests --lib \
         (reviews the diff PNG + re-runs the test against the updated reference)",
        stats.mismatch_count,
        stats.max_diff_r,
        stats.max_diff_g,
        stats.max_diff_b,
        stats.max_diff_a,
        config.pixel_tolerance,
        config.max_diff_percent,
        actual_path.display(),
        diff_path.display(),
    ))
}

// --- Internal helpers ---

/// Panic if `ORITERM_UPDATE_GOLDEN=1` is set in a CI environment.
fn check_ci_guard() {
    if std::env::var("CI").is_ok() && std::env::var("ORITERM_UPDATE_GOLDEN").as_deref() == Ok("1") {
        panic!(
            "ORITERM_UPDATE_GOLDEN=1 is set in a CI environment (CI env var detected). \
             This would silently overwrite reference goldens, masking pixel regressions. \
             Unset ORITERM_UPDATE_GOLDEN before running GPU tests in CI."
        );
    }
}

/// Compute per-pixel diff between two images.
///
/// Returns the number of mismatched pixels and a diff image where:
/// - Matching pixels are transparent black.
/// - Mismatched pixels are red with full alpha.
pub(super) fn pixel_diff(
    reference: &RgbaImage,
    actual: &RgbaImage,
    tolerance: u8,
) -> (usize, RgbaImage) {
    let w = reference.width();
    let h = reference.height();
    let mut diff = RgbaImage::new(w, h);
    let mut count = 0;

    for y in 0..h {
        for x in 0..w {
            let r = reference.get_pixel(x, y);
            let a = actual.get_pixel(x, y);

            let matches =
                r.0.iter()
                    .zip(a.0.iter())
                    .all(|(&rv, &av)| (rv as i16 - av as i16).unsigned_abs() <= tolerance as u16);

            if !matches {
                diff.put_pixel(x, y, Rgba([255, 0, 0, 255]));
                count += 1;
            }
        }
    }

    (count, diff)
}

/// Per-channel diff statistics for strict golden comparison.
pub(crate) struct PixelDiffStats {
    /// Number of pixels that exceed the tolerance in any channel.
    pub mismatch_count: usize,
    /// Per-channel maximum absolute difference across all pixels.
    pub max_diff_r: u8,
    pub max_diff_g: u8,
    pub max_diff_b: u8,
    pub max_diff_a: u8,
    /// The diff image (red pixels on transparent background).
    pub diff_image: RgbaImage,
}

/// Compute per-pixel diff with per-channel max difference tracking.
///
/// Like [`pixel_diff`] but also tracks the maximum per-channel absolute
/// difference across all pixels.
fn pixel_diff_stats(reference: &RgbaImage, actual: &RgbaImage, tolerance: u8) -> PixelDiffStats {
    let w = reference.width();
    let h = reference.height();
    let mut diff = RgbaImage::new(w, h);
    let mut count = 0;
    let mut max_r: u8 = 0;
    let mut max_g: u8 = 0;
    let mut max_b: u8 = 0;
    let mut max_a: u8 = 0;

    for y in 0..h {
        for x in 0..w {
            let r = reference.get_pixel(x, y);
            let a = actual.get_pixel(x, y);

            let dr = (r.0[0] as i16 - a.0[0] as i16).unsigned_abs() as u8;
            let dg = (r.0[1] as i16 - a.0[1] as i16).unsigned_abs() as u8;
            let db = (r.0[2] as i16 - a.0[2] as i16).unsigned_abs() as u8;
            let da = (r.0[3] as i16 - a.0[3] as i16).unsigned_abs() as u8;

            max_r = max_r.max(dr);
            max_g = max_g.max(dg);
            max_b = max_b.max(db);
            max_a = max_a.max(da);

            let exceeds = dr > tolerance || dg > tolerance || db > tolerance || da > tolerance;
            if exceeds {
                diff.put_pixel(x, y, Rgba([255, 0, 0, 255]));
                count += 1;
            }
        }
    }

    PixelDiffStats {
        mismatch_count: count,
        max_diff_r: max_r,
        max_diff_g: max_g,
        max_diff_b: max_b,
        max_diff_a: max_a,
        diff_image: diff,
    }
}
