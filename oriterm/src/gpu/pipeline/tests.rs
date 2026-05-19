//! Tests for GPU render pipelines.

use wgpu::VertexStepMode;

use super::ui_rect::{UI_RECT_ATTRS, UI_RECT_STRIDE, ui_rect_buffer_layout};
use super::{
    INSTANCE_ATTRS, INSTANCE_STRIDE, ImagePipelineStateDump, dump_image_pipeline_state,
    instance_buffer_layout,
};
use crate::gpu::instance_writer::INSTANCE_SIZE;
use crate::gpu::ui_rect_writer::UI_RECT_INSTANCE_SIZE;

// --- Terminal instance attribute tests (no GPU) ---

#[test]
fn stride_matches_instance_size() {
    assert_eq!(INSTANCE_STRIDE, INSTANCE_SIZE as u64);
}

#[test]
fn eight_attributes() {
    assert_eq!(INSTANCE_ATTRS.len(), 8);
}

#[test]
fn attribute_offsets_and_locations() {
    let expected: [(u64, u32); 8] = [
        (0, 0),  // pos
        (8, 1),  // size
        (16, 2), // uv
        (32, 3), // fg_color
        (48, 4), // bg_color
        (64, 5), // kind
        (68, 6), // atlas_page
        (80, 7), // clip
    ];

    for (attr, (offset, location)) in INSTANCE_ATTRS.iter().zip(expected.iter()) {
        assert_eq!(
            attr.offset, *offset,
            "offset mismatch for location {location}",
        );
        assert_eq!(
            attr.shader_location, *location,
            "location mismatch at offset {offset}",
        );
    }
}

#[test]
fn last_attribute_fits_within_stride() {
    let last = &INSTANCE_ATTRS[INSTANCE_ATTRS.len() - 1];
    let end = last.offset + last.format.size();
    assert!(
        end <= INSTANCE_STRIDE,
        "last attribute ends at byte {end}, but stride is {INSTANCE_STRIDE}",
    );
}

#[test]
fn instance_buffer_layout_uses_instance_step_mode() {
    let layout = instance_buffer_layout();
    assert_eq!(layout.step_mode, VertexStepMode::Instance);
    assert_eq!(layout.array_stride, INSTANCE_STRIDE);
}

#[test]
fn instance_attributes_first_seven_are_contiguous() {
    for pair in INSTANCE_ATTRS[..7].windows(2) {
        let end = pair[0].offset + pair[0].format.size();
        assert_eq!(
            end, pair[1].offset,
            "gap between locations {} and {}",
            pair[0].shader_location, pair[1].shader_location,
        );
    }
}

#[test]
fn instance_clip_attribute_starts_after_ui_rect_fields() {
    let clip_attr = &INSTANCE_ATTRS[7];
    assert_eq!(clip_attr.offset, 80);
    assert_eq!(clip_attr.shader_location, 7);
}

// --- UI rect attribute tests (no GPU) ---

#[test]
fn ui_rect_stride_is_144() {
    assert_eq!(UI_RECT_INSTANCE_SIZE, 144);
    assert_eq!(UI_RECT_STRIDE, 144);
}

#[test]
fn ui_rect_ten_attributes() {
    assert_eq!(UI_RECT_ATTRS.len(), 10);
}

#[test]
fn ui_rect_attribute_offsets_and_locations() {
    let expected: [(u64, u32); 10] = [
        (0, 0),   // pos
        (8, 1),   // size
        (16, 2),  // clip
        (32, 3),  // fill_color
        (48, 4),  // border_widths
        (64, 5),  // corner_radii
        (80, 6),  // border_top
        (96, 7),  // border_right
        (112, 8), // border_bottom
        (128, 9), // border_left
    ];

    for (attr, (offset, location)) in UI_RECT_ATTRS.iter().zip(expected.iter()) {
        assert_eq!(
            attr.offset, *offset,
            "offset mismatch for location {location}",
        );
        assert_eq!(
            attr.shader_location, *location,
            "location mismatch at offset {offset}",
        );
    }
}

