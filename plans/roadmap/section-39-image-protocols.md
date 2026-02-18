---
section: 39
title: Image Protocols
status: not-started
tier: 5
goal: "Inline image display via Kitty Graphics Protocol, Sixel, and iTerm2 image protocol. Full GPU-accelerated compositing with text overlay, animation support, memory-managed image cache."
sections:
  - id: "39.1"
    title: Image Storage + Cache
    status: not-started
  - id: "39.2"
    title: Kitty Graphics Protocol
    status: not-started
  - id: "39.3"
    title: Sixel Graphics
    status: not-started
  - id: "39.4"
    title: iTerm2 Image Protocol
    status: not-started
  - id: "39.5"
    title: Image Rendering + GPU Compositing
    status: not-started
  - id: "39.6"
    title: Section Completion
    status: not-started
---

# Section 39: Image Protocols

**Status:** Not Started
**Goal:** Display images inline in the terminal via Kitty Graphics Protocol, Sixel, and iTerm2 image protocol. GPU-accelerated compositing with configurable z-ordering (above or below text), animation support, and memory-managed image cache with eviction. This is a must-have feature — every modern terminal except Alacritty supports at least one image protocol.

**Crate:** `oriterm_core` (image storage, protocol parsing), `oriterm` (GPU rendering, texture management)
**Dependencies:** Section 02 (VTE — DCS/APC/OSC parsing), Section 05 (GPU pipeline), Section 06 (atlas/texture management patterns)

**Reference:**
- Kitty graphics protocol spec: https://sw.kovidgoyal.net/kitty/graphics-protocol/
- Ghostty `src/terminal/kitty/graphics*.zig` (SIMD-accelerated image decode, placement model)
- WezTerm `term/src/image.rs`, `wezterm-gui/src/termwindow/render/` (multi-protocol support)
- Alacritty: deliberately omits image support (we go beyond)

**Why this matters:** Image protocols are what make `viu`, `timg`, `imgcat`, `hologram`, `ranger` previews, Jupyter inline plots, and `kitty icat` work. Without image support, these tools fall back to ASCII art or don't work at all.

---

## 39.1 Image Storage + Cache

In-memory image cache with reference counting, eviction, and configurable memory limits.

**File:** `oriterm_core/src/image.rs`

- [ ] `ImageId(u32)` newtype — unique per image within a terminal instance
- [ ] `ImageData` struct:
  - [ ] `id: ImageId`
  - [ ] `width: u32`, `height: u32` — pixel dimensions
  - [ ] `data: Arc<Vec<u8>>` — decoded RGBA pixel data (shared across placements)
  - [ ] `format: ImageFormat` — `Rgba`, `Rgb`, `Png` (for lazy decode)
  - [ ] `source: ImageSource` — `Direct`, `File(PathBuf)`, `SharedMemory`
- [ ] `ImagePlacement` struct:
  - [ ] `image_id: ImageId` — reference to image data
  - [ ] `placement_id: Option<u32>` — Kitty placement ID (for updates/deletes)
  - [ ] `x: u32`, `y: u32` — pixel offset within image (source rect)
  - [ ] `w: u32`, `h: u32` — display size in pixels
  - [ ] `cell_col: usize`, `cell_row: usize` — grid position (top-left cell)
  - [ ] `cols: usize`, `rows: usize` — cell span
  - [ ] `z_index: i32` — negative = below text, positive = above text
- [ ] `ImageCache` struct:
  - [ ] `images: HashMap<ImageId, ImageData>` — image data store
  - [ ] `placements: Vec<ImagePlacement>` — active placements (sorted by row for render)
  - [ ] `memory_used: usize` — total bytes of decoded image data
  - [ ] `memory_limit: usize` — configurable max (default: 256 MB)
  - [ ] `next_id: u32` — monotonic ID allocator
- [ ] `ImageCache::store(data: ImageData) -> ImageId` — add image, evict LRU if over limit
- [ ] `ImageCache::place(placement: ImagePlacement)` — add placement
- [ ] `ImageCache::remove_image(id: ImageId)` — remove image and all its placements
- [ ] `ImageCache::remove_placement(image_id: ImageId, placement_id: u32)` — remove specific placement
- [ ] `ImageCache::remove_by_position(col: usize, row: usize)` — remove placements at cell
- [ ] `ImageCache::placements_in_viewport(top_row: usize, bottom_row: usize) -> &[ImagePlacement]`
- [ ] `ImageCache::evict_lru()` — remove least-recently-used image when over memory limit
- [ ] Scrollback interaction: placements scroll with text (row indices are absolute)
- [ ] **Tests:**
  - [ ] Store/retrieve image data roundtrip
  - [ ] Placement at cell position, query by viewport
  - [ ] Memory limit triggers LRU eviction
  - [ ] Remove by ID, by placement, by position

---

## 39.2 Kitty Graphics Protocol

