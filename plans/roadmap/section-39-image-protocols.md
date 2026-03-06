---
section: 39
title: Image Protocols
status: in-progress
tier: 5
goal: "Inline image display via Kitty Graphics Protocol, Sixel, and iTerm2 image protocol. Full GPU-accelerated compositing with text overlay, animation support, memory-managed image cache."
sections:
  - id: "39.1"
    title: Image Storage + Cache
    status: complete
  - id: "39.2"
    title: Kitty Graphics Protocol
    status: in-progress
  - id: "39.3"
    title: Sixel Graphics
    status: complete
  - id: "39.4"
    title: iTerm2 Image Protocol
    status: in-progress
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

**Crate:** `oriterm_core` (image storage, protocol parsing, image decode), `oriterm` (GPU rendering in `oriterm/src/gpu/`, texture management)
**Dependencies:** Section 02 (VTE — DCS/OSC parsing), Section 05 (GPU pipeline), Section 06 (atlas/texture management patterns)
**New dependency:** The `image` crate must be added as a runtime dependency of `oriterm_core` for PNG/JPEG/GIF/BMP/WebP decoding. Currently `image` is only a build/dev dependency of `oriterm` (not `oriterm_core`).
**VTE prerequisite:** The local VTE crate (`crates/vte`) must be extended with APC support before Kitty graphics can work. Currently, APC sequences (`ESC _` ... `ST`) enter `SosPmApcString` state (`crates/vte/src/lib.rs:182`) which calls `self.anywhere()` and discards all content. The `Perform` trait needs `apc_start`, `apc_put`, and `apc_end` callbacks. See Ghostty's `src/terminal/apc.zig` for a clean APC handler design.

**Reference:**
- Kitty graphics protocol spec: https://sw.kovidgoyal.net/kitty/graphics-protocol/
- Ghostty `src/terminal/kitty/graphics*.zig` (command parsing, image storage, placement model), `src/terminal/apc.zig` (APC handler dispatching `G` to kitty graphics parser)
- WezTerm `term/src/terminalstate/kitty.rs` (Kitty protocol), `term/src/terminalstate/sixel.rs` (Sixel), `term/src/terminalstate/iterm.rs` (iTerm2), `wezterm-gui/src/termwindow/render/` (GPU compositing)
- Alacritty: deliberately omits image support (we go beyond)

**Why this matters:** Image protocols are what make `viu`, `timg`, `imgcat`, `hologram`, `ranger` previews, Jupyter inline plots, and `kitty icat` work. Without image support, these tools fall back to ASCII art or don't work at all.

---

## 39.1 Image Storage + Cache

In-memory image cache with reference counting, eviction, and configurable memory limits.

**File:** `oriterm_core/src/image/mod.rs` (directory module — will exceed 500 lines as a single file)
**Submodules:** `oriterm_core/src/image/cache.rs` (ImageCache), `oriterm_core/src/image/decode.rs` (format detection + RGBA decode), `oriterm_core/src/image/tests.rs`
**Feature gate:** The `image` crate adds ~15s to clean builds. Feature-gate it: `[features] image-protocol = ["dep:image"]` in `oriterm_core/Cargo.toml`. All image protocol code gated behind `#[cfg(feature = "image-protocol")]`. Enable by default but allows disabling for faster CI/dev builds.

### Types

- [x] `ImageId(u32)` newtype — unique per image within a terminal instance
- [x] `ImageData` struct:
  - [x] `id: ImageId`
  - [x] `width: u32`, `height: u32` — pixel dimensions
  - [x] `data: Arc<Vec<u8>>` — decoded RGBA pixel data (shared across placements). GPU layer receives `&[u8]` via `data.as_slice()` — never clone `Arc` across the core-to-GPU boundary.
  - [x] `format: ImageFormat` — `Rgba`, `Rgb`, `Png` (for lazy decode)
  - [x] `source: ImageSource` — `Direct`, `File(PathBuf)`, `SharedMemory`
  - [x] `last_accessed: u64` — monotonic counter for LRU eviction ordering
- [x] `ImagePlacement` struct:
  - [x] `image_id: ImageId` — reference to image data
  - [x] `placement_id: Option<u32>` — Kitty placement ID (for updates/deletes)
  - [x] `source_x: u32`, `source_y: u32` — pixel offset within image (source rect origin)
  - [x] `source_w: u32`, `source_h: u32` — source rect size in pixels
  - [x] `cell_col: usize` — grid column (top-left cell)
  - [x] `cell_row: StableRowIndex` — grid row as stable row index (survives scrollback eviction). The grid already has `StableRowIndex` support (`oriterm_core/src/grid/stable_index.rs`).
  - [x] `cols: usize`, `rows: usize` — cell span
  - [x] `z_index: i32` — negative = below text, positive = above text
  - [x] `cell_x_offset: u16`, `cell_y_offset: u16` — sub-cell pixel offset (Kitty `X=`/`Y=` params)

### ImageCache

