//! Authenticated CRM write endpoints. Every handler takes `AuthUser`, so the
//! session guard is enforced by the type system, not by remembering to check.

use crate::auth::AuthUser;
use crate::error::ApiError;
use crate::state::AppState;
use axum::extract::State;
use axum::Json;
use syle_types::{Gallery, NewGallery};
use uuid::Uuid;

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
