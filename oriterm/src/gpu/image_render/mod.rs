//! GPU texture cache and rendering support for inline terminal images.
//!
//! Each image gets its own `wgpu::Texture` (not atlased — images vary
//! wildly in size). Textures are uploaded lazily when images enter the
//! viewport and evicted via LRU when GPU memory exceeds the limit.

use std::collections::HashMap;
use std::collections::hash_map::Entry;

use oriterm_core::image::ImageId;
use wgpu::{
    AddressMode, BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindingResource,
    Device, Extent3d, FilterMode, Queue, SamplerDescriptor, Texture, TextureDescriptor,
    TextureDimension, TextureFormat, TextureUsages, TextureView, TextureViewDescriptor,
};

/// Default GPU memory limit for image textures (512 MiB).
const DEFAULT_GPU_MEMORY_LIMIT: usize = 512 * 1024 * 1024;

/// An uploaded image texture on the GPU.
pub(crate) struct GpuImageTexture {
    /// Underlying GPU texture. Production: retained to keep the bind-group's
    /// view valid for the texture's lifetime (wgpu requires the texture to
    /// outlive its views). Test-only: `read_texture_for_test` reads back
    /// pixels from it per §13.6.1 item 3.
    #[allow(
        dead_code,
        reason = "RAII anchor for bind-group view; also read by test-only \
                  read_texture_for_test under cfg(any(test, feature = gpu-tests))"
    )]
    texture: Texture,
    _view: TextureView,
    bind_group: BindGroup,
    size_bytes: usize,
    last_frame: u64,
    /// `ImageData::pixel_generation` observed at the most recent
    /// upload. (xray-scene lag cure) Phase 3 records it; Phase 4 Item 4a wires
    /// the re-upload gate in `ensure_uploaded`'s Occupied arm.
    #[allow(
        dead_code,
        reason = "Phase 3 scaffolding (xray-scene lag cure) — Phase 4 gates re-upload on this field"
    )]
    pixel_generation: u64,
    /// Texture width — recorded so Phase 4's re-upload gate can detect
    /// dimension changes and recreate the texture instead of writing
    /// through `queue.write_texture` (which would mismatch).
    #[allow(
        dead_code,
        reason = "Phase 3 scaffolding (xray-scene lag cure) — Phase 4 gates texture recreate on dim change"
    )]
    width: u32,
    /// Texture height — see `width`.
    #[allow(
        dead_code,
        reason = "Phase 3 scaffolding (xray-scene lag cure) — Phase 4 gates texture recreate on dim change"
    )]
    height: u32,
}

/// GPU-side image texture cache.
///
/// Manages per-image `wgpu::Texture` resources. Uploads are lazy (on first
/// viewport appearance). LRU eviction keeps GPU memory bounded.
pub(crate) struct ImageTextureCache {
    textures: HashMap<ImageId, GpuImageTexture>,
    gpu_memory_used: usize,
    gpu_memory_limit: usize,
    frame_counter: u64,
    sampler: wgpu::Sampler,
}

impl ImageTextureCache {
    /// Create a new empty cache with a shared sampler.
    pub(crate) fn new(device: &Device) -> Self {
        let sampler = device.create_sampler(&SamplerDescriptor {
            label: Some("image_sampler"),
            address_mode_u: AddressMode::ClampToEdge,
            address_mode_v: AddressMode::ClampToEdge,
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            ..Default::default()
        });

        Self {
            textures: HashMap::new(),
            gpu_memory_used: 0,
            gpu_memory_limit: DEFAULT_GPU_MEMORY_LIMIT,
            frame_counter: 0,
            sampler,
        }
    }

    /// Advance the frame counter. Call once per frame before `ensure_uploaded`.
    pub(crate) fn begin_frame(&mut self) {
        self.frame_counter += 1;
    }

