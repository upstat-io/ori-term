//! Kitty graphics storage helpers — decode, direct-store, file-read.

use std::io::Read;
use std::sync::Arc;

#[cfg(unix)]
use std::os::unix::fs::OpenOptionsExt;

use crate::effect::sink::EffectSink;
use crate::image::kitty::KittyTransmission;
use crate::image::{ImageData, ImageId, ImageSource, decode_to_rgba};
use crate::term::Term;

use super::KittyStoreParams;
use super::prepare::{kitty_decode_pixels, prepare_image_bytes};

/// Error from `kitty_store_image` / `kitty_store_from_file`. `Reply`
/// carries store-specific stringly-typed reply text (EBADF, EBIG, EIO,
/// ENOMEM, EINVAL-shaped strings produced inside the store layer).
#[derive(Debug)]
pub(crate) enum KittyStoreError {
    /// Store-layer stringly-typed reply text.
    Reply(String),
}

impl std::fmt::Display for KittyStoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Reply(s) => f.write_str(s),
        }
    }
}

/// RAII guard that removes a file on Drop when armed. Used by
/// `kitty_store_from_file` to enforce the kitty `t=t` (`TempFile`)
/// "delete after consume" semantic on EVERY exit path — success,
/// oversized rejection, IO error, non-regular rejection, stat
/// failure — without duplicating the cleanup at each return site.
struct TempFileGuard<'a> {
    path: &'a std::path::Path,
    armed: bool,
}

impl<'a> TempFileGuard<'a> {
    fn new(path: &'a std::path::Path, transmission: KittyTransmission) -> Self {
        Self {
            path,
            armed: transmission == KittyTransmission::TempFile,
        }
    }
}

impl Drop for TempFileGuard<'_> {
    fn drop(&mut self) {
        if self.armed {
            let _ = std::fs::remove_file(self.path);
        }
    }
}

/// Pre-compute the expected post-decode payload size for raw-pixel formats
/// (`f=32` → `w*h*4`, `f=24` → `w*h*3`). Returns `None` for `f=100` (PNG)
/// and any other format where the decoded size is not derivable from the
/// `s=`/`v=` control fields up front; the caller's `max_bytes` then bounds
/// the helper's output. Overflow saturates to `usize::MAX` so the helper's
/// own cap clamp handles oversized requests with `EBIG`.
pub(super) fn expected_decoded_size_for_format(
    format: u32,
    width: u32,
    height: u32,
) -> Option<usize> {
    let channels: usize = match format {
        24 => 3,
        32 => 4,
        _ => return None,
    };
    Some(
        (width as usize)
            .checked_mul(height as usize)
            .and_then(|wh| wh.checked_mul(channels))
            .unwrap_or(usize::MAX),
    )
}

/// Open `path` with `O_NONBLOCK` on Unix, fstat the OPENED descriptor, and
/// reject non-regular files (FIFO, socket, char-device, dir, Windows
/// reparse-point that doesn't resolve to a regular file). The returned
/// metadata comes from the descriptor (not the path) so it's free of
/// the path-based TOCTOU window between stat and open. `O_NONBLOCK` is
/// harmless on regular files post-fstat; on Unix it prevents indefinite
/// blocking when the path resolves to a FIFO without a writer.
fn open_regular_file(path: &std::path::Path) -> Result<(std::fs::File, std::fs::Metadata), String> {
    let mut opts = std::fs::OpenOptions::new();
    opts.read(true);
    #[cfg(unix)]
    opts.custom_flags(libc::O_NONBLOCK);
    let file = opts
        .open(path)
        .map_err(|e| format!("EBADF: failed to open file: {e}"))?;
    let meta = file
        .metadata()
        .map_err(|e| format!("EBADF: failed to stat file: {e}"))?;
    if !meta.file_type().is_file() {
        return Err("EINVAL: path is not a regular file".to_string());
    }
    Ok((file, meta))
}

