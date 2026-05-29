//! Shared `galleries` row shape, used by both the public and admin handlers so
//! the column order stays in one place (it is positional in `sqlx::query_as`).

use syle_types::Gallery;
use uuid::Uuid;

/// A `galleries` row in column order. Keep in sync with [`GALLERY_COLS`].
pub(crate) type GalleryRow =
    (Uuid, String, String, i32, bool, String, String, String, Option<i32>);

/// Column list for every gallery `SELECT`/`RETURNING`, in [`GalleryRow`] order.
pub(crate) const GALLERY_COLS: &str =
    "id, slug, title, position, published, description, notes, category, year";

pub(crate) fn into_gallery(
    (id, slug, title, position, published, description, notes, category, year): GalleryRow,
) -> Gallery {
    Gallery {
        id,
        slug,
        title,
        position,
        published,
        description,
        notes,
        category,
        year,
    }
}
