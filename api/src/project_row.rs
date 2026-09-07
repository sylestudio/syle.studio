//! Shared `projects` row shape for public and authenticated handlers.

use sqlx::types::Json;
use syle_types::{Project, UploadedImage};
use uuid::Uuid;

/// Keep this tuple in the same order as [`PROJECT_COLS`].
pub(crate) type ProjectRow = (
    Uuid,
    String,
    String,
    String,
    i32,
    bool,
    Option<Json<UploadedImage>>,
);

pub(crate) const PROJECT_COLS: &str = "id, title, url, category, position, published, cover";

pub(crate) fn into_project(
    (id, title, url, category, position, published, cover): ProjectRow,
) -> Project {
    Project {
        id,
        title,
        url,
        category,
        position,
        published,
        cover: cover.map(|Json(image)| image),
    }
}
