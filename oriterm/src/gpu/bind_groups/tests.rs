//! Tests for GPU bind group resources.

use super::{AtlasBindGroup, UniformBuffer, create_placeholder_atlas_texture};
use crate::gpu::pipeline;
use crate::gpu::state::GpuState;

// --- AtlasFiltering ---

#[test]
fn atlas_filtering_from_scale_factor_low_dpi() {
    assert_eq!(
        super::AtlasFiltering::from_scale_factor(1.0),
        super::AtlasFiltering::Linear,
    );
}

#[test]
fn atlas_filtering_from_scale_factor_high_dpi() {
    assert_eq!(
        super::AtlasFiltering::from_scale_factor(2.0),
        super::AtlasFiltering::Nearest,
    );
}

#[test]
fn atlas_filtering_from_scale_factor_boundary() {
    // 1.99 is below the 2.0 threshold → Linear.
    assert_eq!(
        super::AtlasFiltering::from_scale_factor(1.99),
        super::AtlasFiltering::Linear,
    );
}

#[test]
fn atlas_filtering_to_filter_mode_linear() {
    assert_eq!(
        super::AtlasFiltering::Linear.to_filter_mode(),
        wgpu::FilterMode::Linear,
    );
}

#[test]
fn atlas_filtering_to_filter_mode_nearest() {
    assert_eq!(
        super::AtlasFiltering::Nearest.to_filter_mode(),
        wgpu::FilterMode::Nearest,
    );
}

// --- Uniform buffer ---

#[test]
fn uniform_buffer_creation_succeeds() {
    let Ok(gpu) = GpuState::new_headless() else {
        eprintln!("skipped: no GPU adapter available");
        return;
    };

    let layout = pipeline::create_uniform_bind_group_layout(&gpu.device);
    let _uniform = UniformBuffer::new(&gpu.device, &layout);
}

#[test]
fn write_screen_size_does_not_panic() {
    let Ok(gpu) = GpuState::new_headless() else {
        eprintln!("skipped: no GPU adapter available");
        return;
    };

    let layout = pipeline::create_uniform_bind_group_layout(&gpu.device);
    let uniform = UniformBuffer::new(&gpu.device, &layout);

    // Writing screen size should not panic or cause validation errors.
    uniform.write_screen_size(&gpu.queue, 1920.0, 1080.0);
}

#[test]
fn write_screen_size_zero_dimensions() {
    let Ok(gpu) = GpuState::new_headless() else {
        eprintln!("skipped: no GPU adapter available");
        return;
    };

    let layout = pipeline::create_uniform_bind_group_layout(&gpu.device);
    let uniform = UniformBuffer::new(&gpu.device, &layout);

    // Zero dimensions are valid (e.g. minimized window). Should not panic.
    uniform.write_screen_size(&gpu.queue, 0.0, 0.0);
}

#[test]
fn uniform_bind_group_accessor_returns_valid_ref() {
    let Ok(gpu) = GpuState::new_headless() else {
        eprintln!("skipped: no GPU adapter available");
        return;
    };

    let layout = pipeline::create_uniform_bind_group_layout(&gpu.device);
    let uniform = UniformBuffer::new(&gpu.device, &layout);

    // Accessor should return a valid reference (no panic).
    let _bg = uniform.bind_group();
}

// --- Placeholder atlas texture ---

#[test]
fn placeholder_texture_creation_succeeds() {
    let Ok(gpu) = GpuState::new_headless() else {
        eprintln!("skipped: no GPU adapter available");
        return;
    };

    let (_texture, _view) = create_placeholder_atlas_texture(&gpu.device, &gpu.queue);
}

#[test]
fn placeholder_texture_is_1x1_d2array_r8unorm() {
    let Ok(gpu) = GpuState::new_headless() else {
        eprintln!("skipped: no GPU adapter available");
        return;
    };

    let (texture, _view) = create_placeholder_atlas_texture(&gpu.device, &gpu.queue);

    assert_eq!(texture.width(), 1);
    assert_eq!(texture.height(), 1);
    assert_eq!(texture.depth_or_array_layers(), 1);
    assert_eq!(texture.format(), wgpu::TextureFormat::R8Unorm);
}

// --- Atlas bind group ---

#[test]
fn atlas_bind_group_creation_with_placeholder() {
    let Ok(gpu) = GpuState::new_headless() else {
        eprintln!("skipped: no GPU adapter available");
        return;
    };

    let layout = pipeline::create_atlas_bind_group_layout(&gpu.device);
    let (_texture, view) = create_placeholder_atlas_texture(&gpu.device, &gpu.queue);

    let _atlas_bg = AtlasBindGroup::new(&gpu.device, &layout, &view, wgpu::FilterMode::Linear);
}

