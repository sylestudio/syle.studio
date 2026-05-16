use super::{ext, parse_format, remove_media_file};
use crate::auth::AuthUser;
use crate::error::ApiError;
use crate::state::AppState;
use axum::extract::{Multipart, Path, State};
use axum::Json;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use syle_core::ingest::ingest;
use syle_types::{ImageVariant, Photo, UpdatePhoto};
use uuid::Uuid;

/// Responsive widths requested from the pipeline; the pipeline drops any that
/// would upscale the source.
const TARGET_WIDTHS: &[u32] = &[480, 960, 1440, 2400];

type PhotoRow = (Uuid, Uuid, String, String, i32, i32, i32);

async fn load_photo(state: &AppState, id: Uuid) -> Result<Photo, ApiError> {
    let row: Option<PhotoRow> = sqlx::query_as(
        "SELECT id, gallery_id, alt, thumbhash, width, height, position \
         FROM photos WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(&state.pool)
    .await?;
    let (id, gallery_id, alt, thumbhash, width, height, position) =
        row.ok_or(ApiError::NotFound)?;
    let vars: Vec<(String, i32, String)> = sqlx::query_as(
        "SELECT format, width, path FROM photo_variants \
         WHERE photo_id = $1 ORDER BY format, width",
    )
    .bind(id)
    .fetch_all(&state.pool)
    .await?;
    Ok(Photo {
        id,
        gallery_id,
        alt,
        thumbhash,
        width: width as u32,
        height: height as u32,
        position,
        variants: vars
            .into_iter()
            .map(|(f, w, path)| ImageVariant {
                format: parse_format(&f),
                width: w as u32,
                path,
            })
            .collect(),
    })
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

pub async fn update_photo(
    _user: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdatePhoto>,
) -> Result<Json<Photo>, ApiError> {
    let done = sqlx::query(
        "UPDATE photos SET \
           alt = COALESCE($2, alt), \
           position = COALESCE($3, position) \
         WHERE id = $1",
    )
    .bind(id)
    .bind(req.alt)
    .bind(req.position)
    .execute(&state.pool)
    .await?;
    if done.rows_affected() == 0 {
        return Err(ApiError::NotFound);
    }
    Ok(Json(load_photo(&state, id).await?))
}

pub async fn delete_photo(
    _user: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<(), ApiError> {
    let paths: Vec<(String,)> =
        sqlx::query_as("SELECT path FROM photo_variants WHERE photo_id = $1")
            .bind(id)
            .fetch_all(&state.pool)
            .await?;
    for (p,) in &paths {
        remove_media_file(&state.media_dir, p);
    }
    let done = sqlx::query("DELETE FROM photos WHERE id = $1")
        .bind(id)
        .execute(&state.pool)
        .await?;
    if done.rows_affected() == 0 {
        return Err(ApiError::NotFound);
    }
    Ok(())
}
