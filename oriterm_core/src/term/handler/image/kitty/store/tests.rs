//! Tests for `kitty_store_from_file` — bounded file reading, temp-file cleanup, and TOCTOU/special-file rejection.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use crate::effect::VoidEffectSink;
use crate::term::Term;
use crate::term::handler::image::kitty::{KittyStoreParams, KittyTransmission};
use crate::theme::Theme;

/// Fixture builder: creates a temporary file with specified content.
struct TempFileFixture {
    path: PathBuf,
    _keep_alive: bool,
}

impl TempFileFixture {
    /// Create a temporary file with `content` bytes in a unique directory.
    fn new(test_name: &str, content: &[u8]) -> std::io::Result<Self> {
        let dir = std::env::temp_dir();
        let test_dir_name = format!("ori_kitty_test_{}_{}", test_name, std::process::id());
        let test_dir = dir.join(&test_dir_name);

        // Create the test directory
        fs::create_dir_all(&test_dir)?;

        let path = test_dir.join("testfile");

        let mut file = fs::File::create(&path)?;
        file.write_all(content)?;
        file.sync_all()?;
        drop(file);

        Ok(Self {
            path,
            _keep_alive: true,
        })
    }

    fn path(&self) -> &Path {
        &self.path
    }

    /// Returns the path as a UTF-8 string for the fixture payload.
    fn path_str(&self) -> String {
        self.path.to_string_lossy().to_string()
    }

    /// Clean up the test directory and file
    fn cleanup(&self) -> std::io::Result<()> {
        if let Some(parent) = self.path.parent() {
            let _ = fs::remove_file(&self.path);
            let _ = fs::remove_dir(parent);
        }
        Ok(())
    }
}

impl Drop for TempFileFixture {
    fn drop(&mut self) {
        let _ = self.cleanup();
    }
}

#[test]
fn kitty_store_from_file_oversized_t_eq_f_returns_enomem_with_bounded_read() {
    // File size > max_bytes, t=f (File, never removed), ENOMEM reply
    let mut term = Term::new(24, 80, 1000, Theme::default(), VoidEffectSink);
    const MAX_BYTES: usize = 64;
    term.set_image_limits(usize::MAX, MAX_BYTES);

    // Create a 65-byte file (one beyond the limit)
    let fixture = TempFileFixture::new("oversized_t_eq_f", &[0xABu8; 65])
        .expect("failed to create temp file");

    let p = KittyStoreParams {
        image_id: 1,
        image_number: None,
        payload: fixture.path_str().into_bytes(),
        format: 32,
        width: 0,
        height: 0,
        transmission: KittyTransmission::File,
    };

    let result = term.kitty_store_image(p);
    assert!(result.is_err(), "oversized file should return error");
    let err = result.unwrap_err();
    assert!(
        err.contains("ENOMEM"),
        "error should be ENOMEM, got: {}",
        err
    );

    // Verify the file still exists (t=f never removes)
    assert!(fixture.path().exists(), "t=f should NOT remove the file");
}

#[test]
fn kitty_store_from_file_within_size_succeeds_t_eq_t_removes_source() {
    // File ≤ max_bytes, t=t (TempFile), success path: stores image AND removes source
    let mut term = Term::new(24, 80, 1000, Theme::default(), VoidEffectSink);
    const MAX_BYTES: usize = 64;
    term.set_image_limits(usize::MAX, MAX_BYTES);

    // Create a 64-byte file (exactly at the limit) with valid RGBA data
    // format=32 (raw RGBA), width=4, height=4 means 4×4×4 bytes = 64 bytes
    let mut rgba_data = vec![0u8; 64];
    // Fill with recognizable pattern
    for i in 0..64 {
        rgba_data[i] = (i as u8).wrapping_mul(42);
    }

    let fixture =
        TempFileFixture::new("within_size_t_eq_t", &rgba_data).expect("failed to create temp file");
    let fixture_path = fixture.path().to_path_buf();

    let p = KittyStoreParams {
        image_id: 1,
        image_number: None,
        payload: fixture.path_str().into_bytes(),
        format: 32,
        width: 4,
        height: 4,
        transmission: KittyTransmission::TempFile, // t=t
    };

    let result = term.kitty_store_image(p);
    assert!(
        result.is_ok(),
        "file within size should succeed, got: {:?}",
        result
    );

    // Verify the file was removed by the RAII guard
    assert!(
        !fixture_path.exists(),
        "t=t should remove the file on success"
    );
}

