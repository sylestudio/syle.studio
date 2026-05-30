//! WebAuthn relying-party construction + passkey ceremonies.
//!
//! Ceremony state lives in the `webauthn_flows` table (JSONB) for the ~5 min
//! between */start and */finish; */finish consumes it atomically (single-use).
//! Registered public keys live in `webauthn_credentials` as serde JSONB, with
//! the credential id mirrored into a BYTEA column for fast lookup.

use super::audit::{record, AccessEvent, ClientMeta};
use super::session::{issue_session, new_token, now};
use crate::error::ApiError;
use crate::state::AppState;
use axum::extract::State;
use axum::Json;
use axum_extra::extract::cookie::CookieJar;
use sqlx::PgPool;
use syle_types::{
    AccessAction, AccessMethod, AccessOutcome, CredentialInfo, FlowChallenge, User, WebauthnFinish,
    WebauthnStart,
};
use uuid::Uuid;
use webauthn_rs::prelude::*;

/// A passkey `login/finish` is an access (`Login`) event; record both outcomes.
async fn record_passkey_login(
    state: &AppState,
    user_id: Uuid,
    email: &str,
    outcome: AccessOutcome,
    meta: &ClientMeta,
) {
    record(
        &state.pool,
        AccessEvent {
            user_id: Some(user_id),
            email: Some(email.to_string()),
            action: AccessAction::Login,
            method: Some(AccessMethod::Passkey),
            outcome,
        },
        meta,
    )
    .await;
}

/// Ceremony challenges expire fast: they are single-use and a passkey tap takes
/// seconds, not minutes.
const FLOW_TTL_SECS: i64 = 300;

/// Build the relying party from env-driven config.
///
/// `allow_subdomains(true)` lets the prod admin origin
/// (`https://admin.syle.studio`) authenticate under the apex rp_id
/// (`syle.studio`); locally rp_id `localhost` pairs with `http://localhost:8080`.
pub fn build_webauthn(rp_id: &str, rp_origin: &str) -> anyhow::Result<Webauthn> {
    let origin = Url::parse(rp_origin)?;
    let webauthn = WebauthnBuilder::new(rp_id, &origin)?
        .rp_name("syle.studio CRM")
        .allow_subdomains(true)
        .build()?;
    Ok(webauthn)
}

/// Park ceremony state under a fresh `flow_id`, opportunistically reaping any
/// expired rows. Returns the id to hand back to the browser.
async fn store_flow<T: serde::Serialize>(
    pool: &PgPool,
    user_id: Uuid,
    kind: &str,
    state: &T,
) -> Result<String, ApiError> {
    sqlx::query("DELETE FROM webauthn_flows WHERE expires_at < $1")
        .bind(now())
        .execute(pool)
        .await?;
    let flow_id = new_token();
    sqlx::query(
        "INSERT INTO webauthn_flows (flow_id, user_id, kind, state, expires_at) \
         VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(&flow_id)
    .bind(user_id)
    .bind(kind)
    .bind(sqlx::types::Json(state))
    .bind(now() + FLOW_TTL_SECS)
    .execute(pool)
    .await?;
    Ok(flow_id)
}

/// Consume a ceremony flow atomically (single-use). Rejects a missing, wrong-
/// kind, or expired flow as `Unauthorized`. Returns the owning user + state.
async fn consume_flow<T: serde::de::DeserializeOwned + Send + Unpin + 'static>(
    pool: &PgPool,
    flow_id: &str,
    kind: &str,
) -> Result<(Uuid, T), ApiError> {
    let row: Option<(Uuid, sqlx::types::Json<T>, i64)> = sqlx::query_as(
        "DELETE FROM webauthn_flows WHERE flow_id = $1 AND kind = $2 \
         RETURNING user_id, state, expires_at",
    )
    .bind(flow_id)
    .bind(kind)
    .fetch_optional(pool)
    .await?;
    let (user_id, state, expires_at) = row.ok_or(ApiError::Unauthorized)?;
    if expires_at < now() {
        return Err(ApiError::Unauthorized);
    }
    Ok((user_id, state.0))
}

/// Load a user's registered passkeys (deserialized from JSONB).
async fn load_passkeys(pool: &PgPool, user_id: Uuid) -> Result<Vec<Passkey>, ApiError> {
    let rows: Vec<(sqlx::types::Json<Passkey>,)> =
        sqlx::query_as("SELECT passkey FROM webauthn_credentials WHERE user_id = $1")
            .bind(user_id)
            .fetch_all(pool)
            .await?;
    Ok(rows.into_iter().map(|(p,)| p.0).collect())
}

/// Stamp `last_used_at` on the credential that just authenticated and, when the
/// sign counter advanced, persist the updated passkey. A monotonic counter is
/// what lets a future authentication flag a cloned authenticator.
async fn persist_auth(pool: &PgPool, result: &AuthenticationResult) -> Result<(), ApiError> {
    let cred_id = result.cred_id().as_ref();
    if result.needs_update() {
        let row: Option<(sqlx::types::Json<Passkey>,)> =
            sqlx::query_as("SELECT passkey FROM webauthn_credentials WHERE credential_id = $1")
                .bind(cred_id)
                .fetch_optional(pool)
                .await?;
        if let Some((mut pk,)) = row {
            pk.0.update_credential(result);
            sqlx::query(
                "UPDATE webauthn_credentials SET passkey = $1, last_used_at = $2 \
                 WHERE credential_id = $3",
            )
            .bind(sqlx::types::Json(&pk.0))
            .bind(now())
            .bind(cred_id)
            .execute(pool)
            .await?;
            return Ok(());
        }
    }
    sqlx::query("UPDATE webauthn_credentials SET last_used_at = $1 WHERE credential_id = $2")
        .bind(now())
        .bind(cred_id)
        .execute(pool)
        .await?;
    Ok(())
}

