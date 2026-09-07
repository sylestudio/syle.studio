use super::{ext, parse_format, remove_media_file, MediaBatch};
use crate::auth::AuthUser;
use crate::error::ApiError;
use crate::state::AppState;
use axum::extract::{Multipart, Path, State};
use axum::Json;
use syle_core::ingest::{content_key, ingest};
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

    while let Some(mut field) = mp.next_field().await.map_err(|_| ApiError::BadRequest)? {
        match field.name() {
            Some("gallery_id") => {
                let t = field.text().await.map_err(|_| ApiError::BadRequest)?;
                gallery_id = Uuid::parse_str(&t).ok();
            }
            Some("alt") => {
                alt = field.text().await.map_err(|_| ApiError::BadRequest)?;
            }
            Some("file") => {
                // Stream the field chunk-by-chunk into one buffer instead of
                // `bytes().to_vec()` (which double-copies the whole upload).
                // The route's DefaultBodyLimit gates the request body; this
                // per-field counter is the belt-and-suspenders guard.
                let mut buf: Vec<u8> = Vec::new();
                while let Some(chunk) = field.chunk().await.map_err(|_| ApiError::BadRequest)? {
                    if buf.len() + chunk.len() > crate::MAX_UPLOAD_BYTES {
                        return Err(ApiError::BadRequest);
                    }
                    buf.extend_from_slice(&chunk);
                }
                bytes = Some(buf);
            }
            _ => {}
        }
    }

    let gallery_id = gallery_id.ok_or(ApiError::BadRequest)?;
    let src = bytes.ok_or(ApiError::BadRequest)?;
    let gallery_present: bool =
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM galleries WHERE id = $1)")
            .bind(gallery_id)
            .fetch_one(&state.pool)
            .await?;
    if !gallery_present {
        return Err(ApiError::BadRequest);
    }

    let _image_permit = state
        .image_jobs
        .clone()
        .acquire_owned()
        .await
        .map_err(|_| ApiError::ServiceUnavailable)?;
    // Decode + resize + AVIF/JPEG encode is CPU-bound and easily multi-second
    // even in release. Hand it to the blocking pool so the tokio worker stays
    // free for other connections and the per-request handler doesn't stall.
    let src = std::sync::Arc::new(src);
    let src_for_ingest = src.clone();
    let out = tokio::task::spawn_blocking(move || ingest(&src_for_ingest, TARGET_WIDTHS))
        .await
        .map_err(|_| ApiError::Internal)?
        .map_err(|_| ApiError::BadRequest)?;

    let pid = Uuid::new_v4();
    // The digest keeps paths cache-friendly; the owner UUID deliberately keeps
    // deletion lifetimes independent for duplicate uploads.
    let key = format!("{}-{}", content_key(src.as_slice()), pid.simple());
    let mut variants = Vec::with_capacity(out.derivatives.len());
    let mut staged = Vec::with_capacity(out.derivatives.len());
    for d in out.derivatives {
        let e = ext(d.format);
        let rel = format!("media/{e}/{key}_{}.{e}", d.width);
        variants.push(ImageVariant {
            format: d.format,
            width: d.width,
            path: format!("/{rel}"),
        });
        staged.push((rel, d.bytes));
    }

    let media_dir = state.media_dir.clone();
    let mut media = tokio::task::spawn_blocking(move || {
        MediaBatch::stage(
            &media_dir,
            staged
                .iter()
                .map(|(rel, bytes)| (rel.as_str(), bytes.as_slice())),
        )
    })
    .await
    .map_err(|_| ApiError::Internal)?
    .map_err(|_| ApiError::Internal)?;

    let mut tx = state.pool.begin().await?;
    // Serialize position allocation per gallery and reject unknown gallery IDs
    // before publishing any file.
    let gallery_exists: Option<(Uuid,)> =
        sqlx::query_as("SELECT id FROM galleries WHERE id = $1 FOR UPDATE")
            .bind(gallery_id)
            .fetch_optional(&mut *tx)
            .await?;
    if gallery_exists.is_none() {
        return Err(ApiError::BadRequest);
    }
    let (position,): (i32,) =
        sqlx::query_as("SELECT COALESCE(MAX(position), -1) + 1 FROM photos WHERE gallery_id = $1")
            .bind(gallery_id)
            .fetch_one(&mut *tx)
            .await?;

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
    .execute(&mut *tx)
    .await?;

    for variant in &variants {
        let e = ext(variant.format);
        sqlx::query(
            "INSERT INTO photo_variants (photo_id, format, width, path) \
             VALUES ($1,$2,$3,$4)",
        )
        .bind(pid)
        .bind(e)
        .bind(variant.width as i32)
        .bind(&variant.path)
        .execute(&mut *tx)
        .await?;
    }

    media.publish().map_err(|_| ApiError::Internal)?;
    tx.commit().await?;
    media.keep();

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
    let mut tx = state.pool.begin().await?;
    let paths: Vec<(String,)> =
        sqlx::query_as("SELECT path FROM photo_variants WHERE photo_id = $1 FOR UPDATE")
            .bind(id)
            .fetch_all(&mut *tx)
            .await?;
    let done = sqlx::query("DELETE FROM photos WHERE id = $1")
        .bind(id)
        .execute(&mut *tx)
        .await?;
    if done.rows_affected() == 0 {
        return Err(ApiError::NotFound);
    }
    tx.commit().await?;
    // Database truth changes first. A failed best-effort unlink can only leave
    // an orphan; it can never leave a live record pointing at a missing file.
    for (p,) in &paths {
        remove_media_file(&state.media_dir, p);
    }
    Ok(())
}
