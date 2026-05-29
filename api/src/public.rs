//! Public read endpoints. Cacheable, unauthenticated; consumed by the Astro
//! site at build and behind the CDN.

use crate::error::ApiError;
use crate::gallery_row::{into_gallery, GalleryRow, GALLERY_COLS};
use crate::state::AppState;
use axum::extract::{Path, State};
use axum::Json;
use syle_types::{
    Block, BlogPost, Gallery, GalleryDetail, ImageFormat, ImageVariant, Photo, PostStatus,
};
use uuid::Uuid;

fn parse_format(s: &str) -> Result<ImageFormat, ApiError> {
    match s {
        "avif" => Ok(ImageFormat::Avif),
        "jpeg" => Ok(ImageFormat::Jpeg),
        _ => Err(ApiError::Internal),
    }
}

fn parse_status(s: &str) -> PostStatus {
    match s {
        "published" => PostStatus::Published,
        _ => PostStatus::Draft,
    }
}

type PostRow = (Uuid, String, String, String, String, Option<i64>);

/// Columns selected for a post, in `PostRow` order (body is JSON in `body_blocks`).
const POST_COLS: &str = "id, slug, title, body_blocks, status, published_at";

fn into_post((id, slug, title, body_blocks, status, published_at): PostRow) -> BlogPost {
    let blocks: Vec<Block> = serde_json::from_str(&body_blocks).unwrap_or_default();
    let body_html = syle_render::render_blocks(&blocks);
    BlogPost {
        id,
        slug,
        title,
        blocks,
        body_html,
        status: parse_status(&status),
        published_at,
    }
}

/// Published galleries, ordered for display.
pub async fn list_galleries(
    State(state): State<AppState>,
) -> Result<Json<Vec<Gallery>>, ApiError> {
    let rows: Vec<GalleryRow> = sqlx::query_as(&format!(
        "SELECT {GALLERY_COLS} FROM galleries \
         WHERE published = TRUE ORDER BY position, slug"
    ))
    .fetch_all(&state.pool)
    .await?;

    let galleries = rows.into_iter().map(into_gallery).collect();
    Ok(Json(galleries))
}

/// One published gallery with its ordered photos and their variants.
pub async fn get_gallery(
    State(state): State<AppState>,
    Path(slug): Path<String>,
) -> Result<Json<GalleryDetail>, ApiError> {
    let row: GalleryRow = sqlx::query_as(&format!(
        "SELECT {GALLERY_COLS} FROM galleries WHERE slug = $1 AND published = TRUE"
    ))
    .bind(&slug)
    .fetch_optional(&state.pool)
    .await?
    .ok_or(ApiError::NotFound)?;
    let gallery = into_gallery(row);
    let id = gallery.id;

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

/// Published posts, newest first.
pub async fn list_posts(
    State(state): State<AppState>,
) -> Result<Json<Vec<BlogPost>>, ApiError> {
    let rows: Vec<PostRow> = sqlx::query_as(&format!(
        "SELECT {POST_COLS} FROM blog_posts \
         WHERE status = 'published' ORDER BY published_at DESC NULLS LAST"
    ))
    .fetch_all(&state.pool)
    .await?;
    Ok(Json(rows.into_iter().map(into_post).collect()))
}

/// One published post by slug; drafts are 404 to the public.
pub async fn get_post(
    State(state): State<AppState>,
    Path(slug): Path<String>,
) -> Result<Json<BlogPost>, ApiError> {
    let row: PostRow = sqlx::query_as(&format!(
        "SELECT {POST_COLS} FROM blog_posts WHERE slug = $1 AND status = 'published'"
    ))
    .bind(&slug)
    .fetch_optional(&state.pool)
    .await?
    .ok_or(ApiError::NotFound)?;
    Ok(Json(into_post(row)))
}
