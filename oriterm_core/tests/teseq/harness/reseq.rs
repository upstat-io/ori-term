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
/// `reseq` requires two positional arguments: `INPUT OUTPUT`. The output
/// file lives inside a `TempDirGuard`-managed directory so panic-unwind
/// and `?`-propagation paths both clean up — a manual `fs::remove_file`
/// tail only runs on the happy path.
pub fn compile_teseq(teseq_path: &Path) -> Result<Vec<u8>, String> {
    let guard = oriterm_test_support::TempDirGuard::new("reseq_compile");
    let tmp_out = guard.path().join("output.bin");

    let status = std::process::Command::new("reseq")
        .arg(teseq_path)
        .arg(&tmp_out)
        .status()
        .map_err(|e| format!("failed to run reseq: {e}"))?;

    if !status.success() {
        return Err(format!("reseq failed (exit {status})"));
    }

    std::fs::read(&tmp_out)
        .map_err(|e| format!("failed to read reseq output {}: {e}", tmp_out.display()))
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