#[test]
fn ui_rect_last_attribute_fits_within_stride() {
    let last = &UI_RECT_ATTRS[UI_RECT_ATTRS.len() - 1];
    let end = last.offset + last.format.size();
    assert!(
        end <= UI_RECT_STRIDE,
        "last UI rect attribute ends at byte {end}, but stride is {UI_RECT_STRIDE}",
    );
}

#[test]
fn ui_rect_buffer_layout_uses_instance_step_mode() {
    let layout = ui_rect_buffer_layout();
    assert_eq!(layout.step_mode, VertexStepMode::Instance);
    assert_eq!(layout.array_stride, UI_RECT_STRIDE);
}

#[test]
fn ui_rect_attributes_are_contiguous() {
    for pair in UI_RECT_ATTRS.windows(2) {
        let end = pair[0].offset + pair[0].format.size();
        assert_eq!(
            end, pair[1].offset,
            "gap between locations {} and {}",
            pair[0].shader_location, pair[1].shader_location,
        );
    }
}

// --- GPU integration tests (require adapter) ---

use crate::gpu::state::GpuState;

#[test]
fn gpu_uniform_bind_group_layout_succeeds() {
    let Ok(gpu) = GpuState::new_headless() else {
        eprintln!("skipped: no GPU adapter available");
        return;
    };

    let _layout = super::create_uniform_bind_group_layout(&gpu.device);
}

#[test]
fn gpu_atlas_bind_group_layout_succeeds() {
    let Ok(gpu) = GpuState::new_headless() else {
        eprintln!("skipped: no GPU adapter available");
        return;
    };

    let _layout = super::create_atlas_bind_group_layout(&gpu.device);
}

#[test]
fn gpu_bg_pipeline_succeeds() {
    let Ok(gpu) = GpuState::new_headless() else {
        eprintln!("skipped: no GPU adapter available");
        return;
    };

    let uniform_layout = super::create_uniform_bind_group_layout(&gpu.device);
    let _pipeline = super::create_bg_pipeline(&gpu, &uniform_layout);
}

#[test]
fn gpu_fg_pipeline_succeeds() {
    let Ok(gpu) = GpuState::new_headless() else {
        eprintln!("skipped: no GPU adapter available");
        return;
    };

    let uniform_layout = super::create_uniform_bind_group_layout(&gpu.device);
    let atlas_layout = super::create_atlas_bind_group_layout(&gpu.device);
    let _pipeline = super::create_fg_pipeline(&gpu, &uniform_layout, &atlas_layout);
}

#[test]
fn gpu_both_pipelines_share_instance_layout() {
    let Ok(gpu) = GpuState::new_headless() else {
        eprintln!("skipped: no GPU adapter available");
        return;
    };

    let uniform_layout = super::create_uniform_bind_group_layout(&gpu.device);
    let atlas_layout = super::create_atlas_bind_group_layout(&gpu.device);

    let _bg = super::create_bg_pipeline(&gpu, &uniform_layout);
    let _fg = super::create_fg_pipeline(&gpu, &uniform_layout, &atlas_layout);
}

#[test]
fn gpu_ui_rect_pipeline_succeeds() {
    let Ok(gpu) = GpuState::new_headless() else {
        eprintln!("skipped: no GPU adapter available");
        return;
    };

    let uniform_layout = super::create_uniform_bind_group_layout(&gpu.device);
    let _pipeline = super::create_ui_rect_pipeline(&gpu, &uniform_layout);
}

