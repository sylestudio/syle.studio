use super::ext;
use crate::auth::AuthUser;
use crate::error::ApiError;
use crate::state::AppState;
use axum::extract::{Multipart, State};
use axum::Json;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::Arc;
use syle_core::ingest::{ingest, thumbhash_data_url};
use syle_types::{ImageFormat, ImageVariant, UploadedImage};

/// The same responsive widths as gallery photos; the pipeline drops any that
/// would upscale the source.
const TARGET_WIDTHS: &[u32] = &[480, 960, 1440, 2400];

/// Multipart `file` upload for an inline post image. Runs the exact same image
/// pipeline as gallery photos — responsive AVIF+JPEG renditions plus a
/// ThumbHash blur-up — and writes every derivative under `media_dir`. Unlike
/// `upload_photo` it persists no DB row: a blog image is referenced only by the
/// block document, so the returned renditions are copied onto the image block,
/// which becomes the sole record of the asset.
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
    let src_for_ingest = src.clone();
    // CPU-bound decode/resize/encode → blocking pool, like the photo route.
    let out = tokio::task::spawn_blocking(move || ingest(&src_for_ingest, TARGET_WIDTHS))
        .await
        .map_err(|_| ApiError::Internal)?
        .map_err(|_| ApiError::BadRequest)?;

    let mut hasher = DefaultHasher::new();
    src.as_slice().hash(&mut hasher);
    let key = format!("{:016x}", hasher.finish());

    // Write every responsive derivative (AVIF + JPEG), exactly like a gallery
    // photo — only the DB INSERTs are omitted.
    let mut variants = Vec::with_capacity(out.derivatives.len());
    for d in &out.derivatives {
        let e = ext(d.format);
        let rel = format!("media/{e}/{key}_{}.{e}", d.width);
        let abs = state.media_dir.join(&rel);
        if let Some(parent) = abs.parent() {
            std::fs::create_dir_all(parent).map_err(|_| ApiError::Internal)?;
        }
        std::fs::write(&abs, &d.bytes).map_err(|_| ApiError::Internal)?;
        variants.push(ImageVariant {
            format: d.format,
            width: d.width,
            path: format!("/{rel}"),
        });
    }

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