#[test]
fn kitty_store_from_file_oversized_t_eq_t_returns_enomem_and_removes_source() {
    // File > max_bytes, t=t (TempFile), ENOMEM reply + source file IS removed
    let mut term = Term::new(24, 80, 1000, Theme::default(), VoidEffectSink);
    const MAX_BYTES: usize = 64;
    term.set_image_limits(usize::MAX, MAX_BYTES);

    // Create a 65-byte file (one beyond the limit)
    let fixture = TempFileFixture::new("oversized_t_eq_t", &[0xCDu8; 65])
        .expect("failed to create temp file");
    let fixture_path = fixture.path().to_path_buf();

    let p = KittyStoreParams {
        image_id: 1,
        image_number: None,
        payload: fixture.path_str().into_bytes(),
        format: 32,
        width: 0,
        height: 0,
        transmission: KittyTransmission::TempFile, // t=t
    };

    let result = term.kitty_store_image(p);
    assert!(result.is_err(), "oversized file should return error");
    let err = result.unwrap_err();
    assert!(
        err.contains("ENOMEM"),
        "error should be ENOMEM, got: {}",
        err
    );

    // Verify the file WAS removed by the RAII guard (t=t cleanup on error)
    assert!(
        !fixture_path.exists(),
        "t=t should remove the file even on ENOMEM rejection"
    );
}

#[test]
fn kitty_store_from_file_metadata_unavailable_returns_eio() {
    // Broken symlink or unreadable path: EIO reply with clean error
    let mut term = Term::new(24, 80, 1000, Theme::default(), VoidEffectSink);

    let p = KittyStoreParams {
        image_id: 1,
        image_number: None,
        payload: "/nonexistent/path/that/definitely/does/not/exist"
            .as_bytes()
            .to_vec(),
        format: 32,
        width: 0,
        height: 0,
        transmission: KittyTransmission::File,
    };

    let result = term.kitty_store_image(p);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(
        err.contains("EIO") || err.contains("EINVAL"),
        "error should be EIO or EINVAL (path not found), got: {}",
        err
    );
}

#[test]
#[cfg(unix)]
fn kitty_store_from_file_directory_path_returns_einval() {
    // Path points at a directory: EINVAL "path is not a regular file"
    use std::fs;

    let mut term = Term::new(24, 80, 1000, Theme::default(), VoidEffectSink);

    let dir = std::env::temp_dir();
    let test_dir_name = format!("kitty_test_dir_einval_{}", std::process::id());
    let test_dir = dir.join(&test_dir_name);

    // Create a temporary directory
    let _ = fs::create_dir(&test_dir);

    let p = KittyStoreParams {
        image_id: 1,
        image_number: None,
        payload: test_dir.to_string_lossy().to_string().into_bytes(),
        format: 32,
        width: 0,
        height: 0,
        transmission: KittyTransmission::File,
    };

    let result = term.kitty_store_image(p);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(
        err.contains("EINVAL") && err.contains("regular file"),
        "error should be EINVAL with regular file message, got: {}",
        err
    );

    // Clean up
    let _ = fs::remove_dir(&test_dir);
}

#[test]
#[cfg(windows)]
fn kitty_store_from_file_directory_path_returns_eio() {
    // Windows: File::open on a directory fails with EIO (ACCESS_DENIED)
    use std::fs;

    let mut term = Term::new(24, 80, 1000, Theme::default(), VoidEffectSink);

    let dir = std::env::temp_dir();
    let test_dir_name = format!("kitty_test_dir_eio_{}", std::process::id());
    let test_dir = dir.join(&test_dir_name);

    // Create a temporary directory
    let _ = fs::create_dir(&test_dir);

    let p = KittyStoreParams {
        image_id: 1,
        image_number: None,
        payload: test_dir.to_string_lossy().to_string().into_bytes(),
        format: 32,
        width: 0,
        height: 0,
        transmission: KittyTransmission::File,
    };

    let result = term.kitty_store_image(p);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(
        err.contains("EIO") && err.contains("failed to open"),
        "error should be EIO from File::open, got: {}",
        err
    );

    // Clean up
    let _ = fs::remove_dir(&test_dir);
}

