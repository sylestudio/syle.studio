//! Authenticated CRM write + curation endpoints. Every handler takes
//! `AuthUser`, so the session guard is enforced by the type system.

mod galleries;
mod photos;
mod posts;

pub use galleries::*;
pub use photos::*;
pub use posts::*;

use std::path::Path;
use syle_types::ImageFormat;

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

/// Best-effort delete of a stored derivative given its public `/media/...` path.
pub(crate) fn remove_media_file(media_dir: &Path, public_path: &str) {
    let rel = public_path.trim_start_matches('/');
    let _ = std::fs::remove_file(media_dir.join(rel));
}
