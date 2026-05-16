//! Argon2id password hashing + opaque DB-backed sessions.
//! Session cookie is HttpOnly, Secure, SameSite=Strict, scoped to /api/admin.

use crate::error::ApiError;
use crate::state::AppState;
use argon2::password_hash::rand_core::OsRng;
use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use argon2::Argon2;
use axum::extract::State;
use axum::Json;
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use rand::RngCore;
use std::time::{SystemTime, UNIX_EPOCH};
use syle_types::{LoginRequest, User};
use uuid::Uuid;

const SESSION_COOKIE: &str = "sid";
const SESSION_TTL_SECS: i64 = 7 * 24 * 3600;

/// Hash a plaintext password with Argon2id (random per-password salt).
pub fn hash_password(password: &str) -> anyhow::Result<String> {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|h| h.to_string())
        .map_err(|e| anyhow::anyhow!(e))
}

fn verify_password(hash: &str, password: &str) -> bool {
    PasswordHash::new(hash)
        .map(|parsed| {
            Argon2::default()
                .verify_password(password.as_bytes(), &parsed)
                .is_ok()
        })
        .unwrap_or(false)
}

fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64
}

fn new_token() -> String {
    let mut buf = [0u8; 32];
    OsRng.fill_bytes(&mut buf);
    buf.iter().map(|b| format!("{b:02x}")).collect()
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

pub async fn login(
    State(state): State<AppState>,
    jar: CookieJar,
    Json(req): Json<LoginRequest>,
) -> Result<(CookieJar, Json<User>), ApiError> {
    let row: Option<(Uuid, String, String)> =
        sqlx::query_as("SELECT id, email, password_hash FROM users WHERE email = $1")
            .bind(&req.email)
            .fetch_optional(&state.pool)
            .await?;
    let (id, email, hash) = row.ok_or(ApiError::Unauthorized)?;
    if !verify_password(&hash, &req.password) {
        return Err(ApiError::Unauthorized);
    }

    let token = new_token();
    sqlx::query("INSERT INTO sessions (token, user_id, expires_at) VALUES ($1,$2,$3)")
        .bind(&token)
        .bind(id)
        .bind(now() + SESSION_TTL_SECS)
        .execute(&state.pool)
        .await?;

    let cookie = Cookie::build((SESSION_COOKIE, token))
        .path("/api/admin")
        .http_only(true)
        .secure(true)
        .same_site(SameSite::Strict)
        .build();
    Ok((jar.add(cookie), Json(User { id, email })))
}

pub async fn logout(
    State(state): State<AppState>,
    jar: CookieJar,
) -> Result<CookieJar, ApiError> {
    if let Some(c) = jar.get(SESSION_COOKIE) {
        sqlx::query("DELETE FROM sessions WHERE token = $1")
            .bind(c.value())
            .execute(&state.pool)
            .await?;
    }
    Ok(jar.remove(Cookie::from(SESSION_COOKIE)))
}

pub async fn me(
    State(state): State<AppState>,
    jar: CookieJar,
) -> Result<Json<User>, ApiError> {
    Ok(Json(current_user(&jar, &state).await?))
}
