//! Tests for `ImageCache` — lifecycle operations, resize handling,
//! and reflow remapping.
//!
//! Unit tests for cache basics (store, evict, viewport, dirty flag,
//! animation) live in `image/tests.rs`. This file covers the lifecycle
//! methods in `cache/lifecycle.rs` plus the `on_resize` and
//! `remap_placements` behavior added in section 07.
