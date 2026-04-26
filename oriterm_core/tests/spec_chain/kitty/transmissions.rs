//! Per-transmission rung (§13.1) — `t=d` / `t=f` / `t=t` / `t=s`.
//!
//! Drives the transmission dispatch at `kitty_store_image` + `kitty_store_from_file`
//! (`store.rs`). Covers success paths, the path-traversal security guard at
//! `store.rs:88-92`, the tempfile-removal invariant at `store.rs:100-106`,
//! and the shared-memory EINVAL rejection.

use oriterm_test_support::spec_chain::SpecHarness;

use super::fixtures::{
    b64, kitty_apc, ok_reply_for, placement_count, reply_bytes, reply_contains, rgba_4x4_red,
    tmp_dir,
};

/// Catalog row: `KG-TRANSMIT-DIRECT` (t=d — default and common path).
#[test]
fn kitty_transmission_direct_stores_inline_payload() {
    let mut h = SpecHarness::new();
    h.feed(&kitty_apc(
        b"a=T,i=20,t=d,f=32,s=4,v=4",
        &b64(&rgba_4x4_red()),
    ));

    assert_eq!(placement_count(&h), 1);
    assert!(reply_contains(&h, &ok_reply_for(20)));
}

/// Catalog row: `KG-TRANSMIT-FILE` (t=f reads from disk, source NOT removed).
///
/// Reads the decoded-base64 path as a filesystem location and loads the file
/// body. Pins that the source file is NOT removed (unlike t=t).
#[test]
fn kitty_transmission_file_reads_from_disk_without_removing_source() {
    let dir = tmp_dir("file_read");
    let path = dir.path().join("payload.raw");
    std::fs::write(&path, rgba_4x4_red()).expect("write fixture");

    let mut h = SpecHarness::new();
    h.feed(&kitty_apc(
        b"a=T,i=21,t=f,f=32,s=4,v=4",
        &b64(path.to_str().expect("utf-8 path").as_bytes()),
    ));

    assert_eq!(placement_count(&h), 1);
    assert!(reply_contains(&h, &ok_reply_for(21)));
    assert!(
        path.exists(),
        "t=f MUST NOT remove the source file — t=t is the removing variant",
    );
}

/// Catalog row: `KG-TRANSMIT-TEMPFILE` (t=t reads AND removes source file per store.rs:100-106).
#[test]
fn kitty_transmission_tempfile_reads_and_removes_source() {
    let dir = tmp_dir("tempfile_read");
    let path = dir.path().join("payload.raw");
    std::fs::write(&path, rgba_4x4_red()).expect("write fixture");
    assert!(path.exists(), "precondition: fixture file exists");

    let mut h = SpecHarness::new();
    h.feed(&kitty_apc(
        b"a=T,i=22,t=t,f=32,s=4,v=4",
        &b64(path.to_str().expect("utf-8 path").as_bytes()),
    ));

    assert_eq!(placement_count(&h), 1);
    assert!(reply_contains(&h, &ok_reply_for(22)));
    assert!(
        !path.exists(),
        "t=t MUST remove the source file after reading it",
    );
}

/// Catalog row: `KG-TRANSMIT-SHARED-MEM-REJECTED` (t=s rejected with EINVAL — verified-with-deviation).
///
/// Shared-memory transmission is not implemented and is rejected with EINVAL.
/// No placement is created.
#[test]
fn kitty_transmission_shared_memory_rejected_with_einval_reply() {
    let mut h = SpecHarness::new();
    h.feed(&kitty_apc(
        b"a=T,i=23,t=s,f=32,s=4,v=4",
        &b64(b"/dev/shm/fake"),
    ));

    assert_eq!(
        placement_count(&h),
        0,
        "t=s rejection MUST NOT create a placement",
    );
    let replies = reply_bytes(&h);
    let s = String::from_utf8_lossy(&replies);
    assert!(
        s.contains("EINVAL: shared memory"),
        "t=s must emit EINVAL shared-memory reply — got {s:?}",
    );
}

