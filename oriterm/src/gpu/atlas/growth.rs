//! Internal growth, packing, and eviction logic for [`GlyphAtlas`].
//!
//! Extracted from `mod.rs` to keep that file under the 500-line limit.
//! All methods here are private helpers called by the public `insert` /
//! `get_or_insert` API.

use wgpu::{CommandEncoderDescriptor, Device, Extent3d, Queue};

use super::rect_packer::RectPacker;
use super::texture::create_texture_array;
use super::{AtlasPage, GLYPH_PADDING, GlyphAtlas, PAGE_SIZE};

impl GlyphAtlas {
    /// Replace the 1×1 placeholder with the full-size texture and first page.
    ///
    /// Called once on the first [`insert`](Self::insert). Bumps the generation
    /// counter so callers rebuild stale bind groups.
    pub(super) fn materialize(&mut self, device: &Device) {
        debug_assert!(self.lazy, "materialize called on non-lazy atlas");
        log::debug!(
            "atlas materializing: {:?} → {PAGE_SIZE}² texture",
            self.tex_format
        );

        let (texture, view) = create_texture_array(device, PAGE_SIZE, 1, self.tex_format);
        self.texture = texture;
        self.view = view;
        self.texture_layers = 1;
        self.pages.push(AtlasPage {
            packer: RectPacker::new(PAGE_SIZE, PAGE_SIZE),
            last_used_frame: self.frame_counter,
            glyph_count: 0,
        });
        self.generation += 1;
        self.lazy = false;
    }

    /// Grow the GPU texture to match the current page count.
    ///
    /// Creates a new `Texture2DArray` with `self.pages.len()` layers,
    /// copies all existing layers from the old texture, and replaces
    /// `self.texture` and `self.view`. Increments `self.generation` so
    /// callers can detect stale bind groups.
    pub(super) fn grow_texture(&mut self, device: &Device, queue: &Queue) {
        let new_layers = self.pages.len() as u32;
        let old_layers = self.texture_layers;
        debug_assert!(new_layers > old_layers);

        log::debug!(
            "atlas growing: {old_layers} → {new_layers} layers ({:?})",
            self.tex_format,
        );

        let (new_texture, new_view) =
            create_texture_array(device, self.page_size, new_layers, self.tex_format);

        // Copy existing layers from old texture to new texture.
        let mut encoder = device.create_command_encoder(&CommandEncoderDescriptor {
            label: Some("atlas_grow_copy"),
        });
        for layer in 0..old_layers {
            encoder.copy_texture_to_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &self.texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d {
                        x: 0,
                        y: 0,
                        z: layer,
                    },
                    aspect: wgpu::TextureAspect::All,
                },
                wgpu::TexelCopyTextureInfo {
                    texture: &new_texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d {
                        x: 0,
                        y: 0,
                        z: layer,
                    },
                    aspect: wgpu::TextureAspect::All,
                },
                Extent3d {
                    width: self.page_size,
                    height: self.page_size,
                    depth_or_array_layers: 1,
                },
            );
        }
        queue.submit(std::iter::once(encoder.finish()));

        self.texture = new_texture;
        self.view = new_view;
        self.texture_layers = new_layers;
        self.generation += 1;
    }

    /// Find space for a glyph, returning `(page_idx, x, y)`.
    ///
    /// Tries each existing page's guillotine packer. If all are full and
    /// fewer than `max_pages` exist, adds a new page. If at the page limit,
    /// evicts the least-recently-used page.
    pub(super) fn find_space(&mut self, w: u32, h: u32) -> (u32, u32, u32) {
        let padded_w = w + GLYPH_PADDING;
        let padded_h = h + GLYPH_PADDING;

        // Try existing pages.
        for (i, page) in self.pages.iter_mut().enumerate() {
            if let Some((x, y)) = page.packer.pack(padded_w, padded_h) {
                return (i as u32, x, y);
            }
        }

        // All pages full — add a new one if under the limit.
        if (self.pages.len() as u32) < self.max_pages {
            let page_idx = self.pages.len();
            self.pages.push(AtlasPage {
                packer: RectPacker::new(self.page_size, self.page_size),
                last_used_frame: self.frame_counter,
                glyph_count: 0,
            });

            let (x, y) = self.pages[page_idx]
                .packer
                .pack(padded_w, padded_h)
                .expect("fresh page must fit glyph within page_size bounds");

            return (page_idx as u32, x, y);
        }

        // At max pages — LRU eviction.
        let evicted = self.find_lru_page();
        self.evict_page(evicted);

        let (x, y) = self.pages[evicted]
            .packer
            .pack(padded_w, padded_h)
            .expect("freshly evicted page must fit glyph");

        (evicted as u32, x, y)
    }

    /// Find the page index with the smallest `last_used_frame`.
    fn find_lru_page(&self) -> usize {
        self.pages
            .iter()
            .enumerate()
            .min_by_key(|(_, p)| p.last_used_frame)
            .map(|(i, _)| i)
            .expect("at least one page exists")
    }

    /// Evict a page: reset its packer and remove all cache entries on it.
    fn evict_page(&mut self, page_idx: usize) {
        self.pages[page_idx].packer.reset();
        self.pages[page_idx].glyph_count = 0;
        self.pages[page_idx].last_used_frame = self.frame_counter;
        self.cache.retain(|_, e| e.page as usize != page_idx);
    }
}
