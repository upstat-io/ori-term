//! iTerm2 image protocol handler.
//!
//! Handles OSC 1337 File= sequences: parses args, decodes image data,
//! stores in `ImageCache`, and creates a placement at cursor.

use std::sync::Arc;

use log::warn;

use crate::effect::sink::EffectSink;
use crate::grid::StableRowIndex;
use crate::image::iterm2::{Iterm2Image, SizeSpec, parse_iterm2_file};
use crate::image::{
    ImageData, ImageFormat, ImageId, ImagePlacement, ImageSource, PlacementSizing,
    decode_gif_frames, decode_to_rgba, detect_format,
};
use crate::term::Term;

/// Parameters for resolving iTerm2 display dimensions.
struct DisplaySizeParams {
    w_spec: SizeSpec,
    h_spec: SizeSpec,
    img_w: u32,
    img_h: u32,
    cell_w: u32,
    cell_h: u32,
    term_w: u32,
    term_h: u32,
    preserve_aspect: bool,
}

impl<S: EffectSink> Term<S> {
    /// Parse and execute an iTerm2 File= image command.
    pub(in crate::term::handler) fn handle_iterm2_file(&mut self, params: &[&[u8]]) {
        if !self.image_protocol_enabled {
            return;
        }
        let image = match parse_iterm2_file(params) {
            Ok(img) => img,
            Err(e) => {
                warn!("iTerm2 image parse error: {e}");
                return;
            }
        };

        // Non-inline images are downloads — not displayed.
        if !image.inline {
            return;
        }

        let max_bytes = self.image_cache().max_single_image_bytes();
        if image.data.len() > max_bytes {
            warn!(
                "iTerm2 image exceeds max size ({} > {max_bytes})",
                image.data.len()
            );
            return;
        }

        // Try animated GIF extraction first.
        let is_gif = detect_format(&image.data) == Some(ImageFormat::Gif);
        if is_gif {
            if let Some(gif) = decode_gif_frames(&image.data, max_bytes) {
                let id = self.image_cache_mut().next_image_id();
                let frames: Vec<Arc<Vec<u8>>> = gif.frames.into_iter().map(Arc::new).collect();
                let img_data = ImageData {
                    id,
                    width: gif.width,
                    height: gif.height,
                    data: frames[0].clone(),
                    pixel_generation: 0,
                    format: ImageFormat::Rgba,
                    source: ImageSource::Direct,
                    last_accessed: 0,
                    image_number: None,
                };
                match self.image_cache_mut().store_animated(
                    img_data,
                    frames,
                    gif.durations,
                    gif.loop_count,
                ) {
                    Ok(_) => {
                        self.iterm2_create_placement(id, gif.width, gif.height, &image);
                        return;
                    }
                    Err(e) => {
                        warn!("iTerm2 animated GIF store failed: {e}");
                        return;
                    }
                }
            }
        }

        // Decode image to RGBA pixels (single frame or non-GIF).
        let (rgba, img_w, img_h) = match decode_to_rgba(&image.data, max_bytes) {
            Ok(result) => result,
            Err(e) => {
                warn!("iTerm2 image decode failed: {e}");
                return;
            }
        };

        // Store image in cache.
        let id = self.image_cache_mut().next_image_id();
        let img_data = ImageData {
            id,
            width: img_w,
            height: img_h,
            data: Arc::new(rgba),
            pixel_generation: 0,
            format: ImageFormat::Rgba,
            source: ImageSource::Direct,
            last_accessed: 0,
            image_number: None,
        };

        if let Err(e) = self.image_cache_mut().store(img_data) {
            warn!("iTerm2 image store failed: {e}");
            return;
        }

        self.iterm2_create_placement(id, img_w, img_h, &image);
    }

