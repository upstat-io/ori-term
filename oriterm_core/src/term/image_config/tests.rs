use std::sync::Arc;

use crate::effect::VoidEffectSink;
use crate::grid::StableRowIndex;
use crate::image::{ImageData, ImageFormat, ImagePlacement, ImageSource, PlacementSizing};
use crate::term::Term;
use crate::theme::Theme;

/// `set_cell_dimensions` recomputes `FixedPixels` cell-coverage in the
/// ALTERNATE image cache too, not just the primary. The cross-protocol
/// lifecycle matrix + the iTerm2 OSC integration pins exercise the primary
/// cache only; `image_config/mod.rs::set_cell_dimensions` calls
/// `update_cell_coverage` on both `image_cache` and `alt_image_cache`, so this
/// pin guards the alt-cache arm against silent regression.
#[test]
fn set_cell_dimensions_recomputes_alt_screen_fixed_pixel_placement() {
    let mut term = Term::new(24, 100, 1, Theme::default(), VoidEffectSink);
    term.set_cell_dimensions(8, 16);

    // Enter the alt screen so `image_cache_mut()` targets the alt cache.
    term.swap_alt();

    let id = term.image_cache_mut().next_image_id();
    term.image_cache_mut()
        .store(ImageData {
            id,
            width: 16,
            height: 16,
            data: Arc::new(vec![128; 16 * 16 * 4]),
            pixel_generation: 0,
            format: ImageFormat::Rgba,
            source: ImageSource::Direct,
            last_accessed: 0,
            image_number: None,
        })
        .unwrap();
    // Genuine FixedPixels coverage at cell (8,16): cols=ceil(16/8)=2, rows=ceil(16/16)=1.
    term.image_cache_mut().place(ImagePlacement {
        image_id: id,
        placement_id: None,
        source_x: 0,
        source_y: 0,
        source_w: 0,
        source_h: 0,
        cell_col: 0,
        cell_row: StableRowIndex(0),
        cols: 2,
        rows: 1,
        z_index: 0,
        cell_x_offset: 0,
        cell_y_offset: 0,
        sizing: PlacementSizing::FixedPixels {
            width: 16,
            height: 16,
        },
    });

    // Font-metric change → recompute the alt cache: (2,1) -> (1,2).
    term.set_cell_dimensions(16, 8);

    let cache = term.image_cache();
    let placements = cache.placements_in_viewport(StableRowIndex(0), StableRowIndex(u64::MAX));
    let p = placements
        .iter()
        .find(|p| p.image_id == id)
        .expect("alt-cache placement present after set_cell_dimensions");
    assert_eq!(
        (p.cols, p.rows),
        (1, 2),
        "set_cell_dimensions must recompute the ALT cache FixedPixels placement"
    );
}
