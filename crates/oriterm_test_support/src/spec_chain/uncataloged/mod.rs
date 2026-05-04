//! Uncataloged sequence detector for the verification chain.
//!
//! Converts raw `PerformAction` entries into `TupleSig` (the canonical
//! catalog tuple signature) and accumulates distinct tuples in memory.
//! On `SpecHarness::drop()`, accumulated tuples are serialized to a
//! temp file for post-test aggregation by `spec-coverage-report --check`.
//!
//! No file I/O during test execution — flaky-test discipline per
//!.

use std::collections::HashSet;
use std::io::Write;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use vte::ansi::PerformAction;

use crate::catalog::{Category, Tuple, TupleSig};

static INSTANCE_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Accumulates distinct `TupleSig` values from `PerformAction` entries.
///
/// Each `SpecHarness` owns one detector. `feed_actions()` is called after
/// every `SpecHarness::feed()` to extract tuples from newly recorded
/// parser actions. The detector is single-threaded (no `Arc`/`Mutex`
/// needed — each harness runs on one thread).
#[derive(Debug, Default)]
pub struct UncatalogedDetector {
    seen: HashSet<TupleSig>,
}

impl UncatalogedDetector {
    /// Create a new empty detector.
    pub fn new() -> Self {
        Self {
            seen: HashSet::new(),
        }
    }

    /// Extract `TupleSig` values from parser actions and accumulate.
    pub fn feed_actions(&mut self, actions: &[PerformAction]) {
        for action in actions {
            if let Some(sig) = perform_action_to_sig(action) {
                self.seen.insert(sig);
            }
        }
    }

    /// The set of distinct tuple signatures seen so far.
    pub fn seen(&self) -> &HashSet<TupleSig> {
        &self.seen
    }

    /// Serialize accumulated tuples to a temp file under `output_dir`.
    ///
    /// Filename: `<pid>-<counter>-<nanos>.jsonl` (atomic counter +
    /// nanosecond timestamp ensures no overwriting across sequential
    /// tests on the same thread).
    ///
    /// Format: one JSON line per tuple `["category","intermediates","final_byte"]`.
    pub fn serialize_to_dir(&self, output_dir: &PathBuf) -> std::io::Result<()> {
        if self.seen.is_empty() {
            return Ok(());
        }
        std::fs::create_dir_all(output_dir)?;

        let pid = std::process::id();
        let counter = INSTANCE_COUNTER.fetch_add(1, Ordering::Relaxed);
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let filename = format!("{pid}-{counter}-{nanos}.jsonl");
        let path = output_dir.join(filename);

        let mut file = std::fs::File::create(&path)?;
        for (cat, intermediates, final_byte) in &self.seen {
            let intermediates_hex: Vec<String> =
                intermediates.iter().map(|b| format!("{b:02x}")).collect();
            writeln!(
                file,
                "[\"{cat}\",\"{}\",\"{final_byte}\"]",
                intermediates_hex.join(",")
            )?;
        }
        Ok(())
    }
}

/// Convert a `PerformAction` into a canonical `Tuple` (if it represents
/// a catalogable sequence).
///
/// Returns `None` for `Print`, `Put`, `Unhook`, `ApcStart`, `ApcPut`,
/// `ApcEnd` — these are data/framing, not distinct sequence types.
///
/// The runtime observer leaves `params` empty by contract — the
/// `PerformAction` does not surface payload bytes (CSI carries
/// `Vec<Vec<u16>>` numeric params, OSC carries `Vec<Vec<u8>>` parts;
/// the catalog's per-OSC `params` placeholder string is canonicalized
/// via `osc_placeholder` in the catalog/capture path, not here).
/// Made `pub` so the SSOT-alignment matrix can observe the producer's
/// actual `Tuple` shape, not a reconstructed proxy (round-2).
pub fn perform_action_to_tuple(action: &PerformAction) -> Option<Tuple> {
    match action {
        PerformAction::CsiDispatch {
            intermediates,
            action,
            ..
        } => Some(Tuple::new(
            Category::Csi,
            intermediates.clone(),
            String::new(),
            action.to_string(),
        )),
        PerformAction::OscDispatch { params, .. } => {
            // Extract OSC selector from first param.
            let cmd = params
                .first()
                .map(|p| String::from_utf8_lossy(p).to_string())
                .unwrap_or_default();
            Some(Tuple::new(Category::Osc, Vec::new(), String::new(), cmd))
        }
        PerformAction::EscDispatch {
            intermediates,
            byte,
            ..
        } => Some(Tuple::new(
            Category::Esc,
            intermediates.clone(),
            String::new(),
            String::from(*byte as char),
        )),
        PerformAction::Hook {
            intermediates,
            action,
            ..
        } => Some(Tuple::new(
            Category::Dcs,
            intermediates.clone(),
            String::new(),
            action.to_string(),
        )),
        // C0 controls are dispatched per-byte through `Performer::execute`;
        // neither the catalog (`canonical.rs:56`) nor the capture path
        // (`capture_extract.rs::execute` is a no-op) emits a tuple for them.
        // The runtime observer must agree — emitting C0 here would be the
        // sole source of `Category::C0` tuples in the spool, producing
        // false positives in `spec-coverage-report --check` BACKLOG for
        // every test that triggers a newline / tab / bell.
        // Data/framing actions are not catalogable sequence types either.
        PerformAction::Execute { .. }
        | PerformAction::Print { .. }
        | PerformAction::Put { .. }
        | PerformAction::Unhook
        | PerformAction::ApcStart
        | PerformAction::ApcPut { .. }
        | PerformAction::ApcEnd => None,
    }
}

/// Convert a `PerformAction` into a `TupleSig` (if it represents a
/// catalogable sequence). Thin wrapper over [`perform_action_to_tuple`]
/// that projects to the comparison signature.
fn perform_action_to_sig(action: &PerformAction) -> Option<TupleSig> {
    perform_action_to_tuple(action).map(|t| t.signature())
}

/// Read all `.jsonl` files from a directory and parse them into tuples.
///
/// Used by `spec-coverage-report --check` to aggregate tuples from
/// parallel test runs.
pub fn read_accumulated_tuples(dir: &PathBuf) -> std::io::Result<HashSet<TupleSig>> {
    let mut all = HashSet::new();
    if !dir.exists() {
        return Ok(all);
    }
    let entries = std::fs::read_dir(dir)?;
    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        if path.extension().is_some_and(|ext| ext == "jsonl") {
            let content = std::fs::read_to_string(&path)?;
            for line in content.lines() {
                if let Some(sig) = parse_jsonl_tuple(line) {
                    all.insert(sig);
                }
            }
        }
    }
    Ok(all)
}

/// Parse a JSON line `["category","intermediates","final_byte"]` into a `TupleSig`.
fn parse_jsonl_tuple(line: &str) -> Option<TupleSig> {
    let trimmed = line.trim().trim_start_matches('[').trim_end_matches(']');
    let parts: Vec<&str> = trimmed.split(',').collect();
    if parts.len() != 3 {
        return None;
    }
    let cat = parts[0].trim().trim_matches('"').to_string();
    let intermediates_str = parts[1].trim().trim_matches('"');
    let final_byte = parts[2].trim().trim_matches('"').to_string();

    let intermediates: Vec<u8> = if intermediates_str.is_empty() {
        Vec::new()
    } else {
        intermediates_str
            .split(',')
            .filter_map(|h| u8::from_str_radix(h.trim(), 16).ok())
            .collect()
    };

    Some((cat, intermediates, final_byte))
}

#[cfg(test)]
mod tests;
