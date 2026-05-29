//! Single-use recovery codes. The server stores only Argon2id hashes; the
//! plaintext codes are shown to the operator exactly once, at generation.

use super::password::{hash_password, verify_password};
use super::session::{issue_session, now};
use crate::error::ApiError;
use crate::state::AppState;
use argon2::password_hash::rand_core::OsRng;
use axum::extract::State;
use axum::Json;
use axum_extra::extract::cookie::CookieJar;
use rand::RngCore;
use syle_types::{RecoveryCodes, RecoveryRedeem, User};
use uuid::Uuid;

/// Crockford base32 (no I/L/O/U). 256 is a multiple of 32, so `byte % 32` is
/// unbiased.
const ALPHABET: &[u8] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";
const CODE_LEN: usize = 10; // 10 symbols * 5 bits = 50 bits of entropy each.
const CODE_COUNT: usize = 10;

/// One displayable code, e.g. `K3M9Q-7XR2T`.
fn generate_code() -> String {
    let mut raw = [0u8; CODE_LEN];
    OsRng.fill_bytes(&mut raw);
    let s: String = raw.iter().map(|b| ALPHABET[(*b % 32) as usize] as char).collect();
    format!("{}-{}", &s[..5], &s[5..])
}

/// Canonical form used for hashing and matching: alphanumerics only, uppercase.
/// Tolerates the display dash and any user-entered casing or spacing.
fn canonical(code: &str) -> String {
    code.chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .collect::<String>()
        .to_ascii_uppercase()
}

pub async fn recovery_generate(
    State(state): State<AppState>,
    super::AuthUser(user): super::AuthUser,
) -> Result<Json<RecoveryCodes>, ApiError> {
    let codes: Vec<String> = (0..CODE_COUNT).map(|_| generate_code()).collect();
    // Regeneration replaces the prior set entirely.
    sqlx::query("DELETE FROM recovery_codes WHERE user_id = $1")
        .bind(user.id)
        .execute(&state.pool)
        .await?;
    let created = now();
    for code in &codes {
        let hash = hash_password(&canonical(code)).map_err(|_| ApiError::Internal)?;
        sqlx::query(
            "INSERT INTO recovery_codes (id, user_id, code_hash, created_at) \
             VALUES ($1, $2, $3, $4)",
        )
        .bind(Uuid::new_v4())
        .bind(user.id)
        .bind(hash)
        .bind(created)
        .execute(&state.pool)
        .await?;
    }
    Ok(Json(RecoveryCodes { codes }))
}

pub async fn recovery_redeem(
    State(state): State<AppState>,
    jar: CookieJar,
    Json(req): Json<RecoveryRedeem>,
) -> Result<(CookieJar, Json<User>), ApiError> {
    // Cheap format gate BEFORE any Argon2 work: malformed input costs ~nothing,
    // bounding the CPU an unauthenticated caller can burn. A volumetric limiter
    // (per IP+email) belongs at the edge — see deploy notes.
    let canon = canonical(&req.code);
    if canon.len() != CODE_LEN || !canon.bytes().all(|b| ALPHABET.contains(&b)) {
        return Err(ApiError::Unauthorized);
    }

    let account: Option<(Uuid, String)> =
        sqlx::query_as("SELECT id, email FROM users WHERE email = $1")
            .bind(&req.email)
            .fetch_optional(&state.pool)
            .await?;
    let (user_id, email) = account.ok_or(ApiError::Unauthorized)?;

    let rows: Vec<(Uuid, String)> = sqlx::query_as(
        "SELECT id, code_hash FROM recovery_codes WHERE user_id = $1 AND used_at IS NULL",
    )
    .bind(user_id)
    .fetch_all(&state.pool)
    .await?;
    let code_id = rows
        .iter()
        .find(|(_, hash)| verify_password(hash, &canon))
        .map(|(id, _)| *id)
        .ok_or(ApiError::Unauthorized)?;

    // Single-use: the `used_at IS NULL` predicate also closes a double-redeem race.
    let res =
        sqlx::query("UPDATE recovery_codes SET used_at = $1 WHERE id = $2 AND used_at IS NULL")
            .bind(now())
            .bind(code_id)
            .execute(&state.pool)
            .await?;
    if res.rows_affected() == 0 {
        return Err(ApiError::Unauthorized);
    }

    let cookie = issue_session(&state, user_id).await?;
    Ok((jar.add(cookie), Json(User { id: user_id, email })))
}