    /// Ensure the image is uploaded to the GPU, returning its bind group.
    ///
    /// If already uploaded, touches the LRU counter. If not, creates a new
    /// texture and uploads the RGBA data.
    ///
    /// `pixel_generation` is the value from the upstream
    /// `ImageData::pixel_generation` / `RenderableImageData::pixel_generation`
    /// field — Phase 3 records it on the entry but does NOT yet gate
    /// re-upload (cache returns existing bind group on `Occupied`).
    /// Phase 4 Item 4a wires the gate: on `Occupied`, compare entry's
    /// recorded generation against the incoming value; if they differ,
    /// re-upload `data` (recreating texture when dimensions changed) and
    /// stamp the new generation.
    #[expect(
        clippy::too_many_arguments,
        reason = "GPU upload needs device, queue, layout, id, generation, data, and dimensions"
    )]
    pub(crate) fn ensure_uploaded(
        &mut self,
        device: &Device,
        queue: &Queue,
        layout: &BindGroupLayout,
        id: ImageId,
        pixel_generation: u64,
        data: &[u8],
        width: u32,
        height: u32,
    ) -> &BindGroup {
        let frame = self.frame_counter;

        match self.textures.entry(id) {
            Entry::Occupied(e) => {
                let entry = e.into_mut();
                entry.last_frame = frame;
                if entry.pixel_generation != pixel_generation {
                    // Image's pixel content changed (animation advance,
                    // frame edit, frame delete) — re-upload before the
                    // next draw. If the dimensions changed too (rare),
                    // recreate the texture; otherwise write_texture in
                    // place.
                    if entry.width == width && entry.height == height {
                        perform_texture_upload(
                            queue,
                            &entry.texture,
                            data,
                            width,
                            height,
                            frame,
                            true,
                        );
                    } else {
                        // Dimensions changed — full recreate. Account
                        // for the new memory footprint.
                        self.gpu_memory_used =
                            self.gpu_memory_used.saturating_sub(entry.size_bytes);
                        let new_entry = create_texture_entry(
                            device,
                            queue,
                            layout,
                            &self.sampler,
                            data,
                            width,
                            height,
                            frame,
                            pixel_generation,
                        );
                        self.gpu_memory_used += new_entry.size_bytes;
                        *entry = new_entry;
                        return &entry.bind_group;
                    }
                    entry.pixel_generation = pixel_generation;
                }
                &entry.bind_group
            }
            Entry::Vacant(e) => {
                let new_entry = create_texture_entry(
                    device,
                    queue,
                    layout,
                    &self.sampler,
                    data,
                    width,
                    height,
                    frame,
                    pixel_generation,
                );
                self.gpu_memory_used += new_entry.size_bytes;
                let entry = e.insert(new_entry);
                &entry.bind_group
            }
        }
    }

    /// Get the bind group for an already-uploaded image.
    pub(crate) fn get_bind_group(&self, id: ImageId) -> Option<&BindGroup> {
        self.textures.get(&id).map(|t| &t.bind_group)
    }

    /// Frame counter (per-frame eviction-clock SSOT).
    ///
    /// Test surface only — production sites use [`begin_frame`] /
    /// [`ensure_uploaded`] / [`touch_image`] / [`evict_unused`] / [`evict_over_limit`]
    /// which thread the counter internally.
    #[cfg(any(test, feature = "gpu-tests"))]
    #[allow(
        dead_code,
        reason = "consumed by gpu-tests-gated frame-counter delta tests"
    )]
    pub(crate) fn frame_counter(&self) -> u64 {
        self.frame_counter
    }

    /// Touch an already-uploaded image to refresh its LRU position.
    ///
    /// Returns `true` if the image was present and its `last_frame` was
    /// stamped, `false` if the image was not in cache (already evicted).
    /// Idempotent — calling twice in the same frame produces the same
    /// `last_frame` value.
    ///
    /// Used by the multi-pane render path when a pane is served from the
    /// per-pane prepared-frame cache without re-running `prepare_pane_into`
    /// (which would otherwise touch images via [`ensure_uploaded`]). The
    /// `false` return signals the caller that the cached `ImageQuad`s no
    /// longer have valid bind groups (the image was evicted between the
    /// cache write and now) — the caller must invalidate the pane cache
    /// and re-prepare the pane.
    pub(crate) fn touch_image(&mut self, id: ImageId) -> bool {
        let frame = self.frame_counter;
        if let Some(entry) = self.textures.get_mut(&id) {
            entry.last_frame = frame;
            true
        } else {
            false
        }
    }

    /// Evict textures not used in the last `threshold` frames.
    pub(crate) fn evict_unused(&mut self, threshold: u64) {
        let cutoff = self.frame_counter.saturating_sub(threshold);
        let mem = &mut self.gpu_memory_used;
        self.textures.retain(|_id, tex| {
            if tex.last_frame < cutoff {
                *mem = mem.saturating_sub(tex.size_bytes);
                false
            } else {
                true
            }
        });
    }

    /// Evict the oldest textures until GPU memory is under the limit.
    pub(crate) fn evict_over_limit(&mut self) {
        if self.gpu_memory_used <= self.gpu_memory_limit || self.textures.is_empty() {
            return;
        }
        // Sort by last_frame ascending (oldest first), evict until under limit.
        let mut entries: Vec<_> = self
            .textures
            .iter()
            .map(|(&id, t)| (id, t.last_frame))
            .collect();
        entries.sort_unstable_by_key(|&(_, frame)| frame);
        for (id, _) in entries {
            if self.gpu_memory_used <= self.gpu_memory_limit {
                break;
            }
            if let Some(tex) = self.textures.remove(&id) {
                self.gpu_memory_used = self.gpu_memory_used.saturating_sub(tex.size_bytes);
            }
        }
    }

    /// Remove a specific image texture.
    #[cfg(test)]
    pub(crate) fn remove(&mut self, id: ImageId) {
        if let Some(tex) = self.textures.remove(&id) {
            self.gpu_memory_used = self.gpu_memory_used.saturating_sub(tex.size_bytes);
        }
    }

    /// Update the GPU memory limit. Triggers eviction if currently over.
    pub(crate) fn set_gpu_memory_limit(&mut self, limit: usize) {
        self.gpu_memory_limit = limit;
        self.evict_over_limit();
    }

    /// Total GPU memory used by image textures.
    #[cfg(test)]
    pub(crate) fn gpu_memory_used(&self) -> usize {
        self.gpu_memory_used
    }

    /// Number of textures currently cached.
    #[cfg(test)]
    pub(crate) fn texture_count(&self) -> usize {
        self.textures.len()
    }

    /// Test-only diagnostic: read back the raw RGBA bytes of an uploaded
    /// image texture. Returns `None` if no upload exists for `id`.
    ///
    /// Allocates a host-visible staging buffer, encodes a
    /// `copy_texture_to_buffer` from the cached texture, submits + waits
    /// for completion, maps the buffer and returns the bytes (RGBA order,
    /// padding stripped — `len() == width * height * 4`).
    ///
    /// Honors wgpu's `COPY_BYTES_PER_ROW_ALIGNMENT = 256` row-padding rule
    /// internally; returned bytes have NO padding rows.
    ///
    /// Caller passes the `device + queue` from the active `GpuState` —
    /// probes typically reach them via `harness.gpu().device_for_test()` /
    /// `harness.gpu().queue_for_test()`. Used by §13.6.1 item 3
    /// (texture-readback probe) to confirm uploaded pixels match the
    /// transmit payload prior to render — isolating bugs above vs below
    /// the upload boundary.
    #[cfg(any(test, feature = "gpu-tests"))]
    #[allow(
        dead_code,
        reason = "consumed by §13.6.1 item 3 probe pilots authored after \
                  the foundational instrumentation lands; gated on \
                  cfg(any(test, feature = gpu-tests))"
    )]
    pub(crate) fn read_texture_for_test(
        &self,
        id: ImageId,
        device: &Device,
        queue: &Queue,
    ) -> Option<Vec<u8>> {
        let entry = self.textures.get(&id)?;
        let texture = &entry.texture;
        let width = texture.width();
        let height = texture.height();
        let bytes_per_pixel: u32 = 4;
        let unpadded_row = width * bytes_per_pixel;
        let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
        let padded_row = unpadded_row.div_ceil(align) * align;
        let buffer_size = u64::from(padded_row) * u64::from(height);

        let staging = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("image_texture_readback"),
            size: buffer_size,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("image_texture_readback_encoder"),
        });
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &staging,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(padded_row),
                    rows_per_image: Some(height),
                },
            },
            Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );

        queue.submit(std::iter::once(encoder.finish()));

        let slice = staging.slice(..);
        let (tx, rx) = std::sync::mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |result| {
            let _ = tx.send(result);
        });
        device
            .poll(wgpu::PollType::wait_indefinitely())
            .expect("read_texture_for_test: GPU poll failed");
        rx.recv()
            .expect("read_texture_for_test: readback channel disconnected")
            .expect("read_texture_for_test: buffer map failed");

        let data = slice.get_mapped_range();
        let unpadded_row_usize = unpadded_row as usize;
        let padded_row_usize = padded_row as usize;
        let mut out = Vec::with_capacity(unpadded_row_usize * height as usize);
        if padded_row == unpadded_row {
            out.extend_from_slice(&data[..unpadded_row_usize * height as usize]);
        } else {
            for row in 0..height as usize {
                let start = row * padded_row_usize;
                let end = start + unpadded_row_usize;
                out.extend_from_slice(&data[start..end]);
            }
        }
        drop(data);
        staging.unmap();
        Some(out)
    }
}

