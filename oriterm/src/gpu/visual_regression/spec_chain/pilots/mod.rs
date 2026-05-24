//! Visual pilot scenarios for the verification chain.
//!
//! Each pilot drives a sequence through every applicable visual rung
//! (1-8) and asserts all pass. Non-visual pilots (DA1) live under
//! `oriterm_core/tests/spec_chain/pilots/` — they don't need GPU access.

pub mod iterm2_image_bmp_at_cursor;
pub mod iterm2_image_inline_at_cursor;
pub mod iterm2_image_jpeg_at_cursor;
pub mod iterm2_image_png_at_cursor;
pub mod kitty_animation_transition;
pub mod kitty_cure_confirmation;
pub mod kitty_minimal;
pub mod kitty_placeholder_basic;
pub mod kitty_placeholder_multi_cell;
pub mod kitty_placeholder_sixel_coexist;
pub mod kitty_sixel_mixed_with_text;
pub mod kitty_sixel_mixed_z_order;
#[cfg(all(test, feature = "gpu-tests"))]
pub mod minimal_image_quad;
pub mod notcurses_info_visual;
pub mod probe_image_pipeline;
pub mod sixel_12_4_matrix;
pub mod sixel_cr_nl_banding;
pub mod sixel_cursor_right;
pub mod sixel_minimal;
pub mod sixel_occlusion_subcell;
pub mod sixel_occlusion_wide_cjk;
pub mod sixel_occlusion_zwj;
pub mod sixel_palette_switch;
pub mod sixel_repeat;
pub mod sixel_scrolling_off;
pub mod sixel_set_to_bg;
pub mod sixel_transparency;
pub mod xray_replay;