    /// Create an image placement at the current cursor position.
    fn iterm2_create_placement(
        &mut self,
        id: ImageId,
        img_w: u32,
        img_h: u32,
        image: &Iterm2Image,
    ) {
        let cell_w = self.cell_pixel_width.max(1) as u32;
        let cell_h = self.cell_pixel_height.max(1) as u32;
        let term_cols = self.grid().cols();
        let term_lines = self.grid().lines();

        let (display_w, display_h) = resolve_display_size(&DisplaySizeParams {
            w_spec: image.width,
            h_spec: image.height,
            img_w,
            img_h,
            cell_w,
            cell_h,
            term_w: term_cols as u32 * cell_w,
            term_h: term_lines as u32 * cell_h,
            preserve_aspect: image.preserve_aspect_ratio,
        });

        // `resolve_display_size` already fit the result within the terminal
        // bounds (aspect-preserving when requested), so no call-site clamp.
        let cols = display_w.div_ceil(cell_w) as usize;
        let rows = display_h.div_ceil(cell_h) as usize;

        // Cell-count sizing only when both dimensions are explicitly cells.
        // Otherwise use fixed-pixel sizing (auto, pixels, percent all resolve
        // to concrete pixel dimensions that should not scale with cell size).
        let sizing = if matches!(image.width, SizeSpec::Cells(_))
            && matches!(image.height, SizeSpec::Cells(_))
        {
            PlacementSizing::CellCount
        } else {
            PlacementSizing::FixedPixels {
                width: display_w,
                height: display_h,
            }
        };

        let grid = self.grid();
        let col = grid.cursor().col().0;
        let line = grid.cursor().line();
        let stable_row = StableRowIndex::from_visible(grid, line);

        let placement = ImagePlacement {
            image_id: id,
            placement_id: None,
            source_x: 0,
            source_y: 0,
            source_w: img_w,
            source_h: img_h,
            cell_col: col,
            cell_row: stable_row,
            cols,
            rows,
            z_index: 0,
            cell_x_offset: 0,
            cell_y_offset: 0,
            sizing,
        };

        self.image_cache_mut().place(placement);

        // Cursor advances below image.
        let prev = self.grid().total_evicted();
        let grid = self.grid_mut();
        for _ in 0..rows.saturating_sub(1) {
            grid.linefeed();
        }
        self.prune_images_if_evicted(prev);
    }
}

/// Scale `other_native` in proportion to how `driver` relates to
/// `driver_native`, preserving aspect ratio. Integer arithmetic via `u64`
/// avoids f32 precision loss. Returns `other_native` unchanged when
/// `driver_native` is zero (degenerate source dimension).
fn scale_proportional(other_native: u32, driver: u32, driver_native: u32) -> u32 {
    if driver_native > 0 {
        sat_u32(u64::from(other_native) * u64::from(driver) / u64::from(driver_native))
    } else {
        other_native
    }
}

/// Saturating `u64 -> u32` downcast. SSOT for image-dimension arithmetic so
/// an attacker-controlled OSC 1337 size value cannot wrap silently on the
/// downcast; the result is bounded by the terminal in `clamp_to_terminal`
/// regardless, but saturating keeps the intermediate honest.
fn sat_u32(v: u64) -> u32 {
    u32::try_from(v).unwrap_or(u32::MAX)
}

/// Scale `(w, h)` down to fit within the terminal bounds `(term_w, term_h)`.
///
/// SSOT for the terminal-bounds clamp: a size already within bounds is
/// returned unchanged; when `preserve` is set, both axes scale by the same
/// factor (the largest that fits, via integer cross-multiplication) so the
/// aspect ratio survives; otherwise each axis is clamped independently.
/// Folding this into `resolve_display_size` (rather than clamping width
/// alone at the call site) keeps an over-wide explicit width aspect-correct
/// and prevents an over-tall image from scrolling off-screen.
fn clamp_to_terminal(w: u32, h: u32, term_w: u32, term_h: u32, preserve: bool) -> (u32, u32) {
    if w <= term_w && h <= term_h {
        return (w.max(1), h.max(1));
    }
    if !preserve || w == 0 || h == 0 {
        return (w.min(term_w).max(1), h.min(term_h).max(1));
    }
    // scale = min(term_w / w, term_h / h), chosen by cross-multiplication
    // (term_w * h <= term_h * w  <=>  the width bound is the tighter one).
    let use_w = u64::from(term_w) * u64::from(h) <= u64::from(term_h) * u64::from(w);
    let (num, den) = if use_w { (term_w, w) } else { (term_h, h) };
    let out_w = sat_u32(u64::from(w) * u64::from(num) / u64::from(den));
    let out_h = sat_u32(u64::from(h) * u64::from(num) / u64::from(den));
    (out_w.max(1), out_h.max(1))
}