The preferred modern image protocol. Transmission via APC sequences with chunked transfer, multiple placement modes, and animation support.

**File:** `oriterm_core/src/image.rs` (parsing), `oriterm/src/term_handler/image.rs` (handler integration)

**Reference:** Kitty protocol spec, Ghostty `src/terminal/kitty/graphics_command.zig`

- [ ] APC sequence parsing:
  - [ ] `ESC_G <control-data> ; <payload> ST` format
  - [ ] Control data: key=value pairs separated by commas
  - [ ] Payload: base64-encoded image data (or empty for commands)
- [ ] Transmission actions (`a=` key):
  - [ ] `t` (transmit): upload image data
  - [ ] `T` (transmit + display): upload and immediately place
  - [ ] `p` (put/place): place previously uploaded image
  - [ ] `d` (delete): delete image/placement
  - [ ] `f` (frame): animation frame operations
  - [ ] `a` (animate): animation control
- [ ] Transmission formats (`f=` key):
  - [ ] 24 (RGB), 32 (RGBA), 100 (PNG compressed)
- [ ] Transmission methods (`t=` key):
  - [ ] `d` (direct): payload contains base64 image data
  - [ ] `f` (file): payload contains base64 file path
  - [ ] `t` (temp file): payload contains temp file path (deleted after read)
  - [ ] `s` (shared memory): payload contains shm name
- [ ] Chunked transfer:
  - [ ] `m=1`: more chunks follow
  - [ ] `m=0`: final chunk (or single-chunk transfer)
  - [ ] Accumulate chunks into complete payload before decoding
- [ ] Placement parameters:
  - [ ] `i=` image ID, `p=` placement ID
  - [ ] `s=`, `v=` source rect size (pixels)
  - [ ] `c=`, `r=` display size (cells)
  - [ ] `x=`, `y=` source rect offset (pixels)
  - [ ] `X=`, `Y=` cell offset within placement cell
  - [ ] `z=` z-index (layer ordering)
  - [ ] `C=1` cursor movement suppression
- [ ] Delete operations (`d=` key with `a=d`):
  - [ ] `a` (all), `i` (by image ID), `p` (by placement ID)
  - [ ] `c` (at cursor), `n` (in cell range), `z` (by z-index)
- [ ] Animation support:
  - [ ] Frame composition modes: overwrite, blend
  - [ ] Frame timing via `z=` (duration in ms)
  - [ ] Animation control: start, stop, loop count
- [ ] Response: `ESC_G` response for success/failure (when `q=1` or `q=2` requested)
- [ ] **Tests:**
  - [ ] Parse control data key-value pairs
  - [ ] Single-chunk PNG transmission + placement
  - [ ] Multi-chunk transmission accumulates correctly
  - [ ] Delete by image ID removes correct image
  - [ ] Placement respects cell position and span

---

## 39.3 Sixel Graphics

Legacy image protocol using DCS sequences. Widely supported by older terminals and tools.

**File:** `oriterm_core/src/image.rs` (sixel decoder)

**Reference:** WezTerm `term/src/terminalstate/sixel.rs`, VT340 programmer reference

- [ ] DCS sequence parsing:
  - [ ] `DCS P1 ; P2 ; P3 q <sixel-data> ST`
  - [ ] P1: pixel aspect ratio (0 or 2:1 default)
  - [ ] P2: background select (0=device default, 1=no change, 2=set to bg)
  - [ ] P3: horizontal grid size (ignored, use 0)
- [ ] Sixel data decoding:
  - [ ] Character range: 0x3F–0x7E (63–126), subtract 0x3F for 6-bit column
  - [ ] Each character encodes 6 vertical pixels (1 column × 6 rows)
  - [ ] `$` (carriage return): reset x to left margin
  - [ ] `-` (line feed): move down 6 pixel rows, reset x
  - [ ] `!<count><char>` (repeat): repeat character N times
  - [ ] `#<color>` (color): select palette index
  - [ ] `#<idx>;2;<r>;<g>;<b>` (color define): define RGB color (0-100 range)
  - [ ] `#<idx>;1;<h>;<l>;<s>` (color define): define HLS color
- [ ] Sixel to RGBA conversion:
  - [ ] Build palette from color definitions (up to 256 colors)
  - [ ] Decode sixel columns into pixel buffer
  - [ ] Convert palette-indexed pixels to RGBA
- [ ] Placement:
  - [ ] Image placed at current cursor position
  - [ ] Cursor advances past image (configurable: DECSET 80 controls cursor position after sixel)
  - [ ] Image occupies grid cells based on pixel size / cell size
- [ ] **Tests:**
  - [ ] Decode simple sixel: single color, known pattern
  - [ ] Repeat operator produces correct pixel count
  - [ ] Color palette definition (RGB mode)
  - [ ] Multi-row sixel (line feed advances by 6 pixels)
  - [ ] Cursor position after sixel display