/// Catalog row: `KG-TRANSMIT-FILE` (negative — path-traversal guard at store.rs:88-92).
///
/// A `..` component in the decoded path causes EINVAL before `fs::read` is
/// attempted. Defence-in-depth security invariant — a regression here is an
/// unsafe file-access vector.
#[test]
fn kitty_transmission_file_path_traversal_rejected() {
    let mut h = SpecHarness::new();
    h.feed(&kitty_apc(
        b"a=t,i=24,t=f,f=32,s=4,v=4",
        &b64(b"../etc/passwd"),
    ));

    let replies = reply_bytes(&h);
    let s = String::from_utf8_lossy(&replies);
    assert!(
        s.contains("EINVAL: path traversal"),
        "../ in t=f path MUST emit EINVAL path-traversal reply — got {s:?}",
    );
}

// BUG-08-021 — kitty_store_from_file file-size pre-check + bounded reader.
// All tests in this section verify the bug fix: file-size enforcement
// runs BEFORE allocating the file into memory, t=t cleanup runs on
// every exit path via RAII guard, and non-regular paths are rejected
// before any read is attempted.

/// BUG-08-021: `t=f` with file > max_bytes returns ENOMEM via the
/// metadata preflight, BEFORE the bounded read fills file_data. The
/// post-read check is unreachable in this scenario (preflight rejects
/// first). t=f does NOT remove the source file even on rejection.
#[test]
fn kitty_store_from_file_oversized_t_eq_f_returns_enomem_via_preflight() {
    let dir = tmp_dir("oversized_t_f");
    let path = dir.path().join("oversized.raw");
    // 128 bytes, well over the 64-byte max_bytes we'll set.
    std::fs::write(&path, vec![0u8; 128]).expect("write fixture");

    let mut h = SpecHarness::new();
    h.term_mut().set_image_limits(usize::MAX, 64);
    h.feed(&kitty_apc(
        b"a=T,i=80,t=f,f=32,s=4,v=4",
        &b64(path.to_str().expect("utf-8 path").as_bytes()),
    ));

    assert_eq!(
        placement_count(&h),
        0,
        "oversized t=f MUST NOT produce a placement",
    );
    let replies = reply_bytes(&h);
    let s = String::from_utf8_lossy(&replies);
    assert!(
        s.contains("ENOMEM"),
        "oversized t=f MUST emit ENOMEM reply — got {s:?}",
    );
    assert!(
        path.exists(),
        "t=f MUST NOT remove the source file even on oversized rejection",
    );
}

/// BUG-08-021: `t=t` with file > max_bytes returns ENOMEM AND the
/// source file IS removed via the RAII guard's Drop. Pins both the
/// rejection and the cleanup-on-rejection invariant.
#[test]
fn kitty_store_from_file_oversized_t_eq_t_returns_enomem_and_removes_source() {
    let dir = tmp_dir("oversized_t_t");
    let path = dir.path().join("oversized.raw");
    std::fs::write(&path, vec![0u8; 128]).expect("write fixture");
    assert!(path.exists(), "precondition: fixture exists");

    let mut h = SpecHarness::new();
    h.term_mut().set_image_limits(usize::MAX, 64);
    h.feed(&kitty_apc(
        b"a=T,i=81,t=t,f=32,s=4,v=4",
        &b64(path.to_str().expect("utf-8 path").as_bytes()),
    ));

    assert_eq!(placement_count(&h), 0);
    let replies = reply_bytes(&h);
    let s = String::from_utf8_lossy(&replies);
    assert!(
        s.contains("ENOMEM"),
        "oversized t=t MUST emit ENOMEM reply — got {s:?}",
    );
    assert!(
        !path.exists(),
        "t=t MUST remove the source file via RAII guard even on oversized rejection",
    );
}