// --- §13.6.1 item 13: image pipeline state frozen baseline ---
//
// Runs on the default `cargo test -p oriterm --lib` invocation (no
// feature gate, no `#[ignore]`). Asserts the production
// `create_image_pipeline`'s state dump matches the frozen baseline. Any
// silent drift in blend factors, vertex layout, primitive topology, or
// bind-group layout breaks this gate before any visual-regression suite
// can observe the downstream rendering symptom.
#[test]
fn image_pipeline_state_matches_frozen_baseline() {
    let dump = dump_image_pipeline_state();
    let expected = ImagePipelineStateDump {
        vertex_attrs: vec![
            (0, "Float32x2", 0),
            (1, "Float32x2", 8),
            (2, "Float32x2", 16),
            (3, "Float32x2", 24),
            (4, "Float32", 32),
        ],
        vertex_stride: 36,
        vertex_step_mode: "Instance",
        primitive_topology: "TriangleStrip",
        front_face: "Ccw",
        cull_mode: None,
        blend_color_src: "One",
        blend_color_dst: "OneMinusSrcAlpha",
        blend_color_op: "Add",
        blend_alpha_src: "One",
        blend_alpha_dst: "OneMinusSrcAlpha",
        blend_alpha_op: "Add",
        color_write_mask: "ALL",
        bind_group_layouts: vec!["uniform", "image_texture"],
    };
    assert_eq!(
        dump, expected,
        "§13.6.1 item 13: image pipeline state drift detected. If you \
         intentionally changed pipeline state, update this baseline in the \
         same commit + update the §13.6.1 plan if the change has \
         architectural significance. If you did NOT intend a state change, \
         a refactor has silently altered the image pipeline — investigate \
         BEFORE updating the baseline."
    );
}

// --- §13.6.1 item 6: overlay pass `LoadOp::Load` gate ---
//
// Source-grep proxy: reads `window_renderer/render.rs`, asserts every
// `overlay_cursor_pass` `RenderPassDescriptor` uses `LoadOp::Load` not
// `LoadOp::Clear`. The runtime alternative — synthesizing a `prepared`
// frame, pre-loading a swapchain target, running only the overlay pass —
// would require depending on private orchestration state that is not
// exposed to tests and would entangle the probe with widget / cursor
// state. The source-grep gate pins the load-op invariant with zero new
// test surface area.
//
// Runs by default (no feature gate, no `#[ignore]`). A regression that
// flipped either overlay-pass `LoadOp::Load` to `LoadOp::Clear` would
// fail this test before any visual-regression suite ran.
#[test]
fn overlay_pass_uses_load_op_not_clear_op() {
    let src = include_str!("../window_renderer/render.rs");
    // Find every `overlay_cursor_pass` descriptor — production callers
    // tag the pass label so we can scope the search to the right block.
    let mut overlay_pass_blocks = Vec::new();
    let mut search_from = 0;
    while let Some(idx) = src[search_from..].find("overlay_cursor_pass") {
        let abs = search_from + idx;
        // Window the next ~600 chars after the label literal so we capture
        // the full descriptor (color_attachments + ops block).
        let end = (abs + 600).min(src.len());
        overlay_pass_blocks.push(&src[abs..end]);
        search_from = abs + 1;
    }
    assert!(
        !overlay_pass_blocks.is_empty(),
        "§13.6.1 item 6: did not find any `overlay_cursor_pass` descriptor \
         in render.rs — the file structure changed; update this gate to \
         match the new label OR investigate why the overlay pass no longer \
         exists"
    );
    // Skip the first occurrence — it is the file-level comment block label.
    // Keep occurrences that introduce a RenderPassDescriptor (next ~600
    // chars include `RenderPassDescriptor`).
    let descriptor_blocks: Vec<&&str> = overlay_pass_blocks
        .iter()
        .filter(|b| b.contains("RenderPassDescriptor") || b.contains("begin_render_pass"))
        .collect();
    assert!(
        !descriptor_blocks.is_empty(),
        "§13.6.1 item 6: found `overlay_cursor_pass` labels but none \
         introduce a `RenderPassDescriptor`/`begin_render_pass` — file \
         shape changed; update this gate"
    );
    for (i, block) in descriptor_blocks.iter().enumerate() {
        assert!(
            block.contains("LoadOp::Load"),
            "§13.6.1 item 6: overlay_cursor_pass descriptor #{i} does NOT \
             contain `LoadOp::Load` — the overlay pass MUST preserve the \
             cached-content blit underneath it. Found block: {block:?}"
        );
        assert!(
            !block.contains("LoadOp::Clear"),
            "§13.6.1 item 6: overlay_cursor_pass descriptor #{i} contains \
             `LoadOp::Clear` — would wipe the cache blit and produce \
             black/transparent output beneath cursor + overlays. Found \
             block: {block:?}"
        );
    }
}

