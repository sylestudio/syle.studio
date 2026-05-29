use crate::auth::AuthUser;
use crate::error::ApiError;
use crate::state::AppState;
use axum::extract::{Path, State};
use axum::Json;
use std::time::{SystemTime, UNIX_EPOCH};
use syle_types::{is_valid_slug, Block, BlogPost, NewPost, PostStatus, UpdatePost};
use uuid::Uuid;

/// Row shape: the body is stored as JSON text in `body_blocks`.
type PostRow = (Uuid, String, String, String, String, Option<i64>);

/// Columns selected for a post, in `PostRow` order.
const POST_COLS: &str = "id, slug, title, body_blocks, status, published_at";

fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64
}

fn status_of(s: &str) -> PostStatus {
    if s == "published" {
        PostStatus::Published
    } else {
        PostStatus::Draft
    }
}

fn blocks_json(blocks: &[Block]) -> String {
    serde_json::to_string(blocks).unwrap_or_else(|_| "[]".into())
}

fn into_post((id, slug, title, body_blocks, status, published_at): PostRow) -> BlogPost {
    let blocks: Vec<Block> = serde_json::from_str(&body_blocks).unwrap_or_default();
    let body_html = syle_render::render_blocks(&blocks);
    BlogPost {
        id,
        slug,
        title,
        blocks,
        body_html,
        status: status_of(&status),
        published_at,
    }
}

/// All posts including drafts (CRM list).
pub async fn list_posts(
    _user: AuthUser,
    State(state): State<AppState>,
) -> Result<Json<Vec<BlogPost>>, ApiError> {
    let rows: Vec<PostRow> = sqlx::query_as(&format!(
        "SELECT {POST_COLS} FROM blog_posts \
         ORDER BY COALESCE(published_at, 0) DESC, slug"
    ))
    .fetch_all(&state.pool)
    .await?;
    Ok(Json(rows.into_iter().map(into_post).collect()))
}

pub async fn create_post(
    _user: AuthUser,
    State(state): State<AppState>,
    Json(req): Json<NewPost>,
) -> Result<Json<BlogPost>, ApiError> {
    if !is_valid_slug(&req.slug) {
        return Err(ApiError::BadRequest);
    }
    let (status_str, published_at) = match req.status {
        PostStatus::Published => ("published", Some(now())),
        PostStatus::Draft => ("draft", None),
    };
    let id = Uuid::new_v4();
    let body_blocks = blocks_json(&req.blocks);
    sqlx::query(
        "INSERT INTO blog_posts \
         (id, slug, title, body_blocks, status, published_at) \
         VALUES ($1,$2,$3,$4,$5,$6)",
    )
    .bind(id)
    .bind(&req.slug)
    .bind(&req.title)
    .bind(&body_blocks)
    .bind(status_str)
    .bind(published_at)
    .execute(&state.pool)
    .await
    .map_err(|e| match e {
        sqlx::Error::Database(db) if db.is_unique_violation() => ApiError::BadRequest,
        other => other.into(),
    })?;

    let body_html = syle_render::render_blocks(&req.blocks);
    Ok(Json(BlogPost {
        id,
        slug: req.slug,
        title: req.title,
        blocks: req.blocks,
        body_html,
        status: req.status,
        published_at,
    }))
}

pub async fn get_post(
    _user: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<BlogPost>, ApiError> {
    let row: Option<PostRow> = sqlx::query_as(&format!(
        "SELECT {POST_COLS} FROM blog_posts WHERE id = $1"
    ))
    .bind(id)
    .fetch_optional(&state.pool)
    .await?;
    Ok(Json(into_post(row.ok_or(ApiError::NotFound)?)))
}

pub async fn update_post(
    _user: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdatePost>,
) -> Result<Json<BlogPost>, ApiError> {
    if let Some(slug) = &req.slug {
        if !is_valid_slug(slug) {
            return Err(ApiError::BadRequest);
        }
    }
    let current: Option<PostRow> = sqlx::query_as(&format!(
        "SELECT {POST_COLS} FROM blog_posts WHERE id = $1"
    ))
    .bind(id)
    .fetch_optional(&state.pool)
    .await?;
    let current = into_post(current.ok_or(ApiError::NotFound)?);

    let status = req.status.unwrap_or(current.status);
    let (status_str, published_at) = match status {
        // First publish stamps the date; re-publish keeps the original.
        PostStatus::Published => (
            "published",
            current.published_at.or_else(|| Some(now())),
        ),
        PostStatus::Draft => ("draft", current.published_at),
    };
    let body_blocks: Option<String> = req.blocks.as_deref().map(blocks_json);

    let row: PostRow = sqlx::query_as(&format!(
        "UPDATE blog_posts SET \
           slug = COALESCE($2, slug), \
           title = COALESCE($3, title), \
           body_blocks = COALESCE($4, body_blocks), \
           status = $5, \
           published_at = $6 \
         WHERE id = $1 \
         RETURNING {POST_COLS}"
    ))
    .bind(id)
    .bind(req.slug)
    .bind(req.title)
    .bind(body_blocks)
    .bind(status_str)
    .bind(published_at)
    .fetch_one(&state.pool)
    .await
    .map_err(|e| match e {
        sqlx::Error::Database(db) if db.is_unique_violation() => ApiError::BadRequest,
        other => other.into(),
    })?;
    Ok(Json(into_post(row)))
}

pub async fn delete_post(
    _user: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<(), ApiError> {
    let done = sqlx::query("DELETE FROM blog_posts WHERE id = $1")
        .bind(id)
        .execute(&state.pool)
        .await?;
    if done.rows_affected() == 0 {
        return Err(ApiError::NotFound);
    }
    Ok(())
}
