//! Public read endpoints. Cacheable, unauthenticated; consumed by the Astro
//! site at build and behind the CDN.

use crate::error::ApiError;
use crate::state::AppState;
use axum::extract::{Path, State};
use axum::Json;
use syle_types::{Gallery, GalleryDetail, ImageFormat, ImageVariant, Photo};
use uuid::Uuid;

fn parse_format(s: &str) -> Result<ImageFormat, ApiError> {
    match s {
        "avif" => Ok(ImageFormat::Avif),
        "jpeg" => Ok(ImageFormat::Jpeg),
        _ => Err(ApiError::Internal),
    }
}

/// Published galleries, ordered for display.
pub async fn list_galleries(
    State(state): State<AppState>,
) -> Result<Json<Vec<Gallery>>, ApiError> {
    let rows: Vec<(Uuid, String, String, i32, bool)> = sqlx::query_as(
        "SELECT id, slug, title, position, published FROM galleries \
         WHERE published = TRUE ORDER BY position, slug",
    )
    .fetch_all(&state.pool)
    .await?;

    let galleries = rows
        .into_iter()
        .map(|(id, slug, title, position, published)| Gallery {
            id,
            slug,
            title,
            position,
            published,
        })
        .collect();
    Ok(Json(galleries))
}

/// One published gallery with its ordered photos and their variants.
pub async fn get_gallery(
    State(state): State<AppState>,
    Path(slug): Path<String>,
) -> Result<Json<GalleryDetail>, ApiError> {
    let (id, slug, title, position, published): (Uuid, String, String, i32, bool) =
        sqlx::query_as(
            "SELECT id, slug, title, position, published FROM galleries \
             WHERE slug = $1 AND published = TRUE",
        )
        .bind(&slug)
        .fetch_optional(&state.pool)
        .await?
        .ok_or(ApiError::NotFound)?;
    let gallery = Gallery { id, slug, title, position, published };

    let photo_rows: Vec<(Uuid, Uuid, String, String, i32, i32, i32)> = sqlx::query_as(
        "SELECT id, gallery_id, alt, thumbhash, width, height, position \
         FROM photos WHERE gallery_id = $1 ORDER BY position, id",
    )
    .bind(id)
    .fetch_all(&state.pool)
    .await?;

    let mut photos = Vec::with_capacity(photo_rows.len());
    for (pid, gallery_id, alt, thumbhash, width, height, position) in photo_rows {
        let var_rows: Vec<(String, i32, String)> = sqlx::query_as(
            "SELECT format, width, path FROM photo_variants \
             WHERE photo_id = $1 ORDER BY format, width",
        )
        .bind(pid)
        .fetch_all(&state.pool)
        .await?;
        let mut variants = Vec::with_capacity(var_rows.len());
        for (fmt, w, path) in var_rows {
            variants.push(ImageVariant {
                format: parse_format(&fmt)?,
                width: w as u32,
                path,
            });
        }
        photos.push(Photo {
            id: pid,
            gallery_id,
            alt,
            thumbhash,
            width: width as u32,
            height: height as u32,
            position,
            variants,
        });
    }

    Ok(Json(GalleryDetail { gallery, photos }))
}
