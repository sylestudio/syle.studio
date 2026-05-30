//! Opaque DB-backed sessions. The `sid` cookie is HttpOnly, Secure,
//! SameSite=Strict, scoped to `/api/admin`, 7-day TTL.

use super::audit::{record, AccessEvent, ClientMeta};
use super::password::verify_password;
use crate::error::ApiError;
use crate::state::AppState;
use argon2::password_hash::rand_core::OsRng;
use axum::extract::{FromRequestParts, State};
use axum::http::request::Parts;
use axum::Json;
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use rand::RngCore;
use std::time::{SystemTime, UNIX_EPOCH};
use syle_types::{AccessAction, AccessMethod, AccessOutcome, LoginRequest, User};
use uuid::Uuid;

const SESSION_COOKIE: &str = "sid";
const SESSION_TTL_SECS: i64 = 7 * 24 * 3600;

pub(crate) fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64
}

/// 32 bytes of OS entropy, hex-encoded. Doubles as session token and as the
/// opaque WebAuthn ceremony `flow_id`.
pub(crate) fn new_token() -> String {
    let mut buf = [0u8; 32];
    OsRng.fill_bytes(&mut buf);
    buf.iter().map(|b| format!("{b:02x}")).collect()
}

/// Mint a session for `user_id`: persist the token, build the `sid` cookie.
/// Shared by password login and passkey `login/finish` so both yield a
/// byte-identical cookie the `AuthUser` extractor accepts unchanged.
pub(crate) async fn issue_session(
    state: &AppState,
    user_id: Uuid,
) -> Result<Cookie<'static>, ApiError> {
    let token = new_token();
    sqlx::query("INSERT INTO sessions (token, user_id, expires_at) VALUES ($1,$2,$3)")
        .bind(&token)
        .bind(user_id)
        .bind(now() + SESSION_TTL_SECS)
        .execute(&state.pool)
        .await?;
    Ok(Cookie::build((SESSION_COOKIE, token))
        .path("/api/admin")
        .http_only(true)
        .secure(true)
        .same_site(SameSite::Strict)
        .build())
}

/// Resolve the authenticated user from the session cookie, or `Unauthorized`.
async fn current_user(jar: &CookieJar, state: &AppState) -> Result<User, ApiError> {
    let token = jar
        .get(SESSION_COOKIE)
        .map(|c| c.value().to_string())
        .ok_or(ApiError::Unauthorized)?;
    let row: Option<(Uuid, String)> = sqlx::query_as(
        "SELECT u.id, u.email FROM sessions s \
         JOIN users u ON u.id = s.user_id \
         WHERE s.token = $1 AND s.expires_at > $2",
    )
    .bind(&token)
    .bind(now())
    .fetch_optional(&state.pool)
    .await?;
    row.map(|(id, email)| User { id, email })
        .ok_or(ApiError::Unauthorized)
}

/// Extractor that rejects with 401 unless a valid session cookie is present.
/// Use it on every `/api/admin/*` write handler.
pub struct AuthUser(pub User);

impl FromRequestParts<AppState> for AuthUser {
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let jar = CookieJar::from_headers(&parts.headers);
        Ok(AuthUser(current_user(&jar, state).await?))
    }
}

pub async fn login(
    State(state): State<AppState>,
    meta: ClientMeta,
    jar: CookieJar,
    Json(req): Json<LoginRequest>,
) -> Result<(CookieJar, Json<User>), ApiError> {
    // `password_hash` is nullable since the passkeys migration: a passkey-only
    // user has no password and simply can't authenticate via this route.
    let row: Option<(Uuid, String, Option<String>)> =
        sqlx::query_as("SELECT id, email, password_hash FROM users WHERE email = $1")
            .bind(&req.email)
            .fetch_optional(&state.pool)
            .await?;

    // Resolve the attempt; keep the user id (when the email is known) so even a
    // wrong-password failure ties to that account in the audit row.
    let ok = match &row {
        Some((id, email, Some(hash))) if verify_password(hash, &req.password) => {
            Some((*id, email.clone()))
        }
        _ => None,
    };

    let Some((id, email)) = ok else {
        record(
            &state.pool,
            AccessEvent {
                user_id: row.as_ref().map(|(id, ..)| *id),
                email: Some(req.email),
                action: AccessAction::Login,
                method: Some(AccessMethod::Password),
                outcome: AccessOutcome::Failure,
            },
            &meta,
        )
        .await;
        return Err(ApiError::Unauthorized);
    };

    // Record success before minting the session: a session-insert failure must
    // still leave the access trail.
    record(
        &state.pool,
        AccessEvent {
            user_id: Some(id),
            email: Some(email.clone()),
            action: AccessAction::Login,
            method: Some(AccessMethod::Password),
            outcome: AccessOutcome::Success,
        },
        &meta,
    )
    .await;
    let cookie = issue_session(&state, id).await?;
    Ok((jar.add(cookie), Json(User { id, email })))
}

pub async fn logout(
    State(state): State<AppState>,
    meta: ClientMeta,
    jar: CookieJar,
) -> Result<CookieJar, ApiError> {
    if let Some(c) = jar.get(SESSION_COOKIE) {
        // Resolve the session owner for the audit row before revoking it.
        let owner: Option<(Uuid, String)> = sqlx::query_as(
            "SELECT u.id, u.email FROM sessions s JOIN users u ON u.id = s.user_id \
             WHERE s.token = $1",
        )
        .bind(c.value())
        .fetch_optional(&state.pool)
        .await?;
        sqlx::query("DELETE FROM sessions WHERE token = $1")
            .bind(c.value())
            .execute(&state.pool)
            .await?;
        if let Some((id, email)) = owner {
            record(
                &state.pool,
                AccessEvent {
                    user_id: Some(id),
                    email: Some(email),
                    action: AccessAction::Logout,
                    method: None,
                    outcome: AccessOutcome::Success,
                },
                &meta,
            )
            .await;
        }
    }
    Ok(jar.remove(Cookie::from(SESSION_COOKIE)))
}

pub async fn me(
    State(state): State<AppState>,
    jar: CookieJar,
) -> Result<Json<User>, ApiError> {
    Ok(Json(current_user(&jar, &state).await?))
}