/// Upload RGBA pixel data into an existing texture via `queue.write_texture`.
///
/// Free function so it can be called from inside `ensure_uploaded`'s
/// Occupied arm while `&mut entry` is held — a `&self` method would
/// conflict per Plan TPR R1. Latency log is throttled under sustained
/// animation traffic (once per 60 frames) so the warn-on-stall signal
/// stays visible without log-spamming kitty/xray-style workloads.
fn perform_texture_upload(
    queue: &Queue,
    texture: &Texture,
    data: &[u8],
    width: u32,
    height: u32,
    frame_counter: u64,
    animated: bool,
) {
    let upload_start = std::time::Instant::now();
    queue.write_texture(
        texture.as_image_copy(),
        data,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(4 * width),
            rows_per_image: Some(height),
        },
        Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
    );
    let upload_ms = upload_start.elapsed().as_millis();
    if upload_ms >= 16 && (!animated || frame_counter % 60 == 0) {
        log::info!(
            target: "oriterm::gpu::image_render::upload_latency",
            "queue.write_texture {width}x{height} ({} bytes) took {upload_ms}ms (staging buffer pressure)",
            data.len()
        );
    }
}

/// Allocate a fresh `GpuImageTexture` + upload data. Used by both the
/// Vacant arm of `ensure_uploaded` and the dim-change branch of the
/// Occupied arm (when the texture must be recreated rather than
/// re-uploaded into).
#[expect(
    clippy::too_many_arguments,
    reason = "texture creation needs device, queue, layout, sampler, data + dims + frame + generation"
)]
fn create_texture_entry(
    device: &Device,
    queue: &Queue,
    layout: &BindGroupLayout,
    sampler: &wgpu::Sampler,
    data: &[u8],
    width: u32,
    height: u32,
    frame: u64,
    pixel_generation: u64,
) -> GpuImageTexture {
    let size = Extent3d {
        width,
        height,
        depth_or_array_layers: 1,
    };
    let texture = device.create_texture(&TextureDescriptor {
        label: Some("image_texture"),
        size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: TextureDimension::D2,
        format: TextureFormat::Rgba8UnormSrgb,
        usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST | TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    perform_texture_upload(
        queue, &texture, data, width, height, frame, /* animated */ false,
    );
    let view = texture.create_view(&TextureViewDescriptor::default());
    let bind_group = device.create_bind_group(&BindGroupDescriptor {
        label: Some("image_bind_group"),
        layout,
        entries: &[
            BindGroupEntry {
                binding: 0,
                resource: BindingResource::TextureView(&view),
            },
            BindGroupEntry {
                binding: 1,
                resource: BindingResource::Sampler(sampler),
            },
        ],
    });
    let size_bytes = (width as usize) * (height as usize) * 4;
    GpuImageTexture {
        texture,
        _view: view,
        bind_group,
        size_bytes,
        last_frame: frame,
        pixel_generation,
        width,
        height,
    }
}

#[cfg(test)]
mod tests;
