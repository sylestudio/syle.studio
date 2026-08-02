use super::{parse_format, remove_media_file};
use crate::auth::AuthUser;
use crate::error::ApiError;
use crate::gallery_row::{into_gallery, GalleryRow, GALLERY_COLS};
use crate::state::AppState;
use axum::extract::{Path, State};
use axum::Json;
use syle_types::{
    is_valid_slug, Gallery, GalleryDetail, ImageVariant, NewGallery, Photo, Reorder, UpdateGallery,
};
use uuid::Uuid;

/// All galleries including unpublished (CRM list).
pub async fn list_galleries(
    _user: AuthUser,
    State(state): State<AppState>,
) -> Result<Json<Vec<Gallery>>, ApiError> {
    let rows: Vec<GalleryRow> = sqlx::query_as(&format!(
        "SELECT {GALLERY_COLS} FROM galleries ORDER BY position, slug"
    ))
    .fetch_all(&state.pool)
    .await?;
    Ok(Json(rows.into_iter().map(into_gallery).collect()))
}

pub async fn create_gallery(
    _user: AuthUser,
    State(state): State<AppState>,
    Json(req): Json<NewGallery>,
) -> Result<Json<Gallery>, ApiError> {
    if !is_valid_slug(&req.slug) {
        return Err(ApiError::BadRequest);
    }
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
    .map_err(unique_to_bad_request)?;

    Ok(Json(Gallery {
        id,
        slug: req.slug,
        title: req.title,
        position: req.position,
        published: req.published,
        description: String::new(),
        notes: String::new(),
        category: String::new(),
        year: None,
    }))
}

/// One gallery (published or not) with its ordered photos and variants.
pub async fn get_gallery_detail(
    _user: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<GalleryDetail>, ApiError> {
    let row: Option<GalleryRow> = sqlx::query_as(&format!(
        "SELECT {GALLERY_COLS} FROM galleries WHERE id = $1"
    ))
    .bind(id)
    .fetch_optional(&state.pool)
    .await?;
    let gallery = into_gallery(row.ok_or(ApiError::NotFound)?);

    let photo_rows: Vec<(Uuid, Uuid, String, String, i32, i32, i32)> = sqlx::query_as(
        "SELECT id, gallery_id, alt, thumbhash, width, height, position \
         FROM photos WHERE gallery_id = $1 ORDER BY position, id",
    )
    .bind(id)
    .fetch_all(&state.pool)
    .await?;

    let mut photos = Vec::with_capacity(photo_rows.len());
    for (pid, gallery_id, alt, thumbhash, width, height, position) in photo_rows {
        let vars: Vec<(String, i32, String)> = sqlx::query_as(
            "SELECT format, width, path FROM photo_variants \
             WHERE photo_id = $1 ORDER BY format, width",
        )
        .bind(pid)
        .fetch_all(&state.pool)
        .await?;
        photos.push(Photo {
            id: pid,
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
        });
    }
    Ok(Json(GalleryDetail { gallery, photos }))
}

pub async fn update_gallery(
    _user: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateGallery>,
) -> Result<Json<Gallery>, ApiError> {
    if let Some(slug) = &req.slug {
        if !is_valid_slug(slug) {
            return Err(ApiError::BadRequest);
        }
    }
    let row: Option<GalleryRow> = sqlx::query_as(&format!(
        "UPDATE galleries SET \
           title = COALESCE($2, title), \
           slug = COALESCE($3, slug), \
           published = COALESCE($4, published), \
           position = COALESCE($5, position), \
           description = COALESCE($6, description), \
           notes = COALESCE($7, notes), \
           category = COALESCE($8, category), \
           year = COALESCE($9, year) \
         WHERE id = $1 \
         RETURNING {GALLERY_COLS}"
    ))
    .bind(id)
    .bind(req.title)
    .bind(req.slug)
    .bind(req.published)
    .bind(req.position)
    .bind(req.description)
    .bind(req.notes)
    .bind(req.category)
    .bind(req.year)
    .fetch_optional(&state.pool)
    .await
    .map_err(unique_to_bad_request)?;
    Ok(Json(into_gallery(row.ok_or(ApiError::NotFound)?)))
}

pub async fn delete_gallery(
    _user: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<(), ApiError> {
    let mut tx = state.pool.begin().await?;
    let paths: Vec<(String,)> = sqlx::query_as(
        "SELECT v.path FROM photo_variants v \
         JOIN photos p ON p.id = v.photo_id WHERE p.gallery_id = $1 \
         FOR UPDATE OF v",
    )
    .bind(id)
    .fetch_all(&mut *tx)
    .await?;
    // FK ON DELETE CASCADE removes photos + variants.
    let done = sqlx::query("DELETE FROM galleries WHERE id = $1")
        .bind(id)
        .execute(&mut *tx)
        .await?;
    if done.rows_affected() == 0 {
        return Err(ApiError::NotFound);
    }
    tx.commit().await?;
    for (p,) in &paths {
        remove_media_file(&state.media_dir, p);
    }
    Ok(())
}

/// Set each photo's `position` to its index in `ids`, scoped to the gallery.
pub async fn reorder_photos(
    _user: AuthUser,
    State(state): State<AppState>,
    Path(gallery_id): Path<Uuid>,
    Json(req): Json<Reorder>,
) -> Result<(), ApiError> {
    let mut tx = state.pool.begin().await?;
    for (i, pid) in req.ids.iter().enumerate() {
        sqlx::query("UPDATE photos SET position = $1 WHERE id = $2 AND gallery_id = $3")
            .bind(i as i32)
            .bind(pid)
            .bind(gallery_id)
            .execute(&mut *tx)
            .await?;
    }
    tx.commit().await?;
    Ok(())
}

fn unique_to_bad_request(e: sqlx::Error) -> ApiError {
    match e {
        sqlx::Error::Database(db) if db.is_unique_violation() => ApiError::BadRequest,
        other => other.into(),
    }
}