#[test]
fn kitty_store_from_file_empty_file_einval() {
    // 0-byte file: bounded read returns empty, kitty_decode_pixels rejects with EINVAL
    let mut term = Term::new(24, 80, 1000, Theme::default(), VoidEffectSink);

    let fixture = TempFileFixture::new("empty_file", &[]).expect("failed to create temp file");

    let p = KittyStoreParams {
        image_id: 1,
        image_number: None,
        payload: fixture.path_str().into_bytes(),
        format: 32,
        width: 4,
        height: 4,
        transmission: KittyTransmission::File,
    };

    let result = term.kitty_store_image(p);
    assert!(result.is_err());
    let err = result.unwrap_err();
    // Empty file causes RGBA decode to fail (size mismatch)
    assert!(
        err.contains("EINVAL"),
        "error should be EINVAL from pixel decode, got: {}",
        err
    );
}

#[test]
fn kitty_store_from_file_exactly_max_bytes_succeeds() {
    // File exactly max_bytes: bounded read fills to exactly max_bytes,
    // post-read check does not trigger (max_bytes > max_bytes is false)
    let mut term = Term::new(24, 80, 1000, Theme::default(), VoidEffectSink);
    const MAX_BYTES: usize = 64;
    term.set_image_limits(usize::MAX, MAX_BYTES);

    // Create a 64-byte file with valid RGBA data
    let mut rgba_data = vec![0u8; 64];
    for i in 0..64 {
        rgba_data[i] = (i as u8).wrapping_mul(42);
    }

    let fixture =
        TempFileFixture::new("exactly_max_bytes", &rgba_data).expect("failed to create temp file");

    let p = KittyStoreParams {
        image_id: 1,
        image_number: None,
        payload: fixture.path_str().into_bytes(),
        format: 32,
        width: 4,
        height: 4,
        transmission: KittyTransmission::File,
    };

    let result = term.kitty_store_image(p);
    assert!(
        result.is_ok(),
        "file exactly max_bytes should succeed, got: {:?}",
        result
    );
}

#[test]
fn kitty_store_from_file_max_bytes_plus_one_rejects() {
    // File max_bytes+1: metadata preflight check rejects with ENOMEM
    // (fast-path — exits before bounded read)
    let mut term = Term::new(24, 80, 1000, Theme::default(), VoidEffectSink);
    const MAX_BYTES: usize = 64;
    term.set_image_limits(usize::MAX, MAX_BYTES);

    // Create a 65-byte file
    let fixture = TempFileFixture::new("max_bytes_plus_one", &[0xDEu8; 65])
        .expect("failed to create temp file");

    let p = KittyStoreParams {
        image_id: 1,
        image_number: None,
        payload: fixture.path_str().into_bytes(),
        format: 32,
        width: 0,
        height: 0,
        transmission: KittyTransmission::File,
    };

    let result = term.kitty_store_image(p);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(
        err.contains("ENOMEM"),
        "error should be ENOMEM from preflight, got: {}",
        err
    );
}

#[test]
fn kitty_store_from_file_max_bytes_usize_max_does_not_panic() {
    // max_bytes == usize::MAX: saturating_add returns u64::MAX,
    // take(u64::MAX) reads the entire file, no panic
    let mut term = Term::new(24, 80, 1000, Theme::default(), VoidEffectSink);
    term.set_image_limits(usize::MAX, usize::MAX);

    // Create a small file (no USIZE_MAX allocation)
    let small_data = [0xABu8; 64];
    let fixture = TempFileFixture::new("max_bytes_usize_max", &small_data)
        .expect("failed to create temp file");

    let p = KittyStoreParams {
        image_id: 1,
        image_number: None,
        payload: fixture.path_str().into_bytes(),
        format: 32,
        width: 4,
        height: 4,
        transmission: KittyTransmission::File,
    };

    let result = term.kitty_store_image(p);
    assert!(
        result.is_ok(),
        "max_bytes=usize::MAX should not panic, got: {:?}",
        result
    );
}

#[test]
fn kitty_store_from_file_path_traversal_rejected() {
    // Path with ".." component: EINVAL "path traversal not allowed"
    let mut term = Term::new(24, 80, 1000, Theme::default(), VoidEffectSink);

    let p = KittyStoreParams {
        image_id: 1,
        image_number: None,
        payload: b"../../etc/passwd".to_vec(),
        format: 32,
        width: 0,
        height: 0,
        transmission: KittyTransmission::File,
    };

    let result = term.kitty_store_image(p);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(
        err.contains("path traversal"),
        "path traversal should be rejected, got: {}",
        err
    );
}