// === registration (authenticated: enroll a passkey on the current account) ===

pub async fn register_start(
    State(state): State<AppState>,
    super::AuthUser(user): super::AuthUser,
) -> Result<Json<FlowChallenge>, ApiError> {
    // Don't let the operator register the same authenticator twice.
    let existing: Vec<Vec<u8>> =
        sqlx::query_scalar("SELECT credential_id FROM webauthn_credentials WHERE user_id = $1")
            .bind(user.id)
            .fetch_all(&state.pool)
            .await?;
    let exclude = if existing.is_empty() {
        None
    } else {
        Some(existing.into_iter().map(CredentialID::from).collect())
    };

    let (ccr, reg_state) = state
        .webauthn
        .start_passkey_registration(user.id, &user.email, &user.email, exclude)
        .map_err(|_| ApiError::Internal)?;

    let flow_id = store_flow(&state.pool, user.id, "register", &reg_state).await?;
    Ok(Json(FlowChallenge {
        flow_id,
        options: serde_json::to_value(&ccr)?,
    }))
}

pub async fn register_finish(
    State(state): State<AppState>,
    meta: ClientMeta,
    super::AuthUser(user): super::AuthUser,
    Json(req): Json<WebauthnFinish>,
) -> Result<Json<CredentialInfo>, ApiError> {
    let (flow_user, reg_state) =
        consume_flow::<PasskeyRegistration>(&state.pool, &req.flow_id, "register").await?;
    if flow_user != user.id {
        return Err(ApiError::Unauthorized);
    }
    let rpkc: RegisterPublicKeyCredential =
        serde_json::from_value(req.credential).map_err(|_| ApiError::BadRequest)?;
    let passkey = state
        .webauthn
        .finish_passkey_registration(&rpkc, &reg_state)
        .map_err(|_| ApiError::BadRequest)?;

    let id = Uuid::new_v4();
    let created = now();
    sqlx::query(
        "INSERT INTO webauthn_credentials \
         (id, user_id, credential_id, passkey, name, created_at) \
         VALUES ($1, $2, $3, $4, '', $5)",
    )
    .bind(id)
    .bind(user.id)
    .bind(passkey.cred_id().as_ref())
    .bind(sqlx::types::Json(&passkey))
    .bind(created)
    .execute(&state.pool)
    .await?;

    record(
        &state.pool,
        AccessEvent {
            user_id: Some(user.id),
            email: Some(user.email),
            action: AccessAction::PasskeyEnroll,
            method: None,
            outcome: AccessOutcome::Success,
        },
        &meta,
    )
    .await;

    Ok(Json(CredentialInfo {
        id,
        name: String::new(),
        created_at: created,
        last_used_at: None,
    }))
}

// === login (pre-auth: email-first, then a passkey tap) ===

pub async fn login_start(
    State(state): State<AppState>,
    Json(req): Json<WebauthnStart>,
) -> Result<Json<FlowChallenge>, ApiError> {
    // Resolve the account, then its passkeys. A missing account or an account
    // with no enrolled passkey both yield `Unauthorized` — never a challenge
    // we can't complete, and minimal enumeration signal (accounts are
    // provisioned, not self-serve).
    let user_id: Option<Uuid> = sqlx::query_scalar("SELECT id FROM users WHERE email = $1")
        .bind(&req.email)
        .fetch_optional(&state.pool)
        .await?;
    let user_id = user_id.ok_or(ApiError::Unauthorized)?;
    let passkeys = load_passkeys(&state.pool, user_id).await?;
    if passkeys.is_empty() {
        return Err(ApiError::Unauthorized);
    }

    let (rcr, auth_state) = state
        .webauthn
        .start_passkey_authentication(&passkeys)
        .map_err(|_| ApiError::Internal)?;

    let flow_id = store_flow(&state.pool, user_id, "auth", &auth_state).await?;
    Ok(Json(FlowChallenge {
        flow_id,
        options: serde_json::to_value(&rcr)?,
    }))
}

pub async fn login_finish(
    State(state): State<AppState>,
    meta: ClientMeta,
    jar: CookieJar,
    Json(req): Json<WebauthnFinish>,
) -> Result<(CookieJar, Json<User>), ApiError> {
    let (user_id, auth_state) =
        consume_flow::<PasskeyAuthentication>(&state.pool, &req.flow_id, "auth").await?;
    let email: String = sqlx::query_scalar("SELECT email FROM users WHERE id = $1")
        .bind(user_id)
        .fetch_one(&state.pool)
        .await?;

    // A consumed flow that then fails verification (malformed credential or a
    // bad signature) is a failed access attempt — recorded. Infra errors below
    // still surface as 500 and aren't logged as auth failures.
    let pkc: PublicKeyCredential = match serde_json::from_value(req.credential) {
        Ok(p) => p,
        Err(_) => {
            record_passkey_login(&state, user_id, &email, AccessOutcome::Failure, &meta).await;
            return Err(ApiError::BadRequest);
        }
    };
    let result = match state.webauthn.finish_passkey_authentication(&pkc, &auth_state) {
        Ok(r) => r,
        Err(_) => {
            record_passkey_login(&state, user_id, &email, AccessOutcome::Failure, &meta).await;
            return Err(ApiError::Unauthorized);
        }
    };
    persist_auth(&state.pool, &result).await?;

    record_passkey_login(&state, user_id, &email, AccessOutcome::Success, &meta).await;
    let cookie = issue_session(&state, user_id).await?;
    Ok((jar.add(cookie), Json(User { id: user_id, email })))
}