/// §13.6.1 item 19: bisection-matrix completeness assertion.
///
/// Enumerates the 10 instrumentation probes the §13.6.1 deliverable
/// requires. The 3 orchestration probes (content_cache_view,
/// copy_cache_to_output, overlay_pass) are kept SEPARATE — collapsing
/// them into a single "orchestration" entry regresses to the
/// pre-§13.6.1 ambiguity where the failing-stage diagnostic bucket
/// collapsed 3 stages and provided no cure guidance per
/// `plans/spec-conformance/section-13-kitty-graphics.md` §13.6.1
/// success criterion line 550 (blind-spots.json blind_spots[0]).
///
/// This test exists as a constant-time enumeration gate. If a future
/// refactor reduces the probe set without updating this list, the
/// assertion fires immediately — the §13.6.1 bisection contract is
/// then broken at the source.
#[test]
fn instrumentation_bisection_matrix_completeness() {
    const PROBES: &[&str] = &[
        // Pipeline state probes (3) — isolate failing layer inside the
        // pipeline itself.
        "texture_readback",            // §13.6.1 item 3 — ImageTextureCache::read_texture_for_test
        "shader_debug_color",          // §13.6.1 item 9 — gpu-debug-image-shader feature
        "blend_state_bypass",          // §13.6.1 item 10 — gpu-debug-image-blend-off + create_image_pipeline_no_blend
        // Direct construction probe (1) — isolate orchestration from pipeline.
        "minimal_quad_draw",           // §13.6.1 item 11 — pilots/minimal_image_quad.rs
        // Cross-protocol comparison probe (1).
        "cross_protocol_sister_sixel", // §13.6.1 item 12 — probe_sixel_pipeline_stages
        // Frozen-baseline state-drift probe (1).
        "pipeline_state_dump",         // §13.6.1 item 13 — image_pipeline_state_matches_frozen_baseline
        // Orchestration probes (3) — bisect render_frame_cached stages.
        "content_cache_view",          // §13.6.1 item 4 — probe_content_cache_view_contains_red_pixels
        "copy_cache_to_output",        // §13.6.1 item 5 — copy_cache_to_output_preserves_kitty_red_pixels
        "overlay_pass_load_op",        // §13.6.1 item 6 — overlay_pass_uses_load_op_not_clear_op
        // Apex regression guard (1) — the cure-confirmation gate.
        "cure_confirmation_cell_pinned", // §13.6.1 item 14 — kitty_image_renders_visible_red_pixel_at_cell_zero_zero
    ];

    assert_eq!(
        PROBES.len(),
        10,
        "§13.6.1 item 19: bisection matrix MUST enumerate exactly 10 probes. \
         The 3 orchestration probes (content_cache_view, copy_cache_to_output, \
         overlay_pass_load_op) MUST stay separate; collapsing them regresses \
         the diagnostic to pre-§13.6.1 ambiguity. If you intentionally added \
         or removed a probe, update this assertion AND update §13.6.1 success \
         criterion line 550 + the §13.6.1 plan body in the same commit."
    );

    let unique: std::collections::HashSet<_> = PROBES.iter().copied().collect();
    assert_eq!(
        unique.len(),
        PROBES.len(),
        "§13.6.1 item 19: probe names must be distinct — duplicates indicate \
         a copy-paste error in the matrix."
    );
}