/// Resolve display pixel size from iTerm2 size specs, clamped to the
/// terminal bounds.
///
/// Handles `Auto`, `Cells`, `Pixels`, and `Percent` modes for both width
/// and height, with optional aspect ratio preservation, then fits the
/// result within `(term_w, term_h)` via [`clamp_to_terminal`].
fn resolve_display_size(p: &DisplaySizeParams) -> (u32, u32) {
    let (w, h) = resolve_display_size_within_spec(p);
    clamp_to_terminal(w, h, p.term_w, p.term_h, p.preserve_aspect)
}

/// Resolve display pixel size from the iTerm2 size specs alone, before the
/// terminal-bounds clamp.
fn resolve_display_size_within_spec(p: &DisplaySizeParams) -> (u32, u32) {
    let raw_w = resolve_one_dimension(p.w_spec, p.img_w, p.cell_w, p.term_w);
    let raw_h = resolve_one_dimension(p.h_spec, p.img_h, p.cell_h, p.term_h);

    if !p.preserve_aspect {
        return (raw_w.max(1), raw_h.max(1));
    }

    // Preserve aspect ratio when one or both dimensions are auto.
    let w_is_auto = p.w_spec == SizeSpec::Auto;
    let h_is_auto = p.h_spec == SizeSpec::Auto;

    match (w_is_auto, h_is_auto) {
        // Both auto: native size. Terminal fitting is owned solely by
        // `clamp_to_terminal` (SSOT) — do NOT re-clamp width here.
        (true, true) => (p.img_w.max(1), p.img_h.max(1)),
        // Width explicit, height auto: scale height to match.
        (false, true) => {
            let w = raw_w.max(1);
            let h = scale_proportional(p.img_h, w, p.img_w);
            (w, h.max(1))
        }
        // Height explicit, width auto: scale width to match.
        (true, false) => {
            let h = raw_h.max(1);
            let w = scale_proportional(p.img_w, h, p.img_h);
            (w.max(1), h)
        }
        // Both explicit: fit within W×H bbox preserving aspect ratio.
        // Per iterm2.com/3.4/documentation-images.html §Inline Images,
        // preserveAspectRatio=1 (the default) means scale-to-fit, NOT
        // stretch. Compute scale = min(W/img_w, H/img_h) and return the
        // largest aspect-preserved size that fits in the bbox.
        (false, false) => {
            if p.img_w == 0 || p.img_h == 0 {
                return (raw_w.max(1), raw_h.max(1));
            }
            // scale_num/scale_den = min(raw_w / img_w, raw_h / img_h)
            // computed with integer arithmetic to avoid f32 precision loss.
            let by_w_num = u64::from(raw_w);
            let by_w_den = u64::from(p.img_w);
            let by_h_num = u64::from(raw_h);
            let by_h_den = u64::from(p.img_h);
            // Compare by cross-multiplication: (by_w_num * by_h_den) vs (by_h_num * by_w_den).
            let use_w_scale = by_w_num * by_h_den <= by_h_num * by_w_den;
            let (scale_num, scale_den) = if use_w_scale {
                (by_w_num, by_w_den)
            } else {
                (by_h_num, by_h_den)
            };
            let out_w = sat_u32(u64::from(p.img_w) * scale_num / scale_den);
            let out_h = sat_u32(u64::from(p.img_h) * scale_num / scale_den);
            (out_w.max(1), out_h.max(1))
        }
    }
}

/// Resolve a single dimension from a `SizeSpec`.
///
/// `n` / `pct` are attacker-controlled (parsed from the OSC 1337 `File=`
/// value with no upper bound), so the multiplications use saturating
/// arithmetic — a crafted `width=4000000000` must not panic in debug or
/// wrap in release. The result is bounded to the terminal by
/// `clamp_to_terminal` regardless.
fn resolve_one_dimension(spec: SizeSpec, native: u32, cell_size: u32, term_size: u32) -> u32 {
    match spec {
        SizeSpec::Auto => native,
        SizeSpec::Cells(n) => n.saturating_mul(cell_size),
        SizeSpec::Pixels(n) => n,
        SizeSpec::Percent(pct) => sat_u32(u64::from(term_size) * u64::from(pct) / 100),
    }
}
