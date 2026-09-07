use super::{ext, MediaBatch};
use crate::auth::AuthUser;
use crate::error::ApiError;
use crate::state::AppState;
use axum::extract::{Multipart, State};
use axum::Json;
use std::sync::Arc;
use syle_core::ingest::{content_key, ingest, thumbhash_data_url};
use syle_types::{ImageFormat, ImageVariant, UploadedImage};

/// The same responsive widths as gallery photos; the pipeline drops any that
/// would upscale the source.
const TARGET_WIDTHS: &[u32] = &[480, 960, 1440, 2400];

/// Multipart `file` upload for an inline post image or project cover. Runs the
/// exact same pipeline as gallery photos — responsive AVIF+JPEG renditions plus a
/// ThumbHash blur-up — and writes every derivative under `media_dir`. Unlike
/// `upload_photo` it persists no DB row: the caller copies the returned asset
/// into its block document or standalone-project row.
pub async fn upload_blog_asset(
    _user: AuthUser,
    State(state): State<AppState>,
    mut mp: Multipart,
) -> Result<Json<UploadedImage>, ApiError> {
    let mut bytes: Option<Vec<u8>> = None;
    while let Some(mut field) = mp.next_field().await.map_err(|_| ApiError::BadRequest)? {
        if field.name() == Some("file") {
            // Stream chunks into one buffer; the per-field cap backs up the
            // route's DefaultBodyLimit.
            let mut buf: Vec<u8> = Vec::new();
            while let Some(chunk) = field.chunk().await.map_err(|_| ApiError::BadRequest)? {
                if buf.len() + chunk.len() > crate::MAX_UPLOAD_BYTES {
                    return Err(ApiError::BadRequest);
                }
                buf.extend_from_slice(&chunk);
            }
            bytes = Some(buf);
        }
    }

    let src = Arc::new(bytes.ok_or(ApiError::BadRequest)?);
    let _image_permit = state
        .image_jobs
        .clone()
        .acquire_owned()
        .await
        .map_err(|_| ApiError::ServiceUnavailable)?;
    let src_for_ingest = src.clone();
    // CPU-bound decode/resize/encode → blocking pool, like the photo route.
    let out = tokio::task::spawn_blocking(move || ingest(&src_for_ingest, TARGET_WIDTHS))
        .await
        .map_err(|_| ApiError::Internal)?
        .map_err(|_| ApiError::BadRequest)?;

    let asset_id = uuid::Uuid::new_v4();
    let key = format!("{}-{}", content_key(src.as_slice()), asset_id.simple());

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
    media.publish().map_err(|_| ApiError::Internal)?;
    media.keep();

    // The widest JPEG is the broadly-compatible `<img>` fallback / preview src.
    let src_path = variants
        .iter()
        .filter(|v| matches!(v.format, ImageFormat::Jpeg))
        .max_by_key(|v| v.width)
        .map(|v| v.path.clone())
        .ok_or(ApiError::Internal)?;

    // A failed placeholder must not fail the upload — the renderer simply skips
    // the blur-up when it's absent.
    let placeholder = thumbhash_data_url(&out.thumbhash).unwrap_or_default();

    Ok(Json(UploadedImage {
        src: src_path,
        variants,
        placeholder,
        width: out.width,
        height: out.height,
    }))
}
