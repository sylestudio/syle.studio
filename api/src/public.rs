//! Public read endpoints. Cacheable, unauthenticated; consumed by the Astro
//! site at build and behind the CDN.

use crate::error::ApiError;
use crate::state::AppState;
use axum::extract::State;
use axum::Json;
use syle_types::Gallery;
use uuid::Uuid;

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