---

## 39.4 iTerm2 Image Protocol

OSC-based image protocol used by iTerm2 and supported by many tools via `imgcat`.

**File:** `oriterm_core/src/image.rs` (iTerm2 parser)

**Reference:** iTerm2 image protocol spec, WezTerm `term/src/terminalstate/iterm.rs`

- [ ] OSC 1337 parsing:
  - [ ] `OSC 1337 ; File=[args] : <base64-data> ST`
  - [ ] Arguments (semicolon-separated key=value):
    - [ ] `name=<base64>` — filename (base64-encoded)
    - [ ] `size=<bytes>` — file size hint
    - [ ] `width=<spec>` — display width (N, Npx, N%, auto)
    - [ ] `height=<spec>` — display height (same format)
    - [ ] `preserveAspectRatio=0|1` — maintain aspect ratio (default: 1)
    - [ ] `inline=0|1` — display inline (1) or as download (0)
- [ ] Image decode:
  - [ ] Base64-decode payload
  - [ ] Detect format from magic bytes (PNG, JPEG, GIF, BMP)
  - [ ] Decode to RGBA via `image` crate
- [ ] Placement:
  - [ ] Width/height parsing: pixel (`Npx`), cell count (`N`), percentage (`N%`), auto
  - [ ] Auto: use image's native size, clamped to terminal width
  - [ ] Place at current cursor position
  - [ ] Cursor advances below image
- [ ] `inline=0`: download — store file, don't display (stretch goal)
- [ ] **Tests:**
  - [ ] Parse width/height specs: "auto", "80", "100px", "50%"
  - [ ] Base64 payload decoded correctly
  - [ ] Aspect ratio preserved when `preserveAspectRatio=1`
  - [ ] Image placed at cursor position with correct cell span

---

## 39.5 Image Rendering + GPU Compositing

Render cached images as GPU textures composited into the terminal frame.

**File:** `oriterm/src/gpu/render_image.rs`, `oriterm/src/gpu/pipeline.rs` (image pipeline)

- [ ] Image texture management:
  - [ ] Separate `wgpu::Texture` for images (distinct from glyph atlas)
  - [ ] Upload decoded RGBA data as `Rgba8UnormSrgb` texture
  - [ ] Texture atlas or individual textures per image (atlas preferred for small images)
  - [ ] Lazy upload: only upload to GPU when image enters viewport
  - [ ] Evict GPU texture when image scrolls far out of viewport
- [ ] Image render pipeline:
  - [ ] New render pass (or sub-pass) for image compositing
  - [ ] Z-index ordering:
    - [ ] z < 0: render images BEFORE cell backgrounds (below text)
    - [ ] z >= 0: render images AFTER cell foregrounds (above text)
  - [ ] Image instances: position (pixels), size (pixels), UV coords, opacity
  - [ ] WGSL shader: sample image texture, blend with alpha
- [ ] Cell interaction:
  - [ ] Cells covered by an image: still render text on top (for z < 0 images)
  - [ ] Cells covered by z >= 0 images: image obscures text
  - [ ] Background color: use cell's bg color behind transparent image regions
- [ ] Scrolling:
  - [ ] Images scroll with text (absolute row positions)
  - [ ] Partially visible images clipped at viewport boundaries
  - [ ] Smooth scroll offset applied to image positions
- [ ] Animation:
  - [ ] Timer-driven frame switching for animated images
  - [ ] Only animate images in viewport (save CPU/GPU)
  - [ ] Configurable: `terminal.image_animation = true | false` (default: true)
- [ ] Config:
  ```toml
  [terminal]
  image_protocol = true        # enable/disable all image protocols
  image_memory_limit = 268435456  # 256 MB default
  image_animation = true
  ```
- [ ] **Tests:**
  - [ ] Image texture uploads to GPU correctly
  - [ ] Image at z=-1 renders below text
  - [ ] Image at z=1 renders above text
  - [ ] Image scrolls with content
  - [ ] Image clipped at viewport boundary
  - [ ] Memory limit evicts oldest images

---

## 39.6 Section Completion

- [ ] All 39.1–39.5 items complete
- [ ] Kitty Graphics Protocol: transmit, place, delete, animate
- [ ] Sixel: decode and render legacy sixel images
- [ ] iTerm2: `imgcat`-compatible inline image display
- [ ] GPU compositing: images render at correct z-order with text
- [ ] Memory management: configurable limit with LRU eviction
- [ ] Scrolling: images scroll with text, clip at viewport
- [ ] Animation: timer-driven frame switching for animated images
- [ ] `cargo test` — all image protocol tests pass
- [ ] `cargo clippy --target x86_64-pc-windows-gnu` — no warnings

**Exit Criteria:** `kitty icat`, `imgcat`, `viu`, `timg`, and sixel-based tools display images inline in the terminal. Images composite correctly with text, scroll with content, and respect memory limits.