/// BUG-08-021 boundary clamp: file size == max_bytes succeeds.
/// Per Round 2 Codex F6 + Gemini F3 + Opencode F2 (3-of-3 agreement)
/// boundary-from-below pin.
#[test]
fn kitty_store_from_file_exactly_max_bytes_succeeds() {
    let dir = tmp_dir("exactly_max");
    let path = dir.path().join("exact.raw");
    // f=32 4×4 RGBA fixture is exactly 64 bytes. Set max_bytes = 64
    // so the file size equals the boundary.
    std::fs::write(&path, rgba_4x4_red()).expect("write fixture");

    let mut h = SpecHarness::new();
    h.term_mut().set_image_limits(usize::MAX, 64);
    h.feed(&kitty_apc(
        b"a=T,i=82,t=f,f=32,s=4,v=4",
        &b64(path.to_str().expect("utf-8 path").as_bytes()),
    ));

    assert_eq!(
        placement_count(&h),
        1,
        "file size == max_bytes MUST succeed (boundary-from-below)",
    );
    assert!(reply_contains(&h, &ok_reply_for(82)));
}

/// BUG-08-021 boundary clamp: file size == max_bytes + 1 rejects via
/// metadata preflight. Pins the boundary-from-above per the same 3-of-3
/// agreement.
#[test]
fn kitty_store_from_file_max_bytes_plus_one_rejects_via_preflight() {
    let dir = tmp_dir("max_plus_one");
    let path = dir.path().join("plus_one.raw");
    // 65 bytes, max_bytes = 64. Preflight `meta.len() > max_bytes`
    // fires (65 > 64) and returns ENOMEM before the bounded read.
    let mut data = rgba_4x4_red(); // 64 bytes
    data.push(0u8); // 65th byte
    std::fs::write(&path, &data).expect("write fixture");

    let mut h = SpecHarness::new();
    h.term_mut().set_image_limits(usize::MAX, 64);
    h.feed(&kitty_apc(
        b"a=T,i=83,t=f,f=32,s=4,v=4",
        &b64(path.to_str().expect("utf-8 path").as_bytes()),
    ));

    assert_eq!(placement_count(&h), 0);
    let replies = reply_bytes(&h);
    let s = String::from_utf8_lossy(&replies);
    assert!(
        s.contains("ENOMEM"),
        "max_bytes + 1 MUST emit ENOMEM via preflight — got {s:?}",
    );
}

/// BUG-08-021: `max_bytes = usize::MAX` does NOT panic on overflow.
/// Pins the saturating_add invariant against debug-mode overflow.
/// Per Round 1 Codex F5 + Gemini F2 + Opencode F2 (3-of-3 agreement).
#[test]
fn kitty_store_from_file_max_bytes_usize_max_does_not_panic() {
    let dir = tmp_dir("usize_max");
    let path = dir.path().join("normal.raw");
    std::fs::write(&path, rgba_4x4_red()).expect("write fixture");

    let mut h = SpecHarness::new();
    // Stress the saturating_add path — read_cap = (usize::MAX as u64).saturating_add(1) = u64::MAX.
    h.term_mut().set_image_limits(usize::MAX, usize::MAX);
    h.feed(&kitty_apc(
        b"a=T,i=84,t=f,f=32,s=4,v=4",
        &b64(path.to_str().expect("utf-8 path").as_bytes()),
    ));

    // Under saturating_add, this scenario must succeed without panic.
    assert_eq!(
        placement_count(&h),
        1,
        "max_bytes=usize::MAX MUST not panic"
    );
    assert!(reply_contains(&h, &ok_reply_for(84)));
}

/// BUG-08-021 (Unix only): path is a directory → EINVAL via fstat
/// non-regular-file rejection. Per Round 2 Codex F1 + Opencode F1.
#[cfg(unix)]
#[test]
fn kitty_store_from_file_directory_path_returns_einval() {
    let dir = tmp_dir("directory_path");
    // Use the tmp dir itself as the "file" path — it's a directory.
    let dir_path = dir.path().to_path_buf();

    let mut h = SpecHarness::new();
    h.feed(&kitty_apc(
        b"a=T,i=85,t=f,f=32,s=4,v=4",
        &b64(dir_path.to_str().expect("utf-8 path").as_bytes()),
    ));

    assert_eq!(placement_count(&h), 0);
    let replies = reply_bytes(&h);
    let s = String::from_utf8_lossy(&replies);
    // On Unix, File::open succeeds for a directory and fstat reveals the
    // dir file_type — we reject with the exact EINVAL message. Pin the
    // exact string per Round 0 Code TPR Codex F2 (negative-test rigor).
    assert!(
        s.contains("EINVAL: path is not a regular file"),
        "Unix directory path MUST emit `EINVAL: path is not a regular file` — got {s:?}",
    );
    assert!(
        dir_path.exists(),
        "directory MUST persist (cannot be remove_file'd)"
    );
}

