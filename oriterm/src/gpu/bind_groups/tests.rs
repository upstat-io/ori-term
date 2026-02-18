//! Tests for GPU bind group resources.

use super::{AtlasBindGroup, UniformBuffer, create_placeholder_atlas_texture};
use crate::gpu::pipeline;
use crate::gpu::state::GpuState;

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

    let _atlas_bg = AtlasBindGroup::new(&gpu.device, &layout, &view);
}

#[test]
fn atlas_bind_group_rebuild_creates_new_bind_group() {
    let Ok(gpu) = GpuState::new_headless() else {
        eprintln!("skipped: no GPU adapter available");
        return;
    };

    let layout = pipeline::create_atlas_bind_group_layout(&gpu.device);
    let (_texture, view) = create_placeholder_atlas_texture(&gpu.device, &gpu.queue);

    let mut atlas_bg = AtlasBindGroup::new(&gpu.device, &layout, &view);

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
    let atlas_bg = AtlasBindGroup::new(&gpu.device, &layout, &view);

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
    let _atlas_bg = AtlasBindGroup::new(&gpu.device, &atlas_layout, &view);

    // Create pipelines with the same layouts. If the layouts don't match
    // the bind groups, wgpu validation will catch it at draw time.
    let _bg_pipeline = pipeline::create_bg_pipeline(&gpu, &uniform_layout);
    let _fg_pipeline = pipeline::create_fg_pipeline(&gpu, &uniform_layout, &atlas_layout);
}
