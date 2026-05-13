//! GPU resize-stress regression suite (cached path + non-cached path).
//!
//! Tests live in sibling `tests.rs` (the conventional Rust test-file pattern).
//! The whole tests.rs is `#![cfg(all(test, feature = "gpu-tests"))]`.

#[cfg(test)]
mod tests;