/// BUG-08-021 (Unix only): path is a FIFO → rejected without
/// blocking. Pins the O_NONBLOCK + fstat protection against
/// FIFO-without-writer DoS that Round 0 3-of-3 reviewers flagged.
///
/// Uses the system `mkfifo(1)` binary instead of libc::mkfifo to keep
/// the test free of unsafe blocks (workspace `unsafe-code = "deny"`).
/// Skips gracefully when `mkfifo` is unavailable.
#[cfg(unix)]
#[test]
fn kitty_store_from_file_fifo_path_returns_einval_no_block() {
    use std::process::Command;

    let dir = tmp_dir("fifo_path");
    let fifo_path = dir.path().join("fifo");

    // Try `mkfifo <path>` via the system binary. On any failure (binary
    // missing, permission, etc.), skip gracefully.
    let mkfifo_status = Command::new("mkfifo").arg(&fifo_path).status();
    match mkfifo_status {
        Ok(s) if s.success() => {}
        _ => {
            eprintln!("SKIP: mkfifo binary unavailable or failed");
            return;
        }
    }
    assert!(fifo_path.exists(), "precondition: FIFO created");

    let mut h = SpecHarness::new();
    h.feed(&kitty_apc(
        b"a=T,i=86,t=f,f=32,s=4,v=4",
        &b64(fifo_path.to_str().expect("utf-8 path").as_bytes()),
    ));

    assert_eq!(placement_count(&h), 0);
    let replies = reply_bytes(&h);
    let s = String::from_utf8_lossy(&replies);
    // O_NONBLOCK on Unix: open() succeeds immediately, then fstat reveals
    // FIFO file_type and we reject with the exact EINVAL message. Pin the
    // exact string per Round 0 Code TPR Codex F2 (negative-test rigor).
    assert!(
        s.contains("EINVAL: path is not a regular file"),
        "FIFO path MUST emit `EINVAL: path is not a regular file` — got {s:?}",
    );
}

/// BUG-08-021: t=t with FIFO path. Pins the RAII-cleanup-on-EINVAL
/// invariant — `remove_file` CAN remove FIFOs on Unix, so the guard's
/// Drop should successfully unlink the FIFO when t=t requested deletion.
/// Per Round 0 Code TPR Codex F3 (missing non-regular t=t cleanup matrix).
#[cfg(unix)]
#[test]
fn kitty_store_from_file_fifo_t_eq_t_returns_einval_and_removes_source() {
    use std::process::Command;

    let dir = tmp_dir("fifo_t_t");
    let fifo_path = dir.path().join("fifo_t_t");

    let mkfifo_status = Command::new("mkfifo").arg(&fifo_path).status();
    match mkfifo_status {
        Ok(s) if s.success() => {}
        _ => {
            eprintln!("SKIP: mkfifo binary unavailable or failed");
            return;
        }
    }
    assert!(fifo_path.exists(), "precondition: FIFO created");

    let mut h = SpecHarness::new();
    h.feed(&kitty_apc(
        b"a=T,i=88,t=t,f=32,s=4,v=4",
        &b64(fifo_path.to_str().expect("utf-8 path").as_bytes()),
    ));

    assert_eq!(placement_count(&h), 0);
    let replies = reply_bytes(&h);
    let s = String::from_utf8_lossy(&replies);
    assert!(
        s.contains("EINVAL: path is not a regular file"),
        "FIFO t=t MUST emit `EINVAL: path is not a regular file` — got {s:?}",
    );
    assert!(
        !fifo_path.exists(),
        "t=t MUST remove the FIFO via RAII guard even on EINVAL rejection",
    );
}