- [x] `ImageCache` struct:
  - [x] `images: HashMap<ImageId, ImageData>` — image data store
  - [x] `placements: Vec<ImagePlacement>` — active placements (sorted by row for render)
  - [x] `memory_used: usize` — total bytes of decoded image data
  - [x] `memory_limit: usize` — configurable max (default: 320 MB, matching Ghostty)
  - [x] `max_single_image_bytes: usize` — reject images exceeding this limit (default: 64 MB)
  - [x] `next_id: u32` — monotonic ID allocator (start at `2_147_483_647` to avoid collisions with app-assigned IDs, matching Ghostty's mid-range start)
  - [x] `access_counter: u64` — monotonic counter bumped on each image access, used for LRU ordering
  - [x] `dirty: bool` — set when placements/images change; caller clears via `take_dirty()` after consuming. One-way data flow: the renderer never reaches back into `ImageCache` to clear dirty. `Term::renderable_content_into()` calls `take_dirty()` when building `RenderableContent`.
- [x] `ImageCache::take_dirty(&mut self) -> bool` — returns current dirty flag and clears it atomically. Called by `Term::renderable_content_into()`.
- [x] `ImageCache::store(data: ImageData) -> Result<ImageId, ImageError>` — add image, evict LRU if over limit; return error if single image exceeds `max_single_image_bytes`
- [x] `ImageCache::place(placement: ImagePlacement)` — add placement
- [x] `ImageCache::remove_image(id: ImageId)` — remove image and all its placements
- [x] `ImageCache::remove_placement(image_id: ImageId, placement_id: u32)` — remove specific placement
- [x] `ImageCache::remove_by_position(col: usize, row: StableRowIndex)` — remove placements at cell
- [x] `ImageCache::placements_in_viewport(top_row: StableRowIndex, bottom_row: StableRowIndex) -> Vec<&ImagePlacement>` — placements visible in the given row range (returns `Vec` of refs because placements are filtered; alternatively return an iterator)
- [x] `ImageCache::evict_lru()` — remove least-recently-used image when over memory limit. Prefer images with zero placements first, then evict placed images by LRU order (Ghostty pattern).
- [x] `ImageCache::prune_scrollback(evicted_before: StableRowIndex)` — remove placements whose `cell_row` is before the eviction boundary
- [x] `ImageCache::clear()` — remove all images and placements (used by RIS and screen clear)
- [x] `ImageCache::remove_placements_in_region(top: StableRowIndex, bottom: StableRowIndex, left: Option<usize>, right: Option<usize>)` — remove placements overlapping a rectangular region (used by ED/EL erase operations)
- [x] `ImageError` enum: `OversizedImage`, `InvalidFormat`, `DecodeFailed(String)`, `MemoryLimitExceeded`
- [x] Scrollback interaction: placements use `StableRowIndex` so they scroll with text automatically

### Wiring into Term

- [x] Add `image_cache: ImageCache` and `alt_image_cache: ImageCache` fields to `Term<T>` in `oriterm_core/src/term/mod.rs` (one per screen buffer, matching Ghostty's per-screen `ImageStorage`)
- [x] `Term::image_cache(&self) -> &ImageCache` — returns the active screen's cache (primary or alt, based on `ALT_SCREEN` mode)
- [x] `Term::image_cache_mut(&mut self) -> &mut ImageCache` — mutable accessor
- [x] `Term::swap_alt()` / `swap_alt_clear()`: already swaps grids — must also swap `image_cache` and `alt_image_cache`. Add `mem::swap(&mut self.image_cache, &mut self.alt_image_cache)` to `toggle_alt_common()` in `oriterm_core/src/term/alt_screen.rs`
- [x] `Term::esc_reset_state()` (RIS): add `self.image_cache.clear()` and `self.alt_image_cache.clear()` in `oriterm_core/src/term/handler/esc.rs`
- [x] ED/EL erase operations must clear image placements in the erased region. **Design decision:** `Term<T>` clears placements after grid erase in the handler (not inside `Grid::erase_display()`). Grid remains image-unaware. Module boundary discipline: Grid (`grid/`) never imports image types; `Term` (`term/`) coordinates between Grid and ImageCache.
- [x] Scrollback eviction hook: when scrollback evicts rows, `Term<T>` calls `image_cache.prune_scrollback()` with the evicted boundary. Grid is image-unaware (same boundary discipline as erase). `Term<T>` checks `grid.total_evicted()` changes after operations that scroll (linefeed, `scroll_up`, resize) and prunes image placements accordingly. Grid already tracks `total_evicted` in `oriterm_core/src/grid/mod.rs:55`.
- [x] Export from `oriterm_core/src/lib.rs`: `pub mod image;` and re-export `ImageId`, `ImageCache`, `ImagePlacement`, `ImageError`

### Selection behavior over image regions

- [x] Text selection over cells covered by images: extract the underlying cell text (images do not replace cell content — they overlay it). Selection works normally; the image is visual-only.
- [x] If Kitty virtual placeholders (U+10EEEE) are in cells, selection should skip them (treat as empty). Add a `CellFlags::IMAGE_PLACEHOLDER` flag or check the character value directly.

### Tab/pane close cleanup

- [x] When a `Pane` is dropped (pane close), its `Term<T>` is dropped, which drops both `ImageCache` instances. No special cleanup needed — `Arc<Vec<u8>>` handles shared data refcounting. GPU textures are evicted separately (see 39.5).

### Tests

- [x] **Tests** (`oriterm_core/src/image/tests.rs`):
  - [x] Store/retrieve image data roundtrip
  - [x] Placement at cell position, query by viewport range
  - [x] Memory limit triggers LRU eviction (unused images evicted first)
  - [x] Remove by ID, by placement, by position
  - [x] `prune_scrollback` removes placements beyond eviction boundary
  - [x] `remove_placements_in_region` clears placements in rectangular area
  - [x] `clear()` removes everything
  - [x] Oversized single image rejected with `ImageError::OversizedImage`
  - [x] Corrupt image data returns `ImageError::DecodeFailed`
  - [x] Dirty flag set on mutation, cleared by `take_dirty()`

---

## 39.2 Kitty Graphics Protocol

**Implementation order:** All three VTE prerequisites (APC in 39.2, DCS dispatch in 39.3, OSC buffer resize in 39.4) are blocking and must be implemented before any protocol parsing work begins. Recommended order:
1. VTE APC support (39.2 prerequisite)
2. VTE DCS dispatch (39.3 prerequisite)
3. VTE OSC buffer resize (39.4 prerequisite)
4. Protocol parsing: 39.2 Kitty, 39.3 Sixel, 39.4 iTerm2 (can be parallelized)
5. GPU compositing (39.5)

VTE changes are in the library crate (`crates/vte`) which must be done before the consumer crate (`oriterm_core`).

The preferred modern image protocol. Transmission via APC sequences with chunked transfer, multiple placement modes, and animation support.

**File:** `oriterm_core/src/image/kitty/mod.rs` (types + re-exports), `oriterm_core/src/image/kitty/parse.rs` (command parsing), `oriterm_core/src/image/kitty/exec.rs` (command execution), `oriterm_core/src/image/kitty/tests.rs` (tests), `oriterm_core/src/term/handler/image.rs` (handler integration, split parsing from execution following Ghostty's `graphics_command.zig` / `graphics_exec.zig` pattern)

**Reference:** Kitty protocol spec, Ghostty `src/terminal/kitty/graphics_command.zig`

### VTE APC prerequisite (concrete sub-tasks)

The VTE crate must be extended to deliver APC data to the consumer. Currently, `SosPmApcString` in `crates/vte/src/lib.rs:182` calls `self.anywhere()` which discards all bytes. The fix:

- [x] **`crates/vte/src/lib.rs`** — Add `Perform` trait methods (default empty impls, no breakage):
  - [x] `fn apc_start(&mut self) {}` — called when `ESC _` (0x5F after ESC) transitions into APC state
  - [x] `fn apc_put(&mut self, byte: u8) {}` — called for each byte in the APC string body
  - [x] `fn apc_end(&mut self) {}` — called when ST (`ESC \` or 0x9C) terminates the APC string
- [x] **`crates/vte/src/lib.rs`** — Replace `SosPmApcString` handling:
  - [x] Introduce `State::ApcString` (separate from SOS/PM) or split the existing `SosPmApcString` state
  - [x] On entering APC state: call `performer.apc_start()`
  - [x] In APC state: for each byte, call `performer.apc_put(byte)` instead of discarding
  - [x] On ST terminator: call `performer.apc_end()`, transition to Ground
- [x] **`crates/vte/src/ansi.rs`** — Wire APC through the ansi `Processor`:
  - [x] Add `Handler` trait method: `fn apc_dispatch(&mut self, _payload: &[u8]) {}` (raw APC passthrough — consumer dispatches by first byte)
  - [x] In `Processor`'s `Perform` impl: implement `apc_start`/`apc_put`/`apc_end` to buffer APC data, then on `apc_end` dispatch to `handler.apc_dispatch(&payload)`
  - [x] Add `apc_buf: Vec<u8>` field to `ProcessorState` (persists across `advance` calls for chunked input)
  - [x] Cap APC buffer at 32 MB to prevent OOM from malicious input. Discard oversized APC sequences silently.

### Kitty command parsing

- [x] APC sequence format: `ESC _ G <control-data> ; <payload> ESC \` (APC start = `ESC _`, ST = `ESC \`)
  - [x] Control data: key=value pairs separated by commas (keys are single chars, values are unsigned integers or strings)
  - [x] Payload: base64-encoded image data (or empty for non-transmission commands)
- [x] `KittyCommand` struct — parsed representation of one Kitty graphics command:
  - [x] `action: KittyAction` — `Transmit`, `TransmitAndPlace`, `Place`, `Delete`, `Frame`, `Animate`, `Query`
  - [x] `transmission: Option<KittyTransmission>` — format (`f=`), method (`t=`), compression (`o=`)
  - [x] `image_id: Option<u32>`, `image_number: Option<u32>`, `placement_id: Option<u32>`
  - [x] `source_rect: Option<Rect>`, `display_cells: Option<(u32, u32)>`
  - [x] `z_index: i32`, `cursor_movement: bool`
  - [x] `quiet: u8` — 0=normal, 1=suppress OK responses, 2=suppress all responses
  - [x] `payload: Vec<u8>` — accumulated base64-decoded data
- [x] `parse_kitty_command(raw: &[u8]) -> Result<KittyCommand, KittyError>` — parse the APC body (after `G` prefix byte)

### Transmission

- [x] Transmission actions (`a=` key):
  - [x] `t` (transmit): upload image data
  - [x] `T` (transmit + display): upload and immediately place
  - [x] `p` (put/place): place previously uploaded image
  - [x] `d` (delete): delete image/placement
  - [ ] `f` (frame): animation frame operations <!-- blocked-by:39.5 -->
  - [ ] `a` (animate): animation control <!-- blocked-by:39.5 -->
  - [x] `q` (query): query support without side effects
- [x] Transmission formats (`f=` key):
  - [x] 24 (RGB), 32 (RGBA), 100 (PNG compressed)
- [x] Transmission methods (`t=` key):
  - [x] `d` (direct): payload contains base64 image data
  - [x] `f` (file): payload contains file path. **Security:** path traversal (`..`) rejected.
  - [x] `t` (temp file): payload contains temp file path (deleted after read). Same security as `f`.
  - [x] `s` (shared memory): payload contains shm name. **Platform note:** Windows shared memory uses `OpenFileMappingA`/`MapViewOfFile` (not POSIX `shm_open`). Stubbed with error response since shm is rarely used over the wire.
- [x] Chunked transfer:
  - [x] `m=1`: more chunks follow
  - [x] `m=0`: final chunk (or single-chunk transfer)
  - [x] Accumulate chunks into complete payload before decoding
  - [x] Max accumulated chunk payload size = `max_single_image_bytes` from ImageCache config. Discard transmission if exceeded (prevents OOM from unbounded chunked transfer).
  - [x] Store in-progress chunked transmission as `loading: Option<LoadingImage>` field on `Term` (Ghostty pattern). `LoadingImage` holds accumulated payload, image ID, and transmission metadata.

### Placement

- [x] Placement parameters:
  - [x] `i=` image ID, `p=` placement ID
  - [x] `s=`, `v=` source rect size (pixels)
  - [x] `c=`, `r=` display size (cells)
  - [x] `x=`, `y=` source rect offset (pixels)
  - [x] `X=`, `Y=` cell offset within placement cell
  - [x] `z=` z-index (layer ordering)
  - [x] `C=1` cursor movement suppression — when set, cursor does not advance past image
  - [x] `U=1` unicode placeholder mode — virtual placement rendered via U+10EEEE chars in cells (stretch goal, used by some programs for layout stability)

### Delete operations

- [x] Delete operations (`d=` key with `a=d`). Convention: lowercase = delete placements only, UPPERCASE = delete image data + all placements:
  - [x] `a`/`A` (all visible placements / all data), `i`/`I` (by image ID)
  - [x] `p`/`P` (by placement ID), `c`/`C` (at cursor column), `r`/`R` (by cell range), `x`/`X` (by column), `y`/`Y` (by row), `z`/`Z` (by z-index)
  - [x] `n`/`N` (newest by image number — rarely used, stubbed with debug log)

### Animation

- [ ] Animation support (Kitty `a=f` frame and `a=a` animate actions): <!-- blocked-by:39.5 -->
  - [ ] Frame composition modes (`o=` key): overwrite entire image, blend (alpha composite) new frame over existing
  - [ ] Frame timing via `z=` key (duration in ms per frame)
  - [ ] Animation control (`a=a`): start playback, stop playback, set loop count
  - [ ] Store frames as separate `ImageData` entries sharing the same `ImageId` base, with per-frame pixel data and timing
  - Note: Frame/Animate actions are accepted and logged. Full animation requires GPU compositing (39.5).

### Response

- [x] Response format: APC `G` response — `\x1b_G<response-data>\x1b\\`. Sent for success/failure when `q != 2`. When `q=1`, suppress OK responses (only send errors). When `q=2`, suppress all responses.
- [x] Send response via `Event::PtyWrite` (same pattern as DA, DSR, DECRQM responses in `oriterm_core/src/term/handler/status.rs`)

### Handler wiring

- [x] New file: `oriterm_core/src/term/handler/image.rs`
- [x] Add `mod image;` to `oriterm_core/src/term/handler/mod.rs`
- [x] Implement kitty graphics command execution in `Term<T>`:
  - [x] `fn handle_kitty_graphics(&mut self, payload: &[u8])` — parse `KittyCommand`, dispatch by action
  - [x] `Transmit`: decode image data (PNG/RGBA), store in `self.image_cache_mut()`. PNG decode happens synchronously (acceptable for v1 — most images are small; consider background thread for large images later).
  - [x] `Place`: create `ImagePlacement` at current cursor position, add to cache, advance cursor (unless `C=1`)
  - [x] `TransmitAndPlace`: combine transmit + place in one step
  - [x] `Delete`: dispatch to appropriate `ImageCache::remove_*` method based on delete key value
  - [x] `Query`: send OK response without modifying state

### Error handling

- [x] Invalid base64 payload → log warning, send error response if `q != 2`
- [x] Unsupported format value → log warning, send `EINVAL` response
- [x] PNG/RGBA decode failure → log warning, send error response
- [x] Image too large (exceeds `max_single_image_bytes`) → send `ENOMEM` response
- [x] Unknown image ID for placement → send `ENOENT` response
- [x] All errors are non-fatal — never panic, never corrupt terminal state

### DA (Device Attributes) update

- [x] Update `status_identify_terminal()` in `oriterm_core/src/term/handler/status.rs` to include sixel graphics support indicator (DA1 attribute 4). Note: Kitty graphics has no DA attribute — programs probe via `a=q` query.

### Tests

- [x] **Tests** (`oriterm_core/src/image/kitty/tests.rs`):
  - [x] Parse control data key-value pairs (single key, multiple keys, missing value, unknown key)
  - [x] Single-chunk PNG transmission + placement
  - [x] Single-chunk RGBA transmission (f=32, raw pixel data)
  - [x] Multi-chunk transmission accumulates correctly
  - [x] Chunked transfer exceeding buffer limit is rejected
  - [x] Delete by image ID removes correct image and placements
  - [x] Delete by placement ID removes only that placement
  - [x] Delete uppercase variants also remove image data
  - [x] Placement respects cell position and span
  - [x] Cursor movement suppression (`C=1`) leaves cursor unchanged
  - [x] Response includes correct image ID and status
  - [x] Invalid base64 produces error response
  - [x] Unknown image ID for placement produces `ENOENT`

---

## 39.3 Sixel Graphics

Legacy image protocol using DCS sequences. Widely supported by older terminals and tools.

**File:** `oriterm_core/src/image/sixel/mod.rs` (sixel decoder + state machine), `oriterm_core/src/image/sixel/tests.rs` (tests)

**Reference:** WezTerm `term/src/terminalstate/sixel.rs` (decode + state), `wezterm-escape-parser/src/parser/sixel.rs` (low-level parser), VT340 programmer reference

**VTE note:** Sixel uses DCS sequences. The low-level VTE `Perform` trait already has `hook`/`put`/`unhook` for DCS passthrough, but the ansi layer (`crates/vte/src/ansi.rs`) currently logs them as `[unhandled hook]` at line 1347 without dispatching to the `Handler` trait. The ansi layer must be extended to dispatch DCS with action `'q'` (sixel introducer) to new handler methods.

### VTE DCS dispatch prerequisite (concrete sub-tasks)

- [x] **`crates/vte/src/ansi.rs`** — Wire DCS `hook`/`put`/`unhook` through the ansi `Processor`:
  - [x] Currently at line 1347, `hook()` logs `[unhandled hook]` — must check `action` param for `'q'` (sixel introducer)
  - [x] Add `Handler` trait methods (default empty impls):
    - [x] `fn sixel_start(&mut self, _params: &[u16]) {}` — called on DCS hook with action `'q'`, receives P1/P2/P3 params
    - [x] `fn sixel_put(&mut self, _byte: u8) {}` — called for each byte of sixel data
    - [x] `fn sixel_end(&mut self) {}` — called on DCS unhook (ST terminator)
  - [x] In `Processor::hook()`: check if action is `'q'` and route to `handler.sixel_start(params)`; for other DCS sequences, continue logging as unhandled
  - [x] In `Processor::put()`: if sixel is active, call `handler.sixel_put(byte)`
  - [x] In `Processor::unhook()`: if sixel is active, call `handler.sixel_end()`
  - [x] Add `dcs_state: DcsState` enum field to `ProcessorState` to track whether current DCS is sixel or unknown (needed for `put`/`unhook` dispatch)

### Sixel state machine

Sixel parsing is streaming (byte-by-byte via `put`), so a state machine struct accumulates pixels incrementally. If `sixel/mod.rs` approaches 400 lines during implementation, proactively split into `sixel/parser.rs` (state machine + feed) and `sixel/palette.rs` (color palette + HLS/RGB conversion).

- [x] `SixelParser` struct (in `oriterm_core/src/image/sixel/mod.rs`):
  - [x] `width: usize`, `height: usize` — current image dimensions (grow as data arrives)
  - [x] `pixels: Vec<u8>` — RGBA pixel buffer (grows dynamically)
  - [x] `palette: [Rgb; 256]` — color palette (initialized to VT340 defaults)
  - [x] `current_color: u8` — selected palette index
  - [x] `x: usize`, `y: usize` — current drawing position
  - [x] `bg_mode: SixelBgMode` — `DeviceDefault`, `NoChange`, `SetToBg` (from P2 param)
  - [x] `max_width: usize`, `max_height: usize` — limits to prevent OOM (e.g., 10000x10000 max)
- [x] `SixelParser::new(params: &[u16]) -> Self` — initialize from DCS P1/P2/P3
- [x] `SixelParser::feed(&mut self, byte: u8)` — process one byte of sixel data. No allocation per byte — pixel buffer grows by doubling, palette mutations are in-place.
- [x] `SixelParser::finish(self) -> Result<(Vec<u8>, u32, u32), ImageError>` — finalize and return RGBA pixels + dimensions

### DCS sequence parsing

- [x] `DCS P1 ; P2 ; P3 q <sixel-data> ST`
  - [x] P1: pixel aspect ratio (0 or 2:1 default)
  - [x] P2: background select (0=device default, 1=no change, 2=set to bg)
  - [x] P3: horizontal grid size (ignored, use 0)

### Sixel data decoding

- [x] Character range: 0x3F–0x7E (63–126), subtract 0x3F for 6-bit column
- [x] Each character encodes 6 vertical pixels (1 column × 6 rows)
- [x] `$` (carriage return): reset x to left margin
- [x] `-` (line feed): move down 6 pixel rows, reset x
- [x] `!<count><char>` (repeat): repeat character N times. Clamp repeat count to `max_width` to prevent OOM from malicious input.
- [x] `#<color>` (color): select palette index
- [x] `#<idx>;2;<r>;<g>;<b>` (color define): define RGB color (0-100 range, scale to 0-255)
- [x] `#<idx>;1;<h>;<l>;<s>` (color define): define HLS color
  - [x] HLS to RGB conversion: H in 0-360, L in 0-100, S in 0-100. Use standard HSL-to-RGB algorithm (VT340 HLS is H=hue, L=lightness, S=saturation — same as HSL but parameter order is H,L,S not H,S,L)

### Sixel to RGBA conversion

- [x] Build palette from color definitions (up to 256 colors, ignore indices >= 256)
- [x] Decode sixel columns into pixel buffer
- [x] Convert palette-indexed pixels to RGBA
- [x] Transparent pixels: when `bg_mode == NoChange` (P2=1), undrawn pixels are fully transparent (alpha=0); when `SetToBg` (P2=2), fill with terminal background color

### Placement

- [x] Image placed at current cursor position
- [x] Cursor advances past image. Two modes control cursor position after sixel: DECSET 80 (sixel scrolling — cursor moves to line below image) and DECSET 8452 (sixel cursor right — cursor moves to column right of image)
- [x] Image occupies grid cells based on pixel size / cell size
- [x] Store as `ImagePlacement` in `ImageCache` (same path as Kitty/iTerm2)

### Handler wiring

- [x] Add `sixel_parser: Option<SixelParser>` field to `Term<T>` (active during DCS sixel sequence)
- [x] `Term::sixel_start(params)`: create `SixelParser::new(params)`, store in `self.sixel_parser`
- [x] `Term::sixel_put(byte)`: call `self.sixel_parser.as_mut()`. If `None`, ignore byte (malformed sequence — no `unwrap()`, no panic).
- [x] `Term::sixel_end()`: call `self.sixel_parser.take()`. If `None`, return early (malformed sequence). On `Some`, call `finish()`, store decoded image in `ImageCache`, create placement at cursor, advance cursor per mode 80/8452 settings. No `unwrap()`.
- [x] Wire these in `handler/image.rs` alongside Kitty graphics handler

### DECRQM for sixel mode

- [x] Add mode 80 (`SixelScrolling`) and mode 8452 (`SixelCursorRight`) to `NamedPrivateMode` enum in `crates/vte/src/ansi.rs` (add arms in the `PrivateMode::from()` match)
- [x] Handle DECSET/DECRST for these modes in `Term::apply_decset()`/`apply_decrst()` in `oriterm_core/src/term/handler/modes.rs`
- [x] Report mode status in `status_report_private_mode()` in `oriterm_core/src/term/handler/status.rs`

### Tests

- [x] **Tests** (`oriterm_core/src/image/sixel/tests.rs`):
  - [x] Decode simple sixel: single color, known pattern (e.g., 1x6 column)
  - [x] Repeat operator produces correct pixel count
  - [x] Repeat operator clamped at max_width
  - [x] Color palette definition (RGB mode, 0-100 → 0-255 scaling)
  - [x] Color palette definition (HLS mode)
  - [x] Multi-row sixel (line feed advances by 6 pixels)
  - [x] Cursor position after sixel display (both mode 80 on and off)
  - [x] Background select mode: transparent pixels when P2=1
  - [x] Oversized sixel image rejected (exceeds max dimensions)
  - [x] Palette index >= 256 ignored gracefully

---

## 39.4 iTerm2 Image Protocol

OSC-based image protocol used by iTerm2 and supported by many tools via `imgcat`.

**File:** `oriterm_core/src/image/iterm2/mod.rs` (iTerm2 parser + placement), `oriterm_core/src/image/iterm2/tests.rs` (tests)

**Reference:** iTerm2 image protocol spec, WezTerm `term/src/terminalstate/iterm.rs`

### VTE OSC buffer size prerequisite

`MAX_OSC_RAW` is currently 1024 bytes (`crates/vte/src/lib.rs:46`). iTerm2 image payloads can be multi-megabyte. This is a hard blocker: without this change, iTerm2 images > 1 KB silently fail.

- [x] **`crates/vte/src/lib.rs`** — OSC buffer capacity already sufficient for iTerm2:
  - [x] `osc_raw` is already `Vec<u8>` (unbounded) under `std` feature. Added `MAX_OSC_RAW_STD` (64 MiB) cap to prevent OOM from malicious input while supporting multi-megabyte iTerm2 image payloads. The `no_std` path retains the fixed-size `ArrayVec` with `OSC_RAW_BUF_SIZE` const generic.

### OSC 1337 parsing

- [x] `OSC 1337 ; File=[args] : <base64-data> ST`
- [x] Arguments (semicolon-separated key=value pairs within the `File=` section):
  - [x] `name=<base64>` — filename (base64-encoded)
  - [x] `size=<bytes>` — file size hint (informational only)
  - [x] `width=<spec>` — display width (N, Npx, N%, auto)
  - [x] `height=<spec>` — display height (same format)
  - [x] `preserveAspectRatio=0|1` — maintain aspect ratio (default: 1)
  - [x] `inline=0|1` — display inline (1) or as download (0)
- [x] OSC dispatch routing: in `osc_dispatch()` in `crates/vte/src/ansi.rs`, add case for `1337` parameter to call a new `Handler` trait method `fn iterm2_file(&mut self, _params: &[&[u8]]) {}`

### Image decode

- [x] Base64-decode payload
- [x] Detect format from magic bytes (PNG, JPEG, GIF, BMP, WebP)
- [x] Decode to RGBA via `image` crate
- [x] **Cargo.toml change:** Add `image` crate as optional dependency to `oriterm_core/Cargo.toml` with features: `["png", "jpeg", "gif", "bmp", "webp"]`. Gate behind `image-protocol` cargo feature: `image-protocol = ["dep:image"]` (enabled by default).
- [x] Reject payloads exceeding `max_single_image_bytes` from ImageCache config. Log warning and discard.
- [x] If `image::load_from_memory()` fails, log warning and discard (no crash, no terminal state corruption). **Known v1 limitation:** Image decoding is synchronous in the VTE handler path. Large images (multi-megabyte) can block the event loop for 10-100ms. Background thread decoding is a future optimization.

### Placement

- [x] Width/height parsing: pixel (`Npx`), cell count (`N`), percentage (`N%`), auto
- [x] Auto: use image's native size, clamped to terminal width
- [x] Percentage: relative to terminal dimensions (width% of terminal pixel width, height% of terminal pixel height)
- [x] Cell count: N cells wide/tall (requires `cell_pixel_width`/`cell_pixel_height` — see "Cell dimensions in Term" subsection below)
- [x] Place at current cursor position (convert to `StableRowIndex`)
- [x] Cursor advances below image (moves down by the number of cell rows the image occupies)
- [x] Store as `ImagePlacement` in `ImageCache` (same path as Kitty/Sixel)
- [x] `inline=0`: download — store file, don't display (stretch goal — send `Event::FileDownload(name, data)`)

### Cell dimensions in Term

- [x] Add `cell_pixel_width: u16` and `cell_pixel_height: u16` fields to `Term<T>` (set during resize, defaulting to reasonable values like 8x16)
- [x] Update these in `Term::resize()` or via a new `Term::set_cell_dimensions(w, h)` method called from the GUI layer after font metrics are known
- [x] These are also needed by Kitty protocol for `c=`/`r=` cell-count sizing and by Sixel for cell-to-pixel mapping

### GIF animation

- [ ] If decoded image is a GIF with multiple frames: extract all frames and store as an animated image in `ImageCache` <!-- blocked-by:39.5 -->
- [ ] Animation frames share the same `ImageId` but have per-frame pixel data and timing <!-- blocked-by:39.5 -->
- [ ] This reuses the animation infrastructure from 39.5 <!-- blocked-by:39.5 -->

### Tests

- [x] **Tests** (`oriterm_core/src/image/iterm2/tests.rs`):
  - [x] Parse width/height specs: "auto", "80", "100px", "50%"
  - [x] Base64 payload decoded correctly (PNG)
  - [x] Aspect ratio preserved when `preserveAspectRatio=1`
  - [x] Aspect ratio not preserved when `preserveAspectRatio=0`
  - [x] Image placed at cursor position with correct cell span
  - [x] Cursor advances below image by correct number of lines
  - [x] Oversized payload rejected
  - [x] Invalid base64 handled gracefully (no crash)
  - [x] Unknown image format handled gracefully (no crash)
  - [x] `inline=0` does not display image

---

## 39.5 Image Rendering + GPU Compositing

Render cached images as GPU textures composited into the terminal frame.

**File:** `oriterm/src/gpu/image_render/mod.rs` (new directory module), `oriterm/src/gpu/image_render/tests.rs` (tests), `oriterm/src/gpu/shaders/image.wgsl` (new shader), `oriterm/src/gpu/pipeline/mod.rs` (extend), `oriterm/src/gpu/pipelines.rs` (extend `GpuPipelines` struct)

**Performance note:** This is the highest-risk section. It touches the hot render path (`record_draw_passes` in `window_renderer/render.rs`). The image pipeline adds a new bind group layout (per-image texture), meaning per-image draw calls (not instanced like cell backgrounds). For N visible images, this is N additional draw calls per frame. Profile this and consider texture arrays or atlasing if N > ~10 becomes common.

### RenderableContent bridge

- [ ] Add `images: Vec<RenderablePlacement>` field to `RenderableContent` in `oriterm_core/src/term/renderable/mod.rs`
- [ ] Add `images_dirty: bool` field to `RenderableContent` — set from `ImageCache::take_dirty()` during `renderable_content_into()`. The GPU layer uses this to know when to re-upload textures. One-way data flow: dirty state flows downstream through `RenderableContent`, not via callback into core.
- [ ] `RenderablePlacement` struct (new, in `renderable/mod.rs`):
  - [ ] `image_id: ImageId` — for GPU texture lookup
  - [ ] `viewport_x: f32`, `viewport_y: f32` — pixel position in viewport (top-left corner)
  - [ ] `display_width: f32`, `display_height: f32` — size in pixels
  - [ ] `source_x: f32`, `source_y: f32` — UV source rect origin (0.0–1.0)
  - [ ] `source_w: f32`, `source_h: f32` — UV source rect size (0.0–1.0)
  - [ ] `z_index: i32`
  - [ ] `opacity: f32` — for animation fade transitions (default 1.0)
- [ ] In `Term::renderable_content_into()`: query `self.image_cache().placements_in_viewport()`, convert `StableRowIndex` to viewport pixel positions, push into `out.images`
- [ ] Populate `out.image_data: Vec<(ImageId, Arc<Vec<u8>>, u32, u32)>` with `(id, pixel_data, width, height)` for all images referenced by visible placements. Always populate (not just on dirty) because viewport scrolling may bring previously off-screen images into view without `ImageCache::dirty` being set. The GPU layer's `ensure_uploaded()` deduplicates by `ImageId`. The `Arc` clone is cheap (refcount increment, no data copy).
- [ ] `PaneSnapshot` extension: add `images: Vec<WirePlacement>` for daemon-mode rendering (with `WirePlacement` mirroring `RenderablePlacement` but using serializable types). **Note:** Daemon mode image support is a significant complexity multiplier (multi-megabyte payloads per snapshot). Recommended: defer daemon image support — get local image rendering working first, then extend to daemon mode in a follow-up.

### FrameInput bridge

- [ ] Add `images: Vec<RenderablePlacement>` field to `FrameInput` in `oriterm/src/gpu/frame_input/mod.rs`
- [ ] Add `image_data: Vec<(ImageId, Arc<Vec<u8>>, u32, u32)>` field to `FrameInput` — pixel data for GPU texture upload
- [ ] Add `images_dirty: bool` field to `FrameInput` — propagated from `RenderableContent`
- [ ] In `extract_frame_from_snapshot()` in `oriterm/src/gpu/extract/from_snapshot/mod.rs`: convert `WirePlacement` to `RenderablePlacement` and populate `frame.images`. For daemon mode, `WirePlacement` must carry pixel data (serialized) since the daemon has the image cache, not the client.

### Image texture management

- [ ] `ImageTextureCache` struct (in `oriterm/src/gpu/image_render/mod.rs`):
  - [ ] `textures: HashMap<ImageId, GpuImageTexture>` — uploaded textures keyed by image ID
  - [ ] `gpu_memory_used: usize` — total GPU texture bytes
  - [ ] `gpu_memory_limit: usize` — configurable (default: 512 MB, separate from CPU-side ImageCache memory limit)
  - [ ] `GpuImageTexture` struct: `texture: wgpu::Texture`, `view: wgpu::TextureView`, `bind_group: wgpu::BindGroup`, `size_bytes: usize`, `last_frame: u64`
- [ ] Separate `wgpu::Texture` per image (not atlas — images vary wildly in size; atlas wastes space for large images)
- [ ] Upload decoded RGBA data as `Rgba8UnormSrgb` texture
- [ ] Lazy upload: only upload to GPU when image enters viewport
- [ ] Evict GPU texture when image scrolls far out of viewport (LRU by `last_frame` counter)
- [ ] `ImageTextureCache::ensure_uploaded(gpu: &GpuState, id: ImageId, data: &[u8], w: u32, h: u32) -> &GpuImageTexture` — upload if not present, update `last_frame`
- [ ] `ImageTextureCache::evict_unused(current_frame: u64, threshold: u64)` — evict textures not used in last N frames

**Texture upload timing:** All `ensure_uploaded` calls happen during the prepare phase (before `render_frame`). The render pass only reads from pre-uploaded textures. No GPU resource creation during render pass recording. This matches the existing pattern where glyph atlas uploads happen in prepare, not during draw.

### Image render pipeline

- [ ] New WGSL shader: `oriterm/src/gpu/shaders/image.wgsl`
  - [ ] Vertex shader: transform image quad from pixel coords to clip space using `screen_size` uniform
  - [ ] Fragment shader: sample image texture, output with alpha blending
  - [ ] Vertex attributes: `position: vec2<f32>`, `uv: vec2<f32>`, `opacity: f32`
- [ ] New render pipeline: `create_image_pipeline()` in `oriterm/src/gpu/pipeline/mod.rs`
  - [ ] Uses `uniform_layout` (group 0, screen_size) + new `image_texture_layout` (group 1, per-image texture + sampler)
  - [ ] Alpha blending enabled (src_alpha, one_minus_src_alpha)
- [ ] **`GpuPipelines` extension** in `oriterm/src/gpu/pipelines.rs`:
  - [ ] Add `image_pipeline: RenderPipeline` field
  - [ ] Add `image_texture_layout: BindGroupLayout` field
  - [ ] Initialize in `GpuPipelines::new()`
- [ ] **`WindowRenderer` extension** in `oriterm/src/gpu/window_renderer/mod.rs`:
  - [ ] Add `image_texture_cache: ImageTextureCache` field
  - [ ] Add `image_instance_buffer: wgpu::Buffer` (or reusable Vec) for image quad vertices
- [ ] Draw ordering in `render_frame()`:
  - [ ] Pass 1: BG pipeline (cell backgrounds)
  - [ ] Pass 2: Image pipeline for z < 0 images (below text)
  - [ ] Pass 3: FG pipelines (text glyphs)
  - [ ] Pass 4: Image pipeline for z >= 0 images (above text)
- [ ] Each image is rendered as a textured quad (4 vertices, 6 indices or triangle strip)
- [ ] **PreparedFrame extension:** Add `image_quads_below: Vec<ImageQuad>` and `image_quads_above: Vec<ImageQuad>` to the prepared frame struct, split by z-index

### Cell interaction

- [ ] Cells covered by an image: still render text on top (for z < 0 images)
- [ ] Cells covered by z >= 0 images: image obscures text
- [ ] Background color: use cell's bg color behind transparent image regions (z < 0 case)
- [ ] Cell backgrounds under z < 0 images: render normally (image overlays on top of bg but below text)

### Scrolling

- [ ] Images scroll with text (placement positions use `StableRowIndex` → convert to viewport pixel Y using `stable_row_base` and `display_offset`)
- [ ] Partially visible images clipped at viewport boundaries (GPU scissor rect or UV clamping)
- [ ] Smooth scroll offset applied to image positions (if smooth scrolling is implemented)

### Terminal resize behavior
- [ ] Image placements use `StableRowIndex` + column positions — these are stable across resize
- [ ] Cell-count sizing (`c=`/`r=` in Kitty): the display pixel size changes when cell dimensions change. Recalculate pixel dimensions from cell counts on resize.
- [ ] Pixel-sized placements: remain at their pixel size (don't scale with cell size)
- [ ] Grid reflow may change which row a placement is on — `StableRowIndex` remains valid but the visual position shifts. This is acceptable (matches Ghostty/WezTerm behavior).

### Animation

- [ ] Timer-driven frame switching for animated images (Kitty `a=animate` and GIF multi-frame)
- [ ] `AnimationState` struct: `current_frame: usize`, `frame_timer: Instant`, `frame_durations: Vec<Duration>`
- [ ] Only animate images in viewport (save CPU/GPU) — check during frame preparation
- [ ] Configurable: `terminal.image_animation = true | false` (default: true)
- [ ] When animation is disabled, show first frame only

### Config integration

- [ ] Config keys:
  ```toml
  [terminal]
  image_protocol = true           # enable/disable all image protocols
  image_memory_limit = 335544320  # 320 MB default (CPU-side image cache)
  image_gpu_memory_limit = 536870912  # 512 MB default (GPU texture cache)
  image_animation = true
  image_max_single_size = 67108864  # 64 MB default (max single image)
  ```
- [ ] Config wiring path:
  - [ ] Config struct: add fields to the terminal config section
  - [ ] On config load/reload: pass limits to `ImageCache::set_memory_limit()` and `ImageTextureCache::set_memory_limit()`
  - [ ] `image_protocol = false`: skip all image parsing in VTE handler (early return in `handle_kitty_graphics`, `sixel_start`, `iterm2_file`)
  - [ ] On config reload (hot reload): if limit decreased, trigger eviction immediately

### Damage tracking interaction

- [ ] When `ImageCache::dirty` is `true`, mark all lines overlapping image placements as dirty, or mark the entire viewport dirty (simpler, acceptable since image changes are infrequent)
- [ ] `Term::renderable_content_into()` calls `image_cache.take_dirty()` and sets `images_dirty` on `RenderableContent` so the GPU layer knows textures need re-upload. One-way data flow: the GPU layer never reaches back into `ImageCache`.

### Tests

- [ ] **Tests** (`oriterm/src/gpu/image_render/tests.rs`):
  - [ ] Image texture uploads to GPU correctly (mock or headless wgpu)
  - [ ] Image at z=-1 is in `image_quads_below` list
  - [ ] Image at z=1 is in `image_quads_above` list
  - [ ] Image scrolls with content (viewport Y changes with display_offset)
  - [ ] Image clipped at viewport boundary (UV coords adjusted)
  - [ ] GPU memory limit evicts oldest textures
  - [ ] Config `image_protocol = false` produces no image quads
  - [ ] Resize recalculates cell-count-based placement pixel dimensions

---

## 39.6 Section Completion

### Sync points — all locations that must be updated together

**VTE crate changes** (`crates/vte/`):
- [ ] `src/lib.rs` — `Perform` trait: `apc_start`, `apc_put`, `apc_end` methods; `SosPmApcString` → `ApcString` state split
- [ ] `src/ansi.rs` — `Handler` trait: `kitty_graphics_command`, `sixel_start`, `sixel_put`, `sixel_end`, `iterm2_file` methods
- [ ] `src/ansi.rs` — `Processor` struct: `apc_buf`, `dcs_state` fields; `Perform` impl for APC and DCS dispatch
- [ ] `src/ansi.rs` — `MAX_OSC_RAW` increase or Vec migration for iTerm2 support
- [ ] `src/ansi.rs` — `NamedPrivateMode`: add sixel modes (80, 8452)

**oriterm_core changes** (`oriterm_core/src/`):
- [ ] `image/mod.rs` — new module: `ImageId`, `ImageData`, `ImagePlacement`, `ImageCache`, `ImageError`
- [ ] `image/cache.rs` — `ImageCache` implementation
- [ ] `image/decode.rs` — format detection + RGBA decode
- [ ] `image/kitty/mod.rs` — Kitty types + re-exports + `#[cfg(test)] mod tests;`
- [ ] `image/kitty/parse.rs` — Kitty command parser
- [ ] `image/kitty/exec.rs` — Kitty command execution
- [ ] `image/kitty/shm.rs` — shared memory (platform-specific, `#[cfg()]` at module level)
- [ ] `image/kitty/tests.rs` — Kitty protocol tests
- [ ] `image/sixel/mod.rs` — Sixel decoder + state machine + `#[cfg(test)] mod tests;`
- [ ] `image/sixel/tests.rs` — Sixel decoder tests
- [ ] `image/iterm2/mod.rs` — iTerm2 parser + placement + `#[cfg(test)] mod tests;`
- [ ] `image/iterm2/tests.rs` — iTerm2 protocol tests
- [ ] `term/mod.rs` — `Term<T>`: add `image_cache`, `alt_image_cache`, `sixel_parser`, `cell_pixel_width`, `cell_pixel_height` fields; update `new()`, `renderable_content_into()`
- [ ] `term/alt_screen.rs` — `toggle_alt_common()`: swap image caches
- [ ] `term/handler/mod.rs` — add `mod image;`
- [ ] `term/handler/image.rs` — new file: Kitty/Sixel/iTerm2 handler dispatch. If all three protocol handlers exceed 500 lines combined, restructure as `handler/image/mod.rs` with `kitty.rs`, `sixel.rs`, `iterm2.rs` sub-handlers.
- [ ] `term/handler/esc.rs` — `esc_reset_state()`: clear image caches
- [ ] `term/handler/status.rs` — `status_identify_terminal()`: update DA response for graphics capability
- [ ] `term/handler/modes.rs` — `apply_decset`/`apply_decrst`: handle sixel modes 80, 8452
- [ ] `term/renderable/mod.rs` — `RenderableContent`: add `images`, `image_data`, `images_dirty` fields; add `RenderablePlacement` struct
- [ ] `lib.rs` — add `pub mod image;` and re-exports
- [ ] `Cargo.toml` — add `image` crate as optional dependency with format features `["png", "jpeg", "gif", "bmp", "webp"]`; add `image-protocol = ["dep:image"]` feature (default-enabled)

**oriterm GPU changes** (`oriterm/src/gpu/`):
- [ ] `image_render/mod.rs` — new directory module: `ImageTextureCache`, `GpuImageTexture`, `ImageQuad` + `#[cfg(test)] mod tests;`
- [ ] `image_render/tests.rs` — GPU image render tests
- [ ] `shaders/image.wgsl` — new shader
- [ ] `pipeline/mod.rs` — `create_image_pipeline()` function
- [ ] `pipelines.rs` — `GpuPipelines`: add `image_pipeline`, `image_texture_layout` fields
- [ ] `window_renderer/mod.rs` — `WindowRenderer`: add `image_texture_cache`, `image_instance_buffer` fields
- [ ] `window_renderer/render.rs` — `render_frame()`: add image draw passes (below-text and above-text)
- [ ] `frame_input/mod.rs` — `FrameInput`: add `images`, `image_data`, `images_dirty` fields
- [ ] `extract/from_snapshot/mod.rs` — `extract_frame_from_snapshot()`: convert `WirePlacement` to `RenderablePlacement`
- [ ] `prepared_frame/mod.rs` — add `image_quads_below`, `image_quads_above` fields
- [ ] `mod.rs` — add `pub mod image_render;`

**oriterm_mux changes** (`oriterm_mux/src/`):
- [ ] `protocol/snapshot.rs` — `PaneSnapshot`: add `images: Vec<WirePlacement>` field; add `WirePlacement` struct
- [ ] Snapshot extraction: include image placements when serializing pane state for daemon mode

### Completion checklist

- [ ] All 39.1–39.5 items complete
- [ ] Kitty Graphics Protocol: transmit, place, delete, animate, query, response
- [ ] Sixel: decode and render legacy sixel images, HLS palette support
- [ ] iTerm2: `imgcat`-compatible inline image display, all sizing modes
- [ ] GPU compositing: images render at correct z-order with text
- [ ] Memory management: configurable CPU + GPU limits with LRU eviction
- [ ] Error handling: corrupt images, oversized images, invalid protocol commands — all handled gracefully
- [ ] Scrolling: images scroll with text, clip at viewport
- [ ] Resize: images survive terminal resize, cell-count sizing recalculated
- [ ] Animation: timer-driven frame switching for animated images
- [ ] Config: all image settings hot-reloadable
- [ ] Selection: text selection works over image regions (underlying cell text extracted)
- [ ] Screen clear (ED/EL): erased regions also clear image placements
- [ ] Alt screen: image caches swap with grid on alt screen enter/exit
- [ ] RIS: full reset clears all image caches
- [ ] Tab close: image resources cleaned up (Drop-based)
- [ ] Daemon mode: images included in `PaneSnapshot` for remote rendering
- [ ] `./build-all.sh` — builds cleanly
- [ ] `./test-all.sh` — all image protocol tests pass
- [ ] `./clippy-all.sh` — no warnings

**Exit Criteria:** `kitty icat`, `imgcat`, `viu`, `timg`, and sixel-based tools display images inline in the terminal. Images composite correctly with text, scroll with content, and respect memory limits. Corrupt/oversized images are rejected gracefully. Image resources are cleaned up on tab close, screen clear, and terminal reset.
