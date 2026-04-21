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
    let path = dir.join("payload.raw");
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

    std::fs::remove_file(&path).ok();
    std::fs::remove_dir(&dir).ok();
}

/// Catalog row: `KG-TRANSMIT-TEMPFILE` (t=t reads AND removes source file per store.rs:100-106).
#[test]
fn kitty_transmission_tempfile_reads_and_removes_source() {
    let dir = tmp_dir("tempfile_read");
    let path = dir.join("payload.raw");
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

    std::fs::remove_dir(&dir).ok();
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
