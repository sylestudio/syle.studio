use super::remove_media_file;
use crate::auth::AuthUser;
use crate::error::ApiError;
use crate::project_row::{into_project, ProjectRow, PROJECT_COLS};
use crate::state::AppState;
use axum::extract::{Path, State};
use axum::Json;
use sqlx::types::Json as SqlJson;
use std::collections::HashSet;
use syle_types::{is_valid_project_url, NewProject, Project, UpdateProject, UploadedImage};
use uuid::Uuid;

/// Every standalone project, including drafts (CRM list).
pub async fn list_projects(
    _user: AuthUser,
    State(state): State<AppState>,
) -> Result<Json<Vec<Project>>, ApiError> {
    let rows: Vec<ProjectRow> = sqlx::query_as(&format!(
        "SELECT {PROJECT_COLS} FROM projects ORDER BY position, title, id"
    ))
    .fetch_all(&state.pool)
    .await?;
    Ok(Json(rows.into_iter().map(into_project).collect()))
}

pub async fn create_project(
    _user: AuthUser,
    State(state): State<AppState>,
    Json(req): Json<NewProject>,
) -> Result<Json<Project>, ApiError> {
    let title = req.title.trim().to_string();
    let url = req.url.trim().to_string();
    let category = req.category.trim().to_string();
    if title.is_empty() || !is_valid_project_url(&url) {
        return Err(ApiError::BadRequest);
    }

    let row: ProjectRow = sqlx::query_as(&format!(
        "INSERT INTO projects (id, title, url, category, position, published, cover) \
         VALUES ($1,$2,$3,$4,$5,$6,$7) RETURNING {PROJECT_COLS}"
    ))
    .bind(Uuid::new_v4())
    .bind(title)
    .bind(url)
    .bind(category)
    .bind(req.position)
    .bind(req.published)
    .bind(req.cover.map(SqlJson))
    .fetch_one(&state.pool)
    .await?;

    Ok(Json(into_project(row)))
}

pub async fn get_project(
    _user: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<Project>, ApiError> {
    let row: Option<ProjectRow> = sqlx::query_as(&format!(
        "SELECT {PROJECT_COLS} FROM projects WHERE id = $1"
    ))
    .bind(id)
    .fetch_optional(&state.pool)
    .await?;
    Ok(Json(into_project(row.ok_or(ApiError::NotFound)?)))
}

pub async fn update_project(
    _user: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateProject>,
) -> Result<Json<Project>, ApiError> {
    if req.clear_cover && req.cover.is_some() {
        return Err(ApiError::BadRequest);
    }

    let title = req.title.map(|value| value.trim().to_string());
    let url = req.url.map(|value| value.trim().to_string());
    let category = req.category.map(|value| value.trim().to_string());
    if title.as_ref().is_some_and(|value| value.is_empty())
        || url
            .as_ref()
            .is_some_and(|value| !is_valid_project_url(value))
    {
        return Err(ApiError::BadRequest);
    }

    let replacing_cover = req.clear_cover || req.cover.is_some();
    let mut tx = state.pool.begin().await?;
    let old_cover = if replacing_cover {
        let row: Option<(Option<SqlJson<UploadedImage>>,)> =
            sqlx::query_as("SELECT cover FROM projects WHERE id = $1 FOR UPDATE")
                .bind(id)
                .fetch_optional(&mut *tx)
                .await?;
        row.ok_or(ApiError::NotFound)?.0.map(|SqlJson(image)| image)
    } else {
        None
    };

    let row: Option<ProjectRow> = sqlx::query_as(&format!(
        "UPDATE projects SET \
           title = COALESCE($2, title), \
           url = COALESCE($3, url), \
           category = COALESCE($4, category), \
           position = COALESCE($5, position), \
           published = COALESCE($6, published), \
           cover = CASE WHEN $8 THEN NULL ELSE COALESCE($7, cover) END \
         WHERE id = $1 RETURNING {PROJECT_COLS}"
    ))
    .bind(id)
    .bind(title)
    .bind(url)
    .bind(category)
    .bind(req.position)
    .bind(req.published)
    .bind(req.cover.map(SqlJson))
    .bind(req.clear_cover)
    .fetch_optional(&mut *tx)
    .await?;
    let project = into_project(row.ok_or(ApiError::NotFound)?);
    tx.commit().await?;

    if replacing_cover && old_cover.as_ref() != project.cover.as_ref() {
        if let Some(old) = &old_cover {
            remove_cover_files(&state, old);
        }
    }
    Ok(Json(project))
}

pub async fn delete_project(
    _user: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<(), ApiError> {
    let mut tx = state.pool.begin().await?;
    let row: Option<(Option<SqlJson<UploadedImage>>,)> =
        sqlx::query_as("SELECT cover FROM projects WHERE id = $1 FOR UPDATE")
            .bind(id)
            .fetch_optional(&mut *tx)
            .await?;
    let old_cover = row.ok_or(ApiError::NotFound)?.0.map(|SqlJson(image)| image);
    sqlx::query("DELETE FROM projects WHERE id = $1")
        .bind(id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;

    if let Some(old) = &old_cover {
        remove_cover_files(&state, old);
    }
    Ok(())
}

fn remove_cover_files(state: &AppState, cover: &UploadedImage) {
    let mut paths = HashSet::new();
    paths.insert(cover.src.as_str());
    for variant in &cover.variants {
        paths.insert(variant.path.as_str());
    }
    for path in paths {
        remove_media_file(&state.media_dir, path);
    }
}
