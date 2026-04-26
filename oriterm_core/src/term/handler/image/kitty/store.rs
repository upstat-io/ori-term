//! Kitty graphics storage helpers — decode, direct-store, file-read.

use std::io::Read;
use std::sync::Arc;

#[cfg(unix)]
use std::os::unix::fs::OpenOptionsExt;

use crate::effect::sink::EffectSink;
use crate::image::kitty::KittyTransmission;
use crate::image::{ImageData, ImageId, ImageSource, decode_to_rgba, rgb_to_rgba};
use crate::term::Term;

use super::KittyStoreParams;

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

impl<S: EffectSink> Term<S> {
    /// Decode and store image data in the cache.
    pub(super) fn kitty_store_image(&mut self, p: KittyStoreParams) -> Result<(), String> {
        let (pixel_data, source) = match p.transmission {
            KittyTransmission::Direct => (p.payload, ImageSource::Direct),
            KittyTransmission::File | KittyTransmission::TempFile => {
                return self.kitty_store_from_file(&p);
            }
            KittyTransmission::SharedMemory => {
                return Err("EINVAL: shared memory transmission not yet supported".to_string());
            }
        };

        let (rgba_data, w, h) = Self::kitty_decode_pixels(pixel_data, p.format, p.width, p.height)?;

        let img = ImageData {
            id: ImageId(p.image_id),
            width: w,
            height: h,
            data: Arc::new(rgba_data),
            format: crate::image::ImageFormat::Rgba,
            source,
            last_accessed: 0,
            image_number: p.image_number,
        };

        self.image_cache_mut()
            .store(img)
            .map_err(|e| format!("ENOMEM: {e}"))?;

        Ok(())
    }

    /// Decode pixel data from format code to RGBA.
    pub(super) fn kitty_decode_pixels(
        payload: Vec<u8>,
        format: u32,
        width: u32,
        height: u32,
    ) -> Result<(Vec<u8>, u32, u32), String> {
        match format {
            32 => {
                if width == 0 || height == 0 {
                    return Err("EINVAL: missing s= or v= for raw RGBA".to_string());
                }
                let expected = (width as usize) * (height as usize) * 4;
                if payload.len() != expected {
                    return Err(format!(
                        "EINVAL: RGBA payload size {} != expected {expected}",
                        payload.len(),
                    ));
                }
                Ok((payload, width, height))
            }
            24 => {
                if width == 0 || height == 0 {
                    return Err("EINVAL: missing s= or v= for raw RGB".to_string());
                }
                rgb_to_rgba(&payload)
                    .map(|rgba| (rgba, width, height))
                    .ok_or_else(|| "EINVAL: RGB payload length not a multiple of 3".to_string())
            }
            100 => decode_to_rgba(&payload).map_err(|e| format!("EINVAL: PNG decode failed: {e}")),
            _ => Err(format!("EINVAL: unsupported format f={format}")),
        }
    }

    /// Store image from a file path (t=f or t=t transmission).
    pub(super) fn kitty_store_from_file(&mut self, p: &KittyStoreParams) -> Result<(), String> {
        let path_str = std::str::from_utf8(&p.payload)
            .map_err(|_e| "EINVAL: file path is not valid UTF-8".to_string())?;

        let path = std::path::Path::new(path_str);

        // Reject path traversal via component inspection (not string matching,
        // which `contains("..")` would get wrong on paths like `foo/..bar`).
        for component in path.components() {
            if matches!(component, std::path::Component::ParentDir) {
                return Err("EINVAL: path traversal not allowed".to_string());
            }
        }

        let max_bytes = self.image_cache().max_single_image_bytes();

        // RAII guard armed FIRST — covers stat-failure, non-regular
        // rejection, IO-error, and oversized-rejection paths uniformly.
        // Replaces the duplicated `if transmission == TempFile { remove_file }`
        // calls and fixes the early-return temp-file leak that the original
        // `?` on `std::fs::read` had.
        let _guard = TempFileGuard::new(path, p.transmission);

        // Open with O_NONBLOCK on Unix to prevent indefinite blocking on a
        // FIFO-without-writer that the user's program may have planted at
        // the path. After fstat confirms the descriptor is a regular file,
        // O_NONBLOCK is harmless on regular file reads. On Windows, regular
        // file opens cannot block this way.
        let mut opts = std::fs::OpenOptions::new();
        opts.read(true);
        #[cfg(unix)]
        opts.custom_flags(libc::O_NONBLOCK);
        let file = opts
            .open(path)
            .map_err(|e| format!("EIO: failed to open file: {e}"))?;

        // fstat the OPENED descriptor (NOT the path) — eliminates the
        // path-based TOCTOU window between stat and open. Reject anything
        // other than a regular file.
        let meta = file
            .metadata()
            .map_err(|e| format!("EIO: failed to stat file: {e}"))?;
        if !meta.file_type().is_file() {
            return Err("EINVAL: path is not a regular file".to_string());
        }

        // Fast-path size preflight on the verified regular-file descriptor.
        // The bounded read below remains as TOCTOU defense-in-depth (file
        // can grow between this check and read).
        if meta.len() > max_bytes as u64 {
            return Err("ENOMEM: file exceeds max image size".to_string());
        }

        // Bounded read — caps at max_bytes + 1 bytes via Take. The +1 is
        // the detection byte: if `file_data.len() > max_bytes` post-read,
        // the file grew between the preflight and the read. saturating_add
        // guards against the pathological max_bytes == usize::MAX config.
        let mut file_data = Vec::with_capacity(meta.len() as usize);
        file.take((max_bytes as u64).saturating_add(1))
            .read_to_end(&mut file_data)
            .map_err(|e| format!("EIO: failed to read file: {e}"))?;

        if file_data.len() > max_bytes {
            return Err("ENOMEM: file exceeds max image size".to_string());
        }

        let source = ImageSource::File(path.to_path_buf());

        let (rgba_data, w, h) = if p.format == 24 || p.format == 32 {
            Self::kitty_decode_pixels(file_data, p.format, p.width, p.height)?
        } else {
            decode_to_rgba(&file_data).map_err(|e| format!("EINVAL: image decode failed: {e}"))?
        };

        let img = ImageData {
            id: ImageId(p.image_id),
            width: w,
            height: h,
            data: Arc::new(rgba_data),
            format: crate::image::ImageFormat::Rgba,
            source,
            last_accessed: 0,
            image_number: p.image_number,
        };

        self.image_cache_mut()
            .store(img)
            .map_err(|e| format!("ENOMEM: {e}"))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests;