/// BUG-08-021: empty file (0 bytes) with f=32,s=4,v=4 fails decode
/// gracefully with EINVAL. Pins the empty-file policy explicitly —
/// the bounded read returns immediately with `file_data.len() == 0`,
/// then `kitty_decode_pixels` rejects (0 != expected 64).
#[test]
fn kitty_store_from_file_empty_file_returns_einval() {
    let dir = tmp_dir("empty_file");
    let path = dir.path().join("empty.raw");
    std::fs::write(&path, b"").expect("write empty fixture");
    assert_eq!(std::fs::metadata(&path).unwrap().len(), 0);

    let mut h = SpecHarness::new();
    h.feed(&kitty_apc(
        b"a=T,i=87,t=f,f=32,s=4,v=4",
        &b64(path.to_str().expect("utf-8 path").as_bytes()),
    ));

    assert_eq!(placement_count(&h), 0);
    let replies = reply_bytes(&h);
    let s = String::from_utf8_lossy(&replies);
    assert!(
        s.contains("EINVAL"),
        "empty file with f=32,s=4,v=4 MUST emit EINVAL (0 != expected 64) — got {s:?}",
    );
}

/// BUG-08-029: `kitty_decode_pixels` `(w * h * 4)` overflow protection.
/// Pin: feeding `s=u32::MAX,v=u32::MAX,f=32` MUST emit EINVAL without
/// panic (debug build would have panicked on the bare multiplication;
/// release build would have wrapped to 0 and corrupted the size check).
#[test]
fn kitty_decode_pixels_extreme_dimensions_returns_einval_no_panic() {
    let mut h = SpecHarness::new();
    // s=4294967295,v=4294967295 — u32::MAX × u32::MAX × 4 overflows usize on
    // 64-bit (u32::MAX^2 is ~1.8e19, near u64::MAX; ×4 wraps).
    h.feed(&kitty_apc(
        b"a=T,i=89,t=d,f=32,s=4294967295,v=4294967295",
        &b64(b"AAAA"), // tiny payload — irrelevant; size check rejects first
    ));

    assert_eq!(placement_count(&h), 0, "extreme dims MUST NOT produce a placement");
    let replies = reply_bytes(&h);
    let s = String::from_utf8_lossy(&replies);
    assert!(
        s.contains("EINVAL") && s.contains("overflow"),
        "extreme dims MUST emit `EINVAL: ... overflow usize` — got {s:?}",
    );
}

/// BUG-08-029: same overflow protection for `f=24` (RGB) format.
#[test]
fn kitty_decode_pixels_extreme_dimensions_rgb_returns_einval_no_panic() {
    let mut h = SpecHarness::new();
    h.feed(&kitty_apc(
        b"a=T,i=90,t=d,f=24,s=4294967295,v=4294967295",
        &b64(b"AAA"),
    ));

    assert_eq!(placement_count(&h), 0);
    let replies = reply_bytes(&h);
    let s = String::from_utf8_lossy(&replies);
    assert!(
        s.contains("EINVAL") && s.contains("overflow"),
        "extreme RGB dims MUST emit `EINVAL: ... overflow usize` — got {s:?}",
    );
}

/// BUG-08-024: `a=a` (animate) MUST emit ENOENT when `i=` is missing,
/// mirroring `a=p` (place) which already does so at place.rs:16-19.
/// Pin the cross-action consistency.
#[test]
fn kitty_animate_missing_image_id_returns_enoent() {
    let mut h = SpecHarness::new();
    // a=a without i= — both place and animate now return ENOENT.
    h.feed(&kitty_apc(b"a=a", ""));

    let replies = reply_bytes(&h);
    let s = String::from_utf8_lossy(&replies);
    assert!(
        s.contains("ENOENT"),
        "a=a without i= MUST emit ENOENT (mirrors a=p) — got {s:?}",
    );
}
