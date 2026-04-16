//! Spec-chain integration tests for DEC private modes.
//!
//! Each submodule verifies a mode family's DECSET/DECRST flag toggle
//! and DECRQM query/response through the full VTE → Term → Effect
//! pipeline.

mod decnkm_decbkm;
mod mode_2026;
mod mode_2031;
