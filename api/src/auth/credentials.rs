//! Passkey credential management: list and revoke. Revoking is guarded so the
//! operator can't lock themselves out by removing their last auth factor.

use super::audit::{record, AccessEvent, ClientMeta};
use super::AuthUser;
use crate::error::ApiError;
use crate::state::AppState;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use syle_types::{AccessAction, AccessOutcome, CredentialInfo, RenameCredential};
use uuid::Uuid;

pub async fn rename_credential(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(id): Path<Uuid>,
    Json(req): Json<RenameCredential>,
) -> Result<Json<CredentialInfo>, ApiError> {
    let row: Option<(Uuid, String, i64, Option<i64>)> = sqlx::query_as(
        "UPDATE webauthn_credentials SET name = $1 WHERE id = $2 AND user_id = $3 \
         RETURNING id, name, created_at, last_used_at",
    )
    .bind(&req.name)
    .bind(id)
    .bind(user.id)
    .fetch_optional(&state.pool)
    .await?;
    let (id, name, created_at, last_used_at) = row.ok_or(ApiError::NotFound)?;
    Ok(Json(CredentialInfo {
        id,
        name,
        created_at,
        last_used_at,
    }))
}

pub async fn list_credentials(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
) -> Result<Json<Vec<CredentialInfo>>, ApiError> {
    let rows: Vec<(Uuid, String, i64, Option<i64>)> = sqlx::query_as(
        "SELECT id, name, created_at, last_used_at FROM webauthn_credentials \
         WHERE user_id = $1 ORDER BY created_at",
    )
    .bind(user.id)
    .fetch_all(&state.pool)
    .await?;
    Ok(Json(
        rows.into_iter()
            .map(|(id, name, created_at, last_used_at)| CredentialInfo {
                id,
                name,
                created_at,
                last_used_at,
            })
            .collect(),
    ))
}

pub async fn delete_credential(
    State(state): State<AppState>,
    meta: ClientMeta,
    AuthUser(user): AuthUser,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, ApiError> {
    let owns: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM webauthn_credentials WHERE id = $1 AND user_id = $2)",
    )
    .bind(id)
    .bind(user.id)
    .fetch_one(&state.pool)
    .await?;
    if !owns {
        return Err(ApiError::NotFound);
    }

    // Last-factor guard: refuse to remove the only passkey unless another factor
    // remains (a password or an unused recovery code). Password is a permanent
    // fallback in v1, so this normally allows deletion.
    let total: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM webauthn_credentials WHERE user_id = $1")
            .bind(user.id)
            .fetch_one(&state.pool)
            .await?;
    if total <= 1 {
        let has_password: bool =
            sqlx::query_scalar("SELECT password_hash IS NOT NULL FROM users WHERE id = $1")
                .bind(user.id)
                .fetch_one(&state.pool)
                .await?;
        let unused_codes: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM recovery_codes WHERE user_id = $1 AND used_at IS NULL",
        )
        .bind(user.id)
        .fetch_one(&state.pool)
        .await?;
        if !has_password && unused_codes == 0 {
            return Err(ApiError::BadRequest);
        }
    }

    sqlx::query("DELETE FROM webauthn_credentials WHERE id = $1 AND user_id = $2")
        .bind(id)
        .bind(user.id)
        .execute(&state.pool)
        .await?;

    record(
        &state.pool,
        AccessEvent {
            user_id: Some(user.id),
            email: Some(user.email),
            action: AccessAction::PasskeyRevoke,
            method: None,
            outcome: AccessOutcome::Success,
        },
        &meta,
    )
    .await;
    Ok(StatusCode::OK)
}
