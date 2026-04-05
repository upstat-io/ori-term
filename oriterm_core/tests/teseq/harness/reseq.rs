//! Reseq subprocess adapter.
//!
//! Thin adapter that invokes `reseq` to compile `.teseq` files into raw
//! terminal bytes, and `teseq` for optional outbound response analysis.

use std::path::Path;

/// Check if `reseq` is installed and accessible.
pub fn reseq_available() -> bool {
    std::process::Command::new("reseq")
        .arg("--version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok()
}

/// Compile a `.teseq` file to raw bytes via `reseq` subprocess.
///
/// `reseq` requires two positional arguments: `INPUT OUTPUT`.
/// We write to a unique temp file (cross-platform, parallel-safe)
/// and read back the bytes.
pub fn compile_teseq(teseq_path: &Path) -> Result<Vec<u8>, String> {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);

    let id = COUNTER.fetch_add(1, Ordering::Relaxed);
    let tid = std::thread::current().id();
    let filename = format!("oriterm_reseq_{tid:?}_{id}.bin");
    let tmp_out = std::env::temp_dir().join(filename);

    let status = std::process::Command::new("reseq")
        .arg(teseq_path)
        .arg(&tmp_out)
        .status()
        .map_err(|e| format!("failed to run reseq: {e}"))?;

    if !status.success() {
        let _ = std::fs::remove_file(&tmp_out);
        return Err(format!("reseq failed (exit {status})"));
    }

    let bytes = std::fs::read(&tmp_out)
        .map_err(|e| format!("failed to read reseq output {}: {e}", tmp_out.display()))?;

    // Clean up temp file (best-effort).
    let _ = std::fs::remove_file(&tmp_out);

    Ok(bytes)
}

/// Check if `teseq` is installed (for outbound response analysis in Section 03).
pub fn teseq_available() -> bool {
    std::process::Command::new("teseq")
        .arg("--version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok()
}
