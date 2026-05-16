//! Authenticated CRM write endpoints. Every handler takes `AuthUser`, so the
//! session guard is enforced by the type system, not by remembering to check.

use crate::auth::AuthUser;
use crate::error::ApiError;
use crate::state::AppState;
use axum::extract::{Multipart, State};
use axum::Json;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::time::{SystemTime, UNIX_EPOCH};
use syle_core::ingest::ingest;
use syle_types::{
    BlogPost, Gallery, ImageFormat, ImageVariant, NewGallery, NewPost, Photo, PostStatus,
};
use uuid::Uuid;

/// Responsive widths requested from the pipeline; the pipeline drops any that
/// would upscale the source.
const TARGET_WIDTHS: &[u32] = &[480, 960, 1440, 2400];

fn ext(f: ImageFormat) -> &'static str {
    match f {
        ImageFormat::Avif => "avif",
        ImageFormat::Jpeg => "jpeg",
    }
}

/// All galleries including unpublished (CRM list).
pub async fn list_galleries(
    _user: AuthUser,
    State(state): State<AppState>,
) -> Result<Json<Vec<Gallery>>, ApiError> {
    let rows: Vec<(Uuid, String, String, i32, bool)> = sqlx::query_as(
        "SELECT id, slug, title, position, published FROM galleries \
         ORDER BY position, slug",
    )
    .fetch_all(&state.pool)
    .await?;
    Ok(Json(
        rows.into_iter()
            .map(|(id, slug, title, position, published)| Gallery {
                id,
                slug,
                title,
                position,
                published,
            })
            .collect(),
    ))
}

/// All posts including drafts (CRM list).
pub async fn list_posts(
    _user: AuthUser,
    State(state): State<AppState>,
) -> Result<Json<Vec<BlogPost>>, ApiError> {
    let rows: Vec<(Uuid, String, String, String, String, Option<i64>)> = sqlx::query_as(
        "SELECT id, slug, title, body_md, status, published_at FROM blog_posts \
         ORDER BY COALESCE(published_at, 0) DESC, slug",
    )
    .fetch_all(&state.pool)
    .await?;
    Ok(Json(
        rows.into_iter()
            .map(|(id, slug, title, body_md, status, published_at)| BlogPost {
                id,
                slug,
                title,
                body_md,
                status: if status == "published" {
                    PostStatus::Published
                } else {
                    PostStatus::Draft
                },
                published_at,
            })
            .collect(),
    ))
}

pub async fn create_gallery(
    _user: AuthUser,
    State(state): State<AppState>,
    Json(req): Json<NewGallery>,
) -> Result<Json<Gallery>, ApiError> {
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO galleries (id, slug, title, position, published) \
         VALUES ($1,$2,$3,$4,$5)",
    )
    .bind(id)
    .bind(&req.slug)
    .bind(&req.title)
    .bind(req.position)
    .bind(req.published)
    .execute(&state.pool)
    .await
    .map_err(|e| match e {
        sqlx::Error::Database(db) if db.is_unique_violation() => ApiError::BadRequest,
        other => other.into(),
    })?;

    Ok(Json(Gallery {
        id,
        slug: req.slug,
        title: req.title,
        position: req.position,
        published: req.published,
    }))
}

pub async fn create_post(
    _user: AuthUser,
    State(state): State<AppState>,
    Json(req): Json<NewPost>,
) -> Result<Json<BlogPost>, ApiError> {
    let (status_str, published_at) = match req.status {
        PostStatus::Published => {
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64;
            ("published", Some(now))
        }
        PostStatus::Draft => ("draft", None),
    };
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO blog_posts \
         (id, slug, title, body_md, status, published_at) \
         VALUES ($1,$2,$3,$4,$5,$6)",
    )
    .bind(id)
    .bind(&req.slug)
    .bind(&req.title)
    .bind(&req.body_md)
    .bind(status_str)
    .bind(published_at)
    .execute(&state.pool)
    .await
    .map_err(|e| match e {
        sqlx::Error::Database(db) if db.is_unique_violation() => ApiError::BadRequest,
        other => other.into(),
    })?;

    Ok(Json(BlogPost {
        id,
        slug: req.slug,
        title: req.title,
        body_md: req.body_md,
        status: req.status,
        published_at,
    }))
}

/// Multipart upload: `gallery_id`, `alt`, `file`. Runs the pipeline, writes
/// content-addressed derivatives under `media_dir`, persists photo + variants.
pub async fn upload_photo(
    _user: AuthUser,
    State(state): State<AppState>,
    mut mp: Multipart,
) -> Result<Json<Photo>, ApiError> {
    let mut gallery_id: Option<Uuid> = None;
    let mut alt = String::new();
    let mut bytes: Option<Vec<u8>> = None;

    while let Some(field) = mp.next_field().await.map_err(|_| ApiError::BadRequest)? {
        match field.name() {
            Some("gallery_id") => {
                let t = field.text().await.map_err(|_| ApiError::BadRequest)?;
                gallery_id = Uuid::parse_str(&t).ok();
            }
            Some("alt") => {
                alt = field.text().await.map_err(|_| ApiError::BadRequest)?;
            }
            Some("file") => {
                bytes = Some(
                    field
                        .bytes()
                        .await
                        .map_err(|_| ApiError::BadRequest)?
                        .to_vec(),
                );
            }
            _ => {}
        }
    }

    let gallery_id = gallery_id.ok_or(ApiError::BadRequest)?;
    let src = bytes.ok_or(ApiError::BadRequest)?;
    let out = ingest(&src, TARGET_WIDTHS).map_err(|_| ApiError::BadRequest)?;

    let (count,): (i64,) =
        sqlx::query_as("SELECT count(*) FROM photos WHERE gallery_id = $1")
            .bind(gallery_id)
            .fetch_one(&state.pool)
            .await?;
    let position = count as i32;

    let mut hasher = DefaultHasher::new();
    src.hash(&mut hasher);
    let key = format!("{:016x}", hasher.finish());

    let pid = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO photos \
         (id, gallery_id, alt, thumbhash, width, height, position) \
         VALUES ($1,$2,$3,$4,$5,$6,$7)",
    )
    .bind(pid)
    .bind(gallery_id)
    .bind(&alt)
    .bind(&out.thumbhash)
    .bind(out.width as i32)
    .bind(out.height as i32)
    .bind(position)
    .execute(&state.pool)
    .await?;

    let mut variants = Vec::with_capacity(out.derivatives.len());
    for d in &out.derivatives {
        let e = ext(d.format);
        let rel = format!("media/{e}/{key}_{}.{e}", d.width);
        let abs = state.media_dir.join(&rel);
        if let Some(parent) = abs.parent() {
            std::fs::create_dir_all(parent).map_err(|_| ApiError::Internal)?;
        }
        std::fs::write(&abs, &d.bytes).map_err(|_| ApiError::Internal)?;

        let path = format!("/{rel}");
        sqlx::query(
            "INSERT INTO photo_variants (photo_id, format, width, path) \
             VALUES ($1,$2,$3,$4)",
        )
        .bind(pid)
        .bind(e)
        .bind(d.width as i32)
        .bind(&path)
        .execute(&state.pool)
        .await?;

        variants.push(ImageVariant {
            format: d.format,
            width: d.width,
            path,
        });
    }

    Ok(Json(Photo {
        id: pid,
        gallery_id,
        alt,
        thumbhash: out.thumbhash,
        width: out.width,
        height: out.height,
        position,
        variants,
    }))
}
