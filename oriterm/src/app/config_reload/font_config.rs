//! Font config helpers — feature application, hinting/subpixel resolution,
//! codepoint mapping, and UI font registry rebuilds.

use crate::config::FontConfig;
use crate::font::{
    FaceIdx, FontCollection, FontSet, HintingMode, SubpixelMode, UiFontSizes, parse_features,
    parse_hex_range, ui_font_sizes,
};

/// Apply all font configuration settings to a collection after creation.
///
/// Handles: user features, per-fallback metadata (`size_offset`, features),
/// user variable font variations, and codepoint-to-font mappings.
///
/// `fallback_map` maps loaded fallback index → original config index. This
/// accounts for config entries that failed to load: loaded index 0 may
/// correspond to config index 2 if entries 0 and 1 failed. Use this to
/// apply per-fallback metadata and codepoint mappings to the correct face.
pub(crate) fn apply_font_config(
    collection: &mut FontCollection,
    config: &FontConfig,
    fallback_map: &[usize],
) {
    // 1. Apply user-configured OpenType features (replace defaults).
    let feature_refs: Vec<&str> = config.features.iter().map(String::as_str).collect();
    let features = parse_features(&feature_refs);
    collection.set_features(features);

    // 2. Apply per-fallback metadata (size_offset, features) to user fallbacks.
    // `fallback_map[loaded_idx]` gives the config index for each loaded fallback.
    for (loaded_idx, &config_idx) in fallback_map.iter().enumerate() {
        if let Some(fb_config) = config.fallback.get(config_idx) {
            let fb_features = fb_config.features.as_ref().map(|f| {
                let refs: Vec<&str> = f.iter().map(String::as_str).collect();
                parse_features(&refs)
            });
            collection.set_fallback_meta(
                loaded_idx,
                fb_config.size_offset.unwrap_or(0.0),
                fb_features,
            );
        }
    }

    // 3. Apply codepoint-to-font mappings.
    // Codepoint map entries reference families by name. We find the config
    // index for that family, then look up its loaded index via `fallback_map`.
    for entry in &config.codepoint_map {
        let Some((start, end)) = parse_hex_range(&entry.range) else {
            log::warn!(
                "config: invalid codepoint_map range {:?}, skipping",
                entry.range
            );
            continue;
        };
        // Find the config index for this family name, then its loaded index.
        let config_idx = config
            .fallback
            .iter()
            .position(|fb| fb.family == entry.family);
        let loaded_idx = config_idx.and_then(|ci| fallback_map.iter().position(|&mi| mi == ci));
        match loaded_idx {
            Some(li) => {
                let face_idx = FaceIdx::from_fallback_index(li);
                collection.add_codepoint_mapping(start, end, face_idx);
                log::info!(
                    "config: codepoint map {:?} → {:?} (face {})",
                    entry.range,
                    entry.family,
                    face_idx.0,
                );
            }
            None => {
                log::warn!(
                    "config: codepoint_map family {:?} not found in loaded fallbacks, skipping",
                    entry.family,
                );
            }
        }
    }
}

/// Resolve hinting mode from config, falling back to auto-detection.
///
/// Config override takes priority; auto-detection uses display scale factor.
pub(crate) fn resolve_hinting(config: &FontConfig, scale_factor: f64) -> HintingMode {
    match config.hinting.as_deref() {
        Some("full") => HintingMode::Full,
        Some("none") => HintingMode::None,
        Some(other) => {
            log::warn!("config: unknown hinting mode {other:?}, using auto-detection");
            HintingMode::from_scale_factor(scale_factor)
        }
        None => HintingMode::from_scale_factor(scale_factor),
    }
}

/// Resolve subpixel mode from config, falling back to auto-detection.
///
/// Config override takes priority; auto-detection uses display scale factor
/// and background opacity (subpixel is disabled when opacity < 1.0 to avoid
/// color fringing on transparent backgrounds).
pub(crate) fn resolve_subpixel_mode(
    config: &FontConfig,
    scale_factor: f64,
    opacity: f64,
) -> SubpixelMode {
    match config.subpixel_mode.as_deref() {
        Some("rgb" | "bgr") => {
            if opacity < 1.0 {
                log::warn!(
                    "config: subpixel rendering with transparent background may cause color fringing"
                );
            }
            if config.subpixel_mode.as_deref() == Some("rgb") {
                SubpixelMode::Rgb
            } else {
                SubpixelMode::Bgr
            }
        }
        Some("none") => SubpixelMode::None,
        Some(other) => {
            log::warn!("config: unknown subpixel_mode {other:?}, using auto-detection");
            SubpixelMode::for_display(scale_factor, opacity)
        }
        None => SubpixelMode::for_display(scale_factor, opacity),
    }
}

/// Apply font config to all collections in a [`UiFontSizes`] registry and
/// install a post-rebuild hook so DPI changes reapply the same config.
pub(crate) fn apply_font_config_to_ui_sizes(
    sizes: &mut UiFontSizes,
    config: &FontConfig,
    fallback_map: &[usize],
) {
    for fc in sizes.collections_mut() {
        apply_font_config(fc, config, fallback_map);
    }
    let config = config.clone();
    let fallback_map = fallback_map.to_vec();
    sizes.set_post_rebuild_hook(Box::new(move |fc| {
        apply_font_config(fc, &config, &fallback_map);
    }));
}

/// Rebuild the UI font sizes registry on a renderer from a fresh `FontSet`.
///
/// Creates a new [`UiFontSizes`] with the same DPI/format/hinting/weight
/// and applies user font config (features, fallback metadata, codepoint
/// mappings) to every collection. The caller must follow with
/// [`replace_font_collection`] to clear and re-prewarm atlases.
#[expect(
    clippy::too_many_arguments,
    reason = "passes through font config params"
)]
pub(super) fn rebuild_ui_font_sizes(
    renderer: &mut crate::gpu::WindowRenderer,
    font_set: &FontSet,
    dpi: f32,
    format: crate::font::GlyphFormat,
    hinting: HintingMode,
    weight: u16,
    font_config: &FontConfig,
    fallback_map: &[usize],
) {
    match UiFontSizes::new(
        font_set.clone(),
        dpi,
        format,
        hinting,
        weight,
        ui_font_sizes::PRELOAD_SIZES,
    ) {
        Ok(mut ui_sizes) => {
            apply_font_config_to_ui_sizes(&mut ui_sizes, font_config, fallback_map);
            renderer.replace_ui_font_sizes(ui_sizes);
        }
        Err(e) => {
            log::warn!("config reload: UI font registry rebuild failed: {e}");
        }
    }
}
