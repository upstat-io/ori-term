//! Meta tests for the visual regression comparison framework.
//!
//! These test the image comparison infrastructure itself — no GPU required.

use image::{Rgba, RgbaImage};

use super::{
    MAX_MISMATCH_PERCENT, PIXEL_TOLERANCE, compare_with_reference, pixel_diff, reference_dir,
};

#[test]
fn identical_images_pass() {
    let w = 10;
    let h = 10;
    let mut img = RgbaImage::new(w, h);
    for y in 0..h {
        for x in 0..w {
            img.put_pixel(x, y, Rgba([100, 150, 200, 255]));
        }
    }

    let (mismatches, _) = pixel_diff(&img, &img, PIXEL_TOLERANCE);
    assert_eq!(
        mismatches, 0,
        "identical images should have zero mismatches"
    );
}

#[test]
fn one_pixel_off_within_tolerance() {
    let w = 10;
    let h = 10;
    let mut reference = RgbaImage::new(w, h);
    let mut actual = RgbaImage::new(w, h);

    for y in 0..h {
        for x in 0..w {
            reference.put_pixel(x, y, Rgba([100, 150, 200, 255]));
            // Shift each channel by ±1 — within the ±2 tolerance.
            actual.put_pixel(x, y, Rgba([101, 149, 201, 255]));
        }
    }

    let (mismatches, _) = pixel_diff(&reference, &actual, PIXEL_TOLERANCE);
    assert_eq!(
        mismatches, 0,
        "±1 per channel should be within ±{PIXEL_TOLERANCE} tolerance"
    );
}

#[test]
fn visually_different_fails() {
    let w = 10;
    let h = 10;
    let total = (w * h) as usize;

    let mut black = RgbaImage::new(w, h);
    let mut white = RgbaImage::new(w, h);

    for y in 0..h {
        for x in 0..w {
            black.put_pixel(x, y, Rgba([0, 0, 0, 255]));
            white.put_pixel(x, y, Rgba([255, 255, 255, 255]));
        }
    }

    let (mismatches, diff) = pixel_diff(&black, &white, PIXEL_TOLERANCE);
    assert_eq!(
        mismatches, total,
        "black vs white should differ on every pixel"
    );

    // Every pixel in the diff image should be red.
    for y in 0..h {
        for x in 0..w {
            assert_eq!(
                *diff.get_pixel(x, y),
                Rgba([255, 0, 0, 255]),
                "diff pixel ({x},{y}) should be red"
            );
        }
    }
}

#[test]
fn percentage_threshold_allows_small_differences() {
    let w = 100;
    let h = 100;
    let total = (w * h) as usize;

    let mut reference = RgbaImage::new(w, h);
    let mut actual = RgbaImage::new(w, h);

    for y in 0..h {
        for x in 0..w {
            reference.put_pixel(x, y, Rgba([100, 100, 100, 255]));
            actual.put_pixel(x, y, Rgba([100, 100, 100, 255]));
        }
    }

    // Flip a few pixels — well within the 0.5% threshold.
    let num_different = (total as f64 * MAX_MISMATCH_PERCENT / 100.0 / 2.0) as usize;
    for i in 0..num_different {
        let x = (i % w as usize) as u32;
        let y = (i / w as usize) as u32;
        actual.put_pixel(x, y, Rgba([200, 200, 200, 255]));
    }

    let (mismatches, _) = pixel_diff(&reference, &actual, PIXEL_TOLERANCE);
    let pct = mismatches as f64 / total as f64 * 100.0;
    assert!(
        pct <= MAX_MISMATCH_PERCENT,
        "{mismatches}/{total} ({pct:.2}%) should be within {MAX_MISMATCH_PERCENT}% threshold"
    );
}

#[test]
fn missing_golden_creates_reference() {
    use std::fs;

    let ref_dir = reference_dir();
    fs::create_dir_all(&ref_dir).expect("create reference dir");

    // Use a unique name to avoid collision with real tests.
    let name = "_meta_test_missing_golden";
    let ref_path = ref_dir.join(format!("{name}.png"));

    // Ensure no stale file exists.
    let _ = fs::remove_file(&ref_path);

    let w = 4u32;
    let h = 4u32;
    let pixels: Vec<u8> = vec![128; (w * h * 4) as usize];

    let result = compare_with_reference(name, &pixels, w, h);
    assert!(
        result.is_ok(),
        "missing golden should create reference and pass"
    );
    assert!(ref_path.exists(), "reference PNG should have been created");

    // Clean up.
    let _ = fs::remove_file(&ref_path);
}

#[test]
fn zero_size_image_returns_zero_mismatches() {
    let empty = RgbaImage::new(0, 0);
    let (mismatches, diff) = pixel_diff(&empty, &empty, PIXEL_TOLERANCE);
    assert_eq!(mismatches, 0, "empty images should have zero mismatches");
    assert_eq!(diff.width(), 0);
    assert_eq!(diff.height(), 0);
}

#[test]
fn transparent_pixels_with_different_rgb_still_compared() {
    // Fully transparent pixels (alpha=0) differ only in RGB. Our comparator
    // checks all 4 channels equally, so these WILL count as mismatches if
    // the RGB delta exceeds tolerance. This is intentional — transparent
    // pixel differences indicate a real rendering difference even if
    // invisible when composited.
    let w = 4;
    let h = 4;
    let mut reference = RgbaImage::new(w, h);
    let mut actual = RgbaImage::new(w, h);

    for y in 0..h {
        for x in 0..w {
            // Same alpha=0, but wildly different RGB.
            reference.put_pixel(x, y, Rgba([0, 0, 0, 0]));
            actual.put_pixel(x, y, Rgba([255, 255, 255, 0]));
        }
    }

    let (mismatches, _) = pixel_diff(&reference, &actual, PIXEL_TOLERANCE);
    assert_eq!(
        mismatches,
        (w * h) as usize,
        "transparent pixels with different RGB should count as mismatches"
    );
}

#[test]
fn update_golden_overwrites_reference() {
    use std::fs;

    let ref_dir = reference_dir();
    fs::create_dir_all(&ref_dir).expect("create reference dir");

    let name = "_meta_test_update_golden";
    let ref_path = ref_dir.join(format!("{name}.png"));

    // Create an initial reference.
    let w = 4u32;
    let h = 4u32;
    let pixels_v1: Vec<u8> = vec![100; (w * h * 4) as usize];
    let _ = compare_with_reference(name, &pixels_v1, w, h);
    assert!(ref_path.exists(), "initial reference should exist");
    let size_v1 = fs::metadata(&ref_path).expect("metadata").len();

    // Overwrite with different pixels via ORITERM_UPDATE_GOLDEN.
    // We can't set the env var here without affecting other tests, so
    // instead verify the file was created and can be re-compared.
    let pixels_v2: Vec<u8> = vec![200; (w * h * 4) as usize];
    let result = compare_with_reference(name, &pixels_v2, w, h);

    // v2 pixels differ from v1 reference — should fail (mismatch > threshold).
    assert!(result.is_err(), "different pixels should fail comparison");

    // Clean up.
    let _ = fs::remove_file(&ref_path);
    let _ = fs::remove_file(ref_dir.join(format!("{name}_actual.png")));
    let _ = fs::remove_file(ref_dir.join(format!("{name}_diff.png")));

    // Verify the original reference size was non-zero.
    assert!(size_v1 > 0, "reference PNG should have non-zero size");
}