impl<S: EffectSink> Term<S> {
    /// Decode and store image data in the cache.
    pub(super) fn kitty_store_image(&mut self, p: KittyStoreParams) -> Result<(), KittyStoreError> {
        let (pixel_data, source) = match p.transmission {
            KittyTransmission::Direct => (p.payload, ImageSource::Direct),
            KittyTransmission::File | KittyTransmission::TempFile => {
                return self.kitty_store_from_file(&p);
            }
            KittyTransmission::SharedMemory => {
                return Err(KittyStoreError::Reply(
                    "EINVAL: shared memory transmission not yet supported".to_string(),
                ));
            }
        };

        let expected_size = expected_decoded_size_for_format(p.format, p.width, p.height);
        let max_bytes = self.image_cache().max_single_image_bytes();
        let pixel_data = prepare_image_bytes(pixel_data, p.compression, expected_size, max_bytes)?;

        let (rgba_data, w, h) = kitty_decode_pixels(pixel_data, p.format, p.width, p.height)
            .map_err(KittyStoreError::Reply)?;

        let img = ImageData {
            id: ImageId(p.image_id),
            width: w,
            height: h,
            data: Arc::new(rgba_data),
            pixel_generation: 0,
            format: crate::image::ImageFormat::Rgba,
            source,
            last_accessed: 0,
            image_number: p.image_number,
        };

        self.image_cache_mut()
            .store(img)
            .map_err(|e| KittyStoreError::Reply(format!("ENOMEM: {e}")))?;

        Ok(())
    }

    // kitty_decode_pixels + kitty_decode_pixels_inner extracted to free fns
    // at `super::prepare::{kitty_decode_pixels, kitty_decode_pixels_inner}` so
    // the worker-thread runner at `crate::image::worker_pipeline::run_image_decode`
    // can invoke them without coupling to `Term`.

    /// Store image from a file path (t=f or t=t transmission).
    pub(super) fn kitty_store_from_file(
        &mut self,
        p: &KittyStoreParams,
    ) -> Result<(), KittyStoreError> {
        let path_str = std::str::from_utf8(&p.payload).map_err(|_e| {
            KittyStoreError::Reply("EINVAL: file path is not valid UTF-8".to_string())
        })?;

        let path = std::path::Path::new(path_str);

        // Reject path traversal via component inspection (not string matching,
        // which `contains("..")` would get wrong on paths like `foo/..bar`).
        for component in path.components() {
            if matches!(component, std::path::Component::ParentDir) {
                return Err(KittyStoreError::Reply(
                    "EINVAL: path traversal not allowed".to_string(),
                ));
            }
        }

        let max_bytes = self.image_cache().max_single_image_bytes();

        // RAII guard armed FIRST — covers stat-failure, non-regular
        // rejection, IO-error, and oversized-rejection paths uniformly.
        // Replaces the duplicated `if transmission == TempFile { remove_file }`
        // calls and fixes the early-return temp-file leak that the original
        // `?` on `std::fs::read` had.
        let _guard = TempFileGuard::new(path, p.transmission);

        let (file, meta) = open_regular_file(path).map_err(KittyStoreError::Reply)?;

        // Fast-path size preflight on the verified regular-file descriptor.
        // The bounded read below remains as TOCTOU defense-in-depth (file
        // can grow between this check and read). saturating_add guards
        // against the pathological max_bytes == usize::MAX config.
        if meta.len() > max_bytes as u64 {
            return Err(KittyStoreError::Reply(
                "EBIG: file exceeds max image size".to_string(),
            ));
        }

        let mut file_data = Vec::with_capacity(meta.len() as usize);
        file.take((max_bytes as u64).saturating_add(1))
            .read_to_end(&mut file_data)
            .map_err(|e| KittyStoreError::Reply(format!("EIO: failed to read file: {e}")))?;

        if file_data.len() > max_bytes {
            return Err(KittyStoreError::Reply(
                "EBIG: file exceeds max image size".to_string(),
            ));
        }

        let expected_size = expected_decoded_size_for_format(p.format, p.width, p.height);
        let file_data = prepare_image_bytes(file_data, p.compression, expected_size, max_bytes)?;

        let source = ImageSource::File(path.to_path_buf());

        let (rgba_data, w, h) = if p.format == 24 || p.format == 32 {
            kitty_decode_pixels(file_data, p.format, p.width, p.height)
                .map_err(KittyStoreError::Reply)?
        } else {
            decode_to_rgba(&file_data)
                .map_err(|e| KittyStoreError::Reply(format!("EINVAL: image decode failed: {e}")))?
        };

        let img = ImageData {
            id: ImageId(p.image_id),
            width: w,
            height: h,
            data: Arc::new(rgba_data),
            pixel_generation: 0,
            format: crate::image::ImageFormat::Rgba,
            source,
            last_accessed: 0,
            image_number: p.image_number,
        };

        self.image_cache_mut()
            .store(img)
            .map_err(|e| KittyStoreError::Reply(format!("ENOMEM: {e}")))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests;
