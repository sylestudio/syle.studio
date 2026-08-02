//! Authenticated CRM write + curation endpoints. Every handler takes
//! `AuthUser`, so the session guard is enforced by the type system.

mod assets;
mod galleries;
mod photos;
mod posts;

pub use assets::*;
pub use galleries::*;
pub use photos::*;
pub use posts::*;

use std::fs::OpenOptions;
use std::io::{self, Write};
use std::path::{Component, Path, PathBuf};
use syle_types::ImageFormat;
use uuid::Uuid;

/// Parse the DB's format text into the typed enum.
pub(crate) fn parse_format(s: &str) -> ImageFormat {
    match s {
        "avif" => ImageFormat::Avif,
        _ => ImageFormat::Jpeg,
    }
}

pub(crate) fn ext(f: ImageFormat) -> &'static str {
    match f {
        ImageFormat::Avif => "avif",
        ImageFormat::Jpeg => "jpeg",
    }
}

struct StagedFile {
    temp: PathBuf,
    final_path: PathBuf,
    published: bool,
}

/// A set of media writes staged beside their final paths. Publishing uses a
/// same-filesystem rename for each file; unless `keep` is called, `Drop` rolls
/// back both staged files and any files already published by this batch.
///
/// Callers must give every upload its own UUID in the final path. That avoids
/// shared-file lifetime bugs when two photo records contain identical pixels.
pub(crate) struct MediaBatch {
    files: Vec<StagedFile>,
    keep: bool,
}

impl MediaBatch {
    pub(crate) fn stage<'a>(
        media_dir: &Path,
        files: impl IntoIterator<Item = (&'a str, &'a [u8])>,
    ) -> io::Result<Self> {
        let mut batch = Self {
            files: Vec::new(),
            keep: false,
        };

        for (rel, bytes) in files {
            let rel_path = Path::new(rel);
            if rel_path.is_absolute()
                || rel_path
                    .components()
                    .any(|component| !matches!(component, Component::Normal(_)))
            {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "media path must be relative and normalized",
                ));
            }

            let final_path = media_dir.join(rel_path);
            if final_path.exists() {
                return Err(io::Error::new(
                    io::ErrorKind::AlreadyExists,
                    "unique media path already exists",
                ));
            }
            let parent = final_path.parent().ok_or_else(|| {
                io::Error::new(io::ErrorKind::InvalidInput, "media path has no parent")
            })?;
            std::fs::create_dir_all(parent)?;

            let name = final_path
                .file_name()
                .and_then(|name| name.to_str())
                .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "invalid filename"))?;
            let temp = parent.join(format!(".{name}.{}.tmp", Uuid::new_v4().simple()));
            let mut file = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&temp)?;
            batch.files.push(StagedFile {
                temp,
                final_path,
                published: false,
            });
            file.write_all(bytes)?;
            file.sync_all()?;
        }

        Ok(batch)
    }

    pub(crate) fn publish(&mut self) -> io::Result<()> {
        for file in &mut self.files {
            std::fs::rename(&file.temp, &file.final_path)?;
            file.published = true;
        }
        Ok(())
    }

    pub(crate) fn keep(mut self) {
        self.keep = true;
    }
}

impl Drop for MediaBatch {
    fn drop(&mut self) {
        if self.keep {
            return;
        }
        for file in &self.files {
            let _ = std::fs::remove_file(&file.temp);
            if file.published {
                let _ = std::fs::remove_file(&file.final_path);
            }
        }
    }
}

/// Best-effort delete of a stored derivative given its public `/media/...` path.
pub(crate) fn remove_media_file(media_dir: &Path, public_path: &str) {
    let rel = public_path.trim_start_matches('/');
    let _ = std::fs::remove_file(media_dir.join(rel));
}

#[cfg(test)]
mod tests {
    use super::MediaBatch;

    #[test]
    fn failed_publish_rolls_back_already_renamed_files() {
        let root = tempfile::tempdir().unwrap();
        let one = b"one".as_slice();
        let two = b"two".as_slice();
        let mut batch = MediaBatch::stage(
            root.path(),
            [("media/jpeg/one.jpeg", one), ("media/jpeg/two.jpeg", two)],
        )
        .unwrap();

        let obstruction = root.path().join("media/jpeg/two.jpeg");
        std::fs::create_dir(&obstruction).unwrap();
        std::fs::write(obstruction.join("keep"), b"x").unwrap();

        assert!(batch.publish().is_err());
        drop(batch);
        assert!(!root.path().join("media/jpeg/one.jpeg").exists());
        assert!(obstruction.join("keep").exists());
    }
}