#[test]
fn atlas_bind_group_rebuild_creates_new_bind_group() {
    let Ok(gpu) = GpuState::new_headless() else {
        eprintln!("skipped: no GPU adapter available");
        return;
    };

    let layout = pipeline::create_atlas_bind_group_layout(&gpu.device);
    let (_texture, view) = create_placeholder_atlas_texture(&gpu.device, &gpu.queue);

    let mut atlas_bg = AtlasBindGroup::new(&gpu.device, &layout, &view, wgpu::FilterMode::Linear);

    // Simulate atlas growth: create a second placeholder and rebuild.
    let (_texture2, view2) = create_placeholder_atlas_texture(&gpu.device, &gpu.queue);
    atlas_bg.rebuild(&gpu.device, &layout, &view2);

    // Should still return a valid bind group.
    let _bg = atlas_bg.bind_group();
}

#[test]
fn atlas_bind_group_accessor_returns_valid_ref() {
    let Ok(gpu) = GpuState::new_headless() else {
        eprintln!("skipped: no GPU adapter available");
        return;
    };

    let layout = pipeline::create_atlas_bind_group_layout(&gpu.device);
    let (_texture, view) = create_placeholder_atlas_texture(&gpu.device, &gpu.queue);
    let atlas_bg = AtlasBindGroup::new(&gpu.device, &layout, &view, wgpu::FilterMode::Linear);

    let _bg = atlas_bg.bind_group();
}

// --- Integration: bind groups work with pipelines ---

#[test]
fn bind_groups_compatible_with_pipelines() {
    let Ok(gpu) = GpuState::new_headless() else {
        eprintln!("skipped: no GPU adapter available");
        return;
    };

    let uniform_layout = pipeline::create_uniform_bind_group_layout(&gpu.device);
    let atlas_layout = pipeline::create_atlas_bind_group_layout(&gpu.device);

    // Create bind groups using the same layouts as the pipelines.
    let _uniform = UniformBuffer::new(&gpu.device, &uniform_layout);
    let (_texture, view) = create_placeholder_atlas_texture(&gpu.device, &gpu.queue);
    let _atlas_bg =
        AtlasBindGroup::new(&gpu.device, &atlas_layout, &view, wgpu::FilterMode::Linear);

    // Create pipelines with the same layouts. If the layouts don't match
    // the bind groups, wgpu validation will catch it at draw time.
    let _bg_pipeline = pipeline::create_bg_pipeline(&gpu, &uniform_layout);
    let _fg_pipeline = pipeline::create_fg_pipeline(&gpu, &uniform_layout, &atlas_layout);
}

// --- AtlasBindGroup: filter mode ---

#[test]
fn atlas_bind_group_new_with_linear_filter() {
    let Ok(gpu) = GpuState::new_headless() else {
        eprintln!("skipped: no GPU adapter available");
        return;
    };

    let layout = pipeline::create_atlas_bind_group_layout(&gpu.device);
    let (_texture, view) = create_placeholder_atlas_texture(&gpu.device, &gpu.queue);

    let bg = AtlasBindGroup::new(&gpu.device, &layout, &view, wgpu::FilterMode::Linear);
    assert_eq!(bg.filter(), wgpu::FilterMode::Linear);
}

#[test]
fn atlas_bind_group_new_with_nearest_filter() {
    let Ok(gpu) = GpuState::new_headless() else {
        eprintln!("skipped: no GPU adapter available");
        return;
    };

    let layout = pipeline::create_atlas_bind_group_layout(&gpu.device);
    let (_texture, view) = create_placeholder_atlas_texture(&gpu.device, &gpu.queue);

    let bg = AtlasBindGroup::new(&gpu.device, &layout, &view, wgpu::FilterMode::Nearest);
    assert_eq!(bg.filter(), wgpu::FilterMode::Nearest);
}

#[test]
fn atlas_bind_group_rebuild_preserves_filter() {
    let Ok(gpu) = GpuState::new_headless() else {
        eprintln!("skipped: no GPU adapter available");
        return;
    };

    let layout = pipeline::create_atlas_bind_group_layout(&gpu.device);
    let (_texture, view) = create_placeholder_atlas_texture(&gpu.device, &gpu.queue);

    let mut bg = AtlasBindGroup::new(&gpu.device, &layout, &view, wgpu::FilterMode::Nearest);

    // Simulate atlas growth: rebuild with a new texture view.
    let (_texture2, view2) = create_placeholder_atlas_texture(&gpu.device, &gpu.queue);
    bg.rebuild(&gpu.device, &layout, &view2);

    // Filter mode must survive rebuild.
    assert_eq!(bg.filter(), wgpu::FilterMode::Nearest);
}
